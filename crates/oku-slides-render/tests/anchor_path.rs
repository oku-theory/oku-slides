//! Base étendue : texte ancré dans son cadre (`valign`, `inset`), espacé
//! (`tracking`), interligné (`leading`), paragraphes espacés (`spacing`), puce de liste sans couleur propre,
//! et forme `path`.

mod common;

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_core::kinds::{base, essentiel};
use oku_slides_render::{Compiled, ElementBox, FieldBox};
use serde_json::{Value, json};
use speculoos::prelude::*;

fn compile(elements: Value) -> Compiled {
    let d = json!({ "slides": [{ "id": "s", "kind": essentiel::BLANK, "params": { "elements": elements } }] });
    let deck = Deck::from_json(&d.to_string()).unwrap_or_else(|e| panic!("{e}"));
    let mut r = common::renderer();
    r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

/// Un texte d'une ligne sur un cadre de 600 × 400 pt, au style `style`.
fn text(style: Value) -> Value {
    json!([{ "id": "t", "kind": base::TEXT, "frame": { "x": 100, "y": 100, "w": 600, "h": 400 }, "style": style,
        "params": { "body": [{ "runs": [{ "t": "Ancré" }] }] } }])
}

/// La boîte de l'élément `t` et celle de son champ `body`.
fn boxes(c: &Compiled) -> (ElementBox, FieldBox) {
    let layout = c.layout();
    let element = layout.boxes.iter().find(|b| b.id == "t").unwrap_or_else(|| panic!("pas de boîte t"));
    let field = layout.fields.iter().find(|f| f.owner == "t").unwrap_or_else(|| panic!("pas de champ t.body"));
    (element.clone(), field.clone())
}

#[test]
fn text_valign_middle_with_inset_centres_the_body_in_the_inset_frame() {
    let (frame, field) = boxes(&compile(text(json!({ "valign": "middle", "inset": [40, 0, 0, 30] }))));
    // Centre de la zone intérieure : 40 pt sous le haut, 360 pt de haut.
    assert_that!((field.y + field.h / 2.0 - (frame.y + 40.0 + 180.0)).abs()).is_less_than(0.5);
    assert_that!((field.x - (frame.x + 30.0)).abs()).is_less_than(0.5);
}

#[test]
fn text_valign_bottom_puts_the_body_at_the_bottom_of_its_frame() {
    let (frame, field) = boxes(&compile(text(json!({ "valign": "bottom" }))));
    assert_that!((field.y + field.h - (frame.y + frame.h)).abs()).is_less_than(0.5);
}

#[test]
fn text_without_valign_nor_inset_keeps_the_body_at_the_top_left() {
    let (frame, plain) = boxes(&compile(text(json!({}))));
    let (_, top) = boxes(&compile(text(json!({ "valign": "top" }))));
    assert_that!((plain.y - frame.y).abs()).is_less_than(0.5);
    assert_that!((plain.x - frame.x).abs()).is_less_than(0.5);
    assert_that!((top.x, top.y)).is_equal_to((plain.x, plain.y));
}

#[test]
fn text_in_flow_ignores_valign() {
    let slide = |style: Value| {
        let d = json!({ "slides": [{ "id": "s", "kind": essentiel::CONTENT, "params": { "title": "T", "blocks": [
            { "id": "t", "kind": base::TEXT, "style": style, "params": { "body": [{ "runs": [{ "t": "Flux" }] }] } }
        ] } }] });
        let deck = Deck::from_json(&d.to_string()).unwrap_or_else(|e| panic!("{e}"));
        let mut r = common::renderer();
        boxes(&r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))).1
    };
    assert_that!(slide(json!({ "valign": "bottom" }))).is_equal_to(slide(json!({})));
}

#[test]
fn text_tracking_widens_the_body_and_its_absence_keeps_it() {
    let width = |style: Value| boxes(&compile(text(style))).1.w;
    let plain = width(json!({}));
    // « Ancré » : cinq lettres, quatre intervalles de 10 pt de plus.
    assert_that!((width(json!({ "tracking": 10 })) - plain - 40.0).abs()).is_less_than(0.5);
    assert_that!(width(json!({ "tracking": -1 }))).is_less_than(plain);
    assert_that!(width(json!({ "tracking": 0 }))).is_equal_to(plain);
}

/// Les boîtes d'un texte de deux lignes de 20 pt à l'interligne `leading` (em).
fn two_lines(leading: f64) -> (ElementBox, FieldBox) {
    let e = json!([{ "id": "t", "kind": base::TEXT, "frame": { "x": 100, "y": 100, "w": 600, "h": 400 },
        "style": { "size": 20, "leading": leading },
        "params": { "body": [{ "runs": [{ "t": "Une\nDeux" }] }] } }]);
    boxes(&compile(e))
}

