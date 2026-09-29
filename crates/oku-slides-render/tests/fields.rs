//! Champs émis par le rendu : champs vides compris, sans changer la mise en page.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::Deck;
use oku_slides_render::{FieldBox, FieldSide, Fonts, PackWidgets, Renderer, load_registry};
use serde_json::json;
use speculoos::prelude::*;

fn roadmap_json() -> serde_json::Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} illisible : {e}"));
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("{path} invalide : {e}"))
}

fn render(deck: serde_json::Value) -> Vec<FieldBox> {
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("deck invalide : {e}"));
    let mut r = Renderer::new(Fonts::default()).unwrap_or_else(|e| panic!("{e}"));
    r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}")).layout().fields.clone()
}

/// Modifie les params de l'élément `id` de la slide `slide`.
fn with_element(slide: &str, id: &str, edit: impl Fn(&mut serde_json::Value)) -> serde_json::Value {
    let mut d = roadmap_json();
    let s = d["slides"]
        .as_array_mut()
        .and_then(|a| a.iter_mut().find(|s| s["id"] == slide))
        .unwrap_or_else(|| panic!("pas de slide {slide}"));
    let e = s["params"]["blocks"]
        .as_array_mut()
        .and_then(|a| a.iter_mut().find(|e| e["id"] == id))
        .unwrap_or_else(|| panic!("pas d'élément {id}"));
    edit(&mut e["params"]);
    d
}

fn field<'a>(fields: &'a [FieldBox], owner: &str, name: &str) -> &'a FieldBox {
    fields.iter().find(|f| f.owner == owner && f.field == name).unwrap_or_else(|| panic!("pas de champ {owner}.{name}"))
}

#[test]
fn card_without_kicker_emits_an_empty_kicker_before_its_title() {
    let fields = render(with_element("s-priorites", "c1", |p| {
        p.as_object_mut().unwrap().remove("kicker");
    }));
    let kicker = field(&fields, "c1", "kicker");
    let title = field(&fields, "c1", "title");
    assert_that!(kicker.empty).is_true();
    assert_that!(kicker.side).is_equal_to(Some(FieldSide::Before));
    assert_that!(kicker.h).is_equal_to(0.0);
    assert_that!((kicker.y - title.y).abs()).is_less_than(0.01);
    assert_that!(kicker.style.as_ref().map(|s| s.size)).is_some();
}

#[test]
fn card_with_only_a_body_emits_kicker_and_title_on_one_anchor_before_it() {
    // Contrat de l'interface : deux marqueurs vides consécutifs partagent l'ancre
    // (même y, `before`) ; c'est elle qui les empile, dans l'ordre de Tab.
    let fields = render(with_element("s-priorites", "c1", |p| {
        let o = p.as_object_mut().unwrap();
        o.remove("kicker");
        o.remove("title");
    }));
    let kicker = field(&fields, "c1", "kicker");
    let title = field(&fields, "c1", "title");
    let body = field(&fields, "c1", "body");
    for f in [kicker, title] {
        asserting(&f.field).that(&f.empty).is_true();
        asserting(&f.field).that(&f.side).is_equal_to(Some(FieldSide::Before));
        asserting(&f.field).that(&f.h).is_equal_to(0.0);
    }
    assert_that!((kicker.y - title.y).abs()).is_less_than(0.01);
    assert_that!(title.y).is_less_than_or_equal_to(body.y + 0.01);
    let order: Vec<&str> = fields.iter().filter(|f| f.owner == "c1").map(|f| f.field.as_str()).collect();
    assert_that!(order).is_equal_to(vec!["kicker", "title", "body"]);
}

#[test]
fn card_filled_fields_are_not_empty() {
    let fields = render(roadmap_json());
    for name in ["kicker", "title", "body"] {
        let f = field(&fields, "c1", name);
        asserting(name).that(&f.empty).is_false();
        asserting(name).that(&f.side).is_none();
    }
}

#[test]
fn card_with_empty_body_marks_its_measured_body_empty() {
    let fields = render(with_element("s-priorites", "c1", |p| {
        p["body"] = json!([]);
    }));
    let body = field(&fields, "c1", "body");
    assert_that!(body.empty).is_true();
    assert_that!(body.side).is_none();
}

