use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn families() -> BTreeSet<String> {
    ["calibri light", "liberation sans"].map(str::to_owned).into()
}

#[rstest]
#[case("Liberation Sans", true)]
#[case(" LIBERATION SANS ", true)]
#[case("Calibri", true)]
#[case("Calibri Light", true)]
#[case("Calibr", false)]
#[case("Liberation Serif", false)]
#[case("", false)]
fn matches_a_family_or_its_typographic_prefix(#[case] name: &str, #[case] expected: bool) {
    assert_that!(matches(&families(), name)).is_equal_to(expected);
}

#[test]
fn add_without_families_reads_nothing() {
    let mut catalog = FontCatalog::new();
    assert_that!(add(&mut catalog, &BTreeSet::new())).is_equal_to(0);
}