#[test]
fn text_leading_spaces_the_lines_as_its_box_style_says() {
    let ((tight, tight_body), (loose, loose_body)) = (two_lines(0.3), two_lines(2.0));
    // Une ligne de plus d'écart : (2,0 − 0,3) × 20 pt.
    let grown = loose_body.h - tight_body.h;
    assert_that!((grown - 34.0).abs()).is_less_than(0.5);
    // La sonde de l'éditeur annonce l'interligne rendu.
    let lead = |b: &ElementBox| b.style.as_ref().map(|s| s.leading).unwrap_or_else(|| panic!("pas de style"));
    assert_that!((lead(&tight) - 6.0).abs()).is_less_than(0.01);
    assert_that!((lead(&loose) - lead(&tight) - grown).abs()).is_less_than(0.5);
}

/// La hauteur du corps d'un texte de 20 pt, interligne 0,5 em, en deux
/// paragraphes de genre `list` (`null` : paragraphe simple), au style
/// `style` complété.
fn two_paragraphs(list: Value, style: Value) -> f64 {
    let mut s = json!({ "size": 20, "leading": 0.5 });
    if let (Some(s), Some(extra)) = (s.as_object_mut(), style.as_object()) {
        s.extend(extra.clone());
    }
    let para = |t: &str| json!({ "runs": [{ "t": t }], "list": list });
    let e = json!([{ "id": "t", "kind": base::TEXT, "frame": { "x": 100, "y": 100, "w": 600, "h": 400 }, "style": s,
        "params": { "body": [para("Un"), para("Deux")] } }]);
    boxes(&compile(e)).1.h
}

#[rstest::rstest]
#[case::paragraphs(Value::Null)]
#[case::bullets(json!("bullet"))]
#[case::numbers(json!("number"))]
fn text_spacing_sets_the_gap_between_paragraphs(#[case] list: Value) {
    let (none, wide) =
        (two_paragraphs(list.clone(), json!({ "spacing": 0 })), two_paragraphs(list, json!({ "spacing": 30 })));
    // Le seul écart entre les deux paragraphes grandit de 30 pt.
    assert_that!((wide - none - 30.0).abs()).is_less_than(0.01);
}

#[test]
fn text_spacing_between_a_paragraph_and_a_list_item_is_the_same() {
    let mixed = |spacing: f64| {
        let e = json!([{ "id": "t", "kind": base::TEXT, "frame": { "x": 100, "y": 100, "w": 600, "h": 400 },
            "style": { "size": 20, "spacing": spacing },
            "params": { "body": [{ "runs": [{ "t": "Un" }] }, { "runs": [{ "t": "Deux" }], "list": "bullet" }] } }]);
        boxes(&compile(e)).1.h
    };
    assert_that!((mixed(25.0) - mixed(5.0) - 20.0).abs()).is_less_than(0.01);
}

#[test]
fn text_without_spacing_keeps_the_theme_gaps() {
    for list in [Value::Null, json!("bullet")] {
        let theme = two_paragraphs(list.clone(), json!({}));
        assert_that!(two_paragraphs(list.clone(), json!({ "spacing": 1e6 }))).is_equal_to(theme);
        assert_that!(two_paragraphs(list, json!({ "spacing": 0 }))).is_less_than(theme);
    }
}

/// SVG d'une liste à puces en `#00ff00` dont la puce du thème est `•` en `marker_color`.
fn bullets_svg(marker_color: Value) -> String {
    let d = json!({ "tokens": { "list": { "marker": "•", "marker-color": marker_color } },
        "slides": [{ "id": "s", "kind": essentiel::BLANK, "params": { "elements": [
            { "id": "t", "kind": base::TEXT, "frame": { "x": 100, "y": 100, "w": 600, "h": 400 },
              "style": { "color": "#00ff00" },
              "params": { "body": [{ "runs": [{ "t": "Un" }], "list": "bullet" }] } }] } }] });
    let deck = Deck::from_json(&d.to_string()).unwrap_or_else(|e| panic!("{e}"));
    let mut r = common::renderer();
    let c = r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    c.svg(0).unwrap_or_else(|e| panic!("{e}")).to_lowercase()
}

#[test]
fn list_marker_without_color_takes_the_text_color() {
    assert_that!(bullets_svg(json!("#ff0000"))).contains("#ff0000");
    let plain = bullets_svg(Value::Null);
    assert_that!(plain).does_not_contain("#ff0000");
    assert_that!(plain).contains("#00ff00");
}

