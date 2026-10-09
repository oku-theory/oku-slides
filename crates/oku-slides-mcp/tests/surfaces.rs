mod common;

use common::*;
use oku_slides_i18n::{Localize, english};
use oku_slides_mcp::{HubErrorKind, McpEvent};
use oku_slides_session::agent::{AgentScope, AskChoice, forms};
use serde_json::{Value, json};
use speculoos::prelude::*;

fn choice_args() -> Value {
    json!({ "question": "Laquelle ?", "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] })
}

/// L'identifiant d'une surface fermée (`CloseSurface`), pour `wait_event`.
fn closed_id(e: McpEvent) -> Option<String> {
    match e {
        McpEvent::CloseSurface { id } => Some(id),
        _ => None,
    }
}

/// La liste complète des surfaces (`Surfaces`), pour `wait_event`.
fn surfaces_list(e: McpEvent) -> Option<Vec<oku_slides_session::agent::Surface>> {
    match e {
        McpEvent::Surfaces { list } => Some(list),
        _ => None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_on_closed_tab_returns_an_error_to_the_agent() {
    let h = start_closed().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "deck_outline", json!({})).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_choice_blocks_until_user_chooses_while_session_stays_free() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_choice", choice_args()).await }
    });
    let surface = next_surface(&mut rx).await;
    // Pendant l'attente, l'interface peut toujours utiliser la session.
    assert_that!(h.session.try_lock().is_ok()).is_true();
    assert_that!(pending.is_finished()).is_false();
    h.hub.act(action(&surface.id, "choose", json!({ "choice": "b" }))).unwrap();
    let r = pending.await.unwrap();
    assert_that!(json_of(&r)).is_equal_to(json!({ "choice": "b" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_choice_dismissed_returns_cancelled_and_freezes_the_question() {
    let h = start().await;
    h.hub.begin_turn(AgentScope::Slides);
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_choice", choice_args()).await }
    });
    let surface = next_surface(&mut rx).await;
    let surface_id = surface.id.clone();
    h.hub.act(action(&surface.id, "dismiss", json!({}))).unwrap();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "cancelled": true }));
    let frozen = wait_event(&mut rx, |e| match e {
        McpEvent::Surface { surface } if surface.id == surface_id && !surface.live => Some(surface),
        _ => None,
    })
    .await;
    assert_that!(serde_json::to_string(&frozen).unwrap()).contains("Question annulée");
}

#[tokio::test(flavor = "multi_thread")]
async fn answer_turns_the_prompt_into_answer_text() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_choice", choice_args()).await }
    });
    let asked = next_surface(&mut rx).await;
    assert_that!(h.hub.has_pending_ask()).is_true();
    assert_that!(h.hub.answer(&asked.id, "la deuxième")).is_true();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "answer_text": "la deuxième" }));
    assert_that!(h.hub.has_pending_ask()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_asks_releases_every_waiting_tool() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_confirm", json!({ "message": "Sûr ?" })).await }
    });
    next_surface(&mut rx).await;
    h.hub.cancel_asks();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "cancelled": true }));
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_choice_with_one_option_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "ask_choice", json!({ "question": "?", "options": [{ "id": "a", "label": "A" }] })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_choice_with_a_duplicate_option_id_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let options = json!([{ "id": "a", "label": "A" }, { "id": "a", "label": "B" }]);
    let r = call(&c, "ask_choice", json!({ "question": "?", "options": options })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_rewrite_applies_chosen_proposal() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "slide": "s-libre", "owner": "e-texte", "proposals": ["Premier", "Second"] });
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_rewrite", args).await }
    });
    let surface = next_surface(&mut rx).await;
    let paragraphs = json!([{ "runs": [{ "t": "Second" }] }]);
    h.hub
        .act(action(&surface.id, "rewrite", json!({ "kept": 1, "text": "Second", "paragraphs": paragraphs })))
        .unwrap();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "kept": 1, "text": "Second" }));
    let s = h.session.lock().unwrap();
    let view = s.slide("s-libre").unwrap();
    let text = view.fields.iter().find(|f| f.owner == "e-texte" && f.field == "body").map(|f| f.text.as_ref());
    assert_that!(text).is_equal_to(Some("Second"));
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_rewrite_paragraphs_answer_carries_a_bold_run() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "slide": "s-libre", "owner": "e-texte", "proposals": ["Premier", "Second"] });
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_rewrite", args).await }
    });
    let surface = next_surface(&mut rx).await;
    let paragraphs = json!([{ "runs": [{ "t": "Gras", "bold": true }, { "t": " fin" }] }]);
    h.hub
        .act(action(&surface.id, "rewrite", json!({ "kept": 1, "text": "**Gras** fin", "paragraphs": paragraphs })))
        .unwrap();
    pending.await.unwrap();
    let s = h.session.lock().unwrap();
    let located = s.deck().slide("s-libre").unwrap().locate("e-texte").unwrap();
    let run = &located.element.params.rich("body").unwrap()[0].runs[0];
    assert_that!(run.bold).is_true();
    assert_that!(run.t.as_str()).is_equal_to("Gras");
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_rewrite_without_paragraphs_for_an_element_fails() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "slide": "s-libre", "owner": "e-texte", "proposals": ["Premier", "Second"] });
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_rewrite", args).await }
    });
    let surface = next_surface(&mut rx).await;
    h.hub.act(action(&surface.id, "rewrite", json!({ "kept": 1, "text": "Second" }))).unwrap();
    let r = pending.await.unwrap();
    assert_that!(r.is_error).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_rewrite_with_invalid_paragraphs_fails_in_the_user_language() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "slide": "s-libre", "owner": "e-texte", "proposals": ["Premier", "Second"] });
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_rewrite", args).await }
    });
    let surface = next_surface(&mut rx).await;
    h.hub.act(action(&surface.id, "rewrite", json!({ "kept": 1, "text": "Second", "paragraphs": 3 }))).unwrap();
    let r = pending.await.unwrap();
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to("Les paragraphes de la reformulation sont invalides.".to_owned());
}

