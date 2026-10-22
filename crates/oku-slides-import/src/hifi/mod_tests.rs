use std::collections::HashSet;

use oku_slides_core::kinds::base;
use oku_slides_core::{Element, ParamValue, validate};
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::plan::ItemTarget;
use crate::report::{LossKind, Placement};
use crate::source::{
    Fill, Frame, Geometry, Item, ItemKind, Layout, LevelStyle, Master, MasterTextStyles, Placeholder, PlaceholderLink,
    Rgba, Role, Shape, Size, SourceSlide, Styled, TextBox,
};
use crate::test_support::{
    app_fonts, base_pin, essentiel_pin, item, layout, office_theme, para, registry, registry_with_deck_layouts, run,
    run_style, source_deck, source_slide, text_box,
};

/// Un deck Office : master `m1` (un bandeau en décor), layout `l1` au titre
/// en (60, 30, 840, 70) pt, slides `slides`.
fn deck(slides: Vec<SourceSlide>) -> SourceDeck {
    let level = LevelStyle { para: para(Vec::new()).style, run: run_style() };
    let bandeau = Shape {
        geom: Geometry::Preset { name: "rect".into(), adj: Default::default() },
        fill: Fill::Solid { color: Styled::scheme(office_theme().colors["accent1"], "accent1") },
        stroke: None,
        text: None,
        connection: None,
    };
    let master = Master {
        id: "m1".into(),
        name: "Office".into(),
        theme: 0,
        background: Fill::None,
        decor: vec![item("m1/2", (0.0, 520.0, 960.0, 20.0), ItemKind::Shape(Box::new(bandeau)))],
        placeholders: Vec::new(),
        text_styles: MasterTextStyles { title: level.clone(), body: level.clone(), other: level.clone() },
    };
    let title = Placeholder {
        id: "l1/2".into(),
        role: Role::Title,
        idx: None,
        frame: Frame::new(60.0, 30.0, 840.0, 70.0),
        levels: vec![level],
        text: TextBox::default(),
    };
    let mut d = source_deck(slides);
    d.themes = vec![office_theme()];
    d.masters = vec![master];
    d.layouts = vec![Layout { name: "Titre".into(), placeholders: vec![title], ..layout("l1", "title") }];
    d
}

/// Le titre `text` d'une slide, à la boîte `(x, y, w, h)`.
fn title(id: &str, frame: (f64, f64, f64, f64), text: &str) -> Item {
    Item {
        role: Some(Role::Title),
        placeholder: Some(PlaceholderLink { idx: None, layout: Some("l1/2".into()), master: None }),
        ..item(id, frame, ItemKind::Text(text_box(vec![para(vec![run(text)])])))
    }
}

/// Le plan de `deck`, avec les polices de l'app ; contrôlé par le cœur.
fn projected(deck: &SourceDeck) -> ImportPlan {
    projected_with(deck, &app_fonts())
}

/// [`projected`] avec les polices `fonts`.
fn projected_with(deck: &SourceDeck, fonts: &FontCatalog) -> ImportPlan {
    let base = registry();
    let target = HifiTarget {
        registry: &base,
        base: base_pin(),
        essentiel: essentiel_pin(),
        fonts,
        pack_id: Uuid::from_u128(1),
        theme_id: Uuid::from_u128(2),
        name: "Import".into(),
        created: "2026-10-11T08:00:00Z".into(),
        limits: PackLimits::default(),
    };
    let plan = project(deck, &target);
    let registry = registry_with_deck_layouts(&plan.schemas);
    let mut d = Deck::empty();
    apply_all(&mut d, plan.ops.clone(), &registry).unwrap();
    let assets: HashSet<String> = plan.assets.keys().cloned().collect();
    assert_that!(validate(&d, &registry, Some(&assets))).is_empty();
    plan
}

/// La `n`-ième slide ajoutée par le plan.
fn slide(plan: &ImportPlan, n: usize) -> &Slide {
    let mut slides = plan.ops.iter().filter_map(|op| match op {
        Op::AddSlide { slide, .. } => Some(slide),
        _ => None,
    });
    slides.nth(n).unwrap()
}

