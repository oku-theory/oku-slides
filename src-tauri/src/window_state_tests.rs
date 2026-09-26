use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::sleep;
use std::time::Duration;

use speculoos::prelude::*;

use super::*;

const DELAY: Duration = Duration::from_millis(40);

fn counted() -> (Arc<AtomicUsize>, Debounce) {
    let calls = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&calls);
    let debounce = Debounce::new(DELAY, move || {
        c.fetch_add(1, Ordering::SeqCst);
    });
    (calls, debounce)
}

#[test]
fn debounce_burst_of_pings_runs_the_action_once() {
    let (calls, debounce) = counted();
    for _ in 0..20 {
        debounce.ping();
        sleep(Duration::from_millis(2));
    }
    sleep(DELAY * 4);
    assert_that!(calls.load(Ordering::SeqCst)).is_equal_to(1);
}

#[test]
fn debounce_waits_for_the_quiet_before_running() {
    let (calls, debounce) = counted();
    debounce.ping();
    sleep(DELAY / 4);
    assert_that!(calls.load(Ordering::SeqCst)).is_equal_to(0);
}

#[test]
fn debounce_two_separate_bursts_run_the_action_twice() {
    let (calls, debounce) = counted();
    debounce.ping();
    sleep(DELAY * 4);
    debounce.ping();
    sleep(DELAY * 4);
    assert_that!(calls.load(Ordering::SeqCst)).is_equal_to(2);
}

#[test]
fn is_window_gone_webview_no_longer_answering_returns_true() {
    let e =
        tauri_plugin_window_state::Error::Tauri(tauri::Error::Runtime(tauri_runtime::Error::FailedToReceiveMessage));
    assert_that!(is_window_gone(&e)).is_true();
}

#[test]
fn is_window_gone_io_error_returns_false() {
    let e = tauri_plugin_window_state::Error::Io(std::io::Error::other("disque plein"));
    assert_that!(is_window_gone(&e)).is_false();
}
