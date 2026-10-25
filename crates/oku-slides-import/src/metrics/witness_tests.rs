use std::collections::BTreeMap;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::fonts::SUBSTITUTES;
use crate::source::Grouping;
use crate::source::{Chart, ChartKind, Fill, Geometry, Shape, Styled};
use crate::test_support::{bullet, item, para, run, run_with, text_box};

/// `normAutofit` nu : ni échelle ni réduction enregistrées.
const UNSAVED: Autofit = Autofit::Shrink { font_scale: 1.0, line_reduction: 0.0 };

/// Une boîte de texte aux paragraphes `paragraphs`, ajustée `autofit`, ancrée `anchor`.
fn boxed(autofit: Autofit, anchor: Anchor, paragraphs: Vec<Paragraph>) -> TextBox {
    TextBox { anchor, autofit, ..text_box(paragraphs) }
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
    ItemKind::Chart(Chart { kind, grouping: Grouping::default(), title: None, categories: vec![], series: vec![] })
}

fn tracked(spacing: f64) -> TextBox {
    text_box(vec![para(vec![run("x"), run_with("y", |s| s.char_spacing = spacing)])])
}

/// Un run `text` dont seuls les défauts de la présentation nomment la police (t-904).
fn defaulted(text: &str) -> TextBox {
    text_box(vec![para(vec![run("a"), run_with(text, |s| s.font_from_defaults = true)])])
}

/// Un run `text` dans la famille `family` (t-905).
fn in_family(family: &str, text: &str) -> TextBox {
    text_box(vec![para(vec![run_with(text, |s| s.font = Styled::explicit(family.to_owned()))])])
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
#[case::default_font(ItemKind::Text(defaulted("Gauche")), 40.0, false, Some(WitnessLimit::DefaultFont))]
#[case::default_font_in_a_shape(shape_with(defaulted("Droite")), 40.0, false, Some(WitnessLimit::DefaultFont))]
#[case::default_font_on_a_blank(ItemKind::Text(defaulted(" ")), 40.0, false, None)]
fn witness_limit_reads_the_source_item_under_office2pdf(
    #[case] kind: ItemKind,
    #[case] height: f64,
    #[case] in_smartart: bool,
    #[case] expected: Option<WitnessLimit>,
) {
    let it = item("s1/1", (0.0, 0.0, 300.0, height), kind);
    assert_that!(witness_limit(&it, in_smartart, Witness::Office2Pdf)).is_equal_to(expected);
}

// LibreOffice (t-905) : ni l'ajustement (il ne rétrécit qu'un texte qui déborde, ce que la source
// ne dit pas), ni l'interlettrage, ni la police des défauts, ni les graphiques ne sont ses erreurs.
#[rstest]
#[case::calibri_light(ItemKind::Text(in_family("Calibri Light", "Titre")), false, Some(WitnessLimit::NoSubstitute))]
#[case::calibri_light_any_case(
    ItemKind::Text(in_family("CALIBRI LIGHT", "Titre")),
    false,
    Some(WitnessLimit::NoSubstitute)
)]
#[case::calibri_light_in_a_shape(
    shape_with(in_family("Calibri Light", "Titre")),
    false,
    Some(WitnessLimit::NoSubstitute)
)]
#[case::calibri_light_on_a_blank(ItemKind::Text(in_family("Calibri Light", " ")), false, None)]
#[case::calibri(ItemKind::Text(in_family("Calibri", "Corps")), false, None)]
#[case::inter(ItemKind::Text(in_family("Inter", "Corps")), false, None)]
#[case::smartart_child_in_the_reader_style(
    ItemKind::Text(text_box(vec![para(vec![run(" "), run_with("Étape", |s| s.style_from_reader = true)])])),
    true,
    Some(WitnessLimit::SmartArtStyle)
)]
#[case::smartart_child_styled_by_the_file(line(Autofit::None), true, None)]
#[case::smartart_blank_child_in_the_reader_style(
    ItemKind::Text(text_box(vec![para(vec![run_with(" ", |s| s.style_from_reader = true)])])),
    true,
    None
)]
#[case::chart(chart(), false, None)]
#[case::unsaved_short_line(line(UNSAVED), false, None)]
#[case::tracked_text(ItemKind::Text(tracked(2.0)), false, None)]
#[case::default_font(ItemKind::Text(defaulted("Gauche")), false, None)]
fn witness_limit_reads_the_source_item_under_libreoffice(
    #[case] kind: ItemKind,
    #[case] in_smartart: bool,
    #[case] expected: Option<WitnessLimit>,
) {
    let it = item("s1/1", (0.0, 0.0, 300.0, 40.0), kind);
    assert_that!(witness_limit(&it, in_smartart, Witness::LibreOffice)).is_equal_to(expected);
}

#[test]
fn libreoffice_substitutes_every_family_oku_substitutes_but_calibri_light() {
    let unmapped: Vec<&str> = SUBSTITUTES
        .iter()
        .map(|(family, _)| *family)
        .filter(|family| {
            let it = item("s1/1", (0.0, 0.0, 300.0, 40.0), ItemKind::Text(in_family(family, "x")));
            witness_limit(&it, false, Witness::LibreOffice).is_some()
        })
        .collect();
    assert_that!(unmapped).is_equal_to(vec!["calibri light"]);
}
