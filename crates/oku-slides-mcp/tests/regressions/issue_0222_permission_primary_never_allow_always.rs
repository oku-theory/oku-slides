//! t-222 : le bouton principal d'une permission était la première option de
//! l'agent, souvent « toujours autoriser ». Une frappe ou un clic distrait
//! accordait une permission durable.

use oku_slides_mcp::{AnswerOutcome, SurfaceAction};
use oku_slides_session::agent::{PermissionChoice, PermissionKind};
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use crate::common::{next_surface, start};

fn choices() -> Vec<PermissionChoice> {
    [
        ("Toujours autoriser", PermissionKind::AllowAlways),
        ("Autoriser une fois", PermissionKind::AllowOnce),
        ("Refuser", PermissionKind::RejectOnce),
    ]
    .into_iter()
    .map(|(name, kind)| PermissionChoice { name: name.into(), kind })
    .collect()
}

/// Le composant `id` de la surface.
fn component(surface: &Value, id: &str) -> Value {
    surface["messages"]
        .as_array()
        .and_then(|m| m.iter().find_map(|m| m.get("updateComponents")))
        .and_then(|u| u["components"].as_array())
        .and_then(|cs| cs.iter().find(|c| c["id"] == id))
        .cloned()
        .unwrap_or_else(|| panic!("composant absent : {id}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn permission_never_offers_allow_always_as_the_default() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let asking = tokio::spawn({
        let hub = h.hub.clone();
        async move { hub.permission("Écrire le fichier ?", &choices()).await }
    });
    let surface = serde_json::to_value(next_surface(&mut rx).await).unwrap_or_else(|e| panic!("surface : {e}"));

    // « Autoriser une fois » est le bouton principal et le premier ; « toujours autoriser » engage, et le dit.
    assert_that!(component(&surface, "buttons")["children"][0].clone()).is_equal_to(json!("option-1"));
    assert_that!(component(&surface, "option-1")["primary"].clone()).is_equal_to(json!(true));
    let always = component(&surface, "option-0");
    assert_that!(always.get("primary")).is_none();
    assert_that!(always["danger"].clone()).is_equal_to(json!(true));
    assert_that!(always["label"].as_str().unwrap_or_default()).ends_with("· pour toujours");

    // Un clic délibéré sur « toujours autoriser » renvoie son index dans l'ordre de l'agent.
    let id = surface["id"].as_str().unwrap_or_else(|| panic!("surface sans identifiant")).to_owned();
    let mut context = Map::new();
    context.insert("index".into(), json!(0));
    h.hub.act(SurfaceAction { surface: id, name: "permission".into(), context }).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(asking.await.unwrap_or_else(|e| panic!("{e}"))).is_equal_to(Some(0));
}

/// t-997: a typed reply to a permission keeps it open ("say it or click"), so a typed or
/// agent-prefilled "yes, always" grants nothing; only a click or the host's voice route answers.
#[tokio::test(flavor = "multi_thread")]
async fn typed_answer_never_grants_a_permission_and_keeps_it_open() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let asking = tokio::spawn({
        let hub = h.hub.clone();
        async move { hub.permission("Écrire le fichier ?", &choices()).await }
    });
    let asked = next_surface(&mut rx).await;
    assert_that!(h.hub.answer(&asked.id, "oui, toujours")).is_equal_to(AnswerOutcome::KeptOpen);
    tokio::task::yield_now().await;
    assert_that!(asking.is_finished()).is_false();
    h.hub.cancel_asks();
    assert_that!(asking.await.unwrap_or_else(|e| panic!("{e}"))).is_none();
}
