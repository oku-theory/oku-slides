//! The host voice router over the stack: utterances cancel, prompt, or go nowhere.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
mod common;

use common::{Relayed, cmd, is_tool_surface, logged, stack_with_sink_and_log, utterance, wait_for, wait_turn_done};
use oku_slides_agent::{ReportFrom, RouterEnd};
use oku_slides_session::agent::TurnOrigin;
use serde_json::json;
use speculoos::prelude::*;

/// Turn `n` of `tab` shows its surface: the turn runs.
async fn wait_turn_shown(rx: &Relayed, tab: &str, n: u32) {
    let id = format!("turn-{n}");
    wait_for(rx, |(ch, v)| {
        *ch == "mcp" && v["tab"] == tab && v["event"]["kind"] == "surface" && v["event"]["surface"]["id"] == id.as_str()
    })
    .await;
}

/// The texts of every `session/prompt` the agent got, flattened.
fn prompts(log: &std::path::Path) -> Vec<String> {
    logged(log, "session/prompt").iter().map(|p| p["prompt"].to_string()).collect()
}

fn ask() -> serde_json::Value {
    json!({ "tool": "ask_choice", "args": { "question": "Laquelle ?", "options": [
        { "id": "a", "label": "A" }, { "id": "b", "label": "B" } ] } })
}

#[tokio::test(flavor = "multi_thread")]
async fn voice_utterance_during_turn_cancels_and_replaces_it() {
    let script = json!({ "turns": [{ "wait_cancel": true }, {}] });
    let (stack, rx, log) = stack_with_sink_and_log(script, &["t-1"]).await;
    stack.workspace.focus_tab("t-1");
    let voice = stack.voice();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "un" }))).unwrap();
    wait_turn_shown(&rx, "t-1", 1).await;
    voice.send(utterance(1, "mets le titre en rouge")).unwrap();
    wait_turn_done(&rx, "t-1", 2).await;
    let sent = prompts(&log);
    assert_that!((sent.len(), sent[1].contains("mets le titre en rouge"), logged(&log, "session/cancel").len()))
        .is_equal_to((2, true, 1));
}

/// Filler-only utterances ("euh") are noise: they never become a prompt, so never cut a turn.
#[tokio::test(flavor = "multi_thread")]
async fn filler_only_utterance_never_prompts() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}, {}, {}] }), &["t-1"]).await;
    stack.workspace.focus_tab("t-1");
    let voice = stack.voice();
    voice.send(utterance(1, "euh")).unwrap();
    voice.send(utterance(2, "euh, alors…")).unwrap();
    // Sentinel: the first prompt the agent gets is the real instruction, routed after the fillers.
    voice.send(utterance(3, "mets le titre en rouge")).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let sent = prompts(&log);
    assert_that!((sent.len(), sent[0].contains("mets le titre en rouge"))).is_equal_to((1, true));
}

/// A voice that never ends its speech: the read-aloud keeps speaking.
#[derive(Debug)]
struct EndlessVoice;

impl oku_slides_stt::SpeechEngine for EndlessVoice {
    fn status(&self, lang: &str) -> oku_slides_stt::SpeechStatus {
        let voice = oku_slides_stt::VoiceInfo { id: "v".into(), name: "V".into(), lang: lang.into() };
        oku_slides_stt::SpeechStatus::Ready { voices: vec![voice] }
    }

    fn speak(
        &self,
        _id: oku_slides_stt::SpeechId,
        _text: &str,
        _voice: Option<&str>,
        _rate: oku_slides_stt::SpeechRate,
        _done: oku_slides_stt::SpeechDone,
    ) -> Result<(), oku_slides_stt::SttError> {
        Ok(())
    }

    fn stop(&self, _id: oku_slides_stt::SpeechId) -> Result<(), oku_slides_stt::SttError> {
        Ok(())
    }
}

