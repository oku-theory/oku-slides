//! Les outils des widgets du pack, en mode deck et en mode pack.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

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
async fn widget_tools_work_in_pack_mode() {
    let h = start_pack_tab(SET).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_that!(call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await.is_error)
        .is_not_equal_to(Some(true));
    let sources = json_of(&call(&c, "widget_sources", json!({})).await);
    let names: Vec<&str> = sources.as_array().into_iter().flatten().filter_map(|w| w["name"].as_str()).collect();
    assert_that!(names).contains("kpi");
}

#[tokio::test(flavor = "multi_thread")]
async fn remove_widget_used_by_a_slide_is_refused_in_the_user_language() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    let add = json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": "k", "kind": "deck/kpi", "params": { "value": "1" } } });
    assert_that!(call(&c, "apply", json!({ "ops": [add] })).await.is_error).is_not_equal_to(Some(true));
    // Le message de Fluent, dans la langue de l'utilisateur (`fr`).
    assert_that!(error_text(&c, "remove_widget", json!({ "name": "kpi" })).await)
        .starts_with("Le widget « deck/kpi » est encore utilisé ici");
}

/// Un layout minimal : un titre seul.
const COVER: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("cover", category: "slide", params: (title: p.plain()),
  (args, ctx) => field(ctx, "title", args.title))
"#;

/// En mode deck, sans `pack`, un layout va à `deck-layouts` : `pack` le retrouve, comme le
/// disent les descriptions des outils de widgets.
#[tokio::test(flavor = "multi_thread")]
async fn slide_widget_written_without_pack_is_read_through_pack_deck_layouts() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    assert_that!(call(&c, "set_widget", json!({ "name": "cover", "source": COVER })).await.is_error)
        .is_not_equal_to(Some(true));
    let names = |v: &Value| -> Vec<String> {
        v.as_array().into_iter().flatten().filter_map(|w| w["name"].as_str().map(str::to_owned)).collect()
    };
    let layouts = json_of(&call(&c, "widget_sources", json!({ "pack": "deck-layouts" })).await);
    assert_that!(names(&layouts)).contains("cover".to_owned());
    let own = json_of(&call(&c, "widget_sources", json!({})).await);
    assert_that!(names(&own)).does_not_contain("cover".to_owned());
    let source = call(&c, "widget_source", json!({ "pack": "deck-layouts", "name": "cover" })).await;
    assert_that!(text_of(&source)).is_equal_to(COVER.to_owned());
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

/// The built-in widget pack `base`, edited in its own tab.
const SET: uuid::Uuid = oku_slides_session::pack::BASE_PACK;

