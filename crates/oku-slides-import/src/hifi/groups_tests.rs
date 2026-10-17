use speculoos::prelude::*;

use super::*;
use crate::source::{Item, ItemKind, SmartArt};
use crate::test_support::{item, source_deck, source_slide, text_item};

/// Un groupe de deux textes côte à côte, à `x`, d'id `id`.
fn pair(id: &str, x: f64, smartart: bool) -> Item {
    let items = vec![
        text_item(&format!("{id}/a"), (x, 0.0, 100.0, 50.0), "a"),
        text_item(&format!("{id}/b"), (x + 120.0, 0.0, 100.0, 50.0), "b"),
    ];
    let smartart = smartart.then(|| Box::new(SmartArt { layout: None, category: None, nodes: Vec::new() }));
    item(id, (x, 0.0, 220.0, 50.0), ItemKind::Group { items, smartart })
}

#[test]
fn repeated_keeps_a_group_found_on_three_slides() {
    let slides = (1..=3).map(|n| source_slide(n, None, vec![pair(&format!("s{n}/1"), f64::from(n) * 10.0, false)]));
    let groups = repeated(&source_deck(slides.collect()));
    assert_that!(groups).has_length(1);
    let slides: Vec<&str> = groups[0].occurrences.iter().map(|(s, _)| s.as_str()).collect();
    assert_that!(slides).is_equal_to(vec!["s1", "s2", "s3"]);
    assert_that!(groups[0].occurrences[0].1.as_str()).is_equal_to("s1/1");
}

#[test]
fn repeated_ignores_a_group_on_two_slides_even_twice_each() {
    let slides = (1..=2).map(|n| {
        source_slide(n, None, vec![pair(&format!("s{n}/1"), 0.0, false), pair(&format!("s{n}/2"), 300.0, false)])
    });
    assert_that!(repeated(&source_deck(slides.collect()))).is_empty();
}

#[test]
fn repeated_ignores_drawn_smartart() {
    let slides = (1..=3).map(|n| source_slide(n, None, vec![pair(&format!("s{n}/1"), 0.0, true)]));
    assert_that!(repeated(&source_deck(slides.collect()))).is_empty();
}
