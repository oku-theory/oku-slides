//! Oku Slides dictation: microphone → voice detection → engine → text.
//!
//! Three engines, two of them on the machine:
//! - Apple (macOS 26+): SpeechAnalyzer transcribes continuously, as the
//!   audio comes in, through a Swift bridge (`apple.rs`).
//! - Whisper (Turbo, Large, Base): CTranslate2 (the one in faster-whisper),
//!   via ct2rs; it is the engine elsewhere, or on request.
//! - OpenAI-compatible API (`api.rs`): sentence by sentence, without preview.
//!
//! Voice is detected frame by frame (earshot): listening stops by itself
//! after a silence. With Whisper, each finished sentence is transcribed once,
//! at final quality, as soon as the speaker pauses; the preview only
//! retranscribes the current sentence. On stop, only the last sentence
//! is therefore left to process.
//!
//! The engine is polled (`status`), which serves both the Tauri app
//! and the development server: no events to propagate.
//! One press on the microphone is enough: if the model is missing, it is downloaded,
//! then listening starts by itself.

mod api;
mod api_check;
#[cfg(target_os = "macos")]
mod apple;
mod audio;
mod dictation_stream;
mod dictation_whisper;
mod error;
mod locale;
mod models;
mod num;
mod phrase_engine;
#[cfg(any(test, target_os = "macos"))]
mod results;
mod stream;
#[cfg(test)]
mod test_support;
mod vad;
mod whisper;

use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

pub use api::ApiConfig;
pub use api_check::{is_loopback, normalized_url, url_host, valid_api_model, valid_key_env};
use audio::SAMPLE_RATE_F32;
pub use audio::{SAMPLE_RATE, resample};
use dictation_stream::{Held, cancel_held};
use dictation_whisper::{EngineKey, Job, engine_key};
use error::{ApiKeyMissingSnafu, LocaleUnsupportedSnafu, NotDownloadableSnafu};
pub use error::{SttError, SttErrorKind};
pub use models::{Model, download};
use num::{count_f32, seconds};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::Platform;
use oku_slides_platform::config::{ModelsSettings, SttSettings};
use oku_slides_platform::storage::Storage;
use phrase_engine::PhraseEngine;
use stream::{Availability, Stream, StreamEngine};
use vad::Segmenter;
pub use whisper::{Transcriber, Transcript};

/// Where dictation stands, for the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// Idle: ready to listen.
    #[default]
    Idle,
    /// Downloading the model, before listening.
    Downloading,
    /// Listening to the microphone.
    Recording,
    /// Listening stopped: the last sentence is being transcribed.
    Transcribing,
}

/// Durées de silence acceptées pour l'arrêt seul (s).
pub const SILENCE_RANGE: RangeInclusive<f32> = 0.5..=10.0;

/// Version des réglages de la dictée : 1 depuis le moteur d'Apple.
const CONFIG_VERSION: u32 = 1;

/// Dictation settings, saved in `config.json` in the models folder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// Chosen engine: Apple, a Whisper model or the API.
    pub model: Model,
    /// Code de langue (`fr`, `en`…) ou `None` pour la détection automatique.
    pub language: Option<String>,
    /// Arrêter l'écoute seule après un silence.
    pub auto_stop: bool,
    /// Durée de ce silence (s).
    pub silence: f32,
    /// Version des réglages : 0 avant le moteur d'Apple (bascule unique).
    pub version: u32,
    /// Réglages de l'API de transcription (modèle `Api`).
    pub api: ApiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: Model::Turbo,
            language: None,
            auto_stop: true,
            silence: 1.5,
            version: 0,
            api: ApiConfig::default(),
        }
    }
}

/// Réglages d'avant le moteur d'Apple : on y passe une fois, s'il sert ici.
///
/// `apple` : le moteur existe sur cette machine ; `language` : il connaît la
/// langue retenue (prête ou à télécharger). Sans moteur, Apple redevient Turbo.
fn migrate(mut config: Config, apple: bool, language: bool) -> Config {
    if config.version < CONFIG_VERSION {
        if apple && language {
            config.model = Model::Apple;
        }
        config.version = CONFIG_VERSION;
    }
    if config.model == Model::Apple && !apple {
        config.model = Model::Turbo;
    }
    config
}

