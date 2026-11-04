//! Oku Slides speech: dictation (microphone → voice detection → engine → text) and OS voices.
//!
//! The OS voices that read answers aloud live in `speak/` ([`SpeechEngine`]).
//!
//! Three engines, two of them on the machine:
//! - Apple (macOS 26+): SpeechAnalyzer transcribes continuously, as the
//!   audio comes in, through a Swift bridge (`apple.rs`).
//! - Whisper (Turbo, Large, Base): CTranslate2 (the one in faster-whisper),
//!   via ct2rs; it is the engine elsewhere, or on request.
//! - OpenAI or Azure OpenAI API (`api.rs`, async-openai): sentence by sentence, without preview.
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
mod api_extra;
mod api_http;
#[cfg(target_os = "macos")]
mod apple;
mod audio;
mod capture;
mod dictation_stream;
mod dictation_whisper;
mod error;
#[cfg(feature = "e2e-fakes")]
mod fake_source;
mod listen;
mod locale;
mod microphone;
mod models;
mod num;
mod phrase_engine;
#[cfg(any(test, target_os = "macos"))]
mod results;
mod saved_config;
mod speak;
mod speaker;
mod speech_config;
mod stream;
#[cfg(test)]
mod test_support;
mod utterance;
mod vad;
mod whisper;

use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use serde::{Deserialize, Serialize};

pub use api::ApiConfig;
pub use api_check::{is_loopback, normalized_url, url_host, valid_api_model, valid_api_version, valid_key_env};
pub use api_extra::{
    ApiAccessProblem, ApiHeader, HeaderValue, LegacyQueryParam, api_access_problem, format_api_headers,
    parse_api_headers, valid_api_access, valid_api_headers,
};
pub use audio::{SAMPLE_RATE, resample};
use dictation_stream::Held;
use dictation_whisper::{EngineKey, Job, engine_key};
use error::ReceiverTakenSnafu;
pub use error::{SttError, SttErrorKind};
pub use microphone::{Microphone, ModelFiles};
pub use models::{Model, download};
use num::seconds;
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::Platform;
use oku_slides_platform::config::{ModelsSettings, SttSettings};
use oku_slides_platform::storage::Storage;
use phrase_engine::PhraseEngine;
#[cfg(feature = "e2e-fakes")]
pub use speak::fake::FakeSpeechEngine;
pub use speak::{
    NoSpeechEngine, SpeechDone, SpeechEngine, SpeechId, SpeechRate, SpeechStatus, UnavailableReason, VoiceInfo,
    system_speech_engine,
};
pub use speaker::{
    SPEECH_GUARD, SPEECH_HOLD, Speaker, SpeakerClock, Speech, SpeechConfig, SpeechEnded, SystemClock, watchdog,
};
use stream::{Availability, Stream, StreamEngine};
use utterance::{ListenClock, Phrases};
pub use utterance::{SuspendGuard, Utterance, UtteranceSeq};
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
    /// Listening hands-free: each closed utterance goes to the host.
    Listening,
}

impl State {
    /// Settings and model downloads may apply in this state.
    #[must_use]
    pub fn accepts_settings(self) -> bool {
        matches!(self, State::Idle | State::Listening)
    }
}

