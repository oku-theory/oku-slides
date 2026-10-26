use std::path::PathBuf;

use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::resume::{PACK_TAB_STATE_VERSION, VERSION};
use crate::test_support::{edit_of, fresh_dir, rename_title, session_on, stash_of};
use crate::test_support_packs::{CARD_V2, editing, opener_on, pack_tab_from};
use crate::{ESSENTIEL_PACK, Localize, Request, SessionErrorKind, TokenAction};

/// A pack tab on `acme`, its opener and their folders.
fn acme_tab() -> (tempfile::TempDir, tempfile::TempDir, Session, Session, Uuid) {
    let data = fresh_dir("oku-slides-pack-tab-");
    let (mut s, deck, acme) = opener_on(data.path());
    let t = pack_tab_from(&mut s, data.path(), acme);
    (data, deck, s, t, acme)
}

/// A token edit of the pack tab's theme draft.
fn edit_token(t: &mut Session) {
    t.host_token(vec!["color".into(), "accent".into()], TokenAction::Set, Some(json!("#00ff00"))).unwrap();
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

#[rstest]
#[case::widgets(false)]
#[case::theme(true)]
fn pack_tab_is_not_dirty_after_a_save(#[case] theme: bool) {
    let (_data, _deck, _s, mut t, _) = acme_tab();
    if theme {
        edit_token(&mut t);
    } else {
        t.apply("x", vec![rename_title("acme 2")], None).unwrap();
    }
    t.save_pack(false).unwrap();
    assert_that!(stash_of(&t).dirty).is_true();
    assert_that!(t.tab_info().dirty).is_false();
}

#[test]
fn pack_tab_theme_save_never_freezes_the_theme_version() {
    let (_data, _deck, _s, mut t, _) = acme_tab();
    let before = stash_of(&t).sources.deck().themes.clone();
    edit_token(&mut t);
    t.save_pack(false).unwrap();
    let preview = stash_of(&t).sources.deck();
    let pin = preview.theme.as_ref().and_then(|ns| preview.themes.get(ns)).cloned().unwrap();
    assert_that!(before.values().any(|p| p.version == pin.version)).is_false();
    assert_that!(t.themes.is_frozen(pin.id, &pin.version)).is_false();
}

#[test]
fn pack_tab_persists_its_preview_clean() {
    let (_data, _deck, _s, mut t, _) = acme_tab();
    let dir = fresh_dir("oku-slides-pack-tab-dir-");
    assert_that!(t.resume_fresh(dir.path().to_path_buf())).is_true();
    t.apply("x", vec![rename_title("acme 2")], None).unwrap();
    t.save_pack(false).unwrap();
    assert_that!(stash_of(&t).dirty).is_true();
    t.persist(true);
    t.flush_resume();
    let doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("deck").join("document.json")).unwrap()).unwrap();
    assert_that!(doc["dirty"]).is_equal_to(json!(false));
}

#[test]
fn pack_tab_state_is_written_as_version_4() {
    let (_data, _deck, _s, mut t, _) = acme_tab();
    let dir = fresh_dir("oku-slides-pack-tab-dir-");
    assert_that!(t.resume_fresh(dir.path().to_path_buf())).is_true();
    assert_that!(state(dir.path())["version"]).is_equal_to(json!(PACK_TAB_STATE_VERSION));
}

#[test]
fn deck_state_is_still_written_as_version_3() {
    let (_data, _deck, mut s, t, acme) = acme_tab();
    drop(t);
    let dir = fresh_dir("oku-slides-pack-tab-dir-");
    assert_that!(s.resume_fresh(dir.path().to_path_buf())).is_true();
    s.handle(Request::EditPack { pack: acme }).unwrap();
    s.flush_resume();
    let state = state(dir.path());
    assert_that!((state["version"].clone(), state["mode"]["pack"]["host"].clone()))
        .is_equal_to((json!(VERSION), json!("deck")));
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
    assert_that!(editing(&t).map(|(_, host)| host)).is_equal_to(Some(PackHost::Tab));
}

#[rstest]
#[case::save_copy(true)]
#[case::new_deck(false)]
fn save_copy_and_new_deck_in_the_deck_workshop_behave_as_today(#[case] save_copy: bool) {
    let (_data, deck, mut s, _t, _) = acme_tab();
    let d = crate::test_support::adopt_deck_only_pack(&mut s);
    s.edit_pack(d.id).unwrap();
    let path = deck.path().join(format!("copie{}", crate::EXTENSION));
    if save_copy {
        s.save_copy(&path).unwrap();
        assert_that!(path.exists()).is_true();
    } else {
        s.new_deck().unwrap();
        assert_that!(s.doc.blank).is_true();
    }
}

/// A pack-mode exit `edit_pack_and_close_pack_in_a_pack_tab_are_refused` tries.
#[derive(Debug, Clone, Copy)]
enum Exit {
    EditPackRequest,
    ClosePackRequest,
    EditPackMethod,
    ClosePackMethod,
}

#[rstest]
#[case::edit_pack_request(Exit::EditPackRequest)]
#[case::close_pack_request(Exit::ClosePackRequest)]
#[case::edit_pack_method(Exit::EditPackMethod)]
#[case::close_pack_method(Exit::ClosePackMethod)]
fn edit_pack_and_close_pack_in_a_pack_tab_are_refused(#[case] exit: Exit) {
    let (_data, _deck, _s, mut t, acme) = acme_tab();
    let err = match exit {
        Exit::EditPackRequest => t.handle(Request::EditPack { pack: crate::BASE_PACK }).map(|_| ()),
        Exit::ClosePackRequest => t.handle(Request::ClosePack).map(|_| ()),
        Exit::EditPackMethod => t.edit_pack(crate::BASE_PACK),
        Exit::ClosePackMethod => t.close_pack(),
    }
    .unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::InPackTab);
    assert_that!(err.localize(&t.localizer)).starts_with("Cet onglet édite un pack");
    assert_that!(editing(&t)).is_equal_to(Some((acme, PackHost::Tab)));
}

#[test]
fn save_pack_as_new_in_a_pack_tab_keeps_the_tab_host() {
    let (_data, _deck, _s, mut t, acme) = acme_tab();
    t.save_pack(true).unwrap();
    let (pack, host) = editing(&t).unwrap();
    assert_that!((pack != acme, host)).is_equal_to((true, PackHost::Tab));
}

#[test]
fn edit_pack_request_of_a_library_pack_in_a_deck_tab_still_enters_the_workshop() {
    let (_data, _deck, mut s, t, acme) = acme_tab();
    drop(t);
    s.handle(Request::EditPack { pack: acme }).unwrap();
    assert_that!(editing(&s)).is_equal_to(Some((acme, PackHost::Deck)));
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
    ThemeSave,
    Widget,
    LayoutOp,
}

#[rstest]
#[case::save(Wrote::Save)]
#[case::theme_save(Wrote::ThemeSave)]
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
        Wrote::ThemeSave => {
            edit_token(&mut t);
            t.save_pack(false).unwrap();
        }
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
