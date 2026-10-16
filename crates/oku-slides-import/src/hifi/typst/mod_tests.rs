use std::collections::BTreeMap;
use std::fmt::Write as _;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::fonts::{FontPlan, resolve};
use crate::hifi::theme::{theme_colors, theme_master};
use crate::source::{
    Arrow, ArrowKind, ArrowSize, Dash, Fill, Geometry, GradientStop, Opaque, PathCmd, Picture, Rgba, Shape, SourceDeck,
    Stroke, Styled, SubPath, TextBox,
};
use crate::test_support::{app_fonts, fixture, item, office_theme, para, run, text_box};

/// Contexte de génération : page 960 × 540 (échelle 2), thème Office, Calibri
/// (Carlito, police de l'app), assets `sha256:aa` (PNG) et `sha256:ee` (EMF).
pub(crate) struct Fixture {
    page: Page,
    colors: ThemeColors,
    fonts: FontPlan,
    assets: BTreeMap<String, Asset>,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        Self::with_fonts(&["Calibri"])
    }

    /// Le contexte où l'import retient les familles `families`.
    pub(crate) fn with_fonts(families: &[&str]) -> Self {
        let asset = |mime: &str| Asset { mime: mime.to_owned(), bytes: Vec::new() };
        Self {
            page: Page::new(crate::source::Size { w: 960.0, h: 540.0 }),
            colors: ThemeColors::new(&office_theme()),
            fonts: resolve(&[], families, &app_fonts()),
            assets: BTreeMap::from([
                ("sha256:aa".to_owned(), asset("image/png")),
                ("sha256:ee".to_owned(), asset("image/x-emf")),
            ]),
        }
    }

    pub(crate) fn ctx(&self) -> GenCtx<'_> {
        GenCtx {
            page: &self.page,
            colors: &self.colors,
            fonts: &self.fonts,
            assets: &self.assets,
            theme_fonts: None,
            lang: None,
        }
    }

    pub(crate) fn decor(&self, items: &[Item]) -> Decor {
        decor(items, &self.ctx())
    }
}

/// Couleur de l'emplacement `slot` du thème Office, sans modificateur.
fn scheme(slot: &str) -> crate::source::Color {
    Styled::scheme(office_theme().colors[slot], slot)
}

fn solid(slot: &str) -> Fill {
    Fill::Solid { color: scheme(slot) }
}

fn preset(name: &str, adj: &[(&str, f64)]) -> Geometry {
    Geometry::Preset { name: name.to_owned(), adj: adj.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect() }
}

fn shape(id: &str, geom: Geometry, fill: Fill, stroke: Option<Stroke>) -> Item {
    item(
        id,
        (10.0, 20.0, 100.0, 50.0),
        ItemKind::Shape(Box::new(Shape { geom, fill, stroke, text: None, connection: None })),
    )
}

fn stroke(width: f64, dash: Dash) -> Stroke {
    Stroke { width, color: Styled::explicit(Rgba::new(0x12, 0x34, 0x56)), dash, head: None, tail: None }
}

#[rstest]
#[case(12.0, "12")]
#[case(-0.001, "0")]
#[case(1.0 / 3.0, "0.33")]
#[case(-2.675, "-2.68")]
#[case(123_456.789, "123456.79")]
#[case(1e-9, "0")]
#[case(f64::NAN, "0")]
#[case(f64::INFINITY, "0")]
fn num_writes_fixed_notation_to_the_hundredth(#[case] v: f64, #[case] expected: &str) {
    assert_that!(num(v)).is_equal_to(expected.to_owned());
}

#[rstest]
#[case("simple", r#""simple""#)]
#[case(r#"a "b" \c"#, r#""a \"b\" \\c""#)]
#[case("l1\nl2\r\t", r#""l1\nl2\r\t""#)]
#[case("\u{7}", r#""\u{7}""#)]
#[case("#import $x$ @r <l> [c] *_~`", r##""#import $x$ @r <l> [c] *_~`""##)]
fn string_escapes_quotes_backslashes_and_controls(#[case] s: &str, #[case] expected: &str) {
    assert_that!(string(s)).is_equal_to(expected.to_owned());
}

