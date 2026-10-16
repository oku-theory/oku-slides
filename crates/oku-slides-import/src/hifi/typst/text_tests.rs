use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::hifi::typst::mod_tests::Fixture;
use crate::source::{Item, ItemKind, Rgba, Styled};
use crate::test_support::{bullet, item, para, run, run_with, text_box};

fn text_item(tb: TextBox) -> Item {
    item("t", (10.0, 20.0, 400.0, 200.0), ItemKind::Text(tb))
}

#[rstest]
#[case("arabicPeriod", 3, "3.")]
#[case("arabicParenR", 3, "3)")]
#[case("arabicParenBoth", 12, "(12)")]
#[case("arabicPlain", 7, "7")]
#[case("alphaLcPeriod", 3, "c.")]
#[case("alphaUcParenR", 28, "BB)")]
#[case("romanLcPeriod", 4, "iv.")]
#[case("romanUcPeriod", 1994, "MCMXCIV.")]
#[case("thaiAlphaPeriod", 2, "2.")]
fn autonum_formats_the_ooxml_scheme(#[case] scheme: &str, #[case] n: u32, #[case] expected: &str) {
    assert_that!(autonum(scheme, n)).is_equal_to(expected.to_owned());
}

#[test]
fn autonum_of_a_huge_alpha_number_stays_short() {
    assert_that!(autonum("alphaUcPlain", u32::MAX).len()).is_less_than_or_equal_to(ALPHA_MAX as usize);
}

#[test]
fn numbering_follows_its_level_and_restarts_after_another_paragraph() {
    let numbered = |level: u8| {
        let mut p = para(vec![run("x")]);
        p.level = level;
        p.style.bullet = Bullet::AutoNum { scheme: "arabicPeriod".into(), start: 1 };
        p
    };
    let mut n = Numbering::default();
    let seq: Vec<Option<String>> =
        [numbered(0), numbered(1), numbered(1), numbered(0), para(vec![run("y")]), numbered(0)]
            .iter()
            .map(|p| n.next(p))
            .collect();
    let s = |v: &str| Some(v.to_owned());
    assert_that!(seq).is_equal_to(vec![s("1."), s("1."), s("2."), s("2."), None, s("1.")]);
}

#[test]
fn text_box_draws_paragraphs_runs_and_bullets() {
    let mut title = para(vec![
        run_with("Gras ", |s| s.bold = true),
        run_with("italique ", |s| s.italic = true),
        run_with("souligné ", |s| s.underline = true),
        run_with("barré", |s| s.strike = true),
        run("\n"),
        run_with("2", |s| s.baseline = 30),
        run_with("capitales", |s| s.caps = Caps::All),
        run_with("petites", |s| s.caps = Caps::Small),
        run_with("surligné", |s| s.highlight = Some(Styled::explicit(Rgba::new(255, 255, 0)))),
        run_with("lien", |s| s.link = Some("https://example.com/?a=\"b\"".into())),
        run_with("espacé", |s| s.char_spacing = 1.5),
        run_with("Inconnue", |s| s.font = Styled::explicit("Police Absente".into())),
    ]);
    title.style.align = Align::Center;
    title.style.line_spacing = Spacing::Points(30.0);
    title.style.space_after = Spacing::Points(6.0);
    let mut item1 = bullet(1, vec![run("puce")]);
    item1.style.margin_left = 27.0;
    item1.style.indent = -13.5;
    item1.style.space_before = Spacing::Percent(0.5);
    let mut num = para(vec![run("numéro")]);
    num.style.bullet = Bullet::AutoNum { scheme: "alphaLcParenR".into(), start: 1 };
    num.style.indent = 18.0;
    num.style.align = Align::Justify;
    let mut tb = text_box(vec![title, item1, para(Vec::new()), num]);
    tb.anchor = Anchor::Middle;
    insta::assert_snapshot!(Fixture::new().decor(&[text_item(tb)]).code);
}

#[test]
fn text_box_shrinks_fonts_and_line_spacing_by_its_autofit() {
    let mut tb = text_box(vec![para(vec![run("réduit")])]);
    tb.autofit = Autofit::Shrink { font_scale: 0.5, line_reduction: 0.2 };
    let code = Fixture::new().decor(&[text_item(tb)]).code;
    // 18 pt × 0,5 × 2 (page) = 18 pt ; pas 1,2 × 0,8 × 18 = 17,28 pt (0,96 em),
    // moins que la police de Carlito (1,221 em) : la ligne de base remonte.
    assert_that!(code).contains("size: 18pt, top-edge: 0.822em, bottom-edge: -0.138em");
    assert_that!(code).contains("leading: 0pt");
}

#[test]
fn text_box_lays_out_its_columns_and_anchor() {
    let mut tb = text_box(vec![para(vec![run("a")])]);
    tb.columns = 2;
    tb.anchor = Anchor::Bottom;
    assert_that!(Fixture::new().decor(&[text_item(tb)]).code).contains("align(bottom, columns(2, stack(");
}

