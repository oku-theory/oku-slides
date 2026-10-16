use oku_slides_core::asset_id;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::report::RunEffect;
use crate::source::{Arrow, ArrowKind, ArrowSize, Crop, Dash, Opaque, OpaqueReason, Picture, Rgba, Stroke, Styled};
use crate::test_support::{PAPIER, apply_blocks, colors, item, para, run, run_with, text_box, text_item};

const BOX: (f64, f64, f64, f64) = (10.0, 10.0, 200.0, 100.0);

fn png() -> String {
    asset_id(b"png")
}

fn emf() -> String {
    asset_id(b"emf")
}

fn assets() -> BTreeMap<String, Asset> {
    BTreeMap::from([
        (png(), Asset { mime: "image/png".into(), bytes: b"png".to_vec() }),
        (emf(), Asset { mime: "image/emf".into(), bytes: b"emf".to_vec() }),
    ])
}

/// Pistes d'une grille 12 × 6 de Papier titrée, ramenées en pt source (échelle 2).
const TRACK: (f64, f64) = (51.7, 40.8);

/// `unit` converti sur Papier, en élément `e1`.
fn convert_unit(unit: &Unit<'_>) -> (Option<Element>, Vec<Loss>, Option<String>) {
    let (tokens, assets) = (colors(PAPIER), assets());
    let c = element(unit, "e1".into(), &ContentCx { tokens: &tokens, assets: &assets, track: TRACK });
    (c.element, c.losses, c.fallback)
}

/// `item` seul converti sur Papier, en élément `e1`.
fn convert(item: &Item) -> (Option<Element>, Vec<Loss>, Option<String>) {
    convert_unit(&Unit::Item(item))
}

fn accent1() -> source::Color {
    Styled::mapped(Rgba::new(0x44, 0x72, 0xc4), "accent1", "accent1", Vec::new())
}

fn solid(color: source::Color) -> Fill {
    Fill::Solid { color }
}

fn shape_item(id: &str, geom: &str, fill: Fill, stroke: Option<Stroke>, text: Option<&str>) -> Item {
    let s = source::Shape {
        geom: Geometry::Preset { name: geom.into(), adj: BTreeMap::new() },
        fill,
        stroke,
        text: text.map(|t| text_box(vec![para(vec![run(t)])])),
        connection: None,
    };
    item(id, BOX, ItemKind::Shape(Box::new(s)))
}

fn picture(asset: &str) -> Item {
    item("p", BOX, ItemKind::Picture(Picture { asset: asset.into(), crop: Crop::default() }))
}

fn texts(e: &Element, param: &str) -> Vec<String> {
    e.params.rich(param).unwrap_or_default().iter().map(|p| p.runs.iter().map(|r| r.t.as_str()).collect()).collect()
}

#[test]
fn element_of_a_text_box_is_a_text_element() {
    let (e, losses, fallback) = convert(&text_item("t", BOX, "Bonjour"));
    let e = e.unwrap();
    assert_that!((e.id.as_str(), e.kind.as_str())).is_equal_to(("e1", "text"));
    assert_that!(texts(&e, "body")).is_equal_to(vec!["Bonjour".to_owned()]);
    assert_that!(losses).is_empty();
    assert_that!(fallback).is_none();
}

#[test]
fn element_of_an_empty_text_box_is_nothing() {
    let (e, losses, _) = convert(&item("t", BOX, ItemKind::Text(text_box(vec![para(vec![])]))));
    assert_that!(e).is_none();
    assert_that!(losses).is_empty();
}

#[test]
fn element_reports_text_losses_on_the_item() {
    let tb = text_box(vec![para(vec![run_with("souligné", |s| s.underline = true)])]);
    let (_, losses, _) = convert(&item("t", BOX, ItemKind::Text(tb)));
    assert_that!(losses).is_equal_to(vec![Loss::on("t", LossKind::RunStyle(RunEffect::Underline))]);
}

#[test]
fn element_of_a_filled_shape_with_text_is_a_card() {
    let (e, _, _) =
        convert(&shape_item("s", "rect", solid(Styled::explicit(Rgba::new(0xf7, 0xf5, 0xf0))), None, Some("Note")));
    let e = e.unwrap();
    assert_that!(e.kind.as_str()).is_equal_to("card");
    assert_that!(texts(&e, "body")).is_equal_to(vec!["Note".to_owned()]);
    assert_that!(e.params.value("highlight")).is_none();
}

#[test]
fn element_of_a_shape_filled_with_the_accent_is_a_highlighted_card() {
    let (e, _, _) = convert(&shape_item("s", "roundRect", solid(accent1()), None, Some("Clé")));
    assert_that!(e.unwrap().params.value("highlight")).is_equal_to(Some(&Value::Bool(true)));
}

