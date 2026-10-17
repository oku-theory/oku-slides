use std::path::Path;
use std::process::Output;

use speculoos::prelude::*;

use super::*;

/// `/bin/bash -c script` sous le bac à sable d'un « travailleur » `/bin/bash`.
#[expect(clippy::unwrap_used, reason = "assistant de test : l'échec fait échouer le test")]
fn bash(sandbox: &Sandbox, script: &str) -> Output {
    sandbox.command().args(["-c", script]).output().unwrap()
}

#[expect(clippy::unwrap_used, reason = "assistant de test : l'échec fait échouer le test")]
fn sandbox() -> Sandbox {
    Sandbox::new(Path::new("/bin/bash"), None).unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn sandbox_write_inside_work_succeeds() {
    let s = sandbox();
    let out = bash(&s, &format!("echo x > '{}/f'", s.work().display()));
    assert_that!(out.status.success()).is_true();
    assert_that!(s.work().join("f").exists()).is_true();
}

#[test]
fn sandbox_write_outside_work_is_denied() {
    let s = sandbox();
    let outside = tempfile::tempdir().unwrap();
    let out = bash(&s, &format!("echo x > '{}/f'", outside.path().display()));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("Operation not permitted");
    assert_that!(outside.path().join("f").exists()).is_false();
}

#[test]
fn sandbox_canary_is_outside_work_and_cannot_be_created() {
    let s = sandbox();
    assert_that!(s.canary().starts_with(s.work())).is_false();
    let out = bash(&s, &format!("echo x > '{}'", s.canary().display()));
    assert_that!(stderr(&out)).contains("Operation not permitted");
}

#[test]
fn sandbox_tmpdir_is_its_work_dir() {
    let s = sandbox();
    let out = bash(&s, "printf %s \"$TMPDIR\"");
    assert_that!(String::from_utf8_lossy(&out.stdout).into_owned()).is_equal_to(format!("{}/", s.work().display()));
}

#[test]
fn sandbox_exec_of_another_program_is_denied() {
    let out = bash(&sandbox(), "/usr/bin/true");
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("Operation not permitted");
}

#[test]
fn sandbox_exec_of_the_allowed_soffice_succeeds() {
    let s = Sandbox::new(Path::new("/bin/bash"), Some(Path::new("/usr/bin/true"))).unwrap();
    let out = bash(&s, "/usr/bin/true");
    assert_that!(out.status.success()).is_true();
}

#[test]
fn sandbox_ip_connection_is_denied() {
    // Hors bac à sable : « Connection refused » ; dedans, refusé avant tout paquet.
    let out = bash(&sandbox(), "exec 3<>/dev/tcp/127.0.0.1/9");
    assert_that!(stderr(&out)).contains("Operation not permitted");
}

#[test]
fn sandbox_hardlink_from_work_to_outside_is_denied() {
    let s = Sandbox::new(Path::new("/bin/ln"), None).unwrap();
    std::fs::write(s.work().join("f"), b"x").unwrap();
    let outside = tempfile::tempdir().unwrap();
    let out = s.command().arg(s.work().join("f")).arg(outside.path().join("l")).output().unwrap();
    assert_that!(out.status.success()).is_false();
    assert_that!(outside.path().join("l").exists()).is_false();
}

#[test]
fn sandbox_osl_pipe_rule_refuses_a_regular_file() {
    let s = sandbox();
    let uid = uid();
    let path = format!("/private/tmp/OSL_PIPE_{uid}_SingleOfficeIPC_okutest912");
    let out = bash(&s, &format!("echo x > '{path}'"));
    assert_that!(stderr(&out)).contains("Operation not permitted");
    assert_that!(Path::new(&path).exists()).is_false();
}

#[test]
fn sandbox_work_dir_is_removed_with_it() {
    let s = sandbox();
    let work = s.work().to_owned();
    drop(s);
    assert_that!(work.exists()).is_false();
}
