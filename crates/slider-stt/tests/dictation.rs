//! La dictée de bout en bout, sans micro ni vrai modèle : un serveur local
//! sert les fichiers du modèle, un WAV remplace le micro.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use slider_platform::Platform;
use slider_platform::config::Settings;
use slider_platform::storage::Storage;
use slider_stt::{Model, State, Status, Stt, SttErrorKind, SttRequest};
use speculoos::prelude::*;

fn workdir(name: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("oku-slides-dictation-{name}-{}-{n}", std::process::id()))
}

/// Un serveur de modèles : chaque fichier vaut `body` ; les `fail_first`
/// premières requêtes répondent `status`.
fn hub(status: u16, fail_first: usize, body: &'static [u8]) -> String {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap_or_else(|e| panic!("serveur : {e}"));
    let url = format!("http://{}", server.server_addr());
    let seen = Arc::new(AtomicUsize::new(0));
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let n = seen.fetch_add(1, Ordering::Relaxed);
            let response = if n < fail_first {
                tiny_http::Response::from_data(Vec::new()).with_status_code(status)
            } else {
                tiny_http::Response::from_data(body.to_vec())
            };
            let _ = req.respond(response);
        }
    });
    url
}

fn platform(dir: &Path, hub_url: &str, fake_mic: Option<PathBuf>) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(dir.join("data"));
    settings.paths.models = Some(dir.join("models"));
    settings.stt.hub_url = hub_url.to_owned();
    settings.stt.download_backoff_secs = 0;
    settings.stt.fake_mic = fake_mic;
    settings.i18n.locale = Some("fr".into());
    Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"))
}

fn wait_until(stt: &Stt, what: &str, done: impl Fn(&Status) -> bool) -> Status {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let status = stt.status();
        if done(&status) {
            return status;
        }
        asserting(what).that(&(Instant::now() < deadline)).is_true();
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Une seconde de son (une sinusoïde), en WAV 16 kHz.
fn wav_bytes() -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap_or_else(|e| panic!("wav : {e}"));
        for i in 0..16_000 {
            let sample = ((i as f32 / 20.0).sin() * 8000.0) as i16;
            writer.write_sample(sample).unwrap_or_else(|e| panic!("wav : {e}"));
        }
        writer.finalize().unwrap_or_else(|e| panic!("wav : {e}"));
    }
    cursor.into_inner()
}

/// Un modèle « installé » mais illisible, et un WAV en guise de micro.
fn broken_model_and_fake_mic(dir: &Path) -> PathBuf {
    let storage = Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let models = dir.join("models");
    storage.write(&Model::Turbo.path(&models).join(".complete"), Vec::new()).unwrap_or_else(|e| panic!("{e}"));
    let wav = dir.join("voix.wav");
    storage.write(&wav, wav_bytes()).unwrap_or_else(|e| panic!("{e}"));
    wav
}

