use oku_slides_core::{ParamType, ParamValue, text_fits};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::source::{Origin, Styled};
use crate::test_support::{PAPIER, bullet, colors, para, run, run_with, text_box};

fn papier() -> ColorTokens {
    colors(PAPIER)
}

/// Le fond de page de Papier, `#fffdf9`.
fn page() -> Rgba {
    papier().page()
}

fn texts(out: &TextOut) -> Vec<Vec<&str>> {
    out.paragraphs.iter().map(|p| p.runs.iter().map(|r| r.t.as_str()).collect()).collect()
}

#[test]
fn rich_keeps_bold_and_italic_runs() {
    let tb =
        text_box(vec![para(vec![run_with("Bonjour ", |s| s.bold = true), run_with("monde", |s| s.italic = true)])]);
    let out = rich([&tb], &papier(), page());
    let runs = &out.paragraphs[0].runs;
    assert_that!((runs[0].bold, runs[0].italic, runs[1].bold, runs[1].italic)).is_equal_to((true, false, false, true));
    assert_that!(out.losses).is_empty();
}

#[test]
fn rich_merges_adjacent_runs_of_the_same_format() {
    let tb = text_box(vec![para(vec![run("Un "), run("deux"), run_with(" trois", |s| s.size = 40.0)])]);
    assert_that!(texts(&rich([&tb], &papier(), page()))).is_equal_to(vec![vec!["Un deux trois"]]);
}

#[test]
fn rich_omits_paragraphs_without_text() {
    let tb = text_box(vec![para(vec![run("A")]), para(vec![]), para(vec![run("")]), para(vec![run("B")])]);
    assert_that!(texts(&rich([&tb], &papier(), page()))).is_equal_to(vec![vec!["A"], vec!["B"]]);
}

#[test]
fn rich_keeps_line_breaks_inside_a_paragraph() {
    let tb = text_box(vec![para(vec![run("Ligne 1"), run("\n"), run("Ligne 2")])]);
    assert_that!(texts(&rich([&tb], &papier(), page()))).is_equal_to(vec![vec!["Ligne 1\nLigne 2"]]);
}

#[rstest]
#[case(Bullet::None, None)]
#[case(Bullet::Char { char: "–".into(), color: None, size_pct: 1.0, font: None }, Some(ListKind::Bullet))]
#[case(Bullet::AutoNum { scheme: "arabicPeriod".into(), start: 1 }, Some(ListKind::Number))]
fn rich_maps_bullets_to_lists(#[case] b: Bullet, #[case] expected: Option<ListKind>) {
    let mut p = para(vec![run("Point")]);
    p.style.bullet = b;
    assert_that!(rich([&text_box(vec![p])], &papier(), page()).paragraphs[0].list).is_equal_to(expected);
}

