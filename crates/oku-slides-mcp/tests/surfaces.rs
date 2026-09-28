mod common;

use common::*;
use oku_slides_mcp::{McpEvent, SurfaceAction};
use oku_slides_session::agent::{AskChoice, forms};
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

fn action(surface: &str, name: &str, context: Value) -> SurfaceAction {
    SurfaceAction {
        surface: surface.into(),
        name: name.into(),
        context: context.as_object().cloned().unwrap_or_else(Map::new),
    }
}

fn choice_args() -> Value {
    json!({ "question": "Laquelle ?", "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] })
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
async fn ask_choice_dismissed_returns_cancelled_and_closes_surface() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_choice", choice_args()).await }
    });
    let surface = next_surface(&mut rx).await;
    h.hub.act(action(&surface.id, "dismiss", json!({}))).unwrap();
    assert_that!(json_of(&pending.await.unwrap())).is_equal_to(json!({ "cancelled": true }));
    let closed = loop {
        if let McpEvent::CloseSurface { id } = rx.recv().await.unwrap() {
            break id;
        }
    };
    assert_that!(closed).is_equal_to(surface.id);
}

#[tokio::test(flavor = "multi_thread")]
async fn answer_pending_turns_dictation_into_answer_text() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_choice", choice_args()).await }
    });
    next_surface(&mut rx).await;
    assert_that!(h.hub.has_pending_ask()).is_true();
    assert_that!(h.hub.answer_pending("la deuxième")).is_true();
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
async fn ask_rewrite_applies_chosen_proposal() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "slide": "s-libre", "element": "e-texte", "proposals": ["Premier", "Second"] });
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
    let text = s.slide("s-libre").unwrap().targets.iter().find(|t| t.id == "e-texte").and_then(|t| t.text.clone());
    assert_that!(text.as_deref()).is_equal_to(Some("Second"));
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_rewrite_paragraphs_answer_carries_a_bold_run() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "slide": "s-libre", "element": "e-texte", "proposals": ["Premier", "Second"] });
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
    let run = &located.element.body.text().unwrap()[0].runs[0];
    assert_that!(run.bold).is_true();
    assert_that!(run.t.as_str()).is_equal_to("Gras");
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_rewrite_without_paragraphs_for_an_element_fails() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "slide": "s-libre", "element": "e-texte", "proposals": ["Premier", "Second"] });
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
async fn ask_rewrite_with_invalid_paragraphs_fails_in_english() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "slide": "s-libre", "element": "e-texte", "proposals": ["Premier", "Second"] });
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_rewrite", args).await }
    });
    let surface = next_surface(&mut rx).await;
    h.hub.act(action(&surface.id, "rewrite", json!({ "kept": 1, "text": "Second", "paragraphs": 3 }))).unwrap();
    let r = pending.await.unwrap();
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r).starts_with("invalid paragraphs: ")).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn show_variants_returns_at_once_and_pick_applies_with_own_undo_and_note() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let variant = |t: &str| json!({ "label": t, "ops": [{ "op": "set_text", "slide": "s-libre", "element": "e-texte", "text": [{ "runs": [{ "t": t }] }] }] });
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
        { "label": "Thème", "kind": "select", "value": "papier", "op_template": { "op": "set_theme", "theme": "$value" },
          "options": [{ "id": "papier", "label": "Papier" }, { "id": "pro", "label": "Pro" }] }
    ] });
    let id = json_of(&call(&c, "show_params", args).await)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    h.hub.act(action(&id, "param", json!({ "field": 0, "value": "pro" }))).unwrap();
    assert_that!(h.session.lock().unwrap().deck().theme.as_deref()).is_equal_to(Some("pro"));
}

#[tokio::test(flavor = "multi_thread")]
async fn act_keeps_surface_on_apply_error_so_it_can_still_be_dismissed() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let variant = |t: &str| json!({ "label": t, "ops": [{ "op": "set_text", "slide": "s-libre", "element": "e-texte", "text": [{ "runs": [{ "t": t }] }] }] });
    let r =
        call(&c, "show_variants", json!({ "slide": "s-libre", "variants": [variant("Un"), variant("Deux")] })).await;
    let id = json_of(&r)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    // La slide visée par les variantes disparaît sans passer par `prune` :
    // choisir une variante doit maintenant échouer à l'application.
    call(&c, "apply", json!({ "ops": [{ "op": "remove_slide", "slide": "s-libre" }] })).await;
    assert_that!(h.hub.act(action(&id, "variant", json!({ "index": 0 })))).is_err();
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
    let closed = loop {
        if let McpEvent::CloseSurface { id } = rx.recv().await.unwrap() {
            break id;
        }
    };
    assert_that!(closed).is_equal_to(id);
    assert_that!(h.hub.has_pending_ask()).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_rewrite_answer_without_text_fails() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "slide": "s-libre", "element": "e-texte", "proposals": ["Premier", "Second"] });
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_rewrite", args).await }
    });
    let surface = next_surface(&mut rx).await;
    h.hub.act(action(&surface.id, "rewrite", json!({ "kept": 1 }))).unwrap();
    let r = pending.await.unwrap();
    assert_that!(r.is_error).is_equal_to(Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn prune_closes_params_of_removed_slide() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let args = json!({ "title": "x", "slide": "s-libre", "fields": [
        { "label": "Thème", "kind": "text", "value": "papier", "op_template": { "op": "set_theme", "theme": "$value" } }
    ] });
    let id = json_of(&call(&c, "show_params", args).await)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    call(&c, "apply", json!({ "ops": [{ "op": "remove_slide", "slide": "s-libre" }] })).await;
    h.hub.prune();
    let closed = loop {
        if let McpEvent::CloseSurface { id } = rx.recv().await.unwrap() {
            break id;
        }
    };
    assert_that!(closed).is_equal_to(id);
    assert_that!(h.hub.act(action(&closed, "dismiss", json!({})))).is_err();
}

