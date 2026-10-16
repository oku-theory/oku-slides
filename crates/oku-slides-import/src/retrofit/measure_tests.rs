use std::collections::BTreeMap;

use oku_slides_core::{Element, Paragraph, ParamValue, Run, TokenValue};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::test_support::{PAPIER, PRO, tokens};

fn papier() -> TextTokens {
    TextTokens::from_tokens(&tokens(PAPIER))
}

fn text(id: &str, paragraphs: Vec<Paragraph>) -> Element {
    Element::text(id, paragraphs)
}

fn card(body: &str, title: Option<&str>, kicker: Option<&str>) -> Element {
    let mut e = Element::new("c", "card");
    if let Some(t) = title {
        e.params.set("title", ParamValue::Text(t.to_owned()));
    }
    if let Some(k) = kicker {
        e.params.set("kicker", ParamValue::Text(k.to_owned()));
    }
    e.params.set("body", ParamValue::Rich(vec![Paragraph::plain(body)]));
    e
}

fn stack(layout: &str, children: Vec<Element>) -> Element {
    let mut e = Element::new("s", "stack");
    e.params.set("layout", ParamValue::Text(layout.to_owned()));
    e.params.set("children", ParamValue::Slot(children));
    e
}

fn measure<'a>(t: &'a TextTokens, aspects: &'a BTreeMap<String, f64>) -> Measure<'a> {
    Measure { tokens: t, aspects }
}

/// Le corps de Papier : 32 pt, interligne 0,6 em, paragraphes à `md` (24),
/// items de liste à `sm` (16) et retrait `md` ; cartes à marges `lg` (40).
#[test]
fn text_tokens_of_papier_are_the_default_body() {
    let t = papier();
    assert_that!((t.body, t.leading, t.paragraph, t.list_spacing, t.list_indent))
        .is_equal_to((32.0, 0.6, 24.0, 16.0, 24.0));
    assert_that!((t.card.padding_x, t.card.padding_y, t.card.title, t.card.gap)).is_equal_to((40.0, 40.0, 40.0, 16.0));
    assert_that!(t.smaller).is_equal_to([("small", 26.0), ("caption", 22.0)]);
}

/// Pro : corps 21 pt, interligne 0,62 em, cartes à marges 32 × 36.
#[test]
fn text_tokens_of_pro_read_its_own_body_and_card() {
    let t = TextTokens::from_tokens(&tokens(PRO));
    assert_that!((t.body, t.leading, t.title, t.title_leading)).is_equal_to((21.0, 0.62, 56.0, 0.12));
    assert_that!((t.card.padding_x, t.card.padding_y, t.columns_gap)).is_equal_to((32.0, 36.0, 24.0));
}

#[test]
fn text_tokens_of_empty_tokens_are_the_default_tokens() {
    assert_that!(TextTokens::from_tokens(&serde_json::Map::new())).is_equal_to(papier());
}

#[rstest]
#[case("", 100.0, 1)]
#[case("un deux trois", 1000.0, 1)]
// 100 pt à 32 × 0,48 = 15,36 pt par caractère : 6 caractères par ligne.
#[case("un deux trois", 100.0, 3)]
#[case("abcdefghijklm", 100.0, 3)]
#[case("un\ndeux", 1000.0, 2)]
fn lines_wrap_words_at_the_box_width(#[case] s: &str, #[case] width: f64, #[case] expected: usize) {
    assert_that!(lines(s, width, 32.0)).is_equal_to(expected);
}

/// Deux lignes : 2 × 0,72 × 32 + 0,6 × 32 = 65,28 pt (65 au rendu de Papier,
/// le premier bloc de la slide 6 d'alto).
#[test]
fn paragraphs_of_two_lines_take_two_cap_heights_and_one_leading() {
    let t = papier();
    let aspects = BTreeMap::new();
    let p = Paragraph::plain("x".repeat(150));
    let h = measure(&t, &aspects).paragraphs(&[p], 1680.0, 32.0);
    assert_that!(h).is_close_to(65.28, 1e-9);
}

#[test]
fn paragraphs_list_items_are_spaced_by_the_list_spacing() {
    let t = papier();
    let aspects = BTreeMap::new();
    let item = || Paragraph { runs: vec![Run::plain("a")], list: Some(oku_slides_core::ListKind::Bullet) };
    let h = measure(&t, &aspects).paragraphs(&[item(), item(), Paragraph::plain("b")], 1680.0, 32.0);
    assert_that!(h).is_close_to(3.0 * 0.72 * 32.0 + 16.0 + 24.0, 1e-9);
}

/// Un run réduit (`small`, 26 pt) garde l'interligne du corps (0,6 × 32).
#[test]
fn paragraphs_of_reduced_runs_keep_the_leading_of_the_body() {
    let t = papier();
    let aspects = BTreeMap::new();
    let run = Run { size: Some(TokenValue::Name("small".into())), ..Run::plain("x".repeat(150)) };
    let h = measure(&t, &aspects).paragraphs(&[Paragraph { runs: vec![run], list: None }], 1680.0, 32.0);
    assert_that!(h).is_close_to(2.0 * 0.72 * 26.0 + 0.6 * 32.0, 1e-9);
}

/// Carte de 533 pt : marges 2 × 40, surtitre (0,72 × 24), titre d'une ligne
/// (0,72 × 40), corps d'une ligne (0,72 × 32), deux écarts de 16.
#[test]
fn card_adds_paddings_kicker_title_body_and_gaps() {
    let t = papier();
    let aspects = BTreeMap::new();
    let h = measure(&t, &aspects).element(&card("Corps", Some("Titre"), Some("SURTITRE")), 533.0, 690.0, false);
    assert_that!(h).is_close_to(80.0 + 0.72 * 24.0 + 0.72 * 40.0 + 0.72 * 32.0 + 32.0, 1e-9);
}

