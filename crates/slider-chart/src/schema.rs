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
#[path = "schema_tests.rs"]
mod schema_tests;
