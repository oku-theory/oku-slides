use std::collections::BTreeMap;
use std::sync::Arc;

mod common;

use oku_slides_core::{
    Category, DECK_VERSION, Deck, Namespace, PackKind, ParamType, WidgetFile,
    kinds::{base, essentiel},
};
use oku_slides_render::{
    BuiltinPack, Fonts, Layout, PackSet, PackWidgets, Registries, RegistryErrorKind, Renderer, assemble, base_probe,
    builtin_pack_set, builtin_registry, load_registry, probe_file, probe_registry,
};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

/// L'ensemble d'un seul pack `id`, d'espace de noms `id`, aux fichiers `entries`.
fn one_pack(id: &str, entries: &[(&str, &str)]) -> PackSet {
    PackSet::new(vec![common::pack_in(id, id, entries)]).unwrap_or_else(|e| panic!("{e}"))
}

/// L'ensemble du seul pack `base` intégré.
fn base_set() -> PackSet {
    PackSet::new(vec![BuiltinPack::Base.widgets().unwrap_or_else(|e| panic!("{e}"))]).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn load_registry_with_pack_adds_its_qualified_widgets_to_the_base() {
    let set = PackSet::new(vec![
        BuiltinPack::Base.widgets().unwrap(),
        common::pack_in("citations", "citations", &[("quote", QUOTE)]),
    ])
    .unwrap();
    let r = load_registry(&Fonts::default(), &set).unwrap();
    assert_that!(r.get("citations/quote").map(|w| w.params.len())).is_equal_to(Some(2));
    assert_that!(r.get("quote")).is_none();
    assert_that!(r.get(base::CARD)).is_some();
}

#[test]
fn load_registry_pack_that_does_not_compile_returns_its_diagnostics() {
    let err = load_registry(&Fonts::default(), &one_pack("casse", &[("casse", "#let widget = (")])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RegistryErrorKind::Compile);
    assert_that!(
        err.diagnostics()
            .iter()
            .any(|d| d.place.as_deref().is_some_and(|p| p.contains("packs/casse/widgets/casse.typ")))
    )
    .is_true();
}

/// Un `label` ou une `description` qui n'est pas un texte écarte ce seul fichier, avec un
/// message qui nomme l'argument (et non une erreur serde de tout le registre).
#[rstest]
#[case::description_number("description: 42", "la description doit être un texte")]
#[case::description_content("description: [Une valeur]", "la description doit être un texte")]
#[case::label_number("label: 42", "le label doit être un texte")]
#[case::label_content("label: [KPI]", "le label doit être un texte")]
fn probe_file_with_a_non_text_label_or_description_is_set_aside(#[case] arg: &str, #[case] named: &str) {
    let src = format!(
        "#import \"/oku-slides/lib.typ\": widget, params as p\n#let widget = widget(\"kpi\", {arg}, params: (:), (args, ctx) => [])\n"
    );
    let probe = probe_file(&Fonts::default(), PackKind::Widgets, &WidgetFile::parse("kpi").unwrap(), &src, None);
    assert_that!(probe.is_valid()).is_false();
    assert_that!(probe.diagnostics.iter().any(|d| d.message.contains(named))).is_true();
}

/// Un texte est gardé tel quel ; un label ou une description vide (ou d'espaces) vaut `none`.
#[rstest]
#[case::description_text("description: \"Une valeur\"", "description", json!("Une valeur"))]
#[case::description_blank("description: \"   \"", "description", Value::Null)]
#[case::description_empty("description: \"\"", "description", Value::Null)]
#[case::label_text("label: \"KPI\"", "label", json!("KPI"))]
#[case::label_blank("label: \"  \"", "label", Value::Null)]
fn probe_file_text_label_or_description_is_kept_or_blank_becomes_none(
    #[case] arg: &str,
    #[case] field: &str,
    #[case] read: Value,
) {
    let src = format!(
        "#import \"/oku-slides/lib.typ\": widget, params as p\n#let widget = widget(\"kpi\", {arg}, params: (:), (args, ctx) => [])\n"
    );
    let probe = probe_file(&Fonts::default(), PackKind::Widgets, &WidgetFile::parse("kpi").unwrap(), &src, None);
    assert_that!(probe.is_valid()).is_true();
    assert_that!(probe.schema.as_ref().and_then(|s| s.get(field)).cloned()).is_equal_to(Some(read));
}

#[test]
fn pack_widgets_new_with_a_path_like_id_returns_bad_pack_id() {
    for id in ["", "../x", "A", "a/b", "é"] {
        asserting(id)
            .that(
                &PackWidgets::new(id, Namespace::deck(), PackKind::Widgets, BTreeMap::new())
                    .map(|_| ())
                    .map_err(|e| e.kind()),
            )
            .is_equal_to(Err(RegistryErrorKind::BadPackId));
    }
}

#[test]
fn registries_get_without_pack_is_the_renderer_empty_registry() {
    let base = Registries::new(Fonts::default()).get(&PackSet::default()).unwrap();
    let renderer = Renderer::new(Fonts::default()).unwrap();
    assert_that!(Arc::ptr_eq(&base, renderer.registry())).is_true();
}

#[test]
fn registries_get_same_pack_twice_reuses_the_registry() {
    let mut regs = Registries::new(Fonts::default());
    let set = one_pack("citations", &[("quote", QUOTE)]);
    let a = regs.get(&set).unwrap();
    let b = regs.get(&set).unwrap();
    assert_that!(Arc::ptr_eq(&a, &b)).is_true();
    let base = regs.get(&PackSet::default()).unwrap();
    assert_that!(Arc::ptr_eq(&a, &base)).is_false();
}

#[test]
fn registries_get_changed_pack_source_reloads() {
    let mut regs = Registries::new(Fonts::default());
    let a = regs.get(&one_pack("citations", &[("quote", QUOTE)])).unwrap();
    let changed = QUOTE.replace("source: p.plain()", "source: p.plain(), note: p.plain()");
    let b = regs.get(&one_pack("citations", &[("quote", &changed)])).unwrap();
    assert_that!(b.get("citations/quote").map(|w| w.params.len())).is_equal_to(Some(3));
    assert_that!(Arc::ptr_eq(&a, &b)).is_false();
}

#[test]
fn load_registry_base_declares_the_text_widget_with_a_rich_body() {
    let r = load_registry(&Fonts::default(), &base_set()).unwrap();
    let text = r.get(base::TEXT).unwrap();
    assert_that!(text.category).is_equal_to(Category::Element);
    assert_that!(text.param("body").map(|p| p.ty.clone())).is_equal_to(Some(ParamType::Rich));
}

#[test]
fn load_registry_base_declares_every_element_widget() {
    let r = load_registry(&Fonts::default(), &base_set()).unwrap();
    for name in base::ALL {
        asserting(name).that(&r.get(name).map(|w| w.category)).is_equal_to(Some(Category::Element));
    }
    let stack = r.get(base::STACK).unwrap();
    assert_that!(stack.param("children").map(|p| p.ty.clone()))
        .is_equal_to(Some(ParamType::Slot { mode: oku_slides_core::SlotMode::Flow, accepts: None }));
}

#[test]
fn load_registry_essentiel_declares_every_slide_layout() {
    let set = PackSet::new(vec![BuiltinPack::Essentiel.widgets().unwrap()]).unwrap();
    let r = load_registry(&Fonts::default(), &set).unwrap();
    for name in essentiel::ALL {
        asserting(name).that(&r.get(name).map(|w| w.category)).is_equal_to(Some(Category::Slide));
    }
    let title = r.get(essentiel::TITLE).unwrap();
    assert_that!(title.param("title").and_then(|p| p.bind.as_deref())).is_equal_to(Some("title"));
    assert_that!(title.param("kicker").and_then(|p| p.bind.as_deref())).is_equal_to(Some("organization"));
}

#[rstest]
#[case(BuiltinPack::Base, "pack_base")]
#[case(BuiltinPack::Essentiel, "pack_essentiel")]
fn builtin_pack_registry_snapshot(#[case] pack: BuiltinPack, #[case] name: &str) {
    let registry = load_registry(&Fonts::default(), &PackSet::new(vec![pack.widgets().unwrap()]).unwrap()).unwrap();
    insta::assert_json_snapshot!(name, registry);
}

#[test]
fn builtin_registry_fixture_matches_the_live_registry() {
    let live = probe_registry(&Fonts::default(), &builtin_pack_set().unwrap()).unwrap();
    let fixture: Value = serde_json::from_str(include_str!("../../../fixtures/widgets/builtin-registry.json")).unwrap();
    assert_that!(live).is_equal_to(fixture);
}

#[test]
fn a_layouts_pack_widget_declaring_element_is_dropped_with_a_diagnostic() {
    let src = "#import \"/oku-slides/lib.typ\": widget\n#let widget = widget(\"w\", category: \"element\", params: (:), (args, ctx) => [])\n";
    let w = WidgetFile::parse("w").unwrap();
    let fonts = Fonts::default();
    let mut files = BTreeMap::from([(w.clone(), probe_file(&fonts, PackKind::Layouts, &w, src, None))]);
    let acme = Namespace::parse("acme").unwrap();
    let registry = assemble(&base_probe(&fonts).unwrap(), &acme, PackKind::Layouts, &mut files).unwrap();
    assert_that!(files[&w].diagnostics.iter().any(|d| d.message.contains("category"))).is_true();
    assert_that!(registry.get("acme/w")).is_none();
}

#[test]
fn render_with_no_packs_draws_the_unknown_slide_frame() {
    let json =
        json!({"version": DECK_VERSION, "slides": [{"id": "s", "kind": essentiel::TITLE, "params": {"title": "T"}}]});
    let deck = Deck::from_json(&json.to_string()).unwrap();
    let bare = Renderer::new(Fonts::default()).unwrap().render(&deck, &BTreeMap::new()).unwrap();
    let full = common::renderer().render(&deck, &BTreeMap::new()).unwrap();
    // Sans pack, une page « slide inconnue » sans champ ; avec `essentiel`, la couverture et ses champs.
    let fields = |c: &oku_slides_render::Compiled| c.layout().fields.iter().any(|f| f.owner == "s");
    assert_that!((bare.page_count(), fields(&bare), fields(&full))).is_equal_to((1, false, true));
}

fn roadmap() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("{path} invalide : {e}"))
}

