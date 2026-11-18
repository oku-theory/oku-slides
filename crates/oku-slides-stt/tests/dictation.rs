//! Dictation end to end, with no microphone and no real model: a local server
//! serves the model files (wrong bytes: Parakeet's are pinned), a WAV stands in for the microphone.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use oku_slides_platform::Platform;
use oku_slides_platform::config::Settings;
use oku_slides_platform::storage::Storage;
use oku_slides_stt::{Model, State, Status, Stt, SttErrorKind, SttRequest};
use speculoos::prelude::*;

mod common;
use common::workdir;

/// A model hub: every file is `body`; the first `fail_first` requests answer `status`.
fn hub(status: u16, fail_first: usize, body: &'static [u8]) -> String {
    counting_hub(status, fail_first, body).0
}

/// [`hub`], with the number of requests it received.
fn counting_hub(status: u16, fail_first: usize, body: &'static [u8]) -> (String, Arc<AtomicUsize>) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap_or_else(|e| panic!("server: {e}"));
    let url = format!("http://{}", server.server_addr());
    let seen = Arc::new(AtomicUsize::new(0));
    let counted = seen.clone();
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
    (url, counted)
}

fn platform(dir: &Path, hub_url: &str, fake_mic: Option<PathBuf>) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(dir.join("data"));
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
    settings.paths.models = Some(dir.join("models"));
    hub_url.clone_into(&mut settings.models.hub_url);
    settings.models.download_backoff_secs = 0;
    settings.stt.fake_mic = fake_mic;
    settings.i18n.locale = Some("fr".into());
    settings.stt.apple = false;
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
        for i in 0..16_000u16 {
            #[expect(clippy::cast_possible_truncation, reason = "amplitude bounded to ±8000, within i16")]
            let sample = ((f32::from(i) / 20.0).sin() * 8000.0) as i16;
            writer.write_sample(sample).unwrap_or_else(|e| panic!("wav : {e}"));
        }
        writer.finalize().unwrap_or_else(|e| panic!("wav : {e}"));
    }
    cursor.into_inner()
}

/// A model marked installed but unreadable, and a WAV as the microphone.
fn broken_model_and_fake_mic(dir: &Path) -> PathBuf {
    let storage = Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let models = dir.join("models");
    storage.write(&Model::Parakeet.path(&models).join(".complete"), Vec::new()).unwrap_or_else(|e| panic!("{e}"));
    let wav = dir.join("voix.wav");
    storage.write(&wav, wav_bytes()).unwrap_or_else(|e| panic!("{e}"));
    wav
}

#[test]
fn download_wrong_bytes_return_download_error_and_leave_model_missing() {
    let dir = workdir("download");
    let p = platform(&dir, &hub(200, 0, b"poids"), None);
    let mut calls = 0;
    let err = oku_slides_stt::download(Model::Parakeet, &dir, &p.settings.models, &p.storage, |_, _| {
        calls += 1;
        true
    })
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Download);
    assert_that!(calls).is_greater_than(0);
    assert_that!(Model::Parakeet.is_installed(&p.storage, &dir)).is_false();
}

#[test]
fn download_busy_hub_is_retried_until_it_answers() {
    let dir = workdir("retry");
    let (url, seen) = counting_hub(503, 2, b"x");
    let p = platform(&dir, &url, None);
    let err = oku_slides_stt::download(Model::Parakeet, &dir, &p.settings.models, &p.storage, |_, _| true).unwrap_err();
    // Past the two busy answers the hub serves bytes, which the pinned hash refuses.
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Download);
    assert_that!(seen.load(Ordering::Relaxed)).is_greater_than(2);
}

#[test]
fn download_missing_file_returns_download_error() {
    let dir = workdir("404");
    let p = platform(&dir, &hub(404, usize::MAX, b""), None);
    let err = oku_slides_stt::download(Model::Parakeet, &dir, &p.settings.models, &p.storage, |_, _| true).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Download);
    assert_that!(Model::Parakeet.is_installed(&p.storage, &dir)).is_false();
}

#[test]
fn download_cancelled_by_progress_leaves_no_partial_file() {
    let dir = workdir("cancel");
    let p = platform(&dir, &hub(200, 0, b"poids"), None);
    let err =
        oku_slides_stt::download(Model::Parakeet, &dir, &p.settings.models, &p.storage, |_, _| false).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Cancelled);
    let files = p.storage.list_files(&Model::Parakeet.path(&dir)).unwrap();
    assert_that!(files.iter().any(|f| f.extension().is_some_and(|e| e == "part"))).is_false();
}

#[test]
fn status_lists_models_with_localized_labels() {
    let dir = workdir("status");
    let stt = Stt::new(&platform(&dir, "http://127.0.0.1:1", None));
    let status = stt.handle(SttRequest::Status);
    assert_that!(status.state).is_equal_to(State::Idle);
    let labels: Vec<String> = status.models.iter().map(|m| m.label.clone()).collect();
    assert_that!(labels).contains("Parakeet v3 (25 langues européennes)".to_owned());
    assert_that!(status.models.iter().all(|m| !m.installed)).is_true();
}

#[test]
fn start_with_unreadable_model_reports_a_localized_error_and_goes_idle() {
    let dir = workdir("start");
    let wav = broken_model_and_fake_mic(&dir);
    let stt = Stt::new(&platform(&dir, "http://127.0.0.1:1", Some(wav)));
    let status = stt.handle(SttRequest::Start { vocabulary: vec!["Oku Slides".into()] });
    // t-404 : le fil de transcription peut échouer sur le modèle et revenir au
    // repos avant que `handle` construise son statut ; seul compte que
    // l'écoute ait démarré (pas de téléchargement, le modèle est installé).
    assert_that!(vec![State::Recording, State::Idle]).contains(status.state);
    let failed = wait_until(&stt, "erreur du modèle", |s| s.error.is_some());
    assert_that!(failed.error.as_deref()).is_equal_to(Some("Modèle de dictée illisible"));
    wait_until(&stt, "retour au repos", |s| s.state == State::Idle);
}

#[test]
fn start_without_model_downloads_it_and_reports_a_bad_file() {
    let dir = workdir("download-start");
    let storage = Storage::local().unwrap();
    let wav = dir.join("voix.wav");
    storage.write(&wav, wav_bytes()).unwrap();
    let stt = Stt::new(&platform(&dir, &hub(200, 0, b"pas un modele"), Some(wav)));
    assert_that!(stt.status().config.model).is_equal_to(Model::Parakeet);
    let status = stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    assert_that!(status.state).is_equal_to(State::Downloading);
    // The served bytes fail the pinned hash: an error, the model still missing, no listening.
    let failed = wait_until(&stt, "download then error", |s| s.error.is_some() && s.state == State::Idle);
    let parakeet = failed.models.iter().find(|m| m.id == Model::Parakeet).map(|m| m.installed);
    assert_that!(parakeet).is_equal_to(Some(false));
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
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    assert_that!(stt.handle(SttRequest::Cancel).state).is_equal_to(State::Idle);
}

#[test]
fn transcribe_samples_with_unreadable_model_returns_model_error() {
    let dir = workdir("samples");
    broken_model_and_fake_mic(&dir);
    let stt = Stt::new(&platform(&dir, "http://127.0.0.1:1", None));
    let err = stt.transcribe_samples(&[0.0; 1600]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SttErrorKind::Model);
}
