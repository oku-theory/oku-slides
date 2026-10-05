use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{Category, Deck, ParamType, WidgetErrorKind, WidgetFile};
use oku_slides_render::{Fonts, PackWidgets, Registries, RegistryErrorKind, Renderer, load_registry};
use speculoos::prelude::*;

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

/// Les fichiers de widgets d'un pack, par nom.
fn files(entries: &[(&str, &str)]) -> BTreeMap<WidgetFile, String> {
    entries
        .iter()
        .map(|(n, s)| (WidgetFile::parse(n).unwrap_or_else(|| panic!("nom invalide : {n}")), (*s).to_owned()))
        .collect()
}

#[test]
fn load_registry_with_pack_adds_its_widgets_to_the_base() {
    let pack = PackWidgets::new("citations", files(&[("quote", QUOTE)])).unwrap();
    let r = load_registry(&Fonts::default(), Some(&pack)).unwrap();
    assert_that!(r.get("quote").map(|w| w.params.len())).is_equal_to(Some(2));
    assert_that!(r.get("card")).is_some();
}

#[test]
fn load_registry_pack_breaking_card_returns_schema_error() {
    let src = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("card", params: (title: p.plain()), (args, ctx) => [])
"#;
    let pack = PackWidgets::new("casse", files(&[("card", src)])).unwrap();
    let err = load_registry(&Fonts::default(), Some(&pack)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RegistryErrorKind::Schema);
    assert_that!(err.widget_error().map(|e| e.kind())).is_equal_to(Some(WidgetErrorKind::IncompatibleOverride));
}

#[test]
fn load_registry_pack_that_does_not_compile_returns_its_diagnostics() {
    let pack = PackWidgets::new("casse", files(&[("casse", "#let widget = (")])).unwrap();
    let err = load_registry(&Fonts::default(), Some(&pack)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RegistryErrorKind::Compile);
    assert_that!(
        err.diagnostics()
            .iter()
            .any(|d| d.place.as_deref().is_some_and(|p| p.contains("packs/casse/widgets/casse.typ")))
    )
    .is_true();
}

#[test]
fn pack_widgets_new_with_a_path_like_id_returns_bad_pack_id() {
    for id in ["", "../x", "A", "a/b", "é"] {
        asserting(id)
            .that(&PackWidgets::new(id, BTreeMap::new()).map(|_| ()).map_err(|e| e.kind()))
            .is_equal_to(Err(RegistryErrorKind::BadPackId));
    }
}

#[test]
fn registries_get_without_pack_is_the_renderer_base_registry() {
    let base = Registries::new(Fonts::default()).get(None).unwrap();
    let renderer = Renderer::new(Fonts::default()).unwrap();
    assert_that!(Arc::ptr_eq(&base, renderer.registry())).is_true();
}

#[test]
fn registries_get_same_pack_twice_reuses_the_registry() {
    let mut regs = Registries::new(Fonts::default());
    let pack = PackWidgets::new("citations", files(&[("quote", QUOTE)])).unwrap();
    let a = regs.get(Some(&pack)).unwrap();
    let b = regs.get(Some(&pack)).unwrap();
    assert_that!(Arc::ptr_eq(&a, &b)).is_true();
    let base = regs.get(None).unwrap();
    assert_that!(Arc::ptr_eq(&a, &base)).is_false();
}

#[test]
fn registries_get_changed_pack_source_reloads() {
    let mut regs = Registries::new(Fonts::default());
    let a = regs.get(Some(&PackWidgets::new("citations", files(&[("quote", QUOTE)])).unwrap())).unwrap();
    let changed = QUOTE.replace("source: p.plain()", "source: p.plain(), note: p.plain()");
    let b = regs.get(Some(&PackWidgets::new("citations", files(&[("quote", &changed)])).unwrap())).unwrap();
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

#[test]
fn base_registry_fixture_matches_the_live_registry() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
    let text = std::fs::read_to_string(path).unwrap();
    let fixture = oku_slides_core::WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap()).unwrap();
    assert_that!(load_registry(&Fonts::default(), None).unwrap()).is_equal_to(fixture);
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

