use speculoos::prelude::*;

use super::*;

#[test]
fn option_schema_generated_closes_objects_and_lists_series_types() {
    let s = option_schema().unwrap().to_string();
    assert_that!(s.as_str()).contains("\"additionalProperties\":false");
    for t in ["bar", "line", "pie", "scatter", "radar"] {
        asserting(&format!("type {t} absent du schéma")).that(&s.as_str()).contains(format!("\"{t}\"").as_str());
    }
}
