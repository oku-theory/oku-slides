use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::test_support::{PAPIER, PRO, tokens};

fn grid(columns: u32, rows: u32) -> Grid {
    Grid { columns, rows, gap: None }
}

fn cell(col: u32, row: u32, cols: u32, rows: u32) -> Cell {
    Cell { col, row, cols, rows }
}

fn frame(x: f64, y: f64, w: f64, h: f64) -> Frame {
    Frame { x, y, w, h }
}

/// Zone sans titre de Papier : 1920 − 2 × 120 = 1680 de large, 1080 − 150 − 140 = 790 de haut.
fn papier_zone() -> Zone {
    body_zone(&PageTokens::from_tokens(&tokens(PAPIER)), false)
}

#[test]
fn page_tokens_of_papier_are_the_default_tokens() {
    assert_that!(PageTokens::from_tokens(&tokens(PAPIER))).is_equal_to(PageTokens::default());
}

/// Pro : titre `size.title` = 56, filet de 5 pt plus l'espace `sm` (16), gouttière
/// `component.essentiel.title.gap` = 44 : bande de `56 + 21 + 44 = 121` ; `space.lg` = 32.
#[test]
fn page_tokens_of_pro_read_its_margins_title_rule_and_spacing() {
    let page = PageTokens::from_tokens(&tokens(PRO));
    let read = (page.width, page.height, page.margin_x, page.margin_top, page.margin_bottom, page.title_band, page.gap);
    assert_that!(read).is_equal_to((1920.0, 1080.0, 128.0, 112.0, 160.0, 121.0, 32.0));
}

/// Les manifestes livrés des packs intégrés se lisent, dans l'ordre `base`, `essentiel`.
#[test]
fn builtin_manifests_give_the_component_defaults_of_both_packs() {
    let packs = builtin_component_defaults().unwrap();
    let names: Vec<&str> = packs.iter().map(|(ns, _)| ns.as_str()).collect();
    assert_that!(names).is_equal_to(vec!["base", "essentiel"]);
    assert_that!(render_defaults().is_ok()).is_true();
    assert_that!(check_defaults().is_ok()).is_true();
}

/// Les jetons par défaut se lisent, et portent chaque chemin que lisent
/// `PageTokens` et `TextTokens` : aucun repli à 0 n'est jamais pris.
#[test]
fn default_tokens_hold_every_read_path() {
    assert_that!(defaults().is_empty()).is_false();
    let paths: [&[&str]; 14] = [
        &["page", "width"],
        &["page", "height"],
        &["page", "margin-x"],
        &["page", "margin-top"],
        &["page", "margin-bottom"],
        &["space", "lg"],
        &["space", "md"],
        &["space", "sm"],
        &["size", "title"],
        &["size", "body"],
        &["size", "small"],
        &["size", "caption"],
        &["leading", "normal"],
        &["leading", "tight"],
    ];
    for p in paths {
        assert_that!(number(defaults(), p)).named(&p.join(".")).is_some();
    }
    let tokens: [(&str, &[&str]); 11] = [
        ("space", &["list", "spacing"]),
        ("space", &["list", "indent"]),
        ("size", &["component", "essentiel", "title", "size"]),
        ("leading", &["component", "essentiel", "title", "leading"]),
        ("size", &["kicker", "size"]),
        ("space", &["component", "essentiel", "columns", "gap"]),
        ("space", &["component", "base", "card", "padding-x"]),
        ("space", &["component", "base", "card", "padding-y"]),
        ("size", &["component", "base", "card", "title-size"]),
        ("space", &["component", "base", "card", "gap"]),
        ("size", &["component", "base", "table", "size"]),
    ];
    for (family, p) in tokens {
        assert_that!(token_or_default(&Map::new(), family, p)).named(&p.join(".")).is_greater_than(0.0);
    }
}

#[test]
fn page_tokens_of_empty_tokens_fall_back_to_the_defaults() {
    assert_that!(PageTokens::from_tokens(&Map::new())).is_equal_to(PageTokens::default());
}

#[test]
fn page_tokens_read_the_essentiel_title() {
    let t = json!({ "component": { "essentiel": { "title": { "size": 77, "gap": 10 } } } });
    assert_that!(PageTokens::from_tokens(t.as_object().unwrap()).title_band).is_equal_to(87.0);
}

#[rstest]
#[case(false, Zone { x: 120.0, y: 150.0, w: 1680.0, h: 790.0 })]
#[case(true, Zone { x: 120.0, y: 250.0, w: 1680.0, h: 690.0 })]
fn body_zone_takes_the_margins_and_the_title_band(#[case] titled: bool, #[case] expected: Zone) {
    assert_that!(body_zone(&PageTokens::default(), titled)).is_equal_to(expected);
}