#[test]
fn rich_flattens_list_levels_and_reports_it_once() {
    let tb = text_box(vec![bullet(0, vec![run("Un")]), bullet(1, vec![run("Un.a")]), bullet(2, vec![run("Un.a.i")])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(out.paragraphs.iter().all(|p| p.list == Some(ListKind::Bullet))).is_true();
    assert_that!(out.losses).is_equal_to(vec![LossKind::ListLevels]);
}

#[rstest]
#[case::underline(run_with("x", |s| s.underline = true), RunEffect::Underline)]
#[case::strike(run_with("x", |s| s.strike = true), RunEffect::Strike)]
#[case::superscript(run_with("x", |s| s.baseline = 30), RunEffect::Baseline)]
#[case::subscript(run_with("x", |s| s.baseline = -25), RunEffect::Baseline)]
#[case::small_caps(run_with("x", |s| s.caps = Caps::Small), RunEffect::Caps)]
#[case::highlight(run_with("x", |s| s.highlight = Some(Styled::explicit(Rgba::new(255, 255, 0)))), RunEffect::Highlight)]
fn rich_reports_each_lost_effect_and_keeps_the_text(#[case] r: source::Run, #[case] effect: RunEffect) {
    let tb = text_box(vec![para(vec![r.clone(), r])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(texts(&out)).is_equal_to(vec![vec!["xx"]]);
    assert_that!(out.losses).is_equal_to(vec![LossKind::RunStyle(effect)]);
}

#[rstest]
#[case("https://oku.test/a", Some("https://oku.test/a"))]
#[case("MAILTO:a@b.c", Some("MAILTO:a@b.c"))]
#[case("javascript:alert(1)", None)]
#[case("file:///etc/passwd", None)]
fn rich_keeps_safe_links_only(#[case] link: &str, #[case] expected: Option<&str>) {
    let tb = text_box(vec![para(vec![run_with("lien", |s| s.link = Some(link.to_owned()))])]);
    assert_that!(rich([&tb], &papier(), page()).paragraphs[0].runs[0].link.as_deref()).is_equal_to(expected);
}

fn red() -> source::Color {
    Styled::explicit(Rgba::new(255, 0, 0))
}

fn accent1() -> source::Color {
    Styled::mapped(Rgba::new(0x44, 0x72, 0xc4), "accent1", "accent1", Vec::new())
}

/// Toute la boîte en `accent1` : `accent` (`#3f6b5e`) est lisible sur `bg` (5,9:1) et émis.
#[test]
fn rich_of_a_box_in_one_color_emits_its_readable_token() {
    let tb = text_box(vec![para(vec![run_with("Tout en accent", |s| s.color = accent1())])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(out.paragraphs[0].runs[0].color.as_deref()).is_equal_to(Some("accent"));
    assert_that!(out.losses).is_empty();
}

/// Le gris bleuté `#b9c4d6` d'alto se projette sur `rule` (`#ddd7cc`) : 1,3:1 sur
/// `bg`, illisible, rien n'est émis ; sur le fond sombre `ink` d'une carte en
/// relief (`#2e2b26`), 10,1:1 : `rule` est émis.
#[test]
fn rich_emits_a_pale_color_only_on_a_dark_ground() {
    let tb = text_box(vec![para(vec![run_with("Pâle", |s| s.color = Styled::explicit(Rgba::new(0xb9, 0xc4, 0xd6)))])]);
    let dark = papier().card(true);
    let colors: Vec<Option<String>> =
        [page(), dark].iter().map(|bg| rich([&tb], &papier(), *bg).paragraphs[0].runs[0].color.clone()).collect();
    assert_that!(colors).is_equal_to(vec![None, Some("rule".to_owned())]);
}

#[test]
fn rich_upper_cases_all_caps_runs_without_a_loss() {
    let tb = text_box(vec![para(vec![run_with("Notre réponse", |s| s.caps = Caps::All)])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(texts(&out)).is_equal_to(vec![vec!["NOTRE RÉPONSE"]]);
    assert_that!(out.losses).is_empty();
}

#[test]
fn rich_emits_the_token_of_a_run_that_stands_out() {
    let tb = text_box(vec![para(vec![run("Un mot "), run_with("accentué", |s| s.color = accent1()), run(" ici")])]);
    let runs = &rich([&tb], &papier(), page()).paragraphs[0].runs;
    assert_that!(runs.iter().map(|r| r.color.as_deref()).collect::<Vec<_>>()).is_equal_to(vec![
        None,
        Some("accent"),
        None,
    ]);
}

#[test]
fn rich_drops_a_standing_out_color_without_close_token_and_reports_it() {
    let tb = text_box(vec![para(vec![run("Un mot "), run_with("rouge", |s| s.color = red()), run(" ici")])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(texts(&out)).is_equal_to(vec![vec!["Un mot rouge ici"]]);
    assert_that!(out.losses).is_equal_to(vec![LossKind::ColorDropped("#ff0000".into())]);
}

/// Le corps est en `accent1` ; le mot en `dk1` se projette sur `ink`, la couleur par défaut : rien n'est émis.
#[test]
fn rich_never_emits_the_ink_token() {
    let tb = text_box(vec![para(vec![run_with("Corps accentué ", |s| s.color = accent1()), run("noir")])]);
    let runs = &rich([&tb], &papier(), page()).paragraphs[0].runs;
    assert_that!(runs.iter().map(|r| r.color.as_deref()).collect::<Vec<_>>()).is_equal_to(vec![Some("accent"), None]);
}

#[test]
fn rich_of_a_default_color_emits_nothing() {
    let default = Styled { value: Rgba::new(1, 2, 3), origin: Origin::Default };
    let tb = text_box(vec![para(vec![run("Un "), run_with("défaut", |s| s.color = default)])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(out.paragraphs[0].runs.iter().all(|r| r.color.is_none())).is_true();
    assert_that!(out.losses).is_empty();
}

#[test]
fn rich_keeps_at_most_the_core_paragraph_count() {
    let tb = text_box((0..70).map(|i| para(vec![run(&format!("P{i}"))])).collect());
    let out = rich([&tb], &papier(), page());
    assert_that!(out.paragraphs.len()).is_equal_to(RICH_PARAGRAPHS_MAX);
    assert_that!(out.losses).is_equal_to(vec![LossKind::TextTruncated]);
}

/// Un paragraphe d'un seul run s'écrit `{"runs":[{"t":"…"}]}` (19 octets plus le texte),
/// dans `[…]` (2 octets) : il reste `16384 − 21 = 16363` octets de texte.
#[test]
fn rich_cuts_an_oversized_paragraph_to_the_byte_bound() {
    let tb = text_box(vec![para(vec![run(&"a".repeat(20_000))]), para(vec![run("suite")])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(texts(&out)).has_length(1);
    assert_that!(out.paragraphs[0].runs[0].t.len()).is_equal_to(16_363);
    assert_that!(text_fits(&ParamType::Rich, &ParamValue::Rich(out.paragraphs.clone()))).is_true();
    assert_that!(out.losses).is_equal_to(vec![LossKind::TextTruncated]);
}

/// `é` vaut 2 octets, non échappés en JSON : `⌊(16384 − 21) / 2⌋ = 8181` caractères.
#[test]
fn rich_cuts_multibyte_text_on_a_character_boundary() {
    let tb = text_box(vec![para(vec![run(&"é".repeat(10_000))])]);
    let out = rich([&tb], &papier(), page());
    assert_that!(out.paragraphs[0].runs[0].t.chars().count()).is_equal_to(8181);
    assert_that!(out.paragraphs[0].runs[0].t.chars().all(|c| c == 'é')).is_true();
}

/// Chaque paragraphe pèse `19 + 6000 = 6019` octets : après deux (`2 + 6019 + 1 + 6019 = 12041`),
/// virgule comprise il reste `16384 − 12042 = 4342` octets, soit `4342 − 19 = 4323` octets de texte.
#[test]
fn rich_stops_at_the_paragraph_that_overflows_the_byte_bound() {
    let big = || para(vec![run(&"b".repeat(6_000))]);
    let out = rich([&text_box(vec![big(), big(), big()])], &papier(), page());
    assert_that!(out.paragraphs).has_length(3);
    assert_that!(out.paragraphs[2].runs[0].t.len()).is_equal_to(4_323);
    assert_that!(text_fits(&ParamType::Rich, &ParamValue::Rich(out.paragraphs.clone()))).is_true();
    assert_that!(out.losses).is_equal_to(vec![LossKind::TextTruncated]);
}

#[test]
fn rich_within_bounds_reports_no_truncation() {
    let tb = text_box((0..RICH_PARAGRAPHS_MAX).map(|_| para(vec![run("court")])).collect());
    assert_that!(rich([&tb], &papier(), page()).losses).is_empty();
}

#[test]
fn plain_joins_paragraphs_and_line_breaks_with_single_spaces() {
    let tb = text_box(vec![para(vec![run("  Rapport"), run("\n"), run("annuel ")]), para(vec![run("2026")])]);
    assert_that!(plain(&tb)).is_equal_to(("Rapport annuel 2026".to_owned(), Vec::new()));
}

#[test]
fn plain_of_an_empty_box_is_empty() {
    assert_that!(plain(&text_box(vec![para(vec![])]))).is_equal_to((String::new(), Vec::new()));
}

#[test]
fn plain_upper_cases_all_caps_runs() {
    let tb = text_box(vec![para(vec![run_with("the shift", |s| s.caps = Caps::All)])]);
    assert_that!(plain(&tb)).is_equal_to(("THE SHIFT".to_owned(), Vec::new()));
}

#[test]
fn plain_reports_lost_effects_once() {
    let tb = text_box(vec![para(vec![run_with("A", |s| s.underline = true), run_with("B", |s| s.underline = true)])]);
    assert_that!(plain(&tb).1).is_equal_to(vec![LossKind::RunStyle(RunEffect::Underline)]);
}

/// 3 000 `é` font 6 000 octets ; `PLAIN_MAX` = 4 096 octets = 2 048 `é`.
#[test]
fn plain_cuts_to_the_byte_bound_on_a_character_boundary() {
    let (s, losses) = plain(&text_box(vec![para(vec![run(&"é".repeat(3_000))])]));
    assert_that!(s.len()).is_equal_to(PLAIN_MAX);
    assert_that!(s.chars().count()).is_equal_to(2048);
    assert_that!(losses).is_equal_to(vec![LossKind::TextTruncated]);
}