/// t-326 : un champ de la slide s'adresse par `{owner: <slide>, field}` ; le texte gardé y va.
#[tokio::test(flavor = "multi_thread")]
async fn ask_rewrite_of_a_slide_field_applies_the_kept_text() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "slide": "s-constat", "owner": "s-constat", "field": "title", "proposals": ["Un", "Deux"] });
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_rewrite", args).await }
    });
    let surface = next_surface(&mut rx).await;
    let anchor = serde_json::to_value(&surface.anchor).unwrap();
    assert_that!(anchor).is_equal_to(json!({ "slide": "s-constat", "owner": "s-constat", "field": "title" }));
    let paragraphs = json!([{ "runs": [{ "t": "Deux" }] }]);
    h.hub.act(action(&surface.id, "rewrite", json!({ "kept": 1, "text": "Deux", "paragraphs": paragraphs }))).unwrap();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "kept": 1, "text": "Deux" }));
    let s = h.session.lock().unwrap();
    assert_that!(s.deck().slide("s-constat").unwrap().params.text("title")).is_equal_to(Some("Deux"));
}

/// t-326 : une adresse hors du deck actif est refusée avant toute surface, dans la langue de
/// l'utilisateur.
#[tokio::test(flavor = "multi_thread")]
async fn ask_rewrite_outside_the_deck_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let cases = [
        (
            json!({ "slide": "s-constat", "owner": "e-texte" }),
            Some("e-texte n'est ni la slide s-constat ni un de ses éléments."),
        ),
        (
            json!({ "slide": "s-constat", "owner": "s-constat", "field": "nope" }),
            Some("s-constat n'a pas de champ texte nommé nope."),
        ),
        (json!({ "slide": "s-constat", "owner": "s-constat", "field": "title\")#panic(\"" }), None),
        (json!({ "slide": "s-constat", "owner": "s-constat", "field": "blocks/b-points/body" }), None),
        (json!({ "slide": "s-constat", "element": "@title" }), None),
    ];
    for (address, said) in cases {
        let mut args = address.clone();
        args["proposals"] = json!(["A", "B"]);
        let r = call(&c, "ask_rewrite", args).await;
        assert_that!(r.is_error).named(&address.to_string()).is_equal_to(Some(true));
        if let Some(said) = said {
            assert_that!(text_of(&r)).is_equal_to(said.to_owned());
        }
    }
    assert_that!(h.hub.has_pending_ask()).is_false();
    let s = h.session.lock().unwrap();
    assert_that!(s.snapshot().undo_label).is_none();
}

/// t-326 : l'ancre d'une question est résolue de même ; une ancre étrangère ne montre rien.
#[tokio::test(flavor = "multi_thread")]
async fn ask_choice_with_an_anchor_outside_the_deck_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let anchors = [
        json!({ "slide": "s-constat", "owner": "c1" }),
        json!({ "slide": "s-libre", "owner": "e-texte", "field": "title" }),
        json!({ "slide": "s-constat", "owner": "s-constat" }),
        json!({ "slide": "s-ailleurs", "owner": "s-ailleurs", "field": "title" }),
        json!({ "slide": "s-constat", "element": "@title" }),
    ];
    for anchor in anchors {
        let mut args = choice_args();
        args["anchor"] = anchor.clone();
        let r = call(&c, "ask_choice", args).await;
        assert_that!(r.is_error).named(&anchor.to_string()).is_equal_to(Some(true));
    }
    assert_that!(h.hub.has_pending_ask()).is_false();
}

