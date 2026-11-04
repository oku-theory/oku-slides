//! The `update` and `pack_update` tools over MCP: the app port, the turn's after-turn list (t-999).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use std::sync::PoisonError;

use common::refusals::assert_refused;
use common::*;
use oku_slides_mcp::AfterTurn;
use oku_slides_session::agent::UpdateAction;
use oku_slides_session::{HOST_PACKS, Session};
use serde_json::json;
use speculoos::prelude::*;

/// `f` on the harness's session, off the async thread (the library writes to disk).
fn on_session<R>(h: &Harness, f: impl FnOnce(&mut Session) -> R) -> R {
    tokio::task::block_in_place(|| f(&mut h.session.lock().unwrap_or_else(PoisonError::into_inner)))
}

/// The open packs panel's JSON; empty when none is open.
fn packs_panel(h: &Harness) -> String {
    let panel = h.hub.surfaces().into_iter().find(|s| s.id == HOST_PACKS);
    panel.map(|s| serde_json::to_string(&s).unwrap_or_else(|e| panic!("panel: {e}"))).unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread")]
async fn update_check_returns_state() {
    let h = start().await;
    h.app.offer("5.2.0");
    let c = server(&h).await;
    let r = call(&c, "update", json!({ "action": "check" })).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "state": "offer", "version": "5.2.0", "installable": true }));
    assert_that!(lock(&h.app.asked).clone()).is_equal_to(vec![UpdateAction::Check]);
}

#[tokio::test(flavor = "multi_thread")]
async fn update_later_without_offer_refused_localized() {
    let h = start().await;
    let c = server(&h).await;
    let r = call(&c, "update", json!({ "action": "later" })).await;
    let fr = "Aucune mise à jour à reporter ou à installer ici : cherchez d'abord les mises à jour.";
    assert_refused(&h, &r, "agent-error-app-no-offer", fr);
}

#[tokio::test(flavor = "multi_thread")]
async fn update_install_asks_then_runs_after_turn() {
    let h = start().await;
    h.app.offer("5.2.0");
    let turn = h.hub.begin_turn();
    let (message, _, r) = guarded_message(&h, "update", json!({ "action": "install" }), true).await;
    assert_that!(message)
        .is_equal_to(h.localizer.tr_with("agent-confirm-update-install", &[("version", "5.2.0".into())]));
    assert_that!(json_of(&r)).is_equal_to(json!({ "install": "after_turn", "version": "5.2.0" }));
    // The tool itself never installs: the stack runs it once the turn has ended.
    assert_that!(h.app.installs()).is_equal_to(0);
    assert_that!(h.hub.take_after_turn(turn)).is_equal_to(vec![AfterTurn::Install]);
}

#[tokio::test(flavor = "multi_thread")]
async fn update_install_refused_does_nothing() {
    let h = start().await;
    h.app.offer("5.2.0");
    let turn = h.hub.begin_turn();
    let r = guarded(&h, "update", json!({ "action": "install" }), false).await;
    assert_refused(&h, &r, "agent-error-denied", "L'utilisateur a refusé.");
    assert_that!(h.hub.take_after_turn(turn)).is_empty();
    assert_that!(h.app.installs()).is_equal_to(0);
}

#[tokio::test(flavor = "multi_thread")]
async fn update_install_second_in_turn_refused_without_asking() {
    let h = start().await;
    h.app.offer("5.2.0");
    let turn = h.hub.begin_turn();
    assert_that!(h.hub.after_turn(turn, AfterTurn::Quit)).is_true();
    let c = server(&h).await;
    let r = call(&c, "update", json!({ "action": "install" })).await;
    let fr = "Cette réponse se termine déjà en quittant, en installant une mise à jour ou en fermant son onglet.";
    assert_refused(&h, &r, "agent-error-after-turn-taken", fr);
    assert_that!(h.hub.take_after_turn(turn)).is_equal_to(vec![AfterTurn::Quit]);
}

#[tokio::test(flavor = "multi_thread")]
async fn update_install_confirmed_after_the_turn_ended_is_turn_over() {
    let h = start().await;
    h.app.offer("5.2.0");
    let turn = h.hub.begin_turn();
    let hub = h.hub.clone();
    let (_, _, r) = guarded_message_after(&h, "update", json!({ "action": "install" }), true, move || {
        hub.end_turn(turn, "");
    })
    .await;
    let fr = "Le tour de cet appel est terminé : rien n'a été affiché.";
    assert_refused(&h, &r, "agent-error-turn-over", fr);
    assert_that!(h.hub.take_after_turn(turn)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn update_on_serve_unavailable_localized() {
    let h = start_on_serve().await;
    let c = server(&h).await;
    let r = call(&c, "update", json!({ "action": "check" })).await;
    let fr = "Seule l'app Oku Slides peut le faire ; ce n'est pas disponible ici.";
    assert_refused(&h, &r, "agent-error-app-unavailable", fr);
}

#[tokio::test(flavor = "multi_thread")]
async fn pack_update_use_mine_applies_and_prunes() {
    let h = start().await;
    let namespace = on_session(&h, oku_slides_session::pack_fixture::pin_behind_library);
    let shown = tokio::task::block_in_place(|| h.hub.show_packs(None)).unwrap_or_else(|e| panic!("packs panel: {e}"));
    assert_that!(shown).is_equal_to(oku_slides_mcp::ShowOutcome::Shown);
    let button = format!("widgets/{namespace}/use-mine");
    assert_that!(packs_panel(&h).contains(&button)).is_true();
    let c = server(&h).await;
    let r = call(&c, "pack_update", json!({ "pack": namespace.as_str(), "action": "use_mine" })).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "ok": true }));
    let drift = on_session(&h, |s| {
        let view = s.deck_packs().unwrap_or_else(|e| panic!("deck packs: {}", e.one_line()));
        view.widgets.iter().find(|p| p.namespace == namespace).map(|p| p.drift)
    });
    assert_that!(drift).is_equal_to(Some(None));
    // `prune` re-sent the panel: its card no longer offers the update.
    assert_that!(packs_panel(&h).contains(&button)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn pack_update_without_offer_refused() {
    let h = start().await;
    let c = server(&h).await;
    let r = call(&c, "pack_update", json!({ "pack": "base", "action": "use_mine" })).await;
    let fr = "Le panneau des packs ne propose pas cette mise à jour pour ce pack : voir show_packs.";
    assert_refused(&h, &r, "agent-error-no-pack-update", fr);
}
