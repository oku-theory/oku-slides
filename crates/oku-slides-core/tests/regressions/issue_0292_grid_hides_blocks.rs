//! Une slide qui a une grille ne dessine que ses éléments libres : poser la
//! grille d'une slide dont le slot `flow` (`blocks`) est non vide, ou une
//! cellule sur un de ses blocs, faisait disparaître les blocs sans erreur
//! (variantes de l'agent vides sous le titre).

use oku_slides_core::{Cell, Deck, Op, OpErrorKind, SlotRef, Target, apply_all};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::registry;

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"))
}

fn priorites() -> Target {
    Target { slide: "s-priorites".into(), element: None }
}

fn grid(value: Value) -> Op {
    Op::SetParam { target: priorites(), param: "grid".into(), value }
}

#[test]
fn set_param_grid_on_slide_with_blocks_returns_invalid_value_and_keeps_deck() {
    let mut deck = roadmap();
    let before = deck.clone();
    let err = apply_all(&mut deck, vec![grid(json!({ "columns": 2, "rows": 1 }))], &registry()).unwrap_err();
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::InvalidValue);
    assert_that!(err.op_error().to_string()).contains("flow slots are laid out by the slide");
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_cell_on_block_returns_invalid_value() {
    let mut deck = roadmap();
    let cell = Op::SetCell {
        slide: "s-priorites".into(),
        element: "c1".into(),
        cell: Some(Cell { col: 0, row: 0, cols: 1, rows: 1 }),
    };
    let err = apply_all(&mut deck, vec![cell], &registry()).unwrap_err();
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::InvalidValue);
}

#[test]
fn set_param_grid_after_blocks_moved_to_elements_is_accepted() {
    let mut deck = roadmap();
    let blocks = deck
        .slide("s-priorites")
        .and_then(|s| s.params.slot("blocks"))
        .unwrap_or_else(|| panic!("blocs attendus"))
        .to_vec();
    let mut ops: Vec<Op> =
        blocks.iter().map(|b| Op::RemoveElement { slide: "s-priorites".into(), element: b.id.clone() }).collect();
    ops.push(grid(json!({ "columns": 2, "rows": 2 })));
    for (i, b) in blocks.into_iter().enumerate() {
        let mut e = b;
        e.cell = Some(Cell { col: (i % 2) as u32, row: (i / 2) as u32, cols: 1, rows: 1 });
        let slot = SlotRef { owner: priorites(), param: "elements".into() };
        ops.push(Op::AddElement { slot, index: None, defaults: false, element: e });
    }
    assert_that!(apply_all(&mut deck, ops, &registry())).is_ok();
}

#[test]
fn set_param_grid_null_on_slide_with_blocks_is_accepted() {
    let mut deck = roadmap();
    assert_that!(apply_all(&mut deck, vec![grid(json!(null))], &registry())).is_ok();
}
