//! Dictée d'Écho : micro → Whisper en local → texte.
//!
//! Le moteur est interrogé par sondage (`status`), ce qui sert à la fois l'app
//! Tauri et le serveur de développement : pas d'événements à propager.
//! Un appui sur le micro suffit : si le modèle manque, il est téléchargé,
//! puis l'enregistrement commence tout seul.

mod audio;
pub mod engine;
pub mod models;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use engine::{Engine, WhisperCpp};
use serde::{Deserialize, Serialize};

pub use audio::{SAMPLE_RATE, resample};
pub use models::Model;

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
pub struct Config {
    pub model: Model,
    /// Code de langue (`fr`, `en`…) ou `None` pour la détection automatique.
    pub language: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self { model: Model::Turbo, language: None }
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
    /// Transcription en cours (provisoire).
    pub partial: String,
    /// Dernière transcription terminée, et son numéro (change à chaque nouvelle).
    pub text: Option<String>,
    pub seq: u64,
    /// Secondes enregistrées.
    pub seconds: f32,
    pub error: Option<String>,
}

/// Requêtes de l'interface (même forme pour Tauri et HTTP).
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum SttRequest {
    Status,
    Configure {
        model: Model,
        language: Option<String>,
    },
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

struct Inner {
    state: State,
    config: Config,
    download: Option<Download>,
    partial: String,
    text: Option<String>,
    seq: u64,
    error: Option<String>,
    started: Option<Instant>,
    prompt: Option<String>,
    start_after_download: bool,
    recorder: Option<audio::Recorder>,
    context: Option<(Model, Arc<dyn Engine>)>,
    /// Incrémenté à chaque session d'enregistrement : les fils d'une session
    /// terminée s'arrêtent d'eux-mêmes.
    session: u64,
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
                partial: String::new(),
                text: None,
                seq: 0,
                error: None,
                started: None,
                prompt: None,
                start_after_download: false,
                recorder: None,
                context: None,
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
            SttRequest::Configure { model, language } => self.configure(Config { model, language }),
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
            level: g.recorder.as_ref().map(|r| r.level()).unwrap_or(0.0),
            partial: g.partial.clone(),
            text: g.text.clone(),
            seq: g.seq,
            seconds: g.started.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0),
            error: g.error.clone(),
        }
    }

    pub fn configure(&self, config: Config) {
        let mut g = self.inner.lock().unwrap();
        if g.state != State::Idle {
            return;
        }
        if g.context.as_ref().is_some_and(|(m, _)| *m != config.model) {
            g.context = None; // libère la mémoire de l'ancien modèle
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
            g.partial.clear();
            g.started = Some(Instant::now());
            g.recorder = Some(recorder);
            g.session
        };
        // Chargement du modèle en parallèle de l'enregistrement.
        self.ensure_context_async();
        // Transcriptions provisoires pendant qu'on parle.
        let me = self.clone();
        std::thread::spawn(move || me.partial_loop(session));
    }

    fn ensure_context_async(&self) {
        let me = self.clone();
        std::thread::spawn(move || {
            let _ = me.context();
        });
    }

    /// Contexte Whisper du modèle choisi, chargé une fois.
    fn context(&self) -> Result<Arc<dyn Engine>, String> {
        let model = {
            let g = self.inner.lock().unwrap();
            if let Some((m, ctx)) = &g.context
                && *m == g.config.model
            {
                return Ok(ctx.clone());
            }
            g.config.model
        };
        static LOADING: Mutex<()> = Mutex::new(());
        let _loading = LOADING.lock().unwrap();
        if let Some((m, ctx)) = &self.inner.lock().unwrap().context
            && *m == model
        {
            return Ok(ctx.clone());
        }
        let ctx: Arc<dyn Engine> = Arc::new(WhisperCpp::load(&model.path(&self.dir))?);
        self.inner.lock().unwrap().context = Some((model, ctx.clone()));
        Ok(ctx)
    }

    fn partial_loop(&self, session: u64) {
        let interval = match self.inner.lock().unwrap().config.model {
            Model::Large => Duration::from_millis(3000),
            _ => Duration::from_millis(1200),
        };
        loop {
            std::thread::sleep(interval);
            let (captured, language, prompt) = {
                let g = self.inner.lock().unwrap();
                if g.session != session || g.state != State::Recording {
                    return;
                }
                let Some(rec) = &g.recorder else { return };
                let Some((_, _)) = &g.context else { continue };
                (rec.captured.clone(), g.config.language.clone(), g.prompt.clone())
            };
            let mut audio = audio::snapshot(&captured);
            // Les 30 dernières secondes suffisent pour l'aperçu.
            let max = SAMPLE_RATE as usize * 30;
            if audio.len() > max {
                audio.drain(..audio.len() - max);
            }
            let Ok(ctx) = self.context() else { return };
            let t = Instant::now();
            if let Ok(text) = transcribe(ctx.as_ref(), &audio, language.as_deref(), prompt.as_deref(), true) {
                let mut g = self.inner.lock().unwrap();
                if g.session == session && g.state == State::Recording {
                    g.partial = text;
                }
            }
            // Machine trop lente pour l'aperçu : on garde toute la puissance
            // pour la transcription finale.
            if t.elapsed() > Duration::from_secs(4) {
                return;
            }
        }
    }

    /// Termine l'enregistrement et lance la transcription finale.
    pub fn stop(&self) {
        let (recorder, language, prompt) = {
            let mut g = self.inner.lock().unwrap();
            if g.state != State::Recording {
                return;
            }
            g.state = State::Transcribing;
            g.started = None;
            (g.recorder.take(), g.config.language.clone(), g.prompt.clone())
        };
        let Some(recorder) = recorder else { return };
        let me = self.clone();
        std::thread::spawn(move || {
            let audio = recorder.finish();
            let result = if audio.len() < SAMPLE_RATE as usize / 3 {
                Ok(String::new())
            } else {
                me.context()
                    .and_then(|ctx| transcribe(ctx.as_ref(), &audio, language.as_deref(), prompt.as_deref(), false))
            };
            let mut g = me.inner.lock().unwrap();
            g.state = State::Idle;
            g.partial.clear();
            match result {
                Ok(text) => {
                    g.text = Some(text);
                    g.seq += 1;
                }
                Err(e) => g.error = Some(e),
            }
        });
    }

    pub fn cancel(&self) {
        self.cancel_download.store(true, Ordering::Relaxed);
        let mut g = self.inner.lock().unwrap();
        if g.state == State::Recording {
            g.recorder = None;
            g.state = State::Idle;
            g.started = None;
            g.partial.clear();
            g.session += 1;
        }
        g.start_after_download = false;
    }

    /// Transcrit un fichier audio déjà en mémoire (mono 16 kHz), avec le modèle choisi.
    pub fn transcribe_samples(&self, audio: &[f32]) -> Result<String, String> {
        let (language, prompt) = {
            let g = self.inner.lock().unwrap();
            (g.config.language.clone(), g.prompt.clone())
        };
        let ctx = self.context()?;
        transcribe(ctx.as_ref(), audio, language.as_deref(), prompt.as_deref(), false)
    }
}

/// Transcription d'un audio mono 16 kHz, nettoyée.
pub fn transcribe(
    engine: &dyn Engine,
    audio: &[f32],
    language: Option<&str>,
    prompt: Option<&str>,
    quick: bool,
) -> Result<String, String> {
    engine.transcribe(audio, language, prompt, quick).map(|t| clean(&t))
}

/// Retire les artefacts de Whisper (espaces, marqueurs de silence).
pub fn clean(text: &str) -> String {
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
    fn reglages_par_defaut_turbo_detection_auto() {
        let dir = std::env::temp_dir().join(format!("echo-stt-{}", std::process::id()));
        let stt = Stt::new(&dir);
        let s = stt.status();
        assert_eq!(s.config.model, Model::Turbo);
        assert_eq!(s.config.language, None);
        assert_eq!(s.models.len(), 3);
        stt.configure(Config { model: Model::Large, language: Some("fr".into()) });
        assert_eq!(Stt::new(&dir).status().config.model, Model::Large);
        std::fs::remove_dir_all(&dir).ok();
    }
}
