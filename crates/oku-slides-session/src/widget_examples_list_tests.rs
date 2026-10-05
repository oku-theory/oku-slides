//! Listes des valeurs d'exemple des widgets : bornes de taille du cœur et liste vidée gardée `[]`,
//! par les trois chemins (`set_example_values`, `set_example_values_by_agent`,
//! `edit_example_list`).

use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use oku_slides_core::{ListOp, PLAIN_MAX};

use crate::pack::PackManifest;
use crate::test_support::{fresh_dir, opened_on, set_frise_widget};
use crate::widgets_edit::parse_name;
use crate::{Session, SessionErrorKind};

/// Une liste d'au moins un item (`min: 1`), sur 1 à 3 items selon `max`.
fn bornee(max: usize) -> String {
    format!(
        r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("bornee", params: (items: p.list(min: 1, max: {max}, item: (t: p.plain()),
  default: ((t: "un"),))), (args, ctx) => [#args.items.len()])
"#
    )
}

/// Une session en édition des modèles du deck, avec la frise de test et `bornee`.
fn editing() -> (tempfile::TempDir, Session) {
    let data = fresh_dir("oku-slides-example-lists-");
    let mut s = opened_on(data.path());
    let set = s.deck().templates.clone().unwrap().id;
    s.edit_templates(set).unwrap();
    set_frise_widget(&mut s);
    s.set_widget("bornee", bornee(3)).unwrap();
    (data, s)
}

fn one(param: &str, v: Value) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert(param.into(), v);
    m
}

fn stored(s: &Session, widget: &str, param: &str) -> Option<Value> {
    s.example_values(&parse_name(widget).unwrap()).unwrap().get(param).cloned()
}

fn etapes(s: &Session) -> Option<Value> {
    stored(s, "frise", "etapes")
}

/// Le chemin d'écriture d'une liste entière : l'interface ou l'agent.
#[derive(Clone, Copy, Debug)]
enum Path {
    Ui,
    Agent,
}

fn set_list(s: &mut Session, path: Path, widget: &str, param: &str, v: Value) -> crate::Result<Map<String, Value>> {
    match path {
        Path::Ui => s.set_example_values(widget, one(param, v)),
        Path::Agent => s.set_example_values_by_agent(widget, one(param, v)),
    }
}

fn kept_list() -> Value {
    json!([{ "id": "a", "date": "1999", "texte": "Un" }])
}

#[rstest]
#[case::ui(Path::Ui)]
#[case::agent(Path::Agent)]
fn set_example_values_with_a_cell_too_long_is_refused_and_keeps_the_list(#[case] path: Path) {
    let (_d, mut s) = editing();
    set_list(&mut s, path, "frise", "etapes", kept_list()).unwrap();
    let long = json!([{ "id": "a", "texte": "x".repeat(PLAIN_MAX + 1) }]);
    let err = set_list(&mut s, path, "frise", "etapes", long).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TrialValue);
    assert_that!(etapes(&s)).is_equal_to(Some(kept_list()));
}

#[test]
fn edit_example_list_with_a_cell_too_long_is_refused_and_keeps_the_list() {
    let (_d, mut s) = editing();
    s.set_example_values("frise", one("etapes", kept_list())).unwrap();
    let change = ListOp::SetField { item: "a".into(), field: "texte".into(), value: json!("x".repeat(PLAIN_MAX + 1)) };
    let err = s.edit_example_list("frise", "etapes", change).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TrialValue);
    assert_that!(etapes(&s)).is_equal_to(Some(kept_list()));
}

#[rstest]
#[case::ui(Path::Ui)]
#[case::agent(Path::Agent)]
fn set_example_values_empty_list_is_kept_and_null_erases(#[case] path: Path) {
    let (_d, mut s) = editing();
    set_list(&mut s, path, "frise", "etapes", json!([])).unwrap();
    assert_that!(etapes(&s)).is_equal_to(Some(json!([])));
    set_list(&mut s, path, "frise", "etapes", Value::Null).unwrap();
    assert_that!(etapes(&s)).is_none();
    set_list(&mut s, path, "frise", "etapes", json!([])).unwrap();
    assert_that!(etapes(&s)).is_equal_to(Some(json!([])));
}

