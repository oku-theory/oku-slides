//! Téléchargement en tâche de fond contre un faux dépôt local.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use oku_slides_platform::config::ModelsSettings;
use oku_slides_platform::models::{DownloadErrorKind, PinnedFile, is_complete};
use oku_slides_platform::storage::Storage;
use oku_slides_session::Settings;
use sha2::{Digest, Sha256};
use speculoos::prelude::*;

use oku_slides_i18n::Localizer;
use rstest::rstest;

use super::{DownloadState, ModelDownload, failure_text, percent};

/// Un faux dépôt : `/repo/resolve/rev/<nom>` rend le corps de `<nom>` après `delay`.
fn hub(files: HashMap<&'static str, &'static [u8]>, delay: Duration) -> String {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            std::thread::sleep(delay);
            let name = req.url().rsplit('/').next().unwrap_or_default().to_owned();
            let ok = req.url().contains("/repo/resolve/rev/");
            let resp = match files.get(name.as_str()) {
                Some(body) if ok => tiny_http::Response::from_data(body.to_vec()),
                _ => tiny_http::Response::from_data(Vec::new()).with_status_code(404),
            };
            let _ = req.respond(resp);
        }
    });
    url
}

fn settings(hub_url: String) -> ModelsSettings {
    let mut models = Settings::defaults().unwrap().models;
    models.hub_url = hub_url;
    models.download_retries = 0;
    models.download_backoff_secs = 0;
    models
}

fn sha(bytes: &[u8]) -> &'static str {
    Box::leak(hex::encode(Sha256::digest(bytes)).into_boxed_str())
}

/// Deux fichiers épinglés `a.bin` (« hello ») et `b.bin` (« world »), en tranche `'static`.
fn pins(a: &[u8], b: &[u8]) -> &'static [PinnedFile] {
    let file = |name, sha256| PinnedFile { repo: "repo", revision: "rev", name, sha256: Some(sha256) };
    Box::leak(Box::new([file("a.bin", sha(a)), file("b.bin", sha(b))]))
}

fn bodies(a: &'static [u8]) -> HashMap<&'static str, &'static [u8]> {
    HashMap::from([("a.bin", a), ("b.bin", b"world".as_slice())])
}

/// Lance le téléchargement des deux fichiers de `url` dans `dir`.
fn start(
    d: &ModelDownload,
    files: &'static [PinnedFile],
    dir: &Path,
    url: String,
    on_change: impl Fn() + Send + 'static,
) -> bool {
    d.start(files, dir.to_path_buf(), 10, settings(url), Storage::local().unwrap(), on_change)
}