#[test]
fn typst_buffer_indents_by_level() {
    let mut w = Typst::new();
    w.open("f(");
    w.line("x,");
    w.close(")");
    assert_that!(w.finish()).is_equal_to("f(\n  x,\n)\n".to_owned());
}

#[test]
fn decor_of_nothing_is_empty() {
    assert_that!(Fixture::new().decor(&[])).is_equal_to(Decor::default());
}

#[test]
fn decor_draws_a_rotated_rect_with_a_dashed_stroke() {
    let mut rect = shape("r", preset("rect", &[]), solid("accent1"), Some(stroke(1.5, Dash::Dash)));
    rect.frame.rot = 30.0;
    insta::assert_snapshot!(Fixture::new().decor(&[rect]).code);
}

#[test]
fn decor_draws_a_mirrored_round_rect_with_its_radius() {
    let mut rect = shape("r", preset("roundRect", &[("adj", 25_000.0)]), solid("accent2"), None);
    rect.frame.flip_h = true;
    insta::assert_snapshot!(Fixture::new().decor(&[rect]).code);
}

#[test]
fn decor_draws_an_ellipse_with_a_sorted_padded_gradient() {
    let stops = vec![
        GradientStop { pos: 0.8, color: scheme("accent1") },
        GradientStop { pos: 0.2, color: Styled::explicit(Rgba { r: 255, g: 0, b: 0, a: 128 }) },
    ];
    let ellipse = shape("e", preset("ellipse", &[]), Fill::Gradient { angle: 90.0, stops }, None);
    insta::assert_snapshot!(Fixture::new().decor(&[ellipse]).code);
}

