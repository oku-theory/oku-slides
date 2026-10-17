use clap::ValueEnum;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case(ImportMode::Retrofit)]
#[case(ImportMode::Hifi)]
fn import_mode_maps_to_the_session_mode_of_the_same_name(#[case] mode: ImportMode) {
    let flag = mode.to_possible_value().map(|v| v.get_name().to_owned());
    let session = serde_json::to_value(SessionMode::from(mode)).ok().and_then(|v| v.as_str().map(str::to_owned));
    assert_that!(flag).is_equal_to(session);
}

#[test]
fn import_mode_lists_every_session_mode() {
    let mapped: Vec<SessionMode> = ImportMode::value_variants().iter().map(|m| SessionMode::from(*m)).collect();
    assert_that!(mapped).is_equal_to(vec![SessionMode::Retrofit, SessionMode::Hifi]);
}
