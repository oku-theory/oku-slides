use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_core::kinds;

use super::{list, nest_schema};

/// La description de l'outil `name`.
fn tool_description(name: &str) -> String {
    list().into_iter().find(|t| t.name == name).and_then(|t| t.description).unwrap_or_default()
}

/// Chaque `$ref` du schéma désigne un chemin qui existe.
fn refs_resolve(root: &Value, node: &Value) -> bool {
    match node {
        Value::Object(map) => map.iter().all(|(k, v)| {
            if k == "$ref" {
                v.as_str().and_then(|r| r.strip_prefix('#')).is_some_and(|p| root.pointer(p).is_some())
            } else {
                refs_resolve(root, v)
            }
        }),
        Value::Array(items) => items.iter().all(|v| refs_resolve(root, v)),
        _ => true,
    }
}

#[test]
fn nest_schema_moves_defs_under_first_property_and_rewrites_refs() {
    let schema = json!({
        "type": "object",
        "properties": { "ops": { "type": "array", "items": { "$ref": "#/$defs/Op" } } },
        "required": ["ops"],
        "$defs": { "Op": { "type": "object", "properties": { "next": { "$ref": "#/$defs/Op" } } } }
    });
    let nested = nest_schema(schema);
    assert_that!(nested.get("$defs")).is_none();
    assert_that!(nested.pointer("/properties/ops/$defs/Op")).is_some();
    assert_that!(refs_resolve(&nested, &nested)).is_true();
}

#[test]
fn every_tool_schema_keeps_only_properties_and_required_at_root_and_resolves() {
    for tool in list() {
        let schema = serde_json::to_value(&tool.input_schema).unwrap();
        assert_that!(refs_resolve(&schema, &schema)).named(&tool.name).is_true();
    }
}

#[test]
fn tool_names_are_unique() {
    let mut names: Vec<String> = list().into_iter().map(|t| t.name).collect();
    let n = names.len();
    names.sort();
    names.dedup();
    assert_that!(names.len()).is_equal_to(n);
}

#[test]
fn tools_list_includes_report() {
    assert_that!(list().iter().any(|t| t.name == "report")).is_true();
}

#[test]
fn tools_list_includes_render() {
    assert_that!(list().iter().any(|t| t.name == "render")).is_true();
}

#[test]
fn tools_render_description_states_the_real_limits() {
    use oku_slides_session::agent::render::{RENDER_DEPTH, RENDER_MAX};
    let tool = list().into_iter().find(|t| t.name == "render").unwrap();
    let text = serde_json::to_value(&tool).unwrap()["description"].as_str().unwrap_or_default().to_owned();
    assert_that!(text).contains(format!("At most {RENDER_MAX} components").as_str());
    assert_that!(text).contains(format!("{RENDER_DEPTH} levels").as_str());
}

#[test]
fn tools_list_excludes_suggest_next() {
    assert_that!(list().iter().any(|t| t.name == "suggest_next")).is_false();
}

#[test]
fn tools_import_diff_description_states_the_real_bounds() {
    use oku_slides_session::import::{LISTED, REFERENCE_WIDTH, ZOOM};
    let tool = list().into_iter().find(|t| t.name == "import_diff").unwrap();
    let text = tool.description.unwrap_or_default();
    assert_that!(text).contains(format!("top {LISTED}").as_str());
    assert_that!(text).contains(format!("{REFERENCE_WIDTH} px wide").as_str());
    assert_that!(text).contains(format!("{ZOOM}x").as_str());
}

#[test]
fn widgets_description_names_every_builtin_kind() {
    let d = tool_description("widgets");
    let missing: Vec<&str> =
        kinds::base::ALL.iter().chain(kinds::essentiel::ALL.iter()).copied().filter(|k| !d.contains(k)).collect();
    assert_that!(missing).is_empty();
}

/// `op_template`, un champ de `show_params`, n'est pas un modèle de slides.
#[test]
fn no_tool_description_mentions_templates() {
    let offenders: Vec<String> = list()
        .into_iter()
        .filter(|t| {
            let d = t.description.as_deref().unwrap_or_default().replace("op_template", "");
            d.to_lowercase().contains("template")
        })
        .map(|t| t.name)
        .collect();
    assert_that!(offenders).is_empty();
}

#[rstest::rstest]
#[case("widget_sources")]
#[case("widget_source")]
#[case("set_widget")]
#[case("remove_widget")]
#[case("preview_widget")]
fn widget_tool_description_says_how_to_reach_deck_layouts(#[case] name: &str) {
    assert_that!(tool_description(name)).contains("`deck-layouts`");
}
