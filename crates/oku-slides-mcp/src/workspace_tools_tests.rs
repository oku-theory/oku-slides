use oku_slides_session::agent::TabOpen;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use oku_slides_i18n::{Localize, Localizer};

use super::{TabAction, TabArgs, WindowAction, WindowArgs, activated, change_of, tab_number};

#[rstest]
#[case("t-1", Some(1))]
#[case("t-42", Some(42))]
#[case("t-", None)]
#[case("w-1", None)]
#[case("t-1x", None)]
#[case("t-+1", None)]
#[case("t-1234567890123456789", None)]
fn tab_number_reads_only_tab_ids(#[case] id: &str, #[case] number: Option<u64>) {
    assert_that!(tab_number(id)).is_equal_to(number);
}

#[test]
fn tab_args_refuse_unknown_fields() {
    let parsed = serde_json::from_value::<TabArgs>(json!({ "action": "close", "tab": "t-2", "force": true }));
    assert_that!(parsed.is_err()).is_true();
}

#[test]
fn tab_args_read_index_and_window() {
    let parsed: TabArgs =
        serde_json::from_value(json!({ "action": "move", "tab": "t-2", "index": 0, "window": "new" }))
            .unwrap_or_else(|e| panic!("args: {e}"));
    assert_that!(parsed.action).is_equal_to(TabAction::Move);
    assert_that!(parsed.index).is_equal_to(Some(0));
    assert_that!(parsed.window.as_deref()).is_equal_to(Some("new"));
}

#[test]
fn change_of_malformed_window_is_refused_as_unknown_window() {
    let fr = Localizer::new(Some("fr"));
    let refused = change_of(TabAction::Move, None, Some("fenêtre")).err().unwrap_or_else(|| panic!("accepted"));
    assert_that!(refused.localize(&fr)).is_equal_to(fr.tr("agent-error-workspace-unknown-window"));
}

#[rstest]
#[case("new", WindowAction::New)]
#[case("close", WindowAction::Close)]
#[case("detach_agent", WindowAction::DetachAgent)]
#[case("attach_agent", WindowAction::AttachAgent)]
fn window_args_read_each_action(#[case] action: &str, #[case] expected: WindowAction) {
    let parsed: WindowArgs =
        serde_json::from_value(json!({ "action": action })).unwrap_or_else(|e| panic!("args: {e}"));
    assert_that!(parsed.action).is_equal_to(expected);
}

#[test]
fn activated_here_spends_no_move() {
    let (spent, reply) = activated("t-1", TabOpen::Here);
    assert_that!(spent).is_false();
    assert_that!(reply["tab"].as_str()).is_equal_to(Some("t-1"));
}

#[test]
fn activated_focused_spends_the_move() {
    let (spent, reply) = activated("t-1", TabOpen::Focused { tab: "t-2".into(), shown: false });
    assert_that!(spent).is_true();
    assert_that!(reply["shown"].as_bool()).is_equal_to(Some(false));
}
