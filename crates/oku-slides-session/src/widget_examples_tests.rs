use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use crate::pack::{DEFAULT_PACK, EXAMPLE_DEFAULT, Example, PackManifest};
use crate::test_support::{fresh_dir, session_on};
use crate::widgets_edit::parse_name;
use crate::{Request, Session, SessionErrorKind};

const KPI: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => field(ctx, "value", args.value))
"#;

/// Une session en édition du pack par défaut, avec le widget `kpi`.
fn editing() -> (tempfile::TempDir, Session) {
    let data = fresh_dir("oku-slides-examples-");
    let mut s = session_on(data.path());
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_widget("kpi", KPI.into()).unwrap();
    (data, s)
}

fn values(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap()
}

/// Les cas enregistrés dans le manifeste du pack en cours (sans le cas implicite).
fn stored(s: &Session, widget: &str) -> Option<Vec<Example>> {
    let t = s.widget_target().unwrap();
    PackManifest::from_files(&t.files).unwrap().examples.get(&parse_name(widget).unwrap()).cloned()
}

fn names(s: &Session, widget: &str) -> Vec<String> {
    s.examples_of(&parse_name(widget).unwrap()).unwrap().into_iter().map(|e| e.name).collect()
}

fn value_of(s: &Session, widget: &str, example: Option<&str>, param: &str) -> Option<Value> {
    s.example_values(&parse_name(widget).unwrap(), example).unwrap().get(param).cloned()
}

#[test]
fn a_widget_without_stored_examples_has_the_implicit_one() {
    let (_d, s) = editing();
    assert_that!(names(&s, "kpi")).is_equal_to(vec![EXAMPLE_DEFAULT.to_owned()]);
    assert_that!(stored(&s, "kpi")).is_none();
}

#[test]
fn set_example_values_writes_the_first_example_in_the_pack() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", None, values(json!({ "value": "12" }))).unwrap();
    let kept = stored(&s, "kpi").unwrap();
    assert_that!(kept.len()).is_equal_to(1);
    assert_that!(kept[0].name.as_str()).is_equal_to(EXAMPLE_DEFAULT);
    assert_that!(kept[0].values.get("value")).is_equal_to(Some(&json!("12")));
}

#[test]
fn set_example_values_on_a_named_example_leaves_the_others() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Long", None).unwrap();
    s.set_example_values("kpi", Some("Long"), values(json!({ "value": "123456" }))).unwrap();
    assert_that!(value_of(&s, "kpi", Some("Long"), "value")).is_equal_to(Some(json!("123456")));
    assert_that!(value_of(&s, "kpi", None, "value")).is_none();
}

#[test]
fn set_example_values_back_to_neutral_leaves_no_stored_example() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", None, values(json!({ "value": "12" }))).unwrap();
    s.set_example_values("kpi", None, values(json!({ "value": null }))).unwrap();
    assert_that!(stored(&s, "kpi")).is_none();
}

#[test]
fn set_example_values_outside_templates_mode_is_refused() {
    let (_d, mut s) = editing();
    s.close_templates().unwrap();
    let err = s.set_example_values("kpi", None, values(json!({ "value": "12" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotEditingTemplates);
}

#[test]
fn set_example_values_drops_a_stored_value_the_schema_no_longer_takes() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", None, values(json!({ "value": "12", "label": "Ventes" }))).unwrap();
    let v2 = KPI.replace("label: p.plain()", "label: p.number()");
    s.set_widget("kpi", v2).unwrap();
    s.set_example_values("kpi", None, values(json!({ "value": "13" }))).unwrap();
    assert_that!(value_of(&s, "kpi", None, "label")).is_none();
    assert_that!(value_of(&s, "kpi", None, "value")).is_equal_to(Some(json!("13")));
}

#[test]
fn add_example_appends_an_empty_example() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Court", None).unwrap();
    assert_that!(names(&s, "kpi")).is_equal_to(vec![EXAMPLE_DEFAULT.to_owned(), "Court".to_owned()]);
    assert_that!(s.example_values(&parse_name("kpi").unwrap(), Some("Court")).unwrap()).is_empty();
}

#[test]
fn add_example_from_another_copies_its_values() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", None, values(json!({ "value": "12" }))).unwrap();
    s.add_example("kpi", "Copie", Some(EXAMPLE_DEFAULT)).unwrap();
    assert_that!(value_of(&s, "kpi", Some("Copie"), "value")).is_equal_to(Some(json!("12")));
}

#[test]
fn add_example_trims_its_name() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "  Court  ", None).unwrap();
    assert_that!(names(&s, "kpi").last().cloned()).is_equal_to(Some("Court".to_owned()));
}

#[rstest]
#[case::empty("   ", SessionErrorKind::InvalidExampleName)]
#[case::too_long(&"é".repeat(41), SessionErrorKind::InvalidExampleName)]
#[case::taken(" Exemple ", SessionErrorKind::ExampleExists)]
fn add_example_with_a_bad_name_is_refused(#[case] name: &str, #[case] kind: SessionErrorKind) {
    let (_d, mut s) = editing();
    assert_that!(s.add_example("kpi", name, None).unwrap_err().kind()).is_equal_to(kind);
}

