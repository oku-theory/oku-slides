//! Reçus de fin de tour, outil `report`, `render` et cycle de vie des tours du hub.

mod common;

use common::*;
use oku_slides_session::agent::receipt::{NextStep, ReceiptSurfaces, Report, receipt};
use oku_slides_session::agent::{AgentScope, forms};
use serde_json::json;
use speculoos::prelude::*;

fn report_spec(title: &str, actions: &[&str]) -> Report {
    Report {
        title: title.into(),
        facts: vec!["x".into()],
        slide: None,
        swatch: None,
        why: None,
        actions: actions.iter().map(|a| NextStep { label: (*a).into(), prompt: None }).collect(),
    }
}

/// Le reçu et sa version sans actions, comme `Session::report_surface` sans slide.
fn receipt_of(h: &Harness, id: &str, spec: &Report) -> ReceiptSurfaces {
    let plain = (!spec.actions.is_empty())
        .then(|| receipt(id, &Report { actions: Vec::new(), ..spec.clone() }, None, &h.localizer));
    ReceiptSurfaces { surface: receipt(id, spec, None, &h.localizer), plain }
}

#[tokio::test(flavor = "multi_thread")]
async fn report_tool_with_five_facts_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn(AgentScope::Slides);
    let r = call(&c, "report", json!({ "title": "x", "facts": ["a", "b", "c", "d", "e"] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).contains("facts");
}

#[tokio::test(flavor = "multi_thread")]
async fn report_tool_with_a_blank_title_or_no_facts_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn(AgentScope::Slides);
    let r = call(&c, "report", json!({ "title": "  ", "facts": ["a"] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).contains("title");
    let r = call(&c, "report", json!({ "title": "Fait", "facts": [] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).contains("facts");
}

#[tokio::test(flavor = "multi_thread")]
async fn report_tool_with_five_actions_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn(AgentScope::Slides);
    let actions: Vec<_> = (0..5).map(|i| json!({ "label": format!("Suite {i}") })).collect();
    let r = call(&c, "report", json!({ "title": "x", "facts": ["a"], "actions": actions })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn report_tool_on_an_unknown_slide_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn(AgentScope::Slides);
    let r = call(&c, "report", json!({ "title": "x", "facts": ["a"], "slide": "s-nulle-part" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn report_tool_with_a_slide_refers_to_it_without_its_svg() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn(AgentScope::Slides);
    let r = call(&c, "report", json!({ "title": "Fait", "facts": ["a"], "slide": "s-libre" })).await;
    let id = json_of(&r)["shown"].as_str().unwrap().to_owned();
    let json = serde_json::to_string(h.hub.surfaces().iter().find(|s| s.id == id).unwrap()).unwrap();
    assert_that!(json).contains("\"slide\":{\"id\":\"s-libre\",\"hash\":\"");
    assert_that!(json).does_not_contain("<svg");
}

#[tokio::test(flavor = "multi_thread")]
async fn report_tool_outside_a_turn_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "report", json!({ "title": "x", "facts": ["a"] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to("Aucun tour en cours : report ne s'affiche que pendant un tour.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_second_report_replaces_the_first_of_the_turn() {
    let h = start().await;
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.show_report(n, receipt_of(&h, "r-1", &report_spec("un", &[])));
    h.hub.show_report(n, receipt_of(&h, "r-2", &report_spec("deux", &[])));
    assert_that!(open_ids(&h)).is_equal_to(vec!["r-2".to_owned()]);
    assert_that!(h.hub.has_report(n)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_end_turn_without_report_shows_a_fallback_titled_by_the_narration() {
    let h = start().await;
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.end_turn(n, "Fond posé. Titre en blanc.");
    assert_that!(h.hub.has_report(n)).is_true();
    let json = serde_json::to_string(&h.hub.surfaces()).unwrap();
    assert_that!(json).contains("\"title\":\"Fond posé.\"");
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_end_turn_after_a_report_adds_nothing() {
    let h = start().await;
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.show_report(n, receipt_of(&h, "r-1", &report_spec("Fait", &[])));
    h.hub.end_turn(n, "Fond posé.");
    assert_that!(open_ids(&h)).is_equal_to(vec!["r-1".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_close_turn_forgets_that_the_turn_had_its_receipt() {
    let h = start().await;
    let one = h.hub.begin_turn(AgentScope::Slides);
    h.hub.show_report(one, receipt_of(&h, "r-1", &report_spec("un", &[])));
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.show_report(n, receipt_of(&h, "r-2", &report_spec("deux", &[])));
    h.hub.close_turn(one);
    assert_that!(h.hub.has_report(one)).is_false();
    assert_that!(h.hub.has_report(n)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_clear_thread_forgets_every_receipt() {
    let h = start().await;
    let one = h.hub.begin_turn(AgentScope::Slides);
    h.hub.show_report(one, receipt_of(&h, "r-1", &report_spec("un", &[])));
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.end_turn(n, "Fond posé.");
    assert_that!(h.hub.has_report(n)).is_true();
    h.hub.clear_thread(&AgentScope::Slides);
    assert_that!(h.hub.has_report(one)).is_false();
    assert_that!(h.hub.has_report(n)).is_false();
    assert_that!(open_ids(&h)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_report_action_of_the_last_turn_keeps_the_receipt() {
    let h = start().await;
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.show_report(n, receipt_of(&h, "r-1", &report_spec("Fait", &["Encore"])));
    h.hub.act(action("r-1", "suggest", json!({ "prompt": "Encore" }))).unwrap();
    assert_that!(open_ids(&h)).is_equal_to(vec!["r-1".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_stale_end_turn_does_not_close_the_next_turn() {
    // Ordre réel d'un tour interrompu : le tour 2 s'ouvre avant que le relais ne clôture le tour 1.
    let h = start().await;
    let one = h.hub.begin_turn(AgentScope::Slides);
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.end_turn(one, "");
    assert_that!(h.hub.show_static(n, forms::frozen_choice("x", "Q", Some("A"), &h.localizer))).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_show_static_after_end_turn_is_refused() {
    // Un outil tardif d'un tour terminé n'est rangé dans aucun tour.
    let h = start().await;
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.end_turn(n, "");
    assert_that!(h.hub.show_static(n, forms::frozen_choice("late", "Q", Some("A"), &h.localizer))).is_false();
    assert_that!(open_ids(&h)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn report_tool_actions_expire_through_its_plain_version() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn(AgentScope::Slides);
    let r = call(&c, "report", json!({ "title": "Fait", "facts": ["a"], "actions": [{ "label": "Encore" }] })).await;
    let id = json_of(&r)["shown"].as_str().unwrap().to_owned();
    h.hub.begin_turn(AgentScope::Slides);
    assert_that!(open_ids(&h)).contains(id);
    assert_that!(serde_json::to_string(&h.hub.surfaces()).unwrap()).does_not_contain("Encore");
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_report_actions_expire_when_the_next_turn_begins() {
    let h = start().await;
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.show_report(n, receipt_of(&h, "r-1", &report_spec("Fait", &["Encore"])));
    h.hub.begin_turn(AgentScope::Slides);
    // Toujours affiché (pas fermé) :
    assert_that!(open_ids(&h)).contains("r-1".to_owned());
    // La version sans actions remplace le reçu, au même identifiant…
    let json = serde_json::to_string(&h.hub.surfaces()).unwrap();
    assert_that!(json).does_not_contain("Encore");
    // …et une action envoyée par une interface en retard est refusée, dans la langue de l'utilisateur.
    let refused = h.hub.act(action("r-1", "suggest", json!({ "prompt": "Encore" }))).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(oku_slides_mcp::HubErrorKind::Expired);
    assert_that!(oku_slides_i18n::Localize::localize(&refused, &h.localizer))
        .is_equal_to("Les actions d'un tour passé ont expiré.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn render_tool_actions_expire_when_the_next_turn_begins() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn(AgentScope::Slides);
    let id = json_of(&call(&c, "render", render_args()).await)["shown"].as_str().unwrap().to_owned();
    assert_that!(h.hub.act(action(&id, "suggest", json!({ "prompt": "Encore" })))).is_ok();
    h.hub.begin_turn(AgentScope::Slides);
    assert_that!(h.hub.act(action(&id, "suggest", json!({ "prompt": "Encore" })))).is_err();
    // Toujours affichée, sans ses actions :
    assert_that!(open_ids(&h)).contains(id.clone());
    assert_that!(serde_json::to_string(&h.hub.surfaces()).unwrap()).does_not_contain("\"action\"");
}

#[tokio::test(flavor = "multi_thread")]
async fn render_tool_outside_a_turn_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "render", render_args()).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to("Aucun tour en cours : render ne s'affiche que pendant un tour.".to_owned());
}
