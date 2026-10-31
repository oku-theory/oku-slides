use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::DocumentKind;
use rstest::rstest;
use speculoos::prelude::*;

use super::{MAX_NAMES, examples_for};
use crate::tool_error::ToolError;

/// The component names of a reference.
fn names(r: &[oku_slides_a2ui::ComponentReference]) -> Vec<String> {
    r.iter().map(|c| c.component.clone()).collect()
}

/// `examples_for` in a deck tab with these names.
fn deck(wanted: &[&str]) -> Result<Vec<oku_slides_a2ui::ComponentReference>, ToolError> {
    let wanted: Vec<String> = wanted.iter().map(|w| (*w).to_owned()).collect();
    examples_for(Some(&wanted), DocumentKind::Deck)
}

#[test]
fn examples_for_no_list_returns_every_render_component() {
    let all = examples_for(None, DocumentKind::Deck).unwrap();
    assert_that!(names(&all)).is_equal_to(
        ["Column", "Card", "Text", "Row", "Button", "DiffText", "Disclosure", "Badge", "Markdown"]
            .map(str::to_owned)
            .to_vec(),
    );
}

#[test]
fn examples_for_a_list_keeps_only_those_components_in_catalog_order() {
    assert_that!(names(&deck(&["Badge", "Card"]).unwrap())).is_equal_to(vec!["Card".to_owned(), "Badge".to_owned()]);
}

#[test]
fn examples_for_an_empty_list_returns_nothing() {
    assert_that!(deck(&[]).unwrap()).is_empty();
}

#[rstest]
#[case::alone(&["Carousel"])]
#[case::mixed(&["Card", "Carousel"])]
fn examples_for_an_unknown_name_is_a2ui_unknown(#[case] wanted: &[&str]) {
    let e = deck(wanted).unwrap_err();
    assert_that!(matches!(e, ToolError::A2uiUnknown { ref name, .. } if name == "Carousel")).is_true();
}

#[test]
fn examples_for_an_unknown_name_echoes_it_sanitized_and_bounded() {
    let long = format!("<b>{}</b>", "x".repeat(200));
    let e = deck(&[&long]).unwrap_err();
    assert_that!(
        matches!(e, ToolError::A2uiUnknown { ref name, .. } if name.ends_with('\u{2026}') && name.trim_end_matches('\u{2026}').len() <= 32 && name.trim_end_matches('\u{2026}').chars().all(|c| c.is_ascii_alphanumeric()))
    )
    .is_true();
}

#[test]
fn examples_for_more_names_than_the_cap_is_an_argument_error() {
    let many: Vec<String> = (0..=MAX_NAMES).map(|i| format!("C{i}")).collect();
    let e = examples_for(Some(&many), DocumentKind::Deck).unwrap_err();
    assert_that!(matches!(e, ToolError::Arguments { .. })).is_true();
}

#[rstest]
#[case::text_field("TextField", "show_params")]
#[case::list_table("ListTable", "show_params")]
#[case::slider("Slider", "show_params")]
#[case::choice_chips("ChoiceChips", "show_params")]
#[case::receipt("Receipt", "report")]
#[case::thumb("SlideThumb", "report")]
#[case::theme("ThemeChoice", "show_themes")]
fn examples_for_a_component_render_refuses_names_its_tool(#[case] name: &str, #[case] tool: &str) {
    let e = deck(&[name]).unwrap_err();
    assert_that!(matches!(e, ToolError::A2uiElsewhere { tool: t, .. } if t == tool)).is_true();
}

#[test]
fn examples_for_theme_choice_outside_a_deck_tab_is_host_only() {
    let wanted = vec!["ThemeChoice".to_owned()];
    let e = examples_for(Some(&wanted), DocumentKind::Pack).unwrap_err();
    assert_that!(matches!(e, ToolError::A2uiHostOnly { .. })).is_true();
}

#[rstest]
#[case::prompt("Prompt")]
#[case::timeline("Timeline")]
#[case::notice("Notice")]
#[case::chart_data("ChartData")]
#[case::table_data("TableData")]
fn examples_for_a_host_only_component_is_a2ui_host_only(#[case] name: &str) {
    let e = deck(&[name]).unwrap_err();
    assert_that!(matches!(e, ToolError::A2uiHostOnly { .. })).is_true();
}

#[test]
fn a2ui_refusal_is_translated_in_french_and_names_the_tool() {
    let e = deck(&["TextField"]).unwrap_err();
    let fr = e.localize(&Localizer::new(Some("fr")));
    let en = e.localize(&Localizer::new(Some("en")));
    assert_that!(fr.as_str()).is_not_equal_to(en.as_str());
    assert_that!(fr).contains("show_params");
}

#[rstest]
#[case::trailing_space("Card ", "Card\u{2026}")]
#[case::zero_width("Card\u{200b}", "Card\u{2026}")]
#[case::all_symbols("<>!", "?")]
#[case::empty("", "?")]
#[case::clean("Carousel", "Carousel")]
fn examples_for_an_altered_echo_never_looks_like_a_valid_name(#[case] wanted: &str, #[case] echoed: &str) {
    let e = deck(&[wanted]).unwrap_err();
    assert_that!(matches!(e, ToolError::A2uiUnknown { ref name, .. } if name == echoed)).is_true();
}
