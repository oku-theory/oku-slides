use std::sync::Arc;

use oku_slides_a2ui::ChoiceOption;
use oku_slides_i18n::Localizer;
use oku_slides_session::agent::forms::{self, AskChoice};
use oku_slides_session::agent::{Sessions, Surface, TabSession};
use speculoos::prelude::*;
use tokio::task::JoinHandle;

use super::{Answer, McpEvent, SurfaceHub};

/// Aucune session : ces tests ne touchent qu'aux questions, `f` n'est jamais appelée.
#[derive(Debug)]
struct NoSession;

impl Sessions for NoSession {
    fn with(&self, _tab: &str, _f: &mut dyn FnMut(&mut oku_slides_session::Session)) -> oku_slides_session::Result<()> {
        Ok(())
    }
}

fn hub() -> Arc<SurfaceHub> {
    SurfaceHub::new(TabSession::new("t-1", Arc::new(NoSession)), Arc::new(Localizer::new(Some("fr"))))
}

fn choice(id: &str) -> Surface {
    let option = |id: &str| ChoiceOption { id: id.into(), label: id.to_uppercase(), hint: None };
    let spec = AskChoice { question: format!("{id} ?"), options: vec![option("a"), option("b")], anchor: None };
    forms::choice(id, &spec, &Localizer::new(Some("fr")))
}

/// Pose la question `id` et attend qu'elle soit affichée ; rend la tâche qui attend sa réponse.
async fn asked(hub: &Arc<SurfaceHub>, id: &str) -> JoinHandle<Answer> {
    let mut rx = hub.subscribe();
    let task = tokio::spawn({
        let (hub, surface) = (hub.clone(), choice(id));
        async move { hub.ask(surface).await }
    });
    while !matches!(rx.recv().await, Ok(McpEvent::Surface { surface }) if surface.id == id) {}
    task
}

fn answer_text(answer: &Answer) -> Option<&str> {
    match answer {
        Answer::Action { name, context } if name == "answer" => context.get("text").and_then(|v| v.as_str()),
        _ => None,
    }
}

/// La consigne dictée répond à la question que l'interface a désignée (`quickTarget`), même
/// si une autre, ouverte après elle, attend aussi.
#[tokio::test]
async fn answer_given_question_answers_it_and_not_another_pending_one() {
    let hub = hub();
    let first = asked(&hub, "ask-a").await;
    let last = asked(&hub, "ask-b").await;

    assert_that!(hub.answer("ask-a", "la deuxième")).is_true();

    let answer = first.await.unwrap();
    assert_that!(answer_text(&answer)).is_equal_to(Some("la deuxième"));
    assert_that!(last.is_finished()).is_false();
    assert_that!(hub.has_pending_ask()).is_true();
    assert_that!(hub.surfaces().iter().map(|s| s.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["ask-b"]);
}

/// Une question déjà répondue, ou inconnue, n'est pas remplacée par une autre en attente.
#[tokio::test]
async fn answer_question_no_longer_pending_is_refused_and_answers_nothing() {
    let hub = hub();
    let first = asked(&hub, "ask-a").await;
    let last = asked(&hub, "ask-b").await;
    assert_that!(hub.answer("ask-a", "oui")).is_true();
    assert_that!(answer_text(&first.await.unwrap())).is_equal_to(Some("oui"));

    assert_that!(hub.answer("ask-a", "encore")).is_false();
    assert_that!(hub.answer("ask-z", "inconnue")).is_false();
    tokio::task::yield_now().await;

    // Toujours en attente, et non annulée : sa tâche court encore et elle reste affichée.
    assert_that!(last.is_finished()).is_false();
    assert_that!(hub.has_pending_ask()).is_true();
    assert_that!(hub.surfaces().iter().map(|s| s.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["ask-b"]);
}
