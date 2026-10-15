mod common;

use common::*;
use oku_slides_mcp::McpEvent;
use rust_mcp_sdk::McpClient;
use serde_json::json;
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn apply_changes_deck_and_emits_changed() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": theme_ref("pro") }] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    assert_that!(h.session.lock().unwrap().deck().theme.as_ref().and_then(theme_name)).is_equal_to(Some("pro"));
    assert_that!(matches!(wait_event(&mut rx, Some).await, McpEvent::Changed)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_inside_turn_joins_the_turn_group() {
    let h = start().await;
    h.session.lock().unwrap().begin_turn(1, "plus sobre");
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": theme_ref("pro") }] })).await;
    call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": theme_ref("papier") }] })).await;
    let mut s = h.session.lock().unwrap();
    s.end_turn();
    assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Agent : plus sobre"));
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_invalid_op_returns_error_with_path() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(
        &c,
        "apply",
        json!({ "ops": [{ "op": "set_param", "target": { "slide": "s-libre", "element": "absent" }, "param": "body", "value": [] }] }),
    )
    .await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r).contains("absent")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_schema_accepts_every_op_kind_in_tool_list() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let tools = c.request_tool_list(None).await.unwrap().tools;
    let apply = tools.into_iter().find(|t| t.name == "apply").unwrap();
    let description = apply.description.clone().unwrap_or_default();
    assert_that!(description).contains("move_element moves an element with its children");
    let schema = serde_json::to_string(&apply.input_schema).unwrap();
    for op in
        ["set_param", "set_chart_option", "set_data", "set_table_option", "add_slide", "set_theme", "move_element"]
    {
        assert_that!(schema.contains(op)).named(op).is_true();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_list_has_show_as() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let tools = c.request_tool_list(None).await.unwrap().tools;
    assert_that!(tools.iter().any(|t| t.name == "show_as")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_set_param_through_mcp_changes_the_slide() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let ops = json!([{ "op": "set_param", "target": { "slide": "s-libre", "element": "e-texte" }, "param": "body",
        "value": [{ "runs": [{ "t": "Changé par MCP" }] }] }]);
    let r = call(&c, "apply", json!({ "ops": ops })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let s = h.session.lock().unwrap();
    let located = s.deck().slide("s-libre").unwrap().locate("e-texte").unwrap();
    let text = located.element.params.rich("body").unwrap()[0].runs[0].t.as_str();
    assert_that!(text).is_equal_to("Changé par MCP");
}

/// Le premier run du `body` de l'élément `id` de la slide `slide` est-il gras ?
fn bold_in(deck: &oku_slides_core::Deck, slide: &str, id: &str) -> Option<bool> {
    let located = deck.slide(slide)?.locate(id)?;
    Some(located.element.params.rich("body")?.first()?.runs.first()?.bold)
}

/// t-777 : l'agent déplace un élément d'id répété vers une autre slide, pendant
/// qu'une autre copie du deck le met en gras ; après fusion, il est déplacé et gras.
#[tokio::test(flavor = "multi_thread")]
async fn apply_move_element_through_mcp_keeps_the_concurrent_edit() {
    let h = start().await;
    let path = h.data().join("deck.oku-slides.pdf");
    let other = h.data().join("autre.oku-slides.pdf");
    let session = h.session.clone();
    let (saved, copy) = (path.clone(), other.clone());
    // `dup` dans trois slides, et une slide `s-neuve` vide ; puis une autre
    // copie, partie du même fichier, met en gras le `dup` de `s-constat`.
    tokio::task::spawn_blocking(move || {
        let dup = |slide: &str| {
            json!({ "op": "add_element", "slot": { "owner": { "slide": slide }, "param": "blocks" },
                "element": { "id": "dup", "kind": "text", "params": { "body": [{ "runs": [{ "t": slide }] }] } } })
        };
        let neuve = json!({ "op": "add_slide", "slide": { "id": "s-neuve", "kind": "content" } });
        let ops = serde_json::from_value(json!([dup("s-constat"), dup("s-priorites"), dup("s-libre"), neuve]))
            .unwrap_or_else(|e| panic!("ops : {e}"));
        let mut a = session.lock().unwrap_or_else(|e| panic!("session : {e}"));
        a.apply("Préparation", ops, None).unwrap_or_else(|e| panic!("préparation : {}", e.one_line()));
        a.save(Some(&saved)).unwrap_or_else(|e| panic!("enregistrement : {}", e.one_line()));
        drop(a);
        let (platform, data) = platform();
        let mut b = oku_slides_session::Session::new(oku_slides_session::Fonts::default(), &platform)
            .unwrap_or_else(|e| panic!("session B : {}", e.one_line()));
        b.open(&saved).unwrap_or_else(|e| panic!("ouverture : {}", e.one_line()));
        let bold = json!([{ "op": "set_param", "target": { "slide": "s-constat", "element": "dup" }, "param": "body",
            "value": [{ "runs": [{ "t": "s-constat", "bold": true }] }] }]);
        let bold = serde_json::from_value(bold).unwrap_or_else(|e| panic!("op : {e}"));
        b.apply("Gras", bold, None).unwrap_or_else(|e| panic!("gras : {}", e.one_line()));
        b.save(Some(&copy)).unwrap_or_else(|e| panic!("copie : {}", e.one_line()));
        // La plateforme porte un runtime : libérée ici, hors du fil async.
        drop((b, platform, data));
    })
    .await
    .unwrap_or_else(|e| panic!("préparation : {e}"));

    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let ops = json!([{ "op": "move_element", "slide": "s-constat", "element": "dup",
        "to": { "owner": { "slide": "s-neuve" }, "param": "blocks" } }]);
    let r = call(&c, "apply", json!({ "ops": ops })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));

    let session = h.session.clone();
    tokio::task::spawn_blocking(move || {
        let mut a = session.lock().unwrap_or_else(|e| panic!("session : {e}"));
        a.merge_file(&other).unwrap_or_else(|e| panic!("fusion : {}", e.one_line()));
    })
    .await
    .unwrap_or_else(|e| panic!("fusion : {e}"));

    let s = h.session.lock().unwrap();
    let deck = s.deck();
    assert_that!(bold_in(deck, "s-constat", "dup")).is_none();
    assert_that!(bold_in(deck, "s-neuve", "dup")).is_equal_to(Some(true));
    assert_that!(bold_in(deck, "s-priorites", "dup")).is_equal_to(Some(false));
    assert_that!(bold_in(deck, "s-libre", "dup")).is_equal_to(Some(false));
}

#[tokio::test(flavor = "multi_thread")]
async fn focus_emits_focus_event_without_changing_deck() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "focus", json!({ "slide": "s-libre", "element": "e-texte" })).await;
    match wait_event(&mut rx, Some).await {
        McpEvent::Focus { slide, element } => {
            assert_that!(slide.as_str()).is_equal_to("s-libre");
            assert_that!(element.as_deref()).is_equal_to(Some("e-texte"));
        }
        other => panic!("évènement inattendu : {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_then_close_templates_round_trips_mode() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let set = oku_slides_session::DEFAULT_PACK;
    call(&c, "edit_templates", json!({ "set": set })).await;
    assert_that!(h.session.lock().unwrap().snapshot().editing.is_some()).is_true();
    call(&c, "close_templates", json!({})).await;
    assert_that!(h.session.lock().unwrap().snapshot().editing.is_none()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn in_templates_outline_reports_the_edited_theme() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "edit_templates", json!({ "set": oku_slides_session::DEFAULT_PACK })).await;
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    let papier = theme_ref("papier").unwrap();
    assert_that!(outline["mode"].as_str()).is_equal_to(Some("templates"));
    assert_that!(outline["edited_theme"]).is_equal_to(json!({
        "id": papier.id, "version": papier.version, "name": "Papier", "dirty": false,
    }));
}

#[tokio::test(flavor = "multi_thread")]
async fn in_templates_set_theme_alone_chooses_the_edited_theme() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "edit_templates", json!({ "set": oku_slides_session::DEFAULT_PACK })).await;
    let r = call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": theme_ref("pro") }] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["edited_theme"]["name"].as_str()).is_equal_to(Some("Pro"));
}

