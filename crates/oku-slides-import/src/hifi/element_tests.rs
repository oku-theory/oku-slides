use std::collections::BTreeSet;

use oku_slides_core::{Op, ParamValue, Slide};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;
use crate::fonts::{FontPlan, resolve};
use crate::hifi::color::ThemeColors;
use crate::hifi::page::Page;
use crate::hifi::theme::{theme_colors, theme_master};
use crate::hifi::typst::mod_tests::Fixture;
use crate::source::{
    Anchor, Arrow, ArrowKind, ArrowSize, Chart, GradientStop, Grouping, Opaque, OpaqueReason, PathCmd, Picture, Role,
    Series, SourceDeck, SourceTheme, Styled, SubPath, TextBox,
};
use crate::test_support::{app_fonts, applied, fixture, item, office_theme, para, run, text_box};

/// Contexte des tests : celui du générateur (page 960 × 540, échelle 2,
/// thème Office, Calibri retenue en Carlito, assets `sha256:aa` en PNG et
/// `sha256:ee` en EMF), polices du thème Office, effets ignorés sur `effects`.
pub(crate) struct Setup {
    base: Fixture,
    theme: SourceTheme,
    effects: BTreeSet<String>,
}

impl Setup {
    pub(crate) fn new() -> Self {
        Self { base: Fixture::new(), theme: office_theme(), effects: BTreeSet::new() }
    }

    /// Le contexte où l'import retient les familles `families`.
    pub(crate) fn with_fonts(families: &[&str]) -> Self {
        Self { base: Fixture::with_fonts(families), ..Self::new() }
    }

    pub(crate) fn ctx(&self) -> ElementCtx<'_> {
        let base = GenCtx { theme_fonts: Some(&self.theme.fonts), ..self.base.ctx() };
        ElementCtx { base, effects: &self.effects, table: None }
    }

    /// Les éléments de `item`, ids `s1-1`, `s1-2`…
    pub(crate) fn convert(&self, item: &Item) -> Placed {
        let mut k = 0;
        elements(item, &self.ctx(), &mut || {
            k += 1;
            format!("s1-{k}")
        })
    }
}

/// Les éléments posés dans le slot libre d'une slide `blank` du registre de
/// base : panique si le cœur refuse une op ou lève une `Issue`.
pub(crate) fn checked(elements: &[Element], assets: &[String]) {
    if elements.is_empty() {
        return;
    }
    let mut slide = Slide::new("s1", "blank");
    slide.params.set("elements", ParamValue::Slot(elements.to_vec()));
    applied(vec![Op::AddSlide { index: None, defaults: false, slide }], assets);
}

/// Couleur de l'emplacement `slot` du thème Office, sans modificateur.
fn scheme(slot: &str) -> source::Color {
    Styled::scheme(office_theme().colors[slot], slot)
}

fn solid(slot: &str) -> Fill {
    Fill::Solid { color: scheme(slot) }
}

fn preset(name: &str, adj: &[(&str, f64)]) -> Geometry {
    Geometry::Preset { name: name.to_owned(), adj: adj.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect() }
}

fn stroke(dash: Dash) -> Stroke {
    Stroke { width: 1.0, color: Styled::explicit(Rgba::new(0x12, 0x34, 0x56)), dash, head: None, tail: None }
}

/// Le trait de [`stroke`] avec des pointes moyennes `head` et `tail`.
fn arrowed(head: Option<ArrowKind>, tail: Option<ArrowKind>) -> Stroke {
    let arrow = |kind| Arrow { kind, width: ArrowSize::Medium, length: ArrowSize::Medium };
    Stroke { head: head.map(arrow), tail: tail.map(arrow), ..stroke(Dash::Solid) }
}

/// Une forme `s1/1` en (10, 20, 100, 50) pt source : (20, 40, 200, 100) pt page.
fn shape_item(geom: Geometry, fill: Fill, stroke: Option<Stroke>, text: Option<TextBox>) -> Item {
    item(
        "s1/1",
        (10.0, 20.0, 100.0, 50.0),
        ItemKind::Shape(Box::new(Shape { geom, fill, stroke, text, connection: None })),
    )
}

fn shape(geom: Geometry, fill: Fill, stroke: Option<Stroke>) -> Item {
    shape_item(geom, fill, stroke, None)
}

fn with_frame(mut item: Item, f: impl FnOnce(&mut source::Frame)) -> Item {
    f(&mut item.frame);
    item
}

