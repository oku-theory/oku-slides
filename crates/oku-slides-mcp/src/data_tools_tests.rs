use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{label, list_reply};
use crate::limits::{MAX_READ_BYTES, MAX_READ_CELL_CHARS};

/// An entry whose compact JSON is exactly `bytes` long, padded with the 2-byte `é`.
fn entry(bytes: usize) -> Value {
    let skeleton = json!({ "name": "" }).to_string().len();
    let pad = bytes - skeleton;
    let name = format!("{}{}", "é".repeat(pad / 2), "a".repeat(pad % 2));
    let e = json!({ "name": name });
    assert_that!(e.to_string().len()).is_equal_to(bytes);
    e
}

/// Two entries: the second fills the reply to `MAX_READ_BYTES` plus `over`.
#[rstest]
#[case::at_cap(0, 2, false)]
#[case::one_byte_over(1, 1, true)]
fn list_reply_keeps_whole_data_within_the_byte_cap(#[case] over: usize, #[case] kept: usize, #[case] truncated: bool) {
    let empty = list_reply(std::iter::empty()).to_string().len();
    let first = 1000;
    // The second entry adds a comma.
    let second = MAX_READ_BYTES - empty - first - 1 + over;
    let reply = list_reply([entry(first), entry(second)].into_iter());
    assert_that!(reply["data"].as_array().map(Vec::len)).is_equal_to(Some(kept));
    assert_that!(reply["truncated"].as_bool()).is_equal_to(Some(truncated));
    assert_that!(reply["untrusted"]).is_equal_to(json!(true));
    assert_that!(reply.to_string().len()).is_less_than_or_equal_to(MAX_READ_BYTES);
}

#[test]
fn list_reply_first_entry_past_the_cap_keeps_none() {
    let reply = list_reply([entry(MAX_READ_BYTES)].into_iter());
    assert_that!(reply["data"]).is_equal_to(json!([]));
    assert_that!(reply["truncated"].as_bool()).is_equal_to(Some(true));
}

/// A name or label is cut to `MAX_READ_CELL_CHARS` characters, multi-byte ones whole.
#[rstest]
#[case::at_cap(MAX_READ_CELL_CHARS, MAX_READ_CELL_CHARS)]
#[case::over_cap(MAX_READ_CELL_CHARS + 1, MAX_READ_CELL_CHARS)]
fn label_cuts_multibyte_text(#[case] chars: usize, #[case] kept: usize) {
    assert_that!(label(&"漢".repeat(chars)).chars().count()).is_equal_to(kept);
}

#[test]
fn label_strips_bidi() {
    assert_that!(label("a\u{202E}b")).is_equal_to("ab".to_owned());
}