#[test]
fn compile_title_slide_emits_kicker_title_and_subtitle_fields_owned_by_the_slide() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let fields: Vec<&str> = layout.fields.iter().filter(|f| f.owner == "s-title").map(|f| f.field.as_str()).collect();
    assert_that!(fields).is_equal_to(vec!["kicker", "title", "subtitle"]);
    let page = layout.page_of("s-title").unwrap();
    assert_that!(layout.fields.iter().filter(|f| f.owner == "s-title").all(|f| f.page == page)).is_true();
    // Les anciennes boîtes « @champ » ont disparu.
    assert_that!(layout.boxes.iter().any(|b| b.id.starts_with('@'))).is_false();
}

#[test]
fn compile_content_slide_emits_kicker_and_title_fields() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let fields: Vec<&str> = layout.fields.iter().filter(|f| f.owner == "s-constat").map(|f| f.field.as_str()).collect();
    assert_that!(fields).is_equal_to(vec!["kicker", "title"]);
}

/// Deck v3 d'une slide libre avec un seul élément `kind`, sur un cadre.
fn one_element_deck(kind: &str, params: serde_json::Value) -> Deck {
    let json = serde_json::json!({ "version": 3, "slides": [{ "id": "s", "kind": "blank", "params": {
        "elements": [{ "id": "e", "kind": kind, "frame": { "x": 40, "y": 40, "w": 600, "h": 300 }, "params": params }]
    } }] });
    Deck::from_json(&json.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn compile_pack_widget_renders_in_the_deck() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let pack = PackWidgets::new("citations", files(&[("quote", QUOTE)])).unwrap();
    let registry = Arc::new(load_registry(&Fonts::default(), Some(&pack)).unwrap());
    r.set_widgets(Some(&pack), registry.clone()).unwrap();
    assert_that!(Arc::ptr_eq(r.registry(), &registry)).is_true();
    let deck = one_element_deck(
        "quote",
        serde_json::json!({ "body": [{ "runs": [{ "t": "Tout coule." }] }], "source": "Héraclite" }),
    );
    let compiled = r.render(&deck, &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    assert_that!(layout.boxes.iter().any(|b| b.id == "e" && b.w > 0.0)).is_true();
    assert_that!(layout.fields.iter().any(|f| f.owner == "e" && f.field == "body")).is_true();
}

const RED_CARD: &str = r##"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("card",
  params: (kicker: p.plain(), title: p.plain(), body: p.rich(), highlight: p.flag(), icon: p.plain()),
  (args, ctx) => block(fill: rgb("#ff0000"), width: 100%, height: 100%, inset: 12pt,
    field(ctx, "title", text(args.title)) + args.body))
"##;

#[test]
fn compile_pack_override_of_card_replaces_the_base_render() {
    let deck =
        one_element_deck("card", serde_json::json!({ "title": "Rouge", "body": [{ "runs": [{ "t": "corps" }] }] }));
    let mut base = Renderer::new(Fonts::default()).unwrap();
    let plain = base.render(&deck, &BTreeMap::new()).unwrap();

    let mut r = Renderer::new(Fonts::default()).unwrap();
    let pack = PackWidgets::new("rouge", files(&[("card", RED_CARD)])).unwrap();
    let registry = Arc::new(load_registry(&Fonts::default(), Some(&pack)).unwrap());
    assert_that!(registry.get("card").and_then(|w| w.param("icon"))).is_some();
    r.set_widgets(Some(&pack), registry).unwrap();
    let red = r.render(&deck, &BTreeMap::new()).unwrap();

    // Le crate `png` n'est pas une dépendance du workspace : le fond rouge se
    // lit dans le SVG, et le PNG diffère de celui du rendu de base.
    assert_that!(red.svg(0).unwrap().to_lowercase()).contains("#ff0000");
    assert_that!(plain.svg(0).unwrap().to_lowercase()).does_not_contain("#ff0000");
    assert_that!(red.png(0, 0.5).unwrap()).is_not_equal_to(plain.png(0, 0.5).unwrap());
    let layout = red.layout();
    let card = layout.boxes.iter().find(|b| b.id == "e").unwrap();
    let title = layout.fields.iter().find(|f| f.owner == "e" && f.field == "title").unwrap();
    assert_that!(title.x >= card.x && title.y >= card.y && title.x + title.w <= card.x + card.w + 0.01).is_true();
}

#[test]
fn compile_unknown_element_kind_draws_the_unknown_box() {
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&one_element_deck("nope", serde_json::json!({})), &BTreeMap::new()).unwrap();
    assert_that!(compiled.page_count()).is_equal_to(1);
    assert_that!(compiled.layout().boxes.iter().any(|b| b.id == "e")).is_true();
}

