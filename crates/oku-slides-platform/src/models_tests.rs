//! Téléchargement épinglé contre un faux dépôt local.
#![cfg(feature = "download")]

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use speculoos::prelude::*;

use super::{COMPLETE, DownloadErrorKind, PinnedFile, download, is_complete};
use crate::config::ModelsSettings;
use crate::storage::Storage;

/// Un faux dépôt : `/repo/resolve/rev/<nom>` rend le corps de `<nom>` ; les autres URL, 404.
fn hub(files: HashMap<&'static str, &'static [u8]>) -> String {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
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
    ModelsSettings { hub_url, download_retries: 0, download_backoff_secs: 0, read_timeout_secs: 30 }
}

fn sha(bytes: &[u8]) -> &'static str {
    Box::leak(hex::encode(Sha256::digest(bytes)).into_boxed_str())
}

fn file(name: &'static str, sha256: Option<&'static str>) -> PinnedFile {
    PinnedFile { repo: "repo", revision: "rev", name, sha256 }
}

#[test]
fn download_matching_sha_writes_file_and_marker() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::local().unwrap();
    let url = hub(HashMap::from([("a.bin", b"hello".as_slice())]));
    let files = [file("a.bin", Some(sha(b"hello")))];
    download(&files, dir.path(), 5, &settings(url), &storage, |_, _| true).unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(b"hello".to_vec());
    assert_that!(is_complete(&storage, dir.path())).is_true();
}

#[test]
fn download_sha_mismatch_deletes_file_and_fails_integrity() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::local().unwrap();
    let url = hub(HashMap::from([("a.bin", b"tampered".as_slice())]));
    let files = [file("a.bin", Some(sha(b"hello")))];
    let err = download(&files, dir.path(), 8, &settings(url), &storage, |_, _| true).unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Integrity);
    assert_that!(dir.path().join("a.bin").exists()).is_false();
    assert_that!(dir.path().join("a.bin.part").exists()).is_false();
    assert_that!(dir.path().join(COMPLETE).exists()).is_false();
}

#[test]
fn download_progress_false_cancels_and_removes_part() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::local().unwrap();
    let url = hub(HashMap::from([("a.bin", b"hello".as_slice())]));
    // Vrai avant la requête (rien reçu), faux au premier morceau : le `.part` existe alors.
    let err = download(&[file("a.bin", None)], dir.path(), 5, &settings(url), &storage, |r, _| r == 0).unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Cancelled);
    assert_that!(dir.path().join("a.bin.part").exists()).is_false();
}

#[test]
fn download_existing_file_is_kept_and_not_fetched() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::local().unwrap();
    std::fs::write(dir.path().join("a.bin"), b"hello").unwrap();
    // Le dépôt ne sert que b.bin : a.bin, déjà là, ne doit pas être redemandé.
    let url = hub(HashMap::from([("b.bin", b"world".as_slice())]));
    let mut seen = Vec::new();
    download(&[file("a.bin", None), file("b.bin", None)], dir.path(), 10, &settings(url), &storage, |r, t| {
        seen.push((r, t));
        true
    })
    .unwrap();
    assert_that!(seen.last().copied()).is_equal_to(Some((10, 10)));
}

#[test]
fn download_http_error_is_network() {
    let dir = tempfile::tempdir().unwrap();
    let url = hub(HashMap::new());
    let err = download(&[file("a.bin", None)], dir.path(), 5, &settings(url), &Storage::local().unwrap(), |_, _| true)
        .unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Network);
}

#[test]
fn is_complete_without_marker_is_false() {
    let dir = tempfile::tempdir().unwrap();
    assert_that!(is_complete(&Storage::local().unwrap(), dir.path())).is_false();
}

/// Un dépôt qui accepte les requêtes sans jamais répondre.
fn silent_hub() -> String {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    std::thread::spawn(move || {
        let mut held = Vec::new();
        for req in server.incoming_requests() {
            held.push(req);
        }
    });
    url
}

/// Un dépôt toujours occupé (503) ; compte les requêtes reçues.
fn busy_hub() -> (String, Arc<AtomicUsize>) {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let seen = Arc::new(AtomicUsize::new(0));
    let counted = seen.clone();
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            counted.fetch_add(1, Ordering::SeqCst);
            let _ = req.respond(tiny_http::Response::from_data(Vec::new()).with_status_code(503));
        }
    });
    (url, seen)
}

#[test]
fn download_silent_hub_fails_network_within_the_read_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = settings(silent_hub());
    s.read_timeout_secs = 1;
    let started = Instant::now();
    let err = download(&[file("a.bin", None)], dir.path(), 5, &s, &Storage::local().unwrap(), |_, _| true).unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Network);
    assert_that!(started.elapsed()).is_less_than(Duration::from_secs(5));
}

#[test]
fn download_cancel_during_retry_backoff_is_cancelled_quickly() {
    let dir = tempfile::tempdir().unwrap();
    let (url, seen) = busy_hub();
    let mut s = settings(url);
    s.download_retries = 3;
    s.download_backoff_secs = 5;
    // Vrai avant la première requête, faux ensuite : l'annulation tombe pendant l'attente.
    let calls = AtomicUsize::new(0);
    let started = Instant::now();
    let err = download(&[file("a.bin", None)], dir.path(), 5, &s, &Storage::local().unwrap(), |_, _| {
        calls.fetch_add(1, Ordering::SeqCst) == 0
    })
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Cancelled);
    assert_that!(started.elapsed()).is_less_than(Duration::from_secs(2));
    // Le compteur du fil du dépôt peut retarder d'un instant : attente bornée, puis une seule requête
    // (un nouvel essai demanderait 5 s de repli).
    let waited = Instant::now();
    while seen.load(Ordering::SeqCst) == 0 && waited.elapsed() < Duration::from_secs(1) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_that!(seen.load(Ordering::SeqCst)).is_equal_to(1);
}

#[test]
fn download_cancel_while_waiting_for_headers_is_cancelled_quickly() {
    let dir = tempfile::tempdir().unwrap();
    let calls = AtomicUsize::new(0);
    let started = Instant::now();
    let err =
        download(&[file("a.bin", None)], dir.path(), 5, &settings(silent_hub()), &Storage::local().unwrap(), |_, _| {
            calls.fetch_add(1, Ordering::SeqCst) == 0
        })
        .unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Cancelled);
    assert_that!(started.elapsed()).is_less_than(Duration::from_secs(2));
}

#[test]
fn agent_trusts_the_system_certificate_store() {
    let agent = super::download::agent(Duration::from_secs(5));
    let roots = agent.config().tls_config().root_certs();
    assert_that!(matches!(roots, ureq::tls::RootCerts::PlatformVerifier)).is_true();
}
