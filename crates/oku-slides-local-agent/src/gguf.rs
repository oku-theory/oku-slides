//! Moteur Gguf : llama.cpp (Metal) sur un fil dédié, propriétaire du modèle et du contexte.
//!
//! `LlamaContext` n'est pas `Send` : un seul fil le garde, reçoit les complétions par canal et
//! vérifie l'annulation entre les blocs du prompt et avant chaque jeton. Le cache KV est vidé à
//! chaque complétion.
//!
//! Lâcher le moteur arrête le fil et l'attend : Metal refuse de finir le processus (abandon
//! à `exit`) tant que le modèle et le contexte vivent encore.

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::mpsc;
use std::thread::JoinHandle;

use async_trait::async_trait;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::sampling::LlamaSampler;
// Dépendance directe pour épingler la version de llama.cpp ; aucun appel direct.
use llama_cpp_sys_2 as _;
use oku_slides_platform::config::LocalAgentSettings;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::engine::{CancelledSnafu, Completion, Engine, EngineError, FailedSnafu, Reply, Step};
use crate::error::{LocalAgentError, ModelSnafu};

/// Jetons envoyés au modèle par appel à `decode` pendant la lecture du prompt.
const PREFILL_BATCH: u32 = 512;
/// [`PREFILL_BATCH`] as an index (u32 always fits in usize here, 64-bit target).
const PREFILL_CHUNK: usize = PREFILL_BATCH as usize;

/// Graine fixe de l'échantillonneur : un même prompt donne la même réponse.
const SEED: u32 = 1_022;

/// Une complétion confiée au fil du moteur.
struct Job {
    prompt: String,
    grammar: Option<String>,
    cancel: CancellationToken,
    reply: oneshot::Sender<Result<String, EngineError>>,
}

/// Le moteur Gguf ; le lâcher arrête son fil et attend qu'il ait libéré llama.cpp.
pub(crate) struct GgufEngine {
    /// `None` seulement pendant `drop`, pour fermer le canal avant d'attendre le fil.
    jobs: Option<mpsc::Sender<Job>>,
    /// Interrompt la complétion en cours quand le moteur est lâché.
    stop: CancellationToken,
    worker: Option<JoinHandle<()>>,
}

impl Drop for GgufEngine {
    fn drop(&mut self) {
        self.stop.cancel();
        self.jobs = None;
        if let Some(worker) = self.worker.take() {
            // Un fil paniqué a déjà tout lâché : on garde seulement la trace de la panique.
            if let Err(payload) = worker.join() {
                tracing::error!(panic = %panic_text(payload.as_ref()), "le fil du moteur Gguf a paniqué");
            }
        }
    }
}

/// Le message d'une panique de fil, quand c'est une chaîne.
fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "panic payload is not a string".to_owned())
}

/// Refuse des réglages qui feraient échouer ou dérailler chaque complétion.
///
/// Vérifié avant de lancer le fil : aucun appel à llama.cpp.
fn validate(model: &Path, settings: &LocalAgentSettings) -> Result<(), LocalAgentError> {
    let bad = |message: &str| Err(ModelSnafu { path: model, message: message.to_owned() }.build().into());
    if settings.context == 0 {
        return bad("local_agent.context must be above 0");
    }
    if settings.max_reply_tokens >= settings.context {
        return bad("local_agent.max_reply_tokens must be below local_agent.context");
    }
    if !settings.temperature.is_finite() || settings.temperature < 0.0 {
        return bad("local_agent.temperature must be a finite number, 0 or more");
    }
    Ok(())
}

impl std::fmt::Debug for GgufEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GgufEngine")
    }
}

impl GgufEngine {
    /// Charge le modèle sur le fil du moteur ; rend la main quand il est prêt ou en échec.
    pub(crate) fn load(model: &Path, settings: &LocalAgentSettings) -> Result<Self, LocalAgentError> {
        validate(model, settings)?;
        let (jobs, rx) = mpsc::channel::<Job>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let stop = CancellationToken::new();
        let (path, settings, worker_stop) = (model.to_owned(), settings.clone(), stop.clone());
        let handle = std::thread::Builder::new()
            .name("oku-gguf".into())
            .spawn(move || worker(&path, &settings, &worker_stop, &ready_tx, &rx))
            .map_err(|e| ModelSnafu { path: model, message: e.to_string() }.build())?;
        let ready = match ready_rx.recv() {
            Ok(result) => result,
            Err(_) => Err("engine thread stopped".to_owned()),
        };
        if let Err(message) = ready {
            // Le fil a rendu son erreur et s'arrête : l'attendre libère llama.cpp avant de rendre la main.
            let message = match handle.join() {
                Err(payload) => format!("{message} (engine thread panicked: {})", panic_text(payload.as_ref())),
                Ok(()) => message,
            };
            return Err(ModelSnafu { path: model, message }.build().into());
        }
        Ok(Self { jobs: Some(jobs), stop, worker: Some(handle) })
    }
}

