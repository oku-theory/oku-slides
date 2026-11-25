//! A request's JSON is refused past `MAX_APPLY_BYTES`, with a localized error.

use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{MAX_APPLY_BYTES, check_request_size};
use crate::SessionErrorKind;

/// A `snapshot` request whose JSON takes exactly `bytes` bytes.
fn padded(bytes: usize) -> Value {
    let base = json!({ "cmd": "snapshot", "pad": "" });
    let len = serde_json::to_vec(&base).map_or(0, |v| v.len());
    json!({ "cmd": "snapshot", "pad": "x".repeat(bytes - len) })
}

#[rstest]
#[case::at_cap(MAX_APPLY_BYTES, true)]
#[case::over_cap(MAX_APPLY_BYTES + 1, false)]
fn check_request_size_caps_at_8_mib(#[case] bytes: usize, #[case] accepted: bool) {
    let request = padded(bytes);
    assert_that!(serde_json::to_vec(&request).map_or(0, |v| v.len())).is_equal_to(bytes);
    let checked = check_request_size(&request);
    assert_that!(checked.is_ok()).is_equal_to(accepted);
    if let Err(e) = checked {
        assert_that!(e.kind()).is_equal_to(SessionErrorKind::RequestTooLarge);
        let fr = Localizer::new(Some("fr"));
        assert_that!(e.localize(&fr)).is_equal_to(fr.tr_with("session-request-too-large", &[("mib", 8.into())]));
    }
}
