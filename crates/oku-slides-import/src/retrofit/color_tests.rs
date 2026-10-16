use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::source::{ColorMod, Fill, Geometry, ItemKind, Shape as SourceShape, SourceDeck, Styled};
use crate::test_support::{PAPIER, PRO, colors, item, para, run_with, source_deck, source_slide, text_box};

fn hex(s: &str) -> Rgba {
    parse_hex(s).unwrap()
}

#[rstest]
#[case("#abc", Some(Rgba::new(0xaa, 0xbb, 0xcc)))]
#[case("#2e2b26", Some(Rgba::new(0x2e, 0x2b, 0x26)))]
#[case("#2e2b2680", Some(Rgba { r: 0x2e, g: 0x2b, b: 0x26, a: 0x80 }))]
#[case("#2e2b2", None)]
#[case("#ggg", None)]
#[case("2e2b26", None)]
#[case("#+1+", None)]
fn parse_hex_reads_three_six_and_eight_digits_only(#[case] s: &str, #[case] expected: Option<Rgba>) {
    assert_that!(parse_hex(s)).is_equal_to(expected);
}

#[test]
fn color_tokens_from_tokens_resolves_aliases_and_skips_the_unresolvable() {
    let theme = json!({ "color": {
        "ink": "#000", "text": "ink", "body": "text",
        "loop-a": "loop-b", "loop-b": "loop-a",
        "lost": "nowhere", "number": 3, "bad": "#12"
    }});
    let tokens = ColorTokens::from_tokens(theme.as_object().unwrap());
    assert_that!(tokens.get("body")).is_equal_to(Some(Rgba::new(0, 0, 0)));
    assert_that!(tokens.get("text")).is_equal_to(Some(Rgba::new(0, 0, 0)));
    for skipped in ["loop-a", "loop-b", "lost", "number", "bad"] {
        assert_that!(tokens.contains(skipped)).named(skipped).is_false();
    }
}

#[test]
fn color_tokens_of_papier_carry_the_theme_over_the_defaults() {
    let papier = colors(PAPIER);
    assert_that!(papier.get("ink")).is_equal_to(Some(Rgba::new(0x2e, 0x2b, 0x26)));
    assert_that!(papier.contains("accent-soft")).is_true();
}

/// Pro : `accent` et `accent-on-dark` valent tous deux `#00bef4` ; `rule` et
/// `surface` tous deux `#e1e5ea`. À égalité, le premier par nom l'emporte.
#[rstest]
#[case("#00bef4", "accent")]
#[case("#e7e6e6", "rule")]
fn color_tokens_nearest_on_a_tie_takes_the_first_name(#[case] c: &str, #[case] expected: &str) {
    let pro = colors(PRO);
    assert_that!(pro.nearest(hex(c)).map(|(n, _)| n)).is_equal_to(Some(expected));
}

#[test]
fn color_tokens_nearest_without_tokens_is_none() {
    assert_that!(ColorTokens::default().nearest(Rgba::new(1, 2, 3))).is_none();
}

fn scheme(slot: &str, value: &str) -> Color {
    Styled::mapped(hex(value), slot, slot, Vec::new())
}

/// Écarts (×100) au jeton le plus proche, calculés en OKLab :
/// Papier — `#40695f`→`accent` 0,7 ; `#333333`→`ink` 3,2 ; `#ff0000`→`ink-muted` 27,9 ;
/// `#ed7d31`→`rule` 23,1 ; `#2f5597`→`accent` 13,1.
/// Pro — `#4472c4`→`accent-text` 5,1 ; `#00b0f0`→`accent` 3,9 ; `#2f5597`→`accent-text` 8,4.
#[rstest]
#[case::default_value(PAPIER, Styled::default_value(hex("#123456")), Ok(None))]
#[case::dk1(PAPIER, scheme("dk1", "#000000"), Ok(Some("ink")))]
#[case::lt1(PAPIER, scheme("lt1", "#ffffff"), Ok(Some("bg")))]
#[case::dk2(PAPIER, scheme("dk2", "#44546a"), Ok(Some("ink-muted")))]
#[case::lt2(PAPIER, scheme("lt2", "#e7e6e6"), Ok(Some("surface")))]
#[case::accent1(PAPIER, scheme("accent1", "#4472c4"), Ok(Some("accent")))]
#[case::hlink(PAPIER, scheme("hlink", "#0563c1"), Ok(Some("accent")))]
#[case::accent2_far(PAPIER, scheme("accent2", "#ed7d31"), Err("#ed7d31"))]
#[case::modified_accent1_far(
    PAPIER,
    Styled::mapped(hex("#2f5597"), "accent1", "accent1", vec![ColorMod::LumMod(0.75)]),
    Err("#2f5597")
)]
#[case::modified_accent1_near(
    PRO,
    Styled::mapped(hex("#2f5597"), "accent1", "accent1", vec![ColorMod::LumMod(0.75)]),
    Ok(Some("accent-text"))
)]
#[case::pure_black(PAPIER, Styled::explicit(hex("#000000")), Ok(Some("ink")))]
#[case::pure_white(PRO, Styled::explicit(hex("#ffffff")), Ok(Some("bg")))]
#[case::dark_grey(PAPIER, Styled::explicit(hex("#333333")), Ok(Some("ink")))]
#[case::near_accent(PAPIER, Styled::explicit(hex("#40695f")), Ok(Some("accent")))]
#[case::red(PAPIER, Styled::explicit(hex("#ff0000")), Err("#ff0000"))]
#[case::office_blue(PRO, Styled::explicit(hex("#4472c4")), Ok(Some("accent-text")))]
#[case::cyan(PRO, Styled::explicit(hex("#00b0f0")), Ok(Some("accent")))]
fn project_color_maps_to_a_theme_token_or_drops(
    #[case] theme: &str,
    #[case] c: Color,
    #[case] expected: Result<Option<&str>, &str>,
) {
    let expected = expected.map(|o| o.map(str::to_owned)).map_err(|h| LossKind::ColorDropped(h.to_owned()));
    assert_that!(project_color(&c, &colors(theme))).is_equal_to(expected);
}

