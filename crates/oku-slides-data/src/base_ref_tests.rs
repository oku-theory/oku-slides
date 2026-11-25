use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case::uppercase_hex(format!("sha256:{}", "A".repeat(64)))]
#[case::short(format!("sha256:{}", "a".repeat(63)))]
#[case::missing_prefix("a".repeat(64))]
#[case::dot_dot(format!("sha256:..{}", "a".repeat(62)))]
fn base_id_rejects(#[case] id: String) {
    let parsed = serde_json::from_value::<BaseId>(serde_json::Value::String(id));
    assert_that!(parsed).is_err();
}

#[test]
fn base_id_accepts_a_lowercase_sha256() {
    let id = format!("sha256:{}", "0123456789abcdef".repeat(4));
    let parsed = serde_json::from_value::<BaseId>(serde_json::Value::String(id.clone()));
    assert_that!(parsed.map(|b| b.as_str().to_owned())).is_ok_containing(id);
}