/// What the microphone heard of the read-aloud never reaches the agent (`Speaker::hears`).
#[tokio::test(flavor = "multi_thread")]
async fn utterance_begun_while_read_aloud_speaks_is_dropped() {
    let speaker = oku_slides_stt::Speaker::new(std::sync::Arc::new(EndlessVoice), None);
    let (stack, rx, log) = common::stack_with_speaker(json!({ "turns": [{}, {}] }), &["t-1"], speaker.clone()).await;
    stack.workspace.focus_tab("t-1");
    let voice = stack.voice();
    let before = utterance(1, "ajoute une diapo");
    speaker.say(oku_slides_stt::Speech {
        text: "Diapositive ajoutée".into(),
        lang: "fr".into(),
        voice: None,
        rate: oku_slides_stt::SpeechRate::default(),
        ended: None,
    });
    assert_that!(speaker.settle(std::time::Duration::from_secs(10))).is_true();
    voice.send(utterance(2, "diapositive ajoutée")).unwrap();
    // Sentinel: begun before the speech, so heard; the echo before it never prompts.
    voice.send(before).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let sent = prompts(&log);
    assert_that!((sent.len(), sent[0].contains("ajoute une diapo"))).is_equal_to((1, true));
}

#[tokio::test(flavor = "multi_thread")]
async fn stop_cancels_without_new_turn() {
    let script = json!({ "turns": [{ "wait_cancel": true }, {}] });
    let (stack, rx, log) = stack_with_sink_and_log(script, &["t-1"]).await;
    stack.workspace.focus_tab("t-1");
    let voice = stack.voice();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "un" }))).unwrap();
    wait_turn_shown(&rx, "t-1", 1).await;
    voice.send(utterance(1, "stop")).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    // Witness: the next typed prompt is the second one the agent gets, the stop word never one.
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "salut" }))).unwrap();
    wait_turn_done(&rx, "t-1", 2).await;
    let sent = prompts(&log);
    assert_that!((sent.len(), sent[1].contains("salut"), logged(&log, "session/cancel").len()))
        .is_equal_to((2, true, 1));
}

#[tokio::test(flavor = "multi_thread")]
async fn context_during_turn_does_not_cancel_it() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "mcp": [ask()] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "demande-moi" }))).unwrap();
    let asked = wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    let report = json!({ "cmd": "context", "tab": "t-1", "page": "p-1", "seq": 1, "has_focus": true,
        "ui": { "slide": "s-1" } });
    stack
        .handle_from(cmd(report), &ReportFrom::Holder { window: "w-1".into() })
        .unwrap_or_else(|e| panic!("presenting report refused: {e}"));
    let id = asked.1["event"]["surface"]["id"].as_str().unwrap().to_owned();
    let answer =
        json!({ "cmd": "surface_action", "tab": "t-1", "surface": id, "name": "choose", "context": { "choice": "a" } });
    stack.handle(cmd(answer)).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    assert_that!((logged(&log, "session/cancel").len(), prompts(&log).len())).is_equal_to((0, 1));
}

#[tokio::test(flavor = "multi_thread")]
async fn context_from_another_window_is_refused() {
    let (stack, _rx, _log) = stack_with_sink_and_log(json!({ "turns": [] }), &["t-1"]).await;
    let report = json!({ "cmd": "context", "tab": "t-1", "page": "p-1", "seq": 1 });
    assert_that!(stack.handle(cmd(report))).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn utterance_goes_to_focused_window_active_tab() {
    let (stack, rx, _log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1", "t-2"]).await;
    stack.workspace.show("w-1", "t-1");
    stack.workspace.show("w-2", "t-2");
    stack.workspace.focus("w-1");
    stack.workspace.focus("w-2");
    let voice = stack.voice();
    voice.send(utterance(1, "ajoute une diapo")).unwrap();
    wait_turn_done(&rx, "t-2", 1).await;
    let t1_turns =
        rx.lock().unwrap().iter().filter(|(_, v)| v["tab"] == "t-1" && v["event"]["kind"] == "surface").count();
    assert_that!(t1_turns).is_equal_to(0);
}

#[tokio::test(flavor = "multi_thread")]
async fn two_utterances_between_reads_both_prompt() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}, {}] }), &["t-1"]).await;
    stack.workspace.focus_tab("t-1");
    let voice = stack.voice();
    voice.send(utterance(1, "premier ordre")).unwrap();
    voice.send(utterance(2, "second ordre")).unwrap();
    wait_turn_done(&rx, "t-1", 2).await;
    let sent = prompts(&log);
    assert_that!((sent.len(), sent[0].contains("premier ordre"), sent[1].contains("second ordre")))
        .is_equal_to((2, true, true));
}

