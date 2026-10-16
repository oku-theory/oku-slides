use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::lib_tests_support::{data_dir, fixture_path, session_in, session_with_fixture, session_with_frise};

fn names(view: &crate::views::SlideView<'_>) -> Vec<String> {
    view.fields.iter().map(|f| format!("{}.{}", f.owner, f.field)).collect()
}

#[test]
fn slide_view_fields_of_cards_follow_reading_order() {
    let s = session_with_fixture();
    let v = s.slide("s-priorites").unwrap();
    assert_that!(names(&v)).is_equal_to(
        [
            "s-priorites.kicker",
            "s-priorites.title",
            "c1.kicker",
            "c1.title",
            "c1.body",
            "c2.kicker",
            "c2.title",
            "c2.body",
            "c3.kicker",
            "c3.title",
            "c3.body",
        ]
        .map(str::to_owned)
        .to_vec(),
    );
}

#[test]
fn slide_view_bound_title_field_shows_the_deck_title() {
    let s = session_with_fixture();
    let v = s.slide("s-title").unwrap();
    let title = v.fields.iter().find(|f| f.field == "title").unwrap();
    assert_that!(title.owner_kind).is_equal_to(OwnerKind::Slide);
    assert_that!(title.text.as_ref()).is_equal_to("Feuille de route {{annee}}");
}

#[test]
fn slide_view_marks_bound_fields_only() {
    let s = session_with_fixture();
    let cover = s.slide("s-title").unwrap();
    let title = cover.fields.iter().find(|f| f.field == "title").unwrap();
    assert_that!(title.bound).is_true();
    let cards = s.slide("s-priorites").unwrap();
    assert_that!(cards.fields.iter().any(|f| f.bound)).is_false();
}

#[test]
fn slide_view_bound_title_field_wins_over_the_slide_own_param() {
    // Comme le rendu (`_args`) : le paramètre lié l'emporte sur celui de la slide.
    let data = data_dir();
    let mut deck: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(fixture_path()).unwrap()).unwrap();
    let slides = deck["slides"].as_array_mut().unwrap();
    let cover = slides.iter_mut().find(|s| s["id"] == "s-title").unwrap();
    cover["params"]["title"] = json!("Titre propre à la slide");
    let path = data.0.join("cover.json");
    std::fs::write(&path, deck.to_string()).unwrap();
    let mut s = session_in(&data.0);
    s.open(&path).unwrap();
    let v = s.slide("s-title").unwrap();
    let title = v.fields.iter().find(|f| f.field == "title").unwrap();
    assert_that!(title.text.as_ref()).is_equal_to("Feuille de route {{annee}}");
}

#[test]
fn slide_view_rich_field_carries_its_markdown_text() {
    let s = session_with_fixture();
    let v = s.slide("s-constat").unwrap();
    let body = v.fields.iter().find(|f| f.owner == "b-points").unwrap();
    assert_that!(body.kind).is_equal_to(FieldKind::Rich);
    assert_that!(body.text.as_ref()).is_not_equal_to("");
}

#[test]
fn slide_view_empty_subtitle_is_listed_empty() {
    let s = session_with_fixture();
    let v = s.slide("s-ch2").unwrap();
    let sub = v.fields.iter().find(|f| f.field == "subtitle").unwrap();
    assert_that!(sub.empty).is_true();
    assert_that!(sub.text.as_ref()).is_equal_to("");
}

#[test]
fn slide_view_targets_are_element_boxes_only() {
    let s = session_with_fixture();
    let v = s.slide("s-libre").unwrap();
    let shape = v.targets.iter().find(|t| t.id == "e-fond").unwrap();
    assert_that!(shape.kind).is_equal_to("shape");
    assert_that!(shape.free).is_true();
    assert_that!(v.targets.iter().any(|t| t.id.starts_with('@'))).is_false();
}

