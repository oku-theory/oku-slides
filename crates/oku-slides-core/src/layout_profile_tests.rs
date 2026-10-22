use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::*;

#[test]
fn layout_profile_round_trips_with_hints_and_params() {
    let src = json!({"id": "colonnes", "name": "Colonnes", "layout": "content", "role": "content",
        "arrangement": "columns", "blocks": 3, "title": true, "kicker": true, "params": {"layout": "columns"}});
    let p: LayoutProfile = serde_json::from_value(src.clone()).unwrap();
    assert_that!(serde_json::to_value(&p).unwrap()).is_equal_to(src);
}

#[test]
fn layout_profile_minimal_defaults_hints() {
    let p: LayoutProfile = serde_json::from_value(
        json!({"id": "fin", "name": "Fin", "layout": "end", "role": "end", "arrangement": "default"}),
    )
    .unwrap();
    assert_that!((p.blocks, p.title, p.kicker, p.params.is_empty())).is_equal_to((0, false, false, true));
}

#[rstest]
#[case(json!("cover"))]
#[case(json!(1))]
fn layout_profile_unknown_role_is_refused(#[case] role: Value) {
    let src = json!({"id": "x", "name": "X", "layout": "end", "role": role, "arrangement": "default"});
    assert_that!(serde_json::from_value::<LayoutProfile>(src)).is_err();
}
