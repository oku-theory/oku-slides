//! t-1237: a save after an agent turn in a pack tab dropped the turn's version.
//!
//! `begin_turn` opened the turn's undo group on the pack's document only, not on
//! the preview: a save during the turn and one after it shared one preview step
//! (the `pack-adopt` merge key), so the turn's version left the library.

use oku_slides_core::Op;
use oku_slides_session::{ESSENTIEL_PACK, PackLibrary, Request, Storage, Workspace};
use speculoos::prelude::*;

use crate::common::{data_dir, no_user_themes};
use crate::workspace_support::{on, open_pack, workspace};

/// Renames the edited pack to `name` and saves it in tab `tab`: the session version.
fn rename_and_save(ws: &std::sync::Mutex<Workspace>, tab: &str, name: &str) -> String {
    let op = serde_json::json!({ "op": "set_deck_param", "key": "title", "value": name });
    let op: Op = serde_json::from_value(op).unwrap_or_else(|e| panic!("{e}"));
    on(ws, tab, |s| s.apply("Nom", vec![op], None)).unwrap_or_else(|e| panic!("rename: {}", e.one_line()));
    let (saved, _) = Workspace::request(ws, tab, Request::SavePack { as_new: false });
    saved.unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    on(ws, tab, |s| s.snapshot().editing.and_then(|e| e.session_version.clone()))
        .unwrap_or_else(|| panic!("a session version"))
        .to_string()
}

#[test]
fn issue_1237_save_after_turn_keeps_turn_version() {
    let data = data_dir();
    let ws = workspace(&data);
    let tab = open_pack(&ws, "t-1", ESSENTIEL_PACK);
    on(&ws, &tab, |s| s.begin_turn(1, "rename"));
    let turn = rename_and_save(&ws, &tab, "Un");
    on(&ws, &tab, oku_slides_session::Session::end_turn);
    let after = rename_and_save(&ws, &tab, "Deux");
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    let library = PackLibrary::new(Some(data.join("widgets")), Some(data.join("layouts")), no_user_themes(), storage);
    let versions: Vec<String> = library.versions(ESSENTIEL_PACK).into_iter().map(|v| v.version.to_string()).collect();
    assert_that!((versions.contains(&turn), versions.contains(&after))).is_equal_to((true, true));
}
