use std::collections::{BTreeMap, BTreeSet};

use oku_slides_core::{ListKind, ParamValue, RICH_PARAGRAPHS_MAX};
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::fonts::resolve;
use crate::hifi::color::ThemeColors;
use crate::hifi::element::Placed;
use crate::hifi::element::element_tests::{Setup, checked};
use crate::hifi::page::Page;
use crate::hifi::typst::GenCtx;
use crate::source::{
    Autofit, Bullet, Caps, Insets, ItemKind, Paragraph as SourceParagraph, Rgba, Size, Spacing, Styled,
};
use crate::test_support::{app_fonts, bullet, item, office_theme, para, run, run_with, text_box};

/// Les éléments d'une boîte de texte `s1/1` en (10, 20, 100, 50) pt source,
/// validés par le cœur.
fn convert(tb: TextBox) -> Placed {
    let placed = Setup::new().convert(&item("s1/1", (10.0, 20.0, 100.0, 50.0), ItemKind::Text(tb)));
    checked(&placed.elements, &[]);
    placed
}

/// L'unique élément texte de `tb`.
fn element(tb: TextBox) -> Element {
    let placed = convert(tb);
    assert_that!(placed.elements).has_length(1);
    placed.elements[0].clone()
}

fn losses(tb: TextBox) -> Vec<LossKind> {
    convert(tb).losses.into_iter().map(|l| l.kind).collect()
}

fn body(e: &Element) -> Vec<Paragraph> {
    match e.params.get("body") {
        Some(ParamValue::Rich(p)) => p.clone(),
        other => panic!("body : {other:?}"),
    }
}

fn accent(slot: &str) -> Color {
    Styled::scheme(office_theme().colors[slot], slot)
}

#[test]
fn text_element_writes_the_box_style_anchor_and_insets() {
    let tb = TextBox {
        anchor: Anchor::Bottom,
        insets: Insets { l: 1.0, t: 2.0, r: 3.0, b: 4.0 },
        ..text_box(vec![para(vec![run("Bonjour")])])
    };
    let e = element(tb);
    assert_that!(serde_json::to_value(&e).unwrap()).is_equal_to(json!({
        "id": "s1-1",
        "kind": "text",
        "frame": { "x": 20.0, "y": 40.0, "w": 200.0, "h": 100.0 },
        "style": {
            "size": 36.0, "color": "ink", "font": "body", "align": "left", "leading": 0.56,
            // Marges source plus le recalage des lignes de base (Carlito 36 pt : +10,81 et +9,3).
            "valign": "bottom", "inset": [14.81, 6.0, 17.3, 2.0],
        },
        "params": { "body": [{ "runs": [{ "t": "Bonjour" }] }] },
    }));
}

#[test]
fn text_element_marks_runs_whose_size_or_color_differ_from_the_dominant_one() {
    let big = run_with("Titre ", |s| (s.size, s.color) = (24.0, accent("accent1")));
    let e = element(text_box(vec![para(vec![big, run("et un texte plus long")])]));
    assert_that!(e.style.size).is_equal_to(Some(TokenValue::Number(36.0)));
    let runs = &body(&e)[0].runs;
    assert_that!(runs[0].size).is_equal_to(Some(TokenValue::Number(48.0)));
    assert_that!(runs[0].color.as_deref()).is_equal_to(Some("accent"));
    assert_that!(runs[1].size).is_none();
    assert_that!(runs[1].color).is_none();
}

#[test]
fn text_element_loses_a_run_font_other_than_the_box_font() {
    let other = run_with("Arial", |s| s.font = Styled::explicit("Arial".to_owned()));
    let tb = text_box(vec![para(vec![run("Le corps du texte "), other])]);
    assert_that!(losses(tb)).is_equal_to(vec![LossKind::RunStyle(RunEffect::Font)]);
}

#[test]
fn text_element_puts_bold_and_italic_on_the_box_when_they_cover_everything() {
    let b = |t: &str| run_with(t, |s| (s.bold, s.italic) = (true, true));
    let e = element(text_box(vec![para(vec![b("Tout "), b("en gras")])]));
    assert_that!(e.style.weight).is_equal_to(Some(TokenValue::Name("bold".to_owned())));
    assert_that!(e.style.italic).is_equal_to(Some(true));
    assert_that!(body(&e)[0].runs).is_equal_to(vec![Run::plain("Tout en gras")]);
}

