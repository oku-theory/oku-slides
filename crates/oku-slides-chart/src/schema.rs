//! JSON Schema du sous-ensemble v1, à fournir au LLM comme schéma de sortie.

use schemars::schema_for;
use serde_json::Value;

use crate::data_ops::DataOp;
use crate::option::ChartOption;

/// Schema of the full options (creating a chart).
///
/// # Errors
///
/// Fails if the schema does not serialize to JSON.
pub fn option_schema() -> serde_json::Result<Value> {
    serde_json::to_value(schema_for!(ChartOption))
}

/// Schema of a data operation (dictated editing of a value).
///
/// # Errors
///
/// Fails if the schema does not serialize to JSON.
pub fn data_op_schema() -> serde_json::Result<Value> {
    serde_json::to_value(schema_for!(DataOp))
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod schema_tests;
