use speculoos::prelude::*;

use super::*;
use crate::error::too_large;
use crate::source::WarningKind;

#[test]
fn intern_same_path_twice_returns_same_id_and_one_asset() {
    let mut s = Sink::new();
    let a = s.intern("ppt/media/a.png", vec![1, 2, 3], "image/png");
    assert_that!(s.cached_asset("ppt/media/a.png")).is_equal_to(Some(a.clone()));
    assert_that!(a.starts_with("sha256:")).is_true();
    assert_that!(s.finish().unwrap().0.len()).is_equal_to(1);
}

#[test]
fn intern_same_bytes_at_two_paths_keeps_one_asset() {
    let mut s = Sink::new();
    let a = s.intern("ppt/media/a.png", vec![9], "image/png");
    let b = s.intern("ppt/media/b.png", vec![9], "image/png");
    assert_that!(a).is_equal_to(b);
    assert_that!(s.finish().unwrap().0.len()).is_equal_to(1);
}

#[test]
fn warnings_beyond_the_cap_collapse_into_one_truncated() {
    let mut s = Sink::new();
    for _ in 0..(MAX_WARNINGS + 10) {
        s.warn(None, None, WarningKind::MissingPart("x".into()));
    }
    let (_, w) = s.finish().unwrap();
    assert_that!(w.len()).is_equal_to(MAX_WARNINGS + 1);
    assert_that!(w.last().map(|w| w.kind.clone())).is_equal_to(Some(WarningKind::Truncated));
}

#[test]
fn finish_with_a_fatal_error_fails() {
    let mut s = Sink::new();
    s.set_fatal(too_large("p"));
    assert_that!(s.finish().is_err()).is_true();
}

#[test]
fn clip_strips_control_and_bidi_characters_and_truncates() {
    let long = format!("a\u{7}b\u{202e}\n{}", "x".repeat(500));
    let c = clip(&long);
    assert_that!(c.starts_with("abx")).is_true();
    assert_that!(c.chars().count()).is_equal_to(200);
}

#[test]
fn clip_to_cuts_at_the_given_length() {
    assert_that!(clip_to("abcdef", 3)).is_equal_to("abc".to_owned());
}

#[rstest::rstest]
#[case::arabic_letter_mark('\u{061c}')]
#[case::line_separator('\u{2028}')]
#[case::paragraph_separator('\u{2029}')]
#[case::mongolian_vowel_separator('\u{180e}')]
#[case::interlinear_anchor('\u{fff9}')]
#[case::interlinear_separator('\u{fffa}')]
#[case::interlinear_terminator('\u{fffb}')]
fn clip_strips_invisible_formatting_character(#[case] c: char) {
    assert_that!(clip(&format!("a{c}b"))).is_equal_to("ab".to_owned());
}