fn extras(s: &Slide) -> Vec<Element> {
    s.params.slots().flat_map(|(_, list)| list.to_vec()).collect()
}

#[test]
fn project_puts_a_placeholder_at_its_layout_box_in_the_widget_param() {
    let plan =
        projected(&deck(vec![source_slide(1, Some("l1"), vec![title("s1/2", (60.0, 30.0, 840.0, 70.0), "Bonjour")])]));
    let s = slide(&plan, 0);
    assert_that!(s.kind.as_str()).is_equal_to("deck-layouts/l1-titre");
    assert_that!(serde_json::to_value(s.params.get("titre")).unwrap())
        .is_equal_to(json!([{ "runs": [{ "t": "Bonjour" }] }]));
    assert_that!(extras(s)).is_empty();
    assert_that!(plan.report.slides[0].placement).is_equal_to(Placement::Free);
}

#[test]
fn project_puts_a_moved_placeholder_in_extras() {
    let plan =
        projected(&deck(vec![source_slide(1, Some("l1"), vec![title("s1/2", (60.0, 200.0, 840.0, 70.0), "Bas")])]));
    let s = slide(&plan, 0);
    assert_that!(s.params.get("titre")).is_none();
    let kinds: Vec<String> = extras(s).into_iter().map(|e| e.kind).collect();
    assert_that!(kinds).is_equal_to(vec![base::TEXT.to_owned()]);
}

#[test]
fn project_targets_a_layout_placeholder_by_its_param_and_a_moved_one_by_its_element() {
    let items = vec![title("s1/2", (60.0, 30.0, 840.0, 70.0), "Haut")];
    let moved = vec![title("s2/2", (60.0, 200.0, 840.0, 70.0), "Bas")];
    let plan = projected(&deck(vec![source_slide(1, Some("l1"), items), source_slide(2, Some("l1"), moved)]));
    assert_that!(plan.targets.get("s1/2")).is_equal_to(Some(&ItemTarget::Param("titre".into())));
    let ids: Vec<String> = extras(slide(&plan, 1)).into_iter().map(|e| e.id).collect();
    assert_that!(plan.targets.get("s2/2")).is_equal_to(Some(&ItemTarget::Elements(ids)));
}

#[test]
fn project_hides_the_decor_of_a_slide_that_masks_it() {
    let hiding = SourceSlide { show_layout_decor: false, ..source_slide(2, Some("l1"), Vec::new()) };
    let plan = projected(&deck(vec![source_slide(1, Some("l1"), Vec::new()), hiding]));
    assert_that!(slide(&plan, 0).params.get(widget::DECOR_PARAM)).is_none();
    assert_that!(slide(&plan, 1).params.get(widget::DECOR_PARAM)).is_equal_to(Some(&ParamValue::Value(json!(false))));
}

#[test]
fn project_draws_an_own_background_first_and_loses_hidden_slides() {
    let red = Fill::Solid { color: Styled::explicit(Rgba::new(255, 0, 0)) };
    let own = SourceSlide {
        background: red,
        hidden: true,
        ..source_slide(
            1,
            Some("l1"),
            vec![item("s1/3", (0.0, 0.0, 10.0, 10.0), ItemKind::Text(text_box(vec![para(vec![run("x")])])))],
        )
    };
    let plan = projected(&deck(vec![own]));
    let first = &extras(slide(&plan, 0))[0];
    assert_that!((first.id.as_str(), first.style.fill.as_deref())).is_equal_to(("s1-1", Some("#ff0000")));
    let f = first.frame.unwrap();
    assert_that!((f.x, f.y, f.w, f.h)).is_equal_to((0.0, 0.0, 1920.0, 1080.0));
    let kinds: Vec<&LossKind> = plan.report.slides[0].losses.iter().map(|l| &l.kind).collect();
    assert_that!(kinds).is_equal_to(vec![&LossKind::HiddenSlide]);
}

#[test]
fn project_without_layout_takes_the_blank_widget() {
    let text = item("s1/2", (10.0, 10.0, 100.0, 20.0), ItemKind::Text(text_box(vec![para(vec![run("Seul")])])));
    let plan = projected(&deck(vec![source_slide(1, None, vec![text])]));
    let s = slide(&plan, 0);
    assert_that!(s.kind.as_str()).is_equal_to(kinds::essentiel::BLANK);
    assert_that!(s.params.slot("elements").map(<[Element]>::len)).is_equal_to(Some(1));
}

