use oku_slides_session::agent::UpdateState;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::{UpdateArgs, UpdateChoice, installable, state_json};

fn offer(installable: bool) -> UpdateState {
    UpdateState::Offer { version: "5.2.0".into(), installable }
}

#[test]
fn state_json_up_to_date_says_so() {
    assert_that!(state_json(UpdateState::UpToDate).ok()).is_equal_to(Some(json!({ "state": "up_to_date" })));
}

#[test]
fn state_json_offer_names_version_and_installable() {
    let expected = json!({ "state": "offer", "version": "5.2.0", "installable": false });
    assert_that!(state_json(offer(false)).ok()).is_equal_to(Some(expected));
}

#[test]
fn state_json_unavailable_is_the_unavailable_refusal() {
    assert_that!(state_json(UpdateState::Unavailable).err().map(|e| e.key()))
        .is_equal_to(Some("agent-error-app-unavailable"));
}

#[rstest]
#[case::not_installable(offer(false), "agent-error-app-no-offer")]
#[case::up_to_date(UpdateState::UpToDate, "agent-error-app-no-offer")]
#[case::unavailable(UpdateState::Unavailable, "agent-error-app-unavailable")]
fn installable_without_an_installable_offer_is_refused(#[case] state: UpdateState, #[case] key: &str) {
    assert_that!(installable(state).err().map(|e| e.key())).is_equal_to(Some(key));
}

#[test]
fn installable_offer_gives_its_version() {
    assert_that!(installable(offer(true)).ok()).is_equal_to(Some("5.2.0".to_owned()));
}

#[rstest]
#[case::check("check", UpdateChoice::Check)]
#[case::later("later", UpdateChoice::Later)]
#[case::install("install", UpdateChoice::Install)]
fn update_args_parse_each_action(#[case] name: &str, #[case] expected: UpdateChoice) {
    let args: UpdateArgs =
        serde_json::from_value(json!({ "action": name })).unwrap_or_else(|e| panic!("update args: {e}"));
    assert_that!(args.action).is_equal_to(expected);
}

#[test]
fn update_args_refuse_unknown_fields() {
    let parsed = serde_json::from_value::<UpdateArgs>(json!({ "action": "check", "force": true }));
    assert_that!(parsed.is_err()).is_true();
}