/// t-326 : une ancre sur un champ d'élément est acceptée et part telle quelle à l'interface.
#[tokio::test(flavor = "multi_thread")]
async fn ask_choice_anchored_on_an_element_field_goes_to_a_sheet() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let mut args = choice_args();
    args["anchor"] = json!({ "slide": "s-priorites", "owner": "c1", "field": "title" });
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_choice", args).await }
    });
    let surface = next_surface(&mut rx).await;
    assert_that!(surface.place).is_equal_to(forms::Place::Sheet);
    assert_that!(serde_json::to_value(&surface.anchor).unwrap())
        .is_equal_to(json!({ "slide": "s-priorites", "owner": "c1", "field": "title" }));
    h.hub.act(action(&surface.id, "choose", json!({ "choice": "a" }))).unwrap();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "choice": "a" }));
}

/// t-326 : une conversation de widget ne voit pas la scène des slides : une adresse y est hors
/// de portée, même valide dans le deck.
#[tokio::test(flavor = "multi_thread")]
async fn anchored_questions_of_a_widget_turn_are_out_of_scope() {
    let h = start().await;
    h.hub.begin_turn(AgentScope::Widget { pack: uuid::Uuid::from_u128(1), name: "kpi".into() });
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let mut choice = choice_args();
    choice["anchor"] = json!({ "slide": "s-libre", "owner": "e-texte" });
    let rewrite = json!({ "slide": "s-libre", "owner": "e-texte", "proposals": ["A", "B"] });
    let said = "Une question d'une conversation de widget ne s'ancre pas sur une slide : posez-la sans ancre.";
    for (tool, args) in [("ask_choice", choice), ("ask_rewrite", rewrite)] {
        let r = call(&c, tool, args).await;
        assert_that!(r.is_error).named(tool).is_equal_to(Some(true));
        assert_that!(text_of(&r)).named(tool).is_equal_to(said.to_owned());
    }
    assert_that!(h.hub.has_pending_ask()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn show_variants_returns_at_once_and_pick_applies_with_own_undo_and_note() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let variant = |t: &str| json!({ "label": t, "ops": [{ "op": "set_param", "target": { "slide": "s-libre", "element": "e-texte" }, "param": "body", "value": [{ "runs": [{ "t": t }] }] }] });
    let r =
        call(&c, "show_variants", json!({ "slide": "s-libre", "variants": [variant("Un"), variant("Deux")] })).await;
    let id = json_of(&r)["shown"].as_str().unwrap().to_owned();
    let surface = next_surface(&mut rx).await;
    assert_that!(surface.id.as_str()).is_equal_to(id.as_str());
    h.hub.act(action(&id, "variant", json!({ "index": 1 }))).unwrap();
    let mut s = h.session.lock().unwrap();
    assert_that!(s.snapshot().undo_label).is_equal_to(Some("Variante : Deux".to_owned()));
    let ctx = s.agent_context(&Default::default());
    assert_that!(ctx.contains("variant picked: \"Deux\" (s-libre)")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn show_params_change_applies_template_op() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "title": "Apparence", "slide": "s-libre", "fields": [
        { "label": "Thème", "kind": "select", "value": "papier", "op_template": { "op": "set_var", "key": "theme", "value": "$value" },
          "options": [{ "id": "papier", "label": "Papier" }, { "id": "pro", "label": "Pro" }] }
    ] });
    let id = json_of(&call(&c, "show_params", args).await)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    h.hub.act(action(&id, "param", json!({ "field": 0, "value": "pro" }))).unwrap();
    assert_that!(h.session.lock().unwrap().deck().vars.get("theme").cloned()).is_equal_to(Some(json!("pro")));
}