/// A dictation engine as the UI offers it.
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    /// Engine described.
    pub id: Model,
    /// Display name, in the UI language.
    pub label: String,
    /// Bytes to download (0 for Apple and the API).
    pub size: u64,
    /// Ready to serve: files on disk, language installed or key present.
    pub installed: bool,
    /// Faux : le modèle ne gère pas la langue choisie.
    pub available: bool,
}

/// A model download in progress.
#[derive(Debug, Clone, Serialize)]
pub struct Download {
    /// Model being downloaded.
    pub model: Model,
    /// Bytes already received.
    pub received: u64,
    /// Total bytes expected.
    pub total: u64,
}

/// Ce que l'interface affiche.
#[derive(Debug, Clone, Serialize)]
pub struct Status {
    /// Where dictation stands.
    pub state: State,
    /// Settings in effect.
    pub config: Config,
    /// Engines offered on this machine.
    pub models: Vec<ModelInfo>,
    /// Download in progress, if any.
    pub download: Option<Download>,
    /// Niveau du micro, 0..1.
    pub level: f32,
    /// Voix détectée en ce moment.
    pub speaking: bool,
    /// Phrases déjà transcrites (texte définitif).
    pub committed: String,
    /// Phrase en cours (aperçu provisoire).
    pub partial: String,
    /// Langue détectée pendant cette dictée.
    pub detected: Option<String>,
    /// Secondes avant l'arrêt automatique (pendant un silence), sinon `None`.
    pub stop_in: Option<f32>,
    /// Dernière transcription terminée, et son numéro (change à chaque nouvelle).
    pub text: Option<String>,
    /// Number of [`Self::text`], incremented at each finished dictation.
    pub seq: u64,
    /// Secondes écoutées.
    pub seconds: f32,
    /// Last error, already translated into the UI language.
    pub error: Option<String>,
}

/// Requêtes de l'interface (même forme pour Tauri et HTTP).
///
/// Les réglages et les téléchargements passent par la boîte des réglages de l'hôte, pas par ici.
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum SttRequest {
    /// Returns the state, without changing anything.
    Status,
    /// Démarre la dictée ; `vocabulary` aide la reconnaissance (titres du deck, noms propres).
    Start {
        /// Terms to recognize, from most to least useful.
        #[serde(default)]
        vocabulary: Vec<String>,
    },
    /// Stops listening; the last sentence is transcribed.
    Stop,
    /// Abandons the dictation or the download, without text.
    Cancel,
}

/// Où part l'audio d'une dictée.
enum Sink {
    /// Moteur phrase par phrase (Whisper, API) : phrases closes et aperçus, en jobs.
    Whisper { jobs: mpsc::Sender<Job>, busy: Arc<AtomicBool>, previews: bool },
    /// Moteur en continu : tout l'audio, au fil de l'eau ; confié à `held` à l'arrêt.
    Stream { stream: Box<dyn Stream>, held: Held },
}

struct Inner {
    state: State,
    config: Config,
    download: Option<Download>,
    level: f32,
    speaking: bool,
    /// Textes des phrases closes, dans l'ordre (None : pas encore transcrite).
    phrases: Vec<Option<String>>,
    partial: String,
    detected: Option<String>,
    stop_in: Option<f32>,
    text: Option<String>,
    seq: u64,
    error: Option<String>,
    started: Option<Instant>,
    /// Vocabulaire de la dictée en cours.
    vocabulary: Vec<String>,
    start_after_download: bool,
    engine: Option<(EngineKey, Arc<dyn PhraseEngine>)>,
    /// Modèles présents sur le disque (dans l'ordre de `Model::ALL`) : lus au
    /// démarrage et après un téléchargement, pas à chaque sondage.
    installed: [bool; Model::ALL.len()],
    /// Incrémenté à chaque dictée : les fils d'une dictée abandonnée
    /// s'arrêtent d'eux-mêmes.
    session: u64,
    /// Langue du moteur d'Apple et sa disponibilité (None : aucune langue connue).
    apple: Option<(String, Availability)>,
}

impl Inner {
    fn committed(&self) -> String {
        join(self.phrases.iter().flatten())
    }

