//! `set_chart_data` : modifications de `dataset.source`, chacune avec son
//! inverse (pour l'historique du deck).
//!
//! Les index de ligne comptent la ligne d'en-têtes : la ligne 0 contient les
//! noms de colonnes, les données commencent à 1. Renommer une colonne est
//! donc un `set_cell` sur la ligne 0.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{ChartError, Result};
use crate::option::{Cell, Dataset};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChartDataOp {
    SetCell {
        row: usize,
        col: usize,
        value: Cell,
    },
    InsertRow {
        at: usize,
        cells: Vec<Cell>,
    },
    RemoveRow {
        at: usize,
    },
    InsertColumn {
        at: usize,
        cells: Vec<Cell>,
    },
    RemoveColumn {
        at: usize,
    },
    /// Remplace tout le tableau (collage depuis un tableur, réinitialisation).
    Replace {
        source: Vec<Vec<Cell>>,
    },
}

fn err(msg: impl Into<String>) -> ChartError {
    ChartError::DataOp(msg.into())
}

impl ChartDataOp {
    /// Applique l'opération et renvoie son inverse.
    pub fn apply(&self, dataset: &mut Dataset) -> Result<ChartDataOp> {
        let src = &mut dataset.source;
        let width = src.first().map_or(0, Vec::len);
        match self {
            ChartDataOp::SetCell { row, col, value } => {
                let r = src.get_mut(*row).ok_or_else(|| err(format!("ligne {row} inexistante")))?;
                if *col >= width {
                    return Err(err(format!("colonne {col} inexistante ({width} colonnes)")));
                }
                // Une ligne courte (JSON écrit à la main) est complétée.
                if r.len() <= *col {
                    r.resize(width, Cell::Null);
                }
                let previous = std::mem::replace(&mut r[*col], value.clone());
                Ok(ChartDataOp::SetCell { row: *row, col: *col, value: previous })
            }
            ChartDataOp::InsertRow { at, cells } => {
                if *at == 0 || *at > src.len() {
                    return Err(err(format!("insertion de ligne en {at} impossible")));
                }
                let mut cells = cells.clone();
                cells.resize(width, Cell::Null);
                src.insert(*at, cells);
                Ok(ChartDataOp::RemoveRow { at: *at })
            }
            ChartDataOp::RemoveRow { at } => {
                if *at == 0 || *at >= src.len() {
                    return Err(err(format!("suppression de la ligne {at} impossible")));
                }
                let cells = src.remove(*at);
                Ok(ChartDataOp::InsertRow { at: *at, cells })
            }
            ChartDataOp::InsertColumn { at, cells } => {
                if *at > width {
                    return Err(err(format!("insertion de colonne en {at} impossible")));
                }
                if cells.len() > src.len() {
                    return Err(err("plus de cellules que de lignes"));
                }
                for (i, row) in src.iter_mut().enumerate() {
                    row.resize(width, Cell::Null);
                    row.insert(*at, cells.get(i).cloned().unwrap_or(Cell::Null));
                }
                Ok(ChartDataOp::RemoveColumn { at: *at })
            }
            ChartDataOp::RemoveColumn { at } => {
                if *at >= width || width == 1 {
                    return Err(err(format!("suppression de la colonne {at} impossible")));
                }
                let cells = src
                    .iter_mut()
                    .map(|row| {
                        row.resize(width, Cell::Null);
                        row.remove(*at)
                    })
                    .collect();
                Ok(ChartDataOp::InsertColumn { at: *at, cells })
            }
            ChartDataOp::Replace { source } => {
                if source.is_empty() {
                    return Err(err("le tableau doit garder sa ligne d'en-têtes"));
                }
                let previous = std::mem::replace(src, source.clone());
                Ok(ChartDataOp::Replace { source: previous })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn ds() -> Dataset {
        serde_json::from_value(serde_json::json!({
            "source": [["Mois", "Ventes"], ["Jan", 10], ["Fév", 12]]
        }))
        .unwrap()
    }

    #[test]
    fn every_op_is_undone_by_its_inverse() {
        let ops = vec![
            ChartDataOp::SetCell { row: 1, col: 1, value: Cell::Number(99.0) },
            ChartDataOp::SetCell { row: 0, col: 1, value: Cell::Text("CA".into()) },
            ChartDataOp::InsertRow { at: 3, cells: vec![Cell::Text("Mar".into())] },
            ChartDataOp::RemoveRow { at: 1 },
            ChartDataOp::InsertColumn { at: 2, cells: vec![Cell::Text("Marge".into()), Cell::Number(3.0)] },
            ChartDataOp::RemoveColumn { at: 0 },
            ChartDataOp::Replace { source: vec![vec![Cell::Text("x".into())]] },
        ];
        for op in ops {
            let original = ds();
            let mut d = original.clone();
            let inverse = op.apply(&mut d).unwrap();
            assert_ne!(d, original, "{op:?} n'a rien changé");
            inverse.apply(&mut d).unwrap();
            assert_eq!(d, original, "l'inverse de {op:?} ne restaure pas");
        }
    }

    #[test]
    fn header_row_cannot_be_removed() {
        assert!(ChartDataOp::RemoveRow { at: 0 }.apply(&mut ds()).is_err());
        assert!(ChartDataOp::InsertRow { at: 0, cells: vec![] }.apply(&mut ds()).is_err());
    }

    #[test]
    fn ops_deserialize_from_tagged_json() {
        let op: ChartDataOp = serde_json::from_str(r#"{"op":"set_cell","row":2,"col":1,"value":120}"#).unwrap();
        assert_eq!(op, ChartDataOp::SetCell { row: 2, col: 1, value: Cell::Number(120.0) });
    }
}
