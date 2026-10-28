use std::path::PathBuf;

use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::resume::{EDITOR_STATE_VERSION, VERSION};
use crate::test_support::{edit_of, fresh_dir, preview_of, rename_title, session_on};
use crate::test_support_packs::{CARD_V2, editing, opener_on, pack_tab_from};
use crate::{ESSENTIEL_PACK, Request, SessionErrorKind};

/// A pack tab on `acme`, its opener and their folders.
fn acme_tab() -> (tempfile::TempDir, tempfile::TempDir, Session, Session, Uuid) {
    let data = fresh_dir("oku-slides-pack-tab-");
    let (mut s, deck, acme) = opener_on(data.path());
    let t = pack_tab_from(&mut s, data.path(), acme);
    (data, deck, s, t, acme)
}

/// The `state.json` of `dir`.
fn state(dir: &std::path::Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(dir.join("state.json")).unwrap()).unwrap()
}

#[test]
fn pack_tab_is_dirty_after_an_unsaved_edit() {
    let (_data, _deck, _s, mut t, _) = acme_tab();
    assert_that!(t.tab_info().dirty).is_false();
    t.apply("x", vec![rename_title("acme 2")], None).unwrap();
    assert_that!(t.tab_info().dirty).is_true();
}

#[test]
fn pack_tab_is_not_dirty_after_a_save() {
    let (_data, _deck, _s, mut t, _) = acme_tab();
    t.apply("x", vec![rename_title("acme 2")], None).unwrap();
    t.save_pack(false).unwrap();
    assert_that!(preview_of(&t).dirty).is_true();
    assert_that!(t.tab_info().dirty).is_false();
}

#[test]
fn pack_tab_persists_its_preview_clean() {
    let (_data, _deck, _s, mut t, _) = acme_tab();
    let dir = fresh_dir("oku-slides-pack-tab-dir-");
    assert_that!(t.resume_fresh(dir.path().to_path_buf())).is_true();
    t.apply("x", vec![rename_title("acme 2")], None).unwrap();
    t.save_pack(false).unwrap();
    assert_that!(preview_of(&t).dirty).is_true();
    t.persist(true);
    t.flush_resume();
    let doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("deck").join("document.json")).unwrap()).unwrap();
    assert_that!(doc["dirty"]).is_equal_to(json!(false));
}

#[test]
fn pack_tab_state_is_written_as_version_5() {
    let (_data, _deck, _s, mut t, _) = acme_tab();
    let dir = fresh_dir("oku-slides-pack-tab-dir-");
    assert_that!(t.resume_fresh(dir.path().to_path_buf())).is_true();
    assert_that!(state(dir.path())["version"]).is_equal_to(json!(EDITOR_STATE_VERSION));
}

#[test]
fn deck_state_is_written_as_version_3() {
    let (_data, _deck, mut s, t, _) = acme_tab();
    drop(t);
    let dir = fresh_dir("oku-slides-pack-tab-dir-");
    assert_that!(s.resume_fresh(dir.path().to_path_buf())).is_true();
    assert_that!(state(dir.path())["version"]).is_equal_to(json!(VERSION));
}

/// A file write `pack_tab_refuses_writing_a_file` tries.
#[derive(Debug, Clone, Copy)]
enum FileWrite {
    Save,
    SaveCopy,
    Move,
    Merge,
    SaveCopyMethod,
    SaveCopyNewMethod,
    NewDeckMethod,
}

#[rstest]
#[case::save(FileWrite::Save, SessionErrorKind::EditingPack)]
#[case::save_copy(FileWrite::SaveCopy, SessionErrorKind::EditingPack)]
#[case::move_to(FileWrite::Move, SessionErrorKind::EditingPack)]
#[case::merge(FileWrite::Merge, SessionErrorKind::EditingPack)]
#[case::save_copy_method(FileWrite::SaveCopyMethod, SessionErrorKind::InPackTab)]
#[case::save_copy_new_method(FileWrite::SaveCopyNewMethod, SessionErrorKind::InPackTab)]
#[case::new_deck_method(FileWrite::NewDeckMethod, SessionErrorKind::InPackTab)]
fn pack_tab_refuses_writing_a_file(#[case] write: FileWrite, #[case] kind: SessionErrorKind) {
    let (_data, deck, _s, mut t, _) = acme_tab();
    let path: PathBuf = deck.path().join(format!("copie{}", crate::EXTENSION));
    let err = match write {
        FileWrite::Save => t.handle(Request::Save { path: Some(path.clone()), overwrite: true }).map(|_| ()),
        FileWrite::SaveCopy => t.handle(Request::SaveCopy { path: path.clone() }).map(|_| ()),
        FileWrite::Move => t.handle(Request::Move { path: path.clone() }).map(|_| ()),
        FileWrite::Merge => t.handle(Request::Merge { path: path.clone() }).map(|_| ()),
        FileWrite::SaveCopyMethod => t.save_copy(&path),
        FileWrite::SaveCopyNewMethod => t.save_copy_new(&path),
        FileWrite::NewDeckMethod => t.new_deck(),
    }
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(kind);
    assert_that!(path.exists()).is_false();
    assert_that!(t.is_pack_tab()).is_true();
}

