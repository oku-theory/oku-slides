//! SSIP on one end of a Unix socket pair; the test plays Speech Dispatcher on the other.

use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use oku_slides_i18n::{Localize, Localizer};
use speculoos::prelude::*;

use super::*;
use crate::SttErrorKind;

/// The replies Speech Dispatcher gives to the `connect` handshake.
const HANDSHAKE: &str = "208 OK CLIENT NAME SET\r\n220 OK NOTIFICATION SET\r\n220 OK NOTIFICATION SET\r\n";

/// The replies to a `speak` that queues message 21.
const QUEUED_21: &str =
    "209 OK VOICE SET\r\n203 OK RATE SET\r\n230 OK RECEIVING DATA\r\n225-21\r\n225 OK MESSAGE QUEUED\r\n";

/// A connected pair: our end, and the daemon's end with `replies` already written to it.
fn pair(replies: &str) -> (UnixStream, UnixStream) {
    let (ours, daemon) = UnixStream::pair().unwrap_or_else(|e| panic!("socket pair: {e}"));
    (&daemon).write_all(replies.as_bytes()).unwrap_or_else(|e| panic!("replies: {e}"));
    for end in [&ours, &daemon] {
        end.set_read_timeout(Some(Duration::from_millis(100))).unwrap_or_else(|e| panic!("timeout: {e}"));
    }
    (ours, daemon)
}

/// Everything our end sent so far.
fn sent(mut daemon: &UnixStream) -> String {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        match daemon.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => bytes.extend_from_slice(chunk.get(..n).unwrap_or_default()),
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => break,
            Err(e) => panic!("read sent bytes: {e}"),
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|e| panic!("sent bytes not UTF-8: {e}"))
}

fn connected(replies: &str) -> (Ssip<UnixStream>, UnixStream) {
    let (ours, daemon) = pair(&format!("{HANDSHAKE}{replies}"));
    let ssip = Ssip::connect(ours, "joe").unwrap_or_else(|e| panic!("connect: {e}"));
    (ssip, daemon)
}

/// What `speak` sent after the handshake.
fn spoken(text: &str) -> String {
    let (mut ssip, daemon) = connected(QUEUED_21);
    ssip.speak(text, "english", 0).unwrap_or_else(|e| panic!("speak: {e}"));
    sent(&daemon).split_once("SPEAK\r\n").map(|(_, body)| body.to_owned()).unwrap_or_default()
}

#[test]
fn ssip_connect_sends_client_name_and_notifications() {
    let (_ssip, daemon) = connected("");
    assert_that!(sent(&daemon)).is_equal_to(
        "SET self CLIENT_NAME joe:oku-slides:voice\r\nSET self NOTIFICATION END on\r\nSET self NOTIFICATION CANCEL on\r\n"
            .to_owned(),
    );
}

#[rstest::rstest]
#[case::plain("joe", "joe")]
#[case::spaces_and_colon("jo e:x", "jo_e_x")]
#[case::dash_underscore("a-b_c", "a-b_c")]
#[case::non_ascii("zoé", "zo_")]
#[case::empty("", "unknown")]
fn ssip_client_name_keeps_only_allowed_characters(#[case] user: &str, #[case] part: &str) {
    let (ours, daemon) = pair(HANDSHAKE);
    Ssip::connect(ours, user).unwrap_or_else(|e| panic!("connect: {e}"));
    let first = sent(&daemon).lines().next().unwrap_or_default().to_owned();
    assert_that!(first).is_equal_to(format!("SET self CLIENT_NAME {part}:oku-slides:voice"));
}

#[test]
fn ssip_speak_sets_voice_and_rate_then_sends_text() {
    let (mut ssip, daemon) = connected(QUEUED_21);
    ssip.speak("bonjour", "French Male 1", -100).unwrap_or_else(|e| panic!("speak: {e}"));
    let after_handshake = sent(&daemon).split_once("CANCEL on\r\n").map(|(_, rest)| rest.to_owned());
    assert_that!(after_handshake).is_equal_to(Some(
        "SET self SYNTHESIS_VOICE French Male 1\r\nSET self RATE -100\r\nSPEAK\r\nbonjour\r\n.\r\n".to_owned(),
    ));
}

#[test]
fn ssip_speak_returns_message_id_of_225_reply() {
    let (mut ssip, _daemon) = connected(QUEUED_21);
    assert_that!(ssip.speak("bonjour", "english", 0).ok()).is_equal_to(Some(21));
}

#[test]
fn ssip_speak_reply_without_message_id_is_an_error() {
    let replies = "209 OK VOICE SET\r\n203 OK RATE SET\r\n230 OK RECEIVING DATA\r\n227 OK MESSAGE CANCELED\r\n";
    let (mut ssip, _daemon) = connected(replies);
    assert_that!(ssip.speak("x", "english", 0).map_err(|e| e.kind())).is_equal_to(Err(SttErrorKind::VoiceFailed));
}

#[rstest::rstest]
#[case::dot_stuffed(".start\n.\nmid", "..start\r\n..\r\nmid\r\n.\r\n")]
#[case::crlf_kept("a\r\nb", "a\r\nb\r\n.\r\n")]
#[case::lone_cr_is_a_line_end("a\rb", "a\r\nb\r\n.\r\n")]
#[case::nul_is_space("un\0deux", "un deux\r\n.\r\n")]
#[case::dot_inside_line_kept("a.b\n.c", "a.b\r\n..c\r\n.\r\n")]
#[case::terminator_in_text_is_escaped("x\r\n.\r\ny", "x\r\n..\r\ny\r\n.\r\n")]
fn ssip_speak_body_uses_crlf_lines_and_dot_stuffing(#[case] text: &str, #[case] body: &str) {
    assert_that!(spoken(text)).is_equal_to(body.to_owned());
}