#[test]
fn text_box_does_not_bullet_an_empty_paragraph() {
    let tb = text_box(vec![bullet(0, vec![run("  ")])]);
    assert_that!(Fixture::new().decor(&[text_item(tb)]).code).does_not_contain("\"•\"");
}

/// Inter (`hhea` 1984 et −494 sur 2048, capitales 1490).
const INTER: LineMetrics = LineMetrics { cap_height: 0.7275, ascender: 0.96875, descender: 0.2421875 };

#[test]
fn line_box_centres_the_font_in_the_line_step() {
    let page = Page::new(crate::source::Size { w: 1920.0, h: 1080.0 });
    // Titre d'alto, 28 pt à 32,2 pt exacts : la police (33,9 pt) déborde, la
    // ligne de base est à 27,13 − 0,85 pt du haut (mesurée : 26,4 pt).
    let title = LineBox::new(&page, 28.0, Spacing::Points(32.2), 0.0, INTER);
    assert_that!(title.step).is_close_to(32.2, 1e-9);
    assert_that!(title.ascent).is_close_to(26.27, 0.005);
    assert_that!(title.descent()).is_close_to(5.93, 0.005);
    // Un pas plus grand ajoute sa moitié au-dessus de la ligne de base.
    let loose = LineBox::new(&page, 28.0, Spacing::Points(42.2), 0.0, INTER);
    assert_that!(loose.ascent - title.ascent).is_close_to(5.0, 1e-9);
}

#[test]
fn line_box_edges_are_in_em_of_the_paragraph_size() {
    let page = Page::new(crate::source::Size { w: 1920.0, h: 1080.0 });
    let line = LineBox::new(&page, 20.0, Spacing::Percent(1.0), 0.0, INTER);
    // 24 pt de pas : 0,963 em au-dessus de la ligne de base, 0,237 em dessous.
    assert_that!(line.edges(20.0)).is_equal_to("top-edge: 0.963em, bottom-edge: -0.237em".to_owned());
    assert_that!(line.edges(0.0)).is_equal_to("top-edge: 0em, bottom-edge: 0em".to_owned());
}

#[rstest]
#[case(false, 400, None)]
#[case(true, 700, Some("\"bold\""))]
#[case(true, 400, Some("\"bold\""))]
#[case(false, 600, Some("600"))]
#[case(false, 300, Some("300"))]
#[case(true, 900, Some("900"))]
fn weight_arg_keeps_bold_and_emits_other_weights(
    #[case] bold: bool,
    #[case] weight: u16,
    #[case] expected: Option<&str>,
) {
    let s = RunStyle { bold, weight, ..crate::test_support::run_style() };
    assert_that!(weight_arg(&s)).is_equal_to(expected.map(str::to_owned));
}

#[test]
fn cjk_run_falls_back_on_its_east_asian_font() {
    let fixture = Fixture::with_fonts(&["Calibri", "Caladea"]);
    let r = run_with("日本語", |s| s.font_ea = Some(Styled::explicit("Caladea".into())));
    let code = fixture.decor(&[text_item(text_box(vec![para(vec![r])]))]).code;
    assert_that!(code).contains(r#"text(font: ("Carlito", "Caladea"), "#);
}

#[test]
fn run_without_retained_script_font_keeps_its_latin_font() {
    let r = run_with("日本語", |s| s.font_ea = Some(Styled::explicit("Police Absente".into())));
    let code = Fixture::new().decor(&[text_item(text_box(vec![para(vec![r])]))]).code;
    assert_that!(code).contains(r#"text(font: ("Carlito",), "#);
}

#[test]
fn arabic_paragraph_is_right_to_left_in_its_language_and_complex_font() {
    let fixture = Fixture::with_fonts(&["Calibri", "Noto Sans Arabic"]);
    let r = run_with("مرحبا", |s| {
        s.font_cs = Some(Styled::explicit("Noto Sans Arabic".into()));
        s.lang = Some("ar-SA".into());
    });
    let mut p = para(vec![r]);
    p.style.rtl = true;
    let code = fixture.decor(&[text_item(text_box(vec![p]))]).code;
    assert_that!(code).contains(", dir: rtl)");
    assert_that!(code).contains(r#"text(font: ("Carlito", "Noto Sans Arabic"), lang: "ar", region: "SA", "#);
}

/// Ce qu'un fichier altéré glisse dans un texte, ailleurs que dans le texte
/// des runs, reste dans un littéral de chaîne Typst.
const INJECTED: &str = r#"") #panic() (""#;

#[rstest]
#[case::link(para(vec![run_with("lien", |s| s.link = Some(format!("https://x.test/{INJECTED}")))]))]
#[case::bullet_char({
    let mut p = para(vec![run("puce")]);
    p.style.bullet = crate::source::Bullet::Char { char: INJECTED.into(), color: None, size_pct: 1.0, font: None };
    p
})]
#[case::language(para(vec![run_with("langue", |s| s.lang = Some(format!("fr-{INJECTED}")))]))]
fn text_strings_from_the_file_cannot_inject_typst(#[case] p: crate::source::Paragraph) {
    let code = Fixture::new().decor(&[text_item(text_box(vec![p]))]).code;
    assert_that!(code).does_not_contain(INJECTED);
}
