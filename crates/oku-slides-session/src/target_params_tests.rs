use serde_json::{Value, json};
use speculoos::prelude::*;

use oku_slides_core::{ListOp, ParamValue};

use crate::lib_tests_support::{session_with_fixture, session_with_frise};
use crate::{Session, SessionErrorKind};

/// Widget de pack des tests : un paramètre lié au deck, un libre, un avec défaut,
/// un nombre sans bornes, un curseur et un asset.
const MIXTE: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("mixte", label: "Mixte", params: (
  titre: p.plain(bind: "title"), note: p.plain(), ton: p.plain(default: "doux"),
  taille: p.number(min: 0), force: p.number(min: 0, max: 10), logo: p.asset()),
  (args, ctx) => [#args.ton])
"#;

/// Le JSON de la surface (messages A2UI), pour y chercher composants et valeurs.
fn surface_json(s: &Session, slide: &str, element: Option<&str>) -> Option<Value> {
    s.target_settings(slide, element).unwrap().map(|x| serde_json::to_value(x).unwrap())
}

fn has(v: &Value, needle: &str) -> bool {
    v.to_string().contains(needle)
}

/// Clés, valeurs et notes des réglages de la cible.
fn resolved(s: &Session, slide: &str, element: Option<&str>) -> (Vec<String>, Vec<Value>, Vec<String>) {
    let r = s.resolve_target(slide, element).unwrap().unwrap();
    let values = r.params.spec.fields.iter().map(|f| f.value.clone()).collect();
    (r.params.keys, values, r.params.notes)
}

/// Le titre du panneau et les libellés de ses champs.
fn names(s: &Session, slide: &str, element: Option<&str>) -> (String, Vec<String>) {
    let r = s.resolve_target(slide, element).unwrap().unwrap();
    (r.params.spec.title.clone(), r.params.spec.fields.iter().map(|f| f.label.clone()).collect())
}

/// La valeur du champ `key`.
fn value_of(s: &Session, slide: &str, element: Option<&str>, key: &str) -> Value {
    let (keys, values, _) = resolved(s, slide, element);
    let i = keys.iter().position(|k| k == key).unwrap_or_else(|| panic!("pas de champ {key}"));
    values[i].clone()
}

/// Paramètre `name` de la slide, ou de l'élément `element`, dans le deck.
fn param(s: &Session, slide: &str, element: Option<&str>, name: &str) -> Option<Value> {
    let sl = s.deck().slide(slide).unwrap();
    let params = match element {
        Some(id) => &sl.locate(id).unwrap().element.params,
        None => &sl.params,
    };
    params.get(name).map(|v| serde_json::to_value(v).unwrap())
}

/// Les ids des items de la frise `fr`.
fn item_ids(s: &Session) -> Vec<String> {
    match s.deck().slide("s-constat").unwrap().locate("fr").unwrap().element.params.get("etapes") {
        Some(ParamValue::List(items)) => items.iter().map(|i| i.id.clone()).collect(),
        None => Vec::new(),
        Some(other) => panic!("etapes n'est pas une liste : {other:?}"),
    }
}

fn add_element(s: &mut Session, id: &str, kind: &str) {
    let op = serde_json::from_value(json!({
        "op": "add_element",
        "slot": { "owner": { "slide": "s-constat" }, "param": "blocks" },
        "element": { "id": id, "kind": kind }
    }))
    .unwrap();
    s.apply(kind, vec![op], None).unwrap();
}

/// Le fixture, avec le widget `mixte` et un élément `mx` sur `s-constat`.
fn session_with_mixte() -> Session {
    let mut s = session_with_fixture();
    s.set_widget("mixte", MIXTE.into()).unwrap_or_else(|e| panic!("mixte : {}", e.one_line()));
    add_element(&mut s, "mx", "mixte");
    s
}

/// Retire la liste `etapes` de la frise (`set_param` à `null`).
fn remove_etapes(s: &mut Session) {
    let op = serde_json::from_value(json!({
        "op": "set_param", "target": { "slide": "s-constat", "element": "fr" }, "param": "etapes", "value": null
    }))
    .unwrap();
    s.apply("vide", vec![op], None).unwrap();
}

fn insert(id: &str) -> ListOp {
    ListOp::Insert { index: None, id: id.into(), values: serde_json::Map::new() }
}

#[test]
fn target_settings_selected_frise_shows_its_list_table_with_deck_rows() {
    let mut s = session_with_frise();
    s.target_param("s-constat", Some("fr"), "etapes", json!([{ "id": "a", "date": "1999", "texte": "Début" }]))
        .unwrap();
    let v = surface_json(&s, "s-constat", Some("fr")).unwrap();
    assert_that!(has(&v, "\"host-target\"")).is_true();
    assert_that!(has(&v, "ListTable")).is_true();
    let rows = value_of(&s, "s-constat", Some("fr"), "etapes");
    assert_that!(rows).is_equal_to(json!([{ "id": "a", "date": "1999", "texte": "Début" }]));
}

#[test]
fn target_settings_absent_list_shows_an_empty_table() {
    let mut s = session_with_frise();
    remove_etapes(&mut s);
    assert_that!(param(&s, "s-constat", Some("fr"), "etapes")).is_none();
    assert_that!(value_of(&s, "s-constat", Some("fr"), "etapes")).is_equal_to(json!([]));
}

#[test]
fn target_settings_nothing_selected_uses_the_slide_widget_fields() {
    let s = session_with_fixture();
    let v = surface_json(&s, "s-ch1", None).unwrap();
    assert_that!(has(&v, "\"host-target\"")).is_true();
    let (keys, values, _) = resolved(&s, "s-ch1", None);
    assert_that!(keys).is_equal_to(vec!["title".to_owned(), "subtitle".to_owned()]);
    assert_that!(values).is_equal_to(vec![json!("Contexte"), json!("Où en est {{client}} aujourd'hui")]);
}

#[test]
fn target_settings_built_in_chapter_has_localized_title_and_field_names() {
    let s = session_with_fixture();
    let (title, labels) = names(&s, "s-ch1", None);
    assert_that!(title).is_equal_to("Paramètres de Chapitre".to_owned());
    assert_that!(labels).is_equal_to(vec!["Titre".to_owned(), "Sous-titre".to_owned()]);
}

#[test]
fn target_settings_built_in_content_has_localized_title_and_field_names() {
    let s = session_with_fixture();
    let (title, labels) = names(&s, "s-constat", None);
    assert_that!(title).is_equal_to("Paramètres de Contenu".to_owned());
    assert_that!(labels).is_equal_to(vec!["Surtitre".to_owned(), "Titre".to_owned(), "Disposition".to_owned()]);
}

#[test]
fn target_settings_pack_widget_keeps_its_label_and_raw_names_without_key() {
    let s = session_with_mixte();
    let (title, labels) = names(&s, "s-constat", Some("mx"));
    assert_that!(title).is_equal_to("Paramètres de Mixte".to_owned());
    assert_that!(labels.contains(&"note".to_owned())).is_true();
}

#[test]
fn target_settings_value_is_the_deck_value_else_the_schema_default_never_a_placeholder() {
    let mut s = session_with_mixte();
    assert_that!(value_of(&s, "s-constat", Some("mx"), "note")).is_equal_to(Value::Null);
    assert_that!(value_of(&s, "s-constat", Some("mx"), "ton")).is_equal_to(json!("doux"));
    let placeholder = s.localizer.tr("widget-preview-plain");
    let v = surface_json(&s, "s-constat", Some("mx")).unwrap();
    assert_that!(has(&v, &placeholder)).is_false();
    s.target_param("s-constat", Some("mx"), "ton", json!("vif")).unwrap();
    assert_that!(value_of(&s, "s-constat", Some("mx"), "ton")).is_equal_to(json!("vif"));
}

#[test]
fn target_settings_only_bound_params_returns_none() {
    let s = session_with_fixture();
    assert_that!(s.target_settings("s-title", None).unwrap()).is_none();
}

#[test]
fn target_settings_bound_param_next_to_fields_becomes_a_note() {
    let s = session_with_mixte();
    let (keys, _, notes) = resolved(&s, "s-constat", Some("mx"));
    assert_that!(keys.contains(&"titre".to_owned())).is_false();
    assert_that!(keys.contains(&"note".to_owned())).is_true();
    assert_that!(notes).is_equal_to(vec!["titre suit la variable du deck « title »".to_owned()]);
}

#[test]
fn target_settings_asset_param_is_omitted() {
    let s = session_with_fixture();
    let (keys, _, notes) = resolved(&s, "s-libre", Some("e-image"));
    assert_that!(keys).is_equal_to(vec!["fit".to_owned()]);
    assert_that!(notes).is_empty();
}

#[test]
fn target_settings_removed_element_returns_none() {
    let s = session_with_fixture();
    assert_that!(s.target_settings("s-constat", Some("absent")).unwrap()).is_none();
}

#[test]
fn target_settings_unknown_slide_returns_none() {
    let s = session_with_fixture();
    assert_that!(s.target_settings("absente", None).unwrap()).is_none();
}

#[test]
fn target_settings_in_history_returns_none() {
    let mut s = session_with_fixture();
    s.history_open().unwrap();
    assert_that!(s.target_settings("s-ch1", None).unwrap()).is_none();
}

#[test]
fn target_settings_in_help_returns_none() {
    let mut s = session_with_fixture();
    s.open_help("en", None).unwrap();
    let ids: Vec<String> = s.deck().slides.iter().map(|sl| sl.id.clone()).collect();
    let shown: Vec<_> = ids.iter().filter(|id| s.target_settings(id, None).unwrap().is_some()).collect();
    assert_that!(shown).is_empty();
}

#[test]
fn target_settings_template_slide_in_templates_mode_shows_fields() {
    let mut s = session_with_fixture();
    let set = s.deck().templates.clone().unwrap().id;
    s.edit_templates(set).unwrap();
    let slide = s.deck().slides.iter().find(|sl| sl.kind == "content").map(|sl| sl.id.clone()).unwrap();
    let v = surface_json(&s, &slide, None).unwrap();
    assert_that!(has(&v, "\"host-target\"")).is_true();
    let (keys, _, _) = resolved(&s, &slide, None);
    assert_that!(keys.contains(&"title".to_owned())).is_true();
}

#[test]
fn target_param_plain_sets_param_in_one_named_undo_step() {
    let mut s = session_with_fixture();
    s.target_param("s-ch1", None, "title", json!("Nouveau")).unwrap();
    assert_that!(param(&s, "s-ch1", None, "title")).is_equal_to(Some(json!("Nouveau")));
    assert_that!(s.snapshot().undo_label).is_equal_to(Some("Réglage : Titre".to_owned()));
    s.undo().unwrap();
    assert_that!(param(&s, "s-ch1", None, "title")).is_equal_to(Some(json!("Contexte")));
}

#[test]
fn target_param_two_edits_of_the_same_plain_field_are_two_undo_steps() {
    let mut s = session_with_fixture();
    s.target_param("s-ch1", None, "title", json!("A")).unwrap();
    s.target_param("s-ch1", None, "title", json!("AB")).unwrap();
    s.undo().unwrap();
    assert_that!(param(&s, "s-ch1", None, "title")).is_equal_to(Some(json!("A")));
}

#[test]
fn target_param_two_edits_of_the_same_choice_field_are_two_undo_steps() {
    let mut s = session_with_fixture();
    s.target_param("s-constat", None, "layout", json!("columns")).unwrap();
    s.target_param("s-constat", None, "layout", json!("center")).unwrap();
    s.undo().unwrap();
    assert_that!(param(&s, "s-constat", None, "layout")).is_equal_to(Some(json!("columns")));
}

#[test]
fn target_param_two_edits_of_the_same_slider_merge_into_one_undo_step() {
    let mut s = session_with_mixte();
    s.target_param("s-constat", Some("mx"), "force", json!(3)).unwrap();
    s.target_param("s-constat", Some("mx"), "force", json!(7)).unwrap();
    assert_that!(param(&s, "s-constat", Some("mx"), "force")).is_equal_to(Some(json!(7)));
    s.undo().unwrap();
    assert_that!(param(&s, "s-constat", Some("mx"), "force")).is_none();
}

#[test]
fn target_param_two_list_replacements_are_two_undo_steps() {
    let mut s = session_with_frise();
    s.target_param("s-constat", Some("fr"), "etapes", json!([{ "id": "x" }])).unwrap();
    s.target_param("s-constat", Some("fr"), "etapes", json!([{ "id": "y" }])).unwrap();
    s.undo().unwrap();
    assert_that!(item_ids(&s)).is_equal_to(vec!["x".to_owned()]);
}

#[test]
fn target_param_edits_of_two_fields_are_two_undo_steps() {
    let mut s = session_with_fixture();
    s.target_param("s-ch1", None, "title", json!("A")).unwrap();
    s.target_param("s-ch1", None, "subtitle", json!("B")).unwrap();
    s.undo().unwrap();
    assert_that!(param(&s, "s-ch1", None, "title")).is_equal_to(Some(json!("A")));
}

#[test]
fn target_param_flag_text_becomes_bool() {
    let mut s = session_with_fixture();
    s.target_param("s-priorites", Some("c3"), "highlight", json!("false")).unwrap();
    assert_that!(param(&s, "s-priorites", Some("c3"), "highlight")).is_equal_to(Some(json!(false)));
}

#[test]
fn target_param_number_text_becomes_number() {
    let mut s = session_with_mixte();
    s.target_param("s-constat", Some("mx"), "taille", json!("12.5")).unwrap();
    assert_that!(param(&s, "s-constat", Some("mx"), "taille")).is_equal_to(Some(json!(12.5)));
}

#[test]
fn target_param_blank_text_removes_the_param() {
    let mut s = session_with_fixture();
    s.target_param("s-ch1", None, "subtitle", json!("  ")).unwrap();
    assert_that!(param(&s, "s-ch1", None, "subtitle")).is_none();
}

#[test]
fn target_param_invalid_value_is_refused_by_apply_and_deck_unchanged() {
    let mut s = session_with_fixture();
    let before = s.deck().clone();
    let err = s.target_param("s-constat", None, "layout", json!("zigzag")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Op);
    assert_that!(s.deck().clone()).is_equal_to(before);
}

#[test]
fn target_param_list_items_replace_the_list() {
    let mut s = session_with_frise();
    s.target_param("s-constat", Some("fr"), "etapes", json!([{ "id": "x", "date": "1990" }])).unwrap();
    assert_that!(item_ids(&s)).is_equal_to(vec!["x".to_owned()]);
}

#[test]
fn target_param_list_with_non_array_is_host_key_and_list_kept() {
    let mut s = session_with_frise();
    let before = item_ids(&s);
    for value in [json!(null), json!({ "op": "replace" })] {
        let err = s.target_param("s-constat", Some("fr"), "etapes", value).unwrap_err();
        assert_that!(err.kind()).is_equal_to(SessionErrorKind::HostKey);
    }
    assert_that!(item_ids(&s)).is_equal_to(before);
}

#[test]
fn target_param_bound_key_is_host_key() {
    let mut s = session_with_mixte();
    let err = s.target_param("s-constat", Some("mx"), "titre", json!("x")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::HostKey);
}

#[test]
fn target_param_unsettable_key_is_host_key() {
    let mut s = session_with_mixte();
    let err = s.target_param("s-constat", Some("mx"), "logo", json!("sha256:0000")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::HostKey);
}

#[test]
fn target_param_unknown_key_is_host_key() {
    let mut s = session_with_fixture();
    let err = s.target_param("s-ch1", None, "couleur", json!("x")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::HostKey);
}

#[test]
fn target_param_unknown_element_is_host_key() {
    let mut s = session_with_fixture();
    let err = s.target_param("s-constat", Some("absent"), "title", json!("x")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::HostKey);
}

#[test]
fn target_param_unknown_slide_is_no_slide() {
    let mut s = session_with_fixture();
    let err = s.target_param("absente", None, "title", json!("x")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NoSlide);
}

#[test]
fn target_param_in_history_is_in_history() {
    let mut s = session_with_fixture();
    s.history_open().unwrap();
    let err = s.target_param("s-ch1", None, "title", json!("x")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InHistory);
}

#[test]
fn target_list_insert_appends_an_item() {
    let mut s = session_with_frise();
    let mut values = serde_json::Map::new();
    values.insert("date".into(), json!("2030"));
    s.target_list("s-constat", Some("fr"), "etapes", ListOp::Insert { index: None, id: "n1".into(), values }).unwrap();
    assert_that!(item_ids(&s).last().cloned()).is_equal_to(Some("n1".to_owned()));
    assert_that!(item_ids(&s).len()).is_equal_to(5);
    assert_that!(s.snapshot().undo_label).is_equal_to(Some("Réglage : Étapes".to_owned()));
}

#[test]
fn target_list_edits_are_separate_undo_steps() {
    let mut s = session_with_frise();
    s.target_list("s-constat", Some("fr"), "etapes", insert("n1")).unwrap();
    s.target_list("s-constat", Some("fr"), "etapes", insert("n2")).unwrap();
    s.undo().unwrap();
    assert_that!(item_ids(&s).last().cloned()).is_equal_to(Some("n1".to_owned()));
}

#[test]
fn target_list_move_remove_set_field_apply() {
    let mut s = session_with_frise();
    let ids = item_ids(&s);
    let (first, second) = (ids[0].clone(), ids[1].clone());
    s.target_list("s-constat", Some("fr"), "etapes", ListOp::Move { item: first.clone(), to: 1 }).unwrap();
    assert_that!(item_ids(&s)[..2].to_vec()).is_equal_to(vec![second.clone(), first.clone()]);
    s.target_list("s-constat", Some("fr"), "etapes", ListOp::Remove { item: second.clone() }).unwrap();
    assert_that!(item_ids(&s).contains(&second)).is_false();
    let change = ListOp::SetField { item: first.clone(), field: "date".into(), value: json!("1789") };
    s.target_list("s-constat", Some("fr"), "etapes", change).unwrap();
    let rows = value_of(&s, "s-constat", Some("fr"), "etapes");
    assert_that!(rows[0]["date"].clone()).is_equal_to(json!("1789"));
}

#[test]
fn target_list_on_an_absent_list_inserts_into_an_empty_list() {
    let mut s = session_with_frise();
    remove_etapes(&mut s);
    s.target_list("s-constat", Some("fr"), "etapes", insert("n1")).unwrap();
    assert_that!(item_ids(&s)).is_equal_to(vec!["n1".to_owned()]);
}

#[test]
fn target_list_beyond_max_is_refused() {
    let mut s = session_with_frise();
    for i in item_ids(&s).len()..8 {
        s.target_list("s-constat", Some("fr"), "etapes", insert(&format!("n{i}"))).unwrap();
    }
    let err = s.target_list("s-constat", Some("fr"), "etapes", insert("trop")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Op);
    assert_that!(item_ids(&s).len()).is_equal_to(8);
}

#[test]
fn target_list_on_a_plain_key_is_refused_by_apply() {
    let mut s = session_with_fixture();
    let err = s.target_list("s-ch1", None, "title", insert("n1")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Op);
}
