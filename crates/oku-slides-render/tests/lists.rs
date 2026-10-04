//! Paramètres `list` : items rendus par `list-item`, boîtes d'item et champs adressés à l'item.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{Deck, ParamType, WidgetFile};
use oku_slides_render::{Fonts, Layout, PackWidgets, Renderer, load_registry};
use serde_json::json;
use speculoos::prelude::*;

const FRISE: &str = r#"#import "/oku-slides/lib.typ": widget, field, list-item, params as p
#let widget = widget("mini", params: (etapes: p.list(item: (date: p.plain(), texte: p.plain()),
    default: ((date: "2019", texte: "A"),))),
  (args, ctx) => stack(dir: ltr, spacing: 10pt, ..args.etapes.map(e => list-item(ctx, "etapes", e,
    c => stack(field(c, "date", e.date), field(c, "texte", e.texte))))))
"#;

/// Rend un deck d'une slide `content` dont l'unique bloc est `element`, avec le
/// widget de pack `mini` défini par `src`.
fn render_pack_deck(src: &str, element: serde_json::Value) -> Layout {
    let name = WidgetFile::parse("mini").unwrap_or_else(|| panic!("nom de widget invalide"));
    let pack = PackWidgets::new("mini", BTreeMap::from([(name, src.to_owned())])).unwrap_or_else(|e| panic!("{e}"));
    let registry = Arc::new(load_registry(&Fonts::default(), Some(&pack)).unwrap_or_else(|e| panic!("{e}")));
    let mut r = Renderer::new(Fonts::default()).unwrap_or_else(|e| panic!("{e}"));
    r.set_widgets(Some(&pack), registry).unwrap_or_else(|e| panic!("{e}"));
    let deck = json!({
        "version": 3, "params": { "title": "T" },
        "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [element] } }]
    });
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("deck invalide : {e}"));
    r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}")).layout().clone()
}

#[test]
fn list_item_emits_item_boxes_and_prefixed_fields() {
    let layout = render_pack_deck(
        FRISE,
        json!({ "id": "m", "kind": "mini", "params": { "etapes": [
        { "id": "a", "date": "2019", "texte": "Un" }, { "id": "b", "date": "2021", "texte": "Deux" } ] } }),
    );
    let items: Vec<&str> = layout.items.iter().filter(|i| i.owner == "m").map(|i| i.item.as_str()).collect();
    assert_that!(items).is_equal_to(vec!["a", "b"]);
    assert_that!(layout.items.iter().all(|i| i.list == "etapes" && i.page == 1)).is_true();
    assert_that!(layout.fields.iter().any(|f| f.owner == "m" && f.field == "etapes/b/texte")).is_true();
    let a = layout.items.iter().find(|i| i.item == "a").unwrap();
    let b = layout.items.iter().find(|i| i.item == "b").unwrap();
    assert_that!(b.x).is_greater_than(a.x);
    assert_that!(layout.items_on(1).len()).is_equal_to(2);
    assert_that!(layout.items_on(2).len()).is_equal_to(0);
}

#[test]
fn list_absent_renders_no_item() {
    let layout = render_pack_deck(FRISE, json!({ "id": "m", "kind": "mini" }));
    assert_that!(layout.items.iter().filter(|i| i.owner == "m").count()).is_equal_to(0);
}

#[test]
fn list_of_wrong_type_renders_no_item() {
    let layout = render_pack_deck(FRISE, json!({ "id": "m", "kind": "mini", "params": { "etapes": "oops" } }));
    assert_that!(layout.items.iter().filter(|i| i.owner == "m").count()).is_equal_to(0);
}

#[test]
fn list_item_without_id_gets_d_index() {
    let layout =
        render_pack_deck(FRISE, json!({ "id": "m", "kind": "mini", "params": { "etapes": [{ "date": "x" }] } }));
    assert_that!(layout.items.iter().any(|i| i.item == "d0")).is_true();
    assert_that!(layout.fields.iter().any(|f| f.field == "etapes/d0/date")).is_true();
}

#[test]
fn list_item_with_non_string_id_gets_d_index() {
    let layout = render_pack_deck(
        FRISE,
        json!({ "id": "m", "kind": "mini", "params": { "etapes": [
        { "id": "a", "date": "x" }, { "id": 7, "date": "y" } ] } }),
    );
    let items: Vec<&str> = layout.items.iter().map(|i| i.item.as_str()).collect();
    assert_that!(items).is_equal_to(vec!["a", "d1"]);
}

