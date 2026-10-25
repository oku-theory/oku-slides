use std::sync::Arc;
use std::sync::atomic::Ordering;

use speculoos::prelude::*;

use crate::stream::Availability;
use crate::test_support::{ScriptedEngine, french_platform, fresh_dir, wait_until};
use crate::{State, Stt, SttRequest};

fn stt_with(
    dir: &std::path::Path,
    engine: ScriptedEngine,
    tune: impl FnOnce(&mut oku_slides_platform::config::SttSettings),
) -> (Stt, Arc<ScriptedEngine>) {
    let engine = Arc::new(engine);
    (Stt::with_engine(&french_platform(dir, tune), engine.clone(), None), engine)
}

#[test]
fn apple_dictation_shows_partial_while_listening() {
    let dir = fresh_dir("stream-partial");
    let (stt, _) = stt_with(dir.path(), ScriptedEngine::new(Availability::Ready, &["Bonjour à tous."]), |_| {});
    stt.handle(SttRequest::Start { vocabulary: vec!["Oku".into()] });
    let live = wait_until(&stt, "aperçu", |s| s.partial == "bonjour");
    assert_that!(live.state).is_equal_to(State::Recording);
}

#[test]
fn apple_dictation_stop_returns_final_text() {
    let dir = fresh_dir("stream-stop");
    let (stt, _) = stt_with(dir.path(), ScriptedEngine::new(Availability::Ready, &["Bonjour à tous."]), |_| {});
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    wait_until(&stt, "aperçu", |s| s.partial == "bonjour");
    stt.handle(SttRequest::Stop);
    let done = wait_until(&stt, "fin", |s| s.state == State::Idle);
    assert_that!((done.text.as_deref(), done.seq)).is_equal_to((Some("Bonjour à tous."), 1));
}

#[test]
fn apple_missing_language_is_downloaded_then_listening_starts() {
    let dir = fresh_dir("stream-install");
    let (stt, _) = stt_with(dir.path(), ScriptedEngine::new(Availability::Missing, &[]), |_| {});
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    let live = wait_until(&stt, "écoute après téléchargement", |s| s.state == State::Recording);
    assert_that!(live.models[0].installed).is_true();
}

#[test]
fn apple_dictation_cancel_cancels_the_stream() {
    let dir = fresh_dir("stream-cancel");
    let (stt, engine) = stt_with(dir.path(), ScriptedEngine::new(Availability::Ready, &[]), |_| {});
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    wait_until(&stt, "audio reçu", |_| engine.pushed.load(Ordering::Relaxed) > 0);
    stt.handle(SttRequest::Cancel);
    wait_until(&stt, "flux annulé", |_| engine.cancelled.load(Ordering::Relaxed));
}

#[test]
fn apple_stop_without_done_times_out_keeping_final_text() {
    let dir = fresh_dir("stream-hang");
    let mut engine = ScriptedEngine::new(Availability::Ready, &["Début."]);
    engine.hang = true;
    let (stt, engine) = stt_with(dir.path(), engine, |s| s.apple_finish_timeout_secs = 0.3);
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    wait_until(&stt, "aperçu", |s| s.partial == "bonjour");
    stt.handle(SttRequest::Stop);
    let done = wait_until(&stt, "délai dépassé", |s| s.state == State::Idle);
    assert_that!((done.text.as_deref(), done.error.as_deref()))
        .is_equal_to((Some("Début."), Some("La transcription a échoué")));
    wait_until(&stt, "flux annulé après le délai", |_| engine.cancelled.load(Ordering::Relaxed));
}

#[test]
fn apple_cancel_while_transcribing_cancels_the_stream() {
    let dir = fresh_dir("stream-cancel-finishing");
    let mut engine = ScriptedEngine::new(Availability::Ready, &["Début."]);
    engine.hang = true;
    let (stt, engine) = stt_with(dir.path(), engine, |s| s.apple_finish_timeout_secs = 60.0);
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    wait_until(&stt, "aperçu", |s| s.partial == "bonjour");
    stt.handle(SttRequest::Stop);
    wait_until(&stt, "texte final reçu", |s| s.state == State::Transcribing && s.committed == "Début.");
    stt.handle(SttRequest::Cancel);
    wait_until(&stt, "flux annulé", |_| engine.cancelled.load(Ordering::Relaxed));
    let idle = stt.status();
    assert_that!((idle.state, idle.seq)).is_equal_to((State::Idle, 0));
}

#[test]
fn apple_engine_failure_reports_error_without_text() {
    let dir = fresh_dir("stream-fail");
    let mut engine = ScriptedEngine::new(Availability::Ready, &[]);
    engine.fail = true;
    let (stt, _) = stt_with(dir.path(), engine, |_| {});
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    let done = wait_until(&stt, "erreur", |s| s.state == State::Idle && s.error.is_some());
    assert_that!((done.error.as_deref(), done.seq)).is_equal_to((Some("La transcription a échoué"), 0));
}