#[test]
fn project_fits_another_page_ratio_and_says_it_on_the_first_slide() {
    let mut d = deck(vec![source_slide(1, Some("l1"), Vec::new()), source_slide(2, Some("l1"), Vec::new())]);
    d.size = Size { w: 720.0, h: 540.0 };
    let plan = projected(&d);
    let kinds: Vec<&LossKind> = plan.report.slides[0].losses.iter().map(|l| &l.kind).collect();
    assert_that!(kinds).is_equal_to(vec![&LossKind::PageFitted { width: 720, height: 540 }]);
    assert_that!(plan.report.slides[1].losses).is_empty();
}

#[test]
fn project_widget_examples_are_the_first_slide_of_its_layout() {
    let mut first = source_slide(1, Some("l1"), vec![title("s1/2", (60.0, 30.0, 840.0, 70.0), "Premier")]);
    first.notes = "Notes".into();
    let second = source_slide(2, Some("l1"), vec![title("s2/2", (60.0, 30.0, 840.0, 70.0), "Second")]);
    let plan = projected(&deck(vec![first, second]));
    let manifest = oku_slides_core::PackManifest::from_files(plan.pack.as_ref().unwrap()).unwrap();
    let values = &manifest.examples[&oku_slides_core::WidgetFile::parse("l1-titre").unwrap()];
    assert_that!(values["titre"].to_string()).contains("Premier");
    assert_that!(values["titre"].to_string()).does_not_contain("Second");
    assert_that!(
        plan.ops.iter().any(|op| matches!(op, Op::SetNotes { slide, notes } if slide == "s1" && notes == "Notes"))
    )
    .is_true();
}

#[test]
fn sought_fonts_include_the_theme_fonts_of_the_first_master() {
    let found = sought_fonts(&deck(Vec::new()));
    let expected: BTreeSet<String> = ["calibri", "calibri light", "carlito"].map(str::to_owned).into();
    assert_that!(found).is_equal_to(expected);
}

#[test]
fn project_of_more_used_layouts_than_the_pack_holds_puts_the_rest_on_blank() {
    let mut d = deck(Vec::new());
    let red = Fill::Solid { color: Styled::explicit(Rgba::new(255, 0, 0)) };
    for i in 1..=70u32 {
        let filet =
            item(&format!("x{i}/f"), (60.0, 110.0, 840.0, 2.0), ItemKind::Text(text_box(vec![para(vec![run("x")])])));
        d.layouts.push(Layout {
            name: format!("Layout {i}"),
            background: Some(red.clone()),
            decor: vec![filet],
            ..layout(&format!("x{i}"), "obj")
        });
        d.slides.push(source_slide(i, Some(&format!("x{i}")), Vec::new()));
    }
    let plan = projected(&d);
    let pack = plan.pack.as_ref().unwrap();
    assert_that!(pack.widgets().len()).is_less_than_or_equal_to(64);
    // Toutes employées une fois : les 63 premières dans l'ordre de la source ont un widget.
    let kinds: Vec<&str> = plan.report.slides.iter().map(|s| s.profile.as_str()).collect();
    assert_that!(kinds[62]).is_equal_to("deck-layouts/l64-layout-63");
    assert_that!(kinds[63..].iter().all(|k| *k == kinds::essentiel::BLANK)).is_true();
    // Sur `essentiel/blank` : le fond du layout, le bandeau du master, puis le filet du layout.
    let s = slide(&plan, 63);
    let elements = extras(s);
    assert_that!(elements.len()).is_equal_to(3);
    assert_that!(elements[0].style.fill.as_deref()).is_equal_to(Some("#ff0000"));
    assert_that!(elements[2].kind.as_str()).is_equal_to(base::TEXT);
    let dropped: Vec<&LossKind> = plan.report.slides[63].losses.iter().map(|l| &l.kind).collect();
    assert_that!(dropped).is_equal_to(vec![&LossKind::LayoutDropped { layout: "Layout 64".into() }]);
    assert_that!(plan.report.slides[62].losses).is_empty();
}

