use speculoos::prelude::*;

use super::{CodePlace, MAX_WIDGET_TABS, PileSection, UiState};

fn tabs(widgets: &[&str], widget: Option<&str>) -> UiState {
    UiState {
        widgets: widgets.iter().map(|s| (*s).to_owned()).collect(),
        widget: widget.map(str::to_owned),
        ..UiState::default()
    }
}

#[test]
fn sanitized_outside_templates_clears_the_tabs() {
    let ui = tabs(&["kpi"], Some("kpi")).sanitized(false);
    assert_that!(ui.widgets).is_empty();
    assert_that!(ui.widget).is_none();
}

#[test]
fn sanitized_in_templates_drops_invalid_and_duplicate_names() {
    let ui = tabs(&["kpi", "../x", "Kpi", "kpi", "_commun"], Some("kpi")).sanitized(true);
    assert_that!(ui.widgets).is_equal_to(vec!["kpi".to_owned(), "_commun".to_owned()]);
}

#[test]
fn sanitized_in_templates_caps_the_list() {
    let names: Vec<String> = (0..MAX_WIDGET_TABS + 10).map(|i| format!("w{i}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    assert_that!(tabs(&refs, None).sanitized(true).widgets).has_length(MAX_WIDGET_TABS);
}

#[test]
fn sanitized_with_an_active_tab_not_open_drops_it() {
    assert_that!(tabs(&["kpi"], Some("quote")).sanitized(true).widget).is_none();
}

#[test]
fn code_place_serializes_in_lowercase() {
    assert_that!(serde_json::to_string(&CodePlace::Right).unwrap()).is_equal_to(r#""right""#.to_owned());
    assert_that!(serde_json::from_str::<CodePlace>(r#""below""#).unwrap()).is_equal_to(CodePlace::Below);
}

#[test]
fn ui_state_default_places_the_code_below_at_default_sizes() {
    let ui = UiState::default();
    assert_that!(ui.code_place).is_equal_to(CodePlace::Below);
    assert_that!(ui.code_height).is_none();
    assert_that!(ui.code_width).is_none();
}

#[test]
fn sanitized_drops_duplicate_folded_sections() {
    let ui = UiState {
        folded: vec![PileSection::Widgets, PileSection::Templates, PileSection::Widgets],
        ..UiState::default()
    };
    assert_that!(ui.sanitized(true).folded).is_equal_to(vec![PileSection::Widgets, PileSection::Templates]);
}

#[test]
fn sanitized_outside_templates_keeps_folded_sections() {
    let ui = UiState { folded: vec![PileSection::Templates], ..UiState::default() };
    assert_that!(ui.sanitized(false).folded).is_equal_to(vec![PileSection::Templates]);
}