#[test]
fn element_of_an_outlined_shape_with_text_is_a_text_element() {
    let stroke = Stroke { width: 1.0, color: accent1(), dash: Dash::Solid, head: None, tail: None };
    let (e, _, _) = convert(&shape_item("s", "rect", Fill::None, Some(stroke), Some("Encadré")));
    assert_that!(e.unwrap().kind.as_str()).is_equal_to("text");
}

#[rstest]
#[case("rect", "rect")]
#[case("roundRect", "rect")]
#[case("ellipse", "ellipse")]
#[case("line", "line")]
#[case("straightConnector1", "line")]
#[case("bentConnector3", "line")]
fn element_of_a_shape_without_text_maps_its_preset(#[case] preset: &str, #[case] expected: &str) {
    let (e, _, _) = convert(&shape_item("s", preset, solid(accent1()), None, None));
    let e = e.unwrap();
    assert_that!(e.kind.as_str()).is_equal_to("shape");
    assert_that!(e.params.text("shape")).is_equal_to(Some(expected));
}

#[test]
fn element_of_a_shape_projects_its_fill_and_stroke_on_tokens() {
    let stroke = Stroke {
        width: 2.0,
        color: Styled::scheme(Rgba::new(0, 0, 0), "dk1"),
        dash: Dash::Solid,
        head: None,
        tail: None,
    };
    let (e, losses, _) = convert(&shape_item("s", "rect", solid(accent1()), Some(stroke), None));
    let e = e.unwrap();
    assert_that!((e.style.fill.as_deref(), e.style.stroke.as_deref())).is_equal_to((Some("accent"), Some("ink")));
    assert_that!(losses).is_empty();
}

#[test]
fn element_of_a_shape_drops_a_fill_without_close_token() {
    let stroke = Stroke {
        width: 2.0,
        color: Styled::scheme(Rgba::new(0, 0, 0), "dk1"),
        dash: Dash::Solid,
        head: None,
        tail: None,
    };
    let red = solid(Styled::explicit(Rgba::new(255, 0, 0)));
    let (e, losses, _) = convert(&shape_item("s", "rect", red, Some(stroke), None));
    let e = e.unwrap();
    assert_that!((e.style.fill.as_deref(), e.style.stroke.as_deref())).is_equal_to((None, Some("ink")));
    assert_that!(losses).is_equal_to(vec![Loss::on("s", LossKind::ColorDropped("#ff0000".into()))]);
}

#[test]
fn element_of_an_invisible_shape_is_nothing() {
    assert_that!(convert(&shape_item("s", "rect", Fill::None, None, None)).0).is_none();
}

#[test]
fn element_of_a_shape_filled_with_a_picture_is_an_image() {
    let e = convert(&shape_item("s", "rect", Fill::Picture { asset: png() }, None, None)).0.unwrap();
    assert_that!((e.kind.as_str(), e.params.text("asset"))).is_equal_to(("image", Some(png().as_str())));
}

#[test]
fn element_of_a_picture_is_an_image_with_the_same_asset() {
    let e = convert(&picture(&png())).0.unwrap();
    assert_that!(e.kind.as_str()).is_equal_to("image");
    assert_that!(e.params.text("asset")).is_equal_to(Some(png().as_str()));
    assert_that!(e.params.text("fit")).is_equal_to(Some("contain"));
}

#[test]
fn element_drops_an_image_the_renderer_cannot_draw() {
    let (e, losses, _) = convert(&picture(&emf()));
    assert_that!(e).is_none();
    assert_that!(losses).is_equal_to(vec![Loss::on("p", LossKind::UnsupportedImage("image/emf".into()))]);
}

#[test]
fn element_of_a_picture_without_asset_is_nothing() {
    assert_that!(convert(&picture("sha256:absent")).0).is_none();
}

#[test]
fn element_reports_crop_rotation_and_flip_as_one_transform_loss() {
    let mut p = picture(&png());
    p.frame.rot = 90.0;
    p.frame.flip_h = true;
    if let ItemKind::Picture(pic) = &mut p.kind {
        pic.crop = Crop { l: 0.1, ..Crop::default() };
    }
    let (e, losses, _) = convert(&p);
    assert_that!(e).is_some();
    assert_that!(losses).is_equal_to(vec![Loss::on("p", LossKind::Transform)]);
}