#[test]
fn project_without_the_body_font_falls_back_to_a_sans_app_font() {
    // Ni Calibri ni Carlito : le corps et les titres prennent Liberation Sans.
    let mut liberation = FontCatalog::new();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts");
    liberation.add(crate::fonts::FontOrigin::App, std::fs::read(dir.join("liberation-sans-400.ttf")).unwrap());
    let plan = projected_with(&deck(vec![source_slide(1, Some("l1"), Vec::new())]), &liberation);
    let theme = oku_slides_core::ThemeManifest::from_files(plan.theme.as_ref().unwrap()).unwrap();
    assert_that!(theme.tokens["font"].clone())
        .is_equal_to(json!({ "display": ["Liberation Sans"], "body": ["Liberation Sans"] }));
    let fallbacks: Vec<Option<&str>> = plan.report.fonts.iter().map(|c| c.fallback.as_deref()).collect();
    assert_that!(fallbacks).is_equal_to(vec![Some("Liberation Sans"), Some("Liberation Sans")]);

    // Sans aucune police de l'app : pas de jeton de police, le moteur prend la sienne.
    let plan = projected_with(&deck(vec![source_slide(1, Some("l1"), Vec::new())]), &FontCatalog::new());
    let theme = oku_slides_core::ThemeManifest::from_files(plan.theme.as_ref().unwrap()).unwrap();
    assert_that!(theme.tokens.get("font")).is_none();
    assert_that!(plan.report.fonts.iter().all(|c| c.fallback.is_none())).is_true();
}

#[test]
fn project_of_alto_twice_gives_the_same_plan() {
    let deck = crate::test_support::fixture("alto-wealth.pptx");
    let (first, second) = (projected(&deck), projected(&deck));
    assert_that!(first.ops == second.ops).is_true();
    assert_that!(first.pack == second.pack && first.theme == second.theme).is_true();
    assert_that!(first.report == second.report && first.assets == second.assets).is_true();
}

/// Un rapport vide par slide de `deck`.
fn reports(deck: &SourceDeck) -> Vec<SlideReport> {
    deck.slides
        .iter()
        .map(|s| SlideReport {
            source: s.number,
            slide: format!("s{}", s.number),
            profile: String::new(),
            placement: Placement::Free,
            losses: Vec::new(),
            fallbacks: Vec::new(),
            score: None,
        })
        .collect()
}

#[test]
fn attach_pack_losses_go_to_the_first_slide_of_their_layout_or_master_else_the_first() {
    let mut d = deck(vec![source_slide(1, None, Vec::new()), source_slide(2, Some("l1"), Vec::new())]);
    d.layouts.push(Layout { master: "m2".into(), ..layout("l9", "obj") });
    let lost = |item: &str| Loss::on(item, LossKind::Effects);
    let pack = GeneratedPack {
        losses: vec![lost("l1/4"), lost("m1/2"), lost("x7/1"), lost("m2/1")],
        ..GeneratedPack::default()
    };
    let mut slides = reports(&d);
    attach_pack_losses(&d, &pack, &mut slides);
    let items = |r: &SlideReport| r.losses.iter().filter_map(|l| l.item.clone()).collect::<Vec<_>>();
    // `l1/4` et `m1/2` à la slide 2 (layout l1, master m1) ; `x7/1` (inconnu) et `m2/1`
    // (master sans slide) à la première.
    assert_that!(items(&slides[1])).is_equal_to(vec!["l1/4".to_owned(), "m1/2".to_owned()]);
    assert_that!(items(&slides[0])).is_equal_to(vec!["x7/1".to_owned(), "m2/1".to_owned()]);
}

#[test]
fn project_maps_a_free_text_item_to_its_element_in_origins() {
    let text = item("s1/3", (100.0, 200.0, 300.0, 50.0), ItemKind::Text(text_box(vec![para(vec![run("Libre")])])));
    let plan = projected(&deck(vec![source_slide(1, Some("l1"), vec![text])]));
    let element = extras(slide(&plan, 0)).into_iter().next().unwrap();
    assert_that!(plan.origins.get("s1/3")).is_equal_to(Some(&element.id));
}
