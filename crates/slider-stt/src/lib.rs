//! Dictée d'Écho : micro → détection de la voix → Whisper en local → texte.
//!
//! - La voix est détectée trame par trame (earshot) : seules les phrases
//!   parlées sont transcrites, et l'écoute s'arrête seule après un silence.
//! - Chaque phrase terminée est transcrite une fois, en qualité finale, dès
//!   qu'on marque une pause ; l'aperçu ne retranscrit que la phrase en cours.
//!   À l'arrêt, il ne reste donc que la dernière phrase à traiter.
//! - Le moteur est CTranslate2 (celui de faster-whisper), via ct2rs.
//!
//! Le moteur est interrogé par sondage (`status`), ce qui sert à la fois l'app
//! Tauri et le serveur de développement : pas d'événements à propager.
//! Un appui sur le micro suffit : si le modèle manque, il est téléchargé,
//! puis l'écoute commence toute seule.

mod audio;
mod models;
mod vad;
mod whisper;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

pub use audio::{SAMPLE_RATE, resample};
pub use models::{DownloadError, Model, download};
use vad::{Phrase, Segmenter, VadConfig};
pub use whisper::{Transcriber, Transcript};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    #[default]
    Idle,
    Downloading,
    Recording,
    Transcribing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub model: Model,
    /// Code de langue (`fr`, `en`…) ou `None` pour la détection automatique.
    pub language: Option<String>,
    /// Arrêter l'écoute seule après un silence.
    pub auto_stop: bool,
    /// Durée de ce silence (s).
    pub silence: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self { model: Model::Turbo, language: None, auto_stop: true, silence: 1.5 }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: Model,
    pub label: &'static str,
    pub size: u64,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Download {
    pub model: Model,
    pub received: u64,
    pub total: u64,
}

/// Ce que l'interface affiche.
#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub state: State,
    pub config: Config,
    pub models: Vec<ModelInfo>,
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
    pub seq: u64,
    /// Secondes écoutées.
    pub seconds: f32,
    pub error: Option<String>,
}

/// Requêtes de l'interface (même forme pour Tauri et HTTP).
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum SttRequest {
    Status,
    Configure(Config),
    /// Démarre la dictée ; `prompt` aide Whisper (titres du deck, noms propres).
    Start {
        #[serde(default)]
        prompt: Option<String>,
    },
    Stop,
    Cancel,
    Download {
        model: Model,
    },
}

/// Sans aucune parole pendant ce temps, l'écoute s'arrête (s).
const NOTHING_HEARD: f32 = 10.0;
/// Rythme de la boucle d'écoute.
const TICK: Duration = Duration::from_millis(50);
/// Nouvel aperçu quand la phrase en cours a grandi d'au moins (s).
const PREVIEW_STEP: f32 = 0.5;

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
    prompt: Option<String>,
    start_after_download: bool,
    engine: Option<(Model, Arc<Transcriber>)>,
    /// Incrémenté à chaque dictée : les fils d'une dictée abandonnée
    /// s'arrêtent d'eux-mêmes.
    session: u64,
}

impl Inner {
    fn committed(&self) -> String {
        join(self.phrases.iter().flatten())
    }
}

#[derive(Clone)]
pub struct Stt {
    inner: Arc<Mutex<Inner>>,
    dir: PathBuf,
    cancel_download: Arc<AtomicBool>,
}

/// Identifiant de l'app : même dossier que `app_data_dir` de Tauri.
pub const APP_ID: &str = "me.dartiguenave.echo";

/// Dossier des modèles partagé par l'app et le serveur de développement
/// (`ECHO_MODELS` le remplace) : un modèle n'est téléchargé qu'une fois.
pub fn default_models_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("ECHO_MODELS") {
        return dir.into();
    }
    let home =
        std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
    let base = if cfg!(target_os = "macos") {
        home.join("Library/Application Support")
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| home.join("AppData/Roaming"))
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"))
    };
    base.join(APP_ID).join("models")
}

fn config_path(dir: &Path) -> PathBuf {
    dir.join("config.json")
}