#[test]
fn text_element_keeps_bold_on_the_runs_when_mixed() {
    let e = element(text_box(vec![para(vec![run_with("Gras", |s| s.bold = true), run(" maigre")])]));
    assert_that!(e.style.weight).is_none();
    assert_that!(body(&e)[0].runs[0].bold).is_true();
}

#[test]
fn text_element_applies_capitals_and_loses_underline() {
    let r = run_with("crie", |s| (s.caps, s.underline) = (Caps::All, true));
    let placed = convert(text_box(vec![para(vec![r])]));
    assert_that!(body(&placed.elements[0])[0].runs[0].t.as_str()).is_equal_to("CRIE");
    let kinds: Vec<LossKind> = placed.losses.into_iter().map(|l| l.kind).collect();
    assert_that!(kinds).is_equal_to(vec![LossKind::RunStyle(RunEffect::Underline)]);
}

#[test]
fn text_element_turns_an_accent_highlight_into_its_palette_pastille() {
    let accented = run_with("accent", |s| s.highlight = Some(accent("accent2")));
    let odd = run_with("autre", |s| s.highlight = Some(Styled::explicit(Rgba::new(1, 2, 3))));
    let placed = convert(text_box(vec![para(vec![accented, run(" et "), odd])]));
    let runs = &body(&placed.elements[0])[0].runs;
    assert_that!(runs[0].highlight).is_equal_to(Some(2));
    assert_that!(runs[1].t.as_str()).is_equal_to(" et autre");
    assert_that!(runs[1].highlight).is_none();
    let kinds: Vec<LossKind> = placed.losses.into_iter().map(|l| l.kind).collect();
    assert_that!(kinds).is_equal_to(vec![LossKind::RunStyle(RunEffect::Highlight)]);
}

#[test]
fn text_element_takes_the_dominant_alignment_and_loses_the_others() {
    let mut centered = para(vec![run("Un long paragraphe centré")]);
    centered.style.align = Align::Center;
    let placed = convert(text_box(vec![centered, para(vec![run("À gauche")])]));
    assert_that!(placed.elements[0].style.align.as_deref()).is_equal_to(Some("center"));
    let kinds: Vec<LossKind> = placed.losses.into_iter().map(|l| l.kind).collect();
    assert_that!(kinds).is_equal_to(vec![LossKind::ParagraphStyle]);
}

#[test]
fn text_element_loses_justification() {
    let mut p = para(vec![run("Justifié")]);
    p.style.align = Align::Justify;
    assert_that!(losses(text_box(vec![p]))).is_equal_to(vec![LossKind::ParagraphStyle]);
}

#[test]
fn text_element_writes_a_point_line_spacing_in_em() {
    let mut p = para(vec![run("Serré")]);
    p.style.line_spacing = Spacing::Points(27.0);
    // 27 pt source = 54 pt page de pas, pour une police de 36 pt page : 1,5 em moins
    // les capitales de Carlito (0,642 em).
    assert_that!(element(text_box(vec![p])).style.leading).is_equal_to(Some(TokenValue::Number(0.86)));
}

#[test]
fn text_element_leading_gives_the_powerpoint_line_step_above_the_cap_height() {
    // Simple : une ligne de base tous les 1,2 em ; Typst y compte les capitales
    // (bord haut par défaut) plus l'interligne.
    assert_that!(element(text_box(vec![para(vec![run("Simple")])])).style.leading)
        .is_equal_to(Some(TokenValue::Number(0.56)));
    let mut p = para(vec![run_with("Inconnue", |s| s.font = Styled::explicit("Inconnue".to_owned()))]);
    p.style.line_spacing = Spacing::Percent(1.5);
    // Une famille introuvable prend le corps du thème (Calibri, rendue en Carlito).
    assert_that!(element(text_box(vec![p])).style.leading).is_equal_to(Some(TokenValue::Number(1.16)));
}

/// Un paragraphe `text` de Carlito à 36 pt page espacé de `before` et `after` pt source.
fn spaced(text: &str, before: Spacing, after: Spacing) -> SourceParagraph {
    let mut p = para(vec![run(text)]);
    (p.style.space_before, p.style.space_after) = (before, after);
    p
}