#[test]
fn set_widgets_back_to_the_base_drops_the_pack_override() {
    let deck = one_element_deck("card", serde_json::json!({ "title": "Rouge" }));
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let base = r.registry().clone();
    let pack = PackWidgets::new("rouge", files(&[("card", RED_CARD)])).unwrap();
    r.set_widgets(Some(&pack), Arc::new(load_registry(&Fonts::default(), Some(&pack)).unwrap())).unwrap();
    assert_that!(r.render(&deck, &BTreeMap::new()).unwrap().svg(0).unwrap().to_lowercase()).contains("#ff0000");
    r.set_widgets(None, base).unwrap();
    let svg = r.render(&deck, &BTreeMap::new()).unwrap().svg(0).unwrap().to_lowercase();
    assert_that!(svg).does_not_contain("#ff0000");
}

#[test]
fn compile_unknown_slide_kind_renders_a_page_marked_with_the_slide() {
    let json = serde_json::json!({ "version": 3, "slides": [{ "id": "s", "kind": "nope" }] });
    let deck = Deck::from_json(&json.to_string()).unwrap();
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&deck, &BTreeMap::new()).unwrap();
    assert_that!(compiled.page_count()).is_equal_to(1);
    assert_that!(compiled.layout().page_of("s")).is_equal_to(Some(1));
}

/// Élément `kind` sur un cadre, avec des paramètres donnés tels quels.
fn framed(id: &str, kind: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "id": id, "kind": kind, "frame": { "x": 40, "y": 40, "w": 400, "h": 200 }, "params": params })
}

#[test]
fn compile_deck_with_wrong_typed_params_renders_every_page() {
    // Un deck incohérent (écrit sans passer par les ops) s'ouvre et se rend :
    // chaque valeur qui n'a pas le type déclaré retombe sur le défaut.
    let json = serde_json::json!({ "version": 3, "slides": [
        { "id": "s-blank", "kind": "blank", "params": { "grid": "x", "elements": [
            framed("card", "card", serde_json::json!({ "title": 3, "kicker": ["a"], "body": "texte", "highlight": "oui" })),
            framed("stack", "stack", serde_json::json!({ "count": "trois", "layout": 7, "gap": "énorme", "children": "x" })),
            framed("canvas", "canvas", serde_json::json!({ "children": { "a": 1 } })),
            framed("image", "image", serde_json::json!({ "asset": 42, "fit": "blob" })),
            framed("shape", "shape", serde_json::json!({ "shape": false })),
            framed("text", "text", serde_json::json!({ "body": [{ "x": 1 }] })),
            framed("text2", "text", serde_json::json!({ "body": { "runs": [] } })),
            framed("text3", "text", serde_json::json!({ "body": [{ "runs": "a" }] })),
            framed("text4", "text", serde_json::json!({ "body": [{ "runs": [1] }] })),
            framed("chart", "chart", serde_json::json!({ "option": "x" })),
        ] } },
        { "id": "s-content", "kind": "content",
          "params": { "layout": 5, "title": ["t"], "kicker": { "a": 1 }, "grid": [1], "blocks": "x", "elements": 3 } },
        { "id": "s-chapter", "kind": "chapter", "params": { "title": 12, "subtitle": true } },
        { "id": "s-end", "kind": "end", "params": { "text": true } },
        { "id": "s-toc", "kind": "toc", "params": { "title": 1 } },
    ] });
    let deck = Deck::from_json(&json.to_string()).unwrap_or_else(|e| panic!("{e}"));
    let mut r = Renderer::new(Fonts::default()).unwrap();
    let compiled = r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("rendu refusé : {e:?}"));
    assert_that!(compiled.page_count()).is_equal_to(5);
    for page in 0..compiled.page_count() {
        asserting(&format!("page {page}")).that(&compiled.png(page, 0.25).is_ok()).is_true();
    }
}
