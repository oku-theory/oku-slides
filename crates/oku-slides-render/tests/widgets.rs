use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{Category, Deck, ParamType, WidgetErrorKind};
use oku_slides_render::{Fonts, PackWidgets, Registries, RegistryErrorKind, Renderer, load_registry};
use speculoos::prelude::*;

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let quote = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
#let registry = (quote,)
"#;

#[test]
fn load_registry_with_pack_adds_its_widgets_to_the_base() {
    let pack = PackWidgets::new("citations", QUOTE).unwrap();
    let r = load_registry(&Fonts::default(), Some(&pack)).unwrap();
    assert_that!(r.get("quote").map(|w| w.params.len())).is_equal_to(Some(2));
    assert_that!(r.get("card")).is_some();
}

#[test]
fn load_registry_pack_breaking_card_returns_schema_error() {
    let src = r#"#import "/oku-slides/lib.typ": widget, params as p
#let registry = (widget("card", params: (title: p.plain()), (args, ctx) => []),)
"#;
    let pack = PackWidgets::new("casse", src).unwrap();
    let err = load_registry(&Fonts::default(), Some(&pack)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RegistryErrorKind::Schema);
    assert_that!(err.widget_error().map(|e| e.kind())).is_equal_to(Some(WidgetErrorKind::IncompatibleOverride));
}

#[test]
fn load_registry_pack_that_does_not_compile_returns_its_diagnostics() {
    let pack = PackWidgets::new("casse", "#let registry = (").unwrap();
    let err = load_registry(&Fonts::default(), Some(&pack)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RegistryErrorKind::Compile);
    assert_that!(err.diagnostics().iter().any(|d| d.place.as_deref().is_some_and(|p| p.contains("packs/casse"))))
        .is_true();
}

#[test]
fn pack_widgets_new_with_a_path_like_id_returns_bad_pack_id() {
    for id in ["", "../x", "A", "a/b", "é"] {
        asserting(id)
            .that(&PackWidgets::new(id, "").map(|_| ()).map_err(|e| e.kind()))
            .is_equal_to(Err(RegistryErrorKind::BadPackId));
    }
}

#[test]
fn registries_get_same_pack_twice_reuses_the_registry() {
    let mut regs = Registries::new(Fonts::default());
    let pack = PackWidgets::new("citations", QUOTE).unwrap();
    let a = regs.get(Some(&pack)).unwrap();
    let b = regs.get(Some(&pack)).unwrap();
    assert_that!(Arc::ptr_eq(&a, &b)).is_true();
    let base = regs.get(None).unwrap();
    assert_that!(Arc::ptr_eq(&a, &base)).is_false();
}

#[test]
fn registries_get_changed_pack_source_reloads() {
    let mut regs = Registries::new(Fonts::default());
    let a = regs.get(Some(&PackWidgets::new("citations", QUOTE).unwrap())).unwrap();
    let changed = QUOTE.replace("source: p.plain()", "source: p.plain(), note: p.plain()");
    let b = regs.get(Some(&PackWidgets::new("citations", changed).unwrap())).unwrap();
    assert_that!(b.get("quote").map(|w| w.params.len())).is_equal_to(Some(3));
    assert_that!(Arc::ptr_eq(&a, &b)).is_false();
}

#[test]
fn load_registry_base_declares_the_text_widget_with_a_rich_body() {
    let r = load_registry(&Fonts::default(), None).unwrap();
    let text = r.get("text").unwrap();
    assert_that!(text.category).is_equal_to(Category::Element);
    assert_that!(text.param("body").map(|p| p.ty.clone())).is_equal_to(Some(ParamType::Rich));
}

#[test]
fn load_registry_base_declares_every_element_widget() {
    let r = load_registry(&Fonts::default(), None).unwrap();
    for name in ["text", "card", "image", "shape", "chart", "stack", "canvas"] {
        asserting(name).that(&r.get(name).map(|w| w.category)).is_equal_to(Some(Category::Element));
    }
    let stack = r.get("stack").unwrap();
    assert_that!(stack.param("children").map(|p| p.ty.clone()))
        .is_equal_to(Some(ParamType::Slot { mode: oku_slides_core::SlotMode::Flow, accepts: None }));
}

#[test]
fn load_registry_base_declares_every_slide_widget() {
    let r = load_registry(&Fonts::default(), None).unwrap();
    for name in ["title", "toc", "chapter", "content", "end", "blank"] {
        asserting(name).that(&r.get(name).map(|w| w.category)).is_equal_to(Some(Category::Slide));
    }
    let title = r.get("title").unwrap();
    assert_that!(title.param("title").and_then(|p| p.bind.as_deref())).is_equal_to(Some("title"));
    assert_that!(title.param("kicker").and_then(|p| p.bind.as_deref())).is_equal_to(Some("organization"));
}

#[test]
fn load_registry_base_matches_its_snapshot() {
    let r = load_registry(&Fonts::default(), None).unwrap();
    insta::assert_json_snapshot!("base_registry", r);
}

fn roadmap() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("{path} invalide : {e}"))
}

#[test]
fn compile_card_emits_one_field_per_filled_text_param() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let fields: Vec<&str> = layout.fields.iter().filter(|f| f.owner == "c1").map(|f| f.field.as_str()).collect();
    assert_that!(fields).is_equal_to(vec!["kicker", "title", "body"]);
    let card = layout.boxes.iter().find(|b| b.id == "c1").unwrap();
    for f in layout.fields.iter().filter(|f| f.owner == "c1") {
        asserting(&format!("{f:?} dans la carte"))
            .that(&(f.x >= card.x && f.y >= card.y && f.x + f.w <= card.x + card.w + 0.01))
            .is_true();
    }
}

#[test]
fn compile_card_title_field_carries_the_card_title_style() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let title = layout.fields.iter().find(|f| f.owner == "c1" && f.field == "title").unwrap();
    let body = layout.fields.iter().find(|f| f.owner == "c1" && f.field == "body").unwrap();
    let size = |f: &oku_slides_render::FieldBox| f.style.as_ref().unwrap().size;
    assert_that!(size(title)).is_greater_than(size(body));
}

#[test]
fn compile_text_element_emits_its_body_field() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    assert_that!(layout.fields.iter().any(|f| f.owner == "b-points" && f.field == "body")).is_true();
}

#[test]
fn compile_fields_on_page_keep_only_that_page() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let page = layout.fields.iter().find(|f| f.owner == "c1").unwrap().page;
    assert_that!(layout.fields_on(page).iter().all(|f| f.page == page)).is_true();
    assert_that!(layout.fields_on(page).iter().any(|f| f.owner == "c1")).is_true();
}