/// Cercle de centre (0,5 ; 0,5) et de rayon `r`, en quatre cubiques ;
/// `clockwise` : sens horaire à l'écran (y vers le bas).
fn circle(r: f64, clockwise: bool) -> String {
    let k = 0.5523 * r;
    let (c, lo, hi) = (0.5, 0.5 - r, 0.5 + r);
    if clockwise {
        format!(
            "M {hi} {c} C {hi} {} {} {hi} {c} {hi} C {} {hi} {lo} {} {lo} {c} C {lo} {} {} {lo} {c} {lo} C {} {lo} {hi} {} {hi} {c} Z",
            c + k,
            c + k,
            c - k,
            c + k,
            c - k,
            c - k,
            c + k,
            c - k
        )
    } else {
        format!(
            "M {hi} {c} C {hi} {} {} {lo} {c} {lo} C {} {lo} {lo} {} {lo} {c} C {lo} {} {} {hi} {c} {hi} C {} {hi} {hi} {} {hi} {c} Z",
            c - k,
            c + k,
            c - k,
            c - k,
            c + k,
            c - k,
            c + k,
            c + k
        )
    }
}

/// Une forme `path` rouge de 200 × 200 pt.
fn path(d: &str) -> Value {
    json!([{ "id": "p", "kind": base::SHAPE, "frame": { "x": 0, "y": 0, "w": 200, "h": 200 },
        "params": { "shape": "path", "path": { "d": d } }, "style": { "fill": "#ff0000" } }])
}

/// Le pixel RGBA de la page 1 au point (`x`, `y`) de l'élément `p`, à 1 px par pt.
fn pixel(c: &Compiled, x: f64, y: f64) -> [u8; 4] {
    let b = c.layout().boxes.iter().find(|b| b.id == "p").unwrap_or_else(|| panic!("pas de boîte p")).clone();
    let png = c.png(0, 1.0).unwrap_or_else(|e| panic!("{e}"));
    let mut reader = png::Decoder::new(std::io::Cursor::new(png)).read_info().unwrap_or_else(|e| panic!("{e}"));
    let mut buf = vec![0; reader.output_buffer_size().unwrap_or_else(|| panic!("taille de PNG inconnue"))];
    let info = reader.next_frame(&mut buf).unwrap_or_else(|e| panic!("{e}"));
    let channels = info.line_size / info.width as usize;
    let at = (b.y + y) as usize * info.line_size + (b.x + x) as usize * channels;
    let mut out = [255; 4];
    out[..channels.min(4)].copy_from_slice(&buf[at..at + channels.min(4)]);
    out
}

fn is_red(p: [u8; 4]) -> bool {
    p[0] > 200 && p[1] < 60 && p[2] < 60
}

#[test]
fn path_ring_of_two_opposite_subpaths_leaves_its_centre_empty() {
    let ring = compile(path(&format!("{} {}", circle(0.5, true), circle(0.25, false))));
    // Rayon 0,375 : dans l'anneau ; centre : dans le trou.
    assert_that!(is_red(pixel(&ring, 100.0, 25.0))).is_true();
    assert_that!(is_red(pixel(&ring, 25.0, 100.0))).is_true();
    assert_that!(is_red(pixel(&ring, 100.0, 100.0))).is_false();
}

#[test]
fn path_with_subpaths_of_the_same_direction_fills_non_zero() {
    let disc = compile(path(&format!("{} {}", circle(0.5, true), circle(0.25, true))));
    assert_that!(is_red(pixel(&disc, 100.0, 100.0))).is_true();
}

#[test]
fn path_triangle_with_lines_and_quad_stretches_to_its_frame() {
    let c = compile(path("M 0 1 L .5 +0 Q 1 0.5 1. 1 Z"));
    assert_that!(is_red(pixel(&c, 100.0, 180.0))).is_true();
    assert_that!(is_red(pixel(&c, 10.0, 10.0))).is_false();
}

#[rstest::rstest]
#[case::empty("")]
#[case::no_move("L 0 0 L 1 1")]
#[case::missing_number("M 0 L 1 1")]
#[case::extra_number("M 0 0 1 L 1 1")]
#[case::unknown_command("M 0 0 A 1 1 0 0 1 1 1")]
#[case::exponent("M 0 0 L 1e2 1")]
#[case::far_away("M 0 0 L 5000 1")]
#[case::bad_number("M 0 0 L 1.2.3 1")]
#[case::glued_letter("M0 0 L 1 1")]
fn path_malformed_draws_nothing(#[case] d: &str) {
    let svg = compile(path(d)).svg(0).unwrap_or_else(|e| panic!("{e}")).to_lowercase();
    assert_that!(svg).does_not_contain("#ff0000");
}

#[test]
fn path_too_long_draws_nothing() {
    let d = format!("M 0 0{}", " L 1 1".repeat(oku_slides_core::SHAPE_PATH_MAX / 6));
    let svg = compile(path(&d)).svg(0).unwrap_or_else(|e| panic!("{e}")).to_lowercase();
    assert_that!(svg).does_not_contain("#ff0000");
}
