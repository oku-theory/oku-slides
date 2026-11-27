//! Outillage des tests unitaires de `oku-slides-stt`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use oku_slides_platform::Platform;
use oku_slides_platform::config::Settings;

use crate::phrase_engine::PhraseEngine;
use crate::{Config, Model, Status, Stt, SttError, Transcript};

/// Un dossier neuf, créé atomiquement sous un nom aléatoire (t-273).
///
/// Jamais celui d'un lancement précédent, même quand macOS recycle le PID
/// (nextest lance un processus par test). Supprimé au `drop`.
pub(crate) fn fresh_dir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new().prefix(prefix).tempdir().unwrap_or_else(|e| panic!("dossier de test impossible : {e}"))
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
    for _ in 0..crate::num::samples(secs) {
        writer.write_sample(0i16).unwrap_or_else(|e| panic!("wav : {e}"));
    }
    writer.finalize().unwrap_or_else(|e| panic!("wav : {e}"));
    path
}

/// Une plateforme en français, ses modèles dans `dir`, un micro muet .
pub(crate) fn french_platform(dir: &Path) -> Platform {
    platform_in(dir, "fr")
}

/// Une plateforme dans la langue `locale`, sinon comme [`french_platform`].
pub(crate) fn platform_in(dir: &Path, locale: &str) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.models = Some(dir.join("models"));
    settings.i18n.locale = Some(locale.into());
    settings.stt.fake_mic = Some(silent_wav(dir, 1.0));
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

/// Requête reçue par la fausse API : chemin et query, en-têtes, corps.
pub(crate) struct ApiCall {
    pub url: String,
    pub authorization: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Une fausse API : répond `status` et `body` à chaque requête ; rend son URL racine.
pub(crate) fn fake_api(status: u16, body: &'static str) -> (String, mpsc::Receiver<ApiCall>) {
    fake_api_with(status, body.to_owned(), &[])
}

/// Une fausse API qui répond aussi les en-têtes `headers` à chaque requête.
pub(crate) fn fake_api_with(
    status: u16,
    body: String,
    reply_headers: &'static [(&'static str, &'static str)],
) -> (String, mpsc::Receiver<ApiCall>) {
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
            let headers =
                request.headers().iter().map(|h| (h.field.to_string(), h.value.as_str().to_owned())).collect();
            let url = request.url().to_owned();
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(request.as_reader(), &mut bytes).ok();
            tx.send(ApiCall { url, authorization, headers, body: bytes }).ok();
            let mut response = tiny_http::Response::from_string(body.as_str()).with_status_code(status);
            for (name, value) in reply_headers {
                let header = tiny_http::Header::from_bytes(name.as_bytes(), value.as_bytes())
                    .unwrap_or_else(|()| panic!("fausse API : en-tête {name} invalide"));
                response.add_header(header);
            }
            request.respond(response).ok();
        }
    });
    (url, rx)
}

/// A phrase engine answering `motN.` for its N-th final phrase, after `delay`.
#[derive(Debug, Default)]
pub(crate) struct CountingEngine {
    /// Final phrases transcribed so far.
    pub calls: AtomicUsize,
    /// Time each final phrase takes.
    pub delay: Duration,
}

impl PhraseEngine for CountingEngine {
    fn transcribe(
        &self,
        _audio: &[f32],
        _language: Option<&str>,
        _context: Option<&str>,
        quick: bool,
    ) -> Result<Transcript, SttError> {
        if quick {
            return Ok(Transcript { text: String::new(), language: Some("fr".into()), no_speech: 0.0 });
        }
        std::thread::sleep(self.delay);
        let n = self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(Transcript { text: format!("mot{n}."), language: Some("fr".into()), no_speech: 0.0 })
    }
}

/// A part of a fake microphone recording.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Part {
    /// The French speech fixture (`fixtures/stt/dictee-fr.wav`, 5.7 s).
    Speech,
    /// Silence, in seconds.
    Silence(f32),
}

/// The speech fixture, 16 kHz mono.
const SPEECH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/stt/dictee-fr.wav");

/// A WAV of `parts` at 16 kHz in `dir`, as the microphone (then silence forever).
pub(crate) fn speech_wav(dir: &Path, parts: &[Part]) -> PathBuf {
    let speech: Vec<i16> = hound::WavReader::open(SPEECH)
        .unwrap_or_else(|e| panic!("fixture : {e}"))
        .samples::<i16>()
        .map(|s| s.unwrap_or_else(|e| panic!("fixture : {e}")))
        .collect();
    let path = dir.join("voix.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec).unwrap_or_else(|e| panic!("wav : {e}"));
    for part in parts {
        let samples = match part {
            Part::Speech => speech.clone(),
            Part::Silence(secs) => vec![0; crate::num::samples(*secs)],
        };
        for s in samples {
            writer.write_sample(s).unwrap_or_else(|e| panic!("wav : {e}"));
        }
    }
    writer.finalize().unwrap_or_else(|e| panic!("wav : {e}"));
    path
}

/// Parakeet listening on `parts`, its engine faked, silence `silence` s.
pub(crate) fn listening_stt(dir: &Path, parts: &[Part], silence: f32) -> (Stt, Arc<CountingEngine>) {
    listening_stt_with(dir, parts, silence, Duration::ZERO)
}

/// [`listening_stt`] whose engine takes `delay` per final phrase.
pub(crate) fn listening_stt_with(
    dir: &Path,
    parts: &[Part],
    silence: f32,
    delay: Duration,
) -> (Stt, Arc<CountingEngine>) {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.models = Some(dir.join("models"));
    settings.i18n.locale = Some("fr".into());
    // Never the real hub: a download started by a test fails at once.
    "http://127.0.0.1:9".clone_into(&mut settings.models.hub_url);
    settings.models.download_retries = 0;
    settings.stt.fake_mic = Some(speech_wav(dir, parts));
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
    let installed = Model::Parakeet.path(&dir.join("models"));
    std::fs::create_dir_all(&installed).unwrap_or_else(|e| panic!("modèle : {e}"));
    std::fs::write(installed.join(oku_slides_platform::models::COMPLETE), b"")
        .unwrap_or_else(|e| panic!("modèle : {e}"));
    let stt = Stt::new(&platform);
    let config = Config { model: Model::Parakeet, language: Some("fr".into()), silence, ..Config::default() };
    assert!(stt.configure(config.clone()), "réglages refusés");
    let engine = Arc::new(CountingEngine { delay, ..CountingEngine::default() });
    stt.state().engine = Some((crate::dictation_phrase::engine_key(&config), engine.clone()));
    (stt, engine)
}
