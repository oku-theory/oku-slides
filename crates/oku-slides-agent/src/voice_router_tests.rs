use std::sync::Weak;
use std::sync::mpsc;
use std::time::Instant;

use oku_slides_mcp::{Lexicon, Nav};
use oku_slides_session::agent::{FocusSource, TurnOrigin};
use oku_slides_stt::{Utterance, UtteranceSeq};
use rstest::rstest;
use speculoos::prelude::*;

use super::{DropReason, RouterEnd, VoiceRoute, VoiceTarget, route_voice, run, spawn};

fn said(text: &str) -> Utterance {
    let now = Instant::now();
    Utterance { seq: UtteranceSeq(7), text: text.to_owned(), started: now, ended: now, speech_secs: 1.0 }
}

fn target(focus: Option<FocusSource>, presenting: bool, open_question: bool, drawer: bool) -> VoiceTarget {
    VoiceTarget { tab: "t-1".into(), focus, presenting, open_question, drawer }
}

fn focused() -> VoiceTarget {
    target(Some(FocusSource::Host), false, false, false)
}

/// The theme shortcut word of these tests.
fn shortcut(text: &str) -> bool {
    text == "theme"
}

/// A route, as a comparable summary: its kind, then its text or detail.
fn summary(route: &VoiceRoute) -> (String, String) {
    let (kind, detail) = match route {
        VoiceRoute::Stop { .. } => ("stop", String::new()),
        VoiceRoute::Answer { heard, .. } => ("answer", heard.text.clone()),
        VoiceRoute::Nav { nav, .. } => ("nav", format!("{nav:?}")),
        VoiceRoute::Shortcut { text, .. } => ("shortcut", text.clone()),
        VoiceRoute::Prompt { text, origin, .. } => ("prompt", format!("{origin:?}:{text}")),
        VoiceRoute::Drop(reason) => ("drop", format!("{reason:?}")),
    };
    (kind.to_owned(), detail)
}

fn route_en(text: &str, t: Option<&VoiceTarget>) -> (String, String) {
    summary(&route_voice(&said(text), t, &Lexicon::for_locales("en", None), &shortcut))
}

#[rstest]
#[case::en("en", "stop")]
#[case::en_filler("en", "please stop")]
#[case::fr("fr", "arrête")]
#[case::de("de", "hör auf")]
fn stop_word_returns_stop(#[case] locale: &str, #[case] text: &str) {
    let route = route_voice(&said(text), Some(&focused()), &Lexicon::for_locales(locale, None), &shortcut);
    assert_that!(summary(&route).0.as_str()).is_equal_to("stop");
}

#[test]
fn stop_inside_a_sentence_is_a_prompt() {
    assert_that!(route_en("stop the animation on slide two", Some(&focused())))
        .is_equal_to(("prompt".to_owned(), "Voice:stop the animation on slide two".to_owned()));
}

#[test]
fn background_utterance_without_wake_is_dropped() {
    let background = target(None, false, false, false);
    assert_that!(route_en("make the title bigger", Some(&background)))
        .is_equal_to(("drop".to_owned(), "Background".to_owned()));
}

#[test]
fn background_wake_phrase_prompts_rest() {
    let background = target(None, false, false, false);
    assert_that!(route_en("hey oku make the title bigger", Some(&background)))
        .is_equal_to(("prompt".to_owned(), "Wake:make the title bigger".to_owned()));
}

#[test]
fn background_stop_word_returns_stop() {
    let background = target(None, false, false, false);
    assert_that!(route_en("stop", Some(&background)).0.as_str()).is_equal_to("stop");
}

#[test]
fn no_target_drops_without_logging_text() {
    let route = route_voice(&said("secret words"), None, &Lexicon::for_locales("en", None), &shortcut);
    let VoiceRoute::Drop(reason) = route else { panic!("not dropped: {route:?}") };
    assert_that!(reason).is_equal_to(DropReason::NoTarget);
    // The reason is all that is logged: it carries no text.
    assert_that!(format!("{reason:?}")).does_not_contain("secret");
}

#[test]
fn bare_wake_phrase_is_noise() {
    assert_that!(route_en("hey oku", Some(&focused()))).is_equal_to(("drop".to_owned(), "Noise".to_owned()));
}

