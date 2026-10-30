//! Each tab's agent lists only its kind's tools (spec: `tool_list_filtered_by_document_kind`).
#![expect(clippy::panic, reason = "test failures panic")]
mod common;

use common::kinds::{Kind, list_tool_names};
use common::*;
use oku_slides_i18n::Localizer;
use oku_slides_session::PAPIER_THEME;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

/// The spec's test, on a real server per kind; the exact lists are `tools_tests.rs`'s.
#[rstest]
#[case::deck(Kind::Deck, &["apply", "save", "open_pack", "open_theme", "show_packs"][..], &["set_widget_trial", "set_theme_extends", "save_pack"][..])]
#[case::help(Kind::Help, &["apply", "open_pack", "open_theme", "history"][..], &["set_widget_trial", "set_theme_extends"][..])]
#[case::pack(Kind::Pack, &["set_widget", "set_widget_trial", "save_pack", "show_params"][..], &["save", "history", "open_pack", "open_theme", "set_theme_extends"][..])]
#[case::theme(Kind::Theme, &["apply", "set_theme_extends", "theme_tokens", "show_variants"][..], &["set_widget", "save", "open_pack", "open_theme", "save_pack"][..])]
#[tokio::test(flavor = "multi_thread")]
async fn tool_list_filtered_by_document_kind(#[case] kind: Kind, #[case] has: &[&str], #[case] lacks: &[&str]) {
    let h = kind.start().await;
    let c = server(&h).await;
    let names = list_tool_names(&c).await;
    for n in has {
        assert_that!(names).named(n).contains((*n).to_owned());
    }
    for n in lacks {
        assert_that!(names).named(n).does_not_contain((*n).to_owned());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn call_of_a_tool_another_kind_lists_is_unknown() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = server(&h).await;
    let r = call(&c, "set_widget", json!({ "name": "x", "source": "" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let fr = Localizer::new(Some("fr"));
    // Names the key and checks the non-English text: the harness localizer is fr.
    assert_that!(text_of(&r)).is_equal_to(fr.tr_with("agent-error-unknown-tool", &[("name", "set_widget".into())]));
}

#[tokio::test(flavor = "multi_thread")]
async fn call_of_a_listed_tool_in_its_kind_is_served() {
    let h = start_theme_tab(PAPIER_THEME).await;
    let c = server(&h).await;
    assert_that!(call(&c, "theme_tokens", json!({ "theme": "papier" })).await.is_error).is_not_equal_to(Some(true));
}
