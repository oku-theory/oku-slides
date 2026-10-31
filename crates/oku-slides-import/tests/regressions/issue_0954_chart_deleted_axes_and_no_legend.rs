//! Regression (t-954): a chart whose axes are deleted (`c:delete`), or absent, and that
//! has no `c:legend` was drawn with axes and a legend; PowerPoint and LibreOffice draw neither.

use std::collections::HashSet;

use oku_slides_core::{Deck, Namespace, apply_all, validate};
use oku_slides_import::{HifiTarget, PackLimits, hifi, read_pptx};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common;

/// The first chart option of `v`: the object that has `series` and `xAxis`.
fn chart_option(v: &Value) -> Option<&Value> {
    match v {
        Value::Object(o) if o.contains_key("series") && o.contains_key("xAxis") => Some(v),
        Value::Object(o) => o.values().find_map(chart_option),
        Value::Array(a) => a.iter().find_map(chart_option),
        _ => None,
    }
}

/// `kitchen` writes no axis; LibreOffice's round trip of it writes both as `c:delete val="1"`.
#[rstest]
#[case::kitchen(common::pptx("kitchen"))]
#[case::libreoffice(common::file("libreoffice.pptx"))]
fn hifi_chart_without_axes_or_legend_hides_them(#[case] bytes: Vec<u8>) {
    let source = read_pptx(&bytes, &common::limits()).unwrap_or_else(|e| panic!("lecture : {e}"));
    let (base, fonts) = (common::registry(), common::app_fonts());
    let target = HifiTarget {
        registry: &base,
        base: common::base_pin(),
        essentiel: common::essentiel_pin(),
        fonts: &fonts,
        pack_id: Uuid::from_u128(1),
        theme: Namespace::parse("neutre").unwrap_or_else(|| panic!("espace de noms")),
        name: "Import".to_owned(),
        created: "2026-10-11T08:00:00Z".to_owned(),
        limits: PackLimits::default(),
    };
    let plan = hifi::project(&source, &target).unwrap_or_else(|e| panic!("projection : {e}"));
    let registry = common::registry_with_deck_layouts(&plan.schemas);
    let mut deck = Deck::empty();
    apply_all(&mut deck, common::with_theme_closure(plan.ops.clone()), &registry).unwrap_or_else(|e| panic!("{e:?}"));
    let assets: HashSet<String> = plan.assets.keys().cloned().collect();
    assert_that!(validate(&deck, &registry, Some(&assets))).is_empty();

    let ops = serde_json::to_value(&plan.ops).unwrap_or_else(|e| panic!("{e}"));
    let option = chart_option(&ops).unwrap_or_else(|| panic!("no chart option in the plan"));
    let shown = |p: &str| option.pointer(p).cloned();
    assert_that!((shown("/xAxis/show"), shown("/yAxis/show"), shown("/legend/show"))).is_equal_to((
        Some(json!(false)),
        Some(json!(false)),
        Some(json!(false)),
    ));
}
