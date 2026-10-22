use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use oku_slides_core::{ListOp, Slide};

use crate::test_support::{fresh_dir, opened_on, set_frise_widget};
use crate::widget_preview::PreviewFormat;
use crate::{Session, SessionErrorKind};

/// Une session en édition des modèles du deck, avec la frise de test.
fn editing() -> (tempfile::TempDir, Session) {
    let data = fresh_dir("oku-slides-preview-items-");
    let mut s = opened_on(data.path());
    let set = crate::BASE_PACK;
    s.edit_pack(set).unwrap();
    set_frise_widget(&mut s);
    (data, s)
}

/// La vue de l'aperçu de la frise, à ses valeurs d'exemple.
fn preview_view(s: &mut Session) -> Value {
    let preview = s.preview_widget(None, "frise", &Map::new(), PreviewFormat::Svg).unwrap();
    assert_that!(preview.diagnostics.iter().any(|d| d.error)).is_false();
    preview.view.unwrap()
}

fn strings(view: &Value, key: &str, field: &str) -> Vec<String> {
    view[key].as_array().unwrap().iter().map(|v| v[field].as_str().unwrap().to_owned()).collect()
}

fn boxes(items: &Value) -> Vec<[f64; 4]> {
    items.as_array().unwrap().iter().map(|i| ["x", "y", "w", "h"].map(|k| i[k].as_f64().unwrap())).collect()
}

fn insert(id: &str, index: usize) -> ListOp {
    serde_json::from_value(json!({ "op": "insert", "index": index, "id": id })).unwrap()
}

#[test]
fn preview_items_of_a_list_widget_give_its_items_and_list() {
    let (_d, mut s) = editing();
    let view = preview_view(&mut s);
    assert_that!(strings(&view, "items", "item")).is_equal_to(vec![
        "d0".to_owned(),
        "d1".to_owned(),
        "d2".to_owned(),
        "d3".to_owned(),
    ]);
    assert_that!(strings(&view, "items", "list")).is_equal_to(vec!["etapes".to_owned(); 4]);
    assert_that!(strings(&view, "items", "owner")).is_equal_to(vec!["apercu-element".to_owned(); 4]);
    assert_that!(strings(&view, "lists", "list")).is_equal_to(vec!["etapes".to_owned()]);
    assert_that!(view["lists"][0]["len"]).is_equal_to(json!(4));
    assert_that!(view["lists"][0]["max"]).is_equal_to(json!(8));
}

#[test]
fn preview_items_have_the_boxes_of_the_same_element_on_a_slide() {
    let (_d, mut s) = editing();
    let grid = Slide::new("x", oku_slides_core::kinds::essentiel::BLANK).resolved_grid(&s.registry);
    let slide = json!({ "id": "t-frise", "kind": oku_slides_core::kinds::essentiel::BLANK, "params": { "elements": [{
            "id": "apercu-element", "kind": "base/frise",
            "cell": { "col": 0, "row": 0, "cols": grid.columns, "rows": grid.rows },
            "params": { "etapes": [
                { "id": "d0", "date": "2019", "texte": "Création" },
                { "id": "d1", "date": "2021", "texte": "Premier client" },
                { "id": "d2", "date": "2023", "texte": "International" },
                { "id": "d3", "date": "2025", "texte": "Série B" }
            ] }
        }] } });
    crate::test_support::host_slide(&mut s, slide);
    let on_slide = serde_json::to_value(s.slide("t-frise").unwrap().items).unwrap();
    let view = preview_view(&mut s);
    assert_that!(boxes(&view["items"])).is_equal_to(boxes(&on_slide));
}

#[test]
fn preview_items_of_an_emptied_list_keep_the_list_without_items() {
    let (_d, mut s) = editing();
    let mut values = Map::new();
    values.insert("etapes".into(), json!([]));
    s.set_example_values("frise", values).unwrap();
    let view = preview_view(&mut s);
    assert_that!(view["items"]).is_equal_to(json!([]));
    assert_that!(strings(&view, "lists", "list")).is_equal_to(vec!["etapes".to_owned()]);
    assert_that!(view["lists"][0]["len"]).is_equal_to(json!(0));
}

#[test]
fn preview_items_follow_the_example_values() {
    let (_d, mut s) = editing();
    assert_that!(preview_view(&mut s)["lists"][0]["len"]).is_equal_to(json!(4));
    s.edit_example_list("frise", "etapes", insert("neuf", 0)).unwrap();
    let view = preview_view(&mut s);
    assert_that!(strings(&view, "items", "item").first().cloned()).is_equal_to(Some("neuf".to_owned()));
    assert_that!(view["lists"][0]["len"]).is_equal_to(json!(5));
}

#[test]
fn preview_list_edit_never_changes_the_deck() {
    // Hors mode pack : refusé, la frise du deck reste telle quelle.
    let mut s = crate::lib_tests_support::session_with_frise();
    let before = s.deck().clone();
    let err = s.edit_example_list("frise", "etapes", insert("neuf", 0)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::NotEditingPack);
    assert_that!(s.deck()).is_equal_to(&before);
    // En mode pack : seules les valeurs d'exemple changent, ni les modèles ni le deck mis de côté.
    let (_d, mut s) = editing();
    let (templates, user) = (s.deck().slides.clone(), s.user_deck().slides.clone());
    s.edit_example_list("frise", "etapes", insert("neuf", 0)).unwrap();
    assert_that!(s.deck().slides).is_equal_to(templates);
    assert_that!(s.user_deck().slides).is_equal_to(user);
}

#[test]
fn preview_items_of_common_are_refused() {
    let (_d, mut s) = editing();
    let err = s.edit_example_list("_commun", "etapes", insert("neuf", 0)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWidget);
}
