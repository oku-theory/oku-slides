use serde_json::json;
use speculoos::prelude::*;

use super::{ToolSpec, compact, exposed};
use crate::families::Family;

fn spec(name: &str) -> ToolSpec {
    ToolSpec {
        name: name.into(),
        description: "Save the deck. The user confirms first.".into(),
        schema: json!({"type": "object", "properties": {"path": {"type": "string", "description": "long"}}, "required": ["path"]}),
    }
}

#[test]
fn compact_keeps_first_sentence() {
    assert_that!(compact(&spec("save")).description.as_str()).is_equal_to("Save the deck.");
}

#[test]
fn compact_keeps_whole_text_without_sentence_end() {
    let mut s = spec("save");
    s.description = "Save the deck".into();
    assert_that!(compact(&s).description.as_str()).is_equal_to("Save the deck");
}

#[test]
fn compact_drops_nested_descriptions_keeps_required() {
    let c = compact(&spec("save"));
    assert_that!(c.schema["properties"]["path"].get("description")).is_none();
    assert_that!(c.schema["required"]).is_equal_to(json!(["path"]));
}

#[test]
fn compact_drops_descriptions_inside_arrays() {
    let mut s = spec("save");
    s.schema =
        json!({"oneOf": [{"type": "string", "description": "x"}], "items": {"description": "y", "type": "integer"}});
    let c = compact(&s);
    assert_that!(c.schema).is_equal_to(json!({"oneOf": [{"type": "string"}], "items": {"type": "integer"}}));
}

#[test]
fn exposed_adds_ask_report_and_filters_by_route() {
    let all = vec![spec("save"), spec("apply"), spec("report")];
    let names: Vec<String> = exposed(&all, &[Family::Edit]).into_iter().map(|s| s.name).collect();
    assert_that!(names).is_equal_to(vec!["apply".to_owned(), "report".to_owned()]);
}

#[test]
fn compact_keeps_property_named_description() {
    let mut s = spec("apply");
    s.schema = json!({"type": "object", "description": "top", "properties": {"description": {"type": "string", "description": "inner"}}});
    let c = compact(&s);
    assert_that!(c.schema).is_equal_to(json!({"type": "object", "properties": {"description": {"type": "string"}}}));
}
