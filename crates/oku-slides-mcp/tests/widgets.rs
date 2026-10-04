//! Les outils des widgets du pack, en mode deck et en mode modèles.

mod common;

use common::*;
use oku_slides_mcp::McpEvent;
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
async fn the_six_widget_tools_are_listed() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let names: Vec<String> = c.request_tool_list(None).await.unwrap().tools.into_iter().map(|t| t.name).collect();
    for tool in ["widget_sources", "widget_source", "set_widget", "remove_widget", "preview_widget", "set_widget_trial"]
    {
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
    call(&c, "edit_templates", json!({ "set": SET })).await;
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_that!(call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await.is_error)
        .is_not_equal_to(Some(true));
    let slide = json!({ "op": "add_slide", "index": 0, "slide": {
        "id": "t-kpi", "kind": "blank", "name": "Indicateur", "params": { "elements": [
            { "id": "k", "kind": "kpi", "cell": { "col": 0, "row": 0, "cols": 4, "rows": 2 }, "params": { "value": "1" } }
        ] } } });
    call(&c, "apply", json!({ "ops": [slide] })).await;
    // Le message de Fluent, dans la langue de l'utilisateur (`fr`).
    assert_that!(error_text(&c, "remove_widget", json!({ "name": "kpi" })).await)
        .is_equal_to("Le widget « kpi » est utilisé par ces modèles : Indicateur.".to_owned());
}

/// Le texte d'un résultat d'erreur, débarrassé des isolants Unicode de Fluent.
async fn error_text(c: &std::sync::Arc<rust_mcp_sdk::mcp_client::ClientRuntime>, tool: &str, args: Value) -> String {
    let r = call(c, tool, args).await;
    assert_that!(r.is_error).named(tool).is_equal_to(Some(true));
    text_of(&r).replace(['\u{2068}', '\u{2069}'], "")
}

#[tokio::test(flavor = "multi_thread")]
async fn widget_tools_refuse_an_invalid_name_in_the_user_language() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    for (tool, args) in [
        ("set_widget", json!({ "name": "Kpi", "source": KPI })),
        ("widget_source", json!({ "name": "../kpi" })),
        ("remove_widget", json!({ "name": "" })),
        ("preview_widget", json!({ "name": "k pi" })),
    ] {
        let text = error_text(&c, tool, args).await;
        asserting(tool).that(&text).starts_with("Nom de widget invalide");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn widget_tools_on_an_unknown_widget_say_it_is_not_found() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    for (tool, args) in [
        ("widget_source", json!({ "name": "absent" })),
        ("remove_widget", json!({ "name": "absent" })),
        ("set_widget", json!({ "name": "absent", "source": "" })),
        ("preview_widget", json!({ "name": "absent" })),
    ] {
        let text = error_text(&c, tool, args).await;
        asserting(tool).that(&text).is_equal_to("Widget introuvable : « absent ».".to_owned());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_refuses_a_nul_character_and_an_oversized_file() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let nul = error_text(&c, "set_widget", json!({ "name": "kpi", "source": "#let a = 1\u{0}" })).await;
    assert_that!(nul).is_equal_to("Le widget « kpi » contient un caractère nul.".to_owned());
    let big = format!("{KPI}// {}\n", "x".repeat(256 * 1024));
    let large = error_text(&c, "set_widget", json!({ "name": "kpi", "source": big })).await;
    assert_that!(large).is_equal_to("Le widget « kpi » dépasse 256 Kio.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_beyond_max_widgets_is_refused() {
    let h = start_with_settings(|s| s.render.max_widgets = 1).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    assert_that!(call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await.is_error)
        .is_not_equal_to(Some(true));
    let text = error_text(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    assert_that!(text).is_equal_to("Un pack ne peut pas avoir plus de 1 fichiers de widgets.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn widget_tools_on_a_closed_tab_answer_in_the_user_language() {
    let h = start_closed().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let text = error_text(&c, "widget_sources", json!({})).await;
    assert_that!(text).is_equal_to("Cet onglet n'est plus ouvert.".to_owned());
}

const SET: &str = "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10";

async fn in_templates_with_kpi(c: &std::sync::Arc<rust_mcp_sdk::mcp_client::ClientRuntime>) {
    call(c, "edit_templates", json!({ "set": SET })).await;
    call(c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    call(c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
}

fn touched(rx: &mut tokio::sync::broadcast::Receiver<McpEvent>) -> Vec<(String, bool)> {
    let mut seen = Vec::new();
    while let Ok(e) = rx.try_recv() {
        if let McpEvent::WidgetTouched { name, removed } = e {
            seen.push((name, removed));
        }
    }
    seen
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_trial_in_templates_mode_returns_the_trials() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    in_templates_with_kpi(&c).await;
    let r = json_of(&call(&c, "set_widget_trial", json!({ "name": "kpi", "values": { "value": "12" } })).await);
    assert_that!(r["trials"]["value"].as_str()).is_equal_to(Some("12"));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_trial_in_templates_mode_emits_changed() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    in_templates_with_kpi(&c).await;
    let mut rx = h.hub.subscribe();
    call(&c, "set_widget_trial", json!({ "name": "kpi", "values": { "value": "12" } })).await;
    let mut changed = false;
    while let Ok(e) = rx.try_recv() {
        changed |= matches!(e, McpEvent::Changed);
    }
    assert_that!(changed).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_trial_in_deck_mode_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    let msg = error_text(&c, "set_widget_trial", json!({ "name": "kpi", "values": { "value": "12" } })).await;
    assert_that!(msg.as_str()).contains("modèles");
}

#[tokio::test(flavor = "multi_thread")]
async fn set_and_remove_widget_in_templates_mode_emit_widget_touched() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "edit_templates", json!({ "set": SET })).await;
    let mut rx = h.hub.subscribe();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    call(&c, "remove_widget", json!({ "name": "kpi" })).await;
    assert_that!(touched(&mut rx)).is_equal_to(vec![
        ("_commun".to_owned(), false),
        ("kpi".to_owned(), false),
        ("kpi".to_owned(), true),
    ]);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_in_deck_mode_emits_no_widget_touched() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let mut rx = h.hub.subscribe();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_that!(touched(&mut rx)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_refused_emits_no_widget_touched() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "edit_templates", json!({ "set": SET })).await;
    let mut rx = h.hub.subscribe();
    call(&c, "set_widget", json!({ "name": "kpi", "source": "a\0b" })).await;
    assert_that!(touched(&mut rx)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_twice_in_a_templates_turn_is_one_undo_step() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "edit_templates", json!({ "set": SET })).await;
    h.session.lock().unwrap().begin_turn("deux widgets");
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    let session = std::sync::Arc::clone(&h.session);
    tokio::task::spawn_blocking(move || {
        let mut s = session.lock().unwrap();
        s.end_turn();
        assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Agent : deux widgets"));
        s.undo().unwrap();
        assert_that!(s.snapshot().can_undo).is_false();
    })
    .await
    .unwrap();
}