#[test]
fn text_element_with_empty_body_marks_its_body_empty() {
    let fields = render(with_element("s-constat", "b-points", |p| {
        p["body"] = json!([]);
    }));
    assert_that!(field(&fields, "b-points", "body").empty).is_true();
}

#[test]
fn fields_stack_with_empty_markers_keeps_the_stack_height() {
    // Un widget de pack : deux champs, le premier vide, dans fields-stack ; et un
    // témoin identique sans le champ vide. Leurs boîtes d'élément ont la même hauteur.
    let src = r#"#import "/oku-slides/lib.typ": widget, field, fields-stack, params as p
#let duo = widget("duo", params: (a: p.plain(), b: p.plain()),
  (args, ctx) => fields-stack(spacing: 20pt, field(ctx, "a", args.a), field(ctx, "b", args.b)))
#let solo = widget("solo", params: (b: p.plain()),
  (args, ctx) => fields-stack(spacing: 20pt, field(ctx, "b", args.b)))
#let registry = (duo, solo)
"#;
    let pack = PackWidgets::new("duo", src).unwrap();
    let registry = Arc::new(load_registry(&Fonts::default(), Some(&pack)).unwrap());
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_widgets(Some(&pack), registry).unwrap();
    let deck = json!({
        "version": 2, "params": { "title": "T" },
        "slides": [{ "id": "s", "kind": "content", "params": { "blocks": [
            { "id": "d", "kind": "duo", "params": { "b": "Texte" } },
            { "id": "o", "kind": "solo", "params": { "b": "Texte" } }
        ] } }]
    });
    let compiled = r.render(&Deck::from_json(&deck.to_string()).unwrap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let h = |id: &str| layout.boxes.iter().find(|b| b.id == id).map(|b| b.h).unwrap();
    assert_that!((h("d") - h("o")).abs()).is_less_than(0.01);
    let a = layout.fields.iter().find(|f| f.owner == "d" && f.field == "a").unwrap();
    assert_that!(a.empty).is_true();
    assert_that!(a.side).is_equal_to(Some(FieldSide::Before));
}

/// Modifie les params de la slide `slide`.
fn with_slide(slide: &str, edit: impl Fn(&mut serde_json::Value)) -> serde_json::Value {
    let mut d = roadmap_json();
    let s = d["slides"]
        .as_array_mut()
        .and_then(|a| a.iter_mut().find(|s| s["id"] == slide))
        .unwrap_or_else(|| panic!("pas de slide {slide}"));
    if s.get("params").is_none() {
        s["params"] = json!({});
    }
    edit(&mut s["params"]);
    d
}

fn has_field(fields: &[FieldBox], owner: &str, name: &str) -> bool {
    fields.iter().any(|f| f.owner == owner && f.field == name)
}

#[test]
fn card_with_blank_kicker_emits_an_empty_kicker_before_its_title() {
    let fields = render(with_element("s-priorites", "c1", |p| {
        p["kicker"] = json!("");
    }));
    let kicker = field(&fields, "c1", "kicker");
    assert_that!(kicker.empty).is_true();
    assert_that!(kicker.side).is_equal_to(Some(FieldSide::Before));
}

#[test]
fn content_slide_without_kicker_emits_an_empty_kicker() {
    let fields = render(with_slide("s-constat", |p| {
        p.as_object_mut().unwrap().remove("kicker");
    }));
    let kicker = field(&fields, "s-constat", "kicker");
    assert_that!(kicker.empty).is_true();
    assert_that!(kicker.side).is_equal_to(Some(FieldSide::Before));
}

#[test]
fn content_slide_without_title_emits_empty_kicker_and_title_above_the_body() {
    let fields = render(with_slide("s-constat", |p| {
        let o = p.as_object_mut().unwrap();
        o.remove("kicker");
        o.remove("title");
    }));
    let title = field(&fields, "s-constat", "title");
    let body = field(&fields, "b-points", "body");
    assert_that!(title.empty).is_true();
    assert_that!(title.y).is_less_than_or_equal_to(body.y);
    assert_that!(field(&fields, "s-constat", "kicker").empty).is_true();
}

