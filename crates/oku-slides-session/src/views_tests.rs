use speculoos::prelude::*;

use super::{MAX_WIDGET_TABS, PileSection, UiState};

fn tabs(widgets: &[&str], widget: Option<&str>) -> UiState {
    UiState {
        widgets: widgets.iter().map(|s| (*s).to_owned()).collect(),
        widget: widget.map(str::to_owned),
        ..UiState::default()
    }
}

#[test]
fn sanitized_outside_pack_clears_the_tabs() {
    let ui = tabs(&["kpi"], Some("kpi")).sanitized(false);
    assert_that!(ui.widgets).is_empty();
    assert_that!(ui.widget).is_none();
}

#[test]
fn sanitized_in_pack_mode_drops_invalid_and_duplicate_names() {
    let ui = tabs(&["kpi", "../x", "Kpi", "kpi", "_commun"], Some("kpi")).sanitized(true);
    assert_that!(ui.widgets).is_equal_to(vec!["kpi".to_owned(), "_commun".to_owned()]);
}

#[test]
fn sanitized_in_pack_mode_caps_the_list() {
    let names: Vec<String> = (0..MAX_WIDGET_TABS + 10).map(|i| format!("w{i}")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    assert_that!(tabs(&refs, None).sanitized(true).widgets).has_length(MAX_WIDGET_TABS);
}

#[test]
fn sanitized_with_an_active_tab_not_open_drops_it() {
    assert_that!(tabs(&["kpi"], Some("quote")).sanitized(true).widget).is_none();
}

#[test]
fn sanitized_drops_duplicate_folded_sections() {
    let ui = UiState {
        folded: vec![PileSection::Widgets, PileSection::Layouts, PileSection::Widgets],
        ..UiState::default()
    };
    assert_that!(ui.sanitized(true).folded).is_equal_to(vec![PileSection::Widgets, PileSection::Layouts]);
}

#[test]
fn sanitized_outside_pack_keeps_folded_sections() {
    let ui = UiState { folded: vec![PileSection::Layouts], ..UiState::default() };
    assert_that!(ui.sanitized(false).folded).is_equal_to(vec![PileSection::Layouts]);
}

#[test]
fn ui_state_default_has_the_band_at_its_default_height() {
    assert_that!(UiState::default().band).is_none();
}

#[test]
fn ui_state_default_has_the_code_at_its_default_width() {
    assert_that!(UiState::default().code_width).is_none();
}

/// Couche serde : un `UiState` écrit avant t-526 (planche, place et hauteur du code) se relit,
/// ces champs ignorés ; la largeur du code est gardée (la couche fichier : `resume_tests`).
#[test]
fn ui_state_with_the_removed_board_and_code_place_still_reads() {
    let ui: UiState = serde_json::from_str(
        r#"{"slide":null,"code":true,"board":true,"code_place":"right","code_height":300,"code_width":640}"#,
    )
    .unwrap();
    assert_that!(ui).is_equal_to(UiState { code: true, code_width: Some(640), ..UiState::default() });
}

/// Couche serde : un `UiState` écrit avant t-541 (cas d'exemple courant de chaque widget)
/// se relit, `case` ignoré ; les autres champs sont gardés.
#[test]
fn ui_state_with_the_removed_current_cases_still_reads() {
    let ui: UiState =
        serde_json::from_str(r#"{"slide":null,"widgets":["kpi"],"widget":"kpi","case":{"kpi":"Long"}}"#).unwrap();
    assert_that!(ui).is_equal_to(UiState {
        widgets: vec!["kpi".to_owned()],
        widget: Some("kpi".to_owned()),
        ..UiState::default()
    });
}

/// La pile des slides, repliée hors mode pack, se garde comme les sections du mode pack.
#[test]
fn ui_state_with_the_slides_section_folded_reads() {
    let ui: UiState = serde_json::from_str(r#"{"slide":null,"folded":["slides"]}"#).unwrap();
    assert_that!(ui.folded).is_equal_to(vec![PileSection::Slides]);
}

/// La carte des paramètres de la cible, repliée, se garde comme les sections de la pile.
#[test]
fn ui_state_with_the_params_card_folded_reads() {
    let ui: UiState = serde_json::from_str(r#"{"slide":null,"folded":["params"]}"#).unwrap();
    assert_that!(ui.folded).is_equal_to(vec![PileSection::Params]);
}
