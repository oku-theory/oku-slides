use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case::absent(None)]
#[case::empty(Some(""))]
#[case::whitespace_only(Some(" \n\t  "))]
#[case::controls_only(Some("\u{7}\u{1b}\u{202e}"))]
fn normalize_request_without_text_returns_none(#[case] request: Option<&str>) {
    assert_that!(normalize_request(request.map(str::to_owned))).is_none();
}

#[test]
fn normalize_request_trims_the_text() {
    assert_that!(normalize_request(Some("  Passe les titres en bleu.\n ".to_owned())))
        .is_equal_to(Some("Passe les titres en bleu.".to_owned()));
}

#[test]
fn normalize_request_keeps_inner_newlines_and_tabs() {
    assert_that!(normalize_request(Some("a\n\tb".to_owned()))).is_equal_to(Some("a\n\tb".to_owned()));
}

#[rstest]
#[case::bell('\u{7}')]
#[case::escape('\u{1b}')]
#[case::carriage_return('\r')]
#[case::delete('\u{7f}')]
#[case::c1('\u{85}')]
#[case::lre('\u{202a}')]
#[case::rlo('\u{202e}')]
#[case::lri('\u{2066}')]
#[case::pdi('\u{2069}')]
fn normalize_request_drops_control_and_bidi_characters(#[case] c: char) {
    assert_that!(normalize_request(Some(format!("a{c}b")))).is_equal_to(Some("ab".to_owned()));
}

#[test]
fn normalize_request_trims_whitespace_a_dropped_character_exposed() {
    assert_that!(normalize_request(Some("\u{202e} bleu".to_owned()))).is_equal_to(Some("bleu".to_owned()));
}

#[test]
fn fits_exactly_the_cap_returns_true() {
    assert_that!(fits("a".repeat(MAX_REQUEST_BYTES).as_str())).is_true();
}

#[test]
fn fits_one_byte_over_the_cap_returns_false() {
    assert_that!(fits("a".repeat(MAX_REQUEST_BYTES + 1).as_str())).is_false();
}

#[test]
fn fits_multibyte_text_counts_bytes_not_characters() {
    // 1025 characters, 2050 bytes: over the cap in bytes only.
    assert_that!(fits("é".repeat(1025).as_str())).is_false();
}

#[test]
fn no_drafts_prefill_returns_unavailable() {
    assert_that!(NoDrafts.prefill("t-2", "x".to_owned())).is_equal_to(Prefill::Unavailable);
}

#[rstest]
#[case::set(DraftOutcome::Set, "set")]
#[case::kept(DraftOutcome::Kept, "kept")]
#[case::too_long(DraftOutcome::TooLong, "too_long")]
#[case::unavailable(DraftOutcome::Unavailable, "unavailable")]
fn draft_outcome_as_str_returns_its_wire_name(#[case] outcome: DraftOutcome, #[case] name: &str) {
    assert_that!(outcome.as_str()).is_equal_to(name);
}
