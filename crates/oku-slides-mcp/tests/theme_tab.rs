//! A theme tab over MCP: its outline, its draft's parents, and what it refuses.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use common::kinds::{list_tool_names, unknown_tool};
use common::*;
use oku_slides_session::agent::bounds::label;
use oku_slides_session::{LibraryKind, PAPIER_THEME, PRO_THEME, Request, Themes};
use serde_json::json;
use speculoos::prelude::*;

/// The `WrongDocument` refusal naming `kind`, as the harness's localizer says it.
fn wrong_document(h: &Harness, kind: &str) -> String {
    h.localizer.tr_with("session-wrong-document", &[("kind", kind.into())])
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_outline_reports_the_edited_theme() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    let papier = Themes::builtins().iter().find(|b| b.manifest.id == PAPIER_THEME).unwrap().clone();
    assert_that!(outline["mode"].as_str()).is_equal_to(Some("theme"));
    assert_that!(outline["theme"].clone()).is_equal_to(serde_json::Value::Null);
    assert_that!(outline["edited_theme"]).is_equal_to(json!({
        "id": papier.manifest.id, "version": papier.version, "namespace": "papier", "name": "Papier",
        "child": true, "extends": [{ "namespace": "papier", "name": "Papier" }], "dirty": false,
    }));
}

#[tokio::test(flavor = "multi_thread")]
async fn deck_outline_edited_theme_has_namespace() {
    let h = start_theme_tab(PRO_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["edited_theme"]["namespace"].as_str()).is_equal_to(Some("pro"));
}

