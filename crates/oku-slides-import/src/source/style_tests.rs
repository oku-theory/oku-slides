use serde_json::json;
use speculoos::prelude::*;

use super::*;

#[test]
fn rgba_opaque_serializes_as_six_digit_hex() {
    assert_that!(serde_json::to_value(Rgba::new(0x1f, 0x4e, 0x79)).unwrap()).is_equal_to(json!("#1f4e79"));
}

#[test]
fn rgba_translucent_serializes_as_eight_digit_hex() {
    let c = Rgba { r: 255, g: 0, b: 0, a: 128 };
    assert_that!(serde_json::to_value(c).unwrap()).is_equal_to(json!("#ff000080"));
}

#[test]
fn styled_scheme_color_serializes_name_and_slot_without_mods() {
    let c = Styled::scheme(Rgba::new(0x44, 0x72, 0xc4), "accent1");
    assert_that!(serde_json::to_value(&c).unwrap())
        .is_equal_to(json!({"value": "#4472c4", "origin": {"scheme": {"name": "accent1", "slot": "accent1"}}}));
}

#[test]
fn styled_mapped_color_serializes_written_name_slot_and_mods() {
    let c = Styled::mapped(Rgba::new(0xff, 0xff, 0xff), "tx1", "lt1", vec![ColorMod::LumMod(0.75)]);
    assert_that!(serde_json::to_value(&c).unwrap()).is_equal_to(json!({
        "value": "#ffffff",
        "origin": {"scheme": {"name": "tx1", "slot": "lt1", "mods": [{"op": "lum_mod", "value": 0.75}]}}
    }));
}

#[test]
fn styled_explicit_color_says_explicit() {
    assert_that!(serde_json::to_value(Styled::explicit(Rgba::new(0, 0, 0))).unwrap())
        .is_equal_to(json!({"value": "#000000", "origin": "explicit"}));
}

#[test]
fn origin_slot_reads_the_mapped_slot_only_for_scheme_values() {
    let o = Origin::Scheme { name: "bg1".into(), slot: "dk1".into(), mods: vec![] };
    assert_that!((o.slot(), Origin::Explicit.slot())).is_equal_to((Some("dk1"), None));
}
