//! The apply tool over MCP: deck ops, turn group, errors, schema.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use common::*;
use oku_slides_core::kinds;
use oku_slides_mcp::McpEvent;
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
async fn apply_tool_edit_appears_in_the_turn_traces() {
    let h = start().await;
    h.session.lock().unwrap().begin_turn(1, "fond");
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let style = json!({ "op": "set_style", "slide": "s-libre", "element": "e-fond", "style": {} });
    let r = call(&c, "apply", json!({ "ops": [style] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let traces = h.session.lock().unwrap().end_turn();
    assert_that!(traces.into_iter().map(|t| (t.slide, t.element)).collect::<Vec<_>>())
        .is_equal_to(vec![("s-libre".to_owned(), "e-fond".to_owned())]);
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
    let tools = c.list_tools().await.unwrap().tools;
    let apply = tools.into_iter().find(|t| t.name == "apply").unwrap();
    let description = apply.description.clone().unwrap_or_default();
    assert_that!(description).contains("move_element moves an element with its children");
    let schema = serde_json::to_string(&apply.input_schema).unwrap();
    for op in
        ["set_param", "set_chart_option", "edit_data", "set_table_option", "add_slide", "set_theme", "move_element"]
    {
        assert_that!(schema.contains(op)).named(op).is_true();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_list_has_edit_data_not_set_data() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let tools = c.list_tools().await.unwrap().tools;
    let apply = tools.iter().find(|t| t.name == "apply").unwrap();
    let schema = serde_json::to_string(&apply.input_schema).unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert_that!(names).does_not_contain("set_data");
    assert_that!(schema).contains("\"edit_data\"");
    assert_that!(schema).does_not_contain("\"set_data\"");
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_edit_data_on_chart_changes_cell() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let setup = json!([
        { "op": "add_data", "id": "d-mcp", "name": "mcp", "table": {
            "columns": [{ "id": "mois", "name": "Mois" }, { "id": "ventes", "name": "Ventes" }],
            "rows": [{ "id": "jan", "cells": ["Jan", 3] }] } },
        { "op": "add_element", "slot": { "owner": { "slide": "s-libre" }, "param": "blocks" },
          "element": { "id": "e-mcp", "kind": kinds::base::CHART, "params": { "option": {
            "data": "d-mcp", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } } },
    ]);
    let r = call(&c, "apply", json!({ "ops": setup })).await;
    assert_that!(r.is_error).named(&text_of(&r)).is_not_equal_to(Some(true));
    let edit = json!([{ "op": "edit_data", "data": "d-mcp",
        "change": { "op": "set_cell", "row": "jan", "column": "ventes", "value": 9 } }]);
    let r = call(&c, "apply", json!({ "ops": edit })).await;
    assert_that!(r.is_error).named(&text_of(&r)).is_not_equal_to(Some(true));
    let s = h.session.lock().unwrap();
    let dataset = s
        .deck()
        .data
        .get("d-mcp")
        .unwrap()
        .table()
        .unwrap_or_else(|| panic!("a manual data"))
        .to_dataset(None, 0..1, 100)
        .unwrap();
    assert_that!(dataset.source[1][1].as_f64()).is_equal_to(Some(9.0));
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_list_has_show_as() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let tools = c.list_tools().await.unwrap().tools;
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
                "element": { "id": "dup", "kind": kinds::base::TEXT, "params": { "body": [{ "runs": [{ "t": slide }] }] } } })
        };
        let neuve = json!({ "op": "add_slide", "slide": { "id": "s-neuve", "kind": kinds::essentiel::CONTENT } });
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

/// The agent never selects: naming an element is refused, so the agent learns it at once.
#[tokio::test(flavor = "multi_thread")]
async fn focus_tool_with_an_element_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "focus", json!({ "slide": "s-libre", "element": "e-texte" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn focus_tool_shows_the_slide() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "focus", json!({ "slide": "s-libre" })).await;
    match wait_event(&mut rx, Some).await {
        McpEvent::Focus { slide } => assert_that!(slide.as_deref()).is_equal_to(Some("s-libre")),
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn remove_pack_in_use_returns_the_localized_reason() {
    use oku_slides_i18n::Localize;
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "remove_pack", json!({ "kind": "widgets", "namespace": "base" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let refusal = oku_slides_core::remove_pack_refusal(
        h.session.lock().unwrap().deck(),
        oku_slides_core::PackKind::Widgets,
        &oku_slides_core::kinds::base::namespace(),
    )
    .unwrap();
    assert_that!(text_of(&r)).is_equal_to(refusal.localize(&h.localizer));
    h.live.set(std::sync::Arc::new(oku_slides_i18n::Localizer::new(Some("en"))));
    let english = call(&c, "remove_pack", json!({ "kind": "widgets", "namespace": "base" })).await;
    assert_that!(text_of(&english)).is_not_equal_to(text_of(&r));
}

#[tokio::test(flavor = "multi_thread")]
async fn remove_pack_of_an_unused_pack_unpins_it() {
    let h = start_with_library_widget_pack("acme").await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let added = call(&c, "add_pack", json!({ "kind": "widgets", "namespace": "acme" })).await;
    assert_that!(added.is_error).is_not_equal_to(Some(true));
    let acme = || h.session.lock().unwrap().deck().widgets.keys().any(|ns| ns.as_str() == "acme");
    assert_that!(acme()).is_true();
    let removed = call(&c, "remove_pack", json!({ "kind": "widgets", "namespace": "acme" })).await;
    assert_that!(removed.is_error).is_not_equal_to(Some(true));
    assert_that!(acme()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn add_pack_emits_changed() {
    let h = start_with_library_widget_pack("acme").await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "add_pack", json!({ "kind": "widgets", "namespace": "acme" })).await;
    assert_that!(matches!(wait_event(&mut rx, Some).await, McpEvent::Changed)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn show_packs_tool_shows_the_panel() {
    let h = start().await;
    h.hub.begin_turn();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "show_packs", json!({})).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "shown": oku_slides_session::HOST_PACKS }));
    assert_that!(open_ids(&h)).contains(oku_slides_session::HOST_PACKS.to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn show_packs_tool_outside_deck_mode_is_refused() {
    let h = common::kinds::start_help_tab().await;
    h.hub.begin_turn();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "show_packs", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-packs-deck-mode"));
    h.live.set(std::sync::Arc::new(oku_slides_i18n::Localizer::new(Some("en"))));
    let english = call(&c, "show_packs", json!({})).await;
    assert_that!(text_of(&english)).is_not_equal_to(text_of(&r));
}

/// `open_pack` of `pack`: refused by the message that points to `packs`,
/// without entering pack mode (t-355).
async fn assert_wrong_pack_points_to_packs(pack: &str) {
    let h = start().await;
    let c =
        client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"));
    let r = call(&c, "open_pack", json!({ "pack": pack })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to(h.localizer.tr("agent-error-pack-id"));
    let editing = h.session.lock().unwrap_or_else(|e| panic!("session : {e}")).snapshot().editing.is_none();
    assert_that!(editing).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_with_a_slug_points_to_packs() {
    assert_wrong_pack_points_to_packs("essentiel").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn open_pack_with_an_unknown_id_points_to_packs() {
    assert_wrong_pack_points_to_packs("00000000-0000-4000-8000-000000000001").await;
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

#[tokio::test(flavor = "multi_thread")]
async fn apply_tool_schema_lists_data_ops() {
    let h = start().await;
    let c = server(&h).await;
    let schema = tool_schema(&c, "apply").await.to_string();
    for op in ["add_data", "bind_data", "remove_data", "rename_data", "set_column_type", "edit_data"] {
        assert_that!(schema.contains(&format!("\"{op}\""))).named(op).is_true();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_batch_over_100000_cells_is_refused() {
    let h = start().await;
    let c = server(&h).await;
    let rows: Vec<serde_json::Value> = (0..50_001).map(|r| json!({ "id": format!("r{r}"), "cells": [r, r] })).collect();
    let table = json!({ "columns": [{ "id": "a", "name": "A" }, { "id": "b", "name": "B" }], "rows": rows });
    let r =
        call(&c, "apply", json!({ "ops": [{ "op": "add_data", "id": "d1", "name": "grand", "table": table }] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let reply = json_of(&r);
    assert_that!(reply["untrusted"]).is_equal_to(json!(true));
    let expected =
        h.localizer.tr_with("op-data-batch-too-large", &[("count", 100_002.into()), ("max", 100_000.into())]);
    assert_that!(reply["error"].as_str().unwrap_or_default()).contains(expected.as_str());
    assert_that!(h.session.lock().unwrap().deck().data.is_empty()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_remove_data_in_use_returns_view_list_untrusted() {
    let h = start().await;
    let c = server(&h).await;
    let setup = json!([
        { "op": "add_data", "id": "d1", "name": "ventes", "table": {
            "columns": [{ "id": "mois", "name": "Mois" }, { "id": "ventes", "name": "Ventes" }],
            "rows": [{ "id": "jan", "cells": ["Jan", 3] }] } },
        { "op": "add_element", "slot": { "owner": { "slide": "s-libre" }, "param": "blocks" },
          "element": { "id": "e-vue", "kind": kinds::base::CHART, "params": { "option": {
            "data": "d1", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } } },
    ]);
    let r = call(&c, "apply", json!({ "ops": setup })).await;
    assert_that!(r.is_error).named(&text_of(&r)).is_not_equal_to(Some(true));
    let r = call(&c, "apply", json!({ "ops": [{ "op": "remove_data", "data": "d1" }] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let reply = json_of(&r);
    assert_that!(reply["untrusted"]).is_equal_to(json!(true));
    let error = reply["error"].as_str().unwrap_or_default().to_owned();
    assert_that!(error.as_str()).contains("Cette donnée est affichée par");
    assert_that!(error.as_str()).contains("e-vue");
    assert_that!(h.session.lock().unwrap().deck().data.len()).is_equal_to(1);
}

/// A non-data op refused with a message that names a data's column still comes back untrusted:
/// the column is the user's text.
#[tokio::test(flavor = "multi_thread")]
async fn apply_non_data_op_refusal_naming_a_column_is_untrusted() {
    let h = start().await;
    let c = server(&h).await;
    let setup = json!([
        { "op": "add_data", "id": "d1", "name": "ventes", "table": {
            "columns": [{ "id": "mois", "name": "Mois" }, { "id": "ventes", "name": "Ventes" }],
            "rows": [{ "id": "jan", "cells": ["Jan", 3] }] } },
        { "op": "add_element", "slot": { "owner": { "slide": "s-libre" }, "param": "blocks" },
          "element": { "id": "e-vue", "kind": kinds::base::CHART, "params": { "option": {
            "data": "d1", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } } },
    ]);
    let r = call(&c, "apply", json!({ "ops": setup })).await;
    assert_that!(r.is_error).named(&text_of(&r)).is_not_equal_to(Some(true));
    let refused = json!([{ "op": "set_chart_option", "target": { "slide": "s-libre", "element": "e-vue" },
        "param": "option", "patch": { "columns": ["mois", "colonne-absente"] } }]);
    let r = call(&c, "apply", json!({ "ops": refused })).await;
    assert_that!(r.is_error).named(&text_of(&r)).is_equal_to(Some(true));
    assert_that!(untrusted_error_of(&r)).contains("colonne-absente");
}
