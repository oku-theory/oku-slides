use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use crate::pack::{BASE_PACK, PackManifest};
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
    s.edit_pack(BASE_PACK).unwrap();
    s.set_widget(None, "kpi", KPI.into()).unwrap();
    (data, s)
}

fn values(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap()
}

/// Les valeurs enregistrées dans le manifeste du pack en cours.
fn stored(s: &Session, widget: &str) -> Option<Map<String, Value>> {
    let t = s.widget_target(None).unwrap();
    PackManifest::from_files(&t.files).unwrap().examples.get(&parse_name(widget).unwrap()).cloned()
}

fn value_of(s: &Session, widget: &str, param: &str) -> Option<Value> {
    s.example_values(None, &parse_name(widget).unwrap()).unwrap().get(param).cloned()
}

#[test]
fn a_widget_without_stored_values_has_none() {
    let (_d, s) = editing();
    assert_that!(s.example_values(None, &parse_name("kpi").unwrap()).unwrap()).is_empty();
    assert_that!(stored(&s, "kpi")).is_none();
}

#[test]
fn set_example_values_writes_the_widget_values_in_the_pack() {
    let (_d, mut s) = editing();
    let written = s.set_example_values("kpi", values(json!({ "value": "12" }))).unwrap();
    assert_that!(stored(&s, "kpi")).is_equal_to(Some(values(json!({ "value": "12" }))));
    assert_that!(written).is_equal_to(values(json!({ "value": "12" })));
}

#[test]
fn set_example_values_keeps_the_other_values() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", values(json!({ "value": "12" }))).unwrap();
    s.set_example_values("kpi", values(json!({ "label": "Ventes" }))).unwrap();
    assert_that!(stored(&s, "kpi")).is_equal_to(Some(values(json!({ "value": "12", "label": "Ventes" }))));
}

#[test]
fn set_example_values_back_to_neutral_leaves_no_stored_values() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", values(json!({ "value": "12" }))).unwrap();
    s.set_example_values("kpi", values(json!({ "value": null }))).unwrap();
    assert_that!(stored(&s, "kpi")).is_none();
}

#[test]
fn set_example_values_outside_pack_mode_is_refused() {
    let (_d, mut s) = editing();
    s.close_pack().unwrap();
    let err = s.set_example_values("kpi", values(json!({ "value": "12" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotEditingPack);
}

#[test]
fn set_example_values_on_an_unknown_widget_is_refused() {
    let (_d, mut s) = editing();
    let err = s.set_example_values("absent", values(json!({ "value": "12" }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWidget);
}

#[test]
fn set_example_values_drops_a_stored_value_the_schema_no_longer_takes() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", values(json!({ "value": "12", "label": "Ventes" }))).unwrap();
    let v2 = KPI.replace("label: p.plain()", "label: p.number()");
    s.set_widget(None, "kpi", v2).unwrap();
    s.set_example_values("kpi", values(json!({ "value": "13" }))).unwrap();
    assert_that!(value_of(&s, "kpi", "label")).is_none();
    assert_that!(value_of(&s, "kpi", "value")).is_equal_to(Some(json!("13")));
}

#[test]
fn example_requests_outside_pack_mode_are_refused() {
    let (_d, mut s) = editing();
    s.close_pack().unwrap();
    let req = json!({ "cmd": "set_example_value", "name": "kpi", "param": "value", "value": "12" });
    let err = s.handle(serde_json::from_value::<Request>(req).unwrap()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotEditingPack);
}

#[test]
fn set_example_value_request_writes_the_widget_value() {
    let (_d, mut s) = editing();
    let req = json!({ "cmd": "set_example_value", "name": "kpi", "param": "value", "value": "999" });
    s.handle(serde_json::from_value(req).unwrap()).unwrap();
    assert_that!(value_of(&s, "kpi", "value")).is_equal_to(Some(json!("999")));
}

/// Les requêtes des cas d'exemple, retirées par t-541, ne se lisent plus.
#[rstest]
#[case::add(json!({ "cmd": "add_example", "name": "kpi", "example": "Court" }))]
#[case::rename(json!({ "cmd": "rename_example", "name": "kpi", "from": "Exemple", "to": "Court" }))]
#[case::remove(json!({ "cmd": "remove_example", "name": "kpi", "example": "Exemple" }))]
fn removed_example_case_requests_do_not_parse(#[case] req: Value) {
    assert_that!(serde_json::from_value::<Request>(req).is_err()).is_true();
}

#[test]
fn example_requests_change_the_snapshot_revision() {
    let (_d, mut s) = editing();
    let before = s.snapshot().widget_trials;
    let req = json!({ "cmd": "set_example_value", "name": "kpi", "param": "value", "value": "12" });
    s.handle(serde_json::from_value(req).unwrap()).unwrap();
    assert_that!(s.snapshot().widget_trials).is_not_equal_to(before);
}

/// La révision tient dans un nombre exact de JavaScript.
#[test]
fn snapshot_revision_is_a_safe_javascript_integer() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", values(json!({ "value": "12" }))).unwrap();
    assert_that!(s.snapshot().widget_trials).is_less_than_or_equal_to((1u64 << 53) - 1);
}

/// L'écriture des valeurs fait passer le deck mis de côté à la version de la séance, en une étape.
#[test]
fn set_example_values_moves_the_deck_to_the_session_version_in_one_undo_step() {
    let data = fresh_dir("oku-slides-examples-");
    let mut s = session_on(data.path());
    let original = crate::test_support::pin(&s, "base");
    s.edit_pack(BASE_PACK).unwrap();
    s.set_widget(None, "kpi", KPI.into()).unwrap();
    let before = s.snapshot().editing.map(|e| e.base.clone()).unwrap();
    s.set_example_values("kpi", values(json!({ "value": "12" }))).unwrap();
    let written = s.snapshot().editing.map(|e| e.base.clone()).unwrap();
    assert_that!(written).is_not_equal_to(&before);
    s.close_pack().unwrap();
    assert_that!(crate::test_support::pin(&s, "base").map(|r| r.version)).is_equal_to(Some(written));
    s.undo().unwrap();
    assert_that!(crate::test_support::pin(&s, "base")).is_equal_to(original);
}

#[test]
fn rename_widget_carries_its_values() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", values(json!({ "value": "12" }))).unwrap();
    s.rename_widget(None, "kpi", "chiffre").unwrap();
    assert_that!(stored(&s, "kpi")).is_none();
    assert_that!(stored(&s, "chiffre")).is_equal_to(Some(values(json!({ "value": "12" }))));
}

#[test]
fn remove_widget_drops_its_values() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", values(json!({ "value": "12" }))).unwrap();
    s.remove_widget(None, "kpi").unwrap();
    assert_that!(stored(&s, "kpi")).is_none();
}

#[test]
fn set_widget_with_empty_source_drops_its_values() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", values(json!({ "value": "12" }))).unwrap();
    s.set_widget(None, "kpi", String::new()).unwrap();
    assert_that!(stored(&s, "kpi")).is_none();
}