#[tokio::test(flavor = "multi_thread")]
async fn in_templates_set_theme_with_other_ops_is_refused_with_a_dedicated_message() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "edit_templates", json!({ "set": oku_slides_session::DEFAULT_PACK })).await;
    let ops = json!([
        { "op": "set_theme", "theme": theme_ref("pro") },
        { "op": "set_token", "path": ["color", "accent"], "value": "#ff0000" },
    ]);
    let r = call(&c, "apply", json!({ "ops": ops })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).contains("set_token");
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["edited_theme"]["name"].as_str()).is_equal_to(Some("Papier"));
}

/// `edit_templates` sur `set` : refusé par le message qui renvoie vers
/// `templates`, sans passer en mode modèles (t-355).
async fn assert_wrong_set_points_to_templates(set: &str) {
    let h = start().await;
    let c =
        client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"));
    let r = call(&c, "edit_templates", json!({ "set": set })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-pack-id"));
    let editing = h.session.lock().unwrap_or_else(|e| panic!("session : {e}")).snapshot().editing.is_none();
    assert_that!(editing).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_templates_with_a_slug_points_to_templates() {
    assert_wrong_set_points_to_templates("essentiel").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_templates_with_an_unknown_id_points_to_templates() {
    assert_wrong_set_points_to_templates("00000000-0000-4000-8000-000000000001").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn in_history_outline_says_history_and_apply_is_refused() {
    let h = start().await;
    let theme = h.session.lock().unwrap().deck().theme.clone();
    h.session.lock().unwrap().handle(oku_slides_session::Request::HistoryOpen).unwrap();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let outline = json_of(&call(&c, "deck_outline", json!({})).await);
    assert_that!(outline["mode"].as_str()).is_equal_to(Some("history"));
    let r = call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": theme_ref("pro") }] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let mut s = h.session.lock().unwrap();
    s.handle(oku_slides_session::Request::HistoryClose).unwrap();
    assert_that!(s.deck().theme.clone()).is_equal_to(theme);
}
