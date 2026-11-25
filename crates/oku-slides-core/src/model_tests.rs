use rstest::rstest;
use speculoos::prelude::*;

use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::Namespace;

#[test]
fn from_json_version_1_deck_returns_an_error() {
    let err = Deck::from_json(r#"{"version": 1, "slides": [{"id": "s", "kind": "essentiel/content", "blocks": []}]}"#);
    assert_that!(err.map_err(|e| e.to_string()).unwrap_err()).contains("unsupported deck version 1");
}

#[test]
fn from_json_newer_version_returns_an_error() {
    assert_that!(Deck::from_json(&format!(r#"{{"version": {}, "slides": []}}"#, DECK_VERSION + 1))).is_err();
}

#[rstest]
#[case(1)]
#[case(5)]
#[case(6)]
#[case(7)]
#[case(8)]
#[case(9)]
fn deck_from_json_older_version_is_refused(#[case] version: u32) {
    let err = Deck::from_json(&format!(r#"{{"version": {version}, "slides": []}}"#)).unwrap_err();
    assert_that!(err.to_string()).contains(format!("unsupported deck version {version}"));
}

#[test]
fn deck_from_json_current_version_is_read() {
    assert_that!(Deck::from_json(r#"{"version": 10, "slides": []}"#).ok().map(|d| d.version)).is_equal_to(Some(10));
}

#[test]
fn deck_empty_is_version_10() {
    assert_that!(Deck::empty().version).is_equal_to(10);
}

/// The JSON of a deck whose data are `data` (an object of id to data JSON).
fn deck_with_data(data: &serde_json::Value) -> String {
    json!({ "version": DECK_VERSION, "slides": [], "data": data }).to_string()
}

/// The JSON of an empty manual data named `name`.
fn data_json(name: &str) -> serde_json::Value {
    json!({ "name": name, "kind": "manual", "table": { "generation": "g", "columns": [], "rows": [] } })
}

#[test]
fn deck_round_trips_data_json() {
    let data = json!({ "sales": {
        "name": "sales", "label": "Sales", "kind": "manual",
        "table": {
            "generation": "g1",
            "columns": [{ "id": "c", "name": "Amount", "type": "number" }],
            "rows": ["r1", "r2"],
            "cells": { "r1/c": 3.5, "r2/c": 4.0 }
        }
    }});
    let deck = Deck::from_json(&deck_with_data(&data)).unwrap();
    let back = Deck::from_json(&deck.to_json().unwrap()).unwrap();
    assert_that!(deck.data.len()).is_equal_to(1);
    assert_that!(back).is_equal_to(deck);
}

#[test]
fn deck_v9_json_is_refused() {
    let err = Deck::from_json(r#"{"version": 9, "slides": []}"#).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::DeckJsonErrorKind::UnsupportedVersion);
}

#[test]
fn deck_json_over_64_mib_is_refused_before_parsing() {
    // Not JSON at all: a parse would say unreadable, the byte cap says too large.
    let input = "x".repeat(oku_slides_data::limits::MAX_DECK_JSON_BYTES + 1);
    let err = Deck::from_json(&input).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::DeckJsonErrorKind::TooLarge);
}

#[rstest]
#[case::at_bound(oku_slides_data::limits::MAX_DATA, true)]
#[case::past_bound(oku_slides_data::limits::MAX_DATA + 1, false)]
fn deck_json_with_257_data_fails_while_parsing(#[case] count: usize, #[case] parses: bool) {
    let data: serde_json::Map<String, serde_json::Value> =
        (0..count).map(|i| (format!("d{i}"), data_json(&format!("d{i}")))).collect();
    let read = Deck::from_json(&deck_with_data(&serde_json::Value::Object(data)));
    assert_that!(read.as_ref().map(|d| d.data.len()).ok()).is_equal_to(parses.then_some(count));
    if let Err(err) = read {
        assert_that!(err.kind()).is_equal_to(crate::DeckJsonErrorKind::Unreadable);
    }
}

#[test]
fn deck_json_over_bounds_is_refused_fr() {
    let max = oku_slides_data::limits::MAX_DECK_TEXT_BYTES;
    let mut data = data_json("big");
    data["label"] = json!("x".repeat(max));
    let err = Deck::from_json(&deck_with_data(&json!({ "big": data }))).unwrap_err();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(err.kind()).is_equal_to(crate::DeckJsonErrorKind::DeckFull);
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to(fr.tr_with("op-data-deck-full", &[("what", "text".into()), ("max", max.into())]));
}

#[test]
fn deck_json_with_stored_dataset_is_refused_fr() {
    let deck = json!({ "version": DECK_VERSION, "slides": [{ "id": "s", "kind": "essentiel/content", "params": {
        "blocks": [{ "id": "g", "kind": "base/chart", "params": { "option": {
            "dataset": { "source": [["A", "B"], ["x", 1]] }, "series": [{ "type": "bar" }]
        } } }]
    } }] });
    let err = Deck::from_json(&deck.to_string()).unwrap_err();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    assert_that!(err.kind()).is_equal_to(crate::DeckJsonErrorKind::InlineData);
    assert_that!(oku_slides_i18n::Localize::localize(&err, &fr))
        .is_equal_to(fr.tr_with("deck-inline-data", &[("element", "g".into())]));
}

/// A deck whose element `g` holds `option` in its param `option`.
fn deck_with_option(option: &serde_json::Value) -> String {
    json!({ "version": DECK_VERSION, "slides": [{ "id": "s", "kind": "essentiel/content", "params": {
        "blocks": [{ "id": "g", "kind": "base/chart", "params": { "option": option } }]
    } }] })
    .to_string()
}

#[test]
fn deck_json_with_inline_series_data_is_refused() {
    let option = json!({ "xAxis": { "type": "category", "data": ["a"] }, "series": [{ "type": "bar", "data": [1] }] });
    let err = Deck::from_json(&deck_with_option(&option)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::DeckJsonErrorKind::InlineData);
}

#[test]
fn deck_json_with_dataset_key_outside_a_view_option_is_read() {
    let value = json!({ "dataset": "a name", "caption": "not a chart or table option" });
    assert_that!(Deck::from_json(&deck_with_option(&value)).map(|d| d.slides.len())).is_ok_containing(1);
}

#[test]
fn deck_json_with_over_bound_table_fails_while_parsing() {
    let columns: Vec<serde_json::Value> = (0..=oku_slides_data::limits::MAX_COLUMNS)
        .map(|i| json!({ "id": format!("c{i}"), "name": "", "type": "text" }))
        .collect();
    let mut data = data_json("wide");
    data["table"]["columns"] = json!(columns);
    let err = Deck::from_json(&deck_with_data(&json!({ "wide": data }))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::DeckJsonErrorKind::Unreadable);
}

/// Un épinglage quelconque, pour les tests de forme.
fn pin_for_tests() -> PackPin {
    let version = crate::PackVersion::parse(&format!("sha256:{}", "c".repeat(64))).unwrap();
    PackPin { id: Uuid::nil(), version, requires: vec![crate::kinds::base::namespace()] }
}

#[test]
fn deck_layouts_round_trip_and_are_omitted_when_empty() {
    let mut deck = Deck::empty();
    let empty = serde_json::to_value(&deck).unwrap();
    deck.layouts.insert(crate::kinds::essentiel::namespace(), pin_for_tests());
    let back = Deck::from_json(&serde_json::to_string(&deck).unwrap()).unwrap();
    assert_that!((empty.get("layouts").is_none(), back.layouts == deck.layouts)).is_equal_to((true, true));
}

#[test]
fn deck_without_knowledge_serializes_without_key() {
    let deck = Deck::empty();
    assert_that!(deck.knowledge.is_empty()).is_true();
    let value = serde_json::to_value(&deck).unwrap();
    assert_that!(value.get("knowledge").is_none()).is_true();
    assert_that!(Deck::from_json(&value.to_string()).unwrap()).is_equal_to(deck);
}

#[test]
fn deck_knowledge_round_trips() {
    let mut deck = Deck::empty();
    let id = crate::knowledge_file_id();
    let file = KnowledgeFile { path: "/a/b.md".to_owned(), text: "---\r\ntype: x\r\n---\n body \n".to_owned() };
    deck.knowledge.insert(id.clone(), file.clone());
    let json = serde_json::to_string(&deck).unwrap();
    let back = Deck::from_json(&json).unwrap();
    assert_that!(back.knowledge.get(&id)).is_equal_to(Some(&file));
    assert_that!(back).is_equal_to(deck);
}

const REF: &str = r#"{"id":"6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10","version":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#;

#[test]
fn deck_from_json_reads_widget_pins_by_namespace() {
    let json = json!({"version": 10, "widgets": {"essentiel": {"id": Uuid::nil(), "version": format!("sha256:{}", "b".repeat(64))}}, "slides": []});
    let deck = Deck::from_json(&json.to_string()).unwrap();
    assert_that!(deck.widgets.keys().map(Namespace::as_str).collect::<Vec<_>>()).is_equal_to(vec!["essentiel"]);
}

const PIN: &str = r#"{"id":"6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10","version":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","extends":["papier"]}"#;

#[test]
fn deck_from_json_reads_theme_as_a_namespace_and_its_pins() {
    let json = format!(r#"{{"version":10,"theme":"marque","themes":{{"marque":{PIN},"papier":{REF}}},"slides":[]}}"#);
    let deck = Deck::from_json(&json).unwrap();
    assert_that!(deck.theme.as_ref().map(Namespace::as_str)).is_equal_to(Some("marque"));
    let marque = &deck.themes[&Namespace::parse("marque").unwrap()];
    assert_that!(marque.extends.iter().map(Namespace::as_str).collect::<Vec<_>>()).is_equal_to(vec!["papier"]);
    assert_that!(deck.themes[&Namespace::parse("papier").unwrap()].version.hex()).is_equal_to(&*"a".repeat(64));
}

#[test]
fn deck_from_json_with_a_theme_ref_object_is_refused() {
    assert_that!(Deck::from_json(&format!(r#"{{"version":10,"theme":{REF},"slides":[]}}"#)).is_err()).is_true();
}

#[test]
fn deck_without_theme_omits_theme_and_themes() {
    let json = serde_json::to_value(Deck::empty()).unwrap();
    assert_that!((json.get("theme"), json.get("themes"))).is_equal_to((None, None));
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
    let deck = Deck::from_json(r#"{"version": 10, "slides": [{"id": "s", "kind": "essentiel/content"}]}"#).unwrap();
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

#[test]
fn all_pins_lists_widgets_then_layouts_with_their_kind() {
    let mut deck = Deck::empty();
    deck.pins_mut(PackKind::Layouts).insert(Namespace::parse("alpha").unwrap(), pin_for_tests());
    deck.pins_mut(PackKind::Widgets).insert(Namespace::parse("zeta").unwrap(), pin_for_tests());
    let listed: Vec<(PackKind, String)> = deck.all_pins().map(|(k, ns, _)| (k, ns.to_string())).collect();
    assert_that!(listed)
        .is_equal_to(vec![(PackKind::Widgets, "zeta".to_owned()), (PackKind::Layouts, "alpha".to_owned())]);
}

#[test]
fn deck_without_knowledge_keeps_everything_else() {
    let mut deck = Deck::from_json(include_str!("../../../fixtures/roadmap.json")).unwrap();
    let file = crate::KnowledgeFile { path: "/a.md".to_owned(), text: "body".to_owned() };
    deck.knowledge.insert(format!("{:032x}", 1), file);
    let stripped = deck.without_knowledge();
    assert_that!(stripped.knowledge.is_empty()).is_true();
    let mut expected = deck.clone();
    expected.knowledge.clear();
    assert_that!(stripped).is_equal_to(expected);
    assert_that!(deck.knowledge.len()).is_equal_to(1);
}