/// Identifiants des surfaces que le hub tient pour ouvertes.
fn open_ids(h: &Harness) -> Vec<String> {
    h.hub.surfaces().into_iter().map(|s| s.id).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn surfaces_follow_open_and_close_in_order() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let params = json!({ "title": "x", "slide": "s-libre", "fields": [
        { "label": "Thème", "kind": "text", "value": "papier", "op_template": { "op": "set_theme", "theme": "$value" } }
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
    while !matches!(rx.recv().await.unwrap(), McpEvent::CloseSurface { .. }) {}
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
    h.hub.resync();
    let list = loop {
        if let McpEvent::Surfaces { list } = rx.recv().await.unwrap() {
            break list;
        }
    };
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
        { "label": "Thème", "kind": "text", "value": "papier", "op_template": { "op": "set_theme", "theme": "$value" } }
    ] });
    let shown = json_of(&call(&c, "show_params", params).await)["shown"].as_str().unwrap().to_owned();
    next_surface(&mut rx).await;
    h.hub.emit(McpEvent::Surfaces { list: vec![] });
    let list = loop {
        if let McpEvent::Surfaces { list } = rx.recv().await.unwrap() {
            break list;
        }
    };
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
        { "label": "Thème", "kind": "select", "value": "papier", "op_template": { "op": "set_theme", "theme": "$value" },
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

/// t-249 : sans post-it, `prune` ne prend pas la session (un rendu en cours ne la bloque pas).
#[tokio::test(flavor = "multi_thread")]
async fn prune_without_post_it_does_not_wait_for_the_session() {
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

fn suggest_args(labels: &[&str]) -> Value {
    json!({ "suggestions": labels.iter().map(|l| json!({ "label": l })).collect::<Vec<_>>() })
}

#[tokio::test(flavor = "multi_thread")]
async fn suggest_next_returns_at_once_with_suggestion_buttons() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "suggest_next", suggest_args(&["Mettre en barres"])).await;
    let id = json_of(&r)["shown"].as_str().unwrap().to_owned();
    let surface = next_surface(&mut rx).await;
    assert_that!(surface.id.as_str()).is_equal_to(id.as_str());
    assert_that!(surface.place).is_equal_to(oku_slides_session::agent::Place::Suggestions);
}

#[tokio::test(flavor = "multi_thread")]
async fn suggest_next_replaces_the_previous_suggestions() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    call(&c, "suggest_next", suggest_args(&["Un"])).await;
    let second = json_of(&call(&c, "suggest_next", suggest_args(&["Deux"])).await)["shown"].as_str().unwrap().to_owned();
    assert_that!(open_ids(&h)).is_equal_to(vec![second]);
}

#[tokio::test(flavor = "multi_thread")]
async fn suggest_next_with_five_suggestions_is_refused() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let r = call(&c, "suggest_next", suggest_args(&["a", "b", "c", "d", "e"])).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(open_ids(&h)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn clicked_suggestion_closes_its_surface() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let id = json_of(&call(&c, "suggest_next", suggest_args(&["Un"])).await)["shown"].as_str().unwrap().to_owned();
    h.hub.act(action(&id, "suggest", json!({ "prompt": "Un" }))).unwrap();
    assert_that!(open_ids(&h)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn clear_suggestions_closes_them_and_keeps_other_surfaces() {
    let h = start().await;
    let c = client(h.server.url(), Some(h.server.authorization())).await.unwrap();
    let params = json!({ "title": "x", "slide": "s-libre", "fields": [
        { "label": "Thème", "kind": "text", "value": "papier", "op_template": { "op": "set_theme", "theme": "$value" } }
    ] });
    let shown = json_of(&call(&c, "show_params", params).await)["shown"].as_str().unwrap().to_owned();
    call(&c, "suggest_next", suggest_args(&["Un"])).await;
    h.hub.clear_suggestions();
    assert_that!(open_ids(&h)).is_equal_to(vec![shown]);
}