#[test]
fn stack_in_columns_takes_its_tallest_child_at_its_column_width() {
    let t = papier();
    let aspects = BTreeMap::new();
    let m = measure(&t, &aspects);
    let tall = card(&"mot ".repeat(60), None, None);
    let short = card("Court", None, None);
    // Trois colonnes de (1680 − 2 × 40) / 3 = 533,33 pt.
    let cw = (1680.0 - 80.0) / 3.0;
    let expected = m.element(&tall, cw, 690.0, false);
    let h = m.element(&stack("columns", vec![short.clone(), tall, short]), 1680.0, 690.0, false);
    assert_that!(h).is_close_to(expected, 1e-9);
}

#[test]
fn stack_in_rows_adds_its_children_and_gaps() {
    let t = papier();
    let aspects = BTreeMap::new();
    let m = measure(&t, &aspects);
    let one = card("Court", None, None);
    let h = m.element(&stack("rows", vec![one.clone(), one.clone()]), 800.0, 690.0, false);
    assert_that!(h).is_close_to(2.0 * m.element(&one, 800.0, 690.0, false) + 40.0, 1e-9);
}

#[test]
fn image_takes_its_source_aspect_capped_by_the_room() {
    let t = papier();
    let aspects = BTreeMap::from([("a".to_owned(), 0.5)]);
    let m = measure(&t, &aspects);
    let mut img = Element::new("i", "image");
    img.params.set("asset", ParamValue::Text("a".into()));
    assert_that!(m.element(&img, 800.0, 690.0, false)).is_equal_to(400.0);
    assert_that!(m.element(&img, 800.0, 300.0, false)).is_equal_to(300.0);
    assert_that!(m.element(&img, 800.0, 690.0, true)).is_equal_to(0.0);
}

#[test]
fn chart_takes_the_render_aspect() {
    let t = papier();
    let aspects = BTreeMap::new();
    let mut chart = Element::new("g", "chart");
    chart.params.set("option", ParamValue::Value(json!({})));
    assert_that!(measure(&t, &aspects).element(&chart, 800.0, 690.0, false)).is_equal_to(450.0);
}

#[test]
fn table_counts_one_line_per_row() {
    let t = papier();
    let aspects = BTreeMap::new();
    let mut table = Element::new("t", "table");
    table.params.set("data", ParamValue::Value(json!({ "dataset": { "source": [["a"], ["b"], ["c"]] } })));
    let h = measure(&t, &aspects).element(&table, 800.0, 690.0, false);
    assert_that!(h).is_close_to(3.0 * (0.72 * 26.0 + 2.0 * 8.0), 1e-9);
}

/// Mise en page `default` : les blocs et leurs écarts `md` ; un graphique en
/// bloc compte au plus [`MIN_MEDIA`], il prend ce qui reste.
#[test]
fn flow_default_adds_blocks_and_counts_a_lone_media_at_its_minimum() {
    let t = papier();
    let aspects = BTreeMap::new();
    let m = measure(&t, &aspects);
    let body = text("t", vec![Paragraph::plain("Corps")]);
    let mut chart = Element::new("g", "chart");
    chart.params.set("option", ParamValue::Value(json!({})));
    let h = m.flow(&[body.clone(), chart], "default", 1680.0, 690.0);
    assert_that!(h).is_close_to(m.element(&body, 1680.0, 690.0, false) + 24.0 + MIN_MEDIA, 1e-9);
}

#[test]
fn flow_columns_takes_its_tallest_block() {
    let t = papier();
    let aspects = BTreeMap::new();
    let m = measure(&t, &aspects);
    let short = text("a", vec![Paragraph::plain("Court")]);
    let long = text("b", vec![Paragraph::plain("mot ".repeat(80))]);
    // Deux colonnes de (1680 − 64) / 2 = 808 pt (`columns.gap` = `xl`).
    let expected = m.element(&long, 808.0, 690.0, false);
    assert_that!(m.flow(&[short, long], "columns", 1680.0, 690.0)).is_close_to(expected, 1e-9);
}

/// Titre d'une ligne de Papier : 0,72 × 60 + 40 ; avec surtitre, + 0,72 × 24 + 16.
#[rstest]
#[case(None, 0.72 * 60.0 + 40.0)]
#[case(Some("SURTITRE"), 0.72 * 60.0 + 40.0 + 0.72 * 24.0 + 16.0)]
fn title_band_counts_the_title_lines_and_the_kicker(#[case] kicker: Option<&str>, #[case] expected: f64) {
    let band = title_band(&papier(), Some("Un titre"), kicker, 1680.0, 40.0);
    assert_that!(band).is_close_to(expected, 1e-9);
}

#[test]
fn title_band_without_title_is_empty() {
    assert_that!(title_band(&papier(), None, Some("SURTITRE"), 1680.0, 40.0)).is_equal_to(0.0);
}

#[test]
fn resize_sets_the_size_of_every_body_run_down_the_stacks() {
    let mut list = vec![stack("columns", vec![card("A", Some("Titre"), None)]), text("t", vec![Paragraph::plain("B")])];
    resize(&mut list, "small");
    let small = Some(TokenValue::Name("small".into()));
    let child = &list[0].params.slot("children").unwrap()[0];
    assert_that!(child.params.rich("body").unwrap()[0].runs[0].size.clone()).is_equal_to(small.clone());
    assert_that!(list[1].params.rich("body").unwrap()[0].runs[0].size.clone()).is_equal_to(small);
    assert_that!(child.params.text("title")).is_equal_to(Some("Titre"));
}
