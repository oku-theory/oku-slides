use rstest::rstest;
use speculoos::prelude::*;

use super::rescale_cell;
use crate::{Cell, Grid};

fn grid(columns: u32, rows: u32) -> Grid {
    Grid { columns, rows, gap: None }
}

fn cell(col: u32, row: u32, cols: u32, rows: u32) -> Cell {
    Cell { col, row, cols, rows }
}

// Valeurs de l'ancien `rescaleCell` (ui/src/grid.ts, Math.round).
#[rstest]
#[case::halving((12, 6), (6, 6), cell(6, 3, 6, 3), cell(3, 3, 3, 3))]
#[case::single_cell((12, 6), (1, 1), cell(6, 3, 6, 3), cell(0, 0, 1, 1))]
#[case::doubling((4, 2), (8, 4), cell(1, 0, 2, 1), cell(2, 0, 4, 2))]
#[case::fraction((12, 6), (5, 6), cell(5, 0, 2, 1), cell(2, 0, 1, 1))]
#[case::tie_rounds_up((4, 6), (6, 6), cell(1, 0, 1, 1), cell(2, 0, 1, 1))]
#[case::span_never_collapses((12, 6), (3, 6), cell(0, 0, 1, 1), cell(0, 0, 1, 1))]
#[case::last_track((12, 6), (6, 6), cell(11, 0, 1, 1), cell(5, 0, 1, 1))]
fn rescale_cell_valid_cell_keeps_proportions(
    #[case] from: (u32, u32),
    #[case] to: (u32, u32),
    #[case] before: Cell,
    #[case] after: Cell,
) {
    assert_that!(rescale_cell(before, &grid(from.0, from.1), &grid(to.0, to.1))).is_equal_to(after);
}

#[test]
fn rescale_cell_overflowing_cell_lands_on_the_last_track() {
    let c = rescale_cell(cell(u32::MAX, 0, 2, 1), &grid(12, 6), &grid(6, 6));
    assert_that!(c).is_equal_to(cell(5, 0, 1, 1));
}

#[test]
fn rescale_cell_from_an_empty_grid_stays_inside() {
    let c = rescale_cell(cell(0, 0, 1, 1), &grid(0, 0), &grid(6, 6));
    assert_that!(c).is_equal_to(cell(0, 0, 6, 6));
}

#[test]
fn rescale_cell_to_a_huge_grid_does_not_overflow() {
    let c = rescale_cell(cell(1, 0, 1, 1), &grid(2, 1), &grid(u32::MAX, 1));
    assert_that!(c.col.checked_add(c.cols)).is_some();
}