    /// Fin d'une dictée : le texte assemblé est rendu, retour au repos.
    fn complete(&mut self) {
        self.text = Some(self.committed());
        self.seq += 1;
        self.phrases.clear();
        self.partial.clear();
        self.state = State::Idle;
        self.stop_in = None;
        self.level = 0.0;
        self.speaking = false;
        self.session += 1;
    }

    /// Dictée abandonnée : retour au repos, sans texte.
    fn reset(&mut self) {
        self.state = State::Idle;
        self.started = None;
        self.phrases.clear();
        self.partial.clear();
        self.stop_in = None;
        self.level = 0.0;
        self.speaking = false;
        self.session += 1;
    }
}

/// Dictation: shared state, engines and settings. Clones without recreating anything.
#[derive(Clone)]
pub struct Stt {
    inner: Arc<Mutex<Inner>>,
    dir: PathBuf,
    /// Rythmes et seuils (réglages de l'app).
    settings: Arc<SttSettings>,
    /// Dépôt et tentatives des téléchargements de modèles.
    models: Arc<ModelsSettings>,
    storage: Storage,
    localizer: Arc<Localizer>,
    cancel_download: Arc<AtomicBool>,
    /// Moteur en continu (Apple), et s'il existe ici.
    streaming: Arc<dyn StreamEngine>,
    apple_supported: bool,
    /// Langue du système, lue au démarrage (dernier repli du moteur d'Apple).
    system_locale: Option<String>,
}

fn config_path(dir: &Path) -> PathBuf {
    dir.join("config.json")
}

impl std::fmt::Debug for Stt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stt").field("dir", &self.dir).finish_non_exhaustive()
    }
}

impl Stt {
    /// La dictée de la plateforme : dossier des modèles et rythmes de
    /// l'écoute (réglages), fichiers, langue des messages.
    #[must_use]
    pub fn new(platform: &Platform) -> Self {
        Self::with_engine(platform, stream::engine(&platform.settings.stt), sys_locale::get_locale())
    }

