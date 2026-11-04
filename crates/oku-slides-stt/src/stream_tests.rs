use speculoos::prelude::*;

use super::*;
use crate::SttErrorKind;

#[test]
fn from_raw_partial_keeps_text() {
    let event = Event::from_raw(raw::PARTIAL, "bonj".into(), 0.0, 0.0);
    assert_that!(matches!(event, Event::Partial(t) if t == "bonj")).is_true();
}

#[test]
fn from_raw_final_carries_times() {
    let event = Event::from_raw(raw::FINAL, "Bonjour.".into(), 1.5, 2.5);
    let parts = match event {
        Event::Final { text, start, end } => Some((text, start, end)),
        _ => None,
    };
    assert_that!(parts).is_equal_to(Some(("Bonjour.".to_owned(), 1.5, 2.5)));
}

#[test]
fn from_raw_progress_carries_counts() {
    let event = Event::from_raw(raw::PROGRESS, String::new(), 10.0, 40.0);
    assert_that!(matches!(event, Event::Progress { received: 10, total: 40 })).is_true();
}

#[test]
fn from_raw_done_ends_the_stream() {
    assert_that!(matches!(Event::from_raw(raw::DONE, String::new(), 0.0, 0.0), Event::Done)).is_true();
}

#[test]
fn from_raw_error_codes_map_to_error_kinds() {
    let cases = [
        (raw::ERR_ANALYZER, SttErrorKind::Transcription),
        (raw::ERR_LOCALE, SttErrorKind::Model),
        (raw::ERR_ASSET, SttErrorKind::Download),
        (raw::ERR_DENIED, SttErrorKind::Permission),
        (99.0, SttErrorKind::Transcription),
    ];
    for (code, kind) in cases {
        let Event::Failed(e) = Event::from_raw(raw::ERROR, "x".into(), code, 0.0) else {
            panic!("code {code} : pas une erreur");
        };
        asserting(&format!("code {code}")).that(&e.kind()).is_equal_to(kind);
    }
}

#[test]
fn from_raw_unknown_kind_is_an_analyzer_error_naming_it() {
    let Event::Failed(e) = Event::from_raw(42, "x".into(), 0.0, 0.0) else {
        panic!("type inconnu : pas une erreur");
    };
    assert_that!(e.kind()).is_equal_to(SttErrorKind::Transcription);
    assert_that!(e.to_string()).contains("42");
}

#[test]
fn from_raw_unknown_error_code_is_named() {
    let Event::Failed(e) = Event::from_raw(raw::ERROR, "x".into(), 99.0, 0.0) else {
        panic!("code inconnu : pas une erreur");
    };
    assert_that!(e.to_string()).contains("99");
}

#[test]
fn no_stream_engine_supports_nothing() {
    assert_that!(NoStreamEngine.supported()).is_false();
    assert_that!(NoStreamEngine.availability("fr")).is_equal_to(Availability::Unsupported);
}

#[test]
fn no_stream_engine_open_reports_unsupported_language() {
    let (_stream, events) = NoStreamEngine.open("fr", &[]);
    let event = events.recv().unwrap();
    assert_that!(matches!(event, Event::Failed(e) if e.kind() == SttErrorKind::Model)).is_true();
}

/// The Swift bridge, read at build time: its codes must equal `raw` (G16).
const SWIFT: &str = include_str!("../swift/Sources/OkuSpeech/OkuSpeech.swift");

#[rstest::rstest]
#[case::partial(format!("private let kindPartial: Int32 = {}", raw::PARTIAL))]
#[case::final_text(format!("private let kindFinal: Int32 = {}", raw::FINAL))]
#[case::progress(format!("private let kindProgress: Int32 = {}", raw::PROGRESS))]
#[case::done(format!("private let kindDone: Int32 = {}", raw::DONE))]
#[case::error(format!("private let kindError: Int32 = {}", raw::ERROR))]
#[case::analyzer(format!("private let errAnalyzer = {:?}", raw::ERR_ANALYZER))]
#[case::locale(format!("private let errLocale = {:?}", raw::ERR_LOCALE))]
#[case::asset(format!("private let errAsset = {:?}", raw::ERR_ASSET))]
#[case::denied(format!("private let errDenied = {:?}", raw::ERR_DENIED))]
fn swift_error_codes_match_stream_raw(#[case] line: String) {
    assert_that!(SWIFT.lines().any(|l| l.trim() == line)).is_true();
}

#[test]
fn swift_error_codes_match_stream_raw_no_extra_code() {
    let declared =
        SWIFT.lines().filter(|l| l.starts_with("private let kind") || l.starts_with("private let err")).count();
    assert_that!(declared).is_equal_to(9);
}