#[tokio::test(flavor = "multi_thread")]
async fn act_keeps_surface_on_apply_error_so_it_can_still_be_dismissed() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let variant = |t: &str| json!({ "label": t, "ops": [{ "op": "set_param", "target": { "slide": "s-libre", "element": "e-texte" }, "param": "body", "value": [{ "runs": [{ "t": t }] }] }] });
    let r =
        call(&c, "show_variants", json!({ "slide": "s-libre", "variants": [variant("Un"), variant("Deux")] })).await;
    let id = json_of(&r)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    // La slide visée par les variantes disparaît sans passer par `prune` :
    // choisir une variante doit maintenant échouer à l'application.
    call(&c, "apply", json!({ "ops": [{ "op": "remove_slide", "slide": "s-libre" }] })).await;
    let refused = h.hub.act(action(&id, "variant", json!({ "index": 0 }))).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(HubErrorKind::Session);
    // L'erreur de la session, dite en français.
    assert_that!(refused.localize(&h.localizer))
        .is_equal_to("Opération 0 refusée : Slide introuvable : s-libre".to_owned());
    // La surface n'est pas perdue : elle peut encore être fermée.
    assert_that!(h.hub.act(action(&id, "dismiss", json!({})))).is_ok();
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_dropped_before_answer_still_closes_the_surface() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let spec: AskChoice = serde_json::from_value(choice_args()).unwrap_or_else(|e| panic!("AskChoice de test : {e}"));
    let surface = forms::choice(&h.hub.next_id("ask"), &spec, &h.localizer);
    let id = surface.id.clone();
    let waiting = tokio::spawn({
        let hub = h.hub.clone();
        async move { hub.ask(surface).await }
    });
    next_surface(&mut rx).await;
    assert_that!(h.hub.has_pending_ask()).is_true();
    waiting.abort();
    let closed = wait_event(&mut rx, closed_id).await;
    assert_that!(closed).is_equal_to(id);
    assert_that!(h.hub.has_pending_ask()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_rewrite_answer_without_text_fails() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "slide": "s-libre", "owner": "e-texte", "proposals": ["Premier", "Second"] });
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_rewrite", args).await }
    });
    let surface = next_surface(&mut rx).await;
    h.hub.act(action(&surface.id, "rewrite", json!({ "kept": 1 }))).unwrap();
    let r = pending.await.unwrap();
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r)).is_equal_to("La reformulation est revenue sans texte.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_closes_params_of_removed_slide() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "title": "x", "slide": "s-libre", "fields": [
        { "label": "Thème", "kind": "text", "value": "papier", "op_template": { "op": "set_var", "key": "theme", "value": "$value" } }
    ] });
    let id = json_of(&call(&c, "show_params", args).await)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    call(&c, "apply", json!({ "ops": [{ "op": "remove_slide", "slide": "s-libre" }] })).await;
    h.hub.prune();
    let closed = wait_event(&mut rx, closed_id).await;
    assert_that!(closed).is_equal_to(id);
    let refused = h.hub.act(action(&closed, "dismiss", json!({}))).unwrap_err();
    assert_that!(refused.kind()).is_equal_to(HubErrorKind::UnknownSurface);
    assert_that!(refused.localize(&h.localizer)).is_equal_to("Cette carte n'est plus ouverte.".to_owned());
    assert_that!(refused.localize(english())).is_equal_to("This card is no longer open.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn act_refuses_an_unexpected_or_incomplete_action_in_the_user_language() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let variant = |t: &str| json!({ "label": t, "ops": [{ "op": "set_param", "target": { "slide": "s-libre", "element": "e-texte" }, "param": "body", "value": [{ "runs": [{ "t": t }] }] }] });
    let r =
        call(&c, "show_variants", json!({ "slide": "s-libre", "variants": [variant("Un"), variant("Deux")] })).await;
    let variants = json_of(&r)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    let params = json_of(&call(&c, "show_params", params_args()).await)["shown"].as_str().unwrap().to_owned();
    let refused = |id: &str, name: &str, context: Value| h.hub.act(action(id, name, context)).unwrap_err();

    let unexpected = refused(&variants, "param", json!({ "field": 0 }));
    assert_that!(unexpected.kind()).is_equal_to(HubErrorKind::UnexpectedAction);
    assert_that!(unexpected.localize(&h.localizer)).is_equal_to("Action inattendue : param.".to_owned());
    assert_that!(unexpected.localize(english())).is_equal_to("Unexpected action: param.".to_owned());

    let incomplete = refused(&variants, "variant", json!({}));
    assert_that!(incomplete.kind()).is_equal_to(HubErrorKind::Incomplete);
    assert_that!(incomplete.localize(&h.localizer)).is_equal_to("L'action de cette carte est incomplète.".to_owned());

    let no_variant = refused(&variants, "variant", json!({ "index": 7 }));
    assert_that!(no_variant.kind()).is_equal_to(HubErrorKind::NoVariant);
    assert_that!(no_variant.localize(&h.localizer)).is_equal_to("Cette variante n'existe plus.".to_owned());

    let no_field = refused(&params, "param", json!({ "field": 7, "value": "x" }));
    assert_that!(no_field.kind()).is_equal_to(HubErrorKind::NoField);
    assert_that!(no_field.localize(&h.localizer)).is_equal_to("Ce réglage n'existe plus.".to_owned());
}
#[tokio::test(flavor = "multi_thread")]
async fn surfaces_follow_open_and_close_in_order() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let params = json!({ "title": "x", "slide": "s-libre", "fields": [
        { "label": "Thème", "kind": "text", "value": "papier", "op_template": { "op": "set_var", "key": "theme", "value": "$value" } }
    ] });
    let shown = json_of(&call(&c, "show_params", params).await)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_choice", choice_args()).await }
    });
    let asked = next_surface(&mut rx).await.id;
    assert_that!(open_ids(&h)).is_equal_to(vec![shown.clone(), asked.clone()]);

    h.hub.act(action(&asked, "choose", json!({ "choice": "a" }))).unwrap();
    pending.await.unwrap();
    assert_that!(open_ids(&h)).is_equal_to(vec![shown]);

    call(&c, "apply", json!({ "ops": [{ "op": "remove_slide", "slide": "s-libre" }] })).await;
    h.hub.prune();
    assert_that!(open_ids(&h)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn surfaces_after_a_dropped_ask_is_empty() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let spec: AskChoice = serde_json::from_value(choice_args()).unwrap_or_else(|e| panic!("AskChoice de test : {e}"));
    let surface = forms::choice(&h.hub.next_id("ask"), &spec, &h.localizer);
    let waiting = tokio::spawn({
        let hub = h.hub.clone();
        async move { hub.ask(surface).await }
    });
    next_surface(&mut rx).await;
    assert_that!(open_ids(&h)).has_length(1);
    waiting.abort();
    wait_event(&mut rx, closed_id).await;
    assert_that!(open_ids(&h)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn resync_emits_the_whole_list() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_choice", choice_args()).await }
    });
    let asked = next_surface(&mut rx).await;
    h.hub.resync(AgentScope::Slides);
    let list = wait_event(&mut rx, surfaces_list).await;
    assert_that!(list).is_equal_to(vec![asked.clone()]);
    h.hub.act(action(&asked.id, "dismiss", json!({}))).unwrap();
    pending.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn surfaces_event_always_carries_the_hub_list() {
    // Liste lue et envoyée sous le même verrou : aucun delta ne peut passer entre les deux.
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let params = json!({ "title": "x", "slide": "s-libre", "fields": [
        { "label": "Thème", "kind": "text", "value": "papier", "op_template": { "op": "set_var", "key": "theme", "value": "$value" } }
    ] });
    let shown = json_of(&call(&c, "show_params", params).await)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    h.hub.emit(McpEvent::Surfaces { list: vec![] });
    let list = wait_event(&mut rx, surfaces_list).await;
    assert_that!(list.into_iter().map(|s| s.id).collect::<Vec<_>>()).is_equal_to(vec![shown]);
}

