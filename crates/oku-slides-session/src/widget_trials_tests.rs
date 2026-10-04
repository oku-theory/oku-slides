use serde_json::json;
use speculoos::prelude::*;

use super::TemplateScratch;
use crate::widgets_edit::parse_name;

fn name(n: &str) -> oku_slides_core::WidgetFile {
    parse_name(n).unwrap()
}

fn with_title() -> TemplateScratch {
    let mut s = TemplateScratch::default();
    s.set_trial(&name("carte"), "title", Some(json!("x")));
    s
}

#[test]
fn set_trial_new_value_is_stored() {
    let s = with_title();
    assert_that!(s.trials_of(&name("carte")).and_then(|m| m.get("title"))).is_equal_to(Some(&json!("x")));
}

#[test]
fn set_trial_new_value_bumps_the_revision() {
    assert_that!(with_title().revision()).is_equal_to(1);
}

#[test]
fn set_trial_same_value_keeps_the_revision() {
    let mut s = with_title();
    s.set_trial(&name("carte"), "title", Some(json!("x")));
    assert_that!(s.revision()).is_equal_to(1);
}

#[test]
fn set_trial_none_drops_an_emptied_widget() {
    let mut s = with_title();
    s.set_trial(&name("carte"), "title", None);
    assert_that!(s.trials_of(&name("carte"))).is_none();
}

#[test]
fn renamed_widget_moves_its_trials() {
    let mut s = with_title();
    s.renamed(&name("carte"), &name("fiche"));
    assert_that!(s.trials_of(&name("fiche"))).is_some();
}

#[test]
fn renamed_widget_leaves_nothing_under_the_old_name() {
    let mut s = with_title();
    s.renamed(&name("carte"), &name("fiche"));
    assert_that!(s.trials_of(&name("carte"))).is_none();
}

#[test]
fn removed_widget_drops_its_trials() {
    let mut s = with_title();
    s.removed(&name("carte"));
    assert_that!(s.trials_of(&name("carte"))).is_none();
}

#[test]
fn pruned_rejected_trial_is_dropped() {
    let mut s = with_title();
    s.set_trial(&name("carte"), "old", Some(json!("y")));
    s.pruned(&name("carte"), |k, _| k == "title");
    assert_that!(s.trials_of(&name("carte")).map(|m| m.contains_key("old"))).is_equal_to(Some(false));
}

#[test]
fn pruned_keeping_everything_keeps_the_revision() {
    let mut s = with_title();
    s.pruned(&name("carte"), |_, _| true);
    assert_that!(s.revision()).is_equal_to(1);
}

#[test]
fn take_agent_mark_after_mark_is_true_once() {
    let mut s = TemplateScratch::default();
    s.mark_agent(&name("carte"));
    assert_that!((s.take_agent_mark(&name("carte")), s.take_agent_mark(&name("carte")))).is_equal_to((true, false));
}

#[test]
fn renamed_widget_moves_its_agent_mark() {
    let mut s = TemplateScratch::default();
    s.mark_agent(&name("carte"));
    s.renamed(&name("carte"), &name("fiche"));
    assert_that!(s.take_agent_mark(&name("fiche"))).is_true();
}

#[test]
fn removed_widget_drops_its_agent_mark() {
    let mut s = TemplateScratch::default();
    s.mark_agent(&name("carte"));
    s.removed(&name("carte"));
    assert_that!(s.take_agent_mark(&name("carte"))).is_false();
}
