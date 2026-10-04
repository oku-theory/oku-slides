use speculoos::prelude::*;

use super::TemplateScratch;
use crate::widgets_edit::parse_name;

fn name(n: &str) -> oku_slides_core::WidgetFile {
    parse_name(n).unwrap()
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
