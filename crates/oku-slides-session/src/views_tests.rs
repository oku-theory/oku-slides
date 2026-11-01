use rstest::rstest;
use speculoos::prelude::*;

use super::{PileSection, UiState};
use crate::PreviewSpan;

#[test]
fn sanitized_outside_pack_clears_the_selected_item() {
    let ui = UiState { widget: Some("kpi".to_owned()), ..UiState::default() }.sanitized(false);
    assert_that!(ui.widget).is_none();
}

#[rstest]
#[case::widget("kpi")]
#[case::common("_commun")]
fn sanitized_in_pack_mode_keeps_a_valid_widget(#[case] name: &str) {
    let ui = UiState { widget: Some(name.to_owned()), ..UiState::default() }.sanitized(true);
    assert_that!(ui.widget).is_equal_to(Some(name.to_owned()));
}

#[rstest]
#[case::empty("")]
#[case::traversal("../x")]
#[case::nul("a\0b")]
#[case::long("a".repeat(300))]
fn sanitized_in_pack_mode_clears_an_invalid_widget(#[case] name: String) {
    let ui = UiState { widget: Some(name), ..UiState::default() }.sanitized(true);
    assert_that!(ui.widget).is_none();
}

#[test]
fn sanitized_drops_duplicate_folded_sections() {
    let ui =
        UiState { folded: vec![PileSection::Layouts, PileSection::Slides, PileSection::Layouts], ..UiState::default() };
    assert_that!(ui.sanitized(true).folded).is_equal_to(vec![PileSection::Layouts, PileSection::Slides]);
}

#[test]
fn sanitized_outside_pack_keeps_folded_sections() {
    let ui = UiState { folded: vec![PileSection::Layouts], ..UiState::default() };
    assert_that!(ui.sanitized(false).folded).is_equal_to(vec![PileSection::Layouts]);
}

/// Serde layer: a `UiState` written before t-526 (board, code place and height) or t-1419
/// (code width) reads back, those fields ignored (the file layer: `resume_tests`).
#[test]
fn ui_state_with_the_removed_board_and_code_place_still_reads() {
    let ui: UiState = serde_json::from_str(
        r#"{"slide":null,"code":true,"board":true,"code_place":"right","code_height":300,"code_width":640}"#,
    )
    .unwrap();
    assert_that!(ui).is_equal_to(UiState { code: true, ..UiState::default() });
}

/// Serde layer: a `UiState` written before t-541 (each widget's current example case)
/// reads back, `case` ignored; the other fields are kept.
#[test]
fn ui_state_with_the_removed_current_cases_still_reads() {
    let ui: UiState = serde_json::from_str(r#"{"slide":null,"widget":"kpi","case":{"kpi":"Long"}}"#).unwrap();
    assert_that!(ui).is_equal_to(UiState { widget: Some("kpi".to_owned()), ..UiState::default() });
}

/// Serde layer: a `UiState` written with the grid toggle (before t-1283) reads back without it.
#[test]
fn ui_state_with_the_removed_grid_still_reads() {
    let ui: UiState = serde_json::from_str(r#"{"slide":null,"grid":false,"ghosts":true}"#).unwrap();
    assert_that!(ui).is_equal_to(UiState { ghosts: true, ..UiState::default() });
}

/// Serde layer: a `UiState` written with the widget sub-tabs (before t-1244) reads back without them.
#[test]
fn ui_state_with_legacy_widgets_key_reads() {
    let ui: UiState = serde_json::from_str(r#"{"slide":null,"widgets":["kpi","_commun"],"widget":"kpi"}"#).unwrap();
    assert_that!(ui).is_equal_to(UiState { widget: Some("kpi".to_owned()), ..UiState::default() });
}

/// La pile des slides, repliée hors mode pack, se garde comme les sections du mode pack.
#[test]
fn ui_state_with_the_slides_section_folded_reads() {
    let ui: UiState = serde_json::from_str(r#"{"slide":null,"folded":["slides"]}"#).unwrap();
    assert_that!(ui.folded).is_equal_to(vec![PileSection::Slides]);
}

/// The kept state's reader, as `state.json` uses it.
#[derive(serde::Deserialize)]
struct Kept {
    #[serde(deserialize_with = "crate::views::kept_ui")]
    ui: UiState,
}

/// A kept state from before the parameters card was removed reads back without it.
#[test]
fn kept_ui_with_a_retired_section_folded_drops_it() {
    let kept: Kept = serde_json::from_str(r#"{"ui":{"slide":null,"folded":["params","widgets"]}}"#).unwrap();
    assert_that!(kept.ui.folded).is_empty();
}

/// A kept state from before t-1244 folds the widgets section: only that section goes.
#[test]
fn kept_ui_with_legacy_widgets_section_drops_it_only() {
    let kept: Kept = serde_json::from_str(r#"{"ui":{"slide":null,"folded":["widgets","layouts"]}}"#).unwrap();
    assert_that!(kept.ui.folded).is_equal_to(vec![PileSection::Layouts]);
}

/// Hors de l'état gardé, une section inconnue est refusée.
#[test]
fn ui_state_with_an_unknown_section_folded_is_refused() {
    assert_that!(serde_json::from_str::<UiState>(r#"{"slide":null,"folded":["params"]}"#).is_err()).is_true();
}

#[test]
fn ui_state_without_preview_span_reads_full() {
    let ui: UiState = serde_json::from_str(r#"{"slide":null,"code":false}"#).unwrap();
    assert_that!(ui.preview_span).is_equal_to(PreviewSpan::Full);
}
