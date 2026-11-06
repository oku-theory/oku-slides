//! t-1522: an answer a question read aloud tells the user to say is never taken for its echo.

use oku_slides_i18n::Localizer;
use oku_slides_session::agent::{SpokenKind, SpokenText};
use oku_slides_stt::hand_voice::{HandClock, HandEngine};
use oku_slides_stt::{SPEECH_GUARD, Speaker, SpeakerClock, Speech, SpeechRate, Utterance};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::common::{self, logged, utterance, wait_turn_done};

const WITHIN: std::time::Duration = std::time::Duration::from_secs(10);

/// What Oku asks: a danger confirmation, a plain one, or a choice.
#[derive(Debug, Clone, Copy)]
enum Asked {
    Danger,
    Yes,
    Choice,
}

/// The real host speech for `asked` in `locale`, as the reader builds it.
fn spoken(asked: Asked, l: &Localizer) -> SpokenText {
    match asked {
        Asked::Danger => SpokenText::agent_confirm("Delete?", Some(&l.tr("agent-surface-confirm-danger")), l),
        Asked::Yes => SpokenText::confirm(None, l),
        Asked::Choice => SpokenText::question("Which theme?", &["Ocean blue", "Forest green"], l),
    }
}

#[rstest]
#[case::en_danger("en", Asked::Danger, "Yes, do it")]
#[case::fr_danger("fr", Asked::Danger, "oui le faire")]
#[case::ja_danger("ja", Asked::Danger, "はい、実行します")]
#[case::en_yes("en", Asked::Yes, "yes")]
#[case::fr_yes("fr", Asked::Yes, "oui")]
#[case::ja_yes("ja", Asked::Yes, "いいえ")]
#[case::en_option("en", Asked::Choice, "Option 2")]
#[case::fr_option_label("fr", Asked::Choice, "forest green")]
#[case::ja_option("ja", Asked::Choice, "選択肢 2")]
#[tokio::test(flavor = "multi_thread")]
async fn issue_1522_answer_quoted_by_the_question_reaches_the_agent(
    #[case] locale: &str,
    #[case] asked: Asked,
    #[case] answer: &str,
) {
    let l = Localizer::new(Some(locale));
    let spoken = spoken(asked, &l);
    let (engine, clock) = (std::sync::Arc::new(HandEngine::polyglot()), HandClock::new());
    let speaker = Speaker::with_clock(engine.clone(), None, clock.clone());
    let (stack, rx, log) = common::stack_with_speaker(json!({ "turns": [{}] }), &["t-1"], speaker.clone()).await;
    stack.workspace.focus_tab("t-1");
    let voice = stack.voice();
    speaker.say(Speech {
        text: spoken.text().to_owned(),
        lang: spoken.lang().to_owned(),
        voice: None,
        rate: SpeechRate::default(),
        ended: None,
        asks: spoken.kind() == SpokenKind::Question,
    });
    assert_that!(speaker.settle(WITHIN)).is_true();
    engine.done(1);
    assert_that!(speaker.settle(WITHIN)).is_true();
    clock.advance(SPEECH_GUARD);
    speaker.tick();
    assert_that!(speaker.settle(WITHIN)).is_true();
    // Within the echo horizon, once listening resumed: the user's answer.
    voice.send(Utterance { started: clock.now(), ..utterance(1, answer) }).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let sent: Vec<String> = logged(&log, "session/prompt").iter().map(|p| p["prompt"].to_string()).collect();
    assert_that!((sent.len(), sent[0].contains(answer))).is_equal_to((1, true));
}
