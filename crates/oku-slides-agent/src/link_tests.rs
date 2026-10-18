use speculoos::prelude::*;

use std::time::{Duration, Instant};

use super::{FAILURE_TTL, Link, agent_fingerprint};
use crate::Launch;
use crate::error::Unusable;

#[test]
fn agent_fingerprint_hides_the_arguments() {
    let id = agent_fingerprint("npx", &["--api-key".into(), "secret".into()]);
    assert_that!(id.contains("secret")).is_false();
    assert_that!(id.len()).is_equal_to(64);
}

#[test]
fn agent_fingerprint_separates_command_and_arguments() {
    assert_that!(agent_fingerprint("ab", &["c".into()])).is_not_equal_to(agent_fingerprint("a", &["bc".into()]));
}

/// Un lancement de `command`, prêt sous 5 s.
fn launch(command: &str) -> Launch {
    Launch {
        command: command.to_owned(),
        args: Vec::new(),
        fallback_cwd: std::env::temp_dir(),
        ready_within: std::time::Duration::from_secs(5),
    }
}

#[tokio::test]
async fn connect_missing_command_does_not_record_not_found() {
    // `Stack::availability` le revérifie en direct : retenu, il resterait périmé une fois installé.
    let link = Link::new(launch("/nonexistent/oku-slides-agent-absent"));
    assert_that!(link.connect().await.is_err()).is_true();
    assert_that!(link.failure()).is_none();
}

#[tokio::test]
async fn connect_unexecutable_command_records_spawn() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("agent");
    // Présent mais pas exécutable : le lancement échoue autrement que « introuvable ».
    std::fs::write(&file, b"pas un programme").unwrap();
    let command = file.to_string_lossy().into_owned();
    let link = Link::new(launch(&command));
    assert_that!(link.connect().await.is_err()).is_true();
    assert_that!(link.failure()).is_equal_to(Some(Unusable::Spawn { command }));
}

#[tokio::test]
async fn switch_clears_the_recorded_failure() {
    let link = Link::new(launch("npx"));
    link.remember_at(0, Some(Unusable::NoHttp), Instant::now());
    link.switch(launch("/nonexistent/oku-slides-agent-autre"));
    assert_that!(link.failure()).is_none();
}

#[test]
fn failure_is_reported_while_fresh() {
    let link = Link::new(launch("npx"));
    link.remember_at(0, Some(Unusable::NoHttp), Instant::now());
    assert_that!(link.failure()).is_equal_to(Some(Unusable::NoHttp));
}

#[test]
fn failure_older_than_the_ttl_is_no_longer_reported() {
    let link = Link::new(launch("npx"));
    let then = Instant::now().checked_sub(FAILURE_TTL + Duration::from_secs(1)).unwrap();
    link.remember_at(0, Some(Unusable::NoHttp), then);
    assert_that!(link.failure()).is_none();
}

#[test]
fn new_link_has_no_failure() {
    assert_that!(Link::new(launch("npx")).failure()).is_none();
}
