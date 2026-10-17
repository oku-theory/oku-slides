use oku_slides_import::source::{
    Color, Fill, Frame, Geometry, Item, ItemKind, Opaque, OpaqueReason, Rgba, Shape, Stroke, TextBox,
};
use serde_json::json;
use speculoos::prelude::*;

use super::{MAX_SAMPLE_ITEMS, sample};

/// Un item `id` de sorte `kind`, cadré en `frame`.
fn item(id: &str, frame: Frame, kind: ItemKind) -> Item {
    Item { id: id.into(), name: String::new(), alt: None, frame, z: 0, role: None, placeholder: None, kind }
}

/// Un item intraduisible : la plus petite sorte d'item.
fn opaque(id: &str) -> Item {
    item(id, Frame::new(0.0, 0.0, 10.0, 10.0), ItemKind::Opaque(Opaque { reason: OpaqueReason::Math, fallback: None }))
}

/// Une forme `rect` remplie de `fill`, contour bleu explicite.
fn shape(fill: Fill) -> Item {
    let stroke = Stroke {
        width: 1.5,
        color: Color::explicit(Rgba::new(0, 0, 255)),
        dash: oku_slides_import::source::Dash::Solid,
        head: None,
        tail: None,
    };
    let geom = Geometry::Preset { name: "roundRect".into(), adj: Default::default() };
    let kind = ItemKind::Shape(Box::new(Shape { geom, fill, stroke: Some(stroke), text: None, connection: None }));
    item("s3/4", Frame::new(12.25, 40.0, 200.04, 80.0), kind)
}

#[test]
fn sample_of_a_shape_gives_its_kind_frame_in_pt_and_colours() {
    let fill = Fill::Solid { color: Color::mapped(Rgba::new(0x44, 0x72, 0xc4), "accent1", "accent1", Vec::new()) };
    let v = sample(&[shape(fill)]);
    assert_that!(v).is_equal_to(json!({ "items": [{
        "kind": "shape",
        "geometry": "roundRect",
        "frame": [12.3, 40.0, 200.0, 80.0],
        "fill": { "hex": "#4472c4", "slot": "accent1" },
        "stroke": { "hex": "#0000ff" },
    }] }));
}

#[test]
fn sample_of_a_text_box_counts_its_paragraphs() {
    let v = sample(&[item("s3/5", Frame::new(0.0, 0.0, 100.0, 20.0), ItemKind::Text(TextBox::default()))]);
    assert_that!(v)
        .is_equal_to(json!({ "items": [{ "kind": "text", "frame": [0.0, 0.0, 100.0, 20.0], "paragraphs": 0 }] }));
}

#[test]
fn sample_of_a_group_lists_its_children_after_it_with_their_depth() {
    let group = ItemKind::Group { items: vec![opaque("a"), opaque("b")], smartart: None };
    let v = sample(&[item("g", Frame::new(0.0, 0.0, 50.0, 50.0), group)]);
    let kinds: Vec<_> =
        v["items"].as_array().unwrap().iter().map(|i| (i["kind"].clone(), i["depth"].clone())).collect();
    assert_that!(kinds).is_equal_to(vec![
        (json!("group"), json!(null)),
        (json!("opaque"), json!(1)),
        (json!("opaque"), json!(1)),
    ]);
}

#[test]
fn sample_beyond_the_cap_keeps_the_first_items_and_counts_the_rest() {
    let items: Vec<Item> = (0..MAX_SAMPLE_ITEMS + 5).map(|i| opaque(&format!("s1/{i}"))).collect();
    let v = sample(&items);
    assert_that!(v["items"].as_array().map(Vec::len)).is_equal_to(Some(MAX_SAMPLE_ITEMS));
    assert_that!(v["omitted"].as_u64()).is_equal_to(Some(5));
}
