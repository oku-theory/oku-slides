//! JSON Schema du sous-ensemble v1, à fournir au LLM comme schéma de sortie.

use schemars::schema_for;
use serde_json::Value;

use crate::data_ops::ChartDataOp;
use crate::option::ChartOption;

/// Schéma des options complètes (création d'un graphique).
pub fn option_schema() -> serde_json::Result<Value> {
    serde_json::to_value(schema_for!(ChartOption))
}

/// Schéma d'une opération sur les données (édition dictée d'une valeur).
pub fn data_op_schema() -> serde_json::Result<Value> {
    serde_json::to_value(schema_for!(ChartDataOp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_closes_objects_and_lists_series_types() {
        let s = option_schema().unwrap().to_string();
        assert!(s.contains("\"additionalProperties\":false"));
        for t in ["bar", "line", "pie", "scatter", "radar"] {
            assert!(s.contains(&format!("\"{t}\"")), "type {t} absent du schéma");
        }
    }
}