#[tokio::test(flavor = "multi_thread")]
async fn wake_turn_is_marked_wake() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}, {}] }), &["t-1"]).await;
    stack.workspace.focus_tab("t-1");
    let voice = stack.voice();
    voice.send(utterance(1, "ajoute une diapo")).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    assert_that!(stack.turn_origin("t-1", 1)).is_equal_to(Some(TurnOrigin::Voice));
    // In the background, the wake phrase still reaches the last focused tab.
    stack.workspace.background();
    voice.send(utterance(2, "hé oku ajoute un titre")).unwrap();
    wait_turn_done(&rx, "t-1", 2).await;
    assert_that!(stack.turn_origin("t-1", 2)).is_equal_to(Some(TurnOrigin::Wake));
    let sent = prompts(&log);
    assert_that!((sent[1].contains("ajoute un titre"), sent[1].contains("hé oku"))).is_equal_to((true, false));
}

#[tokio::test(flavor = "multi_thread")]
async fn background_wake_goes_to_last_focused_tab_never_spoken_to() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}, {}] }), &["t-1", "t-2"]).await;
    stack.workspace.show("w-1", "t-1");
    stack.workspace.show("w-2", "t-2");
    stack.workspace.focus("w-1");
    let voice = stack.voice();
    voice.send(utterance(1, "ajoute une diapo")).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    // The user moves to w-2 without speaking, then leaves the app.
    stack.workspace.focus("w-2");
    stack.workspace.background();
    voice.send(utterance(2, "hé oku ajoute un titre")).unwrap();
    wait_turn_done(&rx, "t-2", 1).await;
    assert_that!(stack.turn_origin("t-2", 1)).is_equal_to(Some(TurnOrigin::Wake));
    assert_that!(prompts(&log).len()).is_equal_to(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn background_stop_cancels_last_focused_tab_never_spoken_to() {
    let (stack, rx, log) =
        stack_with_sink_and_log(json!({ "turns": [{ "wait_cancel": true }] }), &["t-1", "t-2"]).await;
    stack.workspace.show("w-1", "t-1");
    stack.workspace.show("w-2", "t-2");
    stack.workspace.focus("w-1");
    let voice = stack.voice();
    voice.send(utterance(1, "ajoute une diapo")).unwrap();
    wait_turn_shown(&rx, "t-1", 1).await;
    stack.workspace.focus("w-2");
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-2", "text": "deux" }))).unwrap();
    wait_turn_shown(&rx, "t-2", 1).await;
    stack.workspace.background();
    voice.send(utterance(2, "stop")).unwrap();
    wait_turn_done(&rx, "t-2", 1).await;
    // Only t-2's turn was cancelled: t-1's still runs.
    assert_that!(logged(&log, "session/cancel").len()).is_equal_to(1);
    let t1_done = rx.lock().unwrap().iter().any(|(_, v)| {
        v["tab"] == "t-1"
            && v["event"]["kind"] == "surface_patch"
            && v["event"]["messages"].to_string().contains(r#""path":"/running","value":false"#)
    });
    assert_that!(t1_done).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn background_wake_before_any_utterance_prompts_left_tab() {
    let (stack, rx, _log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    stack.workspace.focus_tab("t-1");
    stack.workspace.background();
    let voice = stack.voice();
    voice.send(utterance(1, "hé oku ajoute un titre")).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    assert_that!(stack.turn_origin("t-1", 1)).is_equal_to(Some(TurnOrigin::Wake));
}

#[tokio::test(flavor = "multi_thread")]
async fn router_ends_closed_once_a_live_stack_shuts_down() {
    let (stack, _rx, _log) = stack_with_sink_and_log(json!({ "turns": [] }), &["t-1"]).await;
    let (voice, router) = stack.voice_router();
    let router = router.unwrap_or_else(|| panic!("router not started"));
    // The stack stays alive (`stack` holds it): only its closing flag ends the router.
    stack.shutdown().await;
    let _ = voice.send(utterance(1, "ajoute une diapo"));
    let end =
        tokio::time::timeout(std::time::Duration::from_secs(10), tokio::task::spawn_blocking(move || router.join()))
            .await
            .unwrap_or_else(|_| panic!("router still running"))
            .unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_else(|_| panic!("router panicked"));
    assert_that!(end).is_equal_to(RouterEnd::Closed);
}

/// A background utterance that is dropped never starts a dormant tab's entry.
#[tokio::test(flavor = "multi_thread")]
async fn dropped_background_utterance_never_starts_a_dormant_tab() {
    let (stack, _rx, _log) = stack_with_sink_and_log(json!({ "turns": [] }), &["t-1"]).await;
    stack.workspace.focus_tab("t-1");
    stack.workspace.background();
    let (voice, router) = stack.voice_router();
    let router = router.unwrap_or_else(|| panic!("router not started"));
    voice.send(utterance(1, "mets le titre en rouge")).unwrap();
    drop(voice);
    // Sentinel: the router ends once it routed every utterance sent before.
    let end =
        tokio::time::timeout(std::time::Duration::from_secs(10), tokio::task::spawn_blocking(move || router.join()))
            .await
            .unwrap_or_else(|_| panic!("router still running"))
            .unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_else(|_| panic!("router panicked"));
    assert_that!(end).is_equal_to(RouterEnd::Disconnected);
    // A tab without an entry refuses a draft: the dropped utterance started none.
    let draft = stack.handle(cmd(json!({ "cmd": "set_draft", "tab": "t-1", "text": "x", "origin": "v-1" })));
    assert_that!(draft).is_err();
}

/// `text`, begun once a question shown now is armed.
fn armed(seq: u64, text: &str) -> oku_slides_stt::Utterance {
    let mut u = utterance(seq, text);
    u.started += oku_slides_mcp::ARMING + std::time::Duration::from_millis(10);
    u
}

/// t-997: from a host-focused window, "yes" and the verb confirm a danger tool: no new prompt.
#[tokio::test(flavor = "multi_thread")]
async fn voice_yes_with_verb_confirms_a_danger_tool() {
    let merge = json!({ "tool": "merge", "args": { "path": "/nowhere/copie.oku-slides.pdf" } });
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "mcp": [merge] }] }), &["t-1"]).await;
    stack.workspace.focus_tab("t-1");
    let voice = stack.voice();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "fusionne" }))).unwrap();
    wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "confirm-")).await;
    voice.send(armed(1, "oui, fusionner")).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let outs: Vec<String> = logged(&log, "mcp/result").iter().map(|r| r["out"].to_string()).collect();
    // Confirmed: the merge ran and failed on its missing file, it was not refused.
    let ran = oku_slides_i18n::Localizer::new(Some("fr")).tr("agent-error-file");
    assert_that!((outs.len(), outs[0].contains(ran.as_str()))).is_equal_to((1, true));
    assert_that!(prompts(&log)).has_length(1);
}