#[test]
fn surfaces_event_serializes_kind_and_list() {
    let json = serde_json::to_value(McpEvent::Surfaces { list: vec![] }).unwrap();
    assert_that!(json).is_equal_to(json!({ "kind": "surfaces", "list": [] }));
}

#[tokio::test(flavor = "multi_thread")]
async fn answered_ask_leaves_the_list_at_once() {
    // Sinon une resynchronisation avant la fin de l'outil la ferait réapparaître.
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let spec: AskChoice = serde_json::from_value(choice_args()).unwrap_or_else(|e| panic!("AskChoice de test : {e}"));
    let surface = forms::choice(&h.hub.next_id("ask"), &spec, &h.localizer);
    let id = surface.id.clone();
    // La question n'est jamais attendue : seul `act` peut la retirer de la liste.
    let hub = h.hub.clone();
    let asking = hub.ask(surface);
    let mut asking = Box::pin(asking);
    assert_that!(futures_poll_once(asking.as_mut()).await).is_false();
    next_surface(&mut rx).await;
    h.hub.act(action(&id, "choose", json!({ "choice": "a" }))).unwrap();
    assert_that!(open_ids(&h)).is_empty();
}

/// Fait avancer un futur d'un pas ; `true` s'il est prêt.
async fn futures_poll_once<F: std::future::Future + Unpin>(f: F) -> bool {
    let mut f = f;
    std::future::poll_fn(|cx| std::task::Poll::Ready(std::pin::Pin::new(&mut f).poll(cx).is_ready())).await
}

#[tokio::test(flavor = "multi_thread")]
async fn param_change_is_kept_for_resync() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "title": "Apparence", "slide": "s-libre", "fields": [
        { "label": "Thème", "kind": "select", "value": "papier", "op_template": { "op": "set_var", "key": "theme", "value": "$value" },
          "options": [{ "id": "papier", "label": "Papier" }, { "id": "pro", "label": "Pro" }] }
    ] });
    let id = json_of(&call(&c, "show_params", args).await)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    h.hub.act(action(&id, "param", json!({ "field": 0, "value": "pro" }))).unwrap();
    let json = serde_json::to_value(h.hub.surfaces()).unwrap();
    let model = json[0]["messages"].as_array().unwrap().iter().find_map(|m| m.get("updateDataModel")).unwrap().clone();
    assert_that!(model["value"]["f0"].clone()).is_equal_to(json!("pro"));
}

#[tokio::test(flavor = "multi_thread")]
async fn close_releases_every_waiting_tool() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_confirm", json!({ "message": "Sûr ?" })).await }
    });
    next_surface(&mut rx).await;
    h.hub.close();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "cancelled": true }));
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_after_close_is_cancelled_without_opening_a_surface() {
    let h = start().await;
    h.hub.close();
    let confirmed = tokio::time::timeout(std::time::Duration::from_secs(2), h.hub.confirm("Sûr ?".into(), false)).await;
    assert_that!(confirmed).is_equal_to(Ok(false));
    let permission = tokio::time::timeout(std::time::Duration::from_secs(2), h.hub.permission("?", &[])).await;
    assert_that!(permission).is_equal_to(Ok(None));
    assert_that!(h.hub.surfaces()).is_empty();
    assert_that!(h.hub.has_pending_ask()).is_false();
}

