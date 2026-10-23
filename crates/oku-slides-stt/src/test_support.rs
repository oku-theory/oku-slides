//! Outillage des tests unitaires de `oku-slides-stt`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use oku_slides_platform::Platform;
use oku_slides_platform::config::{Settings, SttSettings};

use crate::stream::{Availability, Event, Stream, StreamEngine};
use crate::{Status, Stt, locale};

/// Un dossier neuf, créé atomiquement sous un nom aléatoire (t-273).
///
/// Jamais celui d'un lancement précédent, même quand macOS recycle le PID
/// (nextest lance un processus par test). Supprimé au `drop`.
pub(crate) fn fresh_dir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new().prefix(prefix).tempdir().unwrap_or_else(|e| panic!("dossier de test impossible : {e}"))
}

/// Un moteur en continu factice : français seulement, rejoue `finals` à `finish`.
#[derive(Debug)]
pub(crate) struct ScriptedEngine {
    french: Mutex<Availability>,
    finals: Vec<String>,
    /// `finish` ne rend jamais `Done`.
    pub hang: bool,
    /// Le premier `push` rend une erreur du moteur.
    pub fail: bool,
    /// Délai avant les événements de `install` (laisse voir une annulation).
    pub install_delay: Duration,
    /// `install` envoie `Progress` sans fin, sans jamais `Done`.
    pub install_progress_forever: bool,
    /// Échantillons reçus ; `cancel` appelé.
    pub pushed: Arc<AtomicUsize>,
    pub cancelled: Arc<AtomicBool>,
    /// Canaux gardés ouverts par `hang`.
    kept: Arc<Mutex<Vec<mpsc::Sender<Event>>>>,
}

impl ScriptedEngine {
    pub(crate) fn new(french: Availability, finals: &[&str]) -> Self {
        Self {
            french: Mutex::new(french),
            finals: finals.iter().map(|s| (*s).to_owned()).collect(),
            hang: false,
            fail: false,
            install_delay: Duration::ZERO,
            install_progress_forever: false,
            pushed: Arc::default(),
            cancelled: Arc::default(),
            kept: Arc::default(),
        }
    }

    fn french(&self) -> std::sync::MutexGuard<'_, Availability> {
        self.french.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl StreamEngine for ScriptedEngine {
    fn supported(&self) -> bool {
        true
    }

    fn availability(&self, tag: &str) -> Availability {
        if locale::language(tag) == "fr" { *self.french() } else { Availability::Unsupported }
    }

    fn install(&self, _tag: &str) -> mpsc::Receiver<Event> {
        let (tx, rx) = mpsc::channel();
        let delay = self.install_delay;
        if self.install_progress_forever {
            // Plus vite que le tic : la réception n'expire jamais ; s'arrête quand on lâche le canal.
            std::thread::spawn(move || {
                while tx.send(Event::Progress { received: 1, total: 2 }).is_ok() {
                    std::thread::sleep(Duration::from_millis(1));
                }
            });
            return rx;
        }
        *self.french() = Availability::Ready;
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            tx.send(Event::Progress { received: 1, total: 2 }).ok();
            tx.send(Event::Progress { received: 2, total: 2 }).ok();
            tx.send(Event::Done).ok();
        });
        rx
    }

    fn open(&self, _tag: &str, _vocabulary: &[String]) -> (Box<dyn Stream>, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel();
        let stream = ScriptedStream {
            tx,
            finals: self.finals.clone(),
            hang: self.hang,
            fail: self.fail,
            pushed: self.pushed.clone(),
            cancelled: self.cancelled.clone(),
            kept: self.kept.clone(),
        };
        (Box::new(stream), rx)
    }
}

#[derive(Debug)]
struct ScriptedStream {
    tx: mpsc::Sender<Event>,
    finals: Vec<String>,
    hang: bool,
    fail: bool,
    pushed: Arc<AtomicUsize>,
    cancelled: Arc<AtomicBool>,
    kept: Arc<Mutex<Vec<mpsc::Sender<Event>>>>,
}

impl Stream for ScriptedStream {
    fn push(&mut self, samples: &[f32]) {
        if self.pushed.fetch_add(samples.len(), Ordering::Relaxed) == 0 {
            let event = if self.fail {
                Event::Failed(crate::error::AnalyzerSnafu { reason: "panne" }.build().into())
            } else {
                Event::Partial(" bonjour ".into())
            };
            self.tx.send(event).ok();
        }
    }

    fn finish(&mut self) {
        for (i, text) in self.finals.iter().enumerate() {
            let start = i as f32;
            self.tx.send(Event::Final { text: text.clone(), start, end: start + 1.0 }).ok();
        }
        if self.hang {
            self.kept.lock().unwrap_or_else(PoisonError::into_inner).push(self.tx.clone());
        } else {
            self.tx.send(Event::Done).ok();
        }
    }

    fn cancel(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

/// Un WAV muet de `secs` secondes à 16 kHz, en guise de micro.
pub(crate) fn silent_wav(dir: &Path, secs: f32) -> PathBuf {
    let path = dir.join("micro.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec).unwrap_or_else(|e| panic!("wav : {e}"));
    for _ in 0..(secs * 16_000.0) as usize {
        writer.write_sample(0i16).unwrap_or_else(|e| panic!("wav : {e}"));
    }
    writer.finalize().unwrap_or_else(|e| panic!("wav : {e}"));
    path
}

/// Une plateforme en français, ses modèles dans `dir`, un micro muet ; `tune` ajuste la dictée.
pub(crate) fn french_platform(dir: &Path, tune: impl FnOnce(&mut SttSettings)) -> Platform {
    platform_in(dir, "fr", tune)
}

/// Une plateforme dans la langue `locale`, sinon comme [`french_platform`].
pub(crate) fn platform_in(dir: &Path, locale: &str, tune: impl FnOnce(&mut SttSettings)) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.models = Some(dir.join("models"));
    settings.i18n.locale = Some(locale.into());
    settings.stt.fake_mic = Some(silent_wav(dir, 1.0));
    tune(&mut settings.stt);
    Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"))
}

/// Attend que l'état de la dictée vérifie `done` (30 s au plus).
pub(crate) fn wait_until(stt: &Stt, what: &str, done: impl Fn(&Status) -> bool) -> Status {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let status = stt.status();
        if done(&status) {
            return status;
        }
        assert!(Instant::now() < deadline, "attente trop longue : {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Requête reçue par la fausse API : en-tête Authorization et corps.
pub(crate) struct ApiCall {
    pub authorization: String,
    pub body: Vec<u8>,
}

/// Une fausse API : répond `status` et `body` à chaque requête ; rend son URL racine.
pub(crate) fn fake_api(status: u16, body: &'static str) -> (String, mpsc::Receiver<ApiCall>) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap_or_else(|e| panic!("fausse API : {e}"));
    let addr = server.server_addr().to_ip().unwrap_or_else(|| panic!("fausse API : adresse non IP"));
    let url = format!("http://{addr}/v1");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for mut request in server.incoming_requests() {
            let authorization = request
                .headers()
                .iter()
                .find(|h| h.field.equiv("Authorization"))
                .map(|h| h.value.as_str().to_owned())
                .unwrap_or_default();
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(request.as_reader(), &mut bytes).ok();
            tx.send(ApiCall { authorization, body: bytes }).ok();
            request.respond(tiny_http::Response::from_string(body).with_status_code(status)).ok();
        }
    });
    (url, rx)
}