/// Le fil du moteur : initialise llama.cpp, charge le modèle, puis sert les complétions.
fn worker(
    path: &Path,
    settings: &LocalAgentSettings,
    stop: &CancellationToken,
    ready: &mpsc::Sender<Result<(), String>>,
    jobs: &mpsc::Receiver<Job>,
) {
    llama_cpp_2::send_logs_to_tracing(llama_cpp_2::LogOptions::default());
    let loaded = LlamaBackend::init().map_err(|e| e.to_string()).and_then(|backend| {
        let params = LlamaModelParams::default().with_n_gpu_layers(999);
        let model = LlamaModel::load_from_file(&backend, path, &params).map_err(|e| e.to_string())?;
        Ok((backend, model))
    });
    let (backend, model) = match loaded {
        Ok(loaded) => loaded,
        Err(message) => {
            // `load` attend la réponse ; un récepteur parti n'a plus rien à apprendre.
            let _ = ready.send(Err(message));
            return;
        }
    };
    let params =
        LlamaContextParams::default().with_n_ctx(NonZeroU32::new(settings.context)).with_n_batch(PREFILL_BATCH);
    let mut ctx = match model.new_context(&backend, params) {
        Ok(ctx) => ctx,
        Err(e) => {
            let _ = ready.send(Err(e.to_string()));
            return;
        }
    };
    let _ = ready.send(Ok(()));
    while let Ok(job) = jobs.recv() {
        let result = generate(&model, &mut ctx, settings, stop, &job);
        // Un récepteur lâché : le tour a été abandonné (annulé).
        let _ = job.reply.send(result);
    }
}

fn failed(e: impl std::fmt::Display) -> EngineError {
    FailedSnafu { message: e.to_string() }.build().into()
}

fn cancelled() -> EngineError {
    CancelledSnafu.build().into()
}

/// Une complétion : lecture du prompt par blocs, puis échantillonnage jeton par jeton.
fn generate(
    model: &LlamaModel,
    ctx: &mut LlamaContext<'_>,
    settings: &LocalAgentSettings,
    stop: &CancellationToken,
    job: &Job,
) -> Result<String, EngineError> {
    let halted = || job.cancel.is_cancelled() || stop.is_cancelled();
    ctx.clear_kv_cache();
    let vocab = model.vocab();
    // Jetons spéciaux lus : le gabarit ChatML en a besoin ; `chatml` neutralise ceux des corps.
    let tokens = vocab.tokenize(job.prompt.as_bytes(), false, true);
    let budget = usize::try_from(settings.max_reply_tokens).map_err(failed)?;
    let context = usize::try_from(settings.context).map_err(failed)?;
    if tokens.is_empty() || tokens.len() + budget > context {
        return Err(failed("prompt too long"));
    }
    let mut batch = LlamaBatch::new(PREFILL_CHUNK, 1);
    let mut pos: i32 = 0;
    let last = tokens.len() - 1;
    for (n, chunk) in tokens.chunks(PREFILL_CHUNK).enumerate() {
        if halted() {
            return Err(cancelled());
        }
        batch.clear();
        for (i, token) in chunk.iter().enumerate() {
            batch.add(*token, pos, &[0], n * PREFILL_CHUNK + i == last).map_err(failed)?;
            pos += 1;
        }
        ctx.decode(&mut batch).map_err(failed)?;
    }
    let mut sampler = sampler(model, settings, job.grammar.as_deref())?;
    let mut bytes = Vec::new();
    for _ in 0..budget {
        if halted() {
            return Err(cancelled());
        }
        // `sample` accepte aussi le jeton (grammaire comprise) : pas d'`accept` en plus.
        let token = sampler.sample(ctx, batch.n_tokens() - 1);
        if vocab.is_eog(token) {
            break;
        }
        bytes.extend(vocab.token_to_piece(token, false, None));
        batch.clear();
        batch.add(token, pos, &[0], true).map_err(failed)?;
        pos += 1;
        ctx.decode(&mut batch).map_err(failed)?;
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Grammaire et glouton pour le routage ; température et tirage pour agir.
fn sampler(
    model: &LlamaModel,
    settings: &LocalAgentSettings,
    grammar: Option<&str>,
) -> Result<LlamaSampler, EngineError> {
    Ok(match grammar {
        Some(g) => LlamaSampler::chain_simple([
            LlamaSampler::grammar(model, g, "root").map_err(failed)?,
            LlamaSampler::greedy(),
        ]),
        None => LlamaSampler::chain_simple([LlamaSampler::temp(settings.temperature), LlamaSampler::dist(SEED)]),
    })
}

fn stopped() -> EngineError {
    failed("engine thread stopped")
}

#[async_trait]
impl Engine for GgufEngine {
    async fn complete(&self, c: &Completion, cancel: &CancellationToken) -> Result<Reply, EngineError> {
        let grammar = (c.step == Step::Route).then(|| crate::families::route_grammar(&c.offered));
        let (tx, rx) = oneshot::channel();
        let job = Job { prompt: crate::chatml::render(c), grammar, cancel: cancel.clone(), reply: tx };
        self.jobs.as_ref().ok_or_else(stopped)?.send(job).map_err(|_| stopped())?;
        let text = tokio::select! {
            r = rx => r.map_err(|_| stopped())??,
            () = cancel.cancelled() => return Err(cancelled()),
        };
        Ok(match c.step {
            Step::Route => Reply::Text(text),
            Step::Act => crate::reply::parse(&text),
        })
    }
}

#[cfg(test)]
#[path = "gguf_tests.rs"]
mod gguf_tests;