#[test]
fn text_element_spacing_gives_the_powerpoint_gap_between_paragraphs() {
    let none = Spacing::Points(0.0);
    let tb = text_box(vec![
        spaced("Un", Spacing::Points(20.0), Spacing::Points(3.0)),
        spaced("Deux", Spacing::Points(6.0), Spacing::Points(40.0)),
    ]);
    // De la ligne de base du premier au haut des capitales du second : le pas
    // (43,2 pt) moins les capitales (23,1 pt), plus l'espace après du premier
    // et l'espace avant du second (3 + 6 pt source) ; ni l'espace avant du
    // premier ni l'espace après du dernier.
    assert_that!(element(tb).style.spacing).is_equal_to(Some(38.1));
    let tight = text_box(vec![spaced("Un", none, none), spaced("Deux", none, none)]);
    assert_that!(element(tight).style.spacing).is_equal_to(Some(20.1));
}

#[test]
fn text_element_spacing_reads_a_percent_space_in_lines_of_the_paragraph() {
    let none = Spacing::Points(0.0);
    let tb = text_box(vec![spaced("Un", none, none), spaced("Deux", Spacing::Percent(0.5), none)]);
    // Une demi-ligne simple de 36 pt : 21,6 pt.
    assert_that!(element(tb).style.spacing).is_equal_to(Some(41.7));
}

#[test]
fn text_element_spacing_applies_between_list_items() {
    let mut items = vec![bullet(0, vec![run("Un")]), bullet(0, vec![run("Deux")])];
    items[1].style.space_before = Spacing::Points(5.0);
    assert_that!(element(text_box(items)).style.spacing).is_equal_to(Some(30.1));
}

#[test]
fn text_element_of_one_paragraph_has_no_spacing() {
    let tb = text_box(vec![spaced("Seul", Spacing::Points(10.0), Spacing::Points(10.0))]);
    assert_that!(element(tb).style.spacing).is_none();
}

#[test]
fn text_element_takes_the_dominant_gap_and_loses_the_others() {
    let none = Spacing::Points(0.0);
    let tb = text_box(vec![
        spaced("Un", none, none),
        spaced("Deux, le plus long", none, none),
        spaced("Trois", Spacing::Points(10.0), none),
    ]);
    let placed = convert(tb);
    assert_that!(placed.elements[0].style.spacing).is_equal_to(Some(20.1));
    let kinds: Vec<LossKind> = placed.losses.into_iter().map(|l| l.kind).collect();
    assert_that!(kinds).is_equal_to(vec![LossKind::ParagraphStyle]);
}

/// Les marges haute et basse d'un texte de Carlito à 36 pt page, sans marge source, à l'interligne `s`.
fn baseline_insets(s: Spacing) -> (f64, f64) {
    let mut p = para(vec![run("Base")]);
    p.style.line_spacing = s;
    let inset = element(TextBox { insets: Insets { l: 0.0, t: 0.0, r: 0.0, b: 0.0 }, ..text_box(vec![p]) })
        .style
        .inset
        .unwrap();
    (inset[0], inset[2])
}

#[test]
fn text_element_lowers_the_first_baseline_to_the_powerpoint_one() {
    // Ligne simple : pas de 43,2 pt ; Carlito (1,221 em) y déborde de 0,75 pt,
    // la ligne de base est à 34,28 − 0,37 = 33,9 pt du haut, et Typst la pose à
    // la hauteur des capitales (23,1 pt) : 10,81 pt de plus en haut ; en bas,
    // le reste du pas (9,3 pt).
    assert_that!(baseline_insets(Spacing::Percent(1.0))).is_equal_to((10.81, 9.3));
    // Double : la moitié des 43,2 pt de plus au-dessus de la première ligne.
    assert_that!(baseline_insets(Spacing::Percent(2.0))).is_equal_to((32.41, 30.9));
}

#[test]
fn text_element_never_writes_a_negative_inset_for_a_tight_line() {
    // 5 pt source : la ligne de base serait au-dessus des capitales.
    assert_that!(baseline_insets(Spacing::Points(5.0))).is_equal_to((0.0, 0.0));
}

#[test]
fn text_element_flattens_list_levels() {
    let placed = convert(text_box(vec![bullet(0, vec![run("Un")]), bullet(1, vec![run("Deux")])]));
    let lists: Vec<Option<ListKind>> = body(&placed.elements[0]).iter().map(|p| p.list).collect();
    assert_that!(lists).is_equal_to(vec![Some(ListKind::Bullet), Some(ListKind::Bullet)]);
    let kinds: Vec<LossKind> = placed.losses.into_iter().map(|l| l.kind).collect();
    assert_that!(kinds).is_equal_to(vec![LossKind::ListLevels]);
}