#[test]
fn element_of_a_table_is_a_table_element() {
    let cell = source::Cell {
        text: text_box(vec![para(vec![run("A")])]),
        fill: Fill::None,
        borders: source::Borders::default(),
        row_span: 1,
        col_span: 1,
        covered: false,
    };
    let t = source::Table {
        col_widths: vec![100.0],
        row_heights: vec![20.0],
        flags: source::TableFlags::default(),
        rows: vec![vec![cell]],
    };
    let e = convert(&item("t", BOX, ItemKind::Table(t))).0.unwrap();
    assert_that!(e.kind.as_str()).is_equal_to("table");
    assert_that!(e.params.value("data").map(|d| d["dataset"]["source"].clone()))
        .is_equal_to(Some(serde_json::json!([["A"]])));
}

#[test]
fn element_of_a_chart_is_a_chart_element_with_its_losses() {
    let c = source::Chart {
        kind: source::ChartKind::Column,
        grouping: source::Grouping::PercentStacked,
        title: None,
        categories: vec!["a".into()],
        series: vec![source::Series { name: "S".into(), values: vec![Some(1.0)], color: None, x: None, format: None }],
    };
    let (e, losses, _) = convert(&item("c", BOX, ItemKind::Chart(c)));
    assert_that!(e.unwrap().kind.as_str()).is_equal_to("chart");
    assert_that!(losses).is_equal_to(vec![Loss::on("c", LossKind::PercentStacked)]);
}

#[rstest]
#[case(Some(png()), true)]
#[case(None, false)]
fn element_of_an_opaque_item_uses_its_fallback_image_and_reports_it(
    #[case] fallback: Option<String>,
    #[case] image: bool,
) {
    let o = item("o", BOX, ItemKind::Opaque(Opaque { reason: OpaqueReason::Ole, fallback }));
    let (e, losses, _) = convert(&o);
    assert_that!(e.map(|e| e.kind)).is_equal_to(image.then(|| "image".to_owned()));
    assert_that!(losses).is_equal_to(vec![Loss::on("o", LossKind::Opaque(OpaqueReason::Ole))]);
}

#[test]
fn element_of_a_shape_whose_fill_and_stroke_are_dropped_is_nothing() {
    let red = || Styled::explicit(Rgba::new(255, 0, 0));
    let stroke = Stroke { width: 1.0, color: red(), dash: Dash::Solid, head: None, tail: None };
    let (e, losses, _) = convert(&shape_item("s", "rect", solid(red()), Some(stroke), None));
    assert_that!(e).is_none();
    assert_that!(losses).is_equal_to(vec![Loss::on("s", LossKind::ColorDropped("#ff0000".into()))]);
}

/// Un filet de 200 × 2 pt : moins d'un quart d'une ligne de grille (40,8 pt) ;
/// son fond `accent1` devient le trait d'une ligne.
#[test]
fn element_of_a_thin_shape_is_a_line_stroked_with_its_fill() {
    let mut bar = shape_item("s", "rect", solid(accent1()), None, None);
    bar.frame.h = 2.0;
    let e = convert(&bar).0.unwrap();
    assert_that!((e.params.text("shape"), e.style.stroke.as_deref(), e.style.fill.as_deref())).is_equal_to((
        Some("line"),
        Some("accent"),
        None,
    ));
}

fn sized(id: &str, frame: (f64, f64, f64, f64), text: &str, size: f64) -> Item {
    item(id, frame, ItemKind::Text(text_box(vec![para(vec![run_with(text, |s| s.size = size)])])))
}

/// Une carte d'alto (slide 4) : fond blanc, surtitre 9,5 pt, titre 13 pt, sous-titre 9,5 pt, corps 10,5 pt.
fn pillar() -> (Item, Vec<Item>) {
    let ground = shape_item("c", "roundRect", solid(Styled::explicit(Rgba::new(255, 255, 255))), None, None);
    let texts = vec![
        sized("k", (16.0, 18.0, 180.0, 15.0), "PILLAR 1 — PRODUCT", 9.5),
        sized("t", (16.0, 39.0, 180.0, 18.0), "Amundi Funds", 13.0),
        sized("s", (16.0, 63.0, 180.0, 15.0), "Investment products", 9.5),
        sized("b", (16.0, 84.0, 180.0, 52.0), "A manufacturing capability", 10.5),
    ];
    (ground, texts)
}

#[test]
fn element_of_a_card_unit_takes_its_kicker_title_and_body() {
    let (ground, texts) = pillar();
    let unit = Unit::Card { shape: &ground, texts: texts.iter().collect(), inner: Vec::new() };
    let e = convert_unit(&unit).0.unwrap();
    assert_that!((e.kind.as_str(), e.params.text("kicker"), e.params.text("title"))).is_equal_to((
        "card",
        Some("PILLAR 1 — PRODUCT"),
        Some("Amundi Funds"),
    ));
    assert_that!(texts_of(&e))
        .is_equal_to(vec!["Investment products".to_owned(), "A manufacturing capability".to_owned()]);
    assert_that!(e.params.value("highlight")).is_none();
}