fn picture(crop: Crop) -> Item {
    item("s1/1", (10.0, 20.0, 100.0, 50.0), ItemKind::Picture(Picture { asset: "sha256:aa".into(), crop }))
}

fn opaque(fallback: Option<&str>) -> Item {
    let o = Opaque { reason: OpaqueReason::Ole, fallback: fallback.map(str::to_owned) };
    item("s1/1", (10.0, 20.0, 100.0, 50.0), ItemKind::Opaque(o))
}

/// Un tracé libre : un triangle rempli sans contour, puis un trait seul.
fn split_path() -> Geometry {
    let triangle = vec![
        PathCmd::MoveTo { x: 0.0, y: 1.0 },
        PathCmd::LineTo { x: 0.5, y: 0.0 },
        PathCmd::LineTo { x: 1.0, y: 1.0 },
        PathCmd::Close,
    ];
    let rule = vec![PathCmd::MoveTo { x: 0.0, y: 0.5 }, PathCmd::LineTo { x: 1.0, y: 0.5 }];
    Geometry::Path {
        paths: vec![
            SubPath { cmds: triangle, fill: true, stroke: false },
            SubPath { cmds: rule, fill: false, stroke: true },
        ],
    }
}

fn group() -> Item {
    let children = vec![
        shape(preset("rect", &[]), solid("accent1"), None),
        with_frame(picture(Crop::default()), |f| (f.x, f.y) = (200.0, 100.0)),
    ];
    item("s1/g", (10.0, 20.0, 290.0, 130.0), ItemKind::Group { items: children, smartart: None })
}

fn chart_item() -> Item {
    let series = |name: &str, color: Option<source::Color>| Series {
        name: name.into(),
        values: vec![Some(1.0), Some(2.0)],
        color,
        x: None,
        format: None,
    };
    let c = Chart {
        kind: ChartKind::Column,
        grouping: Grouping::Clustered,
        title: None,
        categories: vec!["A".into(), "B".into()],
        series: vec![
            series("Nord", Some(Styled::explicit(Rgba { a: 128, ..Rgba::new(0xc0, 0x50, 0x4d) }))),
            series("Sud", None),
        ],
    };
    item("s1/1", (10.0, 20.0, 100.0, 50.0), ItemKind::Chart(c))
}

/// Le cadre (20, 40, 200, 100) pt page.
fn frame() -> Value {
    json!({ "x": 20.0, "y": 40.0, "w": 200.0, "h": 100.0 })
}

/// Le style de trait de [`stroke`] : `#123456`, 2 pt page.
fn pen(mut style: Value) -> Value {
    style["stroke"] = json!("#123456");
    style["stroke-width"] = json!(2.0);
    style
}

fn image(id: &str, frame: Value) -> Value {
    json!({ "id": id, "kind": "image", "frame": frame, "params": { "asset": "sha256:aa", "fit": "stretch" } })
}