#[test]
fn text_element_numbers_an_autonum_paragraph() {
    let mut p = para(vec![run("Premier")]);
    p.style.bullet = Bullet::AutoNum { scheme: "arabicPeriod".into(), start: 1 };
    assert_that!(body(&element(text_box(vec![p])))[0].list).is_equal_to(Some(ListKind::Number));
}

#[test]
fn text_element_scales_sizes_by_the_shrink_autofit() {
    let tb = TextBox {
        autofit: Autofit::Shrink { font_scale: 0.5, line_reduction: 0.0 },
        ..text_box(vec![para(vec![run("Réduit")])])
    };
    assert_that!(element(tb).style.size).is_equal_to(Some(TokenValue::Number(18.0)));
}

#[test]
fn text_element_keeps_line_breaks_in_the_previous_run() {
    let e = element(text_box(vec![para(vec![run("Ligne 1"), run("\n"), run("Ligne 2")])]));
    assert_that!(body(&e)[0].runs).is_equal_to(vec![Run::plain("Ligne 1\nLigne 2")]);
}

#[test]
fn text_element_keeps_safe_links_only() {
    let web = run_with("site", |s| s.link = Some("https://oku.example".into()));
    let script = run_with(" piège", |s| s.link = Some("javascript:alert(1)".into()));
    let runs = body(&element(text_box(vec![para(vec![web, script])])))[0].runs.clone();
    assert_that!(runs[0].link.as_deref()).is_equal_to(Some("https://oku.example"));
    assert_that!(runs[1].link).is_none();
}

#[test]
fn text_element_omits_empty_paragraphs_and_empty_boxes() {
    let e = element(text_box(vec![para(vec![run("Haut")]), para(Vec::new()), para(vec![run("Bas")])]));
    assert_that!(body(&e)).has_length(2);
    assert_that!(convert(text_box(vec![para(vec![run("   ")])])).elements).is_empty();
}

#[test]
fn text_element_bounds_the_rich_text_with_a_loss() {
    let paragraphs: Vec<SourceParagraph> =
        (0..RICH_PARAGRAPHS_MAX + 1).map(|k| para(vec![run(&k.to_string())])).collect();
    let placed = convert(text_box(paragraphs));
    assert_that!(body(&placed.elements[0])).has_length(RICH_PARAGRAPHS_MAX);
    let kinds: Vec<LossKind> = placed.losses.into_iter().map(|l| l.kind).collect();
    assert_that!(kinds).is_equal_to(vec![LossKind::TextTruncated]);
}

#[test]
fn font_name_writes_the_theme_fonts_by_their_role() {
    let fonts = resolve(&[], &["Calibri Light", "Calibri", "Arial"], &app_fonts());
    let (page, theme) = (Page::new(Size { w: 960.0, h: 540.0 }), office_theme());
    let (colors, assets, effects) = (ThemeColors::new(&theme), BTreeMap::new(), BTreeSet::new());
    let ctx = ElementCtx {
        base: GenCtx {
            page: &page,
            colors: &colors,
            fonts: &fonts,
            assets: &assets,
            theme_fonts: Some(&theme.fonts),
            lang: None,
        },
        effects: &effects,
        table: None,
    };
    let names: Vec<Option<String>> =
        ["Calibri Light", "calibri", "Arial", "Inconnue"].iter().map(|f| font_name(f, &ctx)).collect();
    let expected = [Some("display"), Some("body"), Some("Liberation Sans"), None].map(|n| n.map(str::to_owned));
    assert_that!(names).is_equal_to(expected.to_vec());
}

#[test]
fn placeholder_text_marks_runs_against_the_layout_level() {
    let setup = Setup::new();
    let mut level = LevelStyle { para: para(Vec::new()).style, run: crate::test_support::run_style() };
    level.run.size = 24.0;
    let big = run_with("Grand", |s| s.size = 32.0);
    let tb = text_box(vec![para(vec![
        run_with("Corps ", |s| s.size = 24.0),
        big,
        run_with(" gras", |s| {
            (s.size, s.bold) = (24.0, true);
        }),
    ])]);
    let (paragraphs, losses) = placeholder_text(&tb, &level, &setup.ctx());
    let runs: Vec<(&str, Option<TokenValue>, bool)> =
        paragraphs[0].runs.iter().map(|r| (r.t.as_str(), r.size.clone(), r.bold)).collect();
    assert_that!(runs).is_equal_to(vec![
        ("Corps ", None, false),
        ("Grand", Some(TokenValue::Number(64.0)), false),
        (" gras", None, true),
    ]);
    assert_that!(losses).is_empty();
}

