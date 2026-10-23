use std::collections::BTreeMap;

use oku_slides_core::{Namespace, Op, ThemePin};
use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::library_kind::LibraryKind;
use crate::test_support::{add_user_theme, fresh_dir, ns, session, session_on};
use crate::theme::{PAPIER_NAMESPACE, PAPIER_THEME, PRO_NAMESPACE, PRO_THEME, Themes};
use crate::{Request, Session};

/// Les jetons de Pro, l'accent remplacé par `accent`.
fn tokens_with_accent(accent: &str) -> Map<String, Value> {
    let pro = Themes::builtins().iter().find(|b| b.manifest.id == PRO_THEME).unwrap();
    let mut tokens = pro.manifest.tokens.clone();
    tokens["color"]["accent"] = Value::String(accent.into());
    tokens
}

fn accent(s: &Session) -> Option<String> {
    s.appearance.slide.get("accent").cloned()
}

/// Une session sur `fixtures/roadmap.json` passée au thème intégré `namespace`.
fn session_with_theme(namespace: &str) -> Session {
    let mut s = session();
    s.set_theme(&ns(namespace)).unwrap();
    s
}

/// Le thème du deck de `s` et les espaces de noms qu'il épingle.
fn pinned(s: &Session) -> (Option<String>, Vec<String>) {
    let deck = s.deck();
    (deck.theme.as_ref().map(|t| t.as_str().to_owned()), deck.themes.keys().map(|t| t.as_str().to_owned()).collect())
}

#[test]
fn new_deck_pins_papier() {
    let data = fresh_dir("oku-slides-deck-theme-");
    let s = session_on(data.path());
    assert_that!(pinned(&s)).is_equal_to((Some("papier".to_owned()), vec!["papier".to_owned()]));
    let pin = &s.deck().themes[&ns(PAPIER_NAMESPACE)];
    assert_that!(pin.id).is_equal_to(PAPIER_THEME);
    assert_that!(s.doc.sources.themes.contains_key(&pin.version)).is_true();
    assert_that!(s.tab_info().pristine).is_true();
}

#[test]
fn json_deck_without_theme_adopts_the_default_theme() {
    let s = session();
    assert_that!(pinned(&s)).is_equal_to((Some("papier".to_owned()), vec!["papier".to_owned()]));
    assert_that!(s.doc.sources.themes.len()).is_equal_to(1);
}

#[test]
fn set_theme_embeds_and_is_undoable() {
    let mut s = session_with_theme(PRO_NAMESPACE);
    let pin = s.deck().themes[&ns(PRO_NAMESPACE)].clone();
    assert_that!(pin.id).is_equal_to(PRO_THEME);
    assert_that!(s.doc.sources.themes.contains_key(&pin.version)).is_true();
    s.undo().unwrap();
    assert_that!(pinned(&s)).is_equal_to((Some("papier".to_owned()), vec!["papier".to_owned()]));
}

#[test]
fn set_theme_changes_the_rendered_accent() {
    let mut s = session();
    let before = accent(&s);
    s.set_theme(&ns(PRO_NAMESPACE)).unwrap();
    assert_that!(accent(&s)).is_equal_to(Some("#00bef4".to_owned()));
    s.undo().unwrap();
    assert_that!(accent(&s)).is_equal_to(before);
}

#[rstest::rstest]
#[case::namespace("pro")]
#[case::name("Pro")]
#[case::id("7e1b9c3d-2f4a-4d6e-a8b0-5c7d9e1f3b42")]
fn set_theme_request_takes_a_namespace_an_id_or_a_name(#[case] theme: &str) {
    let mut s = session();
    let snap: Value = serde_json::from_str(&s.handle(Request::SetTheme { theme: theme.into() }).unwrap()).unwrap();
    assert_that!(snap["theme"].as_str()).is_equal_to(Some("pro"));
    assert_that!(snap["theme_pins"]["pro"]["id"].as_str()).is_equal_to(Some(PRO_THEME.to_string().as_str()));
    let listed: Vec<&str> = snap["themes"].as_array().unwrap().iter().filter_map(|t| t["namespace"].as_str()).collect();
    assert_that!(listed).contains("pro");
}