    /// La dictée, avec ce moteur en continu (Apple, ou un moteur factice en test).
    pub(crate) fn with_engine(
        platform: &Platform,
        streaming: Arc<dyn StreamEngine>,
        system_locale: Option<String>,
    ) -> Self {
        let dir = platform.settings.paths.models();
        let storage = platform.storage.clone();
        let apple_supported = streaming.supported();
        // Réglages absents ou illisibles : ceux par défaut.
        let config: Config =
            storage.read(&config_path(&dir)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        // Bascule unique vers Apple : seulement s'il connaît la langue retenue.
        let resolved = apple_supported
            .then(|| {
                resolve_apple(
                    &*streaming,
                    config.language.as_deref(),
                    platform.localizer.locale(),
                    system_locale.as_deref(),
                )
            })
            .flatten();
        let config = migrate(config, apple_supported, resolved.is_some());
        let stt = Self {
            inner: Arc::new(Mutex::new(Inner {
                state: State::Idle,
                config,
                download: None,
                level: 0.0,
                speaking: false,
                phrases: Vec::new(),
                partial: String::new(),
                detected: None,
                stop_in: None,
                text: None,
                seq: 0,
                error: None,
                started: None,
                vocabulary: Vec::new(),
                start_after_download: false,
                engine: None,
                session: 0,
                installed: Model::ALL.map(|m| m.is_installed(&storage, &dir)),
                apple: None,
            })),
            dir,
            settings: Arc::new(platform.settings.stt.clone()),
            models: Arc::new(platform.settings.models.clone()),
            storage,
            localizer: platform.localizer.clone(),
            cancel_download: Arc::new(AtomicBool::new(false)),
            streaming,
            apple_supported,
            system_locale,
        };
        stt.refresh_apple();
        stt
    }

    /// Langue du moteur d'Apple et sa disponibilité, d'après les réglages.
    fn refresh_apple(&self) {
        if !self.apple_supported {
            return;
        }
        let chosen = self.state().config.language.clone();
        let resolved =
            resolve_apple(&*self.streaming, chosen.as_deref(), self.localizer.locale(), self.system_locale.as_deref());
        let mut g = self.state();
        if g.config.model == Model::Apple {
            g.detected = resolved.as_ref().map(|(tag, _)| locale::language(tag));
        }
        g.apple = resolved;
    }

    /// Langue voulue pour la dictée : la choisie, sinon celle de l'interface.
    fn wanted_language(&self) -> String {
        self.state().config.language.clone().unwrap_or_else(|| self.localizer.locale().to_owned())
    }

    /// Signale une erreur à l'interface (et au journal).
    fn report(&self, e: &SttError) {
        tracing::warn!(error = %oku_slides_platform::errors::one_line(e), "dictée");
        self.state().error = Some(e.localize(&self.localizer));
    }

    fn model_info(&self, model: Model, installed: bool, g: &Inner) -> ModelInfo {
        let (installed, available) = match model {
            Model::Apple => (g.apple.as_ref().is_some_and(|(_, a)| a.is_ready()), g.apple.is_some()),
            // « Installée » : la clé est là ; elle n'est jamais rendue.
            Model::Api => (g.config.api.key_present(), true),
            _ => (installed, true),
        };
        let label = model.label(&self.localizer, &g.config.api);
        ModelInfo { id: model, label, size: model.size(), installed, available }
    }

    /// L'état partagé, même si un fil a paniqué en le tenant : l'interface
    /// doit toujours pouvoir le lire, et une dictée neuve le remet d'aplomb.
    fn state(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Folder of the dictation models and their settings.
    #[must_use]
    pub fn models_dir(&self) -> &Path {
        &self.dir
    }

    /// Handles a request and returns the state.
    #[expect(clippy::must_use_candidate, reason = "acts on the dictation; the returned state may be ignored")]
    pub fn handle(&self, req: SttRequest) -> Status {
        match req {
            SttRequest::Status => {}
            SttRequest::Start { vocabulary } => self.start(vocabulary),
            SttRequest::Stop => self.stop(),
            SttRequest::Cancel => self.cancel(),
        }
        self.status()
    }

    /// The state to display: engines, sentences, microphone level, error.
    #[must_use]
    pub fn status(&self) -> Status {
        let g = self.state();
        Status {
            state: g.state,
            config: g.config.clone(),
            models: Model::ALL
                .iter()
                .zip(g.installed)
                .filter(|(m, _)| **m != Model::Apple || self.apple_supported)
                .map(|(&m, installed)| self.model_info(m, installed, &g))
                .collect(),
            download: g.download.clone(),
            level: g.level,
            speaking: g.speaking,
            committed: g.committed(),
            partial: g.partial.clone(),
            detected: g.detected.clone(),
            stop_in: g.stop_in,
            text: g.text.clone(),
            seq: g.seq,
            seconds: g.started.map_or(0.0, |t| t.elapsed().as_secs_f32()),
            error: g.error.clone(),
        }
    }

    /// Applique et enregistre les réglages ; faux si la dictée est occupée.
    #[must_use = "faux : la dictée était occupée, rien n'a changé"]
    pub fn configure(&self, mut config: Config) -> bool {
        config.silence = config.silence.clamp(*SILENCE_RANGE.start(), *SILENCE_RANGE.end());
        config.version = CONFIG_VERSION;
        {
            let mut g = self.state();
            if g.state != State::Idle {
                return false;
            }
            if g.engine.as_ref().is_some_and(|(k, _)| *k != engine_key(&config)) {
                g.engine = None; // libère la mémoire de l'ancien modèle, ou l'ancienne API
            }
            let saved = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string()).and_then(|json| {
                self.storage.write(&config_path(&self.dir), json).map_err(|e| oku_slides_platform::errors::one_line(&e))
            });
            if let Err(e) = saved {
                // Les réglages restent en vigueur pour cette session.
                tracing::warn!(error = %e, "réglages de la dictée non enregistrés");
            }
            g.config = config;
        }
        self.refresh_apple();
        true
    }

    /// Démarre la dictée (téléchargement du modèle d'abord s'il manque).
    pub fn start(&self, vocabulary: Vec<String>) {
        let (model, apple, api) = {
            let mut g = self.state();
            if g.state != State::Idle {
                return;
            }
            g.error = None;
            g.vocabulary = vocabulary;
            (g.config.model, g.apple.clone(), g.config.api.clone())
        };
        if model == Model::Apple {
            match apple {
                Some((_, a)) if a.is_ready() => self.begin_recording(),
                Some(_) => {
                    // Rien à dire de plus : l'échec est déjà signalé dans l'état.
                    let _started = self.download(Model::Apple, true);
                }
                None => self.report(&LocaleUnsupportedSnafu { locale: self.wanted_language() }.build().into()),
            }
            return;
        }
        if model == Model::Api {
            if api.key_present() {
                self.begin_recording();
            } else {
                self.report(&ApiKeyMissingSnafu { var: api.key_env }.build().into());
            }
            return;
        }
        if model.is_installed(&self.storage, &self.dir) {
            self.begin_recording();
        } else {
            // Rien à dire de plus : l'échec est déjà signalé dans l'état.
            let _started = self.download(model, true);
        }
    }

    /// Lance le téléchargement d'un modèle ; faux si rien n'a démarré.
    #[must_use = "faux : rien n'a démarré (occupée, déjà installé, API)"]
    pub fn download(&self, model: Model, then_start: bool) -> bool {
        if model == Model::Apple {
            return self.download_language(then_start);
        }
        if model == Model::Api {
            self.report(&NotDownloadableSnafu.build().into());
            return false;
        }
        {
            let mut g = self.state();
            if g.state != State::Idle || model.is_installed(&self.storage, &self.dir) {
                return false;
            }
            g.state = State::Downloading;
            g.download = Some(Download { model, received: 0, total: model.size() });
            g.start_after_download = then_start;
        }
        self.cancel_download.store(false, Ordering::Relaxed);
        let (me, dir, cancel) = (self.clone(), self.dir.clone(), self.cancel_download.clone());
        std::thread::spawn(move || {
            let mut last = Instant::now();
            let result = models::download(model, &dir, &me.models, &me.storage, |received, total| {
                if last.elapsed() > Duration::from_millis(100) {
                    last = Instant::now();
                    me.state().download = Some(Download { model, received, total });
                }
                !cancel.load(Ordering::Relaxed)
            });
            let start = me.finish_download(model, result.map(|_| ()));
            if start {
                me.begin_recording();
            }
        });
        true
    }

    /// Fin d'un téléchargement : état, message, et reprise de la dictée demandée.
    fn finish_download(&self, model: Model, result: Result<(), SttError>) -> bool {
        let mut g = self.state();
        g.download = None;
        g.state = State::Idle;
        g.installed = Model::ALL.map(|m| m.is_installed(&self.storage, &self.dir));
        match result {
            Ok(()) => {
                tracing::info!(?model, "{}", self.localizer.tr("log-model-downloaded"));
                std::mem::take(&mut g.start_after_download)
            }
            Err(e) if e.kind() == SttErrorKind::Cancelled => false,
            Err(e) => {
                tracing::warn!(?model, error = %snafu::Report::from_error(&e), "téléchargement du modèle");
                g.error = Some(e.localize(&self.localizer));
                false
            }
        }
    }

    fn begin_recording(&self) {
        let recorder = match audio::Recorder::start(self.settings.fake_mic.as_deref(), &self.storage) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %snafu::Report::from_error(&e), "micro indisponible");
                self.state().error = Some(e.localize(&self.localizer));
                return;
            }
        };
        let (session, model, apple, vocabulary) = {
            let mut g = self.state();
            g.session += 1;
            g.state = State::Recording;
            g.phrases.clear();
            g.partial.clear();
            g.detected = None;
            if g.config.model == Model::Apple {
                g.detected = g.apple.as_ref().map(|(tag, _)| locale::language(tag));
            }
            g.stop_in = None;
            g.started = Some(Instant::now());
            (g.session, g.config.model, g.apple.clone(), g.vocabulary.clone())
        };
        let sink = match (model, apple) {
            (Model::Apple, Some((tag, _))) => {
                let (stream, events) = self.streaming.open(&tag, &vocabulary);
                let held = Held::default();
                let (worker, worker_held) = (self.clone(), held.clone());
                std::thread::spawn(move || worker.stream_loop(session, &events, &worker_held));
                Sink::Stream { stream, held }
            }
            (Model::Apple, None) => {
                // `start` l'empêche ; par prudence, pas d'écoute sans langue.
                self.state().reset();
                self.report(&LocaleUnsupportedSnafu { locale: self.wanted_language() }.build().into());
                return;
            }
            _ => {
                let (jobs, rx) = mpsc::channel();
                let busy = Arc::new(AtomicBool::new(false));
                let worker = self.clone();
                let worker_busy = busy.clone();
                std::thread::spawn(move || worker.transcribe_loop(session, &rx, &worker_busy));
                Sink::Whisper { jobs, busy, previews: model.previews() }
            }
        };
        let listener = self.clone();
        std::thread::spawn(move || listener.listen_loop(session, recorder, sink));
    }

    fn alive(&self, session: u64) -> bool {
        self.state().session == session
    }

    /// Écoute : micro → 16 kHz → détection de la voix → moteur.
    fn listen_loop(&self, session: u64, mut recorder: audio::Recorder, mut sink: Sink) {
        let s = &self.settings;
        let mut seg = Segmenter::new(s.vad);
        let mut resampler: Option<audio::Resampler> = None;
        let mut fresh = Vec::new();
        let mut previewed = 0usize;
        loop {
            std::thread::sleep(Duration::from_millis(s.tick_ms));
            let (state, auto_stop, silence_limit) = {
                let g = self.state();
                if g.session != session {
                    drop(g);
                    // Dictée abandonnée : le moteur en continu s'arrête aussi.
                    if let Sink::Stream { stream, .. } = &mut sink {
                        stream.cancel();
                    }
                    return;
                }
                (g.state, g.config.auto_stop, g.config.silence)
            };
            self.pull(&recorder, &mut resampler, &mut fresh, &mut seg, &mut sink);

            if state != State::Recording {
                // Arrêt demandé : derniers échantillons, dernière phrase.
                recorder.finish();
                self.pull(&recorder, &mut resampler, &mut fresh, &mut seg, &mut sink);
                match sink {
                    Sink::Whisper { jobs, .. } => {
                        if let Some(phrase) = seg.close() {
                            self.send_final(&seg, phrase, &jobs);
                        }
                        jobs.send(Job::Done).ok();
                    }
                    Sink::Stream { mut stream, held } => {
                        stream.finish();
                        // Confié au fil des résultats ; s'il est déjà parti, on l'annule ici.
                        *held.lock().unwrap_or_else(PoisonError::into_inner) = Some(stream);
                        if !self.alive(session) {
                            cancel_held(&held);
                        }
                    }
                }
                return;
            }

            // Aperçu de la phrase en cours (Whisper local), quand le fil de transcription est libre.
            let current = seg.current();
            if let Sink::Whisper { jobs, busy, previews: true } = &sink
                && let Some(p) = current
                && !busy.load(Ordering::Relaxed)
                && count_f32(p.end.saturating_sub(previewed.max(p.start))) >= s.preview_step_secs * SAMPLE_RATE_F32
            {
                previewed = p.end;
                busy.store(true, Ordering::Relaxed);
                jobs.send(Job::Preview { audio: seg.slice(p).to_vec() }).ok();
            }

            // Arrêt automatique après un silence (ou si rien n'est dit).
            let silence = seg.silence();
            let limit = if seg.heard { silence_limit } else { s.nothing_heard_secs };
            let stop_in = (auto_stop && current.is_none() && seg.heard && silence > s.countdown_after_secs)
                .then(|| (limit - silence).max(0.0));
            let stop = auto_stop && silence >= limit;
            {
                let mut g = self.state();
                g.level = recorder.level();
                g.speaking = seg.speaking;
                g.stop_in = stop_in;
                // L'aperçu de Whisper suit la phrase en cours ; celui du moteur en continu, ses résultats.
                if current.is_none() && matches!(sink, Sink::Whisper { .. }) {
                    g.partial.clear();
                }
                if stop && g.state == State::Recording {
                    g.state = State::Transcribing;
                    g.started = None;
                }
            }
        }
    }

    /// Échantillons arrivés : 16 kHz, détection de la voix, puis le moteur.
    fn pull(
        &self,
        recorder: &audio::Recorder,
        resampler: &mut Option<audio::Resampler>,
        fresh: &mut Vec<f32>,
        seg: &mut Segmenter,
        sink: &mut Sink,
    ) {
        let (samples, rate) = recorder.take();
        if samples.is_empty() {
            return;
        }
        fresh.clear();
        resampler.get_or_insert_with(|| audio::Resampler::new(rate)).push(&samples, fresh);
        let phrases = seg.push(fresh);
        match sink {
            Sink::Whisper { jobs, .. } => {
                for phrase in phrases {
                    self.send_final(seg, phrase, jobs);
                }
            }
            Sink::Stream { stream, .. } => stream.push(fresh),
        }
    }

    /// Termine l'écoute ; la dernière phrase est transcrite puis le texte assemblé.
    pub fn stop(&self) {
        let mut g = self.state();
        if g.state == State::Recording {
            g.state = State::Transcribing;
            g.started = None;
            g.stop_in = None;
        }
    }

    /// Abandons the current dictation, without text, and cancels a download.
    pub fn cancel(&self) {
        self.cancel_download.store(true, Ordering::Relaxed);
        let mut g = self.state();
        if matches!(g.state, State::Recording | State::Transcribing) {
            g.reset();
        }
        g.start_after_download = false;
    }

    /// Transcribes a complete audio (mono 16 kHz) like a dictation.
    ///
    /// The audio is split by voice detection, sentence by sentence, with the
    /// context. Returns the text and the sentences (start, end in seconds, text).
    ///
    /// # Errors
    ///
    /// If the engine does not load (missing model, misconfigured API) or a sentence
    /// fails; with Apple, if the language is missing or the engine does not finish in time.
    pub fn transcribe_samples(&self, audio: &[f32]) -> Result<(String, Vec<TimedPhrase>), SttError> {
        let model = self.state().config.model;
        if model == Model::Apple {
            return self.transcribe_stream(audio);
        }
        let (mut language, vocabulary) = {
            let g = self.state();
            (g.config.language.clone(), g.vocabulary.clone())
        };
        let engine = self.engine()?;
        let mut texts: Vec<String> = Vec::new();
        let mut phrases = Vec::new();
        for phrase in vad::split(audio, self.settings.vad) {
            let slice = &audio[phrase.start..phrase.end.min(audio.len())];
            let context = context_of(&vocabulary, &join(texts.iter()));
            let t = engine.transcribe(slice, language.as_deref(), Some(&context), false)?;
            if language.is_none() {
                language.clone_from(&t.language);
            }
            let text = keep(&t);
            phrases.push((seconds(phrase.start), seconds(phrase.end), text.clone()));
            texts.push(text);
        }
        Ok((join(texts.iter()), phrases))
    }
}