#[test]
fn project_color_of_a_slot_whose_token_is_missing_takes_the_nearest() {
    let theme = json!({ "color": { "bg": "#ffffff", "paper": "#e8e8e8" } });
    let tokens = ColorTokens::from_tokens(theme.as_object().unwrap());
    assert_that!(project_color(&scheme("lt2", "#e7e6e6"), &tokens)).is_equal_to(Ok(Some("paper".to_owned())));
}

#[test]
fn project_color_ignores_opacity() {
    let c = Styled::explicit(Rgba { r: 0x40, g: 0x69, b: 0x5f, a: 0x40 });
    assert_that!(project_color(&c, &colors(PAPIER))).is_equal_to(Ok(Some("accent".to_owned())));
}

#[test]
fn project_color_dropped_with_opacity_names_the_eight_digit_hex() {
    let c = Styled::explicit(Rgba { r: 0xff, g: 0, b: 0, a: 0x80 });
    assert_that!(project_color(&c, &colors(PAPIER))).is_equal_to(Err(LossKind::ColorDropped("#ff000080".to_owned())));
}

#[rstest]
#[case("#000000", "#ffffff", 21.0)]
#[case("#ffffff", "#ffffff", 1.0)]
#[case("#00bef4", "#ffffff", 2.17)]
#[case("#ffffff", "#00154e", 17.25)]
#[case("#b9c4d6", "#fffdf9", 1.73)]
fn contrast_follows_the_wcag_formula_in_both_orders(#[case] a: &str, #[case] b: &str, #[case] expected: f64) {
    assert_that!(contrast(hex(a), hex(b))).is_close_to(expected, 0.01);
    assert_that!(contrast(hex(b), hex(a))).is_close_to(expected, 0.01);
}

#[test]
fn color_tokens_card_backgrounds_follow_the_card_component() {
    let pro = colors(PRO);
    assert_that!((pro.card(false), pro.card(true))).is_equal_to((hex("#ffffff"), hex("#00154e")));
    // Papier garde les défauts du rendu : `bg`, et `ink` pour le relief.
    let papier = colors(PAPIER);
    assert_that!((papier.card(false), papier.card(true))).is_equal_to((hex("#fffdf9"), hex("#2e2b26")));
}

#[test]
fn color_tokens_on_dark_prefers_on_dark_body_then_white_then_on_accent() {
    assert_that!(colors(PRO).on_dark(hex("#00154e"))).is_equal_to(Some("on-dark-body"));
    assert_that!(colors(PAPIER).on_dark(hex("#2e2b26"))).is_equal_to(Some("on-accent"));
    assert_that!(colors(PAPIER).on_dark(hex("#fffdf9"))).is_none();
}

/// Un run de 10 caractères en 10 pt pèse 1 000 pt² ; un fond de 100 × 50, 5 000 pt².
fn deck_with(colors: &[(&str, f64)]) -> SourceDeck {
    let items = colors
        .iter()
        .enumerate()
        .map(|(k, (c, area))| {
            let shape = SourceShape {
                geom: Geometry::Preset { name: "rect".into(), adj: Default::default() },
                fill: Fill::Solid { color: Styled::explicit(hex(c)) },
                stroke: None,
                text: None,
                connection: None,
            };
            item(&format!("s1/{k}"), (0.0, 0.0, *area, 1.0), ItemKind::Shape(Box::new(shape)))
        })
        .chain([item(
            "s1/t",
            (0.0, 0.0, 10.0, 10.0),
            ItemKind::Text(text_box(vec![para(vec![run_with("0123456789", |s| {
                s.size = 10.0;
                s.color = Styled::explicit(hex("#ff0000"));
            })])])),
        )])
        .collect();
    source_deck(vec![source_slide(1, None, items)])
}

