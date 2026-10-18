use oku_slides_core::Deck;
use speculoos::prelude::*;

use super::*;
use crate::SessionErrorKind as ErrorKind;
use crate::test_support::{add_current, deck_pinning, library_with, namespaces_added};

#[test]
fn closure_ops_prepends_missing_requirement() {
    let lib = library_with(&[("base", &[]), ("acme", &["base"])]);
    let ops = closure_ops(&lib, &Deck::empty(), &[add_current(&lib, "acme")]).unwrap();
    assert_that!(namespaces_added(&ops)).is_equal_to(vec!["base".to_owned(), "acme".to_owned()]);
}

#[test]
fn closure_ops_skips_requirement_already_pinned() {
    let lib = library_with(&[("base", &[]), ("acme", &["base"])]);
    let deck = deck_pinning(&lib, &["base"]);
    let ops = closure_ops(&lib, &deck, &[add_current(&lib, "acme")]).unwrap();
    assert_that!(namespaces_added(&ops)).is_equal_to(vec!["acme".to_owned()]);
}

#[test]
fn closure_ops_diamond_adds_shared_requirement_once() {
    let lib = library_with(&[("base", &[]), ("left", &["base"]), ("right", &["base"]), ("top", &["left", "right"])]);
    let ops = closure_ops(&lib, &Deck::empty(), &[add_current(&lib, "top")]).unwrap();
    let expected: Vec<String> = ["base", "left", "right", "top"].map(str::to_owned).to_vec();
    assert_that!(namespaces_added(&ops)).is_equal_to(expected);
}

#[test]
fn closure_ops_unknown_requirement_is_refused() {
    let lib = library_with(&[("acme", &["ghost"])]);
    let err = closure_ops(&lib, &Deck::empty(), &[add_current(&lib, "acme")]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ErrorKind::UnknownNamespace);
}

#[test]
fn closure_ops_cycle_is_refused() {
    let lib = library_with(&[("a", &["b"]), ("b", &["c"]), ("c", &["a"])]);
    let err = closure_ops(&lib, &Deck::empty(), &[add_current(&lib, "a")]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ErrorKind::RequiresCycle);
    let shown = oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("fr")));
    assert_that!(shown.as_str()).contains("a → b → c → a");
}

#[test]
fn closure_ops_keeps_other_ops_in_place() {
    let lib = library_with(&[("base", &[]), ("acme", &["base"])]);
    let other = oku_slides_core::Op::SetDeckParam { key: "lang".into(), value: serde_json::json!("fr") };
    let ops = closure_ops(&lib, &Deck::empty(), &[other.clone(), add_current(&lib, "acme")]).unwrap();
    assert_that!(ops.first()).is_equal_to(Some(&other));
    assert_that!(ops.len()).is_equal_to(3);
}

#[test]
fn closure_ops_self_requirement_adds_nothing_more() {
    let lib = library_with(&[("acme", &["acme"])]);
    let ops = closure_ops(&lib, &Deck::empty(), &[add_current(&lib, "acme")]).unwrap();
    assert_that!(namespaces_added(&ops)).is_equal_to(vec!["acme".to_owned()]);
}

/// Des épingles `(espace, besoins)`, toutes du même pack (seuls les besoins comptent ici).
fn pins_of(entries: &[(&str, &[&str])]) -> BTreeMap<Namespace, PackPin> {
    let version = PackVersion::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    entries
        .iter()
        .map(|(ns, requires)| {
            let requires = requires.iter().map(|r| crate::test_support::ns(r)).collect();
            (crate::test_support::ns(ns), PackPin { id: uuid::Uuid::nil(), version: version.clone(), requires })
        })
        .collect()
}

fn names(set: &BTreeSet<Namespace>) -> Vec<&str> {
    set.iter().map(Namespace::as_str).collect()
}

#[test]
fn dependents_of_an_off_pack_include_its_transitive_dependents_only() {
    let pins = pins_of(&[("base", &[]), ("mid", &["base"]), ("top", &["mid"]), ("free", &[])]);
    let off = BTreeSet::from([crate::test_support::ns("base")]);
    assert_that!(names(&dependents(&pins, &off))).is_equal_to(vec!["mid", "top"]);
}

#[test]
fn dependents_of_an_unpinned_requirement_are_off() {
    let pins = pins_of(&[("acme", &["absent"]), ("free", &[])]);
    assert_that!(names(&dependents(&pins, &BTreeSet::new()))).is_equal_to(vec!["acme"]);
}