#[rstest]
#[case::rect(
    shape(preset("rect", &[]), solid("accent1"), Some(stroke(Dash::Solid))),
    json!([{ "id": "s1-1", "kind": "shape", "frame": frame(), "style": pen(json!({ "fill": "accent" })),
        "params": { "shape": "rect" } }]),
    vec![],
)]
#[case::round_rect(
    shape(preset("roundRect", &[("adj", 25_000.0)]), solid("accent2"), None),
    json!([{ "id": "s1-1", "kind": "shape", "frame": frame(), "style": { "fill": "accent-2", "radius": 25.0 },
        "params": { "shape": "rect" } }]),
    vec![],
)]
#[case::ellipse_flipped(
    with_frame(shape(preset("ellipse", &[]), solid("accent1"), None), |f| f.flip_h = true),
    json!([{ "id": "s1-1", "kind": "shape", "frame": frame(), "style": { "fill": "accent" },
        "params": { "shape": "ellipse" } }]),
    vec![],
)]
#[case::horizontal_line(
    with_frame(shape(preset("line", &[]), Fill::None, Some(stroke(Dash::Solid))), |f| f.h = 0.0),
    json!([{ "id": "s1-1", "kind": "shape", "frame": { "x": 20.0, "y": 40.0, "w": 200.0, "h": 1.0 },
        "style": pen(json!({})), "params": { "shape": "line" } }]),
    vec![],
)]
#[case::horizontal_line_with_tail_triangle(
    with_frame(shape(preset("line", &[]), Fill::None, Some(arrowed(None, Some(ArrowKind::Triangle)))), |f| f.h = 0.0),
    json!([
        { "id": "s1-1", "kind": "shape", "frame": { "x": 20.0, "y": 40.0, "w": 200.0, "h": 1.0 },
          "style": pen(json!({})), "params": { "shape": "line" } },
        { "id": "s1-2", "kind": "shape", "frame": { "x": 214.0, "y": 37.0, "w": 6.0, "h": 6.0 },
          "style": { "fill": "#123456" }, "params": { "shape": "path", "path": { "d": "M 1 0.5 L 0 1 L 0 0 Z" } } },
    ]),
    vec![],
)]
#[case::flipped_line_with_open_head(
    with_frame(shape(preset("line", &[]), Fill::None, Some(arrowed(Some(ArrowKind::Open), None))), |f| {
        (f.h, f.flip_h) = (0.0, true)
    }),
    json!([
        { "id": "s1-1", "kind": "shape", "frame": { "x": 20.0, "y": 40.0, "w": 200.0, "h": 1.0 },
          "style": pen(json!({})), "params": { "shape": "line" } },
        { "id": "s1-2", "kind": "shape", "frame": { "x": 214.0, "y": 37.0, "w": 6.0, "h": 6.0 },
          "style": pen(json!({})), "params": { "shape": "path", "path": { "d": "M 0 1 L 1 0.5 L 0 0" } } },
    ]),
    vec![],
)]
#[case::diagonal_line_flipped(
    with_frame(shape(preset("line", &[]), Fill::None, Some(stroke(Dash::Solid))), |f| f.flip_h = true),
    json!([{ "id": "s1-1", "kind": "shape", "frame": frame(), "style": pen(json!({})),
        "params": { "shape": "path", "path": { "d": "M 1 0 L 0 1" } } }]),
    vec![],
)]
#[case::vertical_line(
    with_frame(shape(preset("line", &[]), Fill::None, Some(stroke(Dash::Solid))), |f| f.w = 0.0),
    json!([{ "id": "s1-1", "kind": "shape", "frame": { "x": 19.5, "y": 40.0, "w": 1.0, "h": 100.0 },
        "style": pen(json!({})), "params": { "shape": "path", "path": { "d": "M 0.5 0 L 0.5 1" } } }]),
    vec![],
)]
#[case::line_without_stroke(shape(preset("line", &[]), Fill::None, None), json!([]), vec![])]
#[case::preset_as_path(
    shape(preset("triangle", &[]), solid("accent1"), None),
    json!([{ "id": "s1-1", "kind": "shape", "frame": frame(), "style": { "fill": "accent" },
        "params": { "shape": "path", "path": { "d": "M 0 1 L 0.5 0 L 1 1 Z" } } }]),
    vec![],
)]
#[case::filled_and_stroked_apart(
    shape(split_path(), solid("accent1"), Some(stroke(Dash::Solid))),
    json!([
        { "id": "s1-1", "kind": "shape", "frame": frame(), "style": { "fill": "accent" },
          "params": { "shape": "path", "path": { "d": "M 0 1 L 0.5 0 L 1 1 Z" } } },
        { "id": "s1-2", "kind": "shape", "frame": frame(), "style": pen(json!({})),
          "params": { "shape": "path", "path": { "d": "M 0 0.5 L 1 0.5" } } },
    ]),
    vec![],
)]
#[case::path_flipped_vertically(
    with_frame(shape(split_path(), solid("accent1"), None), |f| f.flip_v = true),
    json!([{ "id": "s1-1", "kind": "shape", "frame": frame(), "style": { "fill": "accent" },
        "params": { "shape": "path", "path": { "d": "M 0 0 L 0.5 1 L 1 0 Z" } } }]),
    vec![],
)]
#[case::unknown_preset(
    shape(preset("notAPreset", &[]), solid("accent1"), None),
    json!([{ "id": "s1-1", "kind": "shape", "frame": frame(), "style": { "fill": "accent" },
        "params": { "shape": "rect" } }]),
    vec![LossKind::ShapeStyle(ShapeEffect::Geometry)],
)]
#[case::gradient_and_dash(
    shape(preset("rect", &[]), Fill::Gradient { angle: 0.0, stops: vec![
        GradientStop { pos: 1.0, color: scheme("accent2") },
        GradientStop { pos: 0.0, color: scheme("accent1") },
    ] }, Some(stroke(Dash::Dash))),
    json!([{ "id": "s1-1", "kind": "shape", "frame": frame(), "style": pen(json!({ "fill": "accent" })),
        "params": { "shape": "rect" } }]),
    vec![LossKind::ShapeStyle(ShapeEffect::Gradient), LossKind::ShapeStyle(ShapeEffect::Dash)],
)]
#[case::picture_fill(
    shape(preset("ellipse", &[]), Fill::Picture { asset: "sha256:aa".into() }, Some(stroke(Dash::Solid))),
    json!([
        image("s1-1", frame()),
        { "id": "s1-2", "kind": "shape", "frame": frame(), "style": pen(json!({})), "params": { "shape": "ellipse" } },
    ]),
    vec![LossKind::ShapeStyle(ShapeEffect::Picture)],
)]
#[case::shape_without_paint(shape(preset("rect", &[]), Fill::None, None), json!([]), vec![])]
#[case::picture(picture(Crop::default()), json!([image("s1-1", frame())]), vec![])]
#[case::cropped_picture(
    picture(Crop { l: 0.1, ..Crop::default() }),
    json!([image("s1-1", frame())]),
    vec![LossKind::Transform],
)]
#[case::imperceptible_crop(
    picture(Crop { l: 0.00001, t: 0.00001, r: 0.0009, b: 0.0 }),
    json!([image("s1-1", frame())]),
    vec![],
)]
#[case::crop_at_tolerance(
    picture(Crop { b: 0.001, ..Crop::default() }),
    json!([image("s1-1", frame())]),
    vec![LossKind::Transform],
)]
#[case::unsupported_picture(
    item("s1/1", (10.0, 20.0, 100.0, 50.0), ItemKind::Picture(Picture { asset: "sha256:ee".into(), crop: Crop::default() })),
    json!([]),
    vec![LossKind::UnsupportedImage("image/x-emf".into())],
)]
#[case::group_flattened(
    group(),
    json!([
        { "id": "s1-1", "kind": "shape", "frame": frame(), "style": { "fill": "accent" }, "params": { "shape": "rect" } },
        image("s1-2", json!({ "x": 400.0, "y": 200.0, "w": 200.0, "h": 100.0 })),
    ]),
    vec![],
)]
#[case::opaque_with_fallback(opaque(Some("sha256:aa")), json!([image("s1-1", frame())]), vec![])]
#[case::opaque_without_fallback(opaque(None), json!([]), vec![LossKind::Opaque(OpaqueReason::Ole)])]
#[case::rotated(
    with_frame(shape(preset("rect", &[]), solid("accent1"), None), |f| f.rot = 30.0),
    json!([{ "id": "s1-1", "kind": "shape", "frame": frame(), "style": { "fill": "accent" },
        "params": { "shape": "rect" } }]),
    vec![LossKind::Transform],
)]
#[case::flipped_picture(
    with_frame(picture(Crop::default()), |f| f.flip_v = true),
    json!([image("s1-1", frame())]),
    vec![LossKind::Transform],
)]
#[case::flipped_picture_fill(
    with_frame(shape(preset("rect", &[]), Fill::Picture { asset: "sha256:aa".into() }, None), |f| f.flip_h = true),
    json!([image("s1-1", frame())]),
    vec![LossKind::Transform],
)]
#[case::chart_without_data(
    item("s1/1", (10.0, 20.0, 100.0, 50.0), ItemKind::Chart(Chart {
        kind: ChartKind::Column,
        grouping: Grouping::Clustered,
        title: None,
        categories: Vec::new(),
        series: Vec::new(),
    })),
    json!([]),
    vec![LossKind::EmptyData],
)]
fn elements_of_an_item_are_placed_in_page_points(
    #[case] item: Item,
    #[case] expected: Value,
    #[case] losses: Vec<LossKind>,
) {
    let placed = Setup::new().convert(&item);
    assert_that!(serde_json::to_value(&placed.elements).unwrap()).is_equal_to(expected);
    let kinds: Vec<LossKind> = placed.losses.iter().map(|l| l.kind.clone()).collect();
    assert_that!(kinds).is_equal_to(losses);
    checked(&placed.elements, &["sha256:aa".to_owned()]);
}

