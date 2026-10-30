use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::{Paragraph, Run};

#[rstest]
#[case(1)]
#[case(6)]
fn heading_level_new_in_range_is_some(#[case] n: u8) {
    assert_that!(HeadingLevel::new(n).map(HeadingLevel::get)).is_equal_to(Some(n));
}

#[rstest]
#[case(0)]
#[case(7)]
fn heading_level_try_from_out_of_range_is_heading_level_error(#[case] n: u8) {
    let err: Result<HeadingLevel, HeadingLevelError> = HeadingLevel::try_from(n);
    assert_that!(err.is_err()).is_true();
}

#[test]
fn paragraph_with_heading_round_trips_as_a_number() {
    let p = Paragraph { runs: vec![Run::plain("Plan")], list: None, heading: HeadingLevel::new(2) };
    let v = serde_json::to_value(&p).unwrap();
    assert_that!(v).is_equal_to(json!({ "runs": [{ "t": "Plan" }], "heading": 2 }));
    assert_that!(serde_json::from_value::<Paragraph>(v).unwrap()).is_equal_to(p);
}

#[test]
fn paragraph_without_heading_has_no_heading_key() {
    let v = serde_json::to_value(Paragraph::plain("a")).unwrap();
    assert_that!(v.get("heading")).is_none();
}

#[rstest]
#[case(json!(0))]
#[case(json!(7))]
#[case(json!("2"))]
fn paragraph_with_invalid_heading_is_refused(#[case] heading: serde_json::Value) {
    let r = serde_json::from_value::<Paragraph>(json!({ "runs": [], "heading": heading }));
    assert_that!(r.is_err()).is_true();
}
