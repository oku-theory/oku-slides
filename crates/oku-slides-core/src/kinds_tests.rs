use rstest::rstest;
use speculoos::prelude::*;

use super::{base, essentiel, is_builtin_kind};
use crate::{Namespace, qualify, split_kind};

#[test]
fn base_kinds_are_qualified_by_the_base_namespace() {
    let names = ["text", "card", "image", "shape", "chart", "table", "stack"];
    let built: Vec<String> = names.iter().map(|n| qualify(&base::namespace(), n)).collect();
    assert_that!(base::ALL.map(str::to_owned).to_vec()).is_equal_to(built);
}

#[test]
fn base_kinds_split_into_the_base_namespace() {
    let namespaces: Vec<Option<&str>> = base::ALL.iter().map(|k| split_kind(k).map(|(ns, _)| ns)).collect();
    assert_that!(namespaces).is_equal_to(vec![Some(base::NAMESPACE); 7]);
}

#[test]
fn base_namespace_parses_like_its_constant() {
    assert_that!(Namespace::parse(base::NAMESPACE)).is_equal_to(Some(base::namespace()));
}

#[test]
fn essentiel_kinds_are_qualified_by_the_essentiel_namespace() {
    let names = ["title", "toc", "chapter", "content", "end", "blank"];
    let built: Vec<String> = names.iter().map(|n| qualify(&essentiel::namespace(), n)).collect();
    assert_that!(essentiel::ALL.map(str::to_owned).to_vec()).is_equal_to(built);
}

#[test]
fn essentiel_namespace_parses_like_its_constant() {
    assert_that!(Namespace::parse(essentiel::NAMESPACE)).is_equal_to(Some(essentiel::namespace()));
}

#[rstest]
#[case(base::CARD, true)]
#[case(essentiel::CONTENT, true)]
#[case("essentiel/kpi", false)]
#[case("content", false)]
#[case("acme/card", false)]
fn is_builtin_kind_knows_only_the_shipped_kinds(#[case] kind: &str, #[case] expected: bool) {
    assert_that!(is_builtin_kind(kind)).is_equal_to(expected);
}
