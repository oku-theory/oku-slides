//! Resuming cut model downloads against a scripted raw-TCP hub (t-1052).

use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use speculoos::prelude::*;

use super::download_hub_tests::{range_start, ranges, scripted, send, serve_cut, serve_ranged};
use super::{COMPLETE, DownloadErrorKind, PinnedFile, download};
use crate::config::ModelsSettings;
use crate::storage::Storage;

fn settings(hub_url: String, retries: u32) -> ModelsSettings {
    ModelsSettings { hub_url, download_retries: retries, download_backoff_secs: 0, read_timeout_secs: 2 }
}

fn sha(bytes: &[u8]) -> &'static str {
    Box::leak(hex::encode(Sha256::digest(bytes)).into_boxed_str())
}

fn pinned(name: &'static str, body: &[u8]) -> PinnedFile {
    PinnedFile { repo: "repo", revision: "rev", name, sha256: Some(sha(body)) }
}

fn unpinned(name: &'static str) -> PinnedFile {
    PinnedFile { repo: "repo", revision: "main", name, sha256: None }
}

const BODY: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";

#[test]
fn download_cut_mid_body_resumes_with_range_and_verifies() {
    let dir = tempfile::tempdir().unwrap();
    let (url, seen) = scripted(|n, head, mut stream| {
        if n == 0 {
            // Announces the whole body, sends 10 bytes, then drops the connection.
            send(&mut stream, "200 OK", &[format!("content-length: {}", BODY.len())], &BODY[..10]);
        } else {
            serve_ranged(BODY, head, stream);
        }
    });
    download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 1), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(BODY.to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![None, Some(10)]);
}

#[test]
fn download_cut_without_retries_keeps_part_and_reports_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    let (url, _) = scripted(|_, _, mut stream| {
        send(&mut stream, "200 OK", &[format!("content-length: {}", BODY.len())], &BODY[..10]);
    });
    let err =
        download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
            .unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Incomplete { got: 10, expected: 36 });
    assert_that!(std::fs::read(dir.path().join("a.bin.part")).unwrap()).is_equal_to(BODY[..10].to_vec());
}

#[test]
fn download_existing_part_resumes_on_the_next_run() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), &BODY[..20]).unwrap();
    let (url, seen) = scripted(|_, head, stream| serve_ranged(BODY, head, stream));
    download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(BODY.to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![Some(20)]);
}

#[test]
fn download_range_answered_200_restarts_from_zero() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), &BODY[..20]).unwrap();
    let (url, seen) = scripted(|_, _, mut stream| {
        send(&mut stream, "200 OK", &[format!("content-length: {}", BODY.len())], BODY);
    });
    download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(BODY.to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![Some(20)]);
}

#[test]
fn download_complete_part_answered_416_is_verified_and_renamed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), BODY).unwrap();
    let (url, seen) = scripted(|_, head, stream| serve_ranged(BODY, head, stream));
    download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(BODY.to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![Some(36)]);
}

#[test]
fn download_206_without_usable_range_restarts_without_spending_a_retry() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), &BODY[..20]).unwrap();
    let (url, seen) = scripted(|n, head, mut stream| {
        if n == 0 {
            send(&mut stream, "206 Partial Content", &["content-length: 16".into()], &BODY[20..]);
        } else {
            serve_ranged(BODY, head, stream);
        }
    });
    download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(BODY.to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![Some(20), None]);
}

#[test]
fn download_capped_ranges_continue_without_spending_retries() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), &BODY[..10]).unwrap();
    // A CDN that sends at most 10 bytes per answer, with a range that says so (RFC 9110).
    let (url, seen) = scripted(|_, head, mut stream| {
        let start = range_start(head).unwrap();
        let end = (start + 9).min(BODY.len() - 1);
        let headers =
            [format!("content-length: {}", end + 1 - start), format!("content-range: bytes {start}-{end}/36")];
        send(&mut stream, "206 Partial Content", &headers, &BODY[start..=end]);
    });
    download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(BODY.to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![Some(10), Some(20), Some(30)]);
}

