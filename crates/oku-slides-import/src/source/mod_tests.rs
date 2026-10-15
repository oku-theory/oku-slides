use serde_json::json;
use speculoos::prelude::*;

use super::*;

fn text_item() -> Item {
    Item {
        id: "s1/2".into(),
        name: "Titre 1".into(),
        alt: None,
        frame: Frame::new(10.0, 20.0, 300.0, 40.0),
        z: 0,
        role: Some(Role::Title),
        placeholder: None,
        kind: ItemKind::Text(TextBox::default()),
    }
}

#[test]
fn item_text_is_the_box_of_a_text_or_a_shape_only() {
    let shape = |text| Shape { geom: Geometry::Path { paths: Vec::new() }, fill: Fill::None, stroke: None, text };
    let mut item = text_item();
    assert_that!(item.text()).is_equal_to(Some(&TextBox::default()));
    item.kind = ItemKind::Shape(Box::new(shape(Some(TextBox::default()))));
    assert_that!(item.text()).is_equal_to(Some(&TextBox::default()));
    item.kind = ItemKind::Shape(Box::new(shape(None)));
    assert_that!(item.text()).is_none();
    item.kind = ItemKind::Group { items: Vec::new(), smartart: false };
    assert_that!(item.text()).is_none();
}

#[test]
fn item_text_serializes_with_kind_tag_and_role() {
    let v = serde_json::to_value(text_item()).unwrap();
    assert_that!((v["kind"]["type"].clone(), v["role"].clone())).is_equal_to((json!("text"), json!("title")));
}

#[test]
fn frame_serializes_position_size_rotation_and_flips() {
    let v = serde_json::to_value(text_item()).unwrap();
    assert_that!(v["frame"]).is_equal_to(
        json!({"x": 10.0, "y": 20.0, "w": 300.0, "h": 40.0, "rot": 0.0, "flip_h": false, "flip_v": false}),
    );
}

#[test]
fn default_textbox_has_one_wrapping_column() {
    let tb = TextBox::default();
    assert_that!((tb.columns, tb.wrap)).is_equal_to((1, true));
}

#[test]
fn asset_serializes_its_length_not_its_bytes() {
    let a = Asset { mime: "image/png".into(), bytes: vec![1, 2, 3] };
    assert_that!(serde_json::to_value(a).unwrap()).is_equal_to(json!({"mime": "image/png", "len": 3}));
}

#[test]
fn fill_none_is_none_and_solid_is_not() {
    assert_that!(Fill::None.is_none()).is_true();
    assert_that!(Fill::Solid { color: Styled::explicit(Rgba::new(1, 2, 3)) }.is_none()).is_false();
}

#[rstest::rstest]
#[case("title", LayoutKind::Title)]
#[case("secHead", LayoutKind::SectionHeader)]
#[case("obj", LayoutKind::TitleAndContent)]
#[case("twoObj", LayoutKind::TwoContent)]
#[case("titleOnly", LayoutKind::TitleOnly)]
#[case("blank", LayoutKind::Blank)]
#[case("cust", LayoutKind::Custom)]
#[case("picTx", LayoutKind::Other("picTx".to_owned()))]
fn layout_kind_reads_and_names_its_ooxml_type(#[case] name: &str, #[case] kind: LayoutKind) {
    assert_that!(LayoutKind::from_ooxml(name)).is_equal_to(&kind);
    assert_that!(serde_json::to_value(&kind).unwrap()).is_equal_to(json!(name));
}