#[test]
fn download_serves_every_file_and_marks_the_model_installed() {
    let dir = workdir("download");
    let p = platform(&dir, &hub(200, 0, b"poids"), None);
    let mut last = (0, 0);
    let dest = slider_stt::download(Model::Base, &dir, &p.settings.stt, &p.storage, |got, total| {
        last = (got, total);
        true
    })
    .unwrap();
    assert_that!(Model::Base.is_installed(&p.storage, &dir)).is_true();
    assert_that!(p.storage.read(&dest.join("model.bin")).unwrap()).is_equal_to(b"poids".to_vec());
    assert_that!(last.0).is_equal_to(last.1);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn download_busy_hub_is_retried_until_it_answers() {
    let dir = workdir("retry");
    let p = platform(&dir, &hub(503, 2, b"x"), None);
    assert_that!(slider_stt::download(Model::Base, &dir, &p.settings.stt, &p.storage, |_, _| true)).is_ok();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn download_missing_file_returns_download_error() {
    let dir = workdir("404");
    let p = platform(&dir, &hub(404, usize::MAX, b""), None);
    let err = slider_stt::download(Model::Base, &dir, &p.settings.stt, &p.storage, |_, _| true).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Download);
    assert_that!(Model::Base.is_installed(&p.storage, &dir)).is_false();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn download_cancelled_by_progress_leaves_no_partial_file() {
    let dir = workdir("cancel");
    let p = platform(&dir, &hub(200, 0, b"poids"), None);
    let err = slider_stt::download(Model::Base, &dir, &p.settings.stt, &p.storage, |_, _| false).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Cancelled);
    let files = p.storage.list_files(&Model::Base.path(&dir)).unwrap();
    assert_that!(files.iter().any(|f| f.extension().is_some_and(|e| e == "part"))).is_false();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn status_lists_models_with_localized_labels() {
    let dir = workdir("status");
    let stt = Stt::new(&platform(&dir, "http://127.0.0.1:1", None));
    let status = stt.handle(SttRequest::Status);
    assert_that!(status.state).is_equal_to(State::Idle);
    let labels: Vec<String> = status.models.iter().map(|m| m.label.clone()).collect();
    assert_that!(labels).contains("Base (multilingue, 148 Mo)".to_owned());
    assert_that!(status.models.iter().all(|m| !m.installed)).is_true();
}

#[test]
fn start_with_unreadable_model_reports_a_localized_error_and_goes_idle() {
    let dir = workdir("start");
    let wav = broken_model_and_fake_mic(&dir);
    let stt = Stt::new(&platform(&dir, "http://127.0.0.1:1", Some(wav)));
    let status = stt.handle(SttRequest::Start { prompt: Some("Écho".into()) });
    assert_that!(status.state).is_equal_to(State::Recording);
    let failed = wait_until(&stt, "erreur du modèle", |s| s.error.is_some());
    assert_that!(failed.error.as_deref()).is_equal_to(Some("Modèle de dictée illisible"));
    wait_until(&stt, "retour au repos", |s| s.state == State::Idle);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn start_without_model_downloads_it_then_starts_listening() {
    let dir = workdir("download-start");
    let storage = Storage::local().unwrap();
    let wav = dir.join("voix.wav");
    storage.write(&wav, wav_bytes()).unwrap();
    let stt = Stt::new(&platform(&dir, &hub(200, 0, b"pas un modele"), Some(wav)));
    stt.handle(SttRequest::Configure(slider_stt::Config { model: Model::Base, ..slider_stt::Config::default() }));
    let status = stt.handle(SttRequest::Start { prompt: None });
    assert_that!(status.state).is_equal_to(State::Downloading);
    // Téléchargé, puis l'écoute démarre ; le modèle factice ne se charge pas.
    let failed = wait_until(&stt, "téléchargement puis erreur", |s| s.error.is_some());
    let base = failed.models.iter().find(|m| m.id == Model::Base).map(|m| m.installed);
    assert_that!(base).is_equal_to(Some(true));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn stop_and_cancel_while_idle_change_nothing() {
    let dir = workdir("idle");
    let stt = Stt::new(&platform(&dir, "http://127.0.0.1:1", None));
    assert_that!(stt.handle(SttRequest::Stop).state).is_equal_to(State::Idle);
    assert_that!(stt.handle(SttRequest::Cancel).state).is_equal_to(State::Idle);
}

#[test]
fn cancel_during_recording_returns_to_idle() {
    let dir = workdir("cancel-rec");
    let wav = broken_model_and_fake_mic(&dir);
    let stt = Stt::new(&platform(&dir, "http://127.0.0.1:1", Some(wav)));
    stt.handle(SttRequest::Start { prompt: None });
    assert_that!(stt.handle(SttRequest::Cancel).state).is_equal_to(State::Idle);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn transcribe_samples_with_unreadable_model_returns_model_error() {
    let dir = workdir("samples");
    broken_model_and_fake_mic(&dir);
    let stt = Stt::new(&platform(&dir, "http://127.0.0.1:1", None));
    let err = stt.transcribe_samples(&[0.0; 1600]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
    std::fs::remove_dir_all(&dir).ok();
}
