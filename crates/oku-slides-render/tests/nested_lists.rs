//! Listes imbriquées (t-495) : le widget de test `plan`, étapes à tâches ; boîtes d'item et
//! champs de la liste imbriquée adressés `etapes/<étape>/taches/<tâche>`.

mod common;

use oku_slides_core::ParamType;
use oku_slides_render::{Fonts, ItemBox, Layout, load_registry};
use serde_json::json;
use speculoos::prelude::*;

use common::{plan_pack, render_with_pack};

fn render_plan(params: serde_json::Value) -> Layout {
    render_with_pack(&plan_pack(), json!([{ "id": "p", "kind": "plan", "params": params }]))
}

fn steps() -> serde_json::Value {
    json!({ "etapes": [
        { "id": "a", "titre": "Cadrer", "taches": [{ "id": "t1", "texte": "Objectifs" }, { "id": "t2", "texte": "Équipe", "fait": true }] },
        { "id": "b", "titre": "Livrer" }
    ] })
}

fn items_of<'a>(layout: &'a Layout, list: &str) -> Vec<&'a ItemBox> {
    layout.items.iter().filter(|i| i.owner == "p" && i.list == list).collect()
}

#[test]
fn plan_pack_declares_a_nested_list_column() {
    let r = load_registry(&Fonts::default(), Some(&plan_pack())).unwrap();
    let ParamType::List { item, .. } = &r.get("plan").unwrap().param("etapes").unwrap().ty else { panic!("liste") };
    let taches = item.iter().find(|c| c.name == "taches").unwrap();
    assert_that!(matches!(taches.ty, ParamType::List { .. })).is_true();
}

#[test]
fn nested_list_item_boxes_carry_the_nested_list_path() {
    let layout = render_plan(steps());
    let tasks: Vec<&str> = items_of(&layout, "etapes/a/taches").iter().map(|i| i.item.as_str()).collect();
    assert_that!(tasks).is_equal_to(vec!["t1", "t2"]);
    let top: Vec<&str> = items_of(&layout, "etapes").iter().map(|i| i.item.as_str()).collect();
    assert_that!(top).is_equal_to(vec!["a", "b"]);
    assert_that!(items_of(&layout, "etapes/b/taches")).is_empty();
}

#[test]
fn nested_item_box_lies_within_its_outer_item_box() {
    let layout = render_plan(steps());
    let a = items_of(&layout, "etapes")[0];
    let t2 = items_of(&layout, "etapes/a/taches")[1];
    assert_that!(t2.x).is_greater_than_or_equal_to(a.x - 0.01);
    assert_that!(t2.y + t2.h).is_less_than_or_equal_to(a.y + a.h + 0.01);
    assert_that!(t2.y).is_greater_than(items_of(&layout, "etapes/a/taches")[0].y);
}

#[test]
fn nested_item_fields_are_addressed_by_their_full_path() {
    let layout = render_plan(steps());
    let fields: Vec<&str> = layout.fields.iter().filter(|f| f.owner == "p").map(|f| f.field.as_str()).collect();
    assert_that!(fields).contains("etapes/a/titre");
    assert_that!(fields).contains("etapes/a/taches/t2/texte");
}

#[test]
fn nested_item_without_id_gets_d_index_in_its_list() {
    let layout = render_plan(json!({ "etapes": [{ "id": "a", "taches": [{ "texte": "x" }] }] }));
    assert_that!(layout.fields.iter().any(|f| f.field == "etapes/a/taches/d0/texte")).is_true();
}

#[test]
fn nested_list_of_wrong_type_renders_no_nested_item() {
    let layout = render_plan(json!({ "etapes": [{ "id": "a", "taches": "oops" }] }));
    assert_that!(items_of(&layout, "etapes")).has_length(1);
    assert_that!(layout.items.iter().filter(|i| i.list.contains("/taches")).count()).is_equal_to(0);
}