#[test]
fn download_body_longer_than_its_range_is_refused_and_restarted() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), &BODY[..10]).unwrap();
    // A chunked 206 announcing the 26 bytes left, then sending 30.
    let (url, seen) = scripted(|n, head, mut stream| {
        if n == 0 {
            let headers = ["transfer-encoding: chunked".to_owned(), "content-range: bytes 10-35/36".to_owned()];
            send(&mut stream, "206 Partial Content", &headers, b"1e\r\nabcdefghijklmnopqrstuvwxyz0123\r\n0\r\n\r\n");
        } else {
            serve_ranged(BODY, head, stream);
        }
    });
    download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(BODY.to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![Some(10), None]);
}

#[test]
fn download_progress_starts_the_retry_count_over() {
    let body: Vec<u8> = (0..3u32 << 20).map(|i| i.to_le_bytes()[0]).collect();
    let file = pinned("big.bin", &body);
    let dir = tempfile::tempdir().unwrap();
    let cut = (1 << 20) + (1 << 18);
    let served = body.clone();
    let (url, seen) = scripted(move |_, head, stream| serve_cut(&served, cut, head, stream));
    download(&[file], dir.path(), body.len() as u64, &settings(url, 1), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(std::fs::read(dir.path().join("big.bin")).unwrap()).is_equal_to(body);
    assert_that!(seen.lock().unwrap().len()).is_equal_to(3);
}

#[test]
fn download_retry_budget_caps_failures_despite_progress() {
    let body: Vec<u8> = (0..12u32 << 20).map(|i| i.to_le_bytes()[0]).collect();
    let file = pinned("big.bin", &body);
    let dir = tempfile::tempdir().unwrap();
    let cut = (1 << 20) + (1 << 18);
    let served = body.clone();
    let (url, seen) = scripted(move |_, head, stream| serve_cut(&served, cut, head, stream));
    let err =
        download(&[file], dir.path(), body.len() as u64, &settings(url, 1), &Storage::local().unwrap(), |_, _| true)
            .unwrap_err();
    // One retry per cut, 8 failures in all (`RETRY_BUDGET` × 1), then the ninth ends it.
    assert_that!(seen.lock().unwrap().len()).is_equal_to(9);
    assert_that!(matches!(err.kind(), DownloadErrorKind::Incomplete { .. })).is_true();
}

#[test]
fn download_busy_hub_is_retried_then_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let (url, seen) = scripted(|n, head, mut stream| {
        if n == 0 {
            send(&mut stream, "503 Service Unavailable", &["content-length: 0".into()], b"");
        } else {
            serve_ranged(BODY, head, stream);
        }
    });
    download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 1), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(seen.lock().unwrap().len()).is_equal_to(2);
}

#[test]
fn download_missing_file_fails_at_once_without_retry() {
    let dir = tempfile::tempdir().unwrap();
    let (url, seen) = scripted(|_, _, mut stream| {
        send(&mut stream, "404 Not Found", &["content-length: 0".into()], b"");
    });
    let err =
        download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 3), &Storage::local().unwrap(), |_, _| true)
            .unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Network);
    assert_that!(seen.lock().unwrap().len()).is_equal_to(1);
}

#[test]
fn download_cancel_while_rehashing_deletes_the_part_before_any_request() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), &BODY[..20]).unwrap();
    let (url, seen) = scripted(|_, head, stream| serve_ranged(BODY, head, stream));
    let err =
        download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| false)
            .unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Cancelled);
    assert_that!(dir.path().join("a.bin.part").exists()).is_false();
    assert_that!(seen.lock().unwrap().len()).is_equal_to(0);
}

