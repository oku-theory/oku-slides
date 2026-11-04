//! Spoken answers to the variant drawer, on a real session, and what an agent question reads aloud.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use common::*;
use oku_slides_mcp::{ARMING, AnswerOutcome, HeardUtterance, Lexicon, McpEvent};
use oku_slides_session::Request;
use oku_slides_session::agent::FocusSource;
use serde_json::json;
use speculoos::prelude::*;

/// `text`, said once anything shown is armed.
fn said(text: &str) -> HeardUtterance {
    HeardUtterance { seq: 5, text: text.into(), started: Instant::now() + ARMING + Duration::from_millis(10) }
}

/// Shows two variants of `s-libre`, "Sombre" and "Clair", and waits for the drawer.
async fn drawer(h: &Harness) {
    let mut rx = h.hub.subscribe();
    let variant = |t: &str| json!({ "label": t, "ops": [{ "op": "set_param", "target": { "slide": "s-libre", "element": "e-texte" }, "param": "body", "value": [{ "runs": [{ "t": t }] }] }] });
    call_within(
        &server(h).await,
        "show_variants",
        json!({ "slide": "s-libre", "variants": [variant("Sombre"), variant("Clair")] }),
    )
    .await;
    next_surface(&mut rx).await;
    assert_that!(h.hub.has_variants()).is_true();
}

/// Answers `text` to the drawer off the async thread: picking a variant locks the session.
async fn spoken(h: &Harness, text: &'static str) -> AnswerOutcome {
    let hub = h.hub.clone();
    tokio::task::spawn_blocking(move || {
        hub.answer_voice(&said(text), &Lexicon::for_locales("fr", None), Some(FocusSource::Host), None)
    })
    .await
    .unwrap_or_else(|e| panic!("{e}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn variant_ordinal_applies_one_undo_step() {
    let h = start().await;
    drawer(&h).await;
    assert_that!(spoken(&h, "la deuxième").await).is_equal_to(AnswerOutcome::Answered);
    assert_that!(h.hub.has_variants()).is_false();
    let session = h.session.clone();
    let (picked, undone) = tokio::task::spawn_blocking(move || {
        let mut s = session.lock().unwrap_or_else(|e| panic!("{e}"));
        let picked = s.snapshot().undo_label.clone();
        s.handle(Request::Undo).unwrap_or_else(|e| panic!("{e}"));
        (picked, s.snapshot().undo_label.clone())
    })
    .await
    .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(picked).is_equal_to(Some("Variante : Clair".to_owned()));
    assert_that!(undone).is_not_equal_to(Some("Variante : Clair".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn variant_by_label_applies_it() {
    let h = start().await;
    drawer(&h).await;
    assert_that!(spoken(&h, "sombre").await).is_equal_to(AnswerOutcome::Answered);
    let label = h.session.lock().unwrap_or_else(|e| panic!("{e}")).snapshot().undo_label.clone();
    assert_that!(label).is_equal_to(Some("Variante : Sombre".to_owned()));
}

/// The hub decides under its lock and applies after: while another thread holds the session
/// and then asks the hub (session, then hub: the allowed order), a spoken pick waiting for
/// the session holds no hub lock, so nothing deadlocks.
#[tokio::test(flavor = "multi_thread")]
async fn answer_decides_under_lock_acts_after() {
    let h = start().await;
    drawer(&h).await;
    let (session, hub) = (h.session.clone(), h.hub.clone());
    let (held_tx, held_rx) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let _held = session.lock().unwrap_or_else(|e| panic!("{e}"));
        held_tx.send(()).unwrap_or_else(|e| panic!("{e}"));
        // The drawer leaves the hub under its lock: once gone, the pick has decided and released
        // the hub lock, and waits for the session. Then read the hub under the session lock.
        let deadline = Instant::now() + Duration::from_secs(10);
        while hub.has_variants() {
            assert!(Instant::now() < deadline, "the spoken pick never decided");
            std::thread::sleep(Duration::from_millis(5));
        }
        hub.surfaces().len()
    });
    held_rx.recv().unwrap_or_else(|e| panic!("{e}"));
    let picking = tokio::spawn({
        let hub: Arc<_> = h.hub.clone();
        async move {
            tokio::task::spawn_blocking(move || {
                hub.answer_voice(&said("la première"), &Lexicon::for_locales("fr", None), Some(FocusSource::Host), None)
            })
            .await
        }
    });
    let finished = tokio::time::timeout(Duration::from_secs(20), async {
        let read = tokio::task::spawn_blocking(move || holder.join()).await;
        (read, picking.await)
    })
    .await
    .unwrap_or_else(|_| panic!("deadlock: the hub lock was held while waiting for the session"));
    assert_that!(finished.0.is_ok_and(|r| r.is_ok())).is_true();
    assert_that!(finished.1.ok().and_then(Result::ok)).is_equal_to(Some(AnswerOutcome::Answered));
}

/// What an `ask_confirm` opening says aloud, `danger` or not.
async fn confirm_speech(danger: bool) -> String {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = server(&h).await;
    let pending = tokio::spawn({
        let c = c.clone();
        async move { call(&c, "ask_confirm", json!({ "message": "Sûr ?", "danger": danger })).await }
    });
    let spoken = wait_event(&mut rx, |e| match e {
        McpEvent::Surface { spoken, .. } => spoken,
        _ => None,
    })
    .await;
    h.hub.cancel_asks();
    drop(pending.await);
    spoken.text().to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_confirm_reads_its_question_then_how_to_answer() {
    let l = oku_slides_i18n::Localizer::new(Some("fr"));
    let expected = format!(
        "{} {}",
        l.tr_with("voice-say-question", &[("question", "Sûr ?".into())]),
        l.tr("voice-say-confirm-yes")
    );
    assert_that!(confirm_speech(false).await).is_equal_to(expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn ask_confirm_with_danger_reads_the_verb_to_say() {
    let l = oku_slides_i18n::Localizer::new(Some("fr"));
    let verb = l.tr("agent-surface-confirm-danger");
    let expected = format!(
        "{} {}",
        l.tr_with("voice-say-question", &[("question", "Sûr ?".into())]),
        l.tr_with("voice-say-confirm", &[("verb", verb.into())])
    );
    assert_that!(confirm_speech(true).await).is_equal_to(expected);
}
