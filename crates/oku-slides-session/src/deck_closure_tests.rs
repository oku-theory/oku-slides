use oku_slides_core::Deck;
use speculoos::prelude::*;

use super::*;
use crate::SessionErrorKind as ErrorKind;
use crate::test_support::{add_current, deck_pinning, library_with, namespaces_added};

#[test]
fn closure_ops_prepends_missing_requirement() {
    let lib = library_with(&[("socle", &[]), ("acme", &["socle"])]);
    let ops = closure_ops(&lib, &Deck::empty(), &[add_current(&lib, "acme")]).unwrap();
    assert_that!(namespaces_added(&ops)).is_equal_to(vec!["socle".to_owned(), "acme".to_owned()]);
}

#[test]
fn closure_ops_skips_requirement_already_pinned() {
    let lib = library_with(&[("socle", &[]), ("acme", &["socle"])]);
    let deck = deck_pinning(&lib, &["socle"]);
    let ops = closure_ops(&lib, &deck, &[add_current(&lib, "acme")]).unwrap();
    assert_that!(namespaces_added(&ops)).is_equal_to(vec!["acme".to_owned()]);
}

#[test]
fn closure_ops_diamond_adds_shared_requirement_once() {
    let lib = library_with(&[("socle", &[]), ("left", &["socle"]), ("right", &["socle"]), ("top", &["left", "right"])]);
    let ops = closure_ops(&lib, &Deck::empty(), &[add_current(&lib, "top")]).unwrap();
    let expected: Vec<String> = ["socle", "left", "right", "top"].map(str::to_owned).to_vec();
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
    let lib = library_with(&[("socle", &[]), ("acme", &["socle"])]);
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
    let pins = pins_of(&[("socle", &[]), ("mid", &["socle"]), ("top", &["mid"]), ("free", &[])]);
    let off = BTreeSet::from([crate::test_support::ns("socle")]);
    assert_that!(names(&dependents(&pins, &off))).is_equal_to(vec!["mid", "top"]);
}

#[test]
fn dependents_of_an_unpinned_requirement_are_off() {
    let pins = pins_of(&[("acme", &["absent"]), ("free", &[])]);
    assert_that!(names(&dependents(&pins, &BTreeSet::new()))).is_equal_to(vec!["acme"]);
}

/// Un élément de slot `{"id", "kind"}` ; `@nested:outer>inner` place `inner` dans le slot `items` d'un `outer`.
fn slot_element(kind: &str, i: usize) -> serde_json::Value {
    match kind.strip_prefix("@nested:").and_then(|k| k.split_once('>')) {
        Some((outer, inner)) => serde_json::json!({
            "id": format!("e{i}"),
            "kind": outer,
            "params": { "items": [{ "id": format!("e{i}-in"), "kind": inner }] },
        }),
        None => serde_json::json!({ "id": format!("e{i}"), "kind": kind }),
    }
}

/// Le manifeste du pack `ns`, de besoins `previous`, avec un exemple par kind de `kinds` : `{"blocks": [élément]}`.
fn manifest_with_examples(ns: &str, previous: &[&str], kinds: &[&str]) -> PackManifest {
    let mut m = crate::test_support::manifest_ns(uuid::Uuid::new_v4(), ns, previous);
    for (i, kind) in kinds.iter().enumerate() {
        let name = oku_slides_core::WidgetFile::parse(&format!("w{i}")).unwrap();
        let example = serde_json::json!({ "blocks": [slot_element(kind, i)] });
        m.examples.insert(name, example.as_object().unwrap().clone());
    }
    m
}

#[rstest::rstest]
#[case::own_namespace_excluded(&[], &["acme/a"], &[])]
#[case::previous_requires_first(&["zeta"], &["base/text"], &["zeta", "base"])]
#[case::duplicates_ignored(&[], &["base/text", "base/card", "base/text"], &["base"])]
#[case::nested_slot_walked(&[], &["@nested:base/stack>essentiel-x/w"], &["base", "essentiel-x"])]
fn examples_requires_table(#[case] previous: &[&str], #[case] kinds: &[&str], #[case] expected: &[&str]) {
    let manifest = manifest_with_examples("acme", previous, kinds);
    let got: Vec<String> =
        examples_requires(&crate::test_support::ns("acme"), &manifest).iter().map(ToString::to_string).collect();
    assert_that!(got).is_equal_to(expected.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
}

#[test]
fn pinned_versions_lists_every_pin_and_theme() {
    let lib = library_with(&[("socle", &[]), ("acme", &["socle"])]);
    let mut deck = deck_pinning(&lib, &["socle", "acme"]);
    crate::test_support::pin_builtin_theme(&mut deck, "pro");
    let mut expected: std::collections::BTreeSet<_> = deck.all_pins().map(|(_, _, p)| p.version.clone()).collect();
    expected.extend(deck.themes.values().map(|p| p.version.clone()));
    assert_that!(expected.len()).is_greater_than(2);
    assert_that!(pinned_versions(&deck)).is_equal_to(expected);
}
