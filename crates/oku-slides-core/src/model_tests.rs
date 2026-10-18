use speculoos::prelude::*;

use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::Namespace;

#[test]
fn from_json_version_1_deck_returns_an_error() {
    let err = Deck::from_json(r#"{"version": 1, "slides": [{"id": "s", "kind": "content", "blocks": []}]}"#);
    assert_that!(err.map_err(|e| e.to_string()).unwrap_err()).contains("unsupported deck version 1");
}

#[test]
fn from_json_newer_version_returns_an_error() {
    assert_that!(Deck::from_json(r#"{"version": 7, "slides": []}"#)).is_err();
}

const REF: &str = r#"{"id":"6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10","version":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#;

#[test]
fn deck_from_json_reads_widget_pins_by_namespace() {
    let json = json!({"version": 6, "widgets": {"essentiel": {"id": Uuid::nil(), "version": format!("sha256:{}", "b".repeat(64))}}, "slides": []});
    let deck = Deck::from_json(&json.to_string()).unwrap();
    assert_that!(deck.widgets.keys().map(Namespace::as_str).collect::<Vec<_>>()).is_equal_to(vec!["essentiel"]);
}

#[test]
fn deck_from_json_version_5_is_refused() {
    assert_that!(Deck::from_json(r#"{"version": 5, "slides": []}"#)).is_err();
}

#[test]
fn deck_from_json_reads_theme_as_a_theme_ref() {
    let deck = Deck::from_json(&format!(r#"{{"version":6,"theme":{REF},"slides":[]}}"#)).unwrap();
    let r = deck.theme.unwrap();
    assert_that!(r.id.to_string()).is_equal_to("6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10".to_owned());
    assert_that!(r.version.hex()).is_equal_to(&*"a".repeat(64));
}

#[test]
fn deck_from_json_with_a_theme_name_is_refused() {
    assert_that!(Deck::from_json(r#"{"version":6,"theme":"papier","slides":[]}"#).is_err()).is_true();
}

#[test]
fn deck_from_json_version_3_is_refused() {
    assert_that!(Deck::from_json(r#"{"version":3,"slides":[]}"#).is_err()).is_true();
}

#[test]
fn deck_from_json_version_2_is_refused() {
    assert_that!(Deck::from_json(r#"{"version":2,"slides":[]}"#).is_err()).is_true();
}

#[test]
fn deck_v4_is_refused_as_unsupported_version() {
    let err = Deck::from_json(r#"{"version":4,"slides":[]}"#).map_err(|e| e.to_string()).unwrap_err();
    assert_that!(err).contains("unsupported deck version 4");
}

#[test]
fn from_json_current_version_reads_the_deck() {
    let deck = Deck::from_json(r#"{"version": 6, "slides": [{"id": "s", "kind": "content"}]}"#).unwrap();
    assert_that!(deck.slides.len()).is_equal_to(1);
}

#[test]
fn from_json_without_version_reads_as_current() {
    assert_that!(Deck::from_json(r#"{"slides": []}"#).unwrap().version).is_equal_to(DECK_VERSION);
}

#[rstest::rstest]
#[case("title")]
#[case("subtitle")]
#[case("author")]
#[case("date")]
#[case("organization")]
#[case("lang")]
#[case("toc-title")]
fn deck_params_set_field_then_field_returns_the_text(#[case] key: &str) {
    let mut p = DeckParams::default();
    p.set_field(key, Some("valeur".into()));
    assert_that!(p.field(key)).is_equal_to(Some("valeur"));
    p.set_field(key, None);
    assert_that!(p.field(key)).is_none();
    assert_that!(p).is_equal_to(DeckParams::default());
}

#[test]
fn deck_params_field_of_non_string_extra_returns_none() {
    let mut p = DeckParams::default();
    p.extra.insert("numbering".into(), serde_json::json!(3));
    assert_that!(p.field("numbering")).is_none();
}

#[test]
fn deck_params_is_field_names_the_six_fields_only() {
    for key in ["title", "subtitle", "author", "date", "organization", "lang"] {
        assert_that!(DeckParams::is_field(key)).is_true();
    }
    assert_that!(DeckParams::is_field("numbering")).is_false();
}

#[test]
fn style_with_valign_and_inset_round_trips_through_json() {
    let json = serde_json::json!({ "valign": "middle", "inset": [7.2, 3.6, 0.0, 3.6] });
    let style: Style = serde_json::from_value(json.clone()).unwrap();
    assert_that!(style.valign.as_deref()).is_equal_to(Some("middle"));
    assert_that!(style.inset).is_equal_to(Some([7.2, 3.6, 0.0, 3.6]));
    assert_that!(serde_json::to_value(&style).unwrap()).is_equal_to(json);
}

#[test]
fn style_default_serializes_without_valign_nor_inset() {
    assert_that!(serde_json::to_value(Style::default()).unwrap()).is_equal_to(serde_json::json!({}));
}

#[test]
fn style_merge_takes_valign_and_inset_from_the_patch() {
    let mut style = Style { valign: Some("top".into()), color: Some("accent".into()), ..Default::default() };
    style.merge(Style { valign: Some("bottom".into()), inset: Some([1.0, 2.0, 3.0, 4.0]), ..Default::default() });
    assert_that!(style.valign.as_deref()).is_equal_to(Some("bottom"));
    assert_that!(style.inset).is_equal_to(Some([1.0, 2.0, 3.0, 4.0]));
    assert_that!(style.color.as_deref()).is_equal_to(Some("accent"));
}

#[rstest::rstest]
#[case::valign(Style { valign: Some("center".into()), ..Default::default() }, "style.valign", ValueReason::Valign)]
#[case::negative(Style { inset: Some([0.0, -1.0, 0.0, 0.0]), ..Default::default() }, "style.inset", ValueReason::OutOfBounds)]
#[case::nan(Style { inset: Some([f64::NAN, 0.0, 0.0, 0.0]), ..Default::default() }, "style.inset", ValueReason::OutOfBounds)]
#[case::infinite(Style { inset: Some([0.0, 0.0, f64::INFINITY, 0.0]), ..Default::default() }, "style.inset", ValueReason::OutOfBounds)]
#[case::too_large(Style { inset: Some([0.0, 0.0, 0.0, STYLE_INSET_MAX + 1.0]), ..Default::default() }, "style.inset", ValueReason::OutOfBounds)]
fn style_invalid_value_returns_its_field_and_reason(
    #[case] style: Style,
    #[case] field: &str,
    #[case] reason: ValueReason,
) {
    assert_that!(style.invalid()).is_equal_to(Some((field, reason)));
}

#[test]
fn style_invalid_with_known_valign_and_bounded_inset_returns_none() {
    for valign in ["top", "middle", "bottom"] {
        let style =
            Style { valign: Some(valign.into()), inset: Some([0.0, STYLE_INSET_MAX, 1.5, 0.0]), ..Default::default() };
        assert_that!(style.invalid()).named(valign).is_none();
    }
    assert_that!(Style::default().invalid()).is_none();
}

#[test]
fn style_with_tracking_round_trips_through_json() {
    let json = serde_json::json!({ "tracking": -2.5 });
    let style: Style = serde_json::from_value(json.clone()).unwrap();
    assert_that!(style.tracking).is_equal_to(Some(-2.5));
    assert_that!(serde_json::to_value(&style).unwrap()).is_equal_to(json);
}

#[test]
fn style_merge_takes_tracking_from_the_patch() {
    let mut style = Style { tracking: Some(1.0), color: Some("accent".into()), ..Default::default() };
    style.merge(Style { size: Some(TokenValue::Number(20.0)), ..Default::default() });
    assert_that!(style.tracking).is_equal_to(Some(1.0));
    style.merge(Style { tracking: Some(-3.0), ..Default::default() });
    assert_that!(style.tracking).is_equal_to(Some(-3.0));
}

#[rstest::rstest]
#[case::nan(f64::NAN)]
#[case::infinite(f64::NEG_INFINITY)]
#[case::too_wide(STYLE_TRACKING_MAX + 1.0)]
#[case::too_tight(-STYLE_TRACKING_MAX - 1.0)]
fn style_invalid_tracking_out_of_bounds_returns_its_field(#[case] tracking: f64) {
    let style = Style { tracking: Some(tracking), ..Default::default() };
    assert_that!(style.invalid()).is_equal_to(Some(("style.tracking", ValueReason::OutOfBounds)));
}

#[test]
fn style_invalid_with_bounded_tracking_returns_none() {
    for tracking in [-STYLE_TRACKING_MAX, -1.0, 0.0, 2.5, STYLE_TRACKING_MAX] {
        let style = Style { tracking: Some(tracking), ..Default::default() };
        assert_that!(style.invalid()).named(&tracking.to_string()).is_none();
    }
}

#[test]
fn style_with_spacing_round_trips_through_json() {
    let json = serde_json::json!({ "spacing": 12.5 });
    let style: Style = serde_json::from_value(json.clone()).unwrap();
    assert_that!(style.spacing).is_equal_to(Some(12.5));
    assert_that!(serde_json::to_value(&style).unwrap()).is_equal_to(json);
}

#[test]
fn style_merge_takes_spacing_from_the_patch() {
    let mut style = Style { spacing: Some(6.0), color: Some("accent".into()), ..Default::default() };
    style.merge(Style { size: Some(TokenValue::Number(20.0)), ..Default::default() });
    assert_that!(style.spacing).is_equal_to(Some(6.0));
    style.merge(Style { spacing: Some(0.0), ..Default::default() });
    assert_that!(style.spacing).is_equal_to(Some(0.0));
}

#[rstest::rstest]
#[case::nan(f64::NAN)]
#[case::infinite(f64::INFINITY)]
#[case::negative(-1.0)]
#[case::too_wide(STYLE_SPACING_MAX + 1.0)]
fn style_invalid_spacing_out_of_bounds_returns_its_field(#[case] spacing: f64) {
    let style = Style { spacing: Some(spacing), ..Default::default() };
    assert_that!(style.invalid()).is_equal_to(Some(("style.spacing", ValueReason::OutOfBounds)));
}

#[test]
fn style_with_indent_round_trips_through_json() {
    let json = serde_json::json!({ "indent": [27.0, -27.0] });
    let style: Style = serde_json::from_value(json.clone()).unwrap();
    assert_that!(style.indent).is_equal_to(Some([27.0, -27.0]));
    assert_that!(serde_json::to_value(&style).unwrap()).is_equal_to(json);
}

#[test]
fn style_merge_takes_indent_from_the_patch() {
    let mut style = Style { indent: Some([10.0, 0.0]), ..Default::default() };
    style.merge(Style { size: Some(TokenValue::Number(20.0)), ..Default::default() });
    assert_that!(style.indent).is_equal_to(Some([10.0, 0.0]));
    style.merge(Style { indent: Some([0.0, 5.0]), ..Default::default() });
    assert_that!(style.indent).is_equal_to(Some([0.0, 5.0]));
}

#[rstest::rstest]
#[case::nan_margin([f64::NAN, 0.0])]
#[case::negative_margin([-1.0, 0.0])]
#[case::too_wide_margin([STYLE_INDENT_MAX + 1.0, 0.0])]
#[case::infinite_first([0.0, f64::NEG_INFINITY])]
#[case::too_deep_first([0.0, -STYLE_INDENT_MAX - 1.0])]
fn style_invalid_indent_out_of_bounds_returns_its_field(#[case] indent: [f64; 2]) {
    let style = Style { indent: Some(indent), ..Default::default() };
    assert_that!(style.invalid()).is_equal_to(Some(("style.indent", ValueReason::OutOfBounds)));
}

#[test]
fn style_invalid_with_bounded_indent_returns_none() {
    for indent in [[0.0, 0.0], [27.0, -27.0], [STYLE_INDENT_MAX, -STYLE_INDENT_MAX], [0.0, STYLE_INDENT_MAX]] {
        let style = Style { indent: Some(indent), ..Default::default() };
        assert_that!(style.invalid()).named(&format!("{indent:?}")).is_none();
    }
}

#[test]
fn style_invalid_with_bounded_spacing_returns_none() {
    for spacing in [0.0, 2.5, STYLE_SPACING_MAX] {
        let style = Style { spacing: Some(spacing), ..Default::default() };
        assert_that!(style.invalid()).named(&spacing.to_string()).is_none();
    }
}
