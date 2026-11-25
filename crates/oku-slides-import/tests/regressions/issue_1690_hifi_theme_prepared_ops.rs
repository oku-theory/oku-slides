//! t-1690: the hi-fi plan was prepared with its `set_theme`, which the core refuses without the
//! theme closure the session fills in; the refusal stopped the lifting of the charts' inline
//! data. The ops are now prepared theme-less, then `set_theme` gets its target theme back.

use oku_slides_core::{Deck, Namespace, Op, apply_all, brings_inline};
use oku_slides_import::{HifiTarget, PackLimits, hifi, read_pptx};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common;

#[test]
fn issue_1690_hifi_plan_lifts_chart_data_and_keeps_its_theme() {
    let source = read_pptx(&common::pptx("kitchen"), &common::limits()).unwrap_or_else(|e| panic!("lecture : {e}"));
    let (base, fonts) = (common::registry(), common::app_fonts());
    let theme = Namespace::parse("neutre").unwrap_or_else(|| panic!("espace de noms"));
    let target = HifiTarget {
        registry: &base,
        base: common::base_pin(),
        essentiel: common::essentiel_pin(),
        fonts: &fonts,
        pack_id: Uuid::from_u128(1),
        theme: theme.clone(),
        name: "Import".to_owned(),
        created: "2026-10-11T08:00:00Z".to_owned(),
        limits: PackLimits::default(),
        labels: oku_slides_chart::ChartLabels::english(),
    };
    let plan = hifi::project(&source, &target).unwrap_or_else(|e| panic!("projection : {e}"));
    let themes: Vec<Option<Namespace>> = plan
        .ops
        .iter()
        .filter_map(|op| if let Op::SetTheme { theme, .. } = op { Some(theme.clone()) } else { None })
        .collect();
    assert_that!(themes).is_equal_to(vec![Some(theme)]);
    let lifted = plan.ops.iter().filter(|op| matches!(op, Op::PutViewData { .. })).count();
    assert_that!(lifted).is_greater_than(0);
    let inline = serde_json::to_value(&plan.ops).unwrap_or_else(|e| panic!("{e}"));
    let options = inline.as_array().into_iter().flatten().filter_map(|op| op.pointer("/element/params/option"));
    assert_that!(options.filter(|o| brings_inline(o)).count()).is_equal_to(0);
    let registry = common::registry_with_deck_layouts(&plan.schemas);
    let mut deck = Deck::empty();
    apply_all(&mut deck, common::with_theme_closure(plan.ops.clone()), &registry).unwrap_or_else(|e| panic!("{e:?}"));
    assert_that!(deck.data.len()).is_equal_to(lifted);
}