#[test]
fn apple_nothing_heard_auto_stop_finishes_the_stream() {
    let dir = fresh_dir("stream-auto");
    let (stt, _) = stt_with(dir.path(), ScriptedEngine::new(Availability::Ready, &["Fin."]), |s| {
        s.nothing_heard_secs = 0.3;
    });
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    let done = wait_until(&stt, "arrêt automatique", |s| s.seq == 1);
    assert_that!(done.text.as_deref()).is_equal_to(Some("Fin."));
}

#[test]
fn apple_transcribe_samples_returns_timed_finals() {
    let dir = fresh_dir("stream-samples");
    let (stt, _) = stt_with(dir.path(), ScriptedEngine::new(Availability::Ready, &["Un.", "Deux."]), |_| {});
    let (text, phrases) = stt.transcribe_samples(&vec![0.0; 16_000]).unwrap();
    assert_that!(text).is_equal_to("Un. Deux.".to_owned());
    assert_that!(phrases[1].clone()).is_equal_to((1.0, 2.0, "Deux.".to_owned()));
}

#[test]
fn apple_transcribe_samples_installs_language_after_an_earlier_cancel() {
    let dir = fresh_dir("stream-samples-install");
    let mut engine = ScriptedEngine::new(Availability::Missing, &["Un."]);
    engine.install_delay = std::time::Duration::from_millis(200);
    let (stt, _) = stt_with(dir.path(), engine, |_| {});
    stt.handle(SttRequest::Cancel);
    let (text, _) = stt.transcribe_samples(&vec![0.0; 16_000]).unwrap();
    assert_that!(text).is_equal_to("Un.".to_owned());
}

#[test]
fn apple_transcribe_samples_timeout_cancels_the_stream() {
    let dir = fresh_dir("stream-samples-hang");
    let mut engine = ScriptedEngine::new(Availability::Ready, &["Un."]);
    engine.hang = true;
    let (stt, engine) = stt_with(dir.path(), engine, |s| s.apple_finish_timeout_secs = 0.2);
    let result = stt.transcribe_samples(&vec![0.0; 16_000]);
    assert_that!((result.is_err(), engine.cancelled.load(Ordering::Relaxed))).is_equal_to((true, true));
}

#[test]
fn apple_download_with_unknown_language_reports_error() {
    let dir = fresh_dir("stream-download-xx");
    let (stt, _) = stt_with(dir.path(), ScriptedEngine::new(Availability::Ready, &[]), |_| {});
    // Dictée au repos : le résultat de `configure` est sans objet ici.
    let _applied = stt.configure(crate::Config { language: Some("xx".into()), ..stt.status().config });
    let _started = stt.download(crate::Model::Apple, false);
    let status = stt.status();
    assert_that!((status.state, status.error.as_deref()))
        .is_equal_to((State::Idle, Some("Le moteur d'Apple ne prend pas en charge cette langue (xx)")));
}

#[test]
fn apple_download_cancel_stops_despite_steady_progress() {
    let dir = fresh_dir("stream-install-cancel");
    let mut engine = ScriptedEngine::new(Availability::Missing, &[]);
    engine.install_progress_forever = true;
    let (stt, _) = stt_with(dir.path(), engine, |_| {});
    let _started = stt.download(crate::Model::Apple, false);
    wait_until(&stt, "progression reçue", |s| s.download.as_ref().is_some_and(|d| d.received == 1));
    stt.handle(SttRequest::Cancel);
    let idle = wait_until(&stt, "téléchargement annulé", |s| s.state == State::Idle);
    assert_that!((idle.download.is_none(), idle.error.is_none(), idle.models[0].installed))
        .is_equal_to((true, true, false));
}

/// Une dictée en cours d'enregistrement.
fn recording_stt(prefix: &str) -> (Stt, tempfile::TempDir) {
    let dir = fresh_dir(prefix);
    let (stt, _) = stt_with(dir.path(), ScriptedEngine::new(Availability::Ready, &[]), |_| {});
    stt.handle(SttRequest::Start { vocabulary: Vec::new() });
    wait_until(&stt, "enregistrement", |s| s.state == State::Recording);
    (stt, dir)
}

#[test]
fn configure_while_recording_returns_false() {
    let (stt, _dir) = recording_stt("configure-recording");
    assert_that!(stt.configure(crate::Config { auto_stop: false, ..stt.status().config })).is_false();
}

#[test]
fn configure_while_recording_keeps_the_config() {
    let (stt, _dir) = recording_stt("configure-recording-keeps");
    let before = stt.status().config;
    let _applied = stt.configure(crate::Config { silence: 7.0, ..before.clone() });
    assert_that!(stt.status().config).is_equal_to(before);
}

#[test]
fn download_while_recording_returns_false() {
    let (stt, _dir) = recording_stt("download-recording");
    assert_that!(stt.download(crate::Model::Turbo, false)).is_false();
}
