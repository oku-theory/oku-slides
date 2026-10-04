//! Changement de grille d'une slide (`set_grid`).
//!
//! Les cellules des éléments des slots libres gardent leurs proportions ;
//! les cadres ne bougent pas.

use serde_json::{Map, Value};

use super::{GRID_TRACKS, Target, set_param, slide_mut, slide_ref};
use crate::error::{InvalidValueSnafu, OpError};
use crate::{Cell, Deck, Grid, ParamType, SlotMode, WidgetRegistry};

/// Nouvelle place de `cell` quand la grille passe de `from` à `to`.
///
/// Arrondi au plus proche (½ vers le haut) ; toujours dans la grille.
pub(crate) fn rescale_cell(cell: Cell, from: &Grid, to: &Grid) -> Cell {
    let (col, cols) = fit(cell.col, cell.cols, from.columns, to.columns);
    let (row, rows) = fit(cell.row, cell.rows, from.rows, to.rows);
    Cell { col, row, cols, rows }
}

/// Début et longueur d'une plage de `n` pistes ramenée à `m` pistes.
fn fit(start: u32, len: u32, n: u32, m: u32) -> (u32, u32) {
    let (n, m) = (u128::from(n.max(1)), u128::from(m.max(1)));
    // round(x · m / n) en entiers : ⌊(2xm + n) / 2n⌋ ; u128 : aucun dépassement.
    let scale = |x: u128| (2 * x * m + n) / (2 * n);
    let s = scale(u128::from(start)).min(m - 1);
    let e = scale(u128::from(start) + u128::from(len)).clamp(s + 1, m);
    // s < e ≤ m ≤ u32::MAX.
    (u32::try_from(s).unwrap_or(0), u32::try_from(e - s).unwrap_or(1))
}

/// Applique `set_grid` : la grille de `slide`, puis les cellules de ses slots libres.
pub(super) fn set_grid(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    slide: &str,
    columns: Option<u32>,
    rows: Option<u32>,
) -> Result<(), OpError> {
    let s = slide_ref(deck, slide)?;
    let from = s.resolved_grid(registry);
    let to = Grid { columns: columns.unwrap_or(from.columns), rows: rows.unwrap_or(from.rows), gap: None };
    if !GRID_TRACKS.contains(&to.columns) || !GRID_TRACKS.contains(&to.rows) {
        return Err(InvalidValueSnafu { field: "grid", reason: "1 to 24 columns and rows" }.build().into());
    }
    if (to.columns, to.rows) == (from.columns, from.rows) {
        return Ok(());
    }
    let free: Vec<String> = registry
        .get(&s.kind)
        .map(|w| {
            w.params
                .iter()
                .filter(|p| matches!(p.ty, ParamType::Slot { mode: SlotMode::Free, .. }))
                .map(|p| p.name.clone())
                .collect()
        })
        .unwrap_or_default();
    // Gouttière gardée : celle du paramètre, sinon celle du défaut déclaré.
    let mut value = match s.params.value("grid").cloned().or_else(|| s.declared_grid(registry)) {
        Some(Value::Object(m)) => m,
        _ => Map::new(),
    };
    value.insert("columns".into(), to.columns.into());
    value.insert("rows".into(), to.rows.into());
    set_param(deck, registry, &Target { slide: slide.to_owned(), element: None }, "grid", Value::Object(value))?;
    // `slots_mut` ne crée aucun slot : seuls ceux qui existent sont parcourus.
    for (name, elements) in slide_mut(deck, slide)?.params.slots_mut() {
        if !free.iter().any(|f| f == name) {
            continue;
        }
        for e in elements.iter_mut() {
            if let Some(c) = e.cell {
                e.cell = Some(rescale_cell(c, &from, &to));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "grid_tests.rs"]
mod grid_tests;