#[rstest]
#[case(None, 40.0)]
#[case(Some(TokenValue::Number(12.0)), 12.0)]
#[case(Some(TokenValue::Name("xl".into())), 40.0)]
fn grid_gap_reads_a_number_else_the_page_gap(#[case] gap: Option<TokenValue>, #[case] expected: f64) {
    let g = Grid { columns: 12, rows: 6, gap };
    assert_that!(grid_gap(&g, &PageTokens::default())).is_equal_to(expected);
}

/// 12 colonnes dans 1680 avec 11 gouttières de 40 : `(1680 − 440) / 12 = 103.33…` ;
/// 6 lignes dans 790 avec 5 gouttières : `(790 − 200) / 6 = 98.33…`.
#[test]
fn cell_rect_follows_the_grid_frame_formula() {
    let r = cell_rect(&papier_zone(), &grid(12, 6), 40.0, &cell(8, 0, 4, 6));
    assert_that!(r.x).is_close_to(8.0 * (1240.0 / 12.0 + 40.0), 1e-9);
    assert_that!(r.y).is_equal_to(0.0);
    assert_that!(r.w).is_close_to(4.0 * 1240.0 / 12.0 + 3.0 * 40.0, 1e-9);
    assert_that!(r.h).is_close_to(790.0, 1e-9);
}

#[test]
fn cell_rect_of_the_whole_grid_is_the_zone() {
    let r = cell_rect(&papier_zone(), &grid(3, 2), 40.0, &cell(0, 0, 3, 2));
    assert_that!(r).is_equal_to(frame(0.0, 0.0, 1680.0, 790.0));
}

#[rstest]
#[case::whole_zone(frame(0.0, 0.0, 1680.0, 790.0), cell(0, 0, 12, 6))]
#[case::left_two_thirds(frame(0.0, 0.0, 1100.0, 790.0), cell(0, 0, 8, 6))]
#[case::right_third(frame(1150.0, 0.0, 530.0, 790.0), cell(8, 0, 4, 6))]
#[case::top_left_cell(frame(5.0, 3.0, 90.0, 90.0), cell(0, 0, 1, 1))]
#[case::tiny_box_keeps_one_cell(frame(700.0, 400.0, 2.0, 2.0), cell(5, 3, 1, 1))]
#[case::outside_is_clamped_to_the_edges(frame(-500.0, -500.0, 5000.0, 5000.0), cell(0, 0, 12, 6))]
fn snap_takes_the_nearest_track_edges(#[case] r: Frame, #[case] expected: Cell) {
    assert_that!(snap(&papier_zone(), &grid(12, 6), 40.0, r)).is_equal_to(expected);
}

#[test]
fn snap_of_a_cell_rect_gives_back_the_cell() {
    let (zone, g) = (papier_zone(), grid(12, 6));
    for c in [cell(0, 0, 1, 1), cell(3, 2, 4, 3), cell(11, 5, 1, 1), cell(8, 0, 4, 6)] {
        assert_that!(snap(&zone, &g, 40.0, cell_rect(&zone, &g, 40.0, &c))).is_equal_to(c);
    }
}

/// Bords de colonne d'une grille 2 × 1 dans 1680 : départs 0 et 860, fins 820 et 1680.
/// Un départ à 430 est à égale distance de 0 et 860 : le premier gagne.
#[test]
fn snap_breaks_ties_towards_the_first_track() {
    let c = snap(&papier_zone(), &grid(2, 1), 40.0, frame(430.0, 0.0, 400.0, 790.0));
    assert_that!(c).is_equal_to(cell(0, 0, 1, 1));
}

#[test]
fn snap_always_fits_the_grid() {
    let (zone, g) = (papier_zone(), grid(3, 3));
    for r in [frame(1600.0, 700.0, 10.0, 10.0), frame(0.0, 0.0, 1.0, 1.0), frame(900.0, 100.0, 9000.0, 1.0)] {
        assert_that!(snap(&zone, &g, 40.0, r).fits(&g)).is_true();
    }
}

/// Échelle de page 2, sans ajustement : la page source (0, 0) tombe sur la page
/// Oku (0, 0), soit (−120, −150) relativement à la zone (120, 150).
fn page_fit() -> Fit {
    Fit { scale: 2.0, dx: -120.0, dy: -150.0 }
}

