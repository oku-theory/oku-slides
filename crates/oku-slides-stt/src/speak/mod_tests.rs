use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::SttErrorKind;

fn voice(id: &str, lang: &str) -> VoiceInfo {
    VoiceInfo { id: id.into(), name: id.into(), lang: lang.into() }
}

#[rstest]
#[case::nan(f32::NAN)]
#[case::infinite(f32::INFINITY)]
#[case::too_slow(0.49)]
#[case::too_fast(2.01)]
#[case::zero(0.0)]
#[case::negative(-1.0)]
fn speech_rate_nan_or_out_of_range_refused(#[case] r: f32) {
    let refused = SpeechRate::new(r).err().unwrap_or_else(|| panic!("rate {r} accepted"));
    assert_that!(refused.kind()).is_equal_to(SttErrorKind::VoiceFailed);
    assert_that!(refused.localize(&Localizer::new(Some("fr")))).contains("0,5");
}

#[rstest]
#[case::slowest(0.5)]
#[case::default(1.0)]
#[case::fastest(2.0)]
fn speech_rate_in_range_accepted(#[case] r: f32) {
    let rate = SpeechRate::new(r).unwrap_or_else(|e| panic!("rate {r} refused: {e}"));
    assert_that!(rate.get()).is_equal_to(r);
}

#[test]
fn voice_for_other_language_is_unavailable() {
    let voices = vec![voice("anna", "de-DE"), voice("samantha", "en-US")];
    assert_that!(for_language("fr", voices))
        .is_equal_to(SpeechStatus::Unavailable { reason: UnavailableReason::NoVoiceForLanguage });
}

#[test]
fn voice_for_language_keeps_only_that_language() {
    let voices = vec![voice("thomas", "fr-FR"), voice("samantha", "en-US"), voice("amelie", "fr_CA")];
    assert_that!(for_language("fr", voices))
        .is_equal_to(SpeechStatus::Ready { voices: vec![voice("thomas", "fr-FR"), voice("amelie", "fr_CA")] });
}

#[rstest]
#[case::region("fr-FR", "fr", true)]
#[case::underscore("pt_BR", "pt", true)]
#[case::case("EN-us", "en-GB", true)]
#[case::other("en-US", "fr", false)]
#[case::empty_lang("fr-FR", "", false)]
fn same_language_compares_primary_subtag(#[case] voice_lang: &str, #[case] lang: &str, #[case] same: bool) {
    assert_that!(same_language(voice_lang, lang)).is_equal_to(same);
}

#[test]
fn speak_error_drops_done_uncalled() {
    let calls = Arc::new(AtomicUsize::new(0));
    let witness = Arc::new(());
    let (seen, held) = (Arc::clone(&calls), Arc::clone(&witness));
    let done = SpeechDone::new(move |_| {
        drop(held);
        seen.fetch_add(1, Ordering::SeqCst);
    });
    let refused = NoSpeechEngine.speak(SpeechId::new(1), "bonjour", None, SpeechRate::default(), done);
    assert_that!(refused.map_err(|e| e.kind()))
        .is_equal_to(Err(SttErrorKind::VoiceUnavailable { reason: UnavailableReason::NoEngine }));
    assert_that!(calls.load(Ordering::SeqCst)).is_equal_to(0);
    assert_that!(Arc::strong_count(&witness)).is_equal_to(1);
}

#[rstest]
#[case::no_engine(UnavailableReason::NoEngine, "stt-voice-no-engine", "Aucune voix")]
#[case::no_voice(UnavailableReason::NoVoiceForLanguage, "stt-voice-no-voice", "langue")]
#[case::daemon(UnavailableReason::DaemonSilent, "stt-voice-daemon", "Speech Dispatcher")]
fn unavailable_reason_localized(#[case] reason: UnavailableReason, #[case] key: &str, #[case] fr: &str) {
    let en = Localizer::new(Some("en"));
    assert_that!(reason.localize(&en)).is_equal_to(en.tr(key));
    assert_that!(reason.localize(&en)).is_not_equal_to(key.to_owned());
    assert_that!(reason.localize(&Localizer::new(Some("fr")))).contains(fr);
}

#[test]
fn speech_done_debug_hides_closure() {
    assert_that!(format!("{:?}", SpeechDone::new(|_| {}))).is_equal_to("SpeechDone(..)".to_owned());
}

#[test]
fn voice_failed_localized() {
    let failed: SttError = crate::error::VoiceFailedSnafu { step: "speak" }.build().into();
    let en = Localizer::new(Some("en"));
    assert_that!(failed.kind()).is_equal_to(SttErrorKind::VoiceFailed);
    assert_that!(failed.localize(&en)).is_equal_to(en.tr("stt-voice-failed"));
    assert_that!(failed.localize(&Localizer::new(Some("fr")))).contains("La voix du système");
}

#[rstest]
#[case::empty(None, 2, None)]
#[case::older_current(Some((1, 10)), 2, Some(10))]
#[case::newer_current(Some((3, 30)), 2, Some(30))]
#[case::same_id(Some((2, 20)), 2, Some(20))]
fn install_returns_replaced_handle_and_last_wins(
    #[case] current: Option<(u64, u32)>,
    #[case] id: u64,
    #[case] replaced: Option<u32>,
) {
    let mut slot = current.map(|(n, h)| (SpeechId::new(n), h));
    let loser = install(&mut slot, SpeechId::new(id), 99);
    assert_that!(loser).is_equal_to(replaced);
    assert_that!(slot).is_equal_to(Some((SpeechId::new(id), 99)));
}