#[test]
fn elements_of_a_shape_with_text_put_the_text_over_the_shape_inside_its_text_rect() {
    let tb = TextBox { anchor: Anchor::Middle, ..text_box(vec![para(vec![run("Bonjour")])]) };
    let placed = Setup::new().convert(&shape_item(preset("ellipse", &[]), solid("accent1"), None, Some(tb)));
    let kinds: Vec<&str> = placed.elements.iter().map(|e| e.kind.as_str()).collect();
    assert_that!(kinds).is_equal_to(vec!["shape", "text"]);
    let text = &placed.elements[1];
    assert_that!(text.frame).is_equal_to(placed.elements[0].frame);
    // Zone de texte d'une ellipse : le carré inscrit, ≈ 14,6 % de chaque côté ;
    // puis les marges OOXML (3,6 pt haut et bas, 7,2 pt côtés, × 2) ; en haut et
    // en bas, le recalage des lignes de base (Carlito 36 pt : +10,81 et +9,3 pt).
    assert_that!(text.style.inset).is_equal_to(Some([32.65, 43.69, 31.14, 43.69]));
    assert_that!(text.style.valign.as_deref()).is_equal_to(Some("middle"));
    // La couverture cherche le texte de la forme dans son élément `text`.
    assert_that!(placed.origins).is_equal_to(vec![("s1/1".to_owned(), text.id.clone())]);
    checked(&placed.elements, &[]);
}

