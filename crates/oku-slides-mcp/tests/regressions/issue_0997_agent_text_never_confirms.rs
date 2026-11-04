//! t-997: typed text cannot be told from a draft the agent prefilled, so it never confirms a
//! guarded tool. Only a host-routed microphone utterance, armed and from a host-focused window,
//! or a click does.

use std::time::{Duration, Instant};

use oku_slides_i18n::Localizer;
use oku_slides_mcp::{ARMING, AnswerOutcome, HeardUtterance, Lexicon};
use oku_slides_session::agent::FocusSource;
use serde_json::json;
use speculoos::prelude::*;

use crate::common::*;

/// `text` said `after` the question showed.
fn said(text: &str, after: Duration) -> HeardUtterance {
    HeardUtterance { seq: 7, text: text.into(), started: Instant::now() + after }
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_0997_typed_or_prefilled_reply_never_confirms_only_an_armed_utterance_does() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let c = server(&h).await;
    let pending =
        tokio::spawn(async move { call(&c, "merge", json!({ "path": "/nowhere/copie.oku-slides.pdf" })).await });
    let asked = next_surface(&mut rx).await;
    let lexicon = Lexicon::for_locales("fr", None);
    let host = Some(FocusSource::Host);

    // Typed, or the agent's prefilled draft sent as is: the same typed path, kept open.
    assert_that!(h.hub.answer(&asked.id, "oui, fusionner")).is_equal_to(AnswerOutcome::KeptOpen);
    // Spoken before the question could be read: kept open too.
    assert_that!(h.hub.answer_voice(&said("oui, fusionner", Duration::ZERO), &lexicon, host, None))
        .is_equal_to(AnswerOutcome::KeptOpen);
    tokio::task::yield_now().await;
    assert_that!(pending.is_finished()).is_false();

    let armed = said("oui, fusionner", ARMING + Duration::from_millis(10));
    assert_that!(h.hub.answer_voice(&armed, &lexicon, host, None)).is_equal_to(AnswerOutcome::Answered);
    // Confirmed: the merge ran (and failed on the missing file), it was not refused.
    let result = pending.await.unwrap_or_else(|e| panic!("{e}"));
    assert_that!(text_of(&result)).is_equal_to(Localizer::new(Some("fr")).tr("agent-error-file"));
}
