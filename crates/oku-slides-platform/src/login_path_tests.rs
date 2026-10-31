// The fake shells are `/bin/sh` scripts.
#![cfg(unix)]

use std::ffi::{OsStr, OsString};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;

use speculoos::prelude::*;

use super::{ProgramPath, marked_path, merged, shell_path};

/// A fake shell in a fresh folder: a `/bin/sh` script running `body`.
fn fake_shell(body: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = crate::test_support::fresh_dir("oku-slides-login-path-");
    let shell = tmp.path().join("shell");
    std::fs::write(&shell, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();
    (tmp, shell)
}

#[test]
fn marked_path_skips_what_rc_files_print() {
    let out = b"Welcome!\nnvm ready\n\n__OKU_SLIDES_PATH__=/opt/homebrew/bin:/usr/bin\n";
    assert_that!(marked_path(out)).is_equal_to(Some(OsString::from("/opt/homebrew/bin:/usr/bin")));
}

#[test]
fn marked_path_without_marker_returns_none() {
    assert_that!(marked_path(b"/usr/bin:/bin\n")).is_none();
}

#[test]
fn merged_puts_login_entries_first_then_missing_own_ones() {
    let path = merged(OsStr::new("/opt/homebrew/bin:/usr/bin"), Some(OsStr::new("/usr/bin:/bin:/app/bin")));
    assert_that!(path).is_equal_to(OsString::from("/opt/homebrew/bin:/usr/bin:/bin:/app/bin"));
}

#[test]
fn merged_drops_empty_and_relative_entries() {
    let path = merged(OsStr::new("::/a:rel"), Some(OsStr::new(".:/b:")));
    assert_that!(path).is_equal_to(OsString::from("/a:/b"));
}

#[test]
fn shell_path_runs_the_shell_as_a_login_shell_and_reads_its_path() {
    // The fake shell refuses other flags than `-i -l -c`, then runs the script.
    let body = "[ \"$1 $2 $3\" = \"-i -l -c\" ] || exit 1\nPATH=/login/bin:/usr/bin\nexport PATH\n/bin/sh -c \"$4\"";
    let (_tmp, shell) = fake_shell(body);
    let found = shell_path(&shell, Duration::from_secs(5));
    assert_that!(found).is_equal_to(Some(OsString::from("/login/bin:/usr/bin")));
}

#[test]
fn shell_path_with_a_background_job_holding_the_pipe_returns_at_the_marker() {
    let body = "(sleep 5 &)\nPATH=/login/bin\nexport PATH\n/bin/sh -c \"$4\"";
    let (_tmp, shell) = fake_shell(body);
    let started = std::time::Instant::now();
    assert_that!(shell_path(&shell, Duration::from_secs(4))).is_equal_to(Some(OsString::from("/login/bin")));
    assert_that!(started.elapsed()).is_less_than(Duration::from_secs(3));
}

#[test]
fn shell_path_too_slow_returns_none() {
    let (_tmp, shell) = fake_shell("sleep 5");
    assert_that!(shell_path(&shell, Duration::from_millis(200))).is_none();
}

#[test]
fn shell_path_missing_shell_returns_none() {
    assert_that!(shell_path(&PathBuf::from("/nonexistent/shell"), Duration::from_secs(1))).is_none();
}

#[test]
fn program_path_fixed_returns_its_path() {
    let path = ProgramPath::fixed(Some(OsString::from("/a:/b")));
    assert_that!(path.get()).is_equal_to(Some(OsStr::new("/a:/b")));
}
