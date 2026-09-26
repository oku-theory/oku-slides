use speculoos::prelude::*;

use super::*;
use crate::model::{Frame, GroupLayout, SlideKind, Style};

fn group(id: &str, kind: GroupLayoutKind, children: Vec<Element>) -> Element {
    Element {
        id: id.into(),
        frame: None,
        cell: None,
        style: Style::default(),
        body: ElementBody::Group { layout: GroupLayout { kind, ..Default::default() }, children },
    }
}

fn framed(id: &str) -> Element {
    Element { frame: Some(Frame { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }), ..Element::text(id, vec![]) }
}

fn slide() -> Slide {
    let mut s = Slide::new("s", SlideKind::Content);
    s.blocks = vec![
        Element::text("b1", vec![]),
        group("g", GroupLayoutKind::Free, vec![framed("g1"), Element::text("g2", vec![])]),
    ];
    s.elements = vec![framed("e1")];
    s
}

#[test]
fn element_ids_nested_slide_yields_parents_before_children_and_blocks_before_elements() {
    let s = slide();
    let ids: Vec<&str> = s.element_ids().collect();
    assert_that!(ids).is_equal_to(vec!["b1", "g", "g1", "g2", "e1"]);
}

#[test]
fn locate_group_child_returns_its_place() {
    let s = slide();
    let g2 = s.locate("g2").unwrap();
    assert_that!((g2.zone, g2.parent, g2.index)).is_equal_to((Zone::Blocks, Some("g"), 1));
    asserting("groupe libre, mais ni cadre ni cellule").that(&(g2.in_free_layout && !g2.is_free())).is_true();
}

#[test]
fn locate_framed_child_of_free_group_is_free() {
    let s = slide();
    assert_that!(s.locate("g1").unwrap().is_free()).is_true();
}

#[test]
fn locate_top_level_block_is_not_in_free_layout() {
    let s = slide();
    assert_that!(s.locate("b1").unwrap().in_free_layout).is_false();
}

#[test]
fn locate_free_element_returns_elements_zone_without_parent() {
    let s = slide();
    let e1 = s.locate("e1").unwrap();
    assert_that!((e1.zone, e1.parent)).is_equal_to((Zone::Elements, None));
    assert_that!(e1.is_free()).is_true();
}

#[test]
fn locate_unknown_id_returns_none() {
    let s = slide();
    assert_that!(s.locate("absent")).is_none();
}