#[test]
fn deck_renders_with_embedded_version_not_library_current() {
    let data = fresh_dir("oku-slides-deck-theme-");
    let mut s = crate::test_support::opened_on(data.path());
    let id = Uuid::new_v4();
    let red = add_user_theme(&s.themes, id, "rouge", &[], tokens_with_accent("#ff0000"), None);
    s.set_theme(&ns("rouge")).unwrap();
    assert_that!(accent(&s)).is_equal_to(Some("#ff0000".to_owned()));
    let blue = add_user_theme(&s.themes, id, "rouge", &[], tokens_with_accent("#0000ff"), Some(&red));
    assert_that!(s.themes.current(id)).is_equal_to(Some(blue));
    s.rerender().unwrap();
    assert_that!(s.deck().themes[&ns("rouge")].version.clone()).is_equal_to(red);
    assert_that!(accent(&s)).is_equal_to(Some("#ff0000".to_owned()));
}

#[test]
fn set_theme_unknown_namespace_is_refused() {
    let mut s = session();
    let err = s.set_theme(&ns("absent")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::UnknownTheme);
    assert_that!(pinned(&s).0).is_equal_to(Some("papier".to_owned()));
}

#[test]
fn set_theme_op_to_null_pins_the_default_theme_explicitly() {
    let mut s = session_with_theme(PRO_NAMESPACE);
    s.apply("Défaut", vec![Op::SetTheme { theme: None, themes: BTreeMap::new() }], None).unwrap();
    assert_that!(pinned(&s)).is_equal_to((Some("papier".to_owned()), vec!["papier".to_owned()]));
}

#[test]
fn set_theme_op_without_pins_gets_the_library_closure() {
    let data = fresh_dir("oku-slides-deck-theme-");
    let mut s = crate::test_support::opened_on(data.path());
    add_user_theme(&s.themes, Uuid::new_v4(), "marque", &["pro"], tokens_with_accent("#ff0000"), None);
    let op = Op::SetTheme { theme: Some(ns("marque")), themes: BTreeMap::new() };
    s.apply("Marque", vec![op], None).unwrap();
    assert_that!(pinned(&s)).is_equal_to((Some("marque".to_owned()), vec!["marque".to_owned(), "pro".to_owned()]));
    let embedded = s.deck().themes.values().all(|p| s.doc.sources.themes.contains_key(&p.version));
    assert_that!(embedded).is_true();
}

