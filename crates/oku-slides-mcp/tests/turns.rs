//! Chaque appel d'outil reste dans le tour pendant lequel il est arrivé (t-449) :
//! un appel tardif d'un tour clos n'atterrit jamais dans le tour suivant.

mod common;

use common::*;
use oku_slides_mcp::{Answer, Freeze};
use oku_slides_session::agent::receipt::{ReceiptSurfaces, fallback};
use oku_slides_session::agent::{AskChoice, Render, Surface, forms, render};
use rust_mcp_sdk::schema::CallToolResult;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// Le JSON des surfaces ouvertes rangées dans le tour `turn`.
fn turn_json(h: &Harness, turn: u32) -> String {
    let surfaces: Vec<_> = h.hub.surfaces().into_iter().filter(|s| s.turn == Some(turn)).collect();
    serde_json::to_string(&surfaces).unwrap_or_else(|e| panic!("surfaces en JSON : {e}"))
}

fn all_json(h: &Harness) -> String {
    serde_json::to_string(&h.hub.surfaces()).unwrap_or_else(|e| panic!("surfaces en JSON : {e}"))
}

/// Un appel `tool` arrivé pendant le tour 1, retenu sur la session pendant que le tour 1
/// s'annule et que le tour 2 s'ouvre ; `between` joue ce qui se passe avant qu'il ne reprenne.
async fn call_across_turns(
    tool: &'static str,
    args: Value,
    between: impl FnOnce(&Harness, u32),
) -> (Harness, u32, CallToolResult) {
    let (h, tabs) = start_gated().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client : {e}"));
    let one = h.hub.begin_turn();
    let mut gate = tabs.hold_next();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, tool, args).await }
    });
    gate.entered().await;
    // Tour 1 annulé, sans narration : pas de reçu de repli pour lui.
    h.hub.end_turn(one, "");
    let two = h.hub.begin_turn();
    between(&h, two);
    gate.release();
    let r = pending.await.unwrap_or_else(|e| panic!("appel de {tool} : {e}"));
    (h, two, r)
}

fn late_report() -> Value {
    json!({ "title": "En retard", "facts": ["a"] })
}

#[tokio::test(flavor = "multi_thread")]
async fn report_late_from_a_cancelled_turn_leaves_the_next_turn_its_fallback() {
    let (h, two, r) = call_across_turns("report", late_report(), |_, _| {}).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to("Aucun tour en cours : report ne s'affiche que pendant un tour.".to_owned());
    // Le tour 2 finit sans `report` : il a son reçu de repli, pas celui du tour 1.
    h.hub.end_turn(two, "Fond posé.");
    assert_that!(turn_json(&h, two)).contains("\"title\":\"Fond posé.\"");
    assert_that!(all_json(&h)).does_not_contain("En retard");
}

#[tokio::test(flavor = "multi_thread")]
async fn report_late_from_a_cancelled_turn_does_not_replace_the_next_turn_fallback() {
    // Le tour 2 est déjà fini, avec son reçu de repli, quand l'appel tardif reprend.
    let (h, two, r) = call_across_turns("report", late_report(), |h, two| h.hub.end_turn(two, "Fond posé.")).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(turn_json(&h, two)).contains("\"title\":\"Fond posé.\"");
    assert_that!(all_json(&h)).does_not_contain("En retard");
}

#[tokio::test(flavor = "multi_thread")]
async fn show_params_late_from_a_cancelled_turn_is_refused() {
    let args = json!({ "title": "En retard", "slide": "s-libre", "fields": [
        { "label": "Thème", "kind": "select", "value": "papier", "op_template": { "op": "set_var", "key": "theme", "value": "$value" },
          "options": [{ "id": "papier", "label": "Papier" }, { "id": "pro", "label": "Pro" }] }
    ] });
    let (h, _, r) = call_across_turns("show_params", args, |_, _| {}).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to("Le tour de cet appel est terminé : rien n'a été affiché.".to_owned());
    assert_that!(h.hub.surfaces()).is_empty();
}

fn render_surface(id: &str) -> Surface {
    let spec: Render =
        serde_json::from_value(json!({ "components": [{ "id": "root", "component": "Text", "text": "En retard" }] }))
            .unwrap_or_else(|e| panic!("Render de test : {e}"));
    render::render(id, &spec).unwrap_or_else(|e| panic!("render : {}", e.one_line()))
}

fn receipt(id: &str, narration: &str, h: &Harness) -> ReceiptSurfaces {
    let surface = fallback(id, narration, &h.localizer).unwrap_or_else(|| panic!("reçu de repli vide"));
    ReceiptSurfaces { surface, plain: None }
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_render_late_from_a_closed_turn_is_refused_and_leaves_the_next_turn_alone() {
    let h = start().await;
    let one = h.hub.begin_turn();
    h.hub.end_turn(one, "");
    let two = h.hub.begin_turn();
    assert_that!(h.hub.show_static(one, render_surface("render-1"))).is_false();
    h.hub.end_turn(two, "Fond posé.");
    assert_that!(turn_json(&h, two)).contains("\"title\":\"Fond posé.\"");
    assert_that!(all_json(&h)).does_not_contain("En retard");
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_render_of_the_open_turn_lands_in_it_even_after_a_stale_end() {
    // Le relais clôt le tour 1 après l'ouverture du tour 2 : le tour 2 reste ouvert.
    let h = start().await;
    let one = h.hub.begin_turn();
    let two = h.hub.begin_turn();
    h.hub.end_turn(one, "");
    assert_that!(h.hub.show_static(two, render_surface("render-1"))).is_true();
    assert_that!(h.hub.surfaces()[0].turn).is_equal_to(Some(two));
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_report_late_from_a_closed_turn_keeps_the_next_turn_report() {
    let h = start().await;
    let one = h.hub.begin_turn();
    let two = h.hub.begin_turn();
    assert_that!(h.hub.show_report(two, receipt("report-2", "Le mien.", &h))).is_true();
    assert_that!(h.hub.show_report(one, receipt("report-1", "En retard.", &h))).is_false();
    assert_that!(turn_json(&h, two)).contains("Le mien.");
    assert_that!(all_json(&h)).does_not_contain("En retard");
    assert_that!(h.hub.has_report(one)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_thread_question_late_from_a_closed_turn_is_cancelled_unseen() {
    let h = start().await;
    let one = h.hub.begin_turn();
    h.hub.begin_turn();
    let spec: AskChoice = serde_json::from_value(
        json!({ "question": "Laquelle ?", "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] }),
    )
    .unwrap();
    let l = h.localizer.clone();
    let freeze: Freeze = Box::new(move |_| forms::frozen_choice("ask-1", "Laquelle ?", None, &l));
    let answer = h.hub.ask_then(Some(one), forms::choice("ask-1", &spec, &h.localizer), freeze).await;
    assert_that!(answer).is_equal_to(Answer::Cancelled);
    assert_that!(h.hub.surfaces()).is_empty();
    assert_that!(h.hub.has_pending_ask()).is_false();
}
