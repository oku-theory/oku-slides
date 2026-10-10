use std::collections::BTreeMap;

use oku_slides_core::{Op, Slide};
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::report::{ImportReport, Mode};
use crate::source::{Frame, Item, ItemKind, TextBox};

fn item() -> Item {
    Item {
        id: "s3/7".into(),
        name: "Cercle".into(),
        alt: None,
        frame: Frame::new(10.0, 20.0, 30.0, 40.0),
        z: 0,
        role: None,
        placeholder: None,
        kind: ItemKind::Text(TextBox::default()),
    }
}

#[test]
fn import_plan_serializes_ops_assets_by_length_and_requests() {
    let plan = ImportPlan {
        ops: vec![Op::AddSlide { index: None, defaults: false, slide: Slide::new("s1", "content") }],
        assets: BTreeMap::from([("sha256:ab".to_owned(), Asset { mime: "image/png".into(), bytes: vec![1, 2, 3] })]),
        widget_requests: vec![WidgetRequest {
            signature: "text×1|0000".into(),
            slides: vec!["s3".into(), "s5".into()],
            sample: vec![item()],
            fallback: "stack".into(),
        }],
        report: ImportReport::new(Mode::Retrofit),
    };
    let v = serde_json::to_value(&plan).unwrap();
    assert_that!(v["ops"][0]["op"]).is_equal_to(json!("add_slide"));
    assert_that!(v["assets"]["sha256:ab"]).is_equal_to(json!({ "mime": "image/png", "len": 3 }));
    assert_that!(v["widget_requests"][0]["slides"]).is_equal_to(json!(["s3", "s5"]));
    assert_that!(v["widget_requests"][0]["sample"][0]["id"]).is_equal_to(json!("s3/7"));
    assert_that!(v["report"]["mode"]).is_equal_to(json!("retrofit"));
}

#[test]
fn import_plan_report_json_adds_the_widget_requests_to_the_report() {
    let plan = ImportPlan {
        ops: Vec::new(),
        assets: BTreeMap::new(),
        widget_requests: vec![WidgetRequest {
            signature: "text×1|0044".into(),
            slides: vec!["s2".into()],
            sample: vec![item()],
            fallback: "stack".into(),
        }],
        report: ImportReport::new(Mode::Retrofit),
    };
    let v = plan.report_json(oku_slides_i18n::english());
    assert_that!(v["mode"]).is_equal_to(json!("retrofit"));
    assert_that!(v["widget_requests"][0]["slides"]).is_equal_to(json!(["s2"]));
    assert_that!(v["widget_requests"][0]["fallback"]).is_equal_to(json!("stack"));
}