/// Attend (5 s au plus) que l'état vérifie `done`, et le rend.
fn wait_for(d: &ModelDownload, done: impl Fn(DownloadState) -> bool) -> DownloadState {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let state = d.state();
        if done(state) || Instant::now() > deadline {
            return state;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn finished(s: DownloadState) -> bool {
    !matches!(s, DownloadState::Running { .. })
}

#[test]
fn start_runs_to_idle_and_writes_marker() {
    let dir = tempfile::tempdir().unwrap();
    let url = hub(bodies(b"hello"), Duration::ZERO);
    let d = ModelDownload::default();
    assert_that!(start(&d, pins(b"hello", b"world"), dir.path(), url, || {})).is_true();
    assert_that!(wait_for(&d, finished)).is_equal_to(DownloadState::Idle);
    assert_that!(is_complete(&Storage::local().unwrap(), dir.path())).is_true();
}

#[test]
fn start_twice_while_running_returns_false() {
    let dir = tempfile::tempdir().unwrap();
    let url = hub(bodies(b"hello"), Duration::from_millis(200));
    let d = ModelDownload::default();
    let files = pins(b"hello", b"world");
    assert_that!(start(&d, files, dir.path(), url.clone(), || {})).is_true();
    assert_that!(start(&d, files, dir.path(), url, || {})).is_false();
    assert_that!(wait_for(&d, finished)).is_equal_to(DownloadState::Idle);
}

#[test]
fn cancel_ends_in_failed_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let url = hub(bodies(b"hello"), Duration::from_millis(200));
    let d = ModelDownload::default();
    assert_that!(start(&d, pins(b"hello", b"world"), dir.path(), url, || {})).is_true();
    d.cancel();
    assert_that!(wait_for(&d, finished)).is_equal_to(DownloadState::Failed(DownloadErrorKind::Cancelled));
    assert_that!(is_complete(&Storage::local().unwrap(), dir.path())).is_false();
}

#[test]
fn sha_mismatch_ends_in_failed_integrity() {
    let dir = tempfile::tempdir().unwrap();
    let url = hub(bodies(b"tampered"), Duration::ZERO);
    let d = ModelDownload::default();
    assert_that!(start(&d, pins(b"hello", b"world"), dir.path(), url, || {})).is_true();
    assert_that!(wait_for(&d, finished)).is_equal_to(DownloadState::Failed(DownloadErrorKind::Integrity));
    assert_that!(is_complete(&Storage::local().unwrap(), dir.path())).is_false();
}

/// Attend (5 s au plus) que `calls` ne soit plus nul.
fn wait_calls(calls: &AtomicUsize) -> usize {
    let deadline = Instant::now() + Duration::from_secs(5);
    while calls.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    calls.load(Ordering::SeqCst)
}

#[test]
fn on_change_called_at_the_end_of_a_failure() {
    // Le contenu altéré fait échouer avant 100 % : seul l'appel de fin prévient.
    let dir = tempfile::tempdir().unwrap();
    let url = hub(HashMap::from([("a.bin", b"x".as_slice()), ("b.bin", b"world".as_slice())]), Duration::ZERO);
    let d = ModelDownload::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    // Total énorme : aucun pourcent ne change en route.
    let started = d.start(
        pins(b"hello", b"world"),
        dir.path().to_path_buf(),
        u64::MAX,
        settings(url),
        Storage::local().unwrap(),
        move || {
            counted.fetch_add(1, Ordering::SeqCst);
        },
    );
    assert_that!(started).is_true();
    assert_that!(wait_for(&d, finished)).is_equal_to(DownloadState::Failed(DownloadErrorKind::Integrity));
    assert_that!(wait_calls(&calls)).is_equal_to(1);
}

#[test]
fn on_change_called_at_least_at_the_end() {
    let dir = tempfile::tempdir().unwrap();
    let url = hub(bodies(b"hello"), Duration::ZERO);
    let d = ModelDownload::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let on_change = move || {
        counted.fetch_add(1, Ordering::SeqCst);
    };
    assert_that!(start(&d, pins(b"hello", b"world"), dir.path(), url, on_change)).is_true();
    assert_that!(wait_for(&d, finished)).is_equal_to(DownloadState::Idle);
    // L'appel de fin suit le passage à `Idle` : attendu à son tour.
    let deadline = Instant::now() + Duration::from_secs(5);
    while calls.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_that!(calls.load(Ordering::SeqCst)).is_greater_than_or_equal_to(1);
}

#[rstest]
#[case::start(0, 10, 0)]
#[case::half(5, 10, 50)]
#[case::over(20, 10, 100)]
#[case::empty_total(0, 0, 0)]
fn percent_is_bounded(#[case] received: u64, #[case] total: u64, #[case] expected: u8) {
    assert_that!(percent(received, total)).is_equal_to(expected);
}

#[rstest]
#[case::network(DownloadErrorKind::Network, "agent-model-download-failed")]
#[case::write(DownloadErrorKind::Write, "agent-model-download-failed")]
#[case::integrity(DownloadErrorKind::Integrity, "agent-model-download-integrity")]
#[case::cancelled(DownloadErrorKind::Cancelled, "agent-model-download-cancelled")]
fn failure_text_uses_its_key(#[case] kind: DownloadErrorKind, #[case] key: &str) {
    let fr = Localizer::new(Some("fr"));
    assert_that!(failure_text(kind, &fr)).is_equal_to(fr.tr(key));
}

#[test]
fn failure_text_incomplete_names_both_sizes_in_french() {
    let fr = Localizer::new(Some("fr"));
    let text = failure_text(DownloadErrorKind::Incomplete { got: 3, expected: 7 }, &fr);
    assert_that!(text.as_str()).is_equal_to(
        fr.tr_with("agent-model-download-incomplete", &[("got", 3u64.into()), ("expected", 7u64.into())]).as_str(),
    );
    assert_that!(text.as_str()).contains("octets");
}
