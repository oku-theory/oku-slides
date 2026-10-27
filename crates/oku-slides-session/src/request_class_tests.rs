use std::path::PathBuf;

use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::Request;
use crate::views::UiState;
use crate::{DataEdit, ExtendsEdit, TokenAction, View};

/// A version of the tests.
fn version() -> oku_slides_core::PackVersion {
    crate::test_support::builtin(crate::BASE_PACK).version.clone()
}

#[rstest]
#[case::apply(Request::Apply { label: "x".into(), ops: Vec::new(), merge: None }, true)]
#[case::set_theme(Request::SetTheme { theme: "pro".into() }, true)]
#[case::edit_theme(Request::EditTheme { id: Uuid::nil(), name: None, ops: Vec::new() }, true)]
#[case::undo(Request::Undo, true)]
#[case::redo(Request::Redo, true)]
#[case::undo_turn(Request::UndoTurn { turn: 1 }, true)]
#[case::chart_kind(Request::ChartKind { label: "x".into(), slide: "s".into(), element: "e".into(), kind: oku_slides_chart::SeriesType::Bar }, true)]
#[case::show_as(Request::ShowAs { label: "x".into(), slide: "s".into(), element: "e".into(), view: View::Table }, true)]
#[case::data_edit(Request::DataEdit { slide: "s".into(), element: "e".into(), edit: DataEdit::Stack { stacked: true } }, true)]
#[case::host_token(Request::HostToken { path: vec!["color".into()], action: TokenAction::Reset, value: None }, true)]
#[case::save_pack(Request::SavePack { as_new: false }, true)]
#[case::save_pack_as_new(Request::SavePack { as_new: true }, false)]
#[case::save_theme(Request::SaveTheme { as_new: false }, true)]
#[case::save_theme_as_new(Request::SaveTheme { as_new: true }, false)]
#[case::set_edited_theme_extends(Request::SetEditedThemeExtends { extends: Vec::new() }, true)]
#[case::edit_theme_extends(Request::EditThemeExtends { edit: ExtendsEdit::Add { theme: crate::test_support::ns("pro") } }, true)]
#[case::rename_edited_theme(Request::RenameEditedTheme { name: "x".into() }, true)]
#[case::set_widget(Request::SetWidget { pack: None, name: "w".into(), source: String::new() }, true)]
#[case::remove_widget(Request::RemoveWidget { pack: None, name: "w".into() }, true)]
#[case::rename_widget(Request::RenameWidget { pack: None, from: "a".into(), to: "b".into() }, true)]
#[case::set_example_value(Request::SetExampleValue { name: "w".into(), param: "p".into(), value: serde_json::Value::Null }, true)]
#[case::edit_example_list(Request::EditExampleList { name: "w".into(), param: "p".into(), change: oku_slides_core::ListOp::Remove { item: "i".into() } }, true)]
#[case::snapshot(Request::Snapshot, false)]
#[case::save(Request::Save { path: None, overwrite: false }, false)]
#[case::slide(Request::Slide { id: "s".into() }, false)]
#[case::search(Request::Search { query: "q".into() }, false)]
#[case::svg(Request::Svg { id: "s".into() }, false)]
#[case::schema(Request::Schema, false)]
#[case::widgets(Request::Widgets, false)]
#[case::host_settings(Request::HostSettings { widget: None }, false)]
#[case::host_theme(Request::HostTheme { filter: String::new() }, false)]
#[case::data_panel(Request::DataPanel { slide: "s".into(), element: "e".into() }, false)]
#[case::layouts(Request::Layouts, false)]
#[case::packs(Request::Packs { kind: oku_slides_core::PackKind::Widgets }, false)]
#[case::edit_pack(Request::EditPack { pack: Uuid::nil() }, false)]
#[case::close_pack(Request::ClosePack, false)]
#[case::delete_pack(Request::DeletePack { pack: Uuid::nil() }, false)]
#[case::ui(Request::Ui { ui: UiState::default(), order: None }, false)]
#[case::merge(Request::Merge { path: PathBuf::from("/x") }, false)]
#[case::history_open(Request::HistoryOpen, false)]
#[case::history_seek(Request::HistorySeek { index: 0 }, false)]
#[case::history_restore(Request::HistoryRestore, false)]
#[case::history_close(Request::HistoryClose, false)]
#[case::history_versions(Request::HistoryVersions, false)]
#[case::move_to(Request::Move { path: PathBuf::from("/x") }, false)]
#[case::save_copy(Request::SaveCopy { path: PathBuf::from("/x") }, false)]
#[case::widget_status(Request::WidgetStatus { pack: None }, false)]
#[case::widget_source(Request::WidgetSource { pack: None, name: "w".into() }, false)]
#[case::preview_widget(Request::PreviewWidget { pack: None, name: "w".into(), params: serde_json::Map::new() }, false)]
#[case::use_pack_version(Request::UsePackVersion { pack: Uuid::nil(), version: version(), deck_uid: "d".into() }, false)]
fn writes_edited_draft_classifies_every_request(#[case] request: Request, #[case] writes: bool) {
    assert_that!(request.writes_edited_draft()).is_equal_to(writes);
}