#[test]
fn base_registry_has_frise_with_a_list_param() {
    let r = load_registry(&Fonts::default(), None).unwrap();
    assert_that!(matches!(r.get("frise").unwrap().param("etapes").unwrap().ty, ParamType::List { .. })).is_true();
}

#[test]
fn base_frise_renders_one_item_box_per_step_covering_its_column() {
    let layout = render_base_deck(json!([
        { "id": "f", "kind": "frise", "params": { "etapes": [
            { "id": "a", "date": "2019", "texte": "Création" },
            { "id": "b", "date": "2021", "texte": "Premier client" },
            { "id": "c", "date": "2023", "texte": "International" } ] } }
    ]));
    let items: Vec<_> = layout.items.iter().filter(|i| i.owner == "f").collect();
    assert_that!(items.iter().map(|i| i.item.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["a", "b", "c"]);
    assert_that!(items.windows(2).all(|w| w[1].x > w[0].x + w[0].w - 0.01)).is_true();
    // Les colonnes couvrent la largeur de l'élément : chaque boîte couvre son étape centrée.
    let el = layout.boxes.iter().find(|b| b.id == "f").unwrap();
    let (first, last) = (items[0], items[items.len() - 1]);
    assert_that!((first.x - el.x).abs()).is_less_than(0.01);
    assert_that!((last.x + last.w - (el.x + el.w)).abs()).is_less_than(0.01);
    assert_that!(layout.fields.iter().any(|f| f.owner == "f" && f.field == "etapes/c/texte")).is_true();
}

/// Rend un deck dont les blocs de l'unique slide `content` sont `blocks`, avec les widgets de base.
fn render_base_deck(blocks: serde_json::Value) -> Layout {
    let mut r = Renderer::new(Fonts::default()).unwrap_or_else(|e| panic!("{e}"));
    let deck = json!({
        "version": 3, "params": { "title": "T" },
        "slides": [{ "id": "s", "kind": "content", "params": { "blocks": blocks } }]
    });
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("deck invalide : {e}"));
    r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}")).layout().clone()
}

#[test]
fn base_frise_without_steps_keeps_the_height_of_one_row() {
    let layout = render_base_deck(json!([
        { "id": "vide", "kind": "frise" },
        { "id": "une", "kind": "frise", "params": { "etapes": [{ "id": "a", "date": "2019", "texte": "Création" }] } }
    ]));
    let h = |id: &str| layout.boxes.iter().find(|b| b.id == id).map(|b| b.h).unwrap();
    assert_that!(h("vide")).is_greater_than(0.0);
    assert_that!((h("vide") - h("une")).abs()).is_less_than(1.0);
    assert_that!(layout.items.iter().filter(|i| i.owner == "vide").count()).is_equal_to(0);
    assert_that!(layout.fields.iter().filter(|f| f.owner == "vide").count()).is_equal_to(0);
}

#[test]
fn list_item_called_with_an_integer_id_uses_it_as_text() {
    let src = r#"#import "/oku-slides/lib.typ": widget, field, list-item, params as p
#let widget = widget("mini", params: (texte: p.plain()),
  (args, ctx) => list-item(ctx, "etapes", (id: 7), c => field(c, "texte", "Sept")))
"#;
    let layout = render_pack_deck(src, json!({ "id": "m", "kind": "mini" }));
    let items: Vec<&str> = layout.items.iter().map(|i| i.item.as_str()).collect();
    assert_that!(items).is_equal_to(vec!["7"]);
    assert_that!(layout.fields.iter().any(|f| f.owner == "m" && f.field == "etapes/7/texte")).is_true();
}

#[test]
fn base_frise_empty_item_field_is_measured_and_flagged_empty() {
    let layout = render_base_deck(json!([
        { "id": "f", "kind": "frise", "params": { "etapes": [
            { "id": "a", "date": "2019", "texte": "Création" },
            { "id": "n", "date": "", "texte": "Nouvelle étape" } ] } }
    ]));
    let date = layout.fields.iter().find(|f| f.owner == "f" && f.field == "etapes/n/date");
    assert_that!(date).is_some();
    let date = date.unwrap();
    assert_that!(date.empty).is_true();
    assert_that!(date.w).is_greater_than(0.0);
    assert_that!(date.h).is_greater_than(0.0);
    let full = layout.fields.iter().find(|f| f.field == "etapes/a/date").unwrap();
    assert_that!(full.empty).is_false();
}
