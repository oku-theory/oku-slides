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

fn cases(pairs: &[(&str, &str)]) -> UiState {
    UiState { case: pairs.iter().map(|(w, c)| ((*w).to_owned(), (*c).to_owned())).collect(), ..UiState::default() }
}

#[test]
fn ui_state_default_has_no_current_case() {
    assert_that!(UiState::default().case.is_empty()).is_true();
}

#[test]
fn sanitized_in_templates_keeps_the_current_cases() {
    let ui = cases(&[("kpi", "Long"), ("quote", "Exemple")]).sanitized(true);
    assert_that!(ui.case.get("kpi").map(String::as_str)).is_equal_to(Some("Long"));
    assert_that!(ui.case.len()).is_equal_to(2);
}

#[test]
fn sanitized_outside_templates_clears_the_current_cases() {
    assert_that!(cases(&[("kpi", "Long")]).sanitized(false).case.is_empty()).is_true();
}

#[test]
fn sanitized_drops_cases_of_invalid_widgets_or_with_bad_names() {
    let long = "x".repeat(41);
    let ui = cases(&[("../x", "Long"), ("_commun", "Long"), ("kpi", ""), ("quote", &long), ("carte", "Court")])
        .sanitized(true);
    assert_that!(ui.case.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec!["carte".to_owned()]);
}

#[test]
fn sanitized_caps_the_current_cases() {
    let names: Vec<String> = (0..MAX_WIDGET_TABS + 10).map(|i| format!("w{i}")).collect();
    let pairs: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "Long")).collect();
    assert_that!(cases(&pairs).sanitized(true).case.len()).is_equal_to(MAX_WIDGET_TABS);
}

#[test]
fn ui_state_without_case_reads_as_none() {
    let ui: UiState = serde_json::from_str(r#"{"slide":null}"#).unwrap();
    assert_that!(ui.case.is_empty()).is_true();
}

#[test]
fn ui_state_default_shows_the_scene() {
    assert_that!(UiState::default().board).is_false();
}

#[test]
fn ui_state_without_board_reads_as_the_scene() {
    let ui: UiState = serde_json::from_str(r#"{"slide":null}"#).unwrap();
    assert_that!(ui.board).is_false();
}

/// La planche est commune à tous les widgets : gardée hors du mode modèles aussi.
#[test]
fn sanitized_keeps_the_board_in_and_out_of_templates() {
    let ui = UiState { board: true, ..UiState::default() };
    assert_that!(ui.clone().sanitized(true).board).is_true();
    assert_that!(ui.sanitized(false).board).is_true();
}