#[test]
fn chapter_without_subtitle_emits_an_empty_subtitle_after_the_title() {
    let fields = render(roadmap_json()); // s-ch2 n'a pas de sous-titre
    let sub = field(&fields, "s-ch2", "subtitle");
    let title = field(&fields, "s-ch2", "title");
    assert_that!(sub.empty).is_true();
    assert_that!(sub.side).is_equal_to(Some(FieldSide::After));
    assert_that!(sub.y).is_greater_than_or_equal_to(title.y + title.h - 0.01);
    assert_that!(sub.y).is_less_than_or_equal_to(title.y + title.h + 0.01);
}

#[test]
fn chapter_with_blank_title_marks_its_rendered_title_empty() {
    let fields = render(with_slide("s-ch2", |p| {
        p["title"] = json!("");
    }));
    let title = field(&fields, "s-ch2", "title");
    assert_that!(title.empty).is_true();
    assert_that!(title.side).is_none();
}

#[test]
fn title_slide_emits_kicker_title_and_subtitle() {
    // Thème papier : couverture en mise en page `plain`, surtitre affiché.
    let fields = render(roadmap_json());
    for name in ["kicker", "title", "subtitle"] {
        asserting(name).that(&has_field(&fields, "s-title", name)).is_true();
    }
}

#[test]
fn title_slide_without_subtitle_emits_an_empty_subtitle() {
    let mut deck = roadmap_json();
    deck["params"].as_object_mut().unwrap().remove("subtitle");
    let fields = render(deck);
    assert_that!(field(&fields, "s-title", "subtitle").empty).is_true();
}

#[test]
fn title_slide_in_band_layout_emits_title_and_subtitle_without_kicker() {
    let mut deck = roadmap_json();
    deck["theme"] = json!("pro");
    let fields = render(deck);
    assert_that!(has_field(&fields, "s-title", "title")).is_true();
    assert_that!(has_field(&fields, "s-title", "subtitle")).is_true();
    assert_that!(has_field(&fields, "s-title", "kicker")).is_false();
}

#[test]
fn end_slide_with_empty_text_marks_it_empty() {
    let fields = render(with_slide("s-fin", |p| {
        p["text"] = json!("");
    }));
    assert_that!(field(&fields, "s-fin", "text").empty).is_true();
}

#[test]
fn toc_emits_its_title_field_only() {
    let fields = render(roadmap_json());
    let names: Vec<&str> = fields.iter().filter(|f| f.owner == "s-toc").map(|f| f.field.as_str()).collect();
    // Pas de paramètre `kicker` pour le sommaire : pas de champ.
    assert_that!(names).is_equal_to(vec!["title"]);
}

#[test]
fn content_slide_without_kicker_in_arabic_aligns_the_empty_kicker_like_its_title() {
    let mut deck = with_slide("s-constat", |p| {
        p.as_object_mut().unwrap().remove("kicker");
    });
    deck["params"]["lang"] = json!("ar");
    let fields = render(deck);
    let align = |name| field(&fields, "s-constat", name).style.as_ref().and_then(|s| s.align.clone());
    assert_that!(align("kicker")).is_equal_to(Some("right".to_owned()));
    assert_that!(align("title")).is_equal_to(Some("right".to_owned()));
}

#[test]
fn content_slide_without_title_in_arabic_aligns_empty_fields_right() {
    let mut deck = with_slide("s-constat", |p| {
        let o = p.as_object_mut().unwrap();
        o.remove("kicker");
        o.remove("title");
    });
    deck["params"]["lang"] = json!("ar");
    let fields = render(deck);
    let body = field(&fields, "b-points", "body");
    for name in ["kicker", "title"] {
        let f = field(&fields, "s-constat", name);
        asserting(name).that(&f.style.as_ref().and_then(|s| s.align.clone())).is_equal_to(Some("right".to_owned()));
        // Zone d'ancrage : celle du corps (même colonne, même largeur).
        asserting(name).that(&f.ax).is_equal_to(body.ax);
        asserting(name).that(&f.aw).is_equal_to(body.aw);
    }
}
