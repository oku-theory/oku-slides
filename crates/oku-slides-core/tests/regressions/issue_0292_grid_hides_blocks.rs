//! Une slide qui a une grille ne dessine que ses éléments libres : `set_grid`
//! sur une slide à blocs, ou `set_cell` sur un bloc, faisait disparaître les
//! blocs sans erreur (variantes de l'agent vides sous le titre).

use oku_slides_core::{Cell, Deck, Grid, Op, apply_all};
use speculoos::prelude::*;

fn roadmap() -> Deck {
    Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("fixture : {e}"))
}

fn grid() -> Op {
    Op::SetGrid { slide: "s-priorites".into(), grid: Some(Grid { columns: 2, rows: 1, gap: None }) }
}

#[test]
fn set_grid_on_slide_with_blocks_returns_error_and_keeps_deck() {
    let mut deck = roadmap();
    let before = deck.clone();
    assert_that!(apply_all(&mut deck, vec![grid()])).is_err();
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_cell_on_block_returns_error() {
    let mut deck = roadmap();
    let cell = Op::SetCell {
        slide: "s-priorites".into(),
        element: "c1".into(),
        cell: Some(Cell { col: 0, row: 0, cols: 1, rows: 1 }),
    };
    assert_that!(apply_all(&mut deck, vec![cell])).is_err();
}

#[test]
fn set_grid_after_blocks_moved_to_elements_is_accepted() {
    let mut deck = roadmap();
    let blocks = deck.slide("s-priorites").unwrap().blocks.clone();
    let mut ops: Vec<Op> =
        blocks.iter().map(|b| Op::RemoveElement { slide: "s-priorites".into(), element: b.id.clone() }).collect();
    ops.push(grid());
    for (i, b) in blocks.into_iter().enumerate() {
        let mut e = b;
        e.cell = Some(Cell { col: (i % 2) as u32, row: 0, cols: 1, rows: 1 });
        ops.push(Op::AddElement {
            slide: "s-priorites".into(),
            zone: oku_slides_core::Zone::Elements,
            parent: None,
            element: e,
            index: None,
        });
    }
    assert_that!(apply_all(&mut deck, ops)).is_ok();
}

#[test]
fn set_grid_none_on_slide_with_blocks_is_accepted() {
    let mut deck = roadmap();
    assert_that!(apply_all(&mut deck, vec![Op::SetGrid { slide: "s-priorites".into(), grid: None }])).is_ok();
}