/// La boîte (100, 100, 200, 100) devient (200, 200, 400, 200) sur la page, soit
/// (80, 50, 400, 200) dans la zone.
#[test]
fn to_zone_scales_then_makes_the_frame_zone_relative() {
    let f = to_zone(&source::Frame::new(100.0, 100.0, 200.0, 100.0), &page_fit(), &papier_zone());
    assert_that!(f).is_equal_to(frame(80.0, 50.0, 400.0, 200.0));
}

#[test]
fn to_zone_clamps_a_frame_overflowing_the_zone() {
    let f = to_zone(&source::Frame::new(0.0, 0.0, 960.0, 540.0), &page_fit(), &papier_zone());
    assert_that!(f).is_equal_to(frame(0.0, 0.0, 1680.0, 790.0));
}

#[test]
fn to_zone_keeps_a_minimal_frame_for_a_box_outside_the_zone() {
    let f = to_zone(&source::Frame::new(0.0, 0.0, 10.0, 10.0), &page_fit(), &papier_zone());
    assert_that!(f).is_equal_to(frame(0.0, 0.0, 1.0, 1.0));
    let far = to_zone(&source::Frame::new(2000.0, 2000.0, 10.0, 10.0), &page_fit(), &papier_zone());
    assert_that!(far).is_equal_to(frame(1679.0, 789.0, 1.0, 1.0));
}

#[test]
fn fit_of_no_frame_lays_the_source_page_on_the_page() {
    assert_that!(Fit::new([], 2.0, &papier_zone())).is_equal_to(page_fit());
}

/// Boîte englobante (64, 176)–(896, 460), 832 × 284 pt source ; à l'échelle 2,
/// 1 664 × 568 tient dans 1 680 × 790 : échelle 2, centrée, soit
/// `dx = (1680 − 1664) / 2 − 128 = −120` et `dy = (790 − 568) / 2 − 352 = −241`.
#[test]
fn fit_of_frames_that_fit_keeps_the_page_scale_and_centres_them() {
    let frames = [source::Frame::new(64.0, 176.0, 277.0, 284.0), source::Frame::new(619.0, 176.0, 277.0, 100.0)];
    assert_that!(Fit::new(&frames, 2.0, &papier_zone())).is_equal_to(Fit { scale: 2.0, dx: -120.0, dy: -241.0 });
}

/// Boîte (0, 0)–(960, 540) : 1 920 × 1 080 à l'échelle 2 ne tient pas ; la
/// largeur demande 1680 / 960 = 1,75, la hauteur 790 / 540 ≈ 1,463 : la plus
/// petite, centrée en largeur (`(1680 − 1404,4) / 2`).
#[test]
fn fit_of_frames_too_large_shrinks_them_uniformly_into_the_zone() {
    let zone = papier_zone();
    let fit = Fit::new(&[source::Frame::new(0.0, 0.0, 960.0, 540.0)], 2.0, &zone);
    assert_that!(fit.scale).is_close_to(790.0 / 540.0, 1e-9);
    assert_that!(fit.dx).is_close_to((1680.0 - 960.0 * 790.0 / 540.0) / 2.0, 1e-9);
    assert_that!(fit.dy).is_close_to(0.0, 1e-9);
    let whole = to_zone(&source::Frame::new(0.0, 0.0, 960.0, 540.0), &fit, &zone);
    assert_that!(whole.h).is_close_to(790.0, 1e-9);
}

/// Grille 12 × 6 de Papier sans titre : colonne `(1680 − 11 × 40) / 12 ≈ 103,3`,
/// ligne `(790 − 5 × 40) / 6 = 98,3`.
#[test]
fn track_size_divides_the_zone_between_the_gutters() {
    let (w, h) = track_size(&papier_zone(), &grid(12, 6), 40.0);
    assert_that!(w).is_close_to(1240.0 / 12.0, 1e-9);
    assert_that!(h).is_close_to(590.0 / 6.0, 1e-9);
}

const BROKEN_MANIFEST: &str = r#"{ "id": 3 "#;

#[rstest]
#[case::broken_tokens("[1, 2]", &[], Err(DefaultsFault))]
#[case::broken_manifest("{}", &[(PackKind::Widgets, BROKEN_MANIFEST)], Err(DefaultsFault))]
#[case::no_manifest_is_fine("{\"a\": 1}", &[], Ok(json!({ "a": 1 })))]
fn parse_defaults_of_a_broken_shipped_file_is_an_error(
    #[case] tokens: &str,
    #[case] manifests: &[(PackKind, &'static str)],
    #[case] expected: Result<Value, DefaultsFault>,
) {
    let parsed = parse_defaults(tokens, manifests.iter().copied()).map(Value::Object);
    assert_that!(parsed).is_equal_to(expected);
}
