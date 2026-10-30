use std::sync::Arc;

use oku_slides_a2ui::ChoiceOption;
use oku_slides_i18n::{LiveLocalizer, Localizer};
use oku_slides_session::agent::forms::{self, AskChoice};
use oku_slides_session::agent::{Sessions, Surface, TabSession};
use speculoos::prelude::*;
use tokio::task::JoinHandle;

use super::{Answer, McpEvent, SurfaceHub};

/// Aucune session : ces tests ne touchent qu'aux questions, `f` n'est jamais appelée.
#[derive(Debug)]
pub(super) struct NoSession;

impl Sessions for NoSession {
    fn with(&self, _tab: &str, _f: &mut dyn FnMut(&mut oku_slides_session::Session)) -> oku_slides_session::Result<()> {
        Ok(())
    }

    fn open_pack(
        &self,
        _tab: &str,
        _pack: uuid::Uuid,
        _widget: Option<&str>,
        _gate: &mut dyn FnMut(oku_slides_session::agent::TabGate) -> bool,
    ) -> oku_slides_session::Result<oku_slides_session::agent::TabOpen> {
        unreachable!("NoSession is never asked to open a pack")
    }

    fn open_theme(
        &self,
        _tab: &str,
        _theme: uuid::Uuid,
        _gate: &mut dyn FnMut(oku_slides_session::agent::TabGate) -> bool,
    ) -> oku_slides_session::Result<oku_slides_session::agent::TabOpen> {
        unreachable!("NoSession is never asked to open a theme")
    }

    fn document_kind(&self, _tab: &str) -> oku_slides_session::Result<oku_slides_session::DocumentKind> {
        unreachable!("NoSession is never asked its kind")
    }
}

pub(crate) fn hub() -> Arc<SurfaceHub> {
    SurfaceHub::new(
        TabSession::new("t-1", Arc::new(NoSession)),
        LiveLocalizer::new(Arc::new(Localizer::new(Some("fr")))),
        Arc::new(crate::NoVoice),
    )
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

/// `ask_with` enregistre la question et l'affiche sous un seul verrou (t-454) : à tout instant
/// observé sous ce verrou, les questions en attente sont exactement les surfaces ouvertes, dans
/// le même ordre. Sans crochet dans `ask_with`, la fenêtre entre deux verrous ne se force pas :
/// un observateur relit l'état en boucle pendant que des questions s'ouvrent et se ferment en
/// parallèle (une régression vers deux verrous s'y voit presque à chaque lancement, pas à coup sûr).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ask_registers_and_shows_a_question_under_one_lock() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    let hub = hub();
    let stop = Arc::new(AtomicBool::new(false));
    let seen = Arc::new(AtomicUsize::new(0));
    let observer = std::thread::spawn({
        let (hub, stop, seen) = (hub.clone(), stop.clone(), seen.clone());
        move || {
            while !stop.load(Ordering::Relaxed) {
                let state = hub.state();
                let asks: Vec<&str> = state.asks.iter().map(|a| a.id.as_str()).collect();
                let open: Vec<&str> = state.open.iter().map(|s| s.id.as_str()).collect();
                if asks != open {
                    return Err(format!("en attente {asks:?}, ouvertes {open:?}"));
                }
                if !asks.is_empty() {
                    seen.fetch_add(1, Ordering::Relaxed);
                }
            }
            Ok(())
        }
    });
    for round in 0..50 {
        let tasks: Vec<_> = (0..8)
            .map(|i| {
                let (hub, surface) = (hub.clone(), choice(&format!("ask-{round}-{i}")));
                tokio::spawn(async move { hub.ask(surface).await })
            })
            .collect();
        tokio::task::yield_now().await;
        if round == 0 {
            // Un exécuteur chargé : l'observateur doit avoir vu une question en attente au moins
            // une fois avant la première annulation (sinon `seen` resterait à 0, verrou correct ou non).
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
            while seen.load(Ordering::Relaxed) == 0 && tokio::time::Instant::now() < deadline {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        }
        hub.cancel_asks();
        for task in tasks {
            // Une question ouverte après `cancel_asks` attendrait toujours : annulée à son tour.
            while !task.is_finished() {
                hub.cancel_asks();
                tokio::task::yield_now().await;
            }
            let _ = task.await;
        }
    }
    stop.store(true, Ordering::Relaxed);
    let verdict = observer.join().unwrap_or_else(|_| panic!("l'observateur a paniqué"));
    assert_that!(verdict).is_ok();
    asserting("l'observateur a vu des questions en attente").that(&seen.load(Ordering::Relaxed)).is_greater_than(0);
}
