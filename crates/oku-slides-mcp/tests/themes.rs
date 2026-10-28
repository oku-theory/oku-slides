//! Sélecteur de thème de l'hôte : montré, choisi, rafraîchi par `prune`.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use common::{action, next_surface, start, start_pack_tab, theme_tab_session, wait_event};
use oku_slides_a2ui::{ComponentKind, ServerBody};
use oku_slides_core::Namespace;
use oku_slides_mcp::{HubErrorKind, McpEvent};
use oku_slides_session::agent::Surface;
use oku_slides_session::{HOST_THEME_PICKER, PRO_NAMESPACE};
use serde_json::json;
use speculoos::prelude::*;

/// L'espace de noms du thème intégré Pro.
fn pro_namespace() -> Namespace {
    Namespace::parse(PRO_NAMESPACE).unwrap_or_else(|| panic!("espace de noms"))
}

/// Les `ThemeChoice` d'une surface : (nom, actif, a une miniature).
fn cards(s: &Surface) -> Vec<(String, bool, bool)> {
    s.messages
        .iter()
        .filter_map(|m| match &m.body {
            ServerBody::UpdateComponents(u) => Some(u.components.clone()),
            _ => None,
        })
        .flatten()
        .filter_map(|c| match c.kind {
            ComponentKind::ThemeChoice { name, active, svg, .. } => Some((name, active, svg.is_some())),
            _ => None,
        })
        .collect()
}

fn active_names(s: &Surface) -> Vec<String> {
    cards(s).into_iter().filter(|(_, active, _)| *active).map(|(n, ..)| n).collect()
}

fn show(h: &common::Harness) -> bool {
    tokio::task::block_in_place(|| h.hub.show_themes(Some("s-libre")))
        .unwrap_or_else(|e| panic!("sélecteur de thème : {e}"))
}

fn pick(h: &common::Harness, theme: &str) -> Result<(), oku_slides_mcp::HubError> {
    tokio::task::block_in_place(|| h.hub.act(action(HOST_THEME_PICKER, "theme", &json!({ "theme": theme }))))
}