fn texts_of(e: &Element) -> Vec<String> {
    texts(e, "body")
}

#[test]
fn element_of_a_card_unit_with_a_single_text_keeps_it_as_body() {
    let (ground, texts) = pillar();
    let unit = Unit::Card { shape: &ground, texts: vec![&texts[1]], inner: Vec::new() };
    let e = convert_unit(&unit).0.unwrap();
    assert_that!((e.params.text("title"), texts_of(&e))).is_equal_to((None, vec!["Amundi Funds".to_owned()]));
}

/// Fond `accent1` → `accent` (`#3f6b5e`), 5,9:1 sur le fond d'une carte : relief.
/// Le gris pâle `#b9c4d6` (→ `rule`) se lit sur le relief `ink` (10,1:1) : émis.
#[test]
fn element_of_a_dark_card_unit_is_highlighted_and_colors_against_it() {
    let ground = shape_item("c", "roundRect", solid(accent1()), None, None);
    let pale = item(
        "b",
        (16.0, 40.0, 180.0, 40.0),
        ItemKind::Text(text_box(vec![para(vec![run_with("Pâle", |s| {
            s.color = Styled::explicit(Rgba::new(0xb9, 0xc4, 0xd6));
        })])])),
    );
    let e = convert_unit(&Unit::Card { shape: &ground, texts: vec![&pale], inner: Vec::new() }).0.unwrap();
    assert_that!(e.params.value("highlight")).is_equal_to(Some(&Value::Bool(true)));
    let color = e.params.rich("body").unwrap()[0].runs[0].color.clone();
    assert_that!(color.as_deref()).is_equal_to(Some("rule"));
}

#[test]
fn element_of_merged_texts_puts_their_paragraphs_end_to_end() {
    let (a, b) = (text_item("a", BOX, "Distribution"), text_item("b", BOX, "Matthieu"));
    let e = convert_unit(&Unit::Text(vec![&a, &b])).0.unwrap();
    assert_that!((e.kind.as_str(), texts_of(&e)))
        .is_equal_to(("text", vec!["Distribution".to_owned(), "Matthieu".to_owned()]));
}

#[test]
fn element_of_every_kind_is_accepted_by_add_slide() {
    let items = [
        text_item("t", BOX, "Texte"),
        shape_item("s1", "rect", solid(accent1()), None, Some("Carte")),
        shape_item("s2", "ellipse", solid(accent1()), None, None),
        picture(&png()),
    ];
    let (ground, texts) = pillar();
    let (tokens, assets) = (colors(PAPIER), assets());
    let cx = ContentCx { tokens: &tokens, assets: &assets, track: TRACK };
    let mut units: Vec<Unit<'_>> = items.iter().map(Unit::Item).collect();
    units.push(Unit::Card { shape: &ground, texts: texts.iter().collect(), inner: Vec::new() });
    let blocks: Vec<Element> =
        units.iter().enumerate().filter_map(|(i, u)| element(u, format!("e{i}"), &cx).element).collect();
    assert_that!(blocks).has_length(units.len());
    assert_that!(apply_blocks(blocks)).is_ok();
}

#[test]
fn element_of_an_arrowed_connector_loses_its_arrowheads() {
    let arrow = Some(Arrow { kind: ArrowKind::Triangle, width: ArrowSize::Medium, length: ArrowSize::Medium });
    let stroke = Stroke { width: 1.0, color: accent1(), dash: Dash::Solid, head: None, tail: arrow };
    let (e, losses, _) = convert(&shape_item("s", "straightConnector1", Fill::None, Some(stroke), None));
    assert_that!(e.is_some()).is_true();
    assert_that!(losses.iter().any(|l| l.kind == LossKind::ShapeStyle(ShapeEffect::Arrow))).is_true();
}

#[test]
fn effect_items_keeps_shadows_and_3d_but_not_group_transforms() {
    let warn = |item: &str, e: &str| crate::source::Warning {
        slide: Some(1),
        item: Some(item.to_owned()),
        kind: crate::source::WarningKind::EffectIgnored(e.to_owned()),
    };
    let warnings = [warn("s1/1", "effectLst"), warn("s1/2", "sp3d"), warn("s1/3", "group rotation")];
    assert_that!(effect_items(&warnings))
        .is_equal_to(std::collections::BTreeSet::from(["s1/1".to_owned(), "s1/2".to_owned()]));
}