#[test]
fn ssip_refused_reply_is_voice_failed_localized() {
    let (mut ssip, _daemon) = connected("209 OK VOICE SET\r\n410 ERR RATE TOO HIGH\r\n");
    let refused = ssip.speak("bonjour", "english", 0).err().unwrap_or_else(|| panic!("a refused rate spoke"));
    assert_that!(refused.kind()).is_equal_to(SttErrorKind::VoiceFailed);
    let en = Localizer::new(Some("en"));
    assert_that!(refused.localize(&en)).is_equal_to(en.tr("stt-voice-failed"));
    let fr = Localizer::new(Some("fr"));
    assert_that!(refused.localize(&fr)).is_equal_to(fr.tr("stt-voice-failed"));
}

#[test]
fn ssip_voice_with_control_character_is_refused_unsent() {
    let (mut ssip, daemon) = connected("");
    let refused = ssip.speak("bonjour", "english\r\nSPEAK", 0).map_err(|e| e.kind());
    assert_that!(refused).is_equal_to(Err(SttErrorKind::VoiceFailed));
    assert_that!(sent(&daemon).contains("SYNTHESIS_VOICE")).is_false();
}

#[test]
fn ssip_silent_daemon_is_an_error() {
    let (ours, _daemon) = pair("");
    assert_that!(Ssip::connect(ours, "joe").map(|_| ()).map_err(|e| e.kind()))
        .is_equal_to(Err(SttErrorKind::VoiceFailed));
}

#[test]
fn ssip_malformed_reply_is_an_error() {
    let (ours, _daemon) = pair("hello\r\n");
    assert_that!(Ssip::connect(ours, "joe").map(|_| ()).map_err(|e| e.kind()))
        .is_equal_to(Err(SttErrorKind::VoiceFailed));
}

#[rstest::rstest]
#[case::begin("701-21\r\n701-3\r\n701 BEGIN\r\n", EventKind::Begin)]
#[case::end("702-21\r\n702-3\r\n702 END\r\n", EventKind::End)]
#[case::cancel("703-21\r\n703-3\r\n703 CANCELED\r\n", EventKind::Cancel)]
fn ssip_read_event_parses_notification_for_message(#[case] notification: &str, #[case] kind: EventKind) {
    let (mut ssip, _daemon) = connected(notification);
    assert_that!(ssip.read_event().ok()).is_equal_to(Some(Some(Event { msg: 21, kind })));
}

#[test]
fn ssip_read_event_skips_command_replies_and_other_events() {
    let replies = "213 OK CANCELED\r\n704-21\r\n704-3\r\n704 PAUSED\r\n703-21\r\n703-3\r\n703 CANCELED\r\n";
    let (mut ssip, _daemon) = connected(replies);
    assert_that!(ssip.read_event().ok()).is_equal_to(Some(Some(Event { msg: 21, kind: EventKind::Cancel })));
}

#[test]
fn ssip_read_event_keeps_partial_notification_across_timeouts() {
    let (mut ssip, daemon) = connected("702-21\r\n702-");
    assert_that!(ssip.read_event().ok()).is_equal_to(Some(None));
    (&daemon).write_all(b"3\r\n702 END\r\n").unwrap_or_else(|e| panic!("rest: {e}"));
    assert_that!(ssip.read_event().ok()).is_equal_to(Some(Some(Event { msg: 21, kind: EventKind::End })));
}

#[test]
fn ssip_notification_before_a_reply_is_kept_for_read_event() {
    let (mut ssip, _daemon) = connected(&format!("702-20\r\n702-3\r\n702 END\r\n{QUEUED_21}"));
    assert_that!(ssip.speak("x", "english", 0).ok()).is_equal_to(Some(21));
    assert_that!(ssip.read_event().ok()).is_equal_to(Some(Some(Event { msg: 20, kind: EventKind::End })));
}

#[test]
fn ssip_closed_connection_is_an_error() {
    let (mut ssip, daemon) = connected("");
    drop(daemon);
    assert_that!(ssip.read_event().map_err(|e| e.kind())).is_equal_to(Err(SttErrorKind::VoiceFailed));
}

#[test]
fn ssip_cancel_sends_cancel_self() {
    let (ours, daemon) = pair("");
    cancel(&ours).unwrap_or_else(|e| panic!("cancel: {e}"));
    assert_that!(sent(&daemon)).is_equal_to("CANCEL self\r\n".to_owned());
}

#[test]
fn ssip_quit_sends_quit() {
    let (mut ssip, daemon) = connected("231 HAPPY HACKING\r\n");
    ssip.quit();
    assert_that!(sent(&daemon).ends_with("CANCEL on\r\nQUIT\r\n")).is_true();
}

#[test]
fn ssip_voices_parses_tab_separated_list() {
    let list = "249-english\ten\tnone\r\n249-French (France)\tfr-FR\tnone\r\n249 OK VOICE LIST SENT\r\n";
    let (mut ssip, daemon) = connected(list);
    let voices = ssip.voices().unwrap_or_else(|e| panic!("voices: {e}"));
    let voice = |name: &str, lang: &str| VoiceInfo { id: name.into(), name: name.into(), lang: lang.into() };
    assert_that!(voices).is_equal_to(vec![voice("english", "en"), voice("French (France)", "fr-FR")]);
    assert_that!(sent(&daemon).ends_with("CANCEL on\r\nLIST SYNTHESIS_VOICES\r\n")).is_true();
}