#[cfg(unix)]
#[test]
fn download_unreadable_part_is_fetched_again_from_zero() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let part = dir.path().join("a.bin.part");
    std::fs::write(&part, &BODY[..10]).unwrap();
    std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o000)).unwrap();
    let (url, seen) = scripted(|_, head, stream| serve_ranged(BODY, head, stream));
    download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(BODY.to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![None]);
}

#[test]
fn download_part_longer_than_file_restarts_after_416() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), [b'x'; 50]).unwrap();
    let (url, seen) = scripted(|_, head, stream| serve_ranged(BODY, head, stream));
    download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(BODY.to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![Some(50), None]);
}

#[test]
fn download_stale_part_prefix_restarts_once_from_zero() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), b"STALE-PREFIX").unwrap();
    let (url, seen) = scripted(|_, head, stream| serve_ranged(BODY, head, stream));
    download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(BODY.to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![Some(12), None]);
}

#[test]
fn download_fresh_body_with_wrong_sha_fails_integrity_without_restart() {
    let dir = tempfile::tempdir().unwrap();
    let (url, seen) = scripted(|_, head, stream| serve_ranged(b"tampered", head, stream));
    let err =
        download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
            .unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Integrity);
    assert_that!(dir.path().join("a.bin.part").exists()).is_false();
    assert_that!(dir.path().join(COMPLETE).exists()).is_false();
    assert_that!(seen.lock().unwrap().len()).is_equal_to(1);
}

#[test]
fn download_unpinned_part_restarts_without_range_while_pinned_resumes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), &BODY[..20]).unwrap();
    std::fs::write(dir.path().join("b.json.part"), b"{\"old\":").unwrap();
    let (url, seen) = scripted(|_, head, stream| {
        let body: &[u8] = if head.contains("/b.json") { b"{\"new\":1}" } else { BODY };
        serve_ranged(body, head, stream);
    });
    let files = [pinned("a.bin", BODY), unpinned("b.json")];
    download(&files, dir.path(), 45, &settings(url, 0), &Storage::local().unwrap(), |_, _| true).unwrap();
    assert_that!(std::fs::read(dir.path().join("b.json")).unwrap()).is_equal_to(b"{\"new\":1}".to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![Some(20), None]);
}

#[test]
fn download_headers_then_stalled_body_keeps_part_and_fails_network() {
    let dir = tempfile::tempdir().unwrap();
    let (url, _) = scripted(|_, _, mut stream| {
        send(&mut stream, "200 OK", &[format!("content-length: {}", BODY.len())], &BODY[..8]);
        // Holds the connection open without sending anything more.
        std::thread::sleep(Duration::from_secs(10));
    });
    let mut s = settings(url, 0);
    s.read_timeout_secs = 1;
    let started = Instant::now();
    let err =
        download(&[pinned("a.bin", BODY)], dir.path(), 36, &s, &Storage::local().unwrap(), |_, _| true).unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Network);
    assert_that!(started.elapsed()).is_less_than(Duration::from_secs(8));
    assert_that!(std::fs::read(dir.path().join("a.bin.part")).unwrap()).is_equal_to(BODY[..8].to_vec());
}

#[test]
fn download_half_open_socket_after_5_mib_returns_within_timeout_with_part() {
    const MIB5: usize = 5 << 20;
    let dir = tempfile::tempdir().unwrap();
    let (url, _) = scripted(|_, _, mut stream| {
        // Announces more than it sends, then neither sends nor closes: a half-open socket.
        send(&mut stream, "200 OK", &[format!("content-length: {}", MIB5 * 2)], &vec![7u8; MIB5]);
        std::thread::sleep(Duration::from_secs(15));
    });
    let mut s = settings(url, 0);
    s.read_timeout_secs = 1;
    let file = PinnedFile { repo: "repo", revision: "rev", name: "big.bin", sha256: Some(sha(b"never")) };
    let started = Instant::now();
    let err =
        download(&[file], dir.path(), (MIB5 * 2) as u64, &s, &Storage::local().unwrap(), |_, _| true).unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Network);
    assert_that!(started.elapsed()).is_less_than(Duration::from_secs(12));
    let kept = std::fs::metadata(dir.path().join("big.bin.part")).unwrap().len();
    assert_that!(kept).is_equal_to(MIB5 as u64);
}

