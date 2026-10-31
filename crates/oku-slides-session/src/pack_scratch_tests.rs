use std::collections::BTreeMap;

use oku_slides_core::{PackFiles, PackKind};
use speculoos::prelude::*;

use super::PackScratch;
use crate::widgets_edit::parse_name;

fn name(n: &str) -> oku_slides_core::WidgetFile {
    parse_name(n).unwrap()
}

/// A widgets pack holding widget `widget` with `source`.
fn pack(widget: &str, source: &str) -> PackFiles {
    PackFiles::new(PackKind::Widgets, serde_json::Map::new(), BTreeMap::from([(name(widget), source.to_owned())]))
}

#[test]
fn take_agent_mark_after_mark_is_true_once() {
    let mut s = PackScratch::default();
    s.mark_agent(&name("carte"), "a");
    assert_that!((s.take_agent_mark(&name("carte")), s.take_agent_mark(&name("carte")))).is_equal_to((true, false));
}

#[test]
fn renamed_widget_moves_its_agent_mark() {
    let mut s = PackScratch::default();
    s.mark_agent(&name("carte"), "a");
    s.renamed(&name("carte"), &name("fiche"));
    assert_that!(s.take_agent_mark(&name("fiche"))).is_true();
}

#[test]
fn renamed_widget_mark_follows_its_renamed_source() {
    let mut s = PackScratch::default();
    s.mark_agent(&name("fiche"), "#let w = widget(\"fiche\")");
    s.renamed(&name("fiche"), &name("carte"));
    s.kept_in(&pack("carte", "#let w = widget(\"carte\")"));
    assert_that!(s.take_agent_mark(&name("carte"))).is_true();
}

#[test]
fn removed_widget_drops_its_agent_mark() {
    let mut s = PackScratch::default();
    s.mark_agent(&name("carte"), "a");
    s.removed(&name("carte"));
    assert_that!(s.take_agent_mark(&name("carte"))).is_false();
}

#[test]
fn kept_in_other_source_puts_the_mark_to_sleep() {
    let mut s = PackScratch::default();
    s.mark_agent(&name("carte"), "agent");
    s.kept_in(&pack("carte", "user"));
    assert_that!(s.take_agent_mark(&name("carte"))).is_false();
}

#[test]
fn kept_in_agent_source_again_wakes_a_taken_mark() {
    let mut s = PackScratch::default();
    s.mark_agent(&name("carte"), "agent");
    s.take_agent_mark(&name("carte"));
    s.kept_in(&pack("carte", "agent"));
    assert_that!(s.take_agent_mark(&name("carte"))).is_true();
}

#[test]
fn kept_in_missing_widget_drops_its_mark() {
    let mut s = PackScratch::default();
    s.mark_agent(&name("fiche"), "agent");
    s.kept_in(&pack("carte", "agent"));
    s.kept_in(&pack("fiche", "agent"));
    assert_that!(s.take_agent_mark(&name("fiche"))).is_false();
}