async fn in_pack_mode_with_kpi(c: &std::sync::Arc<rust_mcp_sdk::mcp_client::ClientRuntime>) {
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
async fn set_widget_trial_in_pack_mode_returns_the_trials() {
    let h = start_pack_tab(SET).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    in_pack_mode_with_kpi(&c).await;
    let r = json_of(&call(&c, "set_widget_trial", json!({ "name": "kpi", "values": { "value": "12" } })).await);
    assert_that!(r["trials"]["value"].as_str()).is_equal_to(Some("12"));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_trial_in_pack_mode_emits_changed() {
    let h = start_pack_tab(SET).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    in_pack_mode_with_kpi(&c).await;
    let mut rx = h.hub.subscribe();
    call(&c, "set_widget_trial", json!({ "name": "kpi", "values": { "value": "12" } })).await;
    let mut changed = false;
    while let Ok(e) = rx.try_recv() {
        changed |= matches!(e, McpEvent::Changed);
    }
    assert_that!(changed).is_true();
}

/// t-524 : deux `set_widget_trial` d'un même tour, en mode pack, sont une seule étape
/// d'annulation ; l'annuler ramène l'étape d'avant le tour.
#[tokio::test(flavor = "multi_thread")]
async fn set_widget_trial_twice_in_a_pack_mode_turn_is_one_undo_step() {
    let h = start_pack_tab(SET).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    in_pack_mode_with_kpi(&c).await;
    let session = std::sync::Arc::clone(&h.session);
    let before = tokio::task::spawn_blocking(move || {
        let mut s = session.lock().unwrap();
        s.begin_turn(1, "deux essais");
        s.snapshot().undo_label
    })
    .await
    .unwrap();
    call(&c, "set_widget_trial", json!({ "name": "kpi", "values": { "value": "12" } })).await;
    let r = json_of(&call(&c, "set_widget_trial", json!({ "name": "kpi", "values": { "value": "34" } })).await);
    assert_that!(r["trials"]["value"].as_str()).is_equal_to(Some("34"));
    let session = std::sync::Arc::clone(&h.session);
    tokio::task::spawn_blocking(move || {
        let mut s = session.lock().unwrap();
        s.end_turn();
        assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Agent : deux essais"));
        s.undo().unwrap();
        assert_that!(s.snapshot().undo_label).is_equal_to(before);
    })
    .await
    .unwrap();
    let trials = json_of(&call(&c, "set_widget_trial", json!({ "name": "kpi", "values": {} })).await);
    assert_that!(trials["trials"].as_object().map(serde_json::Map::len)).is_equal_to(Some(0));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_trial_in_deck_mode_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "set_widget", json!({ "name": "_commun", "source": COMMON })).await;
    call(&c, "set_widget", json!({ "name": "kpi", "source": KPI })).await;
    let msg = error_text(&c, "set_widget_trial", json!({ "name": "kpi", "values": { "value": "12" } })).await;
    assert_that!(msg).is_equal_to(h.localizer.tr("session-not-editing-pack"));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_and_remove_widget_in_pack_mode_emit_widget_touched() {
    let h = start_pack_tab(SET).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
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
    let h = start_pack_tab(SET).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let mut rx = h.hub.subscribe();
    call(&c, "set_widget", json!({ "name": "kpi", "source": "a\0b" })).await;
    assert_that!(touched(&mut rx)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_widget_twice_in_a_pack_mode_turn_is_one_undo_step() {
    let h = start_pack_tab(SET).await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.session.lock().unwrap().begin_turn(1, "deux widgets");
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

/// Widget de pack des tests à liste imbriquée : étapes à tâches (`fixtures/widgets/plan-test.typ`).
const PLAN: &str = include_str!("../../../fixtures/widgets/plan-test.typ");

/// Les ids des tâches de l'étape `step` de l'élément `pl` de `s-constat`.
fn task_ids(h: &Harness, step: &str) -> Vec<String> {
    let s = h.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(pl) = s.deck().slide("s-constat").and_then(|sl| sl.locate("pl")).map(|l| l.element.clone()) else {
        panic!("plan absent")
    };
    let steps = pl.params.list("etapes").unwrap_or_default();
    let Some(it) = steps.iter().find(|i| i.id == step) else { panic!("étape {step} absente") };
    it.values.list("taches").unwrap_or_default().iter().map(|t| t.id.clone()).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_edit_list_on_a_nested_path_edits_the_step_tasks_and_undoes() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "set_widget", json!({ "name": "plan", "source": PLAN })).await;
    let add = json!({ "op": "add_element", "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "defaults": true, "element": { "id": "pl", "kind": "deck/plan" } });
    call(&c, "apply", json!({ "ops": [add] })).await;
    let edit = |change: Value| {
        json!({ "ops": [{ "op": "edit_list", "target": { "slide": "s-constat", "element": "pl" },
            "param": "etapes/d0/taches", "change": change }] })
    };
    let r =
        call(&c, "apply", edit(json!({ "op": "insert", "index": 0, "id": "n", "values": { "texte": "Neuf" } }))).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    call(&c, "apply", edit(json!({ "op": "move", "item": "n", "to": 2 }))).await;
    assert_that!(task_ids(&h, "d0")).is_equal_to(vec!["d0".to_owned(), "d1".into(), "n".into()]);
    let undone = h.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner).undo();
    assert_that!(undone.is_ok()).is_true();
    assert_that!(task_ids(&h, "d0")).is_equal_to(vec!["n".to_owned(), "d0".into(), "d1".into()]);
    let bad = call(&c, "apply", edit(json!({ "op": "remove", "item": "absente" }))).await;
    assert_that!(bad.is_error).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn deck_outline_lists_the_pinned_widget_and_layout_packs_by_namespace() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["widget_packs"]["base"]["id"].is_string()).is_true();
    assert_that!(outline["layout_packs"]["essentiel"]["id"].is_string()).is_true();
    assert_that!(outline.get("template_set").is_none()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn widget_sources_of_a_pack_not_pinned_by_the_deck_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "widget_sources", json!({ "pack": "absent" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let bad = call(&c, "widget_sources", json!({ "pack": "Pas Valide" })).await;
    assert_that!(bad.is_error).is_equal_to(Some(true));
}