/// t-249 : sans surface, `prune` ne prend pas la session (un rendu en cours ne la bloque pas).
#[tokio::test(flavor = "multi_thread")]
async fn prune_without_live_settings_does_not_wait_for_the_session() {
    let h = start().await;
    let (session, hub) = (h.session.clone(), h.hub.clone());
    // Session tenue pendant tout l'essai ; `prune` tourne à côté.
    let done = tokio::task::spawn_blocking(move || {
        let _held = session.lock().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            hub.prune();
            let _ = tx.send(());
        });
        rx.recv_timeout(std::time::Duration::from_secs(2)).is_ok()
    })
    .await
    .unwrap();
    assert_that!(done).is_true();
}

/// Les surfaces statiques (`render`, reçus) ne dépendent d'aucune slide : `prune` n'attend pas la session.
#[tokio::test(flavor = "multi_thread")]
async fn prune_with_only_static_surfaces_does_not_wait_for_the_session() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn(AgentScope::Slides);
    let id = json_of(&call(&c, "render", render_args()).await)["shown"].as_str().unwrap().to_owned();
    let (session, hub) = (h.session.clone(), h.hub.clone());
    let done = tokio::task::spawn_blocking(move || {
        let _held = session.lock().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            hub.prune();
            let _ = tx.send(());
        });
        rx.recv_timeout(std::time::Duration::from_secs(2)).is_ok()
    })
    .await
    .unwrap();
    assert_that!(done).is_true();
    assert_that!(open_ids(&h)).contains(id);
}

