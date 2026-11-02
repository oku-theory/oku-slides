use speculoos::prelude::*;

use super::{REDACTED, redact_secrets, register_secret};

#[test]
fn redact_secrets_registered_value_is_masked_everywhere() {
    register_secret("sk-test-abcdef123456");
    let out = redact_secrets("key=sk-test-abcdef123456 and again sk-test-abcdef123456");
    assert_that!(out).is_equal_to(format!("key={REDACTED} and again {REDACTED}"));
}

#[test]
fn redact_secrets_short_value_is_not_registered() {
    register_secret("short");
    assert_that!(redact_secrets("a short word").as_str()).is_equal_to("a short word");
}

#[test]
fn redact_secrets_value_containing_another_masks_the_longest() {
    register_secret("token-12345678");
    register_secret("token-12345678-extended");
    assert_that!(redact_secrets("x token-12345678-extended y")).is_equal_to(format!("x {REDACTED} y"));
}

#[test]
fn redact_secrets_padded_value_is_registered_trimmed() {
    register_secret("  padded-secret-99  ");
    assert_that!(redact_secrets("v=padded-secret-99")).is_equal_to(format!("v={REDACTED}"));
}