#[test]
fn download_short_ranged_body_reports_whole_file_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), &BODY[..10]).unwrap();
    // Chunked 206s that end cleanly while their range announced the whole rest: 10 new bytes,
    // asked again at once, then none, which spends the (zero) retries.
    let (url, seen) = scripted(|n, _, mut stream| {
        let (range, body): (&str, &[u8]) = if n == 0 {
            ("content-range: bytes 10-35/36", b"a\r\nabcdefghij\r\n0\r\n\r\n")
        } else {
            ("content-range: bytes 20-35/36", b"0\r\n\r\n")
        };
        send(&mut stream, "206 Partial Content", &["transfer-encoding: chunked".to_owned(), range.to_owned()], body);
    });
    let file = pinned("a.bin", BODY);
    let err =
        download(&[file], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true).unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Incomplete { got: 20, expected: 36 });
    assert_that!(std::fs::read(dir.path().join("a.bin.part")).unwrap()).is_equal_to(BODY[..20].to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![Some(10), Some(20)]);
}

#[test]
fn download_range_stripping_cutting_proxy_runs_out_of_retries() {
    let dir = tempfile::tempdir().unwrap();
    // Ignores `Range`, then cuts after 10 bytes, every time: no progress past the first cut.
    let (url, seen) = scripted(|_, _, mut stream| {
        send(&mut stream, "200 OK", &[format!("content-length: {}", BODY.len())], &BODY[..10]);
    });
    let err =
        download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 3), &Storage::local().unwrap(), |_, _| true)
            .unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Incomplete { got: 10, expected: 36 });
    assert_that!(seen.lock().unwrap().len()).is_equal_to(4);
}

#[test]
fn download_redirect_keeps_the_range_header() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), &BODY[..20]).unwrap();
    let (url, seen) = scripted(|_, head, mut stream| {
        if head.contains("/resolve/") {
            send(&mut stream, "302 Found", &["location: /cdn/a.bin".into(), "content-length: 0".into()], b"");
        } else {
            serve_ranged(BODY, head, stream);
        }
    });
    download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
        .unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(BODY.to_vec());
    assert_that!(ranges(&seen)).is_equal_to(vec![Some(20), Some(20)]);
}

#[test]
fn download_cancel_during_a_resume_deletes_the_part() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.bin.part"), &BODY[..20]).unwrap();
    let (url, _) = scripted(|_, head, stream| serve_ranged(BODY, head, stream));
    // Accepts until the resumed body arrives (beyond the 20 bytes already there).
    let err =
        download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |r, _| {
            r <= 20
        })
        .unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Cancelled);
    assert_that!(dir.path().join("a.bin.part").exists()).is_false();
}

#[cfg(unix)]
#[test]
fn download_write_error_keeps_part() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let part = dir.path().join("a.bin.part");
    std::fs::write(&part, &BODY[..10]).unwrap();
    std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o444)).unwrap();
    let (url, _) = scripted(|_, head, stream| serve_ranged(BODY, head, stream));
    let err =
        download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
            .unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Write);
    assert_that!(std::fs::read(&part).unwrap()).is_equal_to(BODY[..10].to_vec());
}

#[test]
fn download_206_to_a_request_without_range_fails_as_network() {
    let dir = tempfile::tempdir().unwrap();
    let (url, _) = scripted(|_, _, mut stream| {
        send(&mut stream, "206 Partial Content", &["content-length: 36".into()], BODY);
    });
    let err =
        download(&[pinned("a.bin", BODY)], dir.path(), 36, &settings(url, 0), &Storage::local().unwrap(), |_, _| true)
            .unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Network);
}
