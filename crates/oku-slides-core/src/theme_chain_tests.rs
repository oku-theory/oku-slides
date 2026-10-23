use std::collections::BTreeMap;

use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;

fn ns(s: &str) -> Namespace {
    Namespace::parse(s).unwrap_or_else(|| panic!("espace de noms invalide : {s}"))
}

fn pin(extends: &[&str]) -> ThemePin {
    let version = PackVersion::from_hex(&"a".repeat(64)).unwrap();
    ThemePin { id: Uuid::nil(), version, extends: extends.iter().map(|n| ns(n)).collect() }
}

fn pins(entries: &[(&str, &[&str])]) -> BTreeMap<Namespace, ThemePin> {
    entries.iter().map(|(n, e)| (ns(n), pin(e))).collect()
}

#[rstest]
#[case::single(&[("marque", &[][..])], "marque", &["marque"])]
#[case::multiple(&[("marque", &["papier", "sombre"][..]), ("papier", &[]), ("sombre", &[])], "marque", &["papier", "sombre", "marque"])]
#[case::diamond(&[("marque", &["papier", "sombre"][..]), ("papier", &["neutre"]), ("sombre", &["neutre"]), ("neutre", &[])], "marque", &["neutre", "papier", "sombre", "marque"])]
fn linearize_closure_gives_post_order_each_once(
    #[case] entries: &[(&str, &[&str])],
    #[case] theme: &str,
    #[case] want: &[&str],
) {
    let map = pins(entries);
    assert_that!(linearize(&ns(theme), &map).map(|c| c.iter().map(|n| n.as_str().to_owned()).collect::<Vec<_>>()))
        .is_ok_containing(want.iter().map(|s| s.to_string()).collect::<Vec<_>>());
}

#[test]
fn linearize_cycle_names_the_chain() {
    let map = pins(&[("a", &["b"]), ("b", &["a"])]);
    let err = linearize(&ns("a"), &map).unwrap_err();
    assert_that!(err).is_equal_to(ThemeChainError::Cycle { chain: vec![ns("a"), ns("b"), ns("a")] });
}

#[test]
fn linearize_self_extension_is_a_cycle() {
    let map = pins(&[("a", &["a"])]);
    assert_that!(linearize(&ns("a"), &map)).is_err_containing(ThemeChainError::Cycle { chain: vec![ns("a"), ns("a")] });
}

#[test]
fn linearize_missing_parent_names_it() {
    let map = pins(&[("a", &["b"])]);
    assert_that!(linearize(&ns("a"), &map)).is_err_containing(ThemeChainError::Missing { namespace: ns("b") });
}

#[rstest]
#[case::missing_parent(&[("a", &["b"][..])], Some("a"))]
#[case::extra_pin(&[("a", &[][..]), ("b", &[])], Some("a"))]
#[case::pins_without_theme(&[("a", &[][..])], None)]
#[case::theme_without_pin(&[], Some("a"))]
fn check_closure_not_exact_is_refused(#[case] entries: &[(&str, &[&str])], #[case] theme: Option<&str>) {
    let map = pins(entries);
    assert_that!(check_closure(theme.map(ns).as_ref(), &map)).is_err();
}

#[test]
fn check_closure_extra_pin_names_it() {
    let map = pins(&[("a", &[]), ("b", &[])]);
    assert_that!(check_closure(Some(&ns("a")), &map)).is_err_containing(ThemeChainError::Extra { namespace: ns("b") });
}

#[rstest]
#[case::none(&[], None)]
#[case::diamond(&[("marque", &["papier", "sombre"][..]), ("papier", &["neutre"]), ("sombre", &["neutre"]), ("neutre", &[])], Some("marque"))]
fn check_closure_exact_is_accepted(#[case] entries: &[(&str, &[&str])], #[case] theme: Option<&str>) {
    let map = pins(entries);
    assert_that!(check_closure(theme.map(ns).as_ref(), &map)).is_ok();
}

#[test]
fn theme_pin_without_extends_omits_the_field() {
    let json = serde_json::to_value(pin(&[])).unwrap();
    assert_that!(json.get("extends")).is_none();
    assert_that!(serde_json::from_value::<ThemePin>(json).unwrap()).is_equal_to(pin(&[]));
}