#[test]
fn set_theme_op_with_an_unknown_version_returns_theme_version_missing() {
    let mut s = session();
    let version = oku_slides_core::PackVersion::from_hex(&"0".repeat(64)).unwrap();
    let themes = BTreeMap::from([(ns(PRO_NAMESPACE), ThemePin { id: PRO_THEME, version, extends: Vec::new() })]);
    let e = s.apply("Thème", vec![Op::SetTheme { theme: Some(ns(PRO_NAMESPACE)), themes }], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(crate::SessionErrorKind::ThemeVersionMissing);
}

#[rstest]
#[case::foreign_namespace(PAPIER_NAMESPACE, PAPIER_THEME, PRO_NAMESPACE, Vec::new())]
#[case::foreign_extends(PRO_NAMESPACE, PRO_THEME, PRO_NAMESPACE, vec![PAPIER_NAMESPACE])]
fn set_theme_op_with_a_pin_that_disagrees_with_its_version_returns_theme_closure(
    #[case] source: &str,
    #[case] id: Uuid,
    #[case] key: &str,
    #[case] extends: Vec<&str>,
) {
    let mut s = session();
    let built = Themes::builtins().iter().find(|b| b.manifest.namespace.as_str() == source).unwrap();
    let pin = ThemePin { id, version: built.version.clone(), extends: extends.into_iter().map(ns).collect() };
    let themes = BTreeMap::from([(ns(key), pin)]);
    let e = s.apply("Thème", vec![Op::SetTheme { theme: Some(ns(key)), themes }], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(crate::SessionErrorKind::ThemeClosure);
}

#[test]
fn deck_theme_with_a_missing_version_renders_with_the_default_theme() {
    let mut s = session();
    let papier = accent(&s);
    let mut deck = s.deck().clone();
    let version = oku_slides_core::PackVersion::from_hex(&"1".repeat(64)).unwrap();
    deck.theme = Some(ns(PRO_NAMESPACE));
    deck.themes = BTreeMap::from([(ns(PRO_NAMESPACE), ThemePin { id: PRO_THEME, version, extends: Vec::new() })]);
    s.doc.sources.apply_silent(deck).unwrap();
    s.rerender().unwrap();
    assert_that!(accent(&s)).is_equal_to(papier);
}

#[test]
fn theme_layer_holds_pack_defaults_under_the_theme() {
    let s = session_with_theme(PRO_NAMESPACE);
    let layer = s.theme_layer(s.deck()).unwrap();
    assert_that!(layer["component"]["base"]["card"]["title-weight"]).is_equal_to(json!(800)); // Pro
    assert_that!(layer["component"]["base"]["card"]["height"]).is_equal_to(json!("auto")); // défaut de base
    assert_that!(layer["component"]["essentiel"].is_object()).is_true();
}

#[test]
fn theme_layer_of_a_child_puts_its_tokens_over_its_parent() {
    let data = fresh_dir("oku-slides-deck-theme-");
    let mut s = crate::test_support::opened_on(data.path());
    let own = json!({"color": {"accent": "#ff0000"}}).as_object().cloned().unwrap();
    add_user_theme(&s.themes, Uuid::new_v4(), "marque", &["pro"], own, None);
    s.set_theme(&ns("marque")).unwrap();
    let layer = s.theme_layer(s.deck()).unwrap();
    assert_that!(layer["color"]["accent"]).is_equal_to(json!("#ff0000"));
    assert_that!(layer["component"]["base"]["card"]["title-weight"]).is_equal_to(json!(800));
}

#[test]
fn snapshot_theme_name_is_the_embedded_manifest_name() {
    let name = |s: &Session| serde_json::to_value(s.snapshot()).unwrap()["theme_name"].as_str().map(str::to_owned);
    let mut s = session();
    assert_that!(name(&s)).is_equal_to(Some("Papier".to_owned()));
    s.set_theme(&ns(PRO_NAMESPACE)).unwrap();
    assert_that!(name(&s)).is_equal_to(Some("Pro".to_owned()));
}

#[test]
fn first_bad_embedded_sighting_is_true_once_per_version() {
    let version = Themes::builtins()[0].version.clone();
    let namespace = Namespace::parse(&format!("t{}", Uuid::new_v4().simple())).unwrap();
    assert_that!(super::first_bad_embedded_sighting(&namespace, &version)).is_true();
    assert_that!(super::first_bad_embedded_sighting(&namespace, &version)).is_false();
}

#[test]
fn repair_theme_closure_drops_an_extra_pin_and_keeps_a_valid_closure() {
    let mut s = session_with_theme(PRO_NAMESPACE);
    assert_that!(super::repair_theme_closure(&s.themes, &mut s.doc.sources).unwrap()).is_false();
    let mut deck = s.deck().clone();
    let papier = Themes::builtins().iter().find(|b| b.manifest.id == PAPIER_THEME).unwrap();
    let extra = ThemePin { id: PAPIER_THEME, version: papier.version.clone(), extends: Vec::new() };
    deck.themes.insert(ns(PAPIER_NAMESPACE), extra);
    s.doc.sources.apply_silent(deck).unwrap();
    assert_that!(super::repair_theme_closure(&s.themes, &mut s.doc.sources).unwrap()).is_true();
    assert_that!(pinned(&s)).is_equal_to((Some("pro".to_owned()), vec!["pro".to_owned()]));
}

#[test]
fn repair_theme_closure_takes_a_missing_parent_from_the_embedded_versions() {
    let data = fresh_dir("oku-slides-deck-theme-repair-");
    let mut s = crate::test_support::opened_on(data.path());
    add_user_theme(&s.themes, Uuid::new_v4(), "marque", &["pro"], Default::default(), None);
    s.set_theme(&ns("marque")).unwrap();
    let pro = s.deck().themes[&ns(PRO_NAMESPACE)].clone();
    let mut deck = s.deck().clone();
    deck.themes.remove(&ns(PRO_NAMESPACE));
    s.doc.sources.apply_silent(deck).unwrap();
    assert_that!(super::repair_theme_closure(&s.themes, &mut s.doc.sources).unwrap()).is_true();
    assert_that!(s.deck().themes.get(&ns(PRO_NAMESPACE))).is_equal_to(Some(&pro));
}
