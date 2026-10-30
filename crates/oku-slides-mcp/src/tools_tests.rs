use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_core::kinds;

use oku_slides_session::DocumentKind;
use rstest::rstest;

use super::{kinds_of, list_all, list_for, nest_schema};

/// Every tool, whatever the kinds that list it.
fn list() -> Vec<rust_mcp_sdk::schema::Tool> {
    list_all().iter().map(|(t, _)| t.clone()).collect()
}

/// The description of tool `name`.
fn tool_description(name: &str) -> String {
    list().into_iter().find(|t| t.name == name).and_then(|t| t.description).unwrap_or_default()
}

/// Every `$ref` of the schema points to a path that exists.
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

/// `op_template`, a field of `show_params`, is not a slide template.
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

#[rstest]
#[case("widget_sources")]
#[case("widget_source")]
#[case("set_widget")]
#[case("remove_widget")]
#[case("preview_widget")]
fn widget_tool_description_says_how_to_reach_deck_layouts(#[case] name: &str) {
    assert_that!(tool_description(name)).contains("`deck-layouts`");
}

#[test]
fn open_pack_description_says_one_tab_move_per_turn_and_stop_here() {
    let d = tool_description("open_pack");
    assert_that!(d.as_str()).contains("Open a pack in its own tab to edit it there");
    assert_that!(d.as_str()).contains("One tab move (opening or showing a pack or theme tab) per turn");
    assert_that!(d.as_str()).contains("this request stops here");
    assert_that!(d.as_str()).does_not_contain("In a pack's own tab");
    assert_that!(d.as_str()).does_not_contain("save_pack");
}

#[test]
fn open_theme_description_shares_open_pack_limit() {
    let d = tool_description("open_theme");
    assert_that!(d.as_str()).contains("Same limit as `open_pack`: one tab move per turn");
    assert_that!(d.as_str()).contains("`edit_theme`");
}

#[test]
fn packs_description_names_no_tool_a_pack_or_theme_tab_lacks() {
    let d = tool_description("packs");
    assert_that!(d.as_str()).contains("A deck tab opens one in its own tab.");
    assert_that!(d.as_str()).does_not_contain("open_pack");
    assert_that!(d.as_str()).does_not_contain("edit_pack");
}

#[test]
fn present_description_names_no_pack_mode() {
    assert_that!(tool_description("present").as_str()).does_not_contain("pack mode");
}

/// The tools of a deck tab, and of a help tab (a read-only deck), sorted.
const DECK_TOOLS: &[&str] = &[
    "add_pack",
    "apply",
    "ask_choice",
    "ask_confirm",
    "ask_rewrite",
    "cancel_agent_model_download",
    "cancel_dictation_download",
    "chart_kind",
    "deck_outline",
    "delete_pack",
    "download_agent_model",
    "download_dictation_model",
    "edit_theme",
    "focus",
    "history",
    "import_diff",
    "import_report",
    "layouts",
    "merge",
    "move",
    "new",
    "open",
    "open_pack",
    "open_theme",
    "packs",
    "present",
    "preview_widget",
    "remove_pack",
    "remove_widget",
    "render",
    "render_slide",
    "report",
    "save",
    "save_copy",
    "select_agent",
    "set_dictation",
    "set_interface",
    "set_setting",
    "set_widget",
    "settings",
    "show_as",
    "show_dictation",
    "show_packs",
    "show_params",
    "show_themes",
    "show_variants",
    "slide",
    "theme_tokens",
    "widget_source",
    "widget_sources",
    "widgets",
];

/// The tools of a pack tab, sorted.
const PACK_TOOLS: &[&str] = &[
    "apply",
    "ask_choice",
    "ask_confirm",
    "ask_rewrite",
    "cancel_agent_model_download",
    "cancel_dictation_download",
    "deck_outline",
    "download_agent_model",
    "download_dictation_model",
    "focus",
    "layouts",
    "packs",
    "preview_widget",
    "remove_widget",
    "render",
    "render_slide",
    "report",
    "save_pack",
    "select_agent",
    "select_item",
    "set_dictation",
    "set_interface",
    "set_preview_theme",
    "set_setting",
    "set_widget",
    "set_widget_trial",
    "settings",
    "show_dictation",
    "show_params",
    "show_variants",
    "slide",
    "theme_tokens",
    "widget_source",
    "widget_sources",
    "widgets",
];

/// The tools of a theme tab, sorted.
const THEME_TOOLS: &[&str] = &[
    "apply",
    "ask_choice",
    "ask_confirm",
    "ask_rewrite",
    "cancel_agent_model_download",
    "cancel_dictation_download",
    "deck_outline",
    "download_agent_model",
    "download_dictation_model",
    "focus",
    "packs",
    "rename_theme",
    "render",
    "render_slide",
    "report",
    "save_theme",
    "select_agent",
    "set_dictation",
    "set_interface",
    "set_setting",
    "set_theme_extends",
    "settings",
    "show_dictation",
    "show_params",
    "show_variants",
    "slide",
    "theme_tokens",
    "widgets",
];

#[rstest]
#[case::deck(DocumentKind::Deck, DECK_TOOLS)]
#[case::help(DocumentKind::Help, DECK_TOOLS)]
#[case::pack(DocumentKind::Pack, PACK_TOOLS)]
#[case::theme(DocumentKind::Theme, THEME_TOOLS)]
fn list_for_a_kind_is_exactly_its_tools(#[case] kind: DocumentKind, #[case] expected: &[&str]) {
    let mut names: Vec<String> = list_for(kind).into_iter().map(|t| t.name).collect();
    names.sort();
    assert_that!(names).is_equal_to(expected.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>());
}

/// Every tool is listed somewhere: no table entry is dead.
#[test]
fn list_all_every_tool_is_listed_for_some_kind() {
    for (tool, _) in list_all() {
        let somewhere = [DocumentKind::Deck, DocumentKind::Pack, DocumentKind::Theme]
            .into_iter()
            .any(|k| list_for(k).iter().any(|t| t.name == tool.name));
        assert_that!(somewhere).named(&tool.name).is_true();
    }
}

#[test]
fn kinds_of_an_unknown_name_is_none() {
    assert_that!(kinds_of("nope")).is_none();
}

#[test]
fn apply_description_names_the_add_pack_and_remove_pack_ops() {
    let d = tool_description("apply");
    assert_that!(d.as_str()).contains("the pack ops (add_pack, remove_pack)");
}