#[test]
fn save_pack_as_new_in_a_pack_tab_stays_the_copy_s_pack_tab() {
    let (_data, _deck, _s, mut t, acme) = acme_tab();
    t.save_pack(true).unwrap();
    let pack = editing(&t).unwrap();
    assert_that!((pack != acme, t.tab_info().document)).is_equal_to((true, TabDocument::Pack { pack }));
}

#[test]
fn pack_tab_title_is_a_label() {
    let (_data, _deck, _s, mut t, _) = acme_tab();
    t.apply("x", vec![rename_title("acme\n\u{2069}\"x")], None).unwrap();
    t.save_pack(false).unwrap();
    assert_that!(t.tab_info().title).is_equal_to("acme \u{02BC}x".to_owned());
}

/// A write `wrote_in_pack_tab_after_a_write` makes.
#[derive(Debug, Clone, Copy)]
enum Wrote {
    Save,
    Widget,
    LayoutOp,
}

#[rstest]
#[case::save(Wrote::Save)]
#[case::widget(Wrote::Widget)]
#[case::layout_op(Wrote::LayoutOp)]
fn wrote_in_pack_tab_after_a_write(#[case] wrote: Wrote) {
    let data = fresh_dir("oku-slides-pack-tab-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let pack = if matches!(wrote, Wrote::LayoutOp) { ESSENTIEL_PACK } else { acme };
    let mut t = pack_tab_from(&mut s, data.path(), pack);
    assert_that!(t.wrote_in_pack_tab().is_none()).is_true();
    match wrote {
        Wrote::Save => drop(t.save_pack(false).unwrap()),
        Wrote::Widget => drop(t.set_widget(None, "card", CARD_V2.into()).unwrap()),
        Wrote::LayoutOp => {
            let op = json!({ "op": "set_param", "target": { "slide": "layout-content" }, "param": "title", "value": "Neuf" });
            t.apply("titre", vec![serde_json::from_value(op).unwrap()], None).unwrap();
        }
    }
    let wrote = t.wrote_in_pack_tab().unwrap();
    assert_that!((wrote.pack, Some(wrote.version))).is_equal_to((pack, edit_of(&t).session_version));
}

#[test]
fn wrote_in_pack_tab_is_none_after_undoing_to_the_parent() {
    let (_data, _deck, _s, mut t, _) = acme_tab();
    t.set_widget(None, "card", CARD_V2.into()).unwrap();
    assert_that!(t.wrote_in_pack_tab().is_some()).is_true();
    t.undo().unwrap();
    assert_that!(t.wrote_in_pack_tab().is_none()).is_true();
}

#[test]
fn set_pack_dirs_in_pack_mode_is_refused() {
    let (data, _deck, _s, mut t, _) = acme_tab();
    let err = t.set_pack_dirs(data.path().join("w2"), data.path().join("l2")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::EditingPack);
}

#[test]
fn set_pack_dirs_in_help_is_allowed() {
    let data = fresh_dir("oku-slides-pack-tab-");
    let mut s = session_on(data.path());
    s.open_help("fr", None).unwrap();
    s.set_pack_dirs(data.path().join("w2"), data.path().join("l2")).unwrap();
    let expected = crate::pack_lease::LeaseScope::new(&data.path().join("w2"), &data.path().join("l2"));
    assert_that!(s.library.lease_scope().cloned()).is_equal_to(Some(expected));
}

#[test]
fn delete_pack_of_its_own_pack_in_a_pack_tab_is_refused_as_in_pack_tab() {
    let (_data, _deck, _s, mut t, acme) = acme_tab();
    let err = t.handle(Request::DeletePack { pack: acme }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InPackTab);
    assert_that!(t.library.current(acme).is_some()).is_true();
}

#[test]
fn open_pack_tab_here_opens_a_library_pack_tab() {
    let data = fresh_dir("oku-slides-pack-tab-");
    let mut s = session_on(data.path());
    s.open_pack_tab_here(ESSENTIEL_PACK, Some("content")).unwrap();
    let info = s.tab_info();
    assert_that!((editing(&s), info.document, s.ui.widget.as_deref())).is_equal_to((
        Some(ESSENTIEL_PACK),
        TabDocument::Pack { pack: ESSENTIEL_PACK },
        Some("content"),
    ));
    let view = s.editing_view().unwrap();
    assert_that!((view.read_only, view.leased)).is_equal_to((false, true));
}

#[test]
fn open_pack_tab_here_of_a_deck_only_pack_is_refused() {
    let data = fresh_dir("oku-slides-pack-tab-");
    let mut s = session_on(data.path());
    let pin = crate::test_support::adopt_deck_only_pack(&mut s);
    let err = s.open_pack_tab_here(pin.id, None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::DeckOnly);
    assert_that!(editing(&s)).is_none();
}
