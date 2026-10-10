//! Tableau source → options d'un widget `table` (`host(table)`).
//!
//! Le tableau du cœur n'a ni fusion, ni style de cellule, ni texte riche :
//! chaque cellule devient son texte plain ; une fusion est aplatie (le texte
//! reste dans la cellule d'ancrage, les cellules couvertes sont vides).

use oku_slides_chart::{MAX_TABLE_CELL_CHARS, MAX_TABLE_COLUMNS, MAX_TABLE_ROWS};
use serde_json::{Value, json};

use super::text::{note, plain};
use crate::report::LossKind;
use crate::source;

/// Le texte de `cell`, au plus `MAX_TABLE_CELL_CHARS` caractères ; vrai s'il est coupé.
fn cell_text(cell: &source::Cell, losses: &mut Vec<LossKind>) -> String {
    let (text, lost) = plain(&cell.text);
    for l in lost {
        note(losses, l);
    }
    match text.char_indices().nth(MAX_TABLE_CELL_CHARS) {
        Some((end, _)) => {
            note(losses, LossKind::TableTruncated);
            text[..end].trim_end().to_owned()
        }
        None => text,
    }
}

/// Les options du tableau `t` et ses pertes ; `None` pour un tableau sans cellule.
///
/// La ligne 0 de la source est la ligne d'en-tête du `dataset` ;
/// `header-row`, `header-column` et `striped` viennent des options PowerPoint
/// (`firstRow`, `firstCol`, `bandRow`). Un texte qui ressemble à un nombre
/// reste du texte. Au plus `MAX_TABLE_ROWS` lignes et `MAX_TABLE_COLUMNS`
/// colonnes : le reste est coupé (`TableTruncated`). Une ligne plus courte que
/// les autres est complétée de cellules vides.
pub(crate) fn table(t: &source::Table) -> Option<(Value, Vec<LossKind>)> {
    let width = t.rows.iter().map(Vec::len).max().unwrap_or(0);
    if width == 0 {
        return None;
    }
    let mut losses = Vec::new();
    if t.rows.len() > MAX_TABLE_ROWS || width > MAX_TABLE_COLUMNS {
        note(&mut losses, LossKind::TableTruncated);
    }
    let width = width.min(MAX_TABLE_COLUMNS);
    let mut merged = false;
    let source: Vec<Vec<Value>> = t
        .rows
        .iter()
        .take(MAX_TABLE_ROWS)
        .map(|row| {
            let mut cells: Vec<Value> = row
                .iter()
                .take(width)
                .map(|c| {
                    merged |= c.covered || c.row_span > 1 || c.col_span > 1;
                    if c.covered { Value::Null } else { Value::String(cell_text(c, &mut losses)) }
                })
                .collect();
            cells.resize(width, Value::Null);
            cells
        })
        .collect();
    if merged {
        note(&mut losses, LossKind::MergedCells);
    }
    let option = json!({
        "dataset": { "source": source },
        "header-row": t.flags.first_row,
        "header-column": t.flags.first_col,
        "striped": t.flags.band_row,
    });
    Some((option, losses))
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod table_tests;