/// Papier : `#00154e` (bleu nuit, ΔE 13,4 de `ink`) et `#00bef4` (cyan, 20,7 de
/// `rule`) n'ont pas de jeton proche ; le rouge (1 000 pt²) non plus.
/// `#3f6b5e` est l'accent de Papier : il ne compte pas. `#808080` est neutre.
#[test]
fn color_tokens_with_accents_ranks_saturated_colors_without_a_close_token_by_usage() {
    let deck = deck_with(&[("#00bef4", 300.0), ("#00154e", 5000.0), ("#3f6b5e", 9000.0), ("#808080", 9000.0)]);
    let tokens = colors(PAPIER).with_accents(&deck);
    let projected: Vec<_> =
        ["#00154e", "#ff0000", "#00bef4"].iter().map(|c| project_color(&Styled::explicit(hex(c)), &tokens)).collect();
    assert_that!(projected).is_equal_to(vec![
        Ok(Some("accent".to_owned())),
        Ok(Some("accent-soft".to_owned())),
        Err(LossKind::ColorDropped("#00bef4".to_owned())),
    ]);
}

#[test]
fn color_tokens_with_accents_leaves_a_theme_with_close_tokens_alone() {
    let deck = deck_with(&[("#00bef4", 5000.0), ("#00154e", 300.0)]);
    let tokens = colors(PRO).with_accents(&deck);
    assert_that!(project_color(&Styled::explicit(hex("#00154e")), &tokens))
        .is_equal_to(Ok(Some("brand-ink".to_owned())));
}

/// Pro : un titre blanc sur le bandeau `#00154e` reste lisible (`bg`, 17,3:1) ;
/// `accent` (`#00bef4`) sur blanc ne fait que 2,2:1 : rien. Papier : le gris
/// bleuté `#b9c4d6` se projette sur `rule` (`#ddd7cc`), 1,3:1 sur `bg` : rien.
#[rstest]
#[case::white_on_navy(PRO, "#ffffff", "#00154e", Some("bg"))]
#[case::cyan_on_white(PRO, "#00bef4", "#ffffff", None)]
#[case::cyan_on_navy(PRO, "#00bef4", "#00154e", Some("accent"))]
#[case::pale_on_papier(PAPIER, "#b9c4d6", "#fffdf9", None)]
#[case::pale_on_navy(PRO, "#8fa3c2", "#00154e", Some("on-dark-muted"))]
#[case::pale_on_dark_papier(PAPIER, "#b9c4d6", "#2e2b26", Some("rule"))]
fn text_color_keeps_only_readable_tokens(
    #[case] theme: &str,
    #[case] text: &str,
    #[case] bg: &str,
    #[case] expected: Option<&str>,
) {
    let got = text_color(&Styled::explicit(hex(text)), hex(bg), &colors(theme));
    assert_that!(got).is_equal_to(Ok(expected.map(str::to_owned)));
}

/// Papier n'a ni `on-dark-body` ni `white` : `on-accent` (`#fffdf9`). Le vert
/// `#3f6b5e` (`accent`) sur `ink` (`#2e2b26`) ne fait que 2,3:1 ; plus clair
/// que ce fond sombre, il prend `on-accent` (15,0:1).
#[test]
fn text_color_of_a_light_text_on_a_dark_ground_takes_an_on_dark_token() {
    let got = text_color(&Styled::explicit(hex("#3f6b5e")), hex("#2e2b26"), &colors(PAPIER));
    assert_that!(got).is_equal_to(Ok(Some("on-accent".to_owned())));
}

/// Le surtitre et les puces d'une carte en relief (`highlight-kicker`) se
/// lisent sur son fond (`highlight-bg`) : la règle de contraste du retrofit,
/// qui y pose les cartes en relief des slides importées (t-817). Chaque thème
/// intégré, et le thème par défaut (les seuls jetons par défaut, t-894).
#[test]
fn builtin_theme_highlight_kicker_reads_on_the_highlight_card() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../typst/oku-slides/themes");
    let mut themes: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| (p.display().to_string(), std::fs::read_to_string(&p).unwrap()))
        .collect();
    assert_that!(themes.len()).is_greater_than_or_equal_to(2);
    themes.push(("default-tokens.json".to_owned(), r#"{"tokens": {}}"#.to_owned()));
    for (name, theme) in &themes {
        let tokens = crate::test_support::tokens(theme);
        let kicker = tokens["component"]["card"]["highlight-kicker"].as_str().unwrap();
        let c = colors(theme);
        let ratio = contrast(c.get(kicker).unwrap(), c.card(true));
        asserting(name).that(&ratio).is_greater_than_or_equal_to(MIN_CONTRAST);
    }
}
