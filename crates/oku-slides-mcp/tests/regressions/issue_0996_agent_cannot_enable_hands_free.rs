//! t-996: the agent may turn hands-free listening off, never on.

use oku_slides_i18n::Localizer;
use oku_slides_session::{DictationPatch, VoiceEvent};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::*;

#[tokio::test(flavor = "multi_thread")]
async fn issue_0996_agent_hands_free_on_is_refused_in_french() {
    let h = start().await;
    let r = call_within(&server(&h).await, "set_dictation", json!({"hands_free": true})).await;
    let expected = Localizer::new(Some("fr")).tr("voice-error-hands-free-reserved");
    assert_that!(expected.as_str()).does_not_contain("voice-error-");
    assert_that!(text_of(&r)).is_equal_to(expected);
    assert_that!(*lock(&h.voice.applied)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_0996_agent_hands_free_off_is_applied() {
    let h = start().await;
    call_within(&server(&h).await, "set_dictation", json!({"hands_free": false})).await;
    let applied: Vec<VoiceEvent> = lock(&h.voice.applied).iter().map(|(e, _)| e.clone()).collect();
    let off = VoiceEvent::Dictation(DictationPatch { hands_free: Some(false), ..Default::default() });
    assert_that!(applied).is_equal_to(vec![off]);
}