#[test]
fn add_example_with_a_name_of_forty_chars_is_accepted() {
    let (_d, mut s) = editing();
    assert_that!(s.add_example("kpi", &"é".repeat(40), None)).is_ok();
}

#[test]
fn add_example_beyond_twelve_is_refused() {
    let (_d, mut s) = editing();
    for i in 1..12 {
        s.add_example("kpi", &format!("Cas {i}"), None).unwrap();
    }
    assert_that!(names(&s, "kpi").len()).is_equal_to(12);
    let err = s.add_example("kpi", "Treizième", None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::TooManyExamples);
}

#[test]
fn add_example_from_an_unknown_example_is_refused() {
    let (_d, mut s) = editing();
    let err = s.add_example("kpi", "Copie", Some("Absent")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownExample);
}

#[test]
fn add_example_on_an_unknown_widget_is_refused() {
    let (_d, mut s) = editing();
    let err = s.add_example("absent", "Court", None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWidget);
}

#[test]
fn rename_example_renames_it_in_place() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Court", None).unwrap();
    s.rename_example("kpi", EXAMPLE_DEFAULT, "Vide").unwrap();
    assert_that!(names(&s, "kpi")).is_equal_to(vec!["Vide".to_owned(), "Court".to_owned()]);
}

#[test]
fn rename_example_to_another_taken_name_is_refused() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Court", None).unwrap();
    let err = s.rename_example("kpi", "Court", EXAMPLE_DEFAULT).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ExampleExists);
}

#[test]
fn rename_example_of_an_unknown_example_is_refused() {
    let (_d, mut s) = editing();
    let err = s.rename_example("kpi", "Absent", "Autre").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownExample);
}

#[test]
fn remove_example_removes_it() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Court", None).unwrap();
    s.remove_example("kpi", EXAMPLE_DEFAULT).unwrap();
    assert_that!(names(&s, "kpi")).is_equal_to(vec!["Court".to_owned()]);
}

#[test]
fn remove_example_of_the_last_one_is_refused() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Court", None).unwrap();
    s.remove_example("kpi", "Court").unwrap();
    let err = s.remove_example("kpi", EXAMPLE_DEFAULT).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::LastExample);
}

#[test]
fn remove_example_of_the_implicit_one_is_refused() {
    let (_d, mut s) = editing();
    let err = s.remove_example("kpi", EXAMPLE_DEFAULT).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::LastExample);
}

#[rstest]
#[case::add(json!({ "cmd": "add_example", "name": "kpi", "example": "Court" }))]
#[case::rename(json!({ "cmd": "rename_example", "name": "kpi", "from": "Exemple", "to": "Court" }))]
#[case::remove(json!({ "cmd": "remove_example", "name": "kpi", "example": "Exemple" }))]
#[case::set(json!({ "cmd": "set_example_value", "name": "kpi", "example": null, "param": "value", "value": "12" }))]
fn example_requests_outside_templates_mode_are_refused(#[case] req: Value) {
    let (_d, mut s) = editing();
    s.close_templates().unwrap();
    let err = s.handle(serde_json::from_value::<Request>(req).unwrap()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotEditingTemplates);
}

#[test]
fn set_example_value_request_writes_the_named_example() {
    let (_d, mut s) = editing();
    s.handle(serde_json::from_value(json!({ "cmd": "add_example", "name": "kpi", "example": "Long" })).unwrap())
        .unwrap();
    let req = json!({ "cmd": "set_example_value", "name": "kpi", "example": "Long", "param": "value", "value": "999" });
    s.handle(serde_json::from_value(req).unwrap()).unwrap();
    assert_that!(value_of(&s, "kpi", Some("Long"), "value")).is_equal_to(Some(json!("999")));
}

#[test]
fn example_requests_change_the_snapshot_revision() {
    let (_d, mut s) = editing();
    let before = s.snapshot().widget_trials;
    let req = json!({ "cmd": "set_example_value", "name": "kpi", "example": null, "param": "value", "value": "12" });
    s.handle(serde_json::from_value(req).unwrap()).unwrap();
    let after = s.snapshot().widget_trials;
    assert_that!(after).is_not_equal_to(before);
    s.handle(
        serde_json::from_value(json!({ "cmd": "rename_example", "name": "kpi", "from": "Exemple", "to": "Court" }))
            .unwrap(),
    )
    .unwrap();
    assert_that!(s.snapshot().widget_trials).is_not_equal_to(after);
}

/// La révision tient dans un nombre exact de JavaScript.
#[test]
fn snapshot_revision_is_a_safe_javascript_integer() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", None, values(json!({ "value": "12" }))).unwrap();
    assert_that!(s.snapshot().widget_trials).is_less_than_or_equal_to((1u64 << 53) - 1);
}

