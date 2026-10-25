//! t-1204: a `show_params` stepper with a single bound passed validation, then showed an inverted
//! range, because the UI falls back on 1 and 24 for the missing bound.

use oku_slides_i18n::{FluentValue, Localize, Localizer};
use oku_slides_session::SessionErrorKind;
use oku_slides_session::agent::{AgentFormReason, FieldKind, ParamField, ShowParams};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{fixture_path, session};

fn stepper(min: Option<f64>, max: Option<f64>) -> ShowParams {
    let field = ParamField {
        label: "Colonnes".into(),
        kind: FieldKind::Stepper,
        value: json!(4),
        op_template: json!({ "op": "set_var", "key": "theme", "value": "$value" }),
        min,
        max,
        step: None,
        options: vec![],
        columns: Vec::new(),
        item_label: None,
    };
    ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![field] }
}

#[rstest]
#[case::min_above_default_max(Some(30.0), None, 30.0, 24.0)]
#[case::min_equal_to_default_max(Some(24.0), None, 24.0, 24.0)]
#[case::max_below_default_min(None, Some(0.0), 1.0, 0.0)]
#[case::bounds_equal_once_rounded(Some(2.6), Some(3.4), 3.0, 3.0)]
fn stepper_effective_bounds_inverted_is_refused(
    #[case] min: Option<f64>,
    #[case] max: Option<f64>,
    #[case] effective_min: f64,
    #[case] effective_max: f64,
) {
    let mut s = session();
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("{e}"));
    let err = s.params_surface("show-1", &stepper(min, max)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    let Some(AgentFormReason::BoundsInverted { at, min, max }) = err.agent_form_reason() else {
        panic!("expected BoundsInverted, got {:?}", err.agent_form_reason());
    };
    assert_that!((at.as_str(), *min, *max)).is_equal_to(("fields[0]", effective_min, effective_max));
}

#[rstest]
#[case::both_saturate_to_i64_max(Some(1e19), Some(2e19))]
#[case::max_beyond_js_safe_integers(Some(1.0), Some(9_007_199_254_740_992.0))]
#[case::min_beyond_js_safe_integers(Some(-9_007_199_254_740_992.0), None)]
fn stepper_bound_beyond_js_safe_integers_is_refused(#[case] min: Option<f64>, #[case] max: Option<f64>) {
    let mut s = session();
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("{e}"));
    let err = s.params_surface("show-1", &stepper(min, max)).unwrap_err();
    let Some(AgentFormReason::NotSafeInteger { at }) = err.agent_form_reason() else {
        panic!("expected NotSafeInteger, got {:?}", err.agent_form_reason());
    };
    assert_that!(at.as_str()).is_equal_to("fields[0]");
}

#[test]
fn stepper_min_above_default_max_is_refused_in_the_user_language() {
    let mut s = session();
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("{e}"));
    let err = s.params_surface("show-1", &stepper(Some(30.0), None)).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    let reason = fr.tr_with(
        "session-agent-form-bounds-inverted",
        &[("at", "fields[0]".into()), ("min", FluentValue::from(30.0)), ("max", FluentValue::from(24.0))],
    );
    assert_that!(err.localize(&fr)).is_equal_to(fr.tr_with("session-agent-form", &[("reason", reason.into())]));
}

#[rstest]
#[case::no_bound(None, None)]
#[case::min_within_defaults(Some(20.0), None)]
#[case::max_within_defaults(None, Some(2.0))]
#[case::both_bounds(Some(30.0), Some(40.0))]
fn stepper_effective_bounds_in_order_is_accepted(#[case] min: Option<f64>, #[case] max: Option<f64>) {
    let mut s = session();
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(s.params_surface("show-1", &stepper(min, max))).is_ok();
}