/// What a capture is for: one dictation, or hands-free listening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// One dictation, its text assembled at the end.
    Dictate,
    /// Listening: utterances closed on silence, until stopped.
    Listen,
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
    /// Hands-free listening chosen by the user; never started by itself at launch.
    pub hands_free: bool,
    /// Read answers aloud; `None` until set, turned on when hands-free is first turned on.
    pub read_aloud: Option<bool>,
    /// The read-aloud voice, rate and presentation rule.
    pub speech: SpeechConfig,
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
            hands_free: false,
            read_aloud: None,
            speech: SpeechConfig::default(),
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
    /// First API variable not set, by name only; `None` when all are.
    pub api_missing: Option<String>,
    /// Listening is suspended by at least one [`SuspendGuard`].
    pub suspended: bool,
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
    /// Starts hands-free listening; `vocabulary` as for `Start`.
    Listen {
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
    /// A phrase is open in the segmenter: the user is mid-sentence, pauses between words included.
    phrase_open: bool,
    /// Closed phrases not yet handed out, in order (text `None`: not transcribed yet).
    phrases: Phrases,
    partial: String,
    detected: Option<String>,
    stop_in: Option<f32>,
    text: Option<String>,
    seq: u64,
    error: Option<String>,
    started: Option<Instant>,
    /// Vocabulaire de la dictée en cours.
    vocabulary: Vec<String>,
    /// What starts once the download succeeds, if anything (a capture asked for from idle).
    start_after_download: Option<Mode>,
    /// The download paused listening: it resumes whatever the result (ruling R6).
    listen_after_download: bool,
    engine: Option<(EngineKey, Arc<dyn PhraseEngine>)>,
    /// Modèles présents sur le disque (dans l'ordre de `Model::ALL`) : lus au
    /// démarrage et après un téléchargement, pas à chaque sondage.
    installed: [bool; Model::ALL.len()],
    /// Incrémenté à chaque dictée : les fils d'une dictée abandonnée
    /// s'arrêtent d'eux-mêmes.
    session: u64,
    /// Langue du moteur d'Apple et sa disponibilité (None : aucune langue connue).
    apple: Option<(String, Availability)>,
    /// The single sender of utterances, kept across capture restarts.
    utterances: mpsc::Sender<Utterance>,
    /// Its receiver, until the host takes it ([`Stt::utterances`]).
    receiver: Option<mpsc::Receiver<Utterance>>,
    /// Live [`SuspendGuard`]s.
    suspends: u32,
    /// Changes when the utterance in progress is discarded: its late results are dropped.
    epoch: u64,
    /// Origin of the listening sample offsets, rebased at each resume.
    clock: ListenClock,
}

impl Inner {
    fn committed(&self) -> String {
        self.phrases.committed()
    }

    /// Fin d'une dictée : le texte assemblé est rendu, retour au repos.
    fn complete(&mut self) {
        self.text = Some(self.committed());
        self.seq += 1;
        self.phrases.discard();
        self.partial.clear();
        self.state = State::Idle;
        self.stop_in = None;
        self.level = 0.0;
        self.speaking = false;
        self.phrase_open = false;
        self.session += 1;
    }

    /// Before a download: stops listening, to resume it afterwards, or keeps `then` for a success.
    fn pause_for_download(&mut self, then: Option<Mode>) {
        if self.state == State::Listening {
            self.reset();
            self.listen_after_download = true;
            self.start_after_download = None;
        } else {
            self.start_after_download = then;
        }
    }

    /// Ends the current capture of a listening that goes on: the state stays `Listening`.
    ///
    /// The old capture threads see the new session and stop; nothing can start in between.
    fn restart_listening(&mut self) {
        self.session += 1;
        self.phrases.discard();
        self.partial.clear();
    }