#[test]
fn emptying_by_edit_example_list_or_set_example_values_stores_the_same_value() {
    let (_d, mut s) = editing();
    for id in ["d0", "d1", "d2", "d3"] {
        s.edit_example_list("frise", "etapes", ListOp::Remove { item: id.into() }).unwrap();
    }
    let by_edit = etapes(&s);
    assert_that!(by_edit.clone()).is_equal_to(Some(json!([])));
    s.set_example_values("frise", one("etapes", Value::Null)).unwrap();
    s.set_example_values("frise", one("etapes", json!([]))).unwrap();
    assert_that!(etapes(&s)).is_equal_to(by_edit);
}

#[rstest]
#[case::ui(Path::Ui)]
#[case::agent(Path::Agent)]
fn set_example_values_empty_list_under_the_min_is_refused(#[case] path: Path) {
    let (_d, mut s) = editing();
    let err = set_list(&mut s, path, "bornee", "items", json!([])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TrialValue);
    assert_that!(stored(&s, "bornee", "items")).is_none();
}

#[test]
fn edit_example_list_emptying_a_list_under_the_min_is_refused() {
    let (_d, mut s) = editing();
    let err = s.edit_example_list("bornee", "items", ListOp::Remove { item: "d0".into() }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TrialValue);
    assert_that!(stored(&s, "bornee", "items")).is_none();
}

/// Une liste sans minimum, d'un item par défaut, de colonne `col`, sur au plus `max` items.
fn souple(max: usize, col: &str) -> String {
    format!(
        r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("souple", params: (items: p.list(max: {max}, item: ({col}: p.plain()),
  default: (({col}: "un"),))), (args, ctx) => [#args.items.len()])
"#
    )
}

/// Le schéma ne lit plus la liste gardée : `max` abaissé sous son compte, ou colonne renommée.
#[derive(Clone, Copy, Debug)]
enum Drift {
    LowerMax,
    RenamedColumn,
}

#[rstest]
#[case::lower_max_insert(Drift::LowerMax, true)]
#[case::lower_max_remove(Drift::LowerMax, false)]
#[case::renamed_column_insert(Drift::RenamedColumn, true)]
#[case::renamed_column_remove(Drift::RenamedColumn, false)]
fn edit_example_list_on_a_list_the_schema_no_longer_reads_starts_from_the_defaults(
    #[case] drift: Drift,
    #[case] insert: bool,
) {
    let (_d, mut s) = editing();
    s.set_widget("souple", souple(3, "t")).unwrap();
    let three = json!([{ "id": "a", "t": "1" }, { "id": "b", "t": "2" }, { "id": "c", "t": "3" }]);
    s.set_example_values("souple", one("items", three)).unwrap();
    let (max, col) = match drift {
        Drift::LowerMax => (2, "t"),
        Drift::RenamedColumn => (3, "u"),
    };
    s.set_widget("souple", souple(max, col)).unwrap();
    let change = if insert {
        ListOp::Insert { index: None, id: "n".into(), values: Map::new() }
    } else {
        ListOp::Remove { item: "d0".into() }
    };
    s.edit_example_list("souple", "items", change).unwrap();
    let expected = if insert { json!([{ "id": "d0", col: "un" }, { "id": "n" }]) } else { json!([]) };
    assert_that!(stored(&s, "souple", "items")).is_equal_to(Some(expected));
}

#[test]
fn edit_example_list_on_a_stored_value_that_is_not_a_list_is_refused() {
    let (_d, mut s) = editing();
    let file = parse_name("souple").unwrap();
    s.set_widget("souple", souple(3, "t")).unwrap();
    s.set_example_values("souple", one("items", json!([]))).unwrap();
    // Une valeur gardée qui n'est pas une liste (pack.json écrit à la main) : erreur, pas de départ à neuf.
    let t = s.widget_target().unwrap();
    let mut manifest = PackManifest::from_files(&t.files).unwrap();
    manifest.examples.entry(file).or_default().insert("items".into(), json!("pas une liste"));
    let widgets = t.files.widgets().clone();
    s.commit_manifest(&t, manifest, widgets).unwrap();
    let insert = ListOp::Insert { index: None, id: "n".into(), values: Map::new() };
    let err = s.edit_example_list("souple", "items", insert).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TrialValue);
}
