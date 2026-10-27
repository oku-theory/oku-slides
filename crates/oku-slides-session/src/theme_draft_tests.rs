use serde_json::{Map, Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::Session;
use crate::test_support::{add_user_theme, fresh_dir, ns, session_on};

fn object(v: Value) -> Map<String, Value> {
    serde_json::from_value(v).unwrap_or_default()
}

/// Opens the theme tab of the library theme `namespace` in the fresh session `s`.
fn open_theme_tab(s: &mut Session, namespace: &str) {
    let id = s.themes.by_namespace(&ns(namespace)).unwrap().manifest.id;
    s.open_theme_here(id).unwrap();
}

/// The value Typst resolved at the last render, at `path`.
fn rendered_token(s: &Session, path: &str) -> Option<Value> {
    s.compiled.as_ref()?.layout().tokens.as_ref()?.pointer(path).cloned()
}

#[test]
fn theme_tab_on_a_user_theme_keeps_its_namespace_and_extends() {
    let data = fresh_dir("oku-slides-theme-draft-");
    let mut s = session_on(data.path());
    let own = object(json!({"color": {"surface": "#123456"}}));
    add_user_theme(&s.themes, Uuid::new_v4(), "marque", &["pro"], own, None);
    open_theme_tab(&mut s, "marque");
    assert_that!(s.deck().tokens.clone()).is_equal_to(json!({"color": {"surface": "#123456"}}));
    let view = s.snapshot().theming.unwrap().theme;
    let parents: Vec<String> = view.extends.into_iter().map(|p| p.namespace).collect();
    assert_that!((view.namespace, parents)).is_equal_to(("marque".to_owned(), vec!["pro".to_owned()]));
}

#[test]
fn theme_tab_layer_of_a_child_draft_includes_its_builtin() {
    let data = fresh_dir("oku-slides-theme-draft-");
    let mut s = session_on(data.path());
    open_theme_tab(&mut s, "pro");
    assert_that!(s.deck().tokens.clone()).is_equal_to(Value::Null);
    assert_that!(rendered_token(&s, "/color/brand-ink")).is_equal_to(Some(json!("#00154e")));
}

#[test]
fn theme_tab_layer_of_a_user_theme_draft_is_its_parents_only() {
    let data = fresh_dir("oku-slides-theme-draft-");
    let mut s = session_on(data.path());
    let own = object(json!({"color": {"marque": "#123456"}}));
    add_user_theme(&s.themes, Uuid::new_v4(), "marque", &["pro"], own, None);
    open_theme_tab(&mut s, "marque");
    let layer = s.theme_layer(s.deck()).unwrap();
    assert_that!(layer.get("color").and_then(|c| c.get("marque"))).is_none();
    assert_that!(layer.get("color").and_then(|c| c.get("brand-ink"))).is_equal_to(Some(&json!("#00154e")));
}

#[test]
fn draft_extends_of_a_builtin_names_the_builtin() {
    let data = fresh_dir("oku-slides-theme-draft-");
    let mut s = session_on(data.path());
    open_theme_tab(&mut s, "pro");
    let view = s.snapshot().theming.unwrap().theme;
    let names: Vec<String> = view.extends.into_iter().map(|p| p.name).collect();
    assert_that!((view.child, names)).is_equal_to((true, vec!["Pro".to_owned()]));
}
