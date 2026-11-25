use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_core::kinds;

use oku_slides_session::DocumentKind;
use rstest::rstest;

use super::{kinds_of, list_all, list_for, nest_schema, tool_names_for};
use crate::tool_kinds::Kinds;

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
fn show_params_description_says_a_deck_tab_shows_a_card_from_the_turn_in_the_inspector() {
    let d = tool_description("show_params");
    assert_that!(d.as_str())
        .contains("In a deck tab it shows as a card the user opens from your turn, in the inspector");
}

#[test]
fn packs_description_names_no_tool_a_pack_or_theme_tab_lacks() {
    let d = tool_description("packs");
    assert_that!(d.as_str()).contains("A deck or help tab opens one in its own tab.");
    assert_that!(d.as_str()).does_not_contain("open_pack");
    assert_that!(d.as_str()).does_not_contain("edit_pack");
}

/// A theme tab lists no `widgets`, so `apply` does not send it there (t-1278).
#[test]
fn apply_description_names_widgets_only_for_a_deck_or_pack_tab() {
    let d = tool_description("apply");
    assert_that!(d.as_str()).contains("(a deck or pack tab reads its schema with widgets)");
    assert_that!(d.as_str()).does_not_contain("see widgets");
}

/// The agent learns each data op and its arguments from the prose, not only the schema's enum.
#[rstest]
#[case::add_data("add_data {id?, name, label?, table")]
#[case::bind_data("bind_data {target, param, data, columns?}")]
#[case::remove_data("remove_data {data}")]
#[case::rename_data("rename_data {data, name}")]
#[case::set_column_type("set_column_type {data, column, type: text|number|integer|bool|date}")]
#[case::edit_data("edit_data {data, change}")]
fn apply_description_states_each_data_op_with_its_arguments(#[case] clause: &str) {
    assert_that!(tool_description("apply").as_str()).contains(clause);
}

#[test]
fn present_description_names_no_pack_mode() {
    assert_that!(tool_description("present").as_str()).does_not_contain("pack mode");
}

/// The tools of a deck tab, and of a help tab (a read-only deck), sorted.
const DECK_TOOLS: &[&str] = &[
    "a2ui_examples",
    "add_pack",
    "apply",
    "ask_choice",
    "ask_confirm",
    "ask_rewrite",
    "cancel_agent_model_download",
    "cancel_dictation_download",
    "chart_kind",
    "data_list",
    "data_read",
    "deck_outline",
    "delete_pack",
    "diagnose",
    "download_agent_model",
    "download_dictation_model",
    "edit_theme",
    "focus",
    "help",
    "history",
    "import",
    "import_diff",
    "import_report",
    "knowledge_delete",
    "knowledge_edit",
    "knowledge_list",
    "knowledge_move",
    "knowledge_read",
    "knowledge_search",
    "knowledge_write",
    "layouts",
    "merge",
    "move",
    "new",
    "new_conversation",
    "open",
    "open_pack",
    "open_theme",
    "pack_update",
    "packs",
    "present",
    "preview_widget",
    "quit",
    "redo",
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
    "set_speech",
    "set_widget",
    "settings",
    "show_as",
    "show_dictation",
    "show_packs",
    "show_params",
    "show_themes",
    "show_usage",
    "show_variants",
    "slide",
    "tab",
    "tabs",
    "theme_tokens",
    "undo",
    "undo_turn",
    "update",
    "usage",
    "widget_source",
    "widget_sources",
    "widgets",
    "window",
];

/// The tools of a pack tab, sorted.
const PACK_TOOLS: &[&str] = &[
    "a2ui_examples",
    "apply",
    "ask_choice",
    "ask_confirm",
    "ask_rewrite",
    "cancel_agent_model_download",
    "cancel_dictation_download",
    "deck_outline",
    "diagnose",
    "download_agent_model",
    "download_dictation_model",
    "duplicate_widget",
    "focus",
    "help",
    "layouts",
    "new_conversation",
    "packs",
    "preview_widget",
    "quit",
    "redo",
    "remove_widget",
    "render",
    "render_slide",
    "reorder_widgets",
    "report",
    "save_pack",
    "select_agent",
    "select_item",
    "set_dictation",
    "set_guide",
    "set_interface",
    "set_preview_theme",
    "set_setting",
    "set_speech",
    "set_widget",
    "set_widget_trial",
    "settings",
    "show_dictation",
    "show_params",
    "show_usage",
    "show_variants",
    "slide",
    "tab",
    "tabs",
    "theme_tokens",
    "undo",
    "undo_turn",
    "update",
    "usage",
    "widget_source",
    "widget_sources",
    "widgets",
    "window",
];

/// The tools of a theme tab, sorted.
const THEME_TOOLS: &[&str] = &[
    "a2ui_examples",
    "apply",
    "ask_choice",
    "ask_confirm",
    "cancel_agent_model_download",
    "cancel_dictation_download",
    "deck_outline",
    "diagnose",
    "download_agent_model",
    "download_dictation_model",
    "focus",
    "help",
    "new_conversation",
    "quit",
    "redo",
    "rename_theme",
    "render",
    "render_slide",
    "report",
    "save_theme",
    "select_agent",
    "set_dictation",
    "set_guide",
    "set_interface",
    "set_setting",
    "set_speech",
    "set_theme_extends",
    "settings",
    "show_dictation",
    "show_params",
    "show_usage",
    "show_variants",
    "slide",
    "tab",
    "tabs",
    "theme_tokens",
    "undo",
    "undo_turn",
    "update",
    "usage",
    "window",
];

