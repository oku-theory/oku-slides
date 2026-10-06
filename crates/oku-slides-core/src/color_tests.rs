use rstest::rstest;
use speculoos::prelude::*;

use super::is_hex_color;

#[rstest]
#[case("#abc", true)]
#[case("#A1B2C3", true)]
#[case("#a1b2c3d4", true)]
#[case("#zzz", false)]
#[case("#1234", false)]
#[case("#12345", false)]
#[case("#", false)]
#[case("abc", false)]
#[case(" #abc", false)]
#[case("#abc ", false)]
#[case("#ｆｆｆ", false)]
fn is_hex_color_accepts_only_short_long_and_alpha_hex(#[case] s: &str, #[case] ok: bool) {
    assert_that!(is_hex_color(s)).is_equal_to(ok);
}
