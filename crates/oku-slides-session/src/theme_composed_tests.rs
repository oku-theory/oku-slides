use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{is_builtin_namespace, without_unpinned_components};
use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{add_pack, add_user_theme, add_version, fresh_dir, manifest_ns, ns, opened_on, session};
use crate::theme_schema::Section;

#[test]
fn pack_tokens_of_a_new_deck_are_base_then_essentiel() {
    let s = session();
    let names: Vec<String> = s.pack_tokens(s.deck()).into_iter().map(|(n, _, _)| n.as_str().to_owned()).collect();
    assert_that!(names).is_equal_to(vec!["base".to_owned(), "essentiel".to_owned()]);
}

#[test]
fn token_schema_of_a_new_deck_holds_both_builtin_sections() {
    let s = session();
    let schema = s.token_schema(s.deck());
    for pack in ["base", "essentiel"] {
        asserting(pack).that(&schema.iter().any(|t| t.section == Section::Pack(ns(pack)))).is_true();
    }
}

#[test]
fn builtin_namespaces_are_base_and_essentiel_only() {
    assert_that!((is_builtin_namespace(&ns("base")), is_builtin_namespace(&ns("essentiel")))).is_equal_to((true, true));
    assert_that!(is_builtin_namespace(&ns("frise"))).is_false();
}

/// Le pack de widgets `frise` des tests : un jeton `dot.bg`, sans widgets.
const FRISE: uuid::Uuid = uuid::Uuid::from_u128(0xf415e000_0000_4000_8000_000000000001);

/// Le deck de `s` épingle `frise`, qui déclare les jetons de composant `dot.bg` (« Point ») et `dot.fg` (sans libellé).
fn pin_frise(s: &mut crate::Session) {
    let mut m = manifest_ns(FRISE, "frise", &["base"]);
    let entry = json!({"path": ["dot", "bg"], "default": "bg", "control": "color", "label": "Point"});
    let bare = json!({"path": ["dot", "fg"], "default": "ink", "control": "color"});
    m.tokens = vec![serde_json::from_value(entry).unwrap(), serde_json::from_value(bare).unwrap()];
    let pin = add_version(&s.library, &m, &[]);
    s.apply("Frise", vec![add_pack(&m, &pin)], None).unwrap();
}

/// Une session sur un dossier neuf dont le deck épingle `frise`, et le thème utilisateur `namespace` (jetons `tokens`).
fn session_with_frise_and_theme(namespace: &str, tokens: Value) -> (tempfile::TempDir, crate::Session) {
    let data = fresh_dir("oku-slides-composed-");
    let mut s = opened_on(data.path());
    pin_frise(&mut s);
    let tokens = tokens.as_object().cloned().unwrap();
    add_user_theme(&s.themes, uuid::Uuid::new_v4(), namespace, &[], tokens, None);
    (data, s)
}

#[test]
fn pack_mode_schema_holds_the_packs_of_the_stashed_deck() {
    let (_d, mut s) = session_with_frise_and_theme("marque", json!({}));
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    assert_that!(s.deck().widgets.contains_key(&ns("frise"))).is_false();
    let schema = s.token_schema(s.deck());
    assert_that!(schema.iter().any(|t| t.section == Section::Pack(ns("frise")))).is_true();
}

#[test]
fn user_theme_with_tokens_of_a_stashed_only_pack_can_be_chosen_and_shows_them() {
    let tokens = json!({"component": {"frise": {"dot": {"bg": "#123456"}}}});
    let (_d, mut s) = session_with_frise_and_theme("marque", tokens.clone());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_edited_theme(&ns("marque")).unwrap();
    assert_that!(s.deck().tokens.clone()).is_equal_to(tokens);
    let surface = serde_json::to_string(&s.host_theme("").unwrap().unwrap()).unwrap();
    assert_that!(surface.contains("s-pack:frise")).is_true();
    assert_that!(surface.contains("component.frise.dot.bg")).is_true();
    assert_that!(surface.contains("\"Point\"")).is_true();
}

#[test]
fn theme_with_tokens_of_an_unpinned_pack_can_be_chosen() {
    let tokens = json!({"component": {"ailleurs": {"x": {"y": 1}}}});
    let (_d, mut s) = session_with_frise_and_theme("autre", tokens.clone());
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    s.set_edited_theme(&ns("autre")).unwrap();
    assert_that!(s.deck().tokens.clone()).is_equal_to(tokens);
}

#[test]
fn invalid_token_of_a_pinned_pack_is_still_refused() {
    let tokens = json!({"component": {"frise": {"dot": {"bg": "pas-une-couleur"}}}});
    let (_d, mut s) = session_with_frise_and_theme("fausse", tokens);
    s.edit_pack(ESSENTIEL_PACK).unwrap();
    let err = s.set_edited_theme(&ns("fausse")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::SessionErrorKind::InvalidTokenValue);
}

#[test]
fn without_unpinned_components_keeps_pinned_packs_and_global_tokens() {
    let tokens = json!({"color": {"accent": "#fff"}, "component": {"base": {"card": {}}, "ailleurs": {"x": 1}}});
    let pinned = std::collections::BTreeSet::from([ns("base")]);
    let kept = without_unpinned_components(tokens.as_object().unwrap(), &pinned);
    assert_that!(Value::Object(kept))
        .is_equal_to(json!({"color": {"accent": "#fff"}, "component": {"base": {"card": {}}}}));
}

/// t-1106 : le jeton d'un pack non intégré se nomme par son `label`, sinon par son chemin dans le pack.
#[rstest::rstest]
#[case::label("bg", "Point")]
#[case::path("fg", "dot.fg")]
fn token_label_pack_not_builtin_falls_back_to_label_then_path(#[case] leaf: &str, #[case] expected: &str) {
    let (_d, s) = session_with_frise_and_theme("marque", json!({}));
    let schema = s.token_schema(s.deck());
    let path: Vec<String> = ["component", "frise", "dot", leaf].map(str::to_owned).to_vec();
    let label = crate::theme_panel::token_label(&schema, &path, &oku_slides_i18n::Localizer::new(Some("fr")));
    assert_that!(label).is_equal_to(expected.to_owned());
}