#[test]
fn decor_gradient_of_one_stop_is_solid() {
    let fill = Fill::Gradient { angle: 0.0, stops: vec![GradientStop { pos: 0.5, color: scheme("accent3") }] };
    let code = Fixture::new().decor(&[shape("e", preset("ellipse", &[]), fill, None)]).code;
    assert_that!(code).contains(r#"fill: color-of(t, "accent-3")"#);
}

#[rstest]
#[case(false, false, "start: (0pt, 0pt), end: (200pt, 100pt)")]
#[case(true, false, "start: (200pt, 0pt), end: (0pt, 100pt)")]
#[case(false, true, "start: (0pt, 100pt), end: (200pt, 0pt)")]
fn decor_draws_a_line_on_its_mirrored_diagonal(#[case] flip_h: bool, #[case] flip_v: bool, #[case] ends: &str) {
    let mut line = shape("l", preset("line", &[]), Fill::None, Some(stroke(1.0, Dash::Solid)));
    (line.frame.flip_h, line.frame.flip_v) = (flip_h, flip_v);
    let code = Fixture::new().decor(&[line]).code;
    assert_that!(code).contains(ends);
    assert_that!(code).does_not_contain("scale(");
}

/// Le trait de [`stroke`] (1 pt, continu) avec des pointes moyennes `head` et `tail`.
fn arrowed(head: Option<ArrowKind>, tail: Option<ArrowKind>) -> Stroke {
    let arrow = |kind| Arrow { kind, width: ArrowSize::Medium, length: ArrowSize::Medium };
    Stroke { head: head.map(arrow), tail: tail.map(arrow), ..stroke(1.0, Dash::Solid) }
}

#[test]
fn decor_line_with_a_tail_triangle_draws_it_filled_at_the_end() {
    let line = shape("l", preset("line", &[]), Fill::None, Some(arrowed(None, Some(ArrowKind::Triangle))));
    let code = Fixture::new().decor(&[line]).code;
    assert_that!(code).contains("curve(fill: color-of(t, \"#123456\"), stroke: none,");
    assert_that!(code).contains("curve.move((200pt, 100pt)),");
    assert_that!(code).contains("curve.close(mode: \"straight\"),");
}

#[test]
fn decor_bent_connector_with_an_open_head_strokes_it_at_the_start() {
    let bent = shape("c", preset("bentConnector3", &[]), Fill::None, Some(arrowed(Some(ArrowKind::Open), None)));
    let code = Fixture::new().decor(&[bent]).code;
    // Le coude part vers la droite : la flèche ouverte, tracée, regarde vers la gauche depuis (0, 0).
    assert_that!(code).contains("curve(fill: none, stroke: (paint: color-of(t, \"#123456\"), thickness: 2pt),");
    assert_that!(code).contains("curve.line((0pt, 0pt)),");
}

#[test]
fn decor_line_without_stroke_draws_nothing() {
    let line = shape("l", preset("line", &[]), Fill::None, None);
    assert_that!(Fixture::new().decor(&[line]).code).is_empty();
}

#[test]
fn decor_draws_another_preset_as_a_curve() {
    let arrow = shape("a", preset("rightArrow", &[]), solid("accent6"), Some(stroke(1.0, Dash::Dot)));
    insta::assert_snapshot!(Fixture::new().decor(&[arrow]).code);
}

#[test]
fn decor_draws_filled_and_stroked_parts_of_a_custom_path_apart() {
    let sub = |fill, stroke| SubPath {
        cmds: vec![
            PathCmd::MoveTo { x: 0.0, y: 0.0 },
            PathCmd::LineTo { x: 1.0, y: 0.0 },
            PathCmd::QuadTo { x1: 1.0, y1: 1.0, x: 0.0, y: 1.0 },
            PathCmd::Close,
        ],
        fill,
        stroke,
    };
    let mut stroked = sub(false, true);
    stroked.cmds.truncate(3);
    let geom = Geometry::Path { paths: vec![sub(true, false), stroked] };
    let path = shape("p", geom, solid("accent1"), Some(stroke(2.0, Dash::LongDash)));
    insta::assert_snapshot!(Fixture::new().decor(&[path]).code);
}

#[test]
fn decor_unknown_preset_draws_its_frame() {
    let code = Fixture::new().decor(&[shape("u", preset("noSuchShape", &[]), solid("accent1"), None)]).code;
    assert_that!(code).contains("rect(width: 200pt, height: 100pt");
}

#[test]
fn decor_draws_a_cropped_picture_and_records_its_asset() {
    let crop = Crop { l: 0.1, t: 0.2, r: 0.3, b: 0.0 };
    let pic = item("i", (0.0, 0.0, 300.0, 200.0), ItemKind::Picture(Picture { asset: "sha256:aa".into(), crop }));
    let d = Fixture::new().decor(&[pic]);
    insta::assert_snapshot!(d.code);
    assert_that!(d.assets.iter().collect::<Vec<_>>()).is_equal_to(vec![&"sha256:aa".to_owned()]);
}

#[test]
fn decor_fills_an_ellipse_with_a_picture_tile() {
    let ellipse = shape("e", preset("ellipse", &[]), Fill::Picture { asset: "sha256:aa".into() }, None);
    let d = Fixture::new().decor(&[ellipse]);
    insta::assert_snapshot!(d.code);
    assert_that!(d.assets.contains("sha256:aa")).is_true();
}

#[test]
fn decor_skips_an_undrawable_picture_with_a_loss() {
    let pic = |id: &str| {
        item(
            id,
            (0.0, 0.0, 10.0, 10.0),
            ItemKind::Picture(Picture { asset: "sha256:ee".into(), crop: Crop::default() }),
        )
    };
    let fill = shape("f", preset("rect", &[]), Fill::Picture { asset: "sha256:ee".into() }, None);
    let d = Fixture::new().decor(&[pic("i"), fill]);
    let emf = LossKind::UnsupportedImage("image/x-emf".into());
    assert_that!(d.losses).is_equal_to(vec![Loss::on("i", emf.clone()), Loss::on("f", emf)]);
    assert_that!(d.assets.is_empty()).is_true();
    assert_that!(d.code).contains("fill: none");
}

#[test]
fn decor_skips_a_missing_asset_silently() {
    let pic = item(
        "i",
        (0.0, 0.0, 10.0, 10.0),
        ItemKind::Picture(Picture { asset: "sha256:00".into(), crop: Crop::default() }),
    );
    assert_that!(Fixture::new().decor(&[pic])).is_equal_to(Decor::default());
}

#[test]
fn decor_draws_group_children_at_their_absolute_place() {
    let children = vec![
        shape("g/1", preset("rect", &[]), solid("accent1"), None),
        item(
            "g/2",
            (400.0, 300.0, 50.0, 50.0),
            ItemKind::Picture(Picture { asset: "sha256:aa".into(), crop: Crop::default() }),
        ),
    ];
    let group = item("g", (0.0, 0.0, 960.0, 540.0), ItemKind::Group { items: children, smartart: None });
    insta::assert_snapshot!(Fixture::new().decor(&[group]).code);
}

#[test]
fn decor_draws_an_opaque_fallback_and_loses_the_rest() {
    let opaque = |id: &str, fallback: Option<&str>| {
        let o = Opaque { reason: OpaqueReason::Ole, fallback: fallback.map(str::to_owned) };
        item(id, (0.0, 0.0, 100.0, 100.0), ItemKind::Opaque(o))
    };
    let table = item(
        "t",
        (0.0, 0.0, 10.0, 10.0),
        ItemKind::Table(crate::source::Table {
            col_widths: Vec::new(),
            row_heights: Vec::new(),
            flags: Default::default(),
            rows: Vec::new(),
        }),
    );
    let d = Fixture::new().decor(&[opaque("o1", Some("sha256:aa")), opaque("o2", None), table]);
    assert_that!(d.code).contains(r#"(ctx.load)(asset: "sha256:aa", fit: "stretch")"#);
    assert_that!(d.losses).is_equal_to(vec![
        Loss::on("o2", LossKind::Opaque(OpaqueReason::Ole)),
        Loss::on("t", LossKind::Opaque(OpaqueReason::Unknown("table".into()))),
    ]);
}

#[test]
fn decor_text_of_a_shape_sits_in_its_text_rect_and_is_not_mirrored() {
    let mut s = shape("s", preset("ellipse", &[]), solid("accent1"), None);
    s.frame.flip_v = true;
    s.frame.flip_h = true;
    let ItemKind::Shape(sh) = &mut s.kind else { unreachable!() };
    sh.text = Some(text_box(vec![para(vec![run("Dans l'ellipse")])]));
    insta::assert_snapshot!(Fixture::new().decor(&[s]).code);
}

#[test]
fn decor_text_cannot_inject_typst() {
    let evil = "]\"} #import \"/etc\": * \\\" $x$ @ref <lbl> [b] *g* _i_ ` ~ /* //\n#panic()";
    let tb = TextBox { paragraphs: vec![para(vec![run(evil)])], ..TextBox::default() };
    let code = Fixture::new().decor(&[item("x", (0.0, 0.0, 100.0, 50.0), ItemKind::Text(tb))]).code;
    let literal = string(evil);
    assert_that!(code).contains(literal.as_str());
    // Hors du littéral, rien du texte ne subsiste dans le code.
    let outside = code.replace(&literal, "\"\"");
    for fragment in ["#import", "#panic", "$x$", "@ref", "<lbl>", "/*"] {
        assert_that!(outside).does_not_contain(fragment);
    }
    insta::assert_snapshot!(code);
}

/// Le décor de chaque master et de chaque layout de `deck`, ses assets et ses
/// pertes, en un seul texte (commentaires Typst entre les parties).
fn fixture_decor(deck: &SourceDeck) -> String {
    let Some(master) = theme_master(deck) else { panic!("aucun master") };
    let theme = &deck.themes[master.theme];
    let fonts = resolve(&deck.fonts, &[&theme.fonts.major, &theme.fonts.minor], &app_fonts());
    let (page, colors) = (Page::new(deck.size), theme_colors(deck, master));
    let ctx = GenCtx {
        page: &page,
        colors: &colors,
        fonts: &fonts,
        assets: &deck.assets,
        theme_fonts: Some(&theme.fonts),
        lang: None,
    };
    let parts = deck.masters.iter().map(|m| (&m.id, &m.decor)).chain(deck.layouts.iter().map(|l| (&l.id, &l.decor)));
    let mut out = String::new();
    for (id, items) in parts {
        let d = decor(items, &ctx);
        writeln!(out, "// {id} : assets {:?}, pertes {:?}", d.assets, d.losses).unwrap();
        out.push_str(&d.code);
    }
    out
}

#[rstest]
#[case("kitchen")]
#[case("libreoffice.pptx")]
fn decor_of_fixture_masters_and_layouts(#[case] name: &str) {
    let snapshot = format!("{}_decor", name.trim_end_matches(".pptx"));
    insta::assert_snapshot!(snapshot, fixture_decor(&fixture(name)));
}

#[rstest]
#[case(false, (50.0, 50.0, 100.0, 100.0))]
#[case(true, (50.0, -50.0, 100.0, 100.0))]
fn placement_of_a_text_rect_turns_about_the_shape_center(#[case] flip_h: bool, #[case] expected: (f64, f64, f64, f64)) {
    // Moitié droite d'une boîte 100 × 50 tournée de 90° : son centre (75, 25)
    // passe en (50, 50) ; en miroir, la moitié gauche passe en (50, 0). Page × 2.
    let page = Page::new(crate::source::Size { w: 960.0, h: 540.0 });
    let f = Frame { rot: 90.0, flip_h, ..Frame::new(0.0, 0.0, 100.0, 50.0) };
    let at = Placement::text(&page, &f, [0.5, 0.0, 1.0, 1.0]);
    let r = at.frame;
    assert_that!((r.x, r.y, r.w, r.h)).is_equal_to(expected);
    assert_that!((at.rot, at.flip_h)).is_equal_to((90.0, false));
}

#[test]
fn placement_of_a_vertically_mirrored_text_turns_it_over() {
    let page = Page::new(crate::source::Size { w: 960.0, h: 540.0 });
    let f = Frame { rot: 270.0, flip_v: true, ..Frame::new(0.0, 0.0, 100.0, 50.0) };
    let at = Placement::text(&page, &f, [0.0, 0.0, 1.0, 1.0]);
    assert_that!((at.rot, at.flip_v)).is_equal_to((90.0, false));
}

/// Une famille du thème source s'écrit par son rôle (`font-of`), comme dans
/// les éléments : changer les polices du thème du deck atteint le décor. Une
/// autre famille retenue reste en dur ; une introuvable prend le corps.
#[test]
fn decor_writes_the_theme_fonts_by_their_role() {
    let page = Page::new(crate::source::Size { w: 960.0, h: 540.0 });
    let theme = office_theme();
    let colors = ThemeColors::new(&theme);
    let fonts = resolve(&[], &["Calibri Light", "Calibri", "Arial"], &app_fonts());
    let assets = BTreeMap::new();
    let ctx = GenCtx {
        page: &page,
        colors: &colors,
        fonts: &fonts,
        assets: &assets,
        theme_fonts: Some(&theme.fonts),
        lang: None,
    };
    let font = |family: &str| crate::test_support::run_with("x", |s| s.font.value = family.to_owned());
    let tb = text_box(vec![para(vec![font("Calibri Light"), font("calibri"), font("Arial"), font("Inconnue")])]);
    let code = decor(&[item("t", (10.0, 20.0, 400.0, 200.0), ItemKind::Text(tb))], &ctx).code;
    let fonts: Vec<&str> =
        code.match_indices("font: ").map(|(i, _)| code[i + 6..].split(", size").next().unwrap_or("")).collect();
    assert_that!(fonts).is_equal_to(vec![
        "font-of(t, \"display\")",
        "font-of(t, \"body\")",
        "(\"Liberation Sans\",)",
        "font-of(t, \"body\")",
    ]);
}