#[test]
fn elements_of_a_horizontally_flipped_shape_mirror_its_text_rect() {
    let tb = text_box(vec![para(vec![run("→")])]);
    let geom = preset("rightArrow", &[]);
    let item =
        |flip_h| with_frame(shape_item(geom.clone(), solid("accent1"), None, Some(tb.clone())), |f| f.flip_h = flip_h);
    let inset = |flip_h| Setup::new().convert(&item(flip_h)).elements[1].style.inset.unwrap();
    let ([_, r, _, l], [_, fr, _, fl]) = (inset(false), inset(true));
    assert_that!((fl, fr)).is_equal_to((r, l));
}

#[test]
fn elements_of_a_vertically_flipped_shape_with_text_lose_the_transform() {
    let tb = text_box(vec![para(vec![run("Bonjour")])]);
    let item = with_frame(shape_item(preset("rect", &[]), solid("accent1"), None, Some(tb)), |f| f.flip_v = true);
    let placed = Setup::new().convert(&item);
    assert_that!(placed.losses).is_equal_to(vec![Loss::on("s1/1", LossKind::Transform)]);
}

#[test]
fn elements_of_an_item_with_ignored_effects_lose_them() {
    let mut setup = Setup::new();
    setup.effects.insert("s1/1".to_owned());
    let placed = setup.convert(&shape(preset("rect", &[]), solid("accent1"), None));
    assert_that!(placed.losses).is_equal_to(vec![Loss::on("s1/1", LossKind::Effects)]);
}

#[test]
fn elements_of_a_chart_keep_the_series_colors_as_opaque_hex() {
    let placed = Setup::new().convert(&chart_item());
    let option = match placed.elements[0].params.get("option") {
        Some(ParamValue::Value(v)) => v.clone(),
        other => panic!("option : {other:?}"),
    };
    assert_that!(option["series"][0]["itemStyle"]).is_equal_to(json!({ "color": "#c0504d" }));
    assert_that!(option["series"][1].get("itemStyle")).is_none();
    checked(&placed.elements, &[]);
}

#[test]
fn elements_of_a_table_reuse_the_retrofit_conversion() {
    let cell = |t: &str| source::Cell {
        text: text_box(vec![para(vec![run(t)])]),
        fill: Fill::None,
        borders: source::Borders { l: None, t: None, r: None, b: None },
        row_span: 1,
        col_span: 1,
        covered: false,
    };
    let t = source::Table {
        col_widths: vec![50.0, 50.0],
        row_heights: vec![25.0, 25.0],
        rows: vec![vec![cell("A"), cell("B")], vec![cell("1"), cell("2")]],
        flags: source::TableFlags {
            first_row: true,
            last_row: false,
            first_col: false,
            last_col: false,
            band_row: true,
            band_col: false,
        },
    };
    let placed = Setup::new().convert(&item("s1/1", (10.0, 20.0, 100.0, 50.0), ItemKind::Table(t)));
    assert_that!(placed.elements[0].kind.as_str()).is_equal_to("table");
    assert_that!(placed.elements[0].params.get("data")).is_some();
    checked(&placed.elements, &[]);
}

