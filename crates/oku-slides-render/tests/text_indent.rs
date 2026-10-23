//! Retraits d'un texte (`style.indent`, t-863) : marge gauche des
//! paragraphes et décalage de leur première ligne, puce suspendue.

mod common;

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_core::kinds::{base, essentiel};
use oku_slides_render::Compiled;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// Une slide d'un texte rouge de 40 pt dans un cadre de 800 × 400 pt, au style
/// `style` augmenté, de paragraphes `body` ; puce du thème `•` en vert.
fn compile(style: Value, body: Value) -> Compiled {
    let mut style = style;
    if let Some(s) = style.as_object_mut() {
        s.insert("size".into(), json!(40));
        s.insert("color".into(), json!("#ff0000"));
    }
    let d = json!({ "tokens": { "list": { "marker": "•", "marker-color": "#00ff00" } },
        "slides": [{ "id": "s", "kind": essentiel::BLANK, "params": { "elements": [
            { "id": "t", "kind": base::TEXT, "frame": { "x": 100, "y": 100, "w": 800, "h": 400 },
              "style": style, "params": { "body": body } }] } }] });
    let deck = Deck::from_json(&d.to_string()).unwrap_or_else(|e| panic!("{e}"));
    let mut r = common::renderer();
    r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

/// Une puce d'une ligne.
fn bullet() -> Value {
    json!([{ "runs": [{ "t": "Puce" }], "list": "bullet" }])
}

/// Un paragraphe de trois lignes au moins sur 800 pt.
fn long() -> Value {
    json!([{ "runs": [{ "t": "mot ".repeat(60) }] }])
}

/// La page 1 en RGBA, 1 px par pt : (pixels, largeur).
fn pixels(c: &Compiled) -> (Vec<[u8; 4]>, usize) {
    let png = c.png(0, 1.0).unwrap_or_else(|e| panic!("{e}"));
    let mut reader = png::Decoder::new(std::io::Cursor::new(png)).read_info().unwrap_or_else(|e| panic!("{e}"));
    let mut buf = vec![0; reader.output_buffer_size().unwrap_or_else(|| panic!("taille de PNG inconnue"))];
    let info = reader.next_frame(&mut buf).unwrap_or_else(|e| panic!("{e}"));
    let channels = info.line_size / info.width as usize;
    let px = buf[..info.line_size * info.height as usize]
        .chunks(channels)
        .map(|p| {
            let mut out = [255; 4];
            out[..channels.min(4)].copy_from_slice(&p[..channels.min(4)]);
            out
        })
        .collect();
    (px, info.width as usize)
}

fn is_red(p: [u8; 4]) -> bool {
    p[0] > 200 && p[1] < 80 && p[2] < 80
}

fn is_green(p: [u8; 4]) -> bool {
    p[1] > 200 && p[0] < 80 && p[2] < 80
}

/// Abscisse (pt, depuis la gauche du cadre) du pixel le plus à gauche qui
/// vérifie `pred` entre les ordonnées `y0` et `y1` (pt, depuis son haut).
fn leftmost(c: &Compiled, pred: fn([u8; 4]) -> bool, band: (usize, usize)) -> usize {
    leftmost_from(c, pred, 0, band)
}

/// [`leftmost`] à partir de l'abscisse `x0` (pt, depuis la gauche du cadre).
fn leftmost_from(c: &Compiled, pred: fn([u8; 4]) -> bool, x0: usize, (y0, y1): (usize, usize)) -> usize {
    let b = c.layout().boxes.iter().find(|b| b.id == "t").unwrap_or_else(|| panic!("pas de boîte t")).clone();
    let (left, top) = (b.x.round() as usize, b.y.round() as usize);
    let (px, width) = pixels(c);
    (x0..800)
        .find(|x| (y0..y1).any(|y| pred(px[(top + y) * width + left + x])))
        .unwrap_or_else(|| panic!("aucun pixel entre {y0} et {y1} après {x0}"))
}

/// La première ligne de texte (40 pt) du cadre.
const FIRST: (usize, usize) = (0, 48);
/// La deuxième ligne de texte du cadre.
const SECOND: (usize, usize) = (52, 96);

#[test]
fn text_indent_hangs_the_bullet_and_starts_the_item_text_at_the_margin() {
    let c = compile(json!({ "indent": [60, -60] }), bullet());
    assert_that!(leftmost(&c, is_green, FIRST)).is_less_than(15);
    let text = leftmost(&c, is_red, FIRST);
    asserting(&format!("texte à {text}")).that(&(60..66).contains(&text)).is_true();
}

#[test]
fn text_indent_moves_the_item_text_with_the_margin() {
    let at = |margin: i32| leftmost(&compile(json!({ "indent": [margin, -margin] }), bullet()), is_red, FIRST);
    assert_that!(at(90) - at(60)).is_equal_to(30);
}

#[test]
fn text_indent_hangs_the_wrapped_lines_of_a_paragraph() {
    let c = compile(json!({ "indent": [60, -60] }), long());
    assert_that!(leftmost(&c, is_red, FIRST)).is_less_than(6);
    let second = leftmost(&c, is_red, SECOND);
    asserting(&format!("2e ligne à {second}")).that(&(60..66).contains(&second)).is_true();
}

#[test]
fn text_indent_with_a_positive_offset_indents_the_first_line_only() {
    let c = compile(json!({ "indent": [0, 40] }), long());
    let first = leftmost(&c, is_red, FIRST);
    asserting(&format!("1re ligne à {first}")).that(&(40..46).contains(&first)).is_true();
    assert_that!(leftmost(&c, is_red, SECOND)).is_less_than(6);
}

#[test]
fn text_indent_hangs_the_number_of_an_enum_item() {
    let c = compile(json!({ "indent": [60, -60] }), json!([{ "runs": [{ "t": "Un" }], "list": "number" }]));
    // « 1. » au bord, puis le texte à la marge.
    assert_that!(leftmost(&c, is_red, FIRST)).is_less_than(6);
    let text = leftmost_from(&c, is_red, 40, FIRST);
    asserting(&format!("texte à {text}")).that(&(60..66).contains(&text)).is_true();
}

#[test]
fn text_indent_out_of_its_domain_is_ignored() {
    let plain = pixels(&compile(json!({}), bullet()));
    assert_that!(pixels(&compile(json!({ "indent": [-5, 0] }), bullet()))).is_equal_to(plain);
}