/// t-997: a bare yes to a danger confirmation keeps it open with the heard line, no prompt.
#[tokio::test(flavor = "multi_thread")]
async fn voice_yes_without_verb_keeps_the_danger_tool_waiting() {
    let merge = json!({ "tool": "merge", "args": { "path": "/nowhere/copie.oku-slides.pdf" } });
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{ "mcp": [merge] }] }), &["t-1"]).await;
    stack.workspace.focus_tab("t-1");
    let voice = stack.voice();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "fusionne" }))).unwrap();
    wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "confirm-")).await;
    voice.send(armed(1, "oui")).unwrap();
    // The heard line is the witness that the utterance was handled.
    wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "confirm-") && v.to_string().contains("Entendu : « oui »")).await;
    assert_that!(logged(&log, "mcp/result")).is_empty();
    assert_that!(prompts(&log)).has_length(1);
    assert_that!(logged(&log, "session/cancel")).is_empty();
}

/// Window `w-1`'s page reports that `t-1` presents in the window, from page `p-1`.
fn presents(stack: &common::TestStack) {
    let report = json!({ "cmd": "presenting", "tab": "t-1", "page": "p-1", "seq": 1, "mode": "window" });
    stack
        .handle_from(cmd(report), &ReportFrom::Holder { window: "w-1".into() })
        .unwrap_or_else(|e| panic!("presenting report refused: {e}"));
}