#[tokio::test(flavor = "multi_thread")]
async fn widgets_tool_lists_card_with_its_params() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let widgets = json_of(&call(&c, "widgets", json!({})).await);
    let card = &widgets["widgets"]["card"];
    assert_that!(card["category"].as_str()).is_equal_to(Some("element"));
    let params: Vec<&str> = card["params"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_that!(params).is_equal_to(vec!["kicker", "title", "body", "highlight"]);
    assert_that!(card["params"][2]["type"].as_str()).is_equal_to(Some("rich"));
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_surface_patch_updates_the_open_copy_sent_on_resync() {
    let h = start().await;
    let s = forms::confirm(
        "ask-1",
        &oku_slides_session::agent::AskConfirm { message: "Ok ?".into(), danger: false },
        &h.localizer,
    );
    h.hub.emit(McpEvent::Surface { surface: s });
    h.hub.emit(McpEvent::SurfacePatch {
        id: "ask-1".into(),
        messages: vec![oku_slides_a2ui::ServerMessage::set_data_model("ask-1", json!({"x": 1}))],
    });
    let json = serde_json::to_value(&h.hub.surfaces()[0]).unwrap();
    let model = json["messages"].as_array().unwrap().iter().find_map(|m| m.get("updateDataModel")).unwrap();
    assert_that!(model["value"].clone()).is_equal_to(json!({"x": 1}));
}

fn thread_choice(h: &Harness, id: &str) -> oku_slides_session::agent::Surface {
    let spec: AskChoice = serde_json::from_value(json!({
        "question": "Laquelle ?", "options": [{ "id": "a", "label": "Bleu" }, { "id": "b", "label": "Vert" }]
    }))
    .unwrap_or_else(|e| panic!("AskChoice de test : {e}"));
    forms::choice(id, &spec, &h.localizer)
}

/// Le `Freeze` de `ask_choice` sans ancre, comme dans `surface_tools.rs` (étiquette de l'option choisie).
fn freeze_choice(h: &Harness, id: &str) -> oku_slides_mcp::Freeze {
    let (id, l) = (id.to_owned(), h.localizer.clone());
    Box::new(move |a| {
        let label = match a {
            oku_slides_mcp::Answer::Action { context, .. } => match context.get("choice").and_then(Value::as_str) {
                Some("a") => Some("Bleu"),
                Some("b") => Some("Vert"),
                _ => None,
            },
            oku_slides_mcp::Answer::Cancelled => None,
        };
        forms::frozen_choice(&id, "Laquelle ?", label, &l)
    })
}

fn params_args() -> Value {
    json!({ "title": "Apparence", "slide": "s-libre", "fields": [
        { "label": "Thème", "kind": "select", "value": "papier", "op_template": { "op": "set_var", "key": "theme", "value": "$value" },
          "options": [{ "id": "papier", "label": "Papier" }, { "id": "pro", "label": "Pro" }] }
    ] })
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_show_static_during_a_turn_carries_it() {
    let h = start().await;
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.show_static(n, forms::frozen_choice("x-1", "Q", Some("A"), &h.localizer));
    assert_that!(h.hub.surfaces()[0].turn).is_equal_to(Some(n));
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_answered_thread_question_is_frozen_with_the_chosen_label() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let n = h.hub.begin_turn(AgentScope::Slides);
    let asking = tokio::spawn({
        let (hub, turn, surface, freeze) =
            (h.hub.clone(), h.hub.turn(), thread_choice(&h, "ask-1"), freeze_choice(&h, "ask-1"));
        async move { hub.ask_then(turn, surface, freeze).await }
    });
    next_surface(&mut rx).await;
    h.hub.act(action("ask-1", "choose", json!({ "choice": "b" }))).unwrap();
    asking.await.unwrap();
    let open = h.hub.surfaces();
    assert_that!(open).has_length(1);
    assert_that!(open[0].live).is_false();
    assert_that!(open[0].turn).is_equal_to(Some(n));
    let json = serde_json::to_string(&open[0]).unwrap();
    assert_that!(json).contains("✓ Vert");
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_cancelled_thread_question_is_frozen_as_cancelled() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    h.hub.begin_turn(AgentScope::Slides);
    let asking = tokio::spawn({
        let (hub, turn, surface, freeze) =
            (h.hub.clone(), h.hub.turn(), thread_choice(&h, "ask-1"), freeze_choice(&h, "ask-1"));
        async move { hub.ask_then(turn, surface, freeze).await }
    });
    next_surface(&mut rx).await;
    h.hub.cancel_asks();
    asking.await.unwrap();
    let json = serde_json::to_string(&h.hub.surfaces()).unwrap();
    assert_that!(json).contains("Question annulée");
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_question_cancelled_after_clear_thread_leaves_no_ghost() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    h.hub.begin_turn(AgentScope::Slides);
    let asking = tokio::spawn({
        let (hub, turn, surface, freeze) =
            (h.hub.clone(), h.hub.turn(), thread_choice(&h, "ask-1"), freeze_choice(&h, "ask-1"));
        async move { hub.ask_then(turn, surface, freeze).await }
    });
    next_surface(&mut rx).await;
    // Nouvelle conversation : `clear_thread` relâche lui-même la question, après avoir effacé les tours.
    h.hub.clear_thread(&AgentScope::Slides);
    asking.await.unwrap();
    assert_that!(h.hub.surfaces()).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_question_cancelled_after_close_leaves_no_ghost() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    h.hub.begin_turn(AgentScope::Slides);
    let asking = tokio::spawn({
        let (hub, turn, surface, freeze) =
            (h.hub.clone(), h.hub.turn(), thread_choice(&h, "ask-1"), freeze_choice(&h, "ask-1"));
        async move { hub.ask_then(turn, surface, freeze).await }
    });
    next_surface(&mut rx).await;
    h.hub.close();
    asking.await.unwrap();
    assert_that!(h.hub.surfaces()).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_question_of_an_evicted_turn_leaves_no_ghost() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let n = h.hub.begin_turn(AgentScope::Slides);
    let asking = tokio::spawn({
        let (hub, turn, surface, freeze) =
            (h.hub.clone(), h.hub.turn(), thread_choice(&h, "ask-1"), freeze_choice(&h, "ask-1"));
        async move { hub.ask_then(turn, surface, freeze).await }
    });
    next_surface(&mut rx).await;
    h.hub.close_turn(n);
    h.hub.cancel_asks();
    asking.await.unwrap();
    assert_that!(h.hub.surfaces()).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_new_params_freeze_the_previous_ones() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn(AgentScope::Slides);
    let first = json_of(&call(&c, "show_params", params_args()).await)["shown"].as_str().unwrap().to_owned();
    let second = json_of(&call(&c, "show_params", params_args()).await)["shown"].as_str().unwrap().to_owned();
    let open = h.hub.surfaces();
    assert_that!(open.iter().find(|s| s.id == first).unwrap().live).is_false();
    assert_that!(open.iter().find(|s| s.id == second).unwrap().live).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_dismissed_params_are_frozen_not_closed() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn(AgentScope::Slides);
    let id = json_of(&call(&c, "show_params", params_args()).await)["shown"].as_str().unwrap().to_owned();
    h.hub.act(action(&id, "dismiss", json!({}))).unwrap();
    let open = h.hub.surfaces();
    assert_that!(open).has_length(1);
    assert_that!(open[0].live).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_static_surfaces_are_capped_per_turn() {
    let h = start().await;
    let n = h.hub.begin_turn(AgentScope::Slides);
    for i in 0..(oku_slides_mcp::STATIC_PER_TURN + 2) {
        h.hub.show_static(n, forms::frozen_choice(&format!("x-{i}"), "Q", Some("A"), &h.localizer));
    }
    let expected: Vec<String> = (2..oku_slides_mcp::STATIC_PER_TURN + 2).map(|i| format!("x-{i}")).collect();
    assert_that!(open_ids(&h)).is_equal_to(expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_dismissed_params_lose_their_actions() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    h.hub.begin_turn(AgentScope::Slides);
    let id = json_of(&call(&c, "show_params", params_args()).await)["shown"].as_str().unwrap().to_owned();
    h.hub.act(action(&id, "dismiss", json!({}))).unwrap();
    assert_that!(serde_json::to_string(&h.hub.surfaces()).unwrap()).does_not_contain("\"action\"");
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_show_static_without_an_open_turn_is_refused() {
    let h = start().await;
    let shown = h.hub.show_static(1, forms::frozen_choice("x", "Q", Some("A"), &h.localizer));
    assert_that!(shown).is_false();
    assert_that!(open_ids(&h)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_answered_question_keeps_its_place_in_the_list() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let n = h.hub.begin_turn(AgentScope::Slides);
    let asking = tokio::spawn({
        let (hub, turn, surface, freeze) =
            (h.hub.clone(), h.hub.turn(), thread_choice(&h, "ask-1"), freeze_choice(&h, "ask-1"));
        async move { hub.ask_then(turn, surface, freeze).await }
    });
    next_surface(&mut rx).await;
    h.hub.show_static(n, forms::frozen_choice("after", "Q", Some("A"), &h.localizer));
    h.hub.act(action("ask-1", "choose", json!({ "choice": "a" }))).unwrap();
    asking.await.unwrap();
    assert_that!(open_ids(&h)).is_equal_to(vec!["ask-1".to_owned(), "after".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_surface_of_a_cleared_turn_is_ignored() {
    let h = start().await;
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.clear_thread(&AgentScope::Slides);
    // Le relais, en retard, émet encore la surface du tour effacé.
    let mut late = forms::frozen_choice("turn-late", "Q", Some("A"), &h.localizer);
    late.turn = Some(n);
    h.hub.emit(McpEvent::Surface { surface: late });
    assert_that!(open_ids(&h)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_close_turn_closes_only_that_turn() {
    let h = start().await;
    let one = h.hub.begin_turn(AgentScope::Slides);
    h.hub.show_static(one, forms::frozen_choice("a", "Q", Some("A"), &h.localizer));
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.show_static(n, forms::frozen_choice("b", "Q", Some("A"), &h.localizer));
    h.hub.close_turn(one);
    assert_that!(open_ids(&h)).is_equal_to(vec!["b".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_clear_thread_keeps_surfaces_outside_turns() {
    let h = start().await;
    let confirm = forms::confirm(
        "c",
        &oku_slides_session::agent::AskConfirm { message: "Ok ?".into(), danger: false },
        &h.localizer,
    );
    h.hub.emit(McpEvent::Surface { surface: confirm });
    let n = h.hub.begin_turn(AgentScope::Slides);
    h.hub.show_static(n, forms::frozen_choice("a", "Q", Some("A"), &h.localizer));
    h.hub.clear_thread(&AgentScope::Slides);
    assert_that!(open_ids(&h)).is_equal_to(vec!["c".to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_parallel_ask_choices_are_all_live_and_answered_independently() {
    let h = start().await;
    h.hub.begin_turn(AgentScope::Slides);
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let ask = || {
        let c = c.clone();
        tokio::spawn(async move { call(&c, "ask_choice", choice_args()).await })
    };
    let first = ask();
    let first_id = next_surface(&mut rx).await.id;
    let second = ask();
    let second_id = next_surface(&mut rx).await.id;
    // Les deux questions sont au dock, dans l'ordre d'arrivée.
    let live: Vec<(String, bool)> =
        h.hub.surfaces().into_iter().filter(|s| s.live).map(|s| (s.id, s.question)).collect();
    assert_that!(live).is_equal_to(vec![(first_id.clone(), true), (second_id.clone(), true)]);
    // La seconde répondue d'abord : la première attend toujours.
    h.hub.act(action(&second_id, "choose", json!({ "choice": "a" }))).unwrap();
    assert_that!(json_of(&second.await.unwrap())).is_equal_to(json!({ "choice": "a" }));
    assert_that!(first.is_finished()).is_false();
    assert_that!(h.hub.has_pending_ask()).is_true();
    h.hub.act(action(&first_id, "choose", json!({ "choice": "b" }))).unwrap();
    assert_that!(json_of(&first.await.unwrap())).is_equal_to(json!({ "choice": "b" }));
    assert_that!(h.hub.surfaces().iter().any(|s| s.live)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn hub_close_turn_closes_live_params_of_that_turn() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let n = h.hub.begin_turn(AgentScope::Slides);
    let id = json_of(&call(&c, "show_params", params_args()).await)["shown"].as_str().unwrap().to_owned();
    h.hub.begin_turn(AgentScope::Slides);
    // Évincé, le tour emporte ses réglages encore vivants : fermés (pas figés), et leurs actions refusées.
    h.hub.close_turn(n);
    assert_that!(open_ids(&h)).is_empty();
    assert_that!(h.hub.act(action(&id, "dismiss", json!({})))).is_err();
}