    /// Dictée abandonnée : retour au repos, sans texte.
    fn reset(&mut self) {
        self.state = State::Idle;
        self.started = None;
        self.phrases.discard();
        self.partial.clear();
        self.stop_in = None;
        self.level = 0.0;
        self.speaking = false;
        self.phrase_open = false;
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
    /// False while an unreadable `config.json` could not be kept aside: never overwritten (t-1216).
    savable: bool,
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
        let saved_config::Loaded { config, savable } = saved_config::load(&storage, &dir);
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
        let (utterances, receiver) = mpsc::channel();
        let stt = Self {
            inner: Arc::new(Mutex::new(Inner {
                state: State::Idle,
                config,
                download: None,
                level: 0.0,
                speaking: false,
                phrase_open: false,
                phrases: Phrases::default(),
                partial: String::new(),
                detected: None,
                stop_in: None,
                text: None,
                seq: 0,
                error: None,
                started: None,
                vocabulary: Vec::new(),
                start_after_download: None,
                listen_after_download: false,
                engine: None,
                session: 0,
                installed: Model::ALL.map(|m| m.is_installed(&storage, &dir)),
                apple: None,
                utterances,
                receiver: Some(receiver),
                suspends: 0,
                epoch: 0,
                clock: ListenClock { origin: Instant::now(), stream_base: 0 },
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
            savable,
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
            SttRequest::Listen { vocabulary } => self.listen(vocabulary),
            SttRequest::Stop => self.stop(),
            SttRequest::Cancel => self.cancel(),
        }
        self.status()
    }

    /// The chosen dictation language, then the one detected in the current dictation.
    #[must_use]
    pub fn languages(&self) -> (Option<String>, Option<String>) {
        let g = self.state();
        (g.config.language.clone(), g.detected.clone())
    }

    /// The state to display: engines, sentences, microphone level, error.
    #[must_use]
    pub fn status(&self) -> Status {
        let g = self.state();
        // Utterance text stays in the host: never in the state shown.
        let listening = g.state == State::Listening;
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
            committed: if listening { String::new() } else { g.committed() },
            partial: if listening { String::new() } else { g.partial.clone() },
            detected: g.detected.clone(),
            stop_in: g.stop_in,
            text: g.text.clone(),
            seq: g.seq,
            seconds: g.started.map_or(0.0, |t| t.elapsed().as_secs_f32()),
            error: g.error.clone(),
            api_missing: g.config.api.missing_variable().map(str::to_owned),
            suspended: g.suspends > 0,
        }
    }

    /// Applies and saves the settings; false while dictation is busy.
    ///
    /// `read_aloud` and `speech` are kept from the live state: only
    /// [`Stt::configure_speech`] writes them, so a stale snapshot never reverts them.
    /// While listening, capture stops, the settings apply, then listening starts again
    /// without the utterance in progress.
    #[must_use = "faux : la dictée était occupée, rien n'a changé"]
    pub fn configure(&self, mut config: Config) -> bool {
        config.silence = config.silence.clamp(*SILENCE_RANGE.start(), *SILENCE_RANGE.end());
        config.version = CONFIG_VERSION;
        let restart = {
            let mut g = self.state();
            if !g.state.accepts_settings() {
                return false;
            }
            config.read_aloud = g.config.read_aloud;
            config.speech = g.config.speech.clone();
            let restart = (g.state == State::Listening).then(|| {
                g.restart_listening();
                g.vocabulary.clone()
            });
            if g.engine.as_ref().is_some_and(|(k, _)| *k != engine_key(&config)) {
                g.engine = None; // libère la mémoire de l'ancien modèle, ou l'ancienne API
            }
            self.save(&config);
            g.config = config;
            restart
        };
        self.refresh_apple();
        if let Some(vocabulary) = restart {
            // Still `Listening` meanwhile, so a Start in between is refused.
            self.launch(vocabulary, Mode::Listen, State::Listening);
        }
        true
    }

    /// Writes `config` to `config.json`; a failure is logged, the settings stay for this session.
    fn save(&self, config: &Config) {
        let saved = if self.savable {
            serde_json::to_vec_pretty(config).map_err(|e| e.to_string()).and_then(|json| {
                self.storage
                    .write(&saved_config::path(&self.dir), json)
                    .map_err(|e| oku_slides_platform::errors::one_line(&e))
            })
        } else {
            Err("an unreadable config.json is in the way".to_owned())
        };
        if let Err(e) = saved {
            // Les réglages restent en vigueur pour cette session.
            tracing::warn!(error = %e, "réglages de la dictée non enregistrés");
        }
    }

    fn alive(&self, session: u64) -> bool {
        self.state().session == session
    }

    /// Termine l'écoute ; la dernière phrase est transcrite puis le texte assemblé.
    ///
    /// Ends listening too, discarding the utterance in progress.
    pub fn stop(&self) {
        let mut g = self.state();
        if g.state == State::Recording {
            g.state = State::Transcribing;
            g.started = None;
            g.stop_in = None;
        } else if g.state == State::Listening {
            g.reset();
        }
    }

    /// Abandons the current dictation, without text, and cancels a download.
    pub fn cancel(&self) {
        self.cancel_download.store(true, Ordering::Relaxed);
        let mut g = self.state();
        if matches!(g.state, State::Recording | State::Transcribing | State::Listening) {
            g.reset();
        }
        g.start_after_download = None;
    }

    /// Hands out the utterance receiver once; a second call is an error (`ReceiverTaken`).
    ///
    /// # Errors
    ///
    /// `ReceiverTaken` once the receiver was handed out.
    pub fn utterances(&self) -> Result<mpsc::Receiver<Utterance>, SttError> {
        self.state().receiver.take().ok_or_else(|| ReceiverTakenSnafu.build().into())
    }

    /// True while hands-free listens and a phrase is open: the user is mid-sentence.
    ///
    /// Pauses between words keep it true; it ends when the voice detection closes the phrase.
    #[must_use]
    pub fn user_speaking(&self) -> bool {
        let g = self.state();
        g.state == State::Listening && g.phrase_open
    }

    /// Drops captured samples and the utterance in progress until the guard drops.
    pub fn suspend(&self) -> SuspendGuard {
        SuspendGuard::new(Arc::downgrade(&self.inner))
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
