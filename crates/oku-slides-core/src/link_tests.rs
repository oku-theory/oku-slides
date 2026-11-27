use speculoos::prelude::*;

use super::encode_link_url;

#[test]
fn encode_link_url_leading_lt_and_backslash() {
    assert_that!(encode_link_url("<x")).is_equal_to("%3Cx".to_owned());
    assert_that!(encode_link_url("a\\b\\")).is_equal_to("a%5Cb%5C".to_owned());
}

#[test]
fn encode_link_url_inner_lt_kept() {
    assert_that!(encode_link_url("a<b")).is_equal_to("a<b".to_owned());
}

#[test]
fn encode_link_url_space_and_parentheses_encoded() {
    assert_that!(encode_link_url("https://x.org/a (b)")).is_equal_to("https://x.org/a%20%28b%29".to_owned());
}
