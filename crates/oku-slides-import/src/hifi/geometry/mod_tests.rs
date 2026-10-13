use std::collections::BTreeMap;

use oku_slides_core::ShapePath;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn preset(name: &str, adj: &[(&str, f64)]) -> Geometry {
    Geometry::Preset {
        name: name.to_owned(),
        adj: adj.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect::<BTreeMap<_, _>>(),
    }
}

/// Points d'arrivée de chaque commande (sommets d'un polygone).
fn vertices(commands: &[PathCommand]) -> Vec<(f64, f64)> {
    commands
        .iter()
        .filter_map(|c| match *c {
            PathCommand::Move(x, y) | PathCommand::Line(x, y) | PathCommand::Quad(_, _, x, y) => Some((x, y)),
            PathCommand::Cubic(_, _, _, _, x, y) => Some((x, y)),
            PathCommand::Close => None,
        })
        .map(|(x, y)| ((x * 1e6).round() / 1e6, (y * 1e6).round() / 1e6))
        .collect()
}

fn moves(commands: &[PathCommand]) -> usize {
    commands.iter().filter(|c| matches!(c, PathCommand::Move(..))).count()
}

#[rstest]
#[case::triangle("triangle", 100.0, 100.0, vec![(0.0, 1.0), (0.5, 0.0), (1.0, 1.0)])]
#[case::right_arrow(
    "rightArrow",
    100.0,
    100.0,
    vec![(0.0, 0.25), (0.5, 0.25), (0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.5, 0.75), (0.0, 0.75)]
)]
#[case::chevron_wide(
    "chevron",
    200.0,
    100.0,
    vec![(0.0, 0.0), (0.75, 0.0), (1.0, 0.5), (0.75, 1.0), (0.0, 1.0), (0.25, 0.5)]
)]
#[case::pie_three_quarters("pie", 100.0, 100.0, vec![(1.0, 0.5), (0.5, 1.0), (0.0, 0.5), (0.5, 0.0), (0.5, 0.5)])]
fn outline_default_preset_has_known_vertices(
    #[case] name: &str,
    #[case] w: f64,
    #[case] h: f64,
    #[case] expected: Vec<(f64, f64)>,
) {
    let o = outline(&preset(name, &[]), w, h).unwrap();
    assert_that!(vertices(&o.filled)).is_equal_to(expected);
    assert_that!(o.stroked).is_equal_to(o.filled);
}

#[test]
fn outline_pie_arc_ends_on_the_visual_angle_of_a_wide_ellipse() {
    // 45° visuels sur une ellipse 200 × 100 : tan t = 2, x = 100 + 100/√5, y = 50 + 100/√5.
    let o = outline(&preset("pie", &[("adj2", 2_700_000.0)]), 200.0, 100.0).unwrap();
    let end = 100.0 / 5f64.sqrt();
    let PathCommand::Cubic(.., x, y) = o.filled[1] else { panic!("cubique attendue") };
    assert_that!(x).is_close_to((100.0 + end) / 200.0, 1e-9);
    assert_that!(y).is_close_to((50.0 + end) / 100.0, 1e-9);
}

#[test]
fn outline_round_rect_corners_are_quarter_arcs() {
    // Rayon par défaut : 16 667 / 100 000 du petit côté.
    let o = outline(&preset("roundRect", &[]), 100.0, 100.0).unwrap();
    let r = 0.16667;
    let v = vertices(&o.filled);
    assert_that!(o.filled.iter().filter(|c| matches!(c, PathCommand::Cubic(..))).count()).is_equal_to(4);
    for p in [(0.0, r), (r, 0.0), (1.0 - r, 0.0), (1.0, r), (1.0, 1.0 - r), (1.0 - r, 1.0), (r, 1.0)] {
        assert_that!(v.iter().any(|q| (q.0 - p.0).abs() < 1e-5 && (q.1 - p.1).abs() < 1e-5)).is_true();
    }
}

#[test]
fn outline_donut_has_two_subpaths() {
    let o = outline(&preset("donut", &[]), 100.0, 100.0).unwrap();
    assert_that!(moves(&o.filled)).is_equal_to(2);
}

#[test]
fn outline_star5_has_ten_vertices() {
    let o = outline(&preset("star5", &[]), 100.0, 100.0).unwrap();
    assert_that!(vertices(&o.filled).len()).is_equal_to(10);
}

#[test]
fn outline_callout_line_is_stroked_only() {
    let o = outline(&preset("borderCallout1", &[]), 100.0, 100.0).unwrap();
    assert_that!(moves(&o.filled)).is_equal_to(1);
    assert_that!(moves(&o.stroked)).is_equal_to(2);
}

