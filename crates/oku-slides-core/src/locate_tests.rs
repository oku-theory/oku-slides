use crate::kinds::base;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::params::ParamValue;

fn slide(v: serde_json::Value) -> Slide {
    serde_json::from_value(v).unwrap()
}

/// Slots `elements` et `blocks` (dans le désordre du JSON), une pile
/// imbriquée dans `blocks`.
fn nested() -> Slide {
    slide(json!({
        "id": "s", "kind": "content",
        "params": {
            "title": "Titre",
            "elements": [
                { "id": "e1", "kind": "base/text", "frame": { "x": 0, "y": 0, "w": 10, "h": 10 } },
                { "id": "e2", "kind": "base/text", "cell": { "col": 0, "row": 0 } }
            ],
            "blocks": [
                { "id": "b1", "kind": "base/text" },
                { "id": "st", "kind": "base/stack", "params": { "layout": "columns", "children": [
                    { "id": "c1", "kind": "base/card" },
                    { "id": "c2", "kind": "base/card" }
                ] } },
                { "id": "b2", "kind": "base/text" }
            ]
        }
    }))
}

#[test]
fn walk_parents_before_children_and_slots_in_name_order() {
    let s = nested();
    let ids: Vec<&str> = s.element_ids().collect();
    assert_that!(ids).is_equal_to(vec!["b1", "st", "c1", "c2", "b2", "e1", "e2"]);
}

#[test]
fn walk_child_of_stack_carries_owner_slot_index_and_depth() {
    let s = nested();
    let l = s.locate("c2").unwrap();
    assert_that!(l.owner).is_equal_to(Some("st"));
    assert_that!(l.slot).is_equal_to("children");
    assert_that!(l.index).is_equal_to(1);
    assert_that!(l.depth).is_equal_to(1);
}

#[test]
fn walk_child_of_slide_slot_has_no_owner_and_depth_zero() {
    let s = nested();
    let l = s.locate("e2").unwrap();
    assert_that!(l.owner).is_none();
    assert_that!(l.slot).is_equal_to("elements");
    assert_that!(l.index).is_equal_to(1);
    assert_that!(l.depth).is_equal_to(0);
}

#[test]
fn walk_slide_without_slot_is_empty() {
    let s = slide(json!({ "id": "s", "kind": "title" }));
    assert_that!(s.walk().count()).is_equal_to(0);
}

#[test]
fn walk_over_element_params_uses_the_given_owner() {
    let s = nested();
    let stack = s.locate("st").unwrap().element;
    let found: Vec<(Option<&str>, &str)> =
        Walk::over(&stack.params, Some("st")).map(|l| (l.owner, l.element.id.as_str())).collect();
    assert_that!(found).is_equal_to(vec![(Some("st"), "c1"), (Some("st"), "c2")]);
}

#[test]
fn is_free_with_frame_is_true() {
    assert_that!(nested().locate("e1").unwrap().is_free()).is_true();
}

#[test]
fn is_free_with_cell_is_true() {
    assert_that!(nested().locate("e2").unwrap().is_free()).is_true();
}

#[test]
fn is_free_without_frame_or_cell_is_false() {
    assert_that!(nested().locate("b1").unwrap().is_free()).is_false();
}

#[test]
fn locate_absent_id_returns_none() {
    assert_that!(nested().locate("nope").is_none()).is_true();
}

#[test]
fn slot_depth_element_without_children_is_zero() {
    assert_that!(slot_depth(&Element::new("e", base::TEXT))).is_equal_to(0);
}

#[test]
fn slot_depth_chain_of_twenty_stacks_is_nineteen() {
    let mut e = Element::new("c19", base::STACK);
    for i in (0..19).rev() {
        let mut parent = Element::new(format!("c{i}"), base::STACK);
        parent.params.set("children", ParamValue::Slot(vec![e]));
        e = parent;
    }
    assert_that!(slot_depth(&e)).is_equal_to(19);
}

#[test]
fn walk_deep_chain_reports_each_depth() {
    let mut e = Element::new("c19", base::STACK);
    for i in (0..19).rev() {
        let mut parent = Element::new(format!("c{i}"), base::STACK);
        parent.params.set("children", ParamValue::Slot(vec![e]));
        e = parent;
    }
    let mut s = Slide::new("s", "blank");
    s.params.set("elements", ParamValue::Slot(vec![e]));
    assert_that!(s.locate("c19").map(|l| l.depth)).is_equal_to(Some(19));
}