/// Travail pour le fil de transcription.
enum Job {
    /// Phrase close : texte définitif, rangé à l'indice donné.
    Final { index: usize, audio: Vec<f32> },
    /// Phrase en cours : aperçu.
    Preview { audio: Vec<f32> },
    /// Fin de la dictée : assembler le texte.
    Done,
}

impl Stt {
    /// `dir` : dossier des modèles et des réglages de la dictée.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        let config =
            std::fs::read(config_path(&dir)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Self {
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
                prompt: None,
                start_after_download: false,
                engine: None,
                session: 0,
            })),
            dir,
            cancel_download: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn models_dir(&self) -> &Path {
        &self.dir
    }

    /// Traite une requête et renvoie l'état.
    pub fn handle(&self, req: SttRequest) -> Status {
        match req {
            SttRequest::Status => {}
            SttRequest::Configure(config) => self.configure(config),
            SttRequest::Start { prompt } => self.start(prompt),
            SttRequest::Stop => self.stop(),
            SttRequest::Cancel => self.cancel(),
            SttRequest::Download { model } => self.download(model, false),
        }
        self.status()
    }

    pub fn status(&self) -> Status {
        let g = self.inner.lock().unwrap();
        Status {
            state: g.state,
            config: g.config.clone(),
            models: Model::ALL
                .iter()
                .map(|&m| ModelInfo { id: m, label: m.label(), size: m.size(), installed: m.is_installed(&self.dir) })
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
            seconds: g.started.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0),
            error: g.error.clone(),
        }
    }

    pub fn configure(&self, mut config: Config) {
        config.silence = config.silence.clamp(0.5, 10.0);
        let mut g = self.inner.lock().unwrap();
        if g.state != State::Idle {
            return;
        }
        if g.engine.as_ref().is_some_and(|(m, _)| *m != config.model) {
            g.engine = None; // libère la mémoire de l'ancien modèle
        }
        std::fs::create_dir_all(&self.dir).ok();
        std::fs::write(config_path(&self.dir), serde_json::to_vec_pretty(&config).unwrap_or_default()).ok();
        g.config = config;
    }

    /// Démarre la dictée (téléchargement du modèle d'abord s'il manque).
    pub fn start(&self, prompt: Option<String>) {
        let model = {
            let mut g = self.inner.lock().unwrap();
            if g.state != State::Idle {
                return;
            }
            g.error = None;
            g.prompt = prompt;
            g.config.model
        };
        if model.is_installed(&self.dir) {
            self.begin_recording();
        } else {
            self.download(model, true);
        }
    }

    pub fn download(&self, model: Model, then_start: bool) {
        {
            let mut g = self.inner.lock().unwrap();
            if g.state != State::Idle || model.is_installed(&self.dir) {
                return;
            }
            g.state = State::Downloading;
            g.download = Some(Download { model, received: 0, total: model.size() });
            g.start_after_download = then_start;
        }
        self.cancel_download.store(false, Ordering::Relaxed);
        let (me, dir, cancel) = (self.clone(), self.dir.clone(), self.cancel_download.clone());
        std::thread::spawn(move || {
            let mut last = Instant::now();
            let result = models::download(model, &dir, |received, total| {
                if last.elapsed() > Duration::from_millis(100) {
                    last = Instant::now();
                    if let Ok(mut g) = me.inner.lock() {
                        g.download = Some(Download { model, received, total });
                    }
                }
                !cancel.load(Ordering::Relaxed)
            });
            let start = {
                let mut g = me.inner.lock().unwrap();
                g.download = None;
                g.state = State::Idle;
                match result {
                    Ok(_) => std::mem::take(&mut g.start_after_download),
                    Err(models::DownloadError::Cancelled) => false,
                    Err(e) => {
                        g.error = Some(e.to_string());
                        false
                    }
                }
            };
            if start {
                me.begin_recording();
            }
        });
    }

    fn begin_recording(&self) {
        let recorder = match audio::Recorder::start() {
            Ok(r) => r,
            Err(e) => {
                self.inner.lock().unwrap().error = Some(e);
                return;
            }
        };
        let session = {
            let mut g = self.inner.lock().unwrap();
            g.session += 1;
            g.state = State::Recording;
            g.phrases.clear();
            g.partial.clear();
            g.detected = None;
            g.stop_in = None;
            g.started = Some(Instant::now());
            g.session
        };
        let (jobs, rx) = mpsc::channel();
        let busy = Arc::new(AtomicBool::new(false));
        let worker = self.clone();
        let worker_busy = busy.clone();
        std::thread::spawn(move || worker.transcribe_loop(session, rx, worker_busy));
        let listener = self.clone();
        std::thread::spawn(move || listener.listen_loop(session, recorder, jobs, busy));
    }

    /// Modèle chargé une fois (et gardé tant qu'on ne change pas de modèle).
    fn engine(&self) -> Result<Arc<Transcriber>, String> {
        let model = {
            let g = self.inner.lock().unwrap();
            if let Some((m, e)) = &g.engine
                && *m == g.config.model
            {
                return Ok(e.clone());
            }
            g.config.model
        };
        static LOADING: Mutex<()> = Mutex::new(());
        let _loading = LOADING.lock().unwrap();
        if let Some((m, e)) = &self.inner.lock().unwrap().engine
            && *m == model
        {
            return Ok(e.clone());
        }
        let engine = Arc::new(Transcriber::load(&model.path(&self.dir))?);
        self.inner.lock().unwrap().engine = Some((model, engine.clone()));
        Ok(engine)
    }

    fn alive(&self, session: u64) -> bool {
        self.inner.lock().map(|g| g.session == session).unwrap_or(false)
    }

    /// Écoute : micro → 16 kHz → détection de la voix → phrases.
    fn listen_loop(&self, session: u64, mut recorder: audio::Recorder, jobs: mpsc::Sender<Job>, busy: Arc<AtomicBool>) {
        let mut seg = Segmenter::new(VadConfig::default());
        let mut resampler: Option<audio::Resampler> = None;
        let mut fresh = Vec::new();
        let mut previewed = 0usize;
        let mut pull = |seg: &mut Segmenter, recorder: &audio::Recorder| {
            let (samples, rate) = recorder.take();
            if samples.is_empty() {
                return;
            }
            fresh.clear();
            resampler.get_or_insert_with(|| audio::Resampler::new(rate)).push(&samples, &mut fresh);
            for phrase in seg.push(&fresh) {
                self.send_final(seg, phrase, &jobs);
            }
        };
        loop {
            std::thread::sleep(TICK);
            let (state, config) = match self.inner.lock() {
                Ok(g) if g.session == session => (g.state, g.config.clone()),
                _ => return, // dictée abandonnée
            };
            pull(&mut seg, &recorder);

            if state != State::Recording {
                // Arrêt demandé : derniers échantillons, dernière phrase.
                recorder.finish();
                pull(&mut seg, &recorder);
                if let Some(phrase) = seg.close() {
                    self.send_final(&seg, phrase, &jobs);
                }
                jobs.send(Job::Done).ok();
                return;
            }

            // Aperçu de la phrase en cours, quand le fil de transcription est libre.
            let current = seg.current();
            if let Some(p) = current
                && !busy.load(Ordering::Relaxed)
                && p.end.saturating_sub(previewed.max(p.start)) as f32 >= PREVIEW_STEP * SAMPLE_RATE as f32
            {
                previewed = p.end;
                busy.store(true, Ordering::Relaxed);
                jobs.send(Job::Preview { audio: seg.slice(p).to_vec() }).ok();
            }

            // Arrêt automatique après un silence (ou si rien n'est dit).
            let silence = seg.silence();
            let limit = if seg.heard { config.silence } else { NOTHING_HEARD };
            let stop_in = (config.auto_stop && current.is_none() && seg.heard && silence > 0.3)
                .then(|| (limit - silence).max(0.0));
            let stop = config.auto_stop && silence >= limit;
            if let Ok(mut g) = self.inner.lock() {
                g.level = recorder.level();
                g.speaking = seg.speaking;
                g.stop_in = stop_in;
                if current.is_none() {
                    g.partial.clear();
                }
                if stop && g.state == State::Recording {
                    g.state = State::Transcribing;
                    g.started = None;
                }
            }
        }
    }

    fn send_final(&self, seg: &Segmenter, phrase: Phrase, jobs: &mpsc::Sender<Job>) {
        // Un simple claquement ne fait pas une phrase.
        if phrase.seconds() < 0.3 {
            return;
        }
        let index = {
            let mut g = self.inner.lock().unwrap();
            g.phrases.push(None);
            g.phrases.len() - 1
        };
        jobs.send(Job::Final { index, audio: seg.slice(phrase).to_vec() }).ok();
    }

    /// Transcription : phrases closes en qualité finale, aperçu de la phrase en cours.
    fn transcribe_loop(&self, session: u64, rx: mpsc::Receiver<Job>, busy: Arc<AtomicBool>) {
        // Chargement du modèle pendant qu'on commence à parler.
        let engine = match self.engine() {
            Ok(e) => e,
            Err(e) => {
                let mut g = self.inner.lock().unwrap();
                if g.session == session {
                    g.error = Some(e);
                    g.state = State::Idle;
                    g.session += 1;
                }
                return;
            }
        };
        let mut pending: Vec<Job> = Vec::new();
        loop {
            // Les phrases closes passent avant l'aperçu ; un aperçu périmé est sauté.
            while let Ok(job) = rx.try_recv() {
                pending.push(job);
            }
            if pending.is_empty() {
                match rx.recv() {
                    Ok(job) => pending.push(job),
                    Err(_) => return,
                }
                continue;
            }
            let pick = pending
                .iter()
                .position(|j| matches!(j, Job::Final { .. }))
                .or_else(|| pending.iter().rposition(|j| matches!(j, Job::Preview { .. })))
                .unwrap_or(0);
            let job = pending.remove(pick);
            if matches!(job, Job::Preview { .. }) {
                pending.retain(|j| !matches!(j, Job::Preview { .. }));
            }
            if !self.alive(session) {
                return;
            }
            let (language, context) = {
                let g = self.inner.lock().unwrap();
                let lang = g.config.language.clone().or_else(|| g.detected.clone());
                (lang, context_of(g.prompt.as_deref(), &g.committed()))
            };
            match job {
                Job::Final { index, audio } => {
                    let result = engine.transcribe(&audio, language.as_deref(), Some(&context), false);
                    let mut g = self.inner.lock().unwrap();
                    if g.session != session {
                        return;
                    }
                    match result {
                        Ok(t) => {
                            // La première phrase fixe la langue pour la suite.
                            if g.detected.is_none() && g.config.language.is_none() {
                                g.detected = t.language.clone();
                            }
                            g.phrases[index] = Some(keep(&t));
                            g.partial.clear();
                        }
                        Err(e) => g.error = Some(e),
                    }
                }
                Job::Preview { audio } => {
                    let result = engine.transcribe(&audio, language.as_deref(), Some(&context), true);
                    busy.store(false, Ordering::Relaxed);
                    let mut g = self.inner.lock().unwrap();
                    if g.session != session {
                        return;
                    }
                    if let Ok(t) = result
                        && g.state == State::Recording
                    {
                        g.partial = keep(&t);
                    }
                }
                Job::Done => {
                    if pending.iter().any(|j| matches!(j, Job::Final { .. })) {
                        // Des phrases restent : on les traite d'abord.
                        pending.push(Job::Done);
                        continue;
                    }
                    let mut g = self.inner.lock().unwrap();
                    if g.session != session {
                        return;
                    }
                    g.text = Some(g.committed());
                    g.seq += 1;
                    g.phrases.clear();
                    g.partial.clear();
                    g.state = State::Idle;
                    g.stop_in = None;
                    g.level = 0.0;
                    g.speaking = false;
                    g.session += 1;
                    return;
                }
            }
        }
    }

    /// Termine l'écoute ; la dernière phrase est transcrite puis le texte assemblé.
    pub fn stop(&self) {
        let mut g = self.inner.lock().unwrap();
        if g.state == State::Recording {
            g.state = State::Transcribing;
            g.started = None;
            g.stop_in = None;
        }
    }

    pub fn cancel(&self) {
        self.cancel_download.store(true, Ordering::Relaxed);
        let mut g = self.inner.lock().unwrap();
        if matches!(g.state, State::Recording | State::Transcribing) {
            g.state = State::Idle;
            g.started = None;
            g.phrases.clear();
            g.partial.clear();
            g.stop_in = None;
            g.level = 0.0;
            g.speaking = false;
            g.session += 1;
        }
        g.start_after_download = false;
    }

    /// Transcrit un audio complet (mono 16 kHz) comme une dictée : découpage
    /// par la détection de voix, phrase par phrase, avec le contexte.
    /// Renvoie le texte et les phrases (début, fin en secondes, texte).
    pub fn transcribe_samples(&self, audio: &[f32]) -> Result<(String, Vec<TimedPhrase>), String> {
        let (mut language, prompt) = {
            let g = self.inner.lock().unwrap();
            (g.config.language.clone(), g.prompt.clone())
        };
        let engine = self.engine()?;
        let mut texts: Vec<String> = Vec::new();
        let mut phrases = Vec::new();
        for phrase in vad::split(audio, VadConfig::default()) {
            let slice = &audio[phrase.start..phrase.end.min(audio.len())];
            let context = context_of(prompt.as_deref(), &join(texts.iter()));
            let t = engine.transcribe(slice, language.as_deref(), Some(&context), false)?;
            if language.is_none() {
                language = t.language.clone();
            }
            let text = keep(&t);
            let sr = SAMPLE_RATE as f32;
            phrases.push((phrase.start as f32 / sr, phrase.end as f32 / sr, text.clone()));
            texts.push(text);
        }
        Ok((join(texts.iter()), phrases))
    }
}