/// Langue du moteur d'Apple : la choisie, sinon l'interface, sinon le système.
fn resolve_apple(
    streaming: &dyn StreamEngine,
    chosen: Option<&str>,
    ui: &str,
    system: Option<&str>,
) -> Option<(String, Availability)> {
    locale::resolve(chosen, ui, system, |tag| streaming.availability(tag))
}

/// Phrase transcrite : début et fin (s), texte.
pub type TimedPhrase = (f32, f32, String);

/// Longueur maximale du vocabulaire donné à Whisper (caractères).
const MAX_PROMPT_CHARS: usize = 600;

/// Le vocabulaire en une phrase pour Whisper : termes joints par « . », tronqués.
fn whisper_prompt(vocabulary: &[String]) -> String {
    let joined = vocabulary.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(". ");
    joined.chars().take(MAX_PROMPT_CHARS).collect()
}

/// Contexte donné à Whisper : vocabulaire du deck, puis ce qui vient d'être dicté.
fn context_of(vocabulary: &[String], committed: &str) -> String {
    let prompt = whisper_prompt(vocabulary);
    [prompt.as_str(), committed].iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
}

/// Texte d'une transcription, sans les hallucinations sur du non-parlé.
fn keep(t: &Transcript) -> String {
    let text = clean(&t.text);
    if t.no_speech > 0.6 && text.split_whitespace().count() < 4 { String::new() } else { text }
}

fn join<'a>(parts: impl Iterator<Item = &'a String>) -> String {
    parts.map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
}

/// Retire les artefacts de Whisper (espaces, marqueurs de silence).
fn clean(text: &str) -> String {
    let t = text.trim();
    let noise = ["[BLANK_AUDIO]", "[Musique]", "[Music]", "(musique)", "[silence]"];
    if noise.iter().any(|n| t.eq_ignore_ascii_case(n)) {
        return String::new();
    }
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
