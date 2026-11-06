//! The duplex audio: failure codes and the tap's attachment (hardware: `tests/duplex.rs`).

use std::sync::Arc;

use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::SttErrorKind;

#[rstest]
#[case::echo_cancel(raw::ECHO_CANCEL, "Impossible d'activer l'annulation d'écho pour ce micro")]
#[case::no_input(raw::NO_INPUT, "Aucun micro trouvé")]
#[case::engine_start(3, "Le moteur audio avec annulation d'écho ne démarre pas")]
#[case::unknown(42, "Le moteur audio avec annulation d'écho ne démarre pas")]
fn duplex_failure_code_maps_to_a_microphone_error(#[case] code: i32, #[case] french: &str) {
    let error = failure(code);
    assert_that!(error.kind()).is_equal_to(SttErrorKind::Microphone);
    assert_that!(error.localize(&Localizer::new(Some("fr")))).is_equal_to(french.to_owned());
}

#[test]
fn duplex_later_recorder_keeps_its_attachment_when_an_older_one_drops() {
    let tap = Arc::new(Tap::default());
    let (_, older) = tap.attach();
    let (newer, _kept) = tap.attach();
    drop(older);
    tap.push(&[0.5; 8], 16_000);
    assert_that!(newer.lock().map(|c| c.samples.len()).unwrap_or_default()).is_equal_to(8);
}

#[test]
fn duplex_available_only_on_macos() {
    assert_that!(Duplex::available()).is_equal_to(cfg!(target_os = "macos"));
}

#[test]
fn duplex_tap_level_follows_frames_without_recorder() {
    let tap = Tap::default();
    tap.push(&[0.1; 160], 16_000);
    assert_that!(tap.level()).is_close_to(0.4, 1e-6);
}
