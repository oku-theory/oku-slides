use rstest::rstest;
use speculoos::prelude::*;

#[cfg(debug_assertions)]
use super::EngineChoice;
use super::parse_args;

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn parse_args_without_flag_is_none() {
    assert_that!(parse_args(&args(&["oku-slides", "check"]))).is_none();
}

#[cfg(debug_assertions)]
#[test]
fn parse_args_fake_engine_takes_script_and_log() {
    let a = parse_args(&args(&["x", "--local-agent", "--engine", "fake", "--script", "/s.json", "--log", "/l"]))
        .unwrap()
        .unwrap();
    assert_that!(a.engine).is_equal_to(EngineChoice::Fake);
    assert_that!(a.script.as_deref()).is_equal_to(Some(std::path::Path::new("/s.json")));
    assert_that!(a.log.as_deref()).is_equal_to(Some(std::path::Path::new("/l")));
}

#[rstest]
#[case::no_engine(&["x", "--local-agent"])]
#[case::unknown_engine(&["x", "--local-agent", "--engine", "gpt"])]
#[case::engine_without_value(&["x", "--local-agent", "--engine"])]
fn parse_args_without_known_engine_is_an_error(#[case] v: &[&str]) {
    assert_that!(parse_args(&args(v)).unwrap()).is_err();
}

#[cfg(debug_assertions)]
#[test]
fn build_engine_fake_without_script_is_no_engine() {
    let a = parse_args(&args(&["x", "--local-agent", "--engine", "fake"])).unwrap().unwrap();
    let err = super::build_engine(a).unwrap_err();
    assert_that!(matches!(err.0, crate::error::Inner::NoEngine { .. })).is_true();
}