#[test]
fn compile_card_emits_one_field_per_filled_text_param() {
    let mut r = common::renderer();
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
    let mut r = common::renderer();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let title = layout.fields.iter().find(|f| f.owner == "c1" && f.field == "title").unwrap();
    let body = layout.fields.iter().find(|f| f.owner == "c1" && f.field == "body").unwrap();
    let size = |f: &oku_slides_render::FieldBox| f.style.as_ref().unwrap().size;
    assert_that!(size(title)).is_greater_than(size(body));
}

#[test]
fn compile_text_element_emits_its_body_field() {
    let mut r = common::renderer();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    assert_that!(layout.fields.iter().any(|f| f.owner == "b-points" && f.field == "body")).is_true();
}

#[test]
fn compile_fields_on_page_keep_only_that_page() {
    let mut r = common::renderer();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let page = layout.fields.iter().find(|f| f.owner == "c1").unwrap().page;
    assert_that!(layout.fields_on(page).iter().all(|f| f.page == page)).is_true();
    assert_that!(layout.fields_on(page).iter().any(|f| f.owner == "c1")).is_true();
}

#[test]
fn compile_title_slide_emits_kicker_title_and_subtitle_fields_owned_by_the_slide() {
    let mut r = common::renderer();
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
    let mut r = common::renderer();
    let compiled = r.render(&roadmap(), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    let fields: Vec<&str> = layout.fields.iter().filter(|f| f.owner == "s-constat").map(|f| f.field.as_str()).collect();
    assert_that!(fields).is_equal_to(vec!["kicker", "title"]);
}

/// Deck d'une slide libre avec un seul élément `kind`, sur un cadre.
fn one_element_deck(kind: &str, params: serde_json::Value) -> Deck {
    let json = serde_json::json!({ "version": DECK_VERSION, "slides": [{ "id": "s", "kind": essentiel::BLANK, "params": {
        "elements": [{ "id": "e", "kind": kind, "frame": { "x": 40, "y": 40, "w": 600, "h": 300 }, "params": params }]
    } }] });
    Deck::from_json(&json.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn compile_pack_widget_renders_in_the_deck() {
    let mut r = common::renderer_with(&one_pack("citations", &[("quote", QUOTE)]));
    assert_that!(r.registry().get("citations/quote")).is_some();
    let deck = one_element_deck(
        "citations/quote",
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
fn compile_pack_card_renders_under_its_qualified_kind_beside_the_base_card() {
    let params = serde_json::json!({ "title": "Rouge", "body": [{ "runs": [{ "t": "corps" }] }] });
    let mut r = common::renderer_with(&one_pack("rouge", &[("card", RED_CARD)]));
    let registry = r.registry().clone();
    assert_that!(registry.get("rouge/card").and_then(|w| w.param("icon"))).is_some();
    assert_that!(registry.get(base::CARD).map(|w| w.param("icon").is_none())).is_equal_to(Some(true));
    let plain = r.render(&one_element_deck(base::CARD, params.clone()), &BTreeMap::new()).unwrap();
    let red = r.render(&one_element_deck("rouge/card", params), &BTreeMap::new()).unwrap();

    // Le crate `png` n'est pas une dépendance du workspace : le fond rouge se
    // lit dans le SVG, et le PNG diffère de celui de la carte de `base`.
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
    let mut r = common::renderer();
    let compiled = r.render(&one_element_deck("nope", serde_json::json!({})), &BTreeMap::new()).unwrap();
    assert_that!(compiled.page_count()).is_equal_to(1);
    assert_that!(compiled.layout().boxes.iter().any(|b| b.id == "e")).is_true();
}

#[test]
fn set_widgets_back_to_the_builtins_drops_the_pack_widgets() {
    let deck = one_element_deck("rouge/card", serde_json::json!({ "title": "Rouge" }));
    let mut r = common::renderer_with(&one_pack("rouge", &[("card", RED_CARD)]));
    assert_that!(r.render(&deck, &BTreeMap::new()).unwrap().svg(0).unwrap().to_lowercase()).contains("#ff0000");
    r.set_widgets(&builtin_pack_set().unwrap(), builtin_registry(&Fonts::default()).unwrap()).unwrap();
    let svg = r.render(&deck, &BTreeMap::new()).unwrap().svg(0).unwrap().to_lowercase();
    assert_that!(svg).does_not_contain("#ff0000");
}

#[test]
fn compile_unknown_slide_kind_renders_a_page_marked_with_the_slide() {
    let json = serde_json::json!({ "version": DECK_VERSION, "slides": [{ "id": "s", "kind": "nope" }] });
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
    let json = serde_json::json!({ "version": DECK_VERSION, "slides": [
        { "id": "s-blank", "kind": essentiel::BLANK, "params": { "grid": "x", "elements": [
            framed("card", base::CARD, serde_json::json!({ "title": 3, "kicker": ["a"], "body": "texte", "highlight": "oui" })),
            framed("stack", base::STACK, serde_json::json!({ "count": "trois", "layout": 7, "gap": "énorme", "children": "x" })),
            framed("image", base::IMAGE, serde_json::json!({ "asset": 42, "fit": "blob" })),
            framed("shape", base::SHAPE, serde_json::json!({ "shape": false })),
            framed("text", base::TEXT, serde_json::json!({ "body": [{ "x": 1 }] })),
            framed("text2", base::TEXT, serde_json::json!({ "body": { "runs": [] } })),
            framed("text3", base::TEXT, serde_json::json!({ "body": [{ "runs": "a" }] })),
            framed("text4", base::TEXT, serde_json::json!({ "body": [{ "runs": [1] }] })),
            framed("chart", base::CHART, serde_json::json!({ "option": "x" })),
        ] } },
        { "id": "s-content", "kind": essentiel::CONTENT,
          "params": { "layout": 5, "title": ["t"], "kicker": { "a": 1 }, "grid": [1], "blocks": "x", "elements": 3 } },
        { "id": "s-chapter", "kind": essentiel::CHAPTER, "params": { "title": 12, "subtitle": true } },
        { "id": "s-end", "kind": essentiel::END, "params": { "text": true } },
        { "id": "s-toc", "kind": essentiel::TOC, "params": { "title": 1 } },
    ] });
    let deck = Deck::from_json(&json.to_string()).unwrap_or_else(|e| panic!("{e}"));
    let mut r = common::renderer();
    let compiled = r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("rendu refusé : {e:?}"));
    assert_that!(compiled.page_count()).is_equal_to(5);
    for page in 0..compiled.page_count() {
        asserting(&format!("page {page}")).that(&compiled.png(page, 0.25).is_ok()).is_true();
    }
}

/// Deck d'une slide libre dont les éléments `(id, kind)` sont posés côte à côte.
fn elements_deck(elements: &[(&str, &str)]) -> Deck {
    let elements: Vec<serde_json::Value> = elements
        .iter()
        .enumerate()
        .map(|(i, (id, kind))| {
            serde_json::json!({ "id": id, "kind": kind, "params": { "title": "T" },
                "frame": { "x": 40 + 700 * i, "y": 40, "w": 600, "h": 300 } })
        })
        .collect();
    let json = serde_json::json!({ "version": DECK_VERSION, "slides": [{ "id": "s", "kind": essentiel::BLANK, "params": { "elements": elements } }] });
    Deck::from_json(&json.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn render_deck_with_two_packs_renders_each_qualified_widget() {
    let set = PackSet::new(vec![
        common::pack_in("a1", "acme", &[("card", RED_CARD)]),
        common::pack_in("b1", "beta", &[("card", RED_CARD)]),
    ])
    .unwrap();
    let mut r = common::renderer_with(&set);
    let compiled = r.render(&elements_deck(&[("a", "acme/card"), ("b", "beta/card")]), &BTreeMap::new()).unwrap();
    let layout = compiled.layout();
    for id in ["a", "b"] {
        asserting(id).that(&layout.boxes.iter().any(|b| b.id == id && b.w > 0.0)).is_true();
        asserting(id).that(&layout.fields.iter().any(|f| f.owner == id && f.field == "title")).is_true();
    }
}

/// Un widget de test qui remplit sa hauteur : un rectangle de 100 %.
const TALL: &str = r#"#import "/oku-slides/lib.typ": widget
#let widget = widget("tall", grow: true, (args, ctx) => rect(width: 100%, height: 100%))"#;
/// Le même contenu de 40 pt, avec ou sans `grow`.
const FLAT: &str = r#"#import "/oku-slides/lib.typ": widget
#let widget = widget("flat", (args, ctx) => block(width: 100%, height: 40pt))"#;
/// Un widget qui veut `n` pt mais se réduit à la place reçue (comme `base/table`), en `"fit"`.
const FIT: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("fit", grow: "fit", params: (n: p.number()),
  (args, ctx) => layout(sz => block(width: 100%, height: calc.min(args.n * 1pt, sz.height))))"#;
/// Un libellé dont l'élément porte la typographie de son style.
const LABEL: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p, _text-args
#let widget = widget("label", typography: true, params: (x: p.plain()), (args, ctx) => {
  set text(.._text-args(ctx.tokens, ctx.style))
  field(ctx, "x", args.x)
})"#;
const LABEL_PLAIN: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("plainlabel", params: (x: p.plain()), (args, ctx) => field(ctx, "x", args.x))"#;

fn text_block(id: &str) -> serde_json::Value {
    json!({"id": id, "kind": base::TEXT, "params": {"body": [{"runs": [{"t": "Avant"}]}]}})
}

fn bottom_gap(layout: &Layout, id: &str) -> f64 {
    let zone_bottom = layout.zone_on(1).map_or_else(|| panic!("pas de zone en page 1"), |z| z.y + z.h);
    let b = common::box_of(layout, id);
    zone_bottom - (b.y + b.h)
}

#[test]
fn widget_declaring_grow_fill_takes_the_remaining_height() {
    let pack = common::pack("tall", &[("tall", TALL)]);
    let layout = common::render_with_packs(&[pack], json!([text_block("t"), {"id": "g", "kind": "test/tall"}]));
    assert_that!(bottom_gap(&layout, "g")).is_less_than(1.0);
}

#[test]
fn widget_without_grow_keeps_its_natural_height() {
    let pack = common::pack("flat", &[("flat", FLAT)]);
    let layout = common::render_with_packs(&[pack], json!([text_block("t"), {"id": "g", "kind": "test/flat"}]));
    let g = common::box_of(&layout, "g");
    assert_that!(((g.h - 40.0).abs() < 1.0, bottom_gap(&layout, "g") > 50.0)).is_equal_to((true, true));
}

/// `"fit"` qui tient garde sa hauteur : le bloc suivant suit juste dessous.
#[test]
fn widget_declaring_grow_fit_that_fits_keeps_its_height() {
    let pack = common::pack("fit", &[("fit", FIT)]);
    let blocks = json!([text_block("t"), {"id": "f", "kind": "test/fit", "params": {"n": 40}}, text_block("u")]);
    let layout = common::render_with_packs(&[pack], blocks);
    let (f, u) = (common::box_of(&layout, "f"), common::box_of(&layout, "u"));
    assert_that!(((f.h - 40.0).abs() < 1.0, u.y - (f.y + f.h) < 30.0, bottom_gap(&layout, "u") > 50.0))
        .is_equal_to((true, true, true));
}

/// `"fit"` trop haut prend une part : il se réduit, et le bloc suivant reste dans la zone.
#[test]
fn widget_declaring_grow_fit_too_tall_takes_a_share() {
    let pack = common::pack("fit", &[("fit", FIT)]);
    let blocks = json!([text_block("t"), {"id": "f", "kind": "test/fit", "params": {"n": 2000}}, text_block("u")]);
    let layout = common::render_with_packs(&[pack], blocks);
    let f = common::box_of(&layout, "f");
    assert_that!((f.h < 1000.0, bottom_gap(&layout, "u") > -0.5)).is_equal_to((true, true));
}

#[rstest]
#[case("test/label", true)]
#[case("test/plainlabel", false)]
fn element_box_reports_the_style_size_only_with_typography(#[case] kind: &str, #[case] styled: bool) {
    // Un seul pack : deux packs du même espace de noms seraient refusés (`DuplicatePack`).
    let packs = [common::pack("label", &[("label", LABEL), ("plainlabel", LABEL_PLAIN)])];
    let blocks = json!([{"id": "l", "kind": kind, "style": {"size": "title"}, "params": {"x": "Libellé"}}]);
    let layout = common::render_with_packs(&packs, blocks);
    let size = common::box_of(&layout, "l").style.as_ref().map_or(0.0, |s| s.size);
    let title = common::token_size("title");
    assert_that!((size - title).abs() < 0.5).is_equal_to(styled);
}

#[rstest]
#[case(r#"widget("s", category: "slide", grow: true, (a, c) => [])"#)]
#[case(r#"widget("s", category: "slide", typography: true, (a, c) => [])"#)]
#[case(r#"widget("s", grow: "fill", (a, c) => [])"#)]
#[case(r#"widget("s", grow: 1, (a, c) => [])"#)]
#[case(r#"widget("s", typography: "yes", (a, c) => [])"#)]
fn widget_with_invalid_sizing_is_refused_by_the_probe(#[case] call: &str) {
    let src = format!("#import \"/oku-slides/lib.typ\": widget\n#let widget = {call}\n");
    let probe = probe_file(&Fonts::default(), PackKind::Widgets, &WidgetFile::parse("s").unwrap(), &src, None);
    assert_that!(probe.is_valid()).is_false();
}

/// Une image large et pleine (600 × 100, noire) : dans le flux, à ses proportions, elle ferait 50 pt de haut.
const WIDE_SVG: &[u8] =
    br#"<svg xmlns="http://www.w3.org/2000/svg" width="600" height="100"><rect width="600" height="100"/></svg>"#;

#[test]
fn framed_image_is_stretched_over_its_whole_frame() {
    let deck = common::blank_with_element(
        json!({"id": "i", "kind": base::IMAGE, "frame": {"x": 100, "y": 100, "w": 300, "h": 120},
        "params": {"asset": "sha256:abcd", "fit": "stretch"}}),
    );
    let assets = BTreeMap::from([("sha256:abcd".to_owned(), WIDE_SVG.to_vec())]);
    let compiled = common::renderer().render(&deck, &assets).unwrap();
    // PNG à 1 px/pt : le bas du cadre (5 pt au-dessus de son bord) est noir si l'image
    // remplit le cadre, clair si elle était dessinée à ses proportions (50 pt de haut).
    let i = common::box_of(compiled.layout(), "i").clone();
    let png = compiled.png(0, 1.0).unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(png)).read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    let channels = info.line_size / info.width as usize;
    let at = (i.y + i.h - 5.0) as usize * info.line_size + (i.x + i.w / 2.0) as usize * channels;
    assert_that!(buf[at] < 40).is_true();
}
