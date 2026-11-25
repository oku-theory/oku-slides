//! JSON Schema of the v1 subset, given to the LLM as its output schema.

use schemars::schema_for;
use serde_json::Value;

use crate::option::ChartOption;

/// Schema of the full options (creating a chart).
///
/// # Errors
///
/// Fails if the schema does not serialize to JSON.
pub fn option_schema() -> serde_json::Result<Value> {
    serde_json::to_value(schema_for!(ChartOption))
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod schema_tests;
