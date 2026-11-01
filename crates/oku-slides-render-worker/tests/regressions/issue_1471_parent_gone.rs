//! A worker whose parent exits stops itself, even with its pipes still open (t-1471, t-341).
//!
//! The test process re-runs itself as the worker's parent (`SPAWNER`), which exits
//! at once: the worker keeps the test's stdin and stderr pipes, so only its
//! watchdog can stop it. Windows reads the parent by pid and start time, Unix by
//! `getppid`. This guards the whole path end to end; a reused pid, which no test
//! can force, is covered by `watchdog_tests::identity`.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use speculoos::prelude::*;

/// Set in the intermediate parent: it starts the worker, then exits.
const SPAWNER: &str = "OKU_SLIDES_TEST_WORKER_SPAWNER";

const TEST_NAME: &str = "issue_1471_parent_gone::worker_whose_parent_exits_stops_by_its_watchdog";

#[test]
fn worker_whose_parent_exits_stops_by_its_watchdog() {
    if std::env::var_os(SPAWNER).is_some() {
        // Inherited stdin and stderr: the outer test's pipes.
        Command::new(env!("CARGO_BIN_EXE_oku-slides-render-worker"))
            .arg("--render-worker")
            .stdout(Stdio::null())
            .spawn()
            .expect("the worker starts");
        // Unix: the worker reads its parent once its watchdog starts; leave it
        // time to, or it records PPID 1 and the test fails after its timeout.
        std::thread::sleep(Duration::from_secs(2));
        return;
    }
    let mut spawner = Command::new(std::env::current_exe().expect("test binary path"))
        .args(["--exact", TEST_NAME, "--nocapture", "--test-threads", "1"])
        .env(SPAWNER, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the spawner starts");
    let stdin = spawner.stdin.take();
    let mut stderr = spawner.stderr.take().expect("piped stderr");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        // EOF once both the spawner and the worker have exited.
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        let _ = tx.send(text);
    });
    let status = spawner.wait().expect("the spawner exits");
    assert_that!(status.success()).is_true();
    let stopped = rx.recv_timeout(Duration::from_secs(60));
    // Closing stdin ends a worker the watchdog missed.
    drop(stdin);
    let text = stopped.unwrap_or_else(|_| rx.recv_timeout(Duration::from_secs(10)).unwrap_or_default());
    assert_that!(text.as_str()).contains("stopped by its watchdog: parent process is gone");
}
