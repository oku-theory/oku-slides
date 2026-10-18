use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case("base")]
#[case("essentiel")]
#[case("grilles-2")]
#[case("acme.charts")]
fn namespace_parse_valid_is_some(#[case] s: &str) {
    assert_that!(Namespace::parse(s).map(|n| n.as_str().to_owned())).is_equal_to(Some(s.to_owned()));
}

#[rstest]
#[case("")]
#[case("Base")]
#[case("2d")]
#[case("a/b")]
#[case("a.b.c")]
#[case("a.")]
#[case(".a")]
#[case("a_b")]
fn namespace_parse_invalid_is_none(#[case] s: &str) {
    assert_that!(Namespace::parse(s)).is_none();
}

#[test]
fn namespace_parse_too_long_is_none() {
    assert_that!(Namespace::parse(&"a".repeat(65))).is_none();
}

#[test]
fn qualify_joins_with_a_slash() {
    let ns = Namespace::parse("base").unwrap();
    assert_that!(qualify(&ns, "card")).is_equal_to("base/card".to_owned());
}

#[rstest]
#[case("base/card", Some(("base", "card")))]
#[case("acme.charts/bar", Some(("acme.charts", "bar")))]
#[case("text", None)]
#[case("/card", None)]
#[case("base/", None)]
#[case("a/b/c", None)]
fn split_kind_reads_namespace_and_name(#[case] kind: &str, #[case] expected: Option<(&str, &str)>) {
    assert_that!(split_kind(kind)).is_equal_to(expected);
}

#[test]
fn namespace_deck_is_reserved_name() {
    assert_that!(Namespace::deck().is_deck()).is_true();
    assert_that!(Namespace::parse("deck").unwrap().is_deck()).is_true();
}

#[test]
fn namespace_serde_round_trips_as_string() {
    let ns: Namespace = serde_json::from_str("\"essentiel\"").unwrap();
    assert_that!(serde_json::to_string(&ns).unwrap()).is_equal_to("\"essentiel\"".to_owned());
    assert_that!(serde_json::from_str::<Namespace>("\"Bad\"")).is_err();
}
