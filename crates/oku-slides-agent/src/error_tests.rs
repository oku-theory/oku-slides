use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use snafu::IntoError;
use speculoos::prelude::*;

use super::{AgentError, NoHttpSnafu, NotFoundSnafu, SpawnSnafu, StoppedSnafu, Unusable};

fn io(kind: std::io::ErrorKind) -> std::io::Error {
    std::io::Error::from(kind)
}

#[test]
fn unusable_of_a_missing_command_is_not_found() {
    let e: AgentError = NotFoundSnafu { command: "npx" }.into_error(io(std::io::ErrorKind::NotFound)).into();
    assert_that!(e.unusable()).is_equal_to(Some(Unusable::NotFound { command: "npx".into() }));
}

#[test]
fn unusable_of_a_spawn_failure_is_spawn() {
    let e: AgentError = SpawnSnafu { command: "npx" }.into_error(io(std::io::ErrorKind::PermissionDenied)).into();
    assert_that!(e.unusable()).is_equal_to(Some(Unusable::Spawn { command: "npx".into() }));
}

#[test]
fn unusable_of_an_agent_without_http_is_no_http() {
    let e: AgentError = NoHttpSnafu.build().into();
    assert_that!(e.unusable()).is_equal_to(Some(Unusable::NoHttp));
}

#[test]
fn unusable_of_a_stopped_agent_is_none() {
    let e: AgentError = StoppedSnafu { message: "cancelled" }.build().into();
    assert_that!(e.unusable()).is_none();
}

#[rstest]
#[case::not_found(Unusable::NotFound { command: "npx".into() }, "agent-not-found")]
#[case::spawn(Unusable::Spawn { command: "npx".into() }, "agent-spawn-failed")]
#[case::no_http(Unusable::NoHttp, "agent-no-http")]
fn unusable_localize_uses_its_key(#[case] unusable: Unusable, #[case] key: &str) {
    let fr = Localizer::new(Some("fr"));
    let expected = fr.tr_with(key, &[("command", "npx".into())]);
    assert_that!(unusable.localize(&fr)).is_equal_to(expected);
}

#[test]
fn unusable_spawn_is_not_said_as_not_found() {
    let fr = Localizer::new(Some("fr"));
    let spawn = Unusable::Spawn { command: "npx".into() }.localize(&fr);
    let missing = Unusable::NotFound { command: "npx".into() }.localize(&fr);
    assert_that!(spawn).is_not_equal_to(missing);
    assert_that!(spawn.contains("npx")).is_true();
}