#[test]
fn save_pack_keeps_the_values() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", values(json!({ "value": "12" }))).unwrap();
    s.save_pack(false).unwrap();
    assert_that!(value_of(&s, "kpi", "value")).is_equal_to(Some(json!("12")));
    let lib = crate::test_support::library_of(&s);
    let current = lib.current_version(BASE_PACK).unwrap();
    let kept = current.manifest.examples.get(&parse_name("kpi").unwrap()).cloned();
    assert_that!(kept).is_equal_to(Some(values(json!({ "value": "12" }))));
}

#[test]
fn values_survive_leaving_and_editing_again() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", values(json!({ "value": "12" }))).unwrap();
    s.close_pack().unwrap();
    s.edit_pack(BASE_PACK).unwrap();
    assert_that!(value_of(&s, "kpi", "value")).is_equal_to(Some(json!("12")));
}

/// Régression t-497 : un champ texte vidé reste vide, au lieu de
/// reprendre le texte d'exemple de l'aperçu ; `null` remet la valeur d'aperçu.
#[test]
fn set_example_values_with_an_empty_text_keeps_it_empty() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", values(json!({"value": ""}))).unwrap();
    assert_that!(value_of(&s, "kpi", "value")).is_equal_to(Some(json!("")));
}

#[test]
fn set_example_values_with_null_resets_to_the_preview_text() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", values(json!({"value": ""}))).unwrap();
    s.set_example_values("kpi", values(json!({"value": null}))).unwrap();
    assert_that!(value_of(&s, "kpi", "value")).is_none();
}

#[test]
fn preview_with_an_emptied_field_shows_it_empty() {
    let (_d, mut s) = editing();
    s.set_example_values("kpi", values(json!({"value": ""}))).unwrap();
    let p = s.preview_widget(None, "kpi", &Map::new(), crate::PreviewFormat::Svg).unwrap();
    // Pas le texte d'exemple : la valeur vidée, telle quelle, sur l'élément d'aperçu.
    // Lue dans les paramètres de la slide de la vue, pas dans `fields` : un champ
    // vide ne dessine rien, la mise en page n'en donne pas de boîte.
    let view = p.view.unwrap();
    assert_that!(view["slide"]["params"]["elements"][0]["params"]["value"]).is_equal_to(json!(""));
}

/// Le style de départ de `widget`, écrit dans le manifeste du pack en cours.
fn seed_style(s: &mut Session, widget: &str) {
    let t = s.widget_target(None).unwrap();
    let mut manifest = PackManifest::from_files(&t.files).unwrap();
    let style = oku_slides_core::Style { fill: Some("surface".into()), ..Default::default() };
    manifest.styles.insert(parse_name(widget).unwrap(), style);
    s.commit_manifest(&t, manifest, t.files.widgets().clone()).unwrap();
}

/// Le style de départ de `widget` dans le manifeste du pack en cours.
fn stored_style(s: &Session, widget: &str) -> Option<oku_slides_core::Style> {
    let t = s.widget_target(None).unwrap();
    PackManifest::from_files(&t.files).unwrap().styles.get(&parse_name(widget).unwrap()).cloned()
}

#[test]
fn rename_widget_carries_its_style() {
    let (_d, mut s) = editing();
    seed_style(&mut s, "kpi");
    s.rename_widget(None, "kpi", "chiffre").unwrap();
    assert_that!(stored_style(&s, "kpi")).is_none();
    assert_that!(stored_style(&s, "chiffre").and_then(|st| st.fill)).is_equal_to(Some("surface".to_owned()));
}

#[test]
fn remove_widget_drops_its_style() {
    let (_d, mut s) = editing();
    seed_style(&mut s, "kpi");
    s.remove_widget(None, "kpi").unwrap();
    assert_that!(stored_style(&s, "kpi")).is_none();
}

#[test]
fn set_widget_with_empty_source_drops_its_style() {
    let (_d, mut s) = editing();
    seed_style(&mut s, "kpi");
    s.set_widget(None, "kpi", String::new()).unwrap();
    assert_that!(stored_style(&s, "kpi")).is_none();
}

#[test]
fn save_pack_keeps_the_style() {
    let (_d, mut s) = editing();
    seed_style(&mut s, "kpi");
    s.save_pack(false).unwrap();
    assert_that!(stored_style(&s, "kpi").and_then(|st| st.fill)).is_equal_to(Some("surface".to_owned()));
}
