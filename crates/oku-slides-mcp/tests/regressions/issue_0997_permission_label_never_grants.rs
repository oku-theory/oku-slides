//! t-997 (Q3): the agent writes permission labels and their order, so a spoken answer picks an
//! option by its kind, never by label or position: allow-always is never picked by voice.

use std::time::{Duration, Instant};

use oku_slides_mcp::{ARMING, AnswerOutcome, HeardUtterance, Lexicon};
use oku_slides_session::agent::{FocusSource, PermissionChoice, PermissionKind};
use speculoos::prelude::*;

use crate::common::*;

fn said(text: &str) -> HeardUtterance {
    HeardUtterance { seq: 3, text: text.into(), started: Instant::now() + ARMING + Duration::from_millis(10) }
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_0997_allow_always_labelled_no_is_never_granted_by_voice() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    let choices: Vec<PermissionChoice> = [
        ("non", PermissionKind::AllowAlways),
        ("Autoriser une fois", PermissionKind::AllowOnce),
        ("Refuser", PermissionKind::RejectOnce),
    ]
    .into_iter()
    .map(|(name, kind)| PermissionChoice { name: name.into(), kind })
    .collect();
    let asking = tokio::spawn({
        let hub = h.hub.clone();
        async move { hub.permission("Écrire le fichier ?", &choices).await }
    });
    next_surface(&mut rx).await;
    let lexicon = Lexicon::for_locales("fr", None);
    let host = Some(FocusSource::Host);

    // "The first" names the allow-always option by position: kept open.
    assert_that!(h.hub.answer_voice(&said("le premier"), &lexicon, host, None)).is_equal_to(AnswerOutcome::KeptOpen);
    tokio::task::yield_now().await;
    assert_that!(asking.is_finished()).is_false();
    // "Non" is its label, but a refusal by kind: reject-once.
    assert_that!(h.hub.answer_voice(&said("non"), &lexicon, host, None)).is_equal_to(AnswerOutcome::Answered);
    assert_that!(asking.await.unwrap_or_else(|e| panic!("{e}"))).is_equal_to(Some(2));
}