/// The presentation commands the page got for `t-1`, in order.
fn present_commands(rx: &Relayed) -> Vec<serde_json::Value> {
    let relayed = rx.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    relayed
        .iter()
        .filter(|(ch, v)| *ch == "mcp" && v["tab"] == "t-1" && v["event"]["kind"] == "present")
        .map(|(_, v)| v["event"]["command"].clone())
        .collect()
}

/// t-998: while presenting, a nav word goes to the page at once; other speech is dropped,
/// and only the wake phrase reaches the agent.
#[tokio::test(flavor = "multi_thread")]
async fn presenting_voice_nav_goes_to_the_page_and_other_speech_needs_the_wake_phrase() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    stack.workspace.focus_tab("t-1");
    presents(&stack);
    let voice = stack.voice();
    voice.send(utterance(1, "suivant")).unwrap();
    voice.send(utterance(2, "bonjour à tous")).unwrap();
    voice.send(utterance(3, "oku quelle heure est-il")).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    assert_that!(present_commands(&rx)).is_equal_to(vec![json!({ "action": "next" })]);
    let sent = prompts(&log);
    assert_that!(sent).has_length(1);
    assert_that!((sent[0].contains("quelle heure est-il"), sent[0].contains("bonjour"))).is_equal_to((true, false));
    assert_that!(stack.turn_origin("t-1", 1)).is_equal_to(Some(TurnOrigin::Wake));
}

/// While presenting, "quitter" leaves the presentation.
#[tokio::test(flavor = "multi_thread")]
async fn presenting_voice_quit_exits() {
    let (stack, rx, _log) = stack_with_sink_and_log(json!({ "turns": [] }), &["t-1"]).await;
    stack.workspace.focus_tab("t-1");
    presents(&stack);
    stack.voice().send(utterance(1, "quitter")).unwrap();
    wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"]["kind"] == "present").await;
    assert_that!(present_commands(&rx)).is_equal_to(vec![json!({ "action": "exit" })]);
}

/// Once the page stops presenting (window destroyed, new page id), a nav word is a prompt.
#[rstest::rstest]
#[case::window_destroyed(true)]
#[case::new_page_id(false)]
#[tokio::test(flavor = "multi_thread")]
async fn presenting_cleared_nav_word_is_a_prompt(#[case] destroyed: bool) {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    stack.workspace.focus_tab("t-1");
    presents(&stack);
    if destroyed {
        stack.window_destroyed("w-1");
    } else {
        let report = json!({ "cmd": "context", "tab": "t-1", "page": "p-2", "seq": 1, "has_focus": true });
        stack
            .handle_from(cmd(report), &ReportFrom::Holder { window: "w-1".into() })
            .unwrap_or_else(|e| panic!("presenting report refused: {e}"));
    }
    stack.voice().send(utterance(1, "suivant")).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    assert_that!(present_commands(&rx)).is_empty();
    assert_that!(prompts(&log)[0].contains("suivant")).is_true();
}