#[rstest]
#[case::deck(DocumentKind::Deck, DECK_TOOLS)]
#[case::pack(DocumentKind::Pack, PACK_TOOLS)]
#[case::theme(DocumentKind::Theme, THEME_TOOLS)]
fn list_for_a_kind_is_exactly_its_tools(#[case] kind: DocumentKind, #[case] expected: &[&str]) {
    let mut names: Vec<String> = list_for(kind).into_iter().map(|t| t.name).collect();
    names.sort();
    assert_that!(names).is_equal_to(expected.iter().map(|n| (*n).to_owned()).collect::<Vec<_>>());
}

#[rstest]
#[case::pack(DocumentKind::Pack, true)]
#[case::theme(DocumentKind::Theme, true)]
#[case::deck(DocumentKind::Deck, false)]
#[case::help(DocumentKind::Help, false)]
fn list_for_set_guide_only_in_pack_and_theme_tabs(#[case] kind: DocumentKind, #[case] listed: bool) {
    assert_that!(list_for(kind).iter().any(|t| t.name == "set_guide")).is_equal_to(listed);
}

/// The agent can never approve a received guide: no tool of any kind is named for it (t-1267).
#[rstest]
#[case::deck(DocumentKind::Deck)]
#[case::help(DocumentKind::Help)]
#[case::pack(DocumentKind::Pack)]
#[case::theme(DocumentKind::Theme)]
fn list_for_no_tool_approves_or_postpones_guides(#[case] kind: DocumentKind) {
    for tool in list_for(kind) {
        let name = tool.name.to_lowercase();
        assert_that!(name.contains("approve") || name.contains("later")).named(&tool.name).is_false();
    }
}

/// A help tab lists the deck's tools but the usage ones (it belongs to no deck), the undo ones
/// (read-only) and the knowledge ones (the help has no knowledge base).
#[test]
fn list_for_help_is_the_deck_tools_without_usage_undo_or_knowledge() {
    let mut names: Vec<String> = list_for(DocumentKind::Help).into_iter().map(|t| t.name).collect();
    names.sort();
    let expected: Vec<String> = DECK_TOOLS
        .iter()
        .filter(|n| {
            !matches!(**n, "usage" | "show_usage" | "undo" | "redo" | "undo_turn") && !n.starts_with("knowledge_")
        })
        .map(|n| (*n).to_owned())
        .collect();
    assert_that!(names).is_equal_to(expected);
}

#[rstest]
#[case::deck(DocumentKind::Deck)]
#[case::help(DocumentKind::Help)]
#[case::pack(DocumentKind::Pack)]
#[case::theme(DocumentKind::Theme)]
fn list_for_diagnose_is_listed_with_no_property(#[case] kind: DocumentKind) {
    let tool = list_for(kind).into_iter().find(|t| t.name == "diagnose").unwrap();
    let schema = serde_json::to_value(&tool.input_schema).unwrap();
    assert_that!(schema["properties"].as_object().map_or(0, serde_json::Map::len)).is_equal_to(0);
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

#[test]
fn tools_render_description_lists_the_playbook_components() {
    let tool = list().into_iter().find(|t| t.name == "render").unwrap();
    let components = oku_slides_a2ui::render_components(oku_slides_a2ui::agent_reference().unwrap());
    assert_that!(tool.description.unwrap_or_default())
        .contains(format!("Show a card composed of these A2UI components: {components}.").as_str());
}

#[test]
fn tools_render_description_points_to_a2ui_examples() {
    let tool = list().into_iter().find(|t| t.name == "render").unwrap();
    assert_that!(tool.description.unwrap_or_default()).contains("Call a2ui_examples");
}

/// The data tools read a deck: deck and help tabs list them, pack and theme tabs never do.
#[rstest]
#[case::list("data_list")]
#[case::read("data_read")]
fn data_tools_are_read_kind_and_tab_gated(#[case] name: &str) {
    assert_that!(kinds_of(name)).is_equal_to(Some(Kinds::DECK_OR_HELP));
    assert_that!(tool_names_for(DocumentKind::Help).contains(&name.to_owned())).is_true();
    assert_that!(tool_names_for(DocumentKind::Pack).contains(&name.to_owned())).is_false();
    assert_that!(tool_names_for(DocumentKind::Theme).contains(&name.to_owned())).is_false();
}

/// The `apply` input schema lists the knowledge ops beside the others, and its description says
/// so.
#[test]
fn apply_schema_lists_knowledge_ops() {
    let apply = list().into_iter().find(|t| t.name == "apply").unwrap();
    let schema = serde_json::to_string(&apply.input_schema).unwrap_or_default();
    for op in ["knowledge_write", "knowledge_edit", "knowledge_move", "knowledge_delete", "set_notes"] {
        assert_that!(schema.contains(&format!("\"{op}\""))).named(op).is_true();
    }
    assert_that!(apply.description.unwrap_or_default()).contains("prefer the knowledge_* tools");
}

#[test]
fn tool_descriptions_for_a_deck_fill_the_search_hit_cap() {
    let knowledge = oku_slides_platform::config::Settings::defaults().unwrap().knowledge;
    let described = super::tool_descriptions_for(DocumentKind::Deck, &knowledge);
    let search = described.iter().find(|(n, _)| n == "knowledge_search").map(|(_, d)| d.as_str()).unwrap_or_default();
    assert_that!(search).contains(format!("At most {} hits", knowledge.effective_search_hits()).as_str());
    for (name, description) in &described {
        assert_that!(description.contains("{search_hits}")).named(name).is_false();
    }
}
