use speculoos::prelude::*;

use super::*;
use crate::source::Crop;
use crate::source::Picture;
use crate::test_support::{item, source_deck, source_slide, text_item, with_role};

/// Page 960 × 540 : bande haute sous 64,8 pt, bande basse au-delà de 475,2 pt (centres).
fn footer(slide: u32) -> Item {
    text_item(&format!("s{slide}/f"), (64.0, 497.0, 270.0, 12.0), &format!("{slide} · Wealth Bank as an API"))
}

fn logo(slide: u32) -> Item {
    let mut i = item(
        &format!("s{slide}/logo"),
        (824.0, 484.0, 72.0, 32.0),
        ItemKind::Picture(Picture { asset: "sha256:logo".into(), crop: Crop::default() }),
    );
    i.name = "Amundi".into();
    i
}

/// Quatre slides : pied numéroté et logo sur les slides 2 à 4 (3 ≥ la moitié de 4),
/// un en-tête propre à la slide 1, un numéro seul en slide 4, une date en slide 2.
fn deck() -> SourceDeck {
    let body = |s: u32| text_item(&format!("s{s}/body"), (64.0, 200.0, 800.0, 100.0), "Corps");
    source_deck(vec![
        source_slide(1, None, vec![text_item("s1/brand", (64.0, 40.0, 120.0, 20.0), "ALTO WEALTH"), body(1)]),
        source_slide(
            2,
            None,
            vec![
                body(2),
                footer(2),
                logo(2),
                with_role(text_item("s2/date", (0.0, 200.0, 100.0, 20.0), "10/10/2026"), Role::Date),
            ],
        ),
        source_slide(3, None, vec![body(3), footer(3), logo(3)]),
        source_slide(4, None, vec![body(4), footer(4), logo(4), text_item("s4/n", (64.0, 494.0, 10.0, 16.0), "7")]),
    ])
}

#[test]
fn decor_skips_repeated_header_and_footer_items_and_placeholders() {
    let d = deck();
    let decor = Decor::of(&d);
    let skipped: Vec<&str> =
        d.slides.iter().flat_map(|s| &s.items).filter(|i| decor.skips(i)).map(|i| i.id.as_str()).collect();
    assert_that!(skipped).is_equal_to(vec!["s2/f", "s2/logo", "s2/date", "s3/f", "s3/logo", "s4/f", "s4/logo", "s4/n"]);
}

#[test]
fn decor_reports_each_repeated_motif_once_on_its_first_slide() {
    let decor = Decor::of(&deck());
    // Motifs dans l'ordre de leur clé : `image:…` avant le texte.
    assert_that!(decor.losses(2).to_vec()).is_equal_to(vec![
        Loss::on("s2/logo", LossKind::RepeatedDecor { text: "Amundi".into(), slides: 3 }),
        Loss::on("s2/f", LossKind::RepeatedDecor { text: "2 · Wealth Bank as an API".into(), slides: 3 }),
    ]);
    assert_that!(decor.losses(3).is_empty() && decor.losses(1).is_empty()).is_true();
}

/// Sur deux slides seulement, un pied ne fait pas la moitié de cinq : il reste.
#[test]
fn decor_keeps_a_band_item_repeated_on_fewer_than_half_of_the_slides() {
    let mut d = deck();
    d.slides.push(source_slide(5, None, Vec::new()));
    d.slides.push(source_slide(6, None, Vec::new()));
    d.slides[3].items.retain(|i| i.id != "s4/f");
    let decor = Decor::of(&d);
    assert_that!(decor.skips(&footer(2))).is_false();
}

#[test]
fn decor_keeps_an_item_outside_the_bands() {
    let decor = Decor::of(&deck());
    let body = text_item("s2/body", (64.0, 200.0, 800.0, 100.0), "Corps");
    assert_that!(decor.skips(&body)).is_false();
}

#[test]
fn in_band_and_backdrop_follow_the_page() {
    let page = Size { w: 960.0, h: 540.0 };
    assert_that!(in_band(&Frame::new(0.0, 50.0, 10.0, 20.0), page)).is_true();
    assert_that!(in_band(&Frame::new(0.0, 60.0, 10.0, 20.0), page)).is_false();
    let panel = item(
        "p",
        (0.0, 108.0, 960.0, 432.0),
        ItemKind::Shape(Box::new(crate::source::Shape {
            geom: crate::source::Geometry::Preset { name: "rect".into(), adj: std::collections::BTreeMap::default() },
            fill: crate::source::Fill::None,
            stroke: None,
            text: None,
            connection: None,
        })),
    );
    assert_that!(is_backdrop(&panel, page)).is_true();
}
