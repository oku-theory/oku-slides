//! Dictée d'Oku Slides : micro → détection de la voix → moteur local → texte.
//!
//! Deux moteurs, tous deux sur la machine :
//! - Apple (macOS 26+) : `SpeechAnalyzer` transcrit en continu, au fil de
//!   l'audio, par un pont Swift (`apple.rs`).
//! - Whisper (Turbo, Large, Base) : CTranslate2 (celui de faster-whisper),
//!   via ct2rs ; c'est le moteur ailleurs, ou sur demande.
//!
//! La voix est détectée trame par trame (earshot) : l'écoute s'arrête seule
//! après un silence. Avec Whisper, chaque phrase terminée est transcrite une
//! fois, en qualité finale, dès qu'on marque une pause ; l'aperçu ne
//! retranscrit que la phrase en cours. À l'arrêt, il ne reste donc que la
//! dernière phrase à traiter.
//!
//! Le moteur est interrogé par sondage (`status`), ce qui sert à la fois l'app
//! Tauri et le serveur de développement : pas d'événements à propager.
//! Un appui sur le micro suffit : si le modèle manque, il est téléchargé,
//! puis l'écoute commence toute seule.

#[cfg(target_os = "macos")]
mod apple;
mod audio;
mod dictation_stream;
mod dictation_whisper;
mod error;
mod locale;
mod models;
mod results;
mod stream;
#[cfg(test)]
mod test_support;
mod vad;
mod whisper;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

pub use audio::{SAMPLE_RATE, resample};
use dictation_stream::{Held, cancel_held};
use dictation_whisper::Job;
use error::LocaleUnsupportedSnafu;
pub use error::{SttError, SttErrorKind};
pub use models::{Model, download};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::Platform;
use oku_slides_platform::config::{ModelsSettings, SttSettings};
use oku_slides_platform::storage::Storage;
use stream::{Availability, Stream, StreamEngine};
use vad::Segmenter;
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

/// Version des réglages de la dictée : 1 depuis le moteur d'Apple.
const CONFIG_VERSION: u32 = 1;

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
    /// Version des réglages : 0 avant le moteur d'Apple (bascule unique).
    pub version: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self { model: Model::Turbo, language: None, auto_stop: true, silence: 1.5, version: 0 }
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

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: Model,
    pub label: String,
    pub size: u64,
    pub installed: bool,
    /// Faux : le modèle ne gère pas la langue choisie.
    pub available: bool,
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
    /// Démarre la dictée ; `vocabulary` aide la reconnaissance (titres du deck, noms propres).
    Start {
        #[serde(default)]
        vocabulary: Vec<String>,
    },
    Stop,
    Cancel,
    Download {
        model: Model,
    },
}