fn open(h: &common::Harness) -> bool {
    h.hub.surfaces().iter().any(|s| s.id == HOST_THEME_PICKER)
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_puts_the_picker_live_outside_any_turn() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    assert_that!(show(&h)).is_true();
    let surface = next_surface(&mut rx).await;
    assert_that!((surface.id.as_str(), surface.live, surface.turn)).is_equal_to((HOST_THEME_PICKER, true, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_twice_replaces_the_picker_without_closing_it() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    next_surface(&mut rx).await;
    show(&h);
    let event = wait_event(&mut rx, |e| match e {
        McpEvent::CloseSurface { id } => Some(format!("close {id}")),
        McpEvent::Surface { surface } => Some(format!("show {}", surface.id)),
        _ => None,
    })
    .await;
    assert_that!(event).is_equal_to(format!("show {HOST_THEME_PICKER}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_outside_deck_mode_shows_nothing() {
    let h = start_pack_tab(oku_slides_session::pack::ESSENTIEL_PACK).await;
    assert_that!((show(&h), open(&h))).is_equal_to((false, false));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_sets_the_deck_theme() {
    let h = start().await;
    show(&h);
    pick(&h, PRO_NAMESPACE).unwrap();
    assert_that!(h.session.lock().unwrap().deck().theme.as_ref().map(Namespace::as_str))
        .is_equal_to(Some(PRO_NAMESPACE));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_is_labelled_as_a_theme_change() {
    let h = start().await;
    show(&h);
    pick(&h, PRO_NAMESPACE).unwrap();
    let label = h.localizer.tr("session-undo-theme");
    assert_that!(h.session.lock().unwrap().snapshot().undo_label).is_equal_to(Some(label));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_emits_changed_then_the_picker_with_it_active() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    next_surface(&mut rx).await;
    pick(&h, PRO_NAMESPACE).unwrap();
    wait_event(&mut rx, |e| matches!(e, McpEvent::Changed).then_some(())).await;
    let rebuilt = next_surface(&mut rx).await;
    assert_that!(active_names(&rebuilt)).is_equal_to(vec!["Pro".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_keeps_the_cached_thumbnails() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    let first = next_surface(&mut rx).await;
    pick(&h, PRO_NAMESPACE).unwrap();
    let rebuilt = next_surface(&mut rx).await;
    let thumbs = |s: &Surface| cards(s).into_iter().map(|(n, _, svg)| (n, svg)).collect::<Vec<_>>();
    assert_that!(thumbs(&rebuilt)).is_equal_to(thumbs(&first));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_malformed_namespace_is_incomplete() {
    let h = start().await;
    show(&h);
    assert_that!(pick(&h, "Pas Un Espace").unwrap_err().kind()).is_equal_to(HubErrorKind::Incomplete);
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_malformed_namespace_keeps_the_picker() {
    let h = start().await;
    show(&h);
    let _ = pick(&h, "Pas Un Espace");
    assert_that!(open(&h)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_an_unknown_theme_is_a_session_error() {
    let h = start().await;
    show(&h);
    let refused = pick(&h, "inconnu").unwrap_err();
    assert_that!(refused.kind()).is_equal_to(HubErrorKind::Session);
}

#[tokio::test(flavor = "multi_thread")]
async fn dismiss_closes_the_picker() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    h.hub.act(action(HOST_THEME_PICKER, "dismiss", &json!({}))).unwrap();
    wait_event(&mut rx, |e| matches!(e, McpEvent::CloseSurface { id } if id == HOST_THEME_PICKER).then_some(())).await;
    assert_that!(open(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_after_an_outside_theme_change_marks_it_active() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    next_surface(&mut rx).await;
    tokio::task::block_in_place(|| h.session.lock().unwrap().set_theme(&pro_namespace()).unwrap());
    tokio::task::block_in_place(|| h.hub.prune());
    let refreshed = next_surface(&mut rx).await;
    assert_that!(active_names(&refreshed)).is_equal_to(vec!["Pro".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_in_a_theme_tab_closes_the_picker() {
    let h = start().await;
    show(&h);
    // The tab's session becomes a theme tab: a session turns into one only when fresh.
    let _data = tokio::task::block_in_place(|| {
        let (_platform, tab, data) = theme_tab_session(oku_slides_session::PAPIER_THEME);
        *h.session.lock().unwrap() = tab;
        data
    });
    tokio::task::block_in_place(|| h.hub.prune());
    assert_that!(open(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_after_dismiss_does_not_reopen_the_picker() {
    let h = start().await;
    show(&h);
    h.hub.act(action(HOST_THEME_PICKER, "dismiss", &json!({}))).unwrap();
    tokio::task::block_in_place(|| h.session.lock().unwrap().set_theme(&pro_namespace()).unwrap());
    tokio::task::block_in_place(|| h.hub.prune());
    assert_that!(open(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_of_set_theme_by_the_agent_marks_it_active() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    next_surface(&mut rx).await;
    let c = common::client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let theme = common::theme_ref("pro");
    common::call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": theme }] })).await;
    let refreshed = next_surface(&mut rx).await;
    assert_that!(active_names(&refreshed)).is_equal_to(vec!["Pro".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_after_the_shown_slide_is_removed_closes_the_picker() {
    let h = start().await;
    show(&h);
    tokio::task::block_in_place(|| {
        h.session
            .lock()
            .unwrap()
            .apply("retrait", vec![oku_slides_core::Op::RemoveSlide { slide: "s-libre".into() }], None)
            .unwrap();
    });
    tokio::task::block_in_place(|| h.hub.prune());
    assert_that!(open(&h)).is_false();
}

/// La slide que montrent les `ThemeChoice` d'une surface.
fn shown_slides(s: &Surface) -> Vec<String> {
    s.messages
        .iter()
        .filter_map(|m| match &m.body {
            ServerBody::UpdateComponents(u) => Some(u.components.clone()),
            _ => None,
        })
        .flatten()
        .filter_map(|c| match c.kind {
            ComponentKind::ThemeChoice { slide, .. } => Some(slide),
            _ => None,
        })
        .collect()
}

/// L'agent appelle l'outil `show_themes` avec ces arguments.
async fn call_show_themes(h: &common::Harness, args: serde_json::Value) -> rust_mcp_sdk::schema::CallToolResult {
    let c = common::client(h.server.url(), Some(h.server.authorization()))
        .await
        .unwrap_or_else(|e| panic!("client MCP : {e}"));
    common::call(&c, "show_themes", args).await
}

/// L'agent ouvre le sélecteur ; l'appel doit réussir.
async fn shown(h: &common::Harness, args: serde_json::Value) {
    let r = call_show_themes(h, args).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_tool_returns_the_picker_id_at_once() {
    let h = start().await;
    let r = call_show_themes(&h, json!({})).await;
    assert_that!(common::json_of(&r)).is_equal_to(json!({ "shown": HOST_THEME_PICKER }));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_tool_puts_the_picker_live_outside_any_turn() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    shown(&h, json!({})).await;
    let surface = next_surface(&mut rx).await;
    assert_that!((surface.id.as_str(), surface.live, surface.turn)).is_equal_to((HOST_THEME_PICKER, true, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_tool_renders_the_given_slide() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    shown(&h, json!({ "slide": "s-libre" })).await;
    let slides = shown_slides(&next_surface(&mut rx).await);
    assert_that!(slides).is_not_empty();
    assert_that!(slides.clone()).is_equal_to(vec!["s-libre".to_owned(); slides.len()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_tool_keeps_the_picker_after_the_turn_closes() {
    let h = start().await;
    let n = h.hub.begin_turn();
    shown(&h, json!({})).await;
    h.hub.close_turn(n);
    assert_that!(open(&h)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_tool_pick_is_noted_for_the_agent() {
    let h = start().await;
    shown(&h, json!({})).await;
    pick(&h, PRO_NAMESPACE).unwrap();
    let ctx = h.session.lock().unwrap().agent_context(&oku_slides_session::agent::AgentUi::default());
    assert_that!(ctx).contains("theme picked: \"Pro\"");
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_tool_outside_deck_mode_says_so_in_french() {
    let h = start_pack_tab(oku_slides_session::pack::ESSENTIEL_PACK).await;
    let r = call_show_themes(&h, json!({})).await;
    let said = (r.is_error, common::text_of(&r), open(&h));
    let want = (Some(true), h.localizer.tr("agent-error-themes-deck-mode"), false);
    assert_that!(said).is_equal_to(want);
}

#[tokio::test(flavor = "multi_thread")]
async fn show_themes_tool_on_an_unknown_slide_is_refused_without_a_picker() {
    let h = start().await;
    let r = call_show_themes(&h, json!({ "slide": "s-inconnue" })).await;
    let said = (r.is_error, common::text_of(&r), open(&h));
    let want = (Some(true), "Slide introuvable : s-inconnue".to_owned(), false);
    assert_that!(said).is_equal_to(want);
}

/// Crée la variante « Marque » du thème Pro dans la bibliothèque (sans confirmation, hors outil).
fn with_child_theme(h: &common::Harness) {
    let pro_id = oku_slides_session::PRO_THEME;
    tokio::task::block_in_place(|| {
        let mut s = h.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        s.edit_theme(pro_id, Some("Marque".into()), Vec::new()).unwrap_or_else(|e| panic!("variante : {e}"));
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn deck_outline_lists_themes_by_namespace_with_extends() {
    let h = start().await;
    with_child_theme(&h);
    let c = common::client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let out = common::json_of(&common::call(&c, "deck_outline", json!({})).await);
    let themes = out["themes"].as_array().unwrap();
    let marque = themes.iter().find(|t| t["namespace"] == "marque").unwrap();
    assert_that!(marque["extends"].clone()).is_equal_to(json!(["pro"]));
    assert_that!(themes.iter().all(|t| t.get("deck_only").is_none() && t.get("id").is_none())).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn apply_set_theme_by_namespace_pins_the_closure() {
    let h = start().await;
    with_child_theme(&h);
    let c = common::client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = common::call(&c, "apply", json!({ "ops": [{ "op": "set_theme", "theme": "marque" }] })).await;
    assert_that!(r.is_error).is_not_equal_to(Some(true));
    let s = h.session.lock().unwrap();
    let pinned: Vec<&str> = s.deck().themes.keys().map(Namespace::as_str).collect();
    assert_that!(pinned).is_equal_to(vec!["marque", "pro"]);
    assert_that!(s.deck().theme.as_ref().map(Namespace::as_str)).is_equal_to(Some("marque"));
}