/// Les pertes `TableStyle` d'un tableau sous le style du thème pris de `deck`.
fn table_style_losses(deck: &source::Table, t: source::Table) -> usize {
    let setup = Setup::new();
    let base = setup.ctx().base;
    let look = TableLook::of(deck, base.colors, base.page);
    let ctx = ElementCtx { table: Some(&look), ..setup.ctx() };
    let mut k = 0;
    let placed = elements(&item("s1/1", (0.0, 0.0, 100.0, 50.0), ItemKind::Table(t)), &ctx, &mut || {
        k += 1;
        format!("s1-{k}")
    });
    placed.losses.iter().filter(|l| matches!(l.kind, LossKind::TableStyle)).count()
}

/// Un tableau de deux lignes, sans fond, en encre du thème, `bold` pour la première.
fn plain_table(bold: bool) -> source::Table {
    let ink = Styled::scheme(office_theme().colors["dk1"], "dk1");
    let cell = |t: &str, b: bool| source::Cell {
        text: text_box(vec![para(vec![crate::test_support::run_with(t, |s| (s.color, s.bold) = (ink.clone(), b))])]),
        fill: Fill::None,
        borders: source::Borders::default(),
        row_span: 1,
        col_span: 1,
        covered: false,
    };
    source::Table {
        col_widths: vec![50.0, 50.0],
        row_heights: Vec::new(),
        flags: source::TableFlags::default(),
        rows: vec![vec![cell("A", bold), cell("B", false)], vec![cell("1", false), cell("2", false)]],
    }
}

#[test]
fn elements_of_a_table_in_the_deck_style_lose_nothing_and_an_irregular_one_loses_its_style() {
    let regular = plain_table(false);
    assert_that!(table_style_losses(&regular, regular.clone())).is_equal_to(0);
    assert_that!(table_style_losses(&regular, plain_table(true))).is_equal_to(1);
}

/// Le contexte d'un deck lu : page, thème et polices de son master de thème.
struct DeckSetup {
    page: Page,
    colors: ThemeColors,
    fonts: FontPlan,
    effects: BTreeSet<String>,
}

impl DeckSetup {
    fn new(deck: &SourceDeck) -> Self {
        let Some(master) = theme_master(deck) else { panic!("aucun master") };
        let theme = &deck.themes[master.theme];
        Self {
            page: Page::new(deck.size),
            colors: theme_colors(deck, master),
            fonts: resolve(&deck.fonts, &[&theme.fonts.major, &theme.fonts.minor], &app_fonts()),
            effects: effect_items(&deck.warnings),
        }
    }

    /// Les éléments et les pertes de la slide `number` de `deck`, hors
    /// placeholders de texte, validés par le cœur.
    fn slide(&self, deck: &SourceDeck, number: u32) -> Value {
        let Some(master) = theme_master(deck) else { panic!("aucun master") };
        let ctx = ElementCtx {
            base: GenCtx {
                page: &self.page,
                colors: &self.colors,
                fonts: &self.fonts,
                assets: &deck.assets,
                theme_fonts: Some(&deck.themes[master.theme].fonts),
                lang: None,
            },
            effects: &self.effects,
            table: None,
        };
        let Some(slide) = deck.slides.iter().find(|s| s.number == number) else { panic!("slide {number}") };
        let mut k = 0;
        let mut next_id = || {
            k += 1;
            format!("s{number}-{k}")
        };
        let mut out = Placed::default();
        for item in slide.items.iter().filter(|i| !(i.role.is_some_and(is_text_role) && i.text().is_some())) {
            let placed = elements(item, &ctx, &mut next_id);
            out.elements.extend(placed.elements);
            out.assets.extend(placed.assets);
            out.losses.extend(placed.losses);
        }
        checked(&out.elements, &out.assets.iter().cloned().collect::<Vec<_>>());
        json!({ "elements": out.elements, "losses": out.losses })
    }
}

fn is_text_role(role: Role) -> bool {
    !matches!(role, Role::Picture | Role::Chart | Role::Table | Role::Media)
}

#[test]
fn elements_of_alto_slide_2() {
    let deck = fixture("alto-wealth.pptx");
    insta::assert_json_snapshot!(DeckSetup::new(&deck).slide(&deck, 2));
}

#[test]
fn elements_of_kitchen() {
    let deck = fixture("kitchen");
    let setup = DeckSetup::new(&deck);
    let slides: Vec<Value> = deck.slides.iter().map(|s| setup.slide(&deck, s.number)).collect();
    insta::assert_json_snapshot!(slides);
}