#[test]
fn outline_adjustment_overrides_the_default() {
    let o = outline(&preset("triangle", &[("adj", 0.0)]), 100.0, 100.0).unwrap();
    assert_that!(vertices(&o.filled)).is_equal_to(vec![(0.0, 1.0), (0.0, 0.0), (1.0, 1.0)]);
}

#[test]
fn outline_text_rect_comes_from_the_preset() {
    let o = outline(&preset("triangle", &[]), 100.0, 100.0).unwrap();
    assert_that!(o.text_rect).is_equal_to([0.25, 0.5, 0.75, 1.0]);
}

#[test]
fn outline_flat_line_stays_on_its_edge() {
    let o = outline(&preset("line", &[]), 100.0, 0.0).unwrap();
    assert_that!(vertices(&o.stroked)).is_equal_to(vec![(0.0, 0.0), (1.0, 0.0)]);
}

#[rstest]
#[case::unknown_preset(preset("notAShape", &[]), 10.0, 10.0)]
#[case::negative_size(preset("rect", &[]), -1.0, 10.0)]
#[case::infinite_size(preset("rect", &[]), f64::INFINITY, 10.0)]
#[case::infinite_adjustment(preset("triangle", &[("adj", 1e308)]), 1e308, 1e308)]
fn outline_unusable_input_is_none(#[case] geometry: Geometry, #[case] w: f64, #[case] h: f64) {
    assert_that!(outline(&geometry, w, h)).is_none();
}

#[test]
fn outline_custom_path_keeps_fill_and_stroke_apart() {
    let geometry = Geometry::Path {
        paths: vec![
            SubPath {
                cmds: vec![
                    PathCmd::MoveTo { x: 0.0, y: 0.5 },
                    PathCmd::ArcTo { wr: 0.5, hr: 0.5, start: 180.0, swing: 180.0 },
                    PathCmd::Close,
                ],
                fill: true,
                stroke: false,
            },
            SubPath {
                cmds: vec![PathCmd::MoveTo { x: 0.0, y: 1.0 }, PathCmd::LineTo { x: 1.0, y: 1.0 }],
                fill: false,
                stroke: true,
            },
        ],
    };
    let o = outline(&geometry, 300.0, 100.0).unwrap();
    assert_that!(vertices(&o.filled)).is_equal_to(vec![(0.0, 0.5), (0.5, 0.0), (1.0, 0.5)]);
    assert_that!(vertices(&o.stroked)).is_equal_to(vec![(0.0, 1.0), (1.0, 1.0)]);
    assert_that!(o.text_rect).is_equal_to([0.0, 0.0, 1.0, 1.0]);
}

/// Tous les presets, aux ajustements par défaut, en carré, large et haut :
/// un contour fini, dans une marge raisonnable, accepté par le cœur.
#[rstest]
#[case::square(100.0, 100.0)]
#[case::wide(400.0, 100.0)]
#[case::tall(100.0, 400.0)]
fn outline_every_preset_evaluates(#[case] w: f64, #[case] h: f64) {
    let names: Vec<&str> = presets::names().collect();
    assert_that!(names.len()).is_equal_to(187);
    for name in names {
        let o = outline(&preset(name, &[]), w, h);
        let Some(o) = o else { panic!("{name} : aucun contour") };
        let all: Vec<PathCommand> = o.filled.iter().chain(&o.stroked).copied().collect();
        assert!(!all.is_empty(), "{name} : contour vide");
        for v in all.iter().flat_map(|c| numbers(*c)) {
            assert!((-1.0..=2.0).contains(&v), "{name} : {v} hors marge");
        }
        for part in [&o.filled, &o.stroked] {
            assert!(part.is_empty() || ShapePath::from_commands(part).is_some(), "{name} : refusé par le cœur");
        }
    }
}

/// Ajustements extrêmes sur tous les presets : jamais de panique, et un
/// contour rendu est fini.
#[rstest]
#[case::huge(1e12)]
#[case::negative(-1e12)]
#[case::zero(0.0)]
fn outline_extreme_adjustments_never_panic(#[case] value: f64) {
    for name in presets::names() {
        let adj: Vec<(&str, f64)> = ["adj", "adj1", "adj2", "adj3", "adj4", "adj5", "adj6", "adj7", "adj8"]
            .iter()
            .map(|k| (*k, value))
            .collect();
        if let Some(o) = outline(&preset(name, &adj), 100.0, 50.0) {
            assert!(o.filled.iter().chain(&o.stroked).flat_map(|c| numbers(*c)).all(f64::is_finite), "{name}");
        }
    }
}
