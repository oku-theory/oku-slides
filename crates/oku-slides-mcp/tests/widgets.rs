//! Les outils des widgets du pack, en mode deck et en mode modèles.

mod common;

use common::*;
use rust_mcp_sdk::McpClient;
use rust_mcp_sdk::schema::ContentBlock;
use serde_json::{Value, json};
use speculoos::prelude::*;

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#import "_commun.typ": cadre
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => cadre(field(ctx, "value", args.value)))
"#;

const COMMON: &str = "#let cadre(body) = block(inset: 8pt, body)\n";

/// Le JSON d'une réponse texte.
fn json_of(r: &rust_mcp_sdk::schema::CallToolResult) -> Value {
    serde_json::from_str(&text_of(r)).unwrap_or_else(|e| panic!("réponse non JSON : {e}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_five_widget_tools_are_listed() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let names: Vec<String> = c.request_tool_list(None).await.unwrap().tools.into_iter().map(|t| t.name).collect();
    for tool in ["widget_sources", "widget_source", "set_widget", "remove_widget", "preview_widget"] {
        assert_that!(names.contains(&tool.to_owned())).named(tool).is_true();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_in_deck_mode_returns_its_status_then_previews_it() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    let r = call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let change = json_of(&r);
    assert_that!(change["status"]["name"].as_str()).is_equal_to(Some("kpi"));
    assert_that!(change["status"]["diagnostics"].as_array().map(Vec::len)).is_equal_to(Some(0));
    let sources = json_of(&call(&c, "widget_sources", json!({})).await);
    assert_that!(sources.as_array().map(Vec::len)).is_equal_to(Some(2));
    assert_that!(text_of(&call(&c, "widget_source", json!({ "name": "kpi" })).await)).is_equal_to(KPI.to_owned());
    let preview = call(&c, "preview_widget", json!({ "name": "kpi", "params": { "value": "12" } })).await;
    assert_that!(matches!(preview.content.first(), Some(ContentBlock::ImageContent(_)))).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_on_common_reports_diagnostics_and_broken_widgets() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    let change = json_of(&call(&c, "set_widget", json!({ "name": "_commun", "source": "#let cadre = (" })).await);
    assert_that!(change["broken"][0]["name"].as_str()).is_equal_to(Some("kpi"));
    assert_that!(change["status"]["diagnostics"][0]["line"].as_u64()).is_equal_to(Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn widget_tools_work_in_templates_mode_and_remove_refuses_a_used_widget() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "edit_templates", json!({ "set": "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10" })).await;
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_that!(call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await.is_error)
        .is_not_equal_to(Some(true));
    let slide = json!({ "op": "add_slide", "index": 0, "slide": {
        "id": "t-kpi", "kind": "blank", "name": "Indicateur", "params": { "elements": [
            { "id": "k", "kind": "kpi", "cell": { "col": 0, "row": 0, "cols": 4, "rows": 2 }, "params": { "value": "1" } }
        ] } } });
    call(&c, "apply", json!({ "ops": [slide] })).await;
    let r = call(&c, "remove_widget", json!({ "name": "kpi" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).contains("Indicateur");
}
