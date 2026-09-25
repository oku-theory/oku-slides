use speculoos::prelude::*;

use super::*;

fn ds() -> Dataset {
    serde_json::from_value(serde_json::json!({
        "source": [["Mois", "Ventes"], ["Jan", 10], ["Fév", 12]]
    }))
    .unwrap()
}

#[test]
fn apply_every_op_then_inverse_restores_dataset() {
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
        asserting(&format!("{op:?} n'a rien changé")).that(&d).is_not_equal_to(&original);
        inverse.apply(&mut d).unwrap();
        asserting(&format!("l'inverse de {op:?} ne restaure pas")).that(&d).is_equal_to(&original);
    }
}

#[test]
fn apply_row_op_on_header_row_is_rejected() {
    assert_that!(ChartDataOp::RemoveRow { at: 0 }.apply(&mut ds())).is_err();
    assert_that!(ChartDataOp::InsertRow { at: 0, cells: vec![] }.apply(&mut ds())).is_err();
}

#[test]
fn chart_data_op_tagged_json_deserializes() {
    let op: ChartDataOp = serde_json::from_str(r#"{"op":"set_cell","row":2,"col":1,"value":120}"#).unwrap();
    assert_that!(op).is_equal_to(ChartDataOp::SetCell { row: 2, col: 1, value: Cell::Number(120.0) });
}