/// Phrase transcrite : début et fin (s), texte.
pub type TimedPhrase = (f32, f32, String);

/// Contexte donné à Whisper : vocabulaire du deck, puis ce qui vient d'être dicté.
fn context_of(prompt: Option<&str>, committed: &str) -> String {
    [prompt.unwrap_or(""), committed].iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
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
mod tests {
    use super::*;

    #[test]
    fn nettoie_les_marqueurs_de_silence() {
        assert_eq!(clean("  [BLANK_AUDIO] "), "");
        assert_eq!(clean(" Bonjour   à tous. "), "Bonjour à tous.");
    }

    #[test]
    fn reechantillonne_a_16_khz() {
        let one_second = vec![0.5f32; 48_000];
        let out = resample(&one_second, 48_000, SAMPLE_RATE);
        assert_eq!(out.len(), 16_000);
        assert!(out.iter().all(|&s| (s - 0.5).abs() < 1e-6));
    }

    #[test]
    fn reechantillonnage_par_morceaux_sans_couture() {
        let input: Vec<f32> = (0..48_000).map(|i| (i as f32 / 100.0).sin()).collect();
        let mut r = audio::Resampler::new(48_000);
        let mut out = Vec::new();
        for chunk in input.chunks(997) {
            r.push(chunk, &mut out);
        }
        let whole = resample(&input, 48_000, SAMPLE_RATE);
        assert!((out.len() as i64 - whole.len() as i64).abs() <= 1);
        assert!(out.iter().zip(&whole).all(|(a, b)| (a - b).abs() < 1e-4));
    }

    #[test]
    fn reglages_par_defaut_turbo_detection_auto_arret_seul() {
        let dir = std::env::temp_dir().join(format!("echo-stt-{}", std::process::id()));
        let stt = Stt::new(&dir);
        let s = stt.status();
        assert_eq!(s.config.model, Model::Turbo);
        assert_eq!(s.config.language, None);
        assert!(s.config.auto_stop);
        assert_eq!(s.models.len(), 3);
        stt.configure(Config { model: Model::Large, language: Some("fr".into()), auto_stop: false, silence: 2.0 });
        let back = Stt::new(&dir).status().config;
        assert_eq!(back.model, Model::Large);
        assert!(!back.auto_stop);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn requete_de_reglages_partielle() {
        let r: SttRequest = serde_json::from_str(r#"{"cmd":"configure","model":"base","language":null}"#).unwrap();
        match r {
            SttRequest::Configure(c) => {
                assert_eq!(c.model, Model::Base);
                assert!(c.auto_stop);
            }
            _ => panic!(),
        }
    }
}
