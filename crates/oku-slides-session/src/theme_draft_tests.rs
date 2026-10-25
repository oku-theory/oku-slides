use oku_slides_core::{Op, ThemePin};
use serde_json::{Map, Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{add_user_theme, fresh_dir, ns, opened_on, session};
use crate::theme::PAPIER_NAMESPACE;
use crate::{Mode, Session};

fn p(path: &[&str]) -> Vec<String> {
    path.iter().map(|s| (*s).to_owned()).collect()
}

fn object(v: Value) -> Map<String, Value> {
    serde_json::from_value(v).unwrap_or_default()
}

/// Une session en mode pack dont le brouillon part du thème `namespace` (pris pour le deck d'abord).
fn pack_session_with_theme(s: &mut Session, namespace: &str) {
    s.set_theme(&ns(namespace)).unwrap();
    s.edit_pack(ESSENTIEL_PACK).unwrap();
}

/// L'épinglage du thème du deck de l'utilisateur.
fn user_pin(s: &Session) -> ThemePin {
    s.deck_theme_pin().cloned().unwrap()
}

/// La valeur résolue par Typst au dernier rendu, au chemin `path`.
fn rendered_token(s: &Session, path: &str) -> Option<Value> {
    s.compiled.as_ref()?.layout().tokens.as_ref()?.pointer(path).cloned()
}

fn set_accent(s: &mut Session, accent: &str) {
    s.apply("x", vec![Op::SetToken { path: p(&["color", "accent"]), value: json!(accent) }], None).unwrap();
}

#[test]
fn saving_an_edited_builtin_creates_a_child_with_only_the_edits() {
    let mut s = session();
    pack_session_with_theme(&mut s, PAPIER_NAMESPACE);
    set_accent(&mut s, "#123456");
    s.save_pack(false).unwrap();
    let m = s.theme_manifest(&user_pin(&s)).unwrap();
    assert_that!(m.extends.clone()).is_equal_to(vec![ns("papier")]);
    assert_that!(Value::Object(m.tokens)).is_equal_to(json!({"color": {"accent": "#123456"}}));
    assert_that!(m.name.as_str()).is_equal_to("Papier (variante)");
    assert_that!(m.parent).is_none();
}

#[test]
fn saved_child_is_pinned_with_its_builtin_parent() {
    let mut s = session();
    pack_session_with_theme(&mut s, PAPIER_NAMESPACE);
    set_accent(&mut s, "#123456");
    s.save_pack(false).unwrap();
    s.close_pack().unwrap();
    let pins: Vec<String> = s.deck().themes.keys().map(|n| n.as_str().to_owned()).collect();
    assert_that!(pins.len()).is_equal_to(2);
    assert_that!(pins).contains(PAPIER_NAMESPACE.to_owned());
    assert_that!(rendered_token(&s, "/color/accent")).is_equal_to(Some(json!("#123456")));
}

#[test]
fn editing_continues_on_the_saved_child_as_a_user_theme() {
    let mut s = session();
    pack_session_with_theme(&mut s, PAPIER_NAMESPACE);
    set_accent(&mut s, "#123456");
    s.save_pack(false).unwrap();
    let view = s.snapshot().editing.unwrap().theme;
    assert_that!((view.child, view.dirty)).is_equal_to((false, false));
    let names: Vec<String> = view.extends.into_iter().map(|p| p.name).collect();
    assert_that!(names).is_equal_to(vec!["Papier".to_owned()]);
}

#[test]
fn editing_a_user_theme_keeps_its_namespace_and_extends() {
    let data = fresh_dir("oku-slides-theme-draft-");
    let mut s = opened_on(data.path());
    add_user_theme(
        &s.themes,
        Uuid::new_v4(),
        "marque",
        &["pro"],
        object(json!({"color": {"surface": "#123456"}})),
        None,
    );
    pack_session_with_theme(&mut s, "marque");
    assert_that!(s.deck().tokens.clone()).is_equal_to(json!({"color": {"surface": "#123456"}}));
    set_accent(&mut s, "#abcdef");
    s.save_pack(false).unwrap();
    let m = s.theme_manifest(&user_pin(&s)).unwrap();
    assert_that!((m.namespace.clone(), m.extends.clone())).is_equal_to((ns("marque"), vec![ns("pro")]));
    assert_that!(Value::Object(m.tokens)).is_equal_to(json!({"color": {"surface": "#123456", "accent": "#abcdef"}}));
}

#[test]
fn pack_mode_layer_of_a_child_draft_includes_its_builtin() {
    let mut s = session();
    pack_session_with_theme(&mut s, "pro");
    assert_that!(s.deck().tokens.clone()).is_equal_to(Value::Null);
    assert_that!(rendered_token(&s, "/color/brand-ink")).is_equal_to(Some(json!("#00154e")));
}

#[test]
fn pack_mode_layer_of_a_user_theme_draft_is_its_parents_only() {
    let data = fresh_dir("oku-slides-theme-draft-");
    let mut s = opened_on(data.path());
    let own = object(json!({"color": {"marque": "#123456"}}));
    add_user_theme(&s.themes, Uuid::new_v4(), "marque", &["pro"], own, None);
    pack_session_with_theme(&mut s, "marque");
    let layer = s.theme_layer(s.deck()).unwrap();
    assert_that!(layer.get("color").and_then(|c| c.get("marque"))).is_none();
    assert_that!(layer.get("color").and_then(|c| c.get("brand-ink"))).is_equal_to(Some(&json!("#00154e")));
}

#[test]
fn draft_extends_of_a_builtin_names_the_builtin() {
    let mut s = session();
    pack_session_with_theme(&mut s, "pro");
    let view = s.snapshot().editing.unwrap().theme;
    let names: Vec<String> = view.extends.into_iter().map(|p| p.name).collect();
    assert_that!((view.child, names)).is_equal_to((true, vec!["Pro".to_owned()]));
}

#[test]
fn unchanged_child_draft_saves_nothing() {
    let mut s = session();
    pack_session_with_theme(&mut s, PAPIER_NAMESPACE);
    let themes = s.themes.themes();
    s.save_pack(false).unwrap();
    assert_that!(s.themes.themes()).is_equal_to(themes);
    assert_that!(matches!(s.mode, Mode::Pack { .. })).is_true();
}
