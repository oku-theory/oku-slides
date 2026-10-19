//! Sélecteur de thème de l'hôte : montré, choisi, rafraîchi par `prune`.

mod common;

use common::{action, next_surface, start, wait_event};
use oku_slides_a2ui::{ComponentKind, ServerBody};
use oku_slides_mcp::{HubErrorKind, McpEvent};
use oku_slides_session::agent::Surface;
use oku_slides_session::{HOST_THEME_PICKER, PRO_THEME};
use serde_json::json;
use speculoos::prelude::*;

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
    tokio::task::block_in_place(|| h.hub.show_themes(Some("s-libre".into())))
        .unwrap_or_else(|e| panic!("sélecteur de thème : {e}"))
}

fn pick(h: &common::Harness, theme: &str) -> Result<(), oku_slides_mcp::HubError> {
    tokio::task::block_in_place(|| h.hub.act(action(HOST_THEME_PICKER, "theme", json!({ "theme": theme }))))
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
    let h = start().await;
    tokio::task::block_in_place(|| {
        h.session.lock().unwrap().edit_templates(oku_slides_session::pack::DEFAULT_PACK).unwrap()
    });
    assert_that!((show(&h), open(&h))).is_equal_to((false, false));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_sets_the_deck_theme() {
    let h = start().await;
    show(&h);
    pick(&h, &PRO_THEME.to_string()).unwrap();
    assert_that!(h.session.lock().unwrap().deck().theme.as_ref().map(|r| r.id)).is_equal_to(Some(PRO_THEME));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_is_labelled_as_a_theme_change() {
    let h = start().await;
    show(&h);
    pick(&h, &PRO_THEME.to_string()).unwrap();
    let label = h.localizer.tr("session-undo-theme");
    assert_that!(h.session.lock().unwrap().snapshot().undo_label).is_equal_to(Some(label));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_theme_emits_changed_then_the_picker_with_it_active() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    next_surface(&mut rx).await;
    pick(&h, &PRO_THEME.to_string()).unwrap();
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
    pick(&h, &PRO_THEME.to_string()).unwrap();
    let rebuilt = next_surface(&mut rx).await;
    let thumbs = |s: &Surface| cards(s).into_iter().map(|(n, _, svg)| (n, svg)).collect::<Vec<_>>();
    assert_that!(thumbs(&rebuilt)).is_equal_to(thumbs(&first));
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_malformed_id_is_incomplete() {
    let h = start().await;
    show(&h);
    assert_that!(pick(&h, "pas-un-uuid").unwrap_err().kind()).is_equal_to(HubErrorKind::Incomplete);
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_a_malformed_id_keeps_the_picker() {
    let h = start().await;
    show(&h);
    let _ = pick(&h, "pas-un-uuid");
    assert_that!(open(&h)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn picking_an_unknown_theme_is_a_session_error() {
    let h = start().await;
    show(&h);
    let refused = pick(&h, &uuid::Uuid::from_u128(7).to_string()).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(HubErrorKind::Session);
}

#[tokio::test(flavor = "multi_thread")]
async fn dismiss_closes_the_picker() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    h.hub.act(action(HOST_THEME_PICKER, "dismiss", json!({}))).unwrap();
    wait_event(&mut rx, |e| matches!(e, McpEvent::CloseSurface { id } if id == HOST_THEME_PICKER).then_some(())).await;
    assert_that!(open(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_after_an_outside_theme_change_marks_it_active() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    show(&h);
    next_surface(&mut rx).await;
    tokio::task::block_in_place(|| h.session.lock().unwrap().set_theme(PRO_THEME).unwrap());
    tokio::task::block_in_place(|| h.hub.prune());
    let refreshed = next_surface(&mut rx).await;
    assert_that!(active_names(&refreshed)).is_equal_to(vec!["Pro".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_in_templates_mode_closes_the_picker() {
    let h = start().await;
    show(&h);
    tokio::task::block_in_place(|| {
        h.session.lock().unwrap().edit_templates(oku_slides_session::pack::DEFAULT_PACK).unwrap()
    });
    tokio::task::block_in_place(|| h.hub.prune());
    assert_that!(open(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_after_dismiss_does_not_reopen_the_picker() {
    let h = start().await;
    show(&h);
    h.hub.act(action(HOST_THEME_PICKER, "dismiss", json!({}))).unwrap();
    tokio::task::block_in_place(|| h.session.lock().unwrap().set_theme(PRO_THEME).unwrap());
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
            .unwrap()
    });
    tokio::task::block_in_place(|| h.hub.prune());
    assert_that!(open(&h)).is_false();
}
