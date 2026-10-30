//! Heading paragraphs (t-1284): size and style from the `heading.hN` tokens, kept out of the TOC.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_core::kinds::{base, essentiel};
use oku_slides_render::Compiled;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// Test tokens: a red 120 pt level-2 heading and a green bullet.
fn tokens() -> Value {
    json!({ "heading": { "h2": { "size": 120, "color": "#ff0000", "weight": 800 } },
            "weight": { "strong": 300 }, "list": { "marker": "•", "marker-color": "#00ff00" } })
}

/// The `before` slides, then a blank slide with one text of paragraphs `body` (1600 x 800 frame).
fn compile(before: &[Value], body: &Value) -> Compiled {
    let mut slides = before.to_vec();
    slides.push(json!({ "id": "s", "kind": essentiel::BLANK, "params": { "elements": [
        { "id": "t", "kind": base::TEXT, "frame": { "x": 100, "y": 100, "w": 1600, "h": 800 },
          "params": { "body": body } }] } }));
    let d = json!({ "tokens": tokens(), "slides": slides });
    let deck = Deck::from_json(&d.to_string()).unwrap_or_else(|e| panic!("{e}"));
    let mut r = common::renderer();
    r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

/// Number of pixel rows of the last page holding a pixel that satisfies `pred`.
fn rows(c: &Compiled, pred: fn([u8; 4]) -> bool) -> usize {
    let (px, width) = common::pixels_of(c, c.page_count() - 1);
    px.chunks(width).filter(|row| row.iter().any(|p| pred(*p))).count()
}

/// Number of pixels of the last page that satisfy `pred`.
fn ink(c: &Compiled, pred: fn([u8; 4]) -> bool) -> usize {
    let (px, _) = common::pixels_of(c, c.page_count() - 1);
    px.into_iter().filter(|p| pred(*p)).count()
}

fn is_green(p: [u8; 4]) -> bool {
    p[1] > 200 && p[0] < 80 && p[2] < 80
}

#[test]
fn heading_h2_renders_at_its_token_size() {
    let c = compile(&[], &json!([{ "runs": [{ "t": "H" }], "heading": 2 }]));
    assert_that!(rows(&c, common::is_red)).is_greater_than(60);
}

#[test]
fn paragraph_without_heading_renders_at_body_size() {
    let c = compile(&[], &json!([{ "runs": [{ "t": "H", "color": "#ff0000" }] }]));
    assert_that!(rows(&c, common::is_red)).is_less_than(40);
}

#[test]
fn heading_and_list_render_as_a_heading_without_marker() {
    let c = compile(&[], &json!([{ "runs": [{ "t": "H" }], "heading": 2, "list": "bullet" }]));
    assert_that!(rows(&c, common::is_red)).is_greater_than(60);
    assert_that!(ink(&c, is_green)).is_equal_to(0);
}

#[test]
fn bold_run_in_a_heading_is_never_lighter_than_the_heading() {
    let plain = ink(&compile(&[], &json!([{ "runs": [{ "t": "Haut" }], "heading": 2 }])), common::is_red);
    let bold = ink(&compile(&[], &json!([{ "runs": [{ "t": "Haut", "bold": true }], "heading": 2 }])), common::is_red);
    assert_that!(plain).is_greater_than(0);
    assert_that!(bold * 100).is_greater_than_or_equal_to(plain * 95);
}

#[test]
fn body_heading_leaves_the_toc_page_unchanged() {
    let before = [
        json!({ "id": "s-toc", "kind": essentiel::TOC }),
        json!({ "id": "s-ch", "kind": essentiel::CHAPTER, "params": { "title": "Chapitre" } }),
    ];
    let plain = compile(&before, &json!([{ "runs": [{ "t": "Intrus" }] }]));
    let heading = compile(&before, &json!([{ "runs": [{ "t": "Intrus" }], "heading": 1 }]));
    assert_that!(heading.page_hash(0)).is_equal_to(plain.page_hash(0));
}