/// Où part l'audio d'une dictée.
enum Sink {
    /// Whisper : phrases closes et aperçus, en jobs.
    Whisper { jobs: mpsc::Sender<Job>, busy: Arc<AtomicBool> },
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
    engine: Option<(Model, Arc<Transcriber>)>,
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
                resolve_apple(&*streaming, config.language.as_deref(), platform.localizer.locale(), &system_locale)
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
        let resolved = resolve_apple(&*self.streaming, chosen.as_deref(), self.localizer.locale(), &self.system_locale);
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
    fn report(&self, e: SttError) {
        tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "dictée");
        self.state().error = Some(e.localize(&self.localizer));
    }

    fn model_info(&self, model: Model, installed: bool, g: &Inner) -> ModelInfo {
        let (installed, available) = match model {
            Model::Apple => (matches!(g.apple, Some((_, Availability::Ready))), g.apple.is_some()),
            _ => (installed, true),
        };
        ModelInfo { id: model, label: model.label(&self.localizer), size: model.size(), installed, available }
    }

    /// L'état partagé, même si un fil a paniqué en le tenant : l'interface
    /// doit toujours pouvoir le lire, et une dictée neuve le remet d'aplomb.
    fn state(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn models_dir(&self) -> &Path {
        &self.dir
    }

    /// Traite une requête et renvoie l'état.
    pub fn handle(&self, req: SttRequest) -> Status {
        match req {
            SttRequest::Status => {}
            SttRequest::Configure(config) => self.configure(config),
            SttRequest::Start { vocabulary } => self.start(vocabulary),
            SttRequest::Stop => self.stop(),
            SttRequest::Cancel => self.cancel(),
            SttRequest::Download { model } => self.download(model, false),
        }
        self.status()
    }

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
            seconds: g.started.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0),
            error: g.error.clone(),
        }
    }

    pub fn configure(&self, mut config: Config) {
        config.silence = config.silence.clamp(0.5, 10.0);
        config.version = CONFIG_VERSION;
        {
            let mut g = self.state();
            if g.state != State::Idle {
                return;
            }
            if g.engine.as_ref().is_some_and(|(m, _)| *m != config.model) {
                g.engine = None; // libère la mémoire de l'ancien modèle
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
    }

    /// Démarre la dictée (téléchargement du modèle d'abord s'il manque).
    pub fn start(&self, vocabulary: Vec<String>) {
        let (model, apple) = {
            let mut g = self.state();
            if g.state != State::Idle {
                return;
            }
            g.error = None;
            g.vocabulary = vocabulary;
            (g.config.model, g.apple.clone())
        };
        if model == Model::Apple {
            match apple {
                Some((_, Availability::Ready)) => self.begin_recording(),
                Some(_) => self.download(Model::Apple, true),
                None => self.report(LocaleUnsupportedSnafu { locale: self.wanted_language() }.build().into()),
            }
            return;
        }
        if model.is_installed(&self.storage, &self.dir) {
            self.begin_recording();
        } else {
            self.download(model, true);
        }
    }

    pub fn download(&self, model: Model, then_start: bool) {
        if model == Model::Apple {
            self.download_language(then_start);
            return;
        }
        {
            let mut g = self.state();
            if g.state != State::Idle || model.is_installed(&self.storage, &self.dir) {
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
                std::thread::spawn(move || worker.stream_loop(session, events, worker_held));
                Sink::Stream { stream, held }
            }
            (Model::Apple, None) => {
                // `start` l'empêche ; par prudence, pas d'écoute sans langue.
                self.state().reset();
                self.report(LocaleUnsupportedSnafu { locale: self.wanted_language() }.build().into());
                return;
            }
            _ => {
                let (jobs, rx) = mpsc::channel();
                let busy = Arc::new(AtomicBool::new(false));
                let worker = self.clone();
                let worker_busy = busy.clone();
                std::thread::spawn(move || worker.transcribe_loop(session, rx, worker_busy));
                Sink::Whisper { jobs, busy }
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

            // Aperçu de la phrase en cours (Whisper), quand le fil de transcription est libre.
            let current = seg.current();
            if let Sink::Whisper { jobs, busy } = &sink
                && let Some(p) = current
                && !busy.load(Ordering::Relaxed)
                && p.end.saturating_sub(previewed.max(p.start)) as f32 >= s.preview_step_secs * SAMPLE_RATE as f32
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

    pub fn cancel(&self) {
        self.cancel_download.store(true, Ordering::Relaxed);
        let mut g = self.state();
        if matches!(g.state, State::Recording | State::Transcribing) {
            g.reset();
        }
        g.start_after_download = false;
    }

    /// Transcrit un audio complet (mono 16 kHz) comme une dictée : découpage
    /// par la détection de voix, phrase par phrase, avec le contexte.
    /// Renvoie le texte et les phrases (début, fin en secondes, texte).
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

/// Langue du moteur d'Apple : la choisie, sinon l'interface, sinon le système.
fn resolve_apple(
    streaming: &dyn StreamEngine,
    chosen: Option<&str>,
    ui: &str,
    system: &Option<String>,
) -> Option<(String, Availability)> {
    locale::resolve(chosen, ui, system.as_deref(), |tag| streaming.availability(tag))
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