#[tokio::test(flavor = "multi_thread")]
async fn deck_outline_of_a_deck_tab_has_no_edited_theme() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!((outline["mode"].as_str(), outline["edited_theme"].is_null())).is_equal_to((Some("deck"), true));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_theme_extends_in_a_theme_tab_sets_the_parents() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "set_theme_extends", json!({ "extends": ["pro"] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["edited_theme"]["extends"].clone())
        .is_equal_to(json!([{ "namespace": "pro", "name": "Pro" }]));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_theme_extends_outside_a_theme_tab_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "set_theme_extends", json!({ "extends": ["pro"] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(unknown_tool(&h, "set_theme_extends"));
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_apply_of_set_theme_is_refused() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": theme_ref("pro") }] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(wrong_document(&h, "theme"));
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["edited_theme"]["name"].as_str()).is_equal_to(Some("Papier"));
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_apply_of_a_token_op_changes_the_draft() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let op = json!({ "op": "set_token", "path": ["color", "accent"], "value": "#ff0000" });
    let r = call(&c, "apply", json!({ "ops": [op] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["edited_theme"]["dirty"].as_bool()).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_set_widget_is_refused() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let source = "#import \"/oku-slides/lib.typ\": widget\n#let widget = widget(\"w\", (args, ctx) => [])\n";
    let r = call(&c, "set_widget", json!({ "name": "w", "source": source })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(unknown_tool(&h, "set_widget"));
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_remove_pack_is_refused() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "remove_pack", json!({ "kind": "layouts", "namespace": "essentiel" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(unknown_tool(&h, "remove_pack"));
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_save_copy_is_refused_without_asking() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let dir = temp_dir("oku-slides-theme-tab-copy-");
    let path = dir.path().join("copie.oku-slides.pdf");
    let r = call(&c, "save_copy", json!({ "path": path.to_string_lossy() })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(unknown_tool(&h, "save_copy"));
    assert_that!((path.exists(), h.hub.has_pending_ask())).is_equal_to((false, false));
}

#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_new_deck_is_refused() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = server(&h).await;
    let r = call_within(&c, "new", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(unknown_tool(&h, "new"));
    assert_that!(h.hub.has_pending_ask()).is_false();
    assert_that!(h.session.lock().unwrap().document_kind()).is_equal_to(oku_slides_session::DocumentKind::Theme);
}

/// The `AgentForm` refusal whose reason `key` says, with `args`, as the harness's localizer says it.
fn agent_form(h: &Harness, key: &str, args: &[(&str, oku_slides_i18n::FluentValue<'_>)]) -> String {
    h.localizer.tr_with("session-agent-form", &[("reason", h.localizer.tr_with(key, args).into())])
}

/// The edited theme's name, as the tab's snapshot shows it.
fn theming_name(h: &Harness) -> String {
    let s = lock(&h.session);
    s.snapshot().theming.map_or_else(|| panic!("a theme tab"), |t| t.theme.name)
}

/// The edited theme's base id, as the tab's snapshot shows it.
fn theming_base(h: &Harness) -> uuid::Uuid {
    let s = lock(&h.session);
    s.snapshot().theming.map_or_else(|| panic!("a theme tab"), |t| t.theme.base.id)
}

/// The ids of the library's themes, as a fresh snapshot lists them (it rereads the library).
fn library_theme_ids(h: &Harness) -> Vec<String> {
    let snapshot = || lock(&h.session).handle(Request::Snapshot);
    let json = tokio::task::block_in_place(snapshot).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let snap: serde_json::Value = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
    let themes = snap["themes"].as_array().unwrap_or_else(|| panic!("a theme list"));
    themes.iter().filter_map(|t| t["id"].as_str().map(str::to_owned)).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn rename_theme_renames_the_draft_in_one_undo_step() {
    let h = start_theme_tab(PRO_THEME).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("{e}"));
    let r = call(&c, "rename_theme", json!({ "name": "Mon pro" })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(theming_name(&h)).is_equal_to("Mon pro".to_owned());
    lock(&h.session).handle(Request::Undo).unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(theming_name(&h)).is_equal_to("Pro".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn rename_theme_with_a_hidden_control_is_refused_and_renames_nothing() {
    let h = start_theme_tab(PRO_THEME).await;
    let c = server(&h).await;
    let r = call(&c, "rename_theme", json!({ "name": "Pro\u{202E}" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let expected = agent_form(&h, "session-agent-form-hidden-characters", &[("at", "name".into())]);
    assert_that!(text_of(&r)).is_equal_to(expected);
    assert_that!(text_of(&r)).contains("invisibles");
    assert_that!(theming_name(&h)).is_equal_to("Pro".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn rename_theme_too_long_is_refused() {
    let h = start_theme_tab(PRO_THEME).await;
    let c = server(&h).await;
    let r = call(&c, "rename_theme", json!({ "name": "a".repeat(61) })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let args = [("at", "name".into()), ("count", 61.into()), ("max", 60.into())];
    assert_that!(text_of(&r)).is_equal_to(agent_form(&h, "session-agent-form-too-long", &args));
    assert_that!(theming_name(&h)).is_equal_to("Pro".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn save_theme_confirmed_saves_a_copy_the_tab_then_edits() {
    let h = start_theme_tab(PRO_THEME).await;
    let r = guarded(&h, "save_theme", json!({}), true).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let copy = theming_base(&h);
    assert_that!(copy).is_not_equal_to(PRO_THEME);
    let ids = library_theme_ids(&h);
    assert_that!(ids.contains(&PRO_THEME.to_string()) && ids.contains(&copy.to_string())).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn save_theme_denied_saves_nothing() {
    let h = start_theme_tab(PRO_THEME).await;
    let before = library_theme_ids(&h);
    let r = guarded(&h, "save_theme", json!({}), false).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-denied"));
    assert_that!((theming_base(&h), library_theme_ids(&h))).is_equal_to((PRO_THEME, before));
}

#[tokio::test(flavor = "multi_thread")]
async fn save_theme_asks_with_the_edited_theme_name() {
    let h = start_theme_tab(PRO_THEME).await;
    let name = theming_name(&h);
    let (message, danger, _r) = guarded_message(&h, "save_theme", json!({}), false).await;
    let expected = h.localizer.tr_with("agent-confirm-copy-edited-theme", &[("name", label(&name).into())]);
    assert_that!((message, danger)).is_equal_to((expected, false));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_params_in_a_theme_tab_is_listed() {
    let h = start_theme_tab(PRO_THEME).await;
    let c = server(&h).await;
    let names = list_tool_names(&c).await;
    assert_that!(names.contains(&"show_params".to_owned()) && names.contains(&"show_variants".to_owned())).is_true();
}

/// A theme tab neither lists nor serves the tools that need a deck's widgets or its text (t-1278).
#[rstest::rstest]
#[case::widgets("widgets", json!({}))]
#[case::packs("packs", json!({}))]
#[case::ask_rewrite("ask_rewrite", json!({ "slide": "s1", "owner": "s1", "field": "title" }))]
#[tokio::test(flavor = "multi_thread")]
async fn theme_tab_does_not_serve_the_deck_only_shared_tools(#[case] tool: &str, #[case] args: serde_json::Value) {
    let h = start_theme_tab(PRO_THEME).await;
    let c = server(&h).await;
    assert_that!(list_tool_names(&c).await.contains(&tool.to_owned())).named(tool).is_false();
    let r = call(&c, tool, args).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(unknown_tool(&h, tool));
}
