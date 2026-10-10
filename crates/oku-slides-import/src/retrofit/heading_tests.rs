use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::source::ItemKind;
use crate::test_support::{item, para, run_with, text_box, text_item, with_role};

const PAGE: Size = Size { w: 960.0, h: 540.0 };

/// Une boîte de texte d'un paragraphe en taille `size`.
fn sized(id: &str, frame: (f64, f64, f64, f64), text: &str, size: f64) -> Item {
    item(id, frame, ItemKind::Text(text_box(vec![para(vec![run_with(text, |s| s.size = size)])])))
}

/// Slide 2 d'alto : surtitre 9,5 pt, titre 28 pt, cartes (titres 13 pt, corps 10,5 pt).
/// Tailles triées : 9,5 10,5 10,5 13 13 28 ; médiane basse (rang 2) : 10,5 ; 28 ≥ 1,4 × 10,5.
fn conviction() -> Vec<Item> {
    vec![
        sized("kicker", (64.0, 56.0, 857.0, 15.3), "OUR CONVICTION", 9.5),
        sized("title", (64.0, 89.8, 772.5, 66.4), "The agent becomes the only interface", 28.0),
        sized("c1", (80.0, 196.2, 252.7, 17.6), "The only window", 13.0),
        sized("b1", (80.0, 219.8, 252.7, 69.2), "The agent becomes the single window", 10.5),
        sized("c2", (357.3, 196.2, 252.7, 17.6), "A deeper confidant", 13.0),
        sized("b2", (357.3, 219.8, 252.7, 86.0), "People will confide", 10.5),
    ]
}

/// Slide 1 d'alto : bandeau (15 et 9,5 pt), surtitre 9,5, titre 52, sous-titre 12, corps 16, méta 9,5.
/// Tailles triées : 9,5 ×4, 12, 15, 16, 52 ; médiane basse (rang 3) : 9,5 ; 12 ≥ 1,15 × 9,5.
fn cover() -> Vec<Item> {
    vec![
        sized("brand", (64.0, 48.0, 117.8, 23.0), "ALTO WEALTH", 15.0),
        sized("doc", (637.5, 51.8, 165.9, 15.3), "D2C Strategy — Working Document", 9.5),
        sized("kicker", (64.0, 172.9, 857.0, 15.3), "FROM A SURVIVAL QUESTION", 9.5),
        sized("title", (64.0, 200.2, 857.0, 111.2), "The Agent Is the New Front Door", 52.0),
        sized("subtitle", (64.0, 323.4, 857.0, 18.8), "From Software Vendor to Wealth Infrastructure", 12.0),
        sized("body", (64.0, 354.2, 643.8, 69.2), "As the conversational agent becomes…", 16.0),
        sized("meta1", (64.0, 435.4, 857.0, 15.3), "Distribution to Customer", 9.5),
        sized("meta2", (64.0, 462.7, 857.0, 15.3), "Matthieu Dartiguenave", 9.5),
    ]
}

fn roles(items: &[Item]) -> Vec<(String, Heading)> {
    infer_roles(&items.iter().collect::<Vec<_>>(), PAGE)
}

#[test]
fn infer_roles_of_a_content_slide_finds_its_title_and_kicker() {
    assert_that!(roles(&conviction()))
        .is_equal_to(vec![("title".to_owned(), Heading::Title), ("kicker".to_owned(), Heading::Kicker)]);
}

#[test]
fn infer_roles_of_a_cover_adds_the_subtitle_below_a_large_title() {
    assert_that!(roles(&cover())).is_equal_to(vec![
        ("title".to_owned(), Heading::Title),
        ("kicker".to_owned(), Heading::Kicker),
        ("subtitle".to_owned(), Heading::Subtitle),
    ]);
}

/// Pas de sous-titre sous un titre de 28 pt, même juste au-dessous.
#[test]
fn infer_roles_under_a_small_title_finds_no_subtitle() {
    let mut items = conviction();
    items.push(sized("under", (64.0, 160.0, 700.0, 20.0), "Our client is disappearing", 14.0));
    assert_that!(roles(&items).iter().any(|(_, h)| *h == Heading::Subtitle)).is_false();
}

#[rstest]
#[case::too_small(vec![sized("t", (64.0, 90.0, 800.0, 30.0), "Petit", 18.0), sized("b", (64.0, 200.0, 800.0, 30.0), "Corps", 10.0)])]
#[case::too_low(vec![sized("t", (64.0, 300.0, 800.0, 30.0), "Bas", 28.0), sized("b", (64.0, 400.0, 800.0, 30.0), "Corps", 10.0)])]
#[case::not_above_the_rest(vec![sized("t", (64.0, 90.0, 800.0, 30.0), "Égal", 24.0), sized("b", (64.0, 200.0, 800.0, 30.0), "Corps", 20.0)])]
#[case::no_text(vec![])]
fn infer_roles_without_a_standing_out_title_finds_nothing(#[case] items: Vec<Item>) {
    assert_that!(roles(&items)).is_empty();
}

/// Un surtitre trop loin (plus de 0,1 × 540 = 54 pt au-dessus) ou de deux paragraphes n'en est pas un.
#[test]
fn infer_roles_rejects_a_far_or_long_kicker() {
    let mut items = conviction();
    items[0].frame.y = 10.0;
    assert_that!(roles(&items)).is_equal_to(vec![("title".to_owned(), Heading::Title)]);
    let mut items = conviction();
    items[0].kind = ItemKind::Text(text_box(vec![
        para(vec![run_with("UN", |s| s.size = 9.5)]),
        para(vec![run_with("DEUX", |s| s.size = 9.5)]),
    ]));
    assert_that!(roles(&items)).is_equal_to(vec![("title".to_owned(), Heading::Title)]);
}

#[test]
fn headings_of_a_cover_find_title_subtitle_kicker_and_the_cover() {
    let items = cover();
    let refs: Vec<&Item> = items.iter().collect();
    let h = headings(&refs, PAGE);
    let id = |i: Option<&Item>| i.map(|i| i.id.clone());
    assert_that!((id(h.title), id(h.subtitle), id(h.kicker), h.cover)).is_equal_to((
        Some("title".to_owned()),
        Some("subtitle".to_owned()),
        Some("kicker".to_owned()),
        true,
    ));
}

/// Une autre ligne de 28 pt sous le titre de 52 pt : un second titre, pas une couverture.
#[test]
fn headings_with_another_large_text_are_not_a_cover() {
    let mut items = cover();
    items.push(sized("second", (64.0, 440.0, 800.0, 40.0), "Encore un titre", 28.0));
    let refs: Vec<&Item> = items.iter().collect();
    assert_that!(headings(&refs, PAGE).cover).is_false();
}

#[test]
fn headings_prefer_the_placeholders() {
    let items = [
        with_role(text_item("ph", (40.0, 300.0, 880.0, 60.0), "Titre"), Role::Title),
        with_role(text_item("sub", (40.0, 380.0, 880.0, 30.0), "Sous-titre"), Role::Subtitle),
        sized("big", (40.0, 20.0, 880.0, 60.0), "Gros texte", 60.0),
    ];
    let refs: Vec<&Item> = items.iter().collect();
    let h = headings(&refs, PAGE);
    assert_that!((h.title.map(|i| i.id.as_str()), h.subtitle.map(|i| i.id.as_str()), h.kicker, h.cover)).is_equal_to((
        Some("ph"),
        Some("sub"),
        None,
        false,
    ));
}