#[test]
fn slide_view_element_plain_field_shows_its_param_text() {
    let s = session_with_fixture();
    let v = s.slide("s-priorites").unwrap();
    let title = v.fields.iter().find(|f| f.owner == "c1" && f.field == "title").unwrap();
    let card = s.deck().slide("s-priorites").and_then(|sl| sl.locate("c1")).unwrap().element;
    assert_that!(title.kind).is_equal_to(FieldKind::Plain);
    assert_that!(Some(title.text.as_ref())).is_equal_to(card.params.text("title"));
    assert_that!(title.text.as_ref()).is_not_equal_to("");
}

#[test]
fn field_views_rich_slide_param_carries_its_markdown_text() {
    // Un widget de slide (de pack) peut déclarer un paramètre `rich`.
    let registry = WidgetRegistry::from_probe(json!({ "base": [{
        "name": "note", "label": null, "category": "slide",
        "params": [{ "name": "body", "label": null, "type": "rich", "default": null, "bind": null }]
    }] }))
    .unwrap();
    let deck: Deck = serde_json::from_value(json!({ "slides": [{
        "id": "s-1", "kind": "note", "params": { "body": [{ "runs": [{ "t": "Une note" }] }] }
    }] }))
    .unwrap();
    let slide = deck.slide("s-1").unwrap();
    let boxes: Vec<FieldBox> = serde_json::from_value(json!([{
        "owner": "s-1", "field": "body", "page": 1, "x": 0.0, "y": 0.0, "w": 100.0, "h": 20.0
    }]))
    .unwrap();
    let views = field_views(&deck, slide, &registry, &boxes);
    let body = views.iter().find(|f| f.field == "body").unwrap();
    assert_that!(body.kind).is_equal_to(FieldKind::Rich);
    assert_that!(body.owner_kind).is_equal_to(OwnerKind::Slide);
    assert_that!(body.text.as_ref()).is_equal_to("Une note");
}

#[test]
fn field_views_of_a_frise_carry_list_item_and_column() {
    let s = session_with_frise();
    let view = s.slide("s-constat").unwrap();
    let f = view.fields.iter().find(|f| f.field == "etapes/d1/texte").unwrap();
    assert_that!((f.list.as_deref(), f.item, f.column)).is_equal_to((Some("etapes"), Some("d1"), "texte"));
    assert_that!(f.text.as_ref()).is_equal_to("Premier client");
    assert_that!((f.owner, f.owner_kind)).is_equal_to(("fr", OwnerKind::Element));
}

#[test]
fn field_views_of_a_plain_param_have_no_list_nor_item() {
    let s = session_with_fixture();
    let v = s.slide("s-priorites").unwrap();
    let f = v.fields.iter().find(|f| f.owner == "c1" && f.field == "title").unwrap();
    assert_that!((f.list.as_deref(), f.item, f.column)).is_equal_to((None, None, "title"));
}

#[test]
fn field_views_of_a_freshly_inserted_frise_step_include_its_empty_date() {
    let mut s = session_with_frise();
    let insert = serde_json::from_value(serde_json::json!({
        "op": "edit_list", "target": { "slide": "s-constat", "element": "fr" }, "param": "etapes",
        "change": { "op": "insert", "index": 0, "id": "n" }
    }))
    .unwrap();
    s.apply("insérer", vec![insert], None).unwrap();
    let view = s.slide("s-constat").unwrap();
    let f = view.fields.iter().find(|f| f.field == "etapes/n/date").unwrap();
    assert_that!(f.empty).is_true();
    assert_that!(f.w).is_greater_than(0.0);
}

#[test]
fn field_views_of_a_nested_item_carry_the_nested_list_path() {
    let s = crate::lib_tests_support::session_with_plan();
    let v = s.slide("s-constat").unwrap();
    let f = v.fields.iter().find(|f| f.owner == "pl" && f.field == "etapes/d0/taches/d1/texte").unwrap();
    assert_that!((f.list.as_deref(), f.item, f.column)).is_equal_to((Some("etapes/d0/taches"), Some("d1"), "texte"));
    assert_that!(f.text.as_ref()).is_equal_to("Équipe");
}