#[rstest]
#[case::snapshot(Request::Snapshot, true)]
#[case::slide(Request::Slide { id: "s".into() }, true)]
#[case::search(Request::Search { query: "q".into() }, true)]
#[case::svg(Request::Svg { id: "s".into() }, true)]
#[case::schema(Request::Schema, true)]
#[case::widgets(Request::Widgets, true)]
#[case::host_theme(Request::HostTheme { filter: String::new() }, true)]
#[case::host_token(Request::HostToken { path: vec!["color".into()], action: TokenAction::Reset, value: None }, true)]
#[case::ui(Request::Ui { ui: UiState::default(), order: None }, true)]
#[case::apply(Request::Apply { label: "x".into(), ops: Vec::new(), merge: None }, true)]
#[case::undo(Request::Undo, true)]
#[case::redo(Request::Redo, true)]
#[case::undo_turn(Request::UndoTurn { turn: 1 }, true)]
#[case::rename_edited_theme(Request::RenameEditedTheme { name: "x".into() }, true)]
#[case::set_edited_theme_extends(Request::SetEditedThemeExtends { extends: Vec::new() }, true)]
#[case::edit_theme_extends(Request::EditThemeExtends { edit: ExtendsEdit::Add { theme: crate::test_support::ns("pro") } }, true)]
#[case::save_theme(Request::SaveTheme { as_new: false }, true)]
#[case::save_theme_as_new(Request::SaveTheme { as_new: true }, true)]
#[case::save(Request::Save { path: None, overwrite: false }, false)]
#[case::save_pack(Request::SavePack { as_new: false }, false)]
#[case::edit_pack(Request::EditPack { pack: Uuid::nil() }, false)]
#[case::edit_theme(Request::EditTheme { id: Uuid::nil(), name: None, ops: Vec::new() }, false)]
#[case::set_theme(Request::SetTheme { theme: "pro".into() }, false)]
#[case::host_settings(Request::HostSettings { widget: None }, false)]
#[case::layouts(Request::Layouts, false)]
#[case::packs(Request::Packs { kind: oku_slides_core::PackKind::Widgets }, false)]
#[case::widget_status(Request::WidgetStatus { pack: None }, false)]
#[case::widget_source(Request::WidgetSource { pack: None, name: "w".into() }, false)]
#[case::preview_widget(Request::PreviewWidget { pack: None, name: "w".into(), params: serde_json::Map::new() }, false)]
#[case::history_versions(Request::HistoryVersions, false)]
#[case::history_open(Request::HistoryOpen, false)]
#[case::close_pack(Request::ClosePack, false)]
#[case::save_copy(Request::SaveCopy { path: PathBuf::from("/x") }, false)]
fn theme_tab_allows_only_the_draft_requests(#[case] request: Request, #[case] allows: bool) {
    assert_that!(request.theme_tab_allows()).is_equal_to(allows);
}
