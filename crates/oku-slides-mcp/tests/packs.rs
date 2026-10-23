//! Panneau des packs du deck : montré par le hub, ses actions, rafraîchi par `prune`.

mod common;

use common::{action, component_ids, last_surface, next_surface, start, start_with_library_widget_pack};
use oku_slides_a2ui::{ComponentKind, Dynamic, ServerBody};
use oku_slides_mcp::HubErrorKind;
use oku_slides_session::agent::Surface;
use oku_slides_session::pack::ESSENTIEL_PACK;
use oku_slides_session::{HOST_PACKS, HOST_THEME_PICKER, PAPIER_NAMESPACE, PRO_NAMESPACE};
use serde_json::json;
use speculoos::prelude::*;

fn show(h: &common::Harness) -> bool {
    tokio::task::block_in_place(|| h.hub.show_packs()).unwrap_or_else(|e| panic!("panneau des packs : {e}"))
}

fn open(h: &common::Harness) -> bool {
    h.hub.surfaces().iter().any(|s| s.id == HOST_PACKS)
}

fn edit_essentiel(h: &common::Harness) {
    tokio::task::block_in_place(|| {
        h.session.lock().unwrap_or_else(|e| panic!("session : {e}")).edit_pack(ESSENTIEL_PACK)
    })
    .unwrap_or_else(|e| panic!("mode pack : {e}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_packs_puts_the_panel_live_outside_any_turn() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    assert_that!(show(&h)).is_true();
    let surface = next_surface(&mut rx).await;
    assert_that!((surface.id.as_str(), surface.live, surface.turn)).is_equal_to((HOST_PACKS, true, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn packs_add_event_pins_and_the_last_panel_lists_it() {
    let h = start_with_library_widget_pack("acme").await;
    let mut rx = h.hub.subscribe();
    show(&h);
    let _ = next_surface(&mut rx).await;
    tokio::task::block_in_place(|| {
        h.hub.act(action(HOST_PACKS, "add_pack", json!({ "kind": "widgets", "namespace": "acme" })))
    })
    .unwrap_or_else(|e| panic!("ajout : {e}"));
    let last = last_surface(&mut rx, HOST_PACKS);
    assert_that!(component_ids(&last)).contains("widgets/acme".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn packs_malformed_context_is_incomplete() {
    let h = start().await;
    show(&h);
    let r = tokio::task::block_in_place(|| h.hub.act(action(HOST_PACKS, "add_pack", json!({ "kind": "themes" }))));
    assert_that!(r.map_err(|e| e.kind())).is_equal_to(Err(HubErrorKind::Incomplete));
}

#[tokio::test(flavor = "multi_thread")]
async fn packs_malformed_context_keeps_the_panel() {
    let h = start().await;
    show(&h);
    let _ = tokio::task::block_in_place(|| h.hub.act(action(HOST_PACKS, "add_pack", json!({ "kind": "themes" }))));
    assert_that!(open(&h)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn packs_refused_removal_is_a_session_error() {
    let h = start().await;
    show(&h);
    let r = tokio::task::block_in_place(|| {
        h.hub.act(action(HOST_PACKS, "remove_pack", json!({ "kind": "widgets", "namespace": "base" })))
    });
    assert_that!(r.map_err(|e| e.kind())).is_equal_to(Err(HubErrorKind::Session));
}

#[tokio::test(flavor = "multi_thread")]
async fn packs_dismiss_closes_the_panel() {
    let h = start().await;
    show(&h);
    h.hub.act(action(HOST_PACKS, "dismiss", json!({}))).unwrap_or_else(|e| panic!("fermeture : {e}"));
    assert_that!(open(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn packs_panel_closes_when_pack_mode_starts() {
    let h = start().await;
    show(&h);
    edit_essentiel(&h);
    tokio::task::block_in_place(|| h.hub.prune());
    assert_that!(open(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_without_change_emits_no_new_panel() {
    let h = start().await;
    show(&h);
    let mut rx = h.hub.subscribe();
    tokio::task::block_in_place(|| h.hub.prune());
    assert_that!(rx.try_recv().is_err()).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_after_an_outside_add_rebuilds_the_panel() {
    let h = start_with_library_widget_pack("acme").await;
    show(&h);
    let mut rx = h.hub.subscribe();
    let ns = oku_slides_core::Namespace::parse("acme").unwrap_or_else(|| panic!("espace de noms"));
    tokio::task::block_in_place(|| {
        let mut s = h.session.lock().unwrap_or_else(|e| panic!("session : {e}"));
        s.add_library_pack(oku_slides_core::PackKind::Widgets, &ns)
    })
    .unwrap_or_else(|e| panic!("ajout : {e}"));
    tokio::task::block_in_place(|| h.hub.prune());
    assert_that!(component_ids(&last_surface(&mut rx, HOST_PACKS))).contains("widgets/acme".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn show_packs_outside_deck_mode_shows_nothing() {
    let h = start().await;
    edit_essentiel(&h);
    assert_that!((show(&h), open(&h))).is_equal_to((false, false));
}

/// Le texte littéral du composant `id` d'une surface.
fn text_of_component(s: &Surface, id: &str) -> Option<String> {
    s.messages
        .iter()
        .filter_map(|m| match &m.body {
            ServerBody::UpdateComponents(u) => Some(u.components.clone()),
            _ => None,
        })
        .flatten()
        .find(|c| c.id == id)
        .and_then(|c| match c.kind {
            ComponentKind::Text { text: Dynamic::Literal(t), .. } => Some(t),
            _ => None,
        })
}

/// Les noms des cartes actives du sélecteur de thème.
fn active_cards(s: &Surface) -> Vec<String> {
    s.messages
        .iter()
        .filter_map(|m| match &m.body {
            ServerBody::UpdateComponents(u) => Some(u.components.clone()),
            _ => None,
        })
        .flatten()
        .filter_map(|c| match c.kind {
            ComponentKind::ThemeChoice { name, active: true, .. } => Some(name),
            _ => None,
        })
        .collect()
}

/// Le sélecteur de thème et le panneau des packs, montrés ensemble.
fn show_both(h: &common::Harness) {
    let picker = tokio::task::block_in_place(|| h.hub.show_themes(Some("s-libre".into())))
        .unwrap_or_else(|e| panic!("sélecteur de thème : {e}"));
    assert_that!((picker, show(h))).is_equal_to((true, true));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_in_the_picker_refreshes_the_packs_panel() {
    let h = start().await;
    show_both(&h);
    let mut rx = h.hub.subscribe();
    tokio::task::block_in_place(|| h.hub.act(action(HOST_THEME_PICKER, "theme", json!({ "theme": PRO_NAMESPACE }))))
        .unwrap_or_else(|e| panic!("choix du thème : {e}"));
    let panel = last_surface(&mut rx, HOST_PACKS);
    assert_that!(text_of_component(&panel, "theme-name")).is_equal_to(Some("Pro".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_in_the_packs_panel_refreshes_the_picker() {
    let h = start().await;
    show_both(&h);
    let mut rx = h.hub.subscribe();
    tokio::task::block_in_place(|| h.hub.act(action(HOST_PACKS, "theme", json!({ "theme": PRO_NAMESPACE }))))
        .unwrap_or_else(|e| panic!("choix du thème : {e}"));
    let picker = last_surface(&mut rx, HOST_THEME_PICKER);
    assert_that!(active_cards(&picker)).is_equal_to(vec!["Pro".to_owned()]);
}

/// L'identifiant d'une surface que l'agent vient de montrer par `tool`.
async fn shown_by(h: &common::Harness, tool: &str, args: serde_json::Value) -> String {
    let c =
        common::client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client : {e}"));
    let r = common::call(&c, tool, args).await;
    common::json_of(&r)["shown"].as_str().unwrap_or_else(|| panic!("{tool} sans surface")).to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_variant_refreshes_the_packs_panel() {
    let h = start().await;
    show(&h);
    let variant = |label: &str, theme: &str| json!({ "label": label, "ops": [{ "op": "set_theme", "theme": theme }] });
    let variants = [variant("Pro", PRO_NAMESPACE), variant("Papier", PAPIER_NAMESPACE)];
    let id = shown_by(&h, "show_variants", json!({ "slide": "s-libre", "variants": variants })).await;
    let mut rx = h.hub.subscribe();
    tokio::task::block_in_place(|| h.hub.act(action(&id, "variant", json!({ "index": 0 }))))
        .unwrap_or_else(|e| panic!("variante : {e}"));
    let panel = last_surface(&mut rx, HOST_PACKS);
    assert_that!(text_of_component(&panel, "theme-name")).is_equal_to(Some("Pro".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn changing_a_param_refreshes_the_packs_panel() {
    let h = start().await;
    show(&h);
    let field = json!({ "label": "Thème", "kind": "select", "value": "papier",
        "op_template": { "op": "set_theme", "theme": "$value" },
        "options": [{ "id": "papier", "label": "Papier" }, { "id": PRO_NAMESPACE, "label": "Pro" }] });
    let id = shown_by(&h, "show_params", json!({ "title": "Apparence", "slide": "s-libre", "fields": [field] })).await;
    let mut rx = h.hub.subscribe();
    tokio::task::block_in_place(|| h.hub.act(action(&id, "param", json!({ "field": 0, "value": PRO_NAMESPACE }))))
        .unwrap_or_else(|e| panic!("réglage : {e}"));
    let panel = last_surface(&mut rx, HOST_PACKS);
    assert_that!(text_of_component(&panel, "theme-name")).is_equal_to(Some("Pro".to_owned()));
}