#[test]
fn text_element_takes_the_dominant_letter_spacing_as_tracking() {
    let spaced = |t: &str| run_with(t, |s| s.char_spacing = 1.5);
    let e = element(text_box(vec![para(vec![spaced("ALTO "), spaced("WEALTH")])]));
    assert_that!(e.style.tracking).is_equal_to(Some(3.0));
    assert_that!(losses(text_box(vec![para(vec![spaced("ALTO WEALTH")])]))).is_empty();
}

#[test]
fn text_element_without_letter_spacing_has_no_tracking() {
    assert_that!(element(text_box(vec![para(vec![run("Serré")])])).style.tracking).is_none();
}

#[test]
fn text_element_loses_a_run_letter_spacing_other_than_the_box_one() {
    let tight = run_with("court", |s| s.char_spacing = -1.0);
    let tb = text_box(vec![para(vec![run("Le corps du texte "), tight])]);
    assert_that!(element(tb.clone()).style.tracking).is_none();
    assert_that!(losses(tb)).is_equal_to(vec![LossKind::RunStyle(RunEffect::Tracking)]);
}

#[test]
fn placeholder_text_loses_a_run_letter_spacing_other_than_the_level_one() {
    let setup = Setup::new();
    let mut level = LevelStyle { para: para(Vec::new()).style, run: crate::test_support::run_style() };
    level.run.char_spacing = 0.5;
    let same = run_with("Pareil ", |s| s.char_spacing = 0.5);
    let (_, losses) = placeholder_text(&text_box(vec![para(vec![same.clone()])]), &level, &setup.ctx());
    assert_that!(losses).is_empty();
    let other = run_with("autre", |s| s.char_spacing = 2.0);
    let (_, losses) = placeholder_text(&text_box(vec![para(vec![same, other])]), &level, &setup.ctx());
    assert_that!(losses).is_equal_to(vec![LossKind::RunStyle(RunEffect::Tracking)]);
}

#[test]
fn placeholder_text_loses_a_regular_run_on_a_bold_level() {
    let setup = Setup::new();
    let mut level = LevelStyle { para: para(Vec::new()).style, run: crate::test_support::run_style() };
    level.run.bold = true;
    let bold = run_with("Titre ", |s| s.bold = true);
    let (_, losses) = placeholder_text(&text_box(vec![para(vec![bold.clone()])]), &level, &setup.ctx());
    assert_that!(losses).is_empty();
    let (_, losses) = placeholder_text(&text_box(vec![para(vec![bold, run("maigre")])]), &level, &setup.ctx());
    assert_that!(losses).is_equal_to(vec![LossKind::RunStyle(RunEffect::Weight)]);
}

#[test]
fn placeholder_text_loses_an_upright_run_on_an_italic_level() {
    let setup = Setup::new();
    let mut level = LevelStyle { para: para(Vec::new()).style, run: crate::test_support::run_style() };
    level.run.italic = true;
    let italic = run_with("Citation ", |s| s.italic = true);
    let (_, losses) = placeholder_text(&text_box(vec![para(vec![italic.clone()])]), &level, &setup.ctx());
    assert_that!(losses).is_empty();
    let (_, losses) = placeholder_text(&text_box(vec![para(vec![italic, run("droit")])]), &level, &setup.ctx());
    assert_that!(losses).is_equal_to(vec![LossKind::RunStyle(RunEffect::Slant)]);
}

fn script_losses(text: &str, rtl: bool) -> Vec<LossKind> {
    let setup = Setup::with_fonts(&["Calibri", "Caladea"]);
    let r = run_with(text, |s| s.font_ea = Some(Styled::explicit("Caladea".into())));
    let mut p = para(vec![r]);
    p.style.rtl = rtl;
    let placed = setup.convert(&item("s1/1", (10.0, 20.0, 100.0, 50.0), ItemKind::Text(text_box(vec![p]))));
    placed.losses.into_iter().map(|l| l.kind).collect()
}

#[test]
fn cjk_run_with_an_east_asian_font_outside_the_theme_loses_it() {
    assert_that!(script_losses("日本語", false)).is_equal_to(vec![LossKind::RunStyle(RunEffect::Font)]);
}

#[test]
fn latin_run_with_an_east_asian_font_loses_nothing() {
    assert_that!(script_losses("abc", false)).is_empty();
}

#[test]
fn right_to_left_paragraph_in_a_left_to_right_deck_loses_its_direction() {
    assert_that!(script_losses("abc", true)).is_equal_to(vec![LossKind::ParagraphStyle]);
}
