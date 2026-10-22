use std::collections::BTreeMap;

use oku_slides_core::kinds::base;
use oku_slides_core::{Op, PackFiles, Slide, WidgetFile};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::report::{ImportReport, Mode};
use crate::source::{Frame, Item, ItemKind, TextBox};
use crate::{ImportErrorKind, ReadLimits};

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
        ops: vec![Op::AddSlide {
            index: None,
            defaults: false,
            slide: Slide::new("s1", oku_slides_core::kinds::essentiel::CONTENT),
        }],
        assets: BTreeMap::from([("sha256:ab".to_owned(), Asset { mime: "image/png".into(), bytes: vec![1, 2, 3] })]),
        widget_requests: vec![WidgetRequest {
            signature: "text×1|0000".into(),
            slides: vec!["s3".into(), "s5".into()],
            sample: vec![item()],
            fallback: base::STACK.into(),
        }],
        ..ImportPlan::new(ImportReport::new(Mode::Retrofit))
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
        widget_requests: vec![WidgetRequest {
            signature: "text×1|0044".into(),
            slides: vec!["s2".into()],
            sample: vec![item()],
            fallback: base::STACK.into(),
        }],
        ..ImportPlan::new(ImportReport::new(Mode::Retrofit))
    };
    let v = plan.report_json(oku_slides_i18n::english());
    assert_that!(v["mode"]).is_equal_to(json!("retrofit"));
    assert_that!(v["widget_requests"][0]["slides"]).is_equal_to(json!(["s2"]));
    assert_that!(v["widget_requests"][0]["fallback"]).is_equal_to(json!(base::STACK));
}

/// Un plan dont une seule partie de la sortie est non vide, et la taille de cette sortie.
fn plan_with(part: &str) -> (ImportPlan, u64) {
    let mut plan = ImportPlan::new(ImportReport::new(Mode::Hifi));
    let files = || {
        let manifest = json!({ "id": "p", "name": "Généré" });
        let widgets = BTreeMap::from([(WidgetFile::common(), "#let x = 1".to_owned())]);
        PackFiles::new(oku_slides_core::PackKind::Layouts, manifest.as_object().cloned().unwrap(), widgets)
    };
    match part {
        "assets" => {
            plan.assets.insert("sha256:a".into(), Asset { mime: "image/png".into(), bytes: vec![0; 10] });
        }
        "fonts" => {
            plan.fonts.insert("a.ttf".into(), vec![0; 12]);
        }
        "ops" => {
            plan.ops = vec![Op::AddSlide {
                index: None,
                defaults: false,
                slide: Slide::new("s1", oku_slides_core::kinds::essentiel::CONTENT),
            }]
        }
        "theme" => plan.theme = Some(files()),
        "pack" => plan.pack = Some(files()),
        _ => unreachable!("partie inconnue"),
    }
    // Les ops se sérialisent toujours, même vides (`[]`).
    let size = serde_json::to_vec(&plan.ops).unwrap().len()
        + match part {
            "assets" => 10,
            "fonts" => 12,
            "ops" => 0,
            _ => serde_json::to_vec(files().manifest()).unwrap().len() + "#let x = 1".len(),
        };
    (plan, size as u64)
}

fn limits(max_output_bytes: u64) -> ReadLimits {
    ReadLimits { max_output_bytes, ..ReadLimits::default() }
}

#[rstest]
#[case("assets")]
#[case("fonts")]
#[case("ops")]
#[case("theme")]
#[case("pack")]
fn check_budget_output_at_limit_is_ok(#[case] part: &str) {
    let (plan, size) = plan_with(part);
    assert_that!(plan.check_budget(&limits(size))).is_ok();
}

#[rstest]
#[case("assets")]
#[case("fonts")]
#[case("ops")]
#[case("theme")]
#[case("pack")]
fn check_budget_output_one_byte_over_is_too_large(#[case] part: &str) {
    let (plan, size) = plan_with(part);
    let err = plan.check_budget(&limits(size - 1)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn check_budget_parts_sum_together_is_too_large() {
    let (mut plan, size) = plan_with("assets");
    plan.fonts.insert("a.ttf".into(), vec![0; 12]);
    assert_that!(plan.clone().check_budget(&limits(size + 12))).is_ok();
    assert_that!(plan.check_budget(&limits(size + 11)).map_err(|e| e.kind()))
        .is_equal_to(Err(ImportErrorKind::TooLarge));
}

fn replica(max_replica_bytes: u64) -> ReadLimits {
    ReadLimits { max_replica_bytes, ..ReadLimits::default() }
}

#[test]
fn check_budget_ops_at_the_replica_limit_is_ok() {
    let (plan, size) = plan_with("ops");
    assert_that!(plan.check_budget(&replica(size))).is_ok();
}

#[test]
fn check_budget_ops_one_byte_over_the_replica_limit_is_too_large() {
    let (plan, size) = plan_with("ops");
    let err = plan.check_budget(&replica(size - 1)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn check_budget_assets_do_not_count_against_the_replica() {
    let (plan, _) = plan_with("assets");
    assert_that!(plan.check_budget(&replica(2))).is_ok();
}

#[test]
fn checked_plan_reads_as_its_plan_and_gives_up_its_ops() {
    let (plan, size) = plan_with("ops");
    let ops = plan.ops.clone();
    let mut checked = plan.check_budget(&limits(size)).unwrap();
    assert_that!(checked.take_ops()).is_equal_to(ops);
    assert_that!(checked.ops.is_empty()).is_true();
}

#[test]
fn checked_plan_gives_up_its_generated_theme_pack_and_fonts() {
    let (mut plan, _) = plan_with("pack");
    plan.fonts.insert("a.ttf".into(), vec![0; 3]);
    let mut checked = plan.check_budget(&ReadLimits::default()).unwrap();
    let taken = (checked.take_theme().is_some(), checked.take_pack().is_some(), checked.take_fonts().len());
    assert_that!(taken).is_equal_to((false, true, 1));
    assert_that!((checked.pack.is_none(), checked.fonts.is_empty())).is_equal_to((true, true));
}

#[test]
fn budget_take_json_that_cannot_serialize_does_not_fit() {
    // Une clé non textuelle : erreur de sérialisation qui n'est pas une erreur d'écriture.
    let v = BTreeMap::from([(vec![1u8], 1u8)]);
    let mut budget = Budget { left: 1 << 20 };
    assert_that!(budget.take_json(&v)).is_false();
}