/// L'écriture d'un cas fait passer le deck mis de côté à la version de la séance, en une étape.
#[test]
fn set_example_values_moves_the_deck_to_the_session_version_in_one_undo_step() {
    let data = fresh_dir("oku-slides-examples-");
    let mut s = session_on(data.path());
    let original = s.deck().templates.clone();
    s.edit_templates(DEFAULT_PACK).unwrap();
    s.set_widget("kpi", KPI.into()).unwrap();
    let before = s.snapshot().editing.map(|e| e.base.clone()).unwrap();
    s.set_example_values("kpi", None, values(json!({ "value": "12" }))).unwrap();
    let written = s.snapshot().editing.map(|e| e.base.clone()).unwrap();
    assert_that!(written).is_not_equal_to(&before);
    s.close_templates().unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.version.clone())).is_equal_to(Some(written));
    s.undo().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(original);
}

#[test]
fn rename_widget_carries_its_examples() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Court", None).unwrap();
    s.rename_widget("kpi", "chiffre").unwrap();
    assert_that!(stored(&s, "kpi")).is_none();
    assert_that!(names(&s, "chiffre")).is_equal_to(vec![EXAMPLE_DEFAULT.to_owned(), "Court".to_owned()]);
}

#[test]
fn remove_widget_drops_its_examples() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Court", None).unwrap();
    s.remove_widget("kpi").unwrap();
    assert_that!(stored(&s, "kpi")).is_none();
}

#[test]
fn set_widget_with_empty_source_drops_its_examples() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Court", None).unwrap();
    s.set_widget("kpi", String::new()).unwrap();
    assert_that!(stored(&s, "kpi")).is_none();
}

#[test]
fn save_templates_keeps_the_examples() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Court", None).unwrap();
    s.save_templates(false).unwrap();
    assert_that!(names(&s, "kpi")).is_equal_to(vec![EXAMPLE_DEFAULT.to_owned(), "Court".to_owned()]);
    let lib = crate::test_support::library_of(&s);
    let current = lib.current_version(DEFAULT_PACK).unwrap();
    assert_that!(current.manifest.examples.get(&parse_name("kpi").unwrap()).map(Vec::len)).is_equal_to(Some(2));
}

#[test]
fn examples_survive_leaving_and_editing_again() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Court", None).unwrap();
    s.close_templates().unwrap();
    s.edit_templates(DEFAULT_PACK).unwrap();
    assert_that!(names(&s, "kpi")).is_equal_to(vec![EXAMPLE_DEFAULT.to_owned(), "Court".to_owned()]);
}

/// Noms des cas de `widget` dans le snapshot.
fn snapshot_names(s: &Session, widget: &str) -> Option<Vec<String>> {
    s.snapshot().examples.get(widget).cloned()
}

#[test]
fn snapshot_lists_the_implicit_example_of_a_widget_without_stored_ones() {
    let (_d, s) = editing();
    assert_that!(snapshot_names(&s, "kpi")).is_equal_to(Some(vec![EXAMPLE_DEFAULT.to_owned()]));
}

#[test]
fn snapshot_lists_the_examples_in_order() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Long", None).unwrap();
    s.add_example("kpi", "Court", None).unwrap();
    assert_that!(snapshot_names(&s, "kpi")).is_equal_to(Some(vec![
        EXAMPLE_DEFAULT.to_owned(),
        "Long".to_owned(),
        "Court".to_owned(),
    ]));
}

#[test]
fn snapshot_lists_no_examples_for_common() {
    let (_d, s) = editing();
    assert_that!(snapshot_names(&s, "_commun")).is_none();
}

#[test]
fn snapshot_lists_no_examples_outside_templates_mode() {
    let (_d, mut s) = editing();
    s.add_example("kpi", "Long", None).unwrap();
    s.close_templates().unwrap();
    assert_that!(s.snapshot().examples.is_empty()).is_true();
}

/// Régression t-497 : un champ texte vidé dans un cas reste vide (cas « Vide »), au lieu de
/// reprendre le texte d'exemple de l'aperçu ; `null` remet la valeur d'aperçu.
#[test]
fn set_example_values_with_an_empty_text_keeps_it_empty() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", None, values(json!({"value": ""}))).unwrap();
    assert_that!(value_of(&s, "kpi", None, "value")).is_equal_to(Some(json!("")));
}

#[test]
fn set_example_values_with_null_resets_to_the_preview_text() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", None, values(json!({"value": ""}))).unwrap();
    s.set_example_values("kpi", None, values(json!({"value": null}))).unwrap();
    assert_that!(value_of(&s, "kpi", None, "value")).is_none();
}

#[test]
fn preview_of_an_example_with_an_emptied_field_shows_it_empty() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", None, values(json!({"value": ""}))).unwrap();
    let p = s.preview_widget("kpi", None, &Map::new(), crate::PreviewFormat::Svg).unwrap();
    // Pas le texte d'exemple : la valeur vidée, telle quelle, sur l'élément d'aperçu.
    // Lue dans les paramètres de la slide de la vue, pas dans `fields` : un champ
    // vide ne dessine rien, la mise en page n'en donne pas de boîte.
    let view = p.view.unwrap();
    assert_that!(view["slide"]["params"]["elements"][0]["params"]["value"]).is_equal_to(json!(""));
}
