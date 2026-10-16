use std::ffi::OsString;
use std::path::Path;
use std::time::{Duration, Instant};

use speculoos::prelude::*;

use super::*;

fn sh(script: &str, within: Duration) -> Result<Finished, RunError> {
    let args: Vec<OsString> = vec!["-c".into(), script.into()];
    run(Path::new("/bin/sh"), &args, Instant::now() + within)
}

#[test]
fn run_of_a_quick_program_gives_its_status_and_output() {
    let done = sh("echo bonjour; echo oups >&2; exit 3", Duration::from_secs(10)).unwrap();
    assert_that!((done.status.code(), done.stdout, done.stderr)).is_equal_to((
        Some(3),
        b"bonjour\n".to_vec(),
        b"oups\n".to_vec(),
    ));
}

#[test]
fn run_keeps_at_most_max_capture_bytes_of_output() {
    let done = sh("head -c 200000 /dev/zero", Duration::from_secs(10)).unwrap();
    assert_that!(done.stdout.len()).is_equal_to(MAX_CAPTURE);
}

#[test]
fn run_of_a_missing_program_is_a_spawn_error() {
    let err = run(Path::new("/nonexistent/soffice"), &[], Instant::now() + Duration::from_secs(1)).unwrap_err();
    assert_that!(matches!(err, RunError::Spawn(_))).is_true();
}

#[test]
fn run_past_its_deadline_is_a_timeout_without_waiting_for_the_program() {
    let start = Instant::now();
    let err = sh("sleep 30", Duration::from_millis(300)).unwrap_err();
    assert_that!(matches!(err, RunError::Timeout)).is_true();
    assert_that!(start.elapsed()).is_less_than(Duration::from_secs(5));
}

#[test]
fn run_past_its_deadline_kills_the_children_of_the_program_too() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("pid");
    let script = format!("sleep 30 & echo $! > '{}'; wait", pid_file.display());
    let _ = sh(&script, Duration::from_millis(300));
    let pid = std::fs::read_to_string(&pid_file).unwrap().trim().to_owned();
    let alive = std::process::Command::new("/bin/kill").args(["-0", &pid]).status().unwrap().success();
    assert_that!(alive).is_false();
}
