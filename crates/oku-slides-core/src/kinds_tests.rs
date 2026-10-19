use speculoos::prelude::*;

use super::base;
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
