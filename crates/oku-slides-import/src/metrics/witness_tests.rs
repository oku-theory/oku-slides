use std::collections::BTreeMap;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::source::{Chart, ChartKind, Fill, Geometry, Shape};
use crate::test_support::{bullet, item, para, run, run_with, text_box};

/// `normAutofit` nu : ni échelle ni réduction enregistrées.
const UNSAVED: Autofit = Autofit::Shrink { font_scale: 1.0, line_reduction: 0.0 };

/// Une boîte de texte aux paragraphes `paragraphs`, ajustée `autofit`, ancrée `anchor`.
fn boxed(autofit: Autofit, anchor: Anchor, paragraphs: Vec<Paragraph>) -> TextBox {
    TextBox { autofit, anchor, ..text_box(paragraphs) }
}

/// Une boîte d'un paragraphe d'un run de 18 pt, ajustée `autofit`, ancrée en haut.
fn line(autofit: Autofit) -> ItemKind {
    ItemKind::Text(boxed(autofit, Anchor::Top, vec![para(vec![run("x")])]))
}

/// Une forme rectangulaire portant `text`.
fn shape_with(text: TextBox) -> ItemKind {
    let geom = Geometry::Preset { name: "rect".into(), adj: BTreeMap::new() };
    ItemKind::Shape(Box::new(Shape { geom, fill: Fill::None, stroke: None, text: Some(text), connection: None }))
}

fn chart() -> ItemKind {
    let kind = ChartKind::Column;
    ItemKind::Chart(Chart { kind, grouping: Default::default(), title: None, categories: vec![], series: vec![] })
}

fn tracked(spacing: f64) -> TextBox {
    text_box(vec![para(vec![run("x"), run_with("y", |s| s.char_spacing = spacing)])])
}

fn two_paragraphs(anchor: Anchor) -> ItemKind {
    ItemKind::Text(boxed(UNSAVED, anchor, vec![para(vec![run("a")]), para(vec![run("b")])]))
}

/// Un paragraphe aux runs de corps `sizes`, en boîte `normAutofit` nue.
fn sized(sizes: &[f64]) -> ItemKind {
    let runs = sizes.iter().map(|&s| run_with("x", |r| r.size = s)).collect();
    ItemKind::Text(boxed(UNSAVED, Anchor::Top, vec![para(runs)]))
}

// Boîte de 40 pt (marges de 3,6 pt : 32,8 pt utiles) ; un run de 18 pt a une
// ligne de 21,6 pt, deux lignes de 43,2 pt : la boîte est courte.
#[rstest]
#[case::plain_text(line(Autofit::None), 40.0, false, None)]
#[case::resize(line(Autofit::Resize), 40.0, false, None)]
#[case::unsaved_short_line(line(UNSAVED), 40.0, false, Some(WitnessLimit::Autofit))]
#[case::unsaved_tall_box(line(UNSAVED), 60.0, false, None)]
#[case::saved_scale(line(Autofit::Shrink { font_scale: 0.9, line_reduction: 0.0 }), 40.0, false, None)]
#[case::saved_reduction(line(Autofit::Shrink { font_scale: 1.0, line_reduction: 0.1 }), 40.0, false, None)]
#[case::two_paragraphs_on_top(two_paragraphs(Anchor::Top), 40.0, false, None)]
#[case::two_paragraphs_in_the_middle(two_paragraphs(Anchor::Middle), 400.0, false, Some(WitnessLimit::Autofit))]
#[case::bullet_line(
    ItemKind::Text(boxed(UNSAVED, Anchor::Top, vec![bullet(0, vec![run("x")])])), 40.0, false, None
)]
#[case::line_break(
    ItemKind::Text(boxed(UNSAVED, Anchor::Top, vec![para(vec![run("a"), run("\n"), run("b")])])), 40.0, false, None
)]
#[case::mixed_sizes_short(sized(&[18.0, 24.0]), 60.0, false, Some(WitnessLimit::Autofit))]
#[case::mixed_sizes_tall(sized(&[18.0, 24.0]), 70.0, false, None)]
#[case::tracked_text(ItemKind::Text(tracked(2.0)), 40.0, false, Some(WitnessLimit::Tracking))]
#[case::condensed_text(ItemKind::Text(tracked(-1.0)), 40.0, false, Some(WitnessLimit::Tracking))]
#[case::tracked_shape(shape_with(tracked(1.5)), 40.0, false, Some(WitnessLimit::Tracking))]
#[case::untracked_shape(shape_with(tracked(0.0)), 40.0, false, None)]
#[case::chart(chart(), 40.0, false, Some(WitnessLimit::Chart))]
#[case::smartart_child(line(Autofit::None), 40.0, true, Some(WitnessLimit::SmartArt))]
fn witness_limit_reads_the_source_item(
    #[case] kind: ItemKind,
    #[case] height: f64,
    #[case] in_smartart: bool,
    #[case] expected: Option<WitnessLimit>,
) {
    let it = item("s1/1", (0.0, 0.0, 300.0, height), kind);
    assert_that!(witness_limit(&it, in_smartart)).is_equal_to(expected);
}