/// Every row of the routing table: text, (focus, presenting, open question, drawer), route.
#[rstest]
#[case::stop_presenting("stop", (true, true, false, false), "stop", "")]
#[case::stop_with_question("stop", (true, false, true, false), "stop", "")]
#[case::presenting_wake_phrase_prompts_rest("oku next slide please", (true, true, false, false), "prompt", "Wake:next slide please")]
#[case::wake_with_question("oku yes", (true, false, true, false), "prompt", "Wake:yes")]
#[case::presenting_nav_word_emits_nav("next slide", (true, true, false, false), "nav", "Next")]
#[case::nav_first_presenting("first slide", (true, true, false, false), "nav", "First")]
#[case::nav_last_presenting("last slide", (true, true, false, false), "nav", "Last")]
#[case::nav_back_presenting("go back", (true, true, false, false), "nav", "Previous")]
#[case::presenting_other_speech_is_dropped("make it red", (true, true, false, false), "drop", "Presenting")]
#[case::nav_not_presenting("next slide", (true, false, false, false), "prompt", "Voice:next slide")]
#[case::nav_background("next slide", (false, true, false, false), "drop", "Background")]
#[case::shortcut("theme", (true, false, false, false), "shortcut", "theme")]
#[case::shortcut_over_question("theme", (true, false, true, false), "shortcut", "theme")]
#[case::shortcut_presenting("theme", (true, true, false, false), "drop", "Presenting")]
#[case::shortcut_presenting_with_question("theme", (true, true, true, false), "answer", "theme")]
#[case::answer_question("yes", (true, false, true, false), "answer", "yes")]
#[case::answer_drawer("the second one", (true, false, false, true), "answer", "the second one")]
#[case::prompt("make it red", (true, false, false, false), "prompt", "Voice:make it red")]
#[case::other_background("make it red", (false, false, false, false), "drop", "Background")]
#[case::answer_form_idle("yes", (true, false, false, false), "prompt", "Voice:yes")]
#[case::answer_form_presenting("yes", (true, true, false, false), "drop", "Presenting")]
#[case::answer_form_background("yes", (false, false, true, false), "drop", "Background")]
#[case::nav_with_question("next slide", (true, false, true, false), "answer", "next slide")]
#[case::wake_background("oku add a slide", (false, false, false, false), "prompt", "Wake:add a slide")]
#[case::stop_background("stop", (false, false, false, false), "stop", "")]
#[case::shortcut_background("theme", (false, false, false, false), "drop", "Background")]
#[case::answer_presenting_with_question("yes", (true, true, true, false), "answer", "yes")]
#[case::nav_presenting_with_question("next slide", (true, true, true, false), "answer", "next slide")]
#[case::drawer_presenting("the second one", (true, true, false, true), "answer", "the second one")]
#[case::empty("   ", (true, false, false, false), "drop", "Noise")]
#[case::filler_hmm("hmm", (true, false, false, false), "drop", "Noise")]
#[case::filler_uh("uh", (true, false, false, false), "drop", "Noise")]
#[case::filler_line("um, thank you", (true, false, false, false), "drop", "Noise")]
#[case::filler_presenting("hmm", (true, true, false, false), "drop", "Noise")]
#[case::filler_with_question("um", (true, false, true, false), "answer", "um")]
#[case::wake_filler_background("oku hmm", (false, false, false, false), "drop", "Noise")]
#[case::wake_filler_focused("Oku, uh…", (true, false, false, false), "drop", "Noise")]
#[case::wake_filler_with_question("oku um", (true, false, true, false), "drop", "Noise")]
fn route_voice_table(
    #[case] text: &str,
    #[case] columns: (bool, bool, bool, bool),
    #[case] kind: &str,
    #[case] detail: &str,
) {
    let (has_focus, presenting, open_question, drawer) = columns;
    let t = target(has_focus.then_some(FocusSource::PageReported), presenting, open_question, drawer);
    assert_that!(route_en(text, Some(&t))).is_equal_to((kind.to_owned(), detail.to_owned()));
}

#[test]
fn route_voice_nav_kinds_are_the_lexicon_ones() {
    let t = target(Some(FocusSource::Host), true, false, false);
    let route = route_voice(&said("exit presentation"), Some(&t), &Lexicon::for_locales("en", None), &shortcut);
    let VoiceRoute::Nav { nav, tab } = route else { panic!("not nav: {route:?}") };
    assert_that!((nav, tab.as_str())).is_equal_to((Nav::Exit, "t-1"));
}

#[test]
fn route_voice_wake_turn_is_marked_wake() {
    let route =
        route_voice(&said("ok oku add a slide"), Some(&focused()), &Lexicon::for_locales("en", None), &shortcut);
    let VoiceRoute::Prompt { origin, .. } = route else { panic!("not a prompt: {route:?}") };
    assert_that!(origin).is_equal_to(TurnOrigin::Wake);
}

#[test]
fn router_exits_once_stack_is_dropped() {
    let (tx, rx) = mpsc::channel();
    let handle = spawn(Weak::new(), rx).unwrap_or_else(|e| panic!("spawn: {e}"));
    tx.send(said("hello")).unwrap_or_else(|e| panic!("send: {e}"));
    let end = handle.join().unwrap_or_else(|_| panic!("router thread panicked"));
    assert_that!(end).is_equal_to(RouterEnd::Closed);
}

#[test]
fn router_without_sender_ends_disconnected() {
    let (tx, rx) = mpsc::channel::<Utterance>();
    drop(tx);
    assert_that!(run(&rx, |_| true)).is_equal_to(RouterEnd::Disconnected);
}

#[test]
fn router_panic_is_logged_and_thread_ends() {
    let (tx, rx) = mpsc::channel();
    tx.send(said("hello")).unwrap_or_else(|e| panic!("send: {e}"));
    let end = run(&rx, |u| {
        assert!(u.is_none(), "routing step failed on purpose");
        true
    });
    assert_that!(end).is_equal_to(RouterEnd::Panicked);
}
