use rstest::rstest;
use speculoos::prelude::*;

use super::{AVAILABILITY, ERRORS, c_text, kind_of, model_of};
use crate::{AppleModel, ErrorKind};

const SWIFT: &str = include_str!("../swift/Sources/OkuModel/OkuModel.swift");

/// The value of `private let <name>: Int32 = <value>` in the Swift bridge.
fn swift_code(name: &str) -> i32 {
    let head = format!("private let {name}: Int32 = ");
    let line = SWIFT.lines().find_map(|l| l.strip_prefix(head.as_str())).unwrap();
    line.trim().parse().unwrap()
}

#[rstest]
#[case::ready("availReady", AppleModel::Ready)]
#[case::device("availDeviceNotEligible", AppleModel::DeviceNotEligible)]
#[case::off("availIntelligenceOff", AppleModel::IntelligenceOff)]
#[case::not_ready("availModelNotReady", AppleModel::ModelNotReady)]
#[case::too_old("availMacOsTooOld", AppleModel::MacOsTooOld)]
#[case::unknown("availUnknown", AppleModel::Unknown)]
fn availability_swift_code_maps_to_state(#[case] swift: &str, #[case] model: AppleModel) {
    assert_that!(model_of(swift_code(swift))).is_equal_to(model);
}

#[rstest]
#[case::overflow("errContextOverflow", ErrorKind::ContextOverflow)]
#[case::refused("errRefused", ErrorKind::Refused)]
#[case::failed("errFailed", ErrorKind::Failed)]
#[case::cancelled("errCancelled", ErrorKind::Cancelled)]
fn error_swift_code_maps_to_kind(#[case] swift: &str, #[case] kind: ErrorKind) {
    assert_that!(kind_of(swift_code(swift))).is_equal_to(kind);
}

#[test]
fn availability_table_each_state_has_one_code() {
    for (code, model) in AVAILABILITY {
        assert_that!(AVAILABILITY.iter().filter(|(_, m)| *m == model).count()).is_equal_to(1);
        assert_that!(model_of(code)).is_equal_to(model);
    }
}

#[test]
fn error_table_each_kind_has_one_code() {
    for (code, kind) in ERRORS {
        assert_that!(ERRORS.iter().filter(|(_, k)| *k == kind).count()).is_equal_to(1);
        assert_that!(kind_of(code)).is_equal_to(kind);
    }
}

#[rstest]
#[case::negative(-1)]
#[case::past_end(99)]
fn model_of_unknown_code_is_unknown(#[case] code: i32) {
    assert_that!(model_of(code)).is_equal_to(AppleModel::Unknown);
}

#[rstest]
#[case::zero(0)]
#[case::past_end(99)]
fn kind_of_unknown_code_is_failed(#[case] code: i32) {
    assert_that!(kind_of(code)).is_equal_to(ErrorKind::Failed);
}

#[test]
fn c_text_interior_nul_becomes_a_replacement_character() {
    let text = c_text("before\0after").unwrap();
    assert_that!(text.to_str().unwrap()).is_equal_to("before\u{FFFD}after");
}

#[test]
fn c_text_plain_text_is_unchanged() {
    assert_that!(c_text("plain é").unwrap().to_str().unwrap()).is_equal_to("plain é");
}

#[cfg(target_os = "macos")]
mod real {
    use std::time::{Duration, Instant};

    use speculoos::prelude::*;
    use tokio_util::sync::CancellationToken;

    use crate::{AppleModel, ErrorKind, Request, availability, respond};

    const SCHEMA: &str = r#"{"title":"Reply","type":"object","additionalProperties":false,
        "properties":{"tool":{"type":"string","enum":["set_setting","report"]},
        "arguments":{"title":"Arguments","type":"object","additionalProperties":false,
        "properties":{"key":{"type":"string"},"value":{"type":"integer"}},
        "required":["key","value"],"x-order":["key","value"]}},
        "required":["tool","arguments"],"x-order":["tool","arguments"]}"#;

    fn request(schema: &str) -> Request<'_> {
        Request {
            instructions: "You change Oku Slides settings. history.limit is the number of kept versions.",
            prompt: "Keep 30 versions in history.",
            schema,
            temperature: 0.2,
            max_tokens: 200,
        }
    }

    #[test]
    #[ignore = "manual: needs macOS 26+ with Apple Intelligence on"]
    fn respond_real_model_follows_schema() {
        assert_that!(availability()).is_equal_to(AppleModel::Ready);
        let started = Instant::now();
        let json = respond(&request(SCHEMA), &CancellationToken::new()).unwrap();
        eprintln!("{:?} {json}", started.elapsed());
        assert_that!(json.as_str()).contains("set_setting");
    }

    #[test]
    #[ignore = "manual: needs macOS 26+ with Apple Intelligence on"]
    fn respond_real_model_cancelled_returns_cancelled() {
        let cancel = CancellationToken::new();
        let late = cancel.clone();
        let timer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            late.cancel();
        });
        let started = Instant::now();
        let err = respond(&request(SCHEMA), &cancel).unwrap_err();
        timer.join().unwrap();
        assert_that!(err.kind()).is_equal_to(ErrorKind::Cancelled);
        assert_that!(started.elapsed()).is_less_than(Duration::from_secs(1));
    }

    #[test]
    #[ignore = "manual: needs macOS 26+ with Apple Intelligence on"]
    fn respond_invalid_schema_is_failed() {
        let err = respond(&request("{\"type\": 3}"), &CancellationToken::new()).unwrap_err();
        assert_that!(err.kind()).is_equal_to(ErrorKind::Failed);
    }
}
