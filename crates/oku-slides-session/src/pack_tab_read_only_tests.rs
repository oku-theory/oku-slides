use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::library::{DraftWrite, draft_lock};
use crate::library_index::library_writes;
use crate::pack_lease::{LeaseScope, pack_leased};
use crate::pack_library::{restore_session_version, write_session_version};
use crate::resume::Resumed;
use crate::test_support::{add_version, edit_of, fresh_dir, manifest_ns, rename_title, session_on};
use crate::test_support_packs::{
    CARD_V2, ReadOnly, delete_from_library, dormant_editor, draft_of, opener_on, pack_tab_from, read_only,
    read_only_pack_tab, read_only_workshop, scope_of, snapshot_json, unfreeze,
};
use crate::{BASE_PACK, Localize, PackLibrary, Request, SessionErrorKind, TokenAction};

/// The versions of `pack` the library of `s` has.
fn versions(s: &Session, pack: Uuid) -> Vec<PackVersion> {
    s.library.versions(pack).into_iter().map(|v| v.version).collect()
}

/// The read-only flags of `s`'s snapshot.
fn flags(s: &Session) -> (bool, bool) {
    let view = s.editing_view().unwrap_or_else(|| panic!("pack mode"));
    (view.read_only, view.leased)
}

/// An edit `read_only_pack_tab_refuses_every_edit` tries, and the path it takes.
#[derive(Debug, Clone, Copy)]
enum Edit {
    SavePackRequest,
    SavePackMethod,
    WidgetRequest,
    WidgetByAgent,
    OpRequest,
    OpMethod,
    UndoRequest,
    UndoMethod,
    RedoMethod,
    HostToken,
    RenameEditedTheme,
}

/// `edit` on `t`.
fn try_edit(t: &mut Session, edit: Edit) -> crate::Result<()> {
    let op = || rename_title("acme renommé");
    let card = || CARD_V2.replace("strong", "underline");
    match edit {
        Edit::SavePackRequest => t.handle(Request::SavePack { as_new: false }).map(drop),
        Edit::SavePackMethod => t.save_pack(false).map(drop),
        Edit::WidgetRequest => {
            t.handle(Request::SetWidget { pack: None, name: "card".into(), source: card() }).map(drop)
        }
        Edit::WidgetByAgent => t.set_widget_by_agent(None, "card", card()).map(drop),
        Edit::OpRequest => t.handle(Request::Apply { label: "x".into(), ops: vec![op()], merge: None }).map(drop),
        Edit::OpMethod => t.apply("x", vec![op()], None),
        Edit::UndoRequest => t.handle(Request::Undo).map(drop),
        Edit::UndoMethod => t.undo(),
        Edit::RedoMethod => t.redo(),
        Edit::HostToken => {
            let path = vec!["color".into(), "accent".into()];
            t.handle(Request::HostToken { path, action: TokenAction::Set, value: Some(json!("#0000ff")) }).map(drop)
        }
        Edit::RenameEditedTheme => t.handle(Request::RenameEditedTheme { name: "Autre".into() }).map(drop),
    }
}

#[rstest]
#[case::tab_save_request(PackHost::Tab, Edit::SavePackRequest)]
#[case::tab_save_method(PackHost::Tab, Edit::SavePackMethod)]
#[case::tab_widget_request(PackHost::Tab, Edit::WidgetRequest)]
#[case::tab_widget_by_agent(PackHost::Tab, Edit::WidgetByAgent)]
#[case::tab_op_request(PackHost::Tab, Edit::OpRequest)]
#[case::tab_op_method(PackHost::Tab, Edit::OpMethod)]
#[case::tab_undo_request(PackHost::Tab, Edit::UndoRequest)]
#[case::tab_undo_method(PackHost::Tab, Edit::UndoMethod)]
#[case::tab_redo_method(PackHost::Tab, Edit::RedoMethod)]
#[case::tab_host_token(PackHost::Tab, Edit::HostToken)]
#[case::tab_rename_edited_theme(PackHost::Tab, Edit::RenameEditedTheme)]
#[case::deck_save_request(PackHost::Deck, Edit::SavePackRequest)]
#[case::deck_save_method(PackHost::Deck, Edit::SavePackMethod)]
#[case::deck_widget_request(PackHost::Deck, Edit::WidgetRequest)]
#[case::deck_widget_by_agent(PackHost::Deck, Edit::WidgetByAgent)]
#[case::deck_op_request(PackHost::Deck, Edit::OpRequest)]
#[case::deck_op_method(PackHost::Deck, Edit::OpMethod)]
#[case::deck_undo_request(PackHost::Deck, Edit::UndoRequest)]
#[case::deck_undo_method(PackHost::Deck, Edit::UndoMethod)]
#[case::deck_redo_method(PackHost::Deck, Edit::RedoMethod)]
#[case::deck_host_token(PackHost::Deck, Edit::HostToken)]
#[case::deck_rename_edited_theme(PackHost::Deck, Edit::RenameEditedTheme)]
fn read_only_pack_tab_refuses_every_edit(#[case] host: PackHost, #[case] edit: Edit) {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { mut tab, h, pack, holder: _holder, dir: _dir, deck_dir: _deck_dir } = read_only(data.path(), host);
    let (before, listed, current) = (snapshot_json(&tab), versions(&tab, pack), tab.library.current(pack));
    let err = try_edit(&mut tab, edit).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackOpenInTab);
    assert_that!(snapshot_json(&tab)).is_equal_to(before);
    assert_that!((versions(&tab, pack), tab.library.current(pack))).is_equal_to((listed, current));
    assert_that!(draft_of(&tab, pack, &h)).is_equal_to(DraftWrite::Replace(h));
}

#[test]
fn read_only_pack_tab_keeps_its_unsaved_work() {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { tab, holder: _holder, dir: _dir, deck_dir: _deck_dir, pack: _pack, h: _h } =
        read_only_pack_tab(data.path());
    assert_that!(tab.widget_source(None, "card").unwrap()).is_equal_to(CARD_V2.to_owned());
}

#[test]
fn read_only_pack_tab_takes_the_lease_once_freed() {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { mut tab, holder, pack, dir: _dir, deck_dir: _deck_dir, h: _h } = read_only_pack_tab(data.path());
    drop(holder);
    let saved = tab.save_pack(false).unwrap();
    assert_that!(tab.library.current(pack)).is_equal_to(Some(saved));
    assert_that!(pack_leased(&scope_of(&tab), pack)).is_equal_to(Some(label(&edit_of(&tab).name)));
}

/// The state `snapshot_reports_read_only_and_leased` reads.
#[derive(Debug, Clone, Copy)]
enum Flags {
    Held,
    HeldElsewhere,
    HolderClosed,
    AfterASave,
    Retaken,
    Orphaned,
    DeckOnlyWorkshop,
    WithoutDirs,
    BaseFilesAbsent,
}

/// An editor in the state `state`, with whatever must outlive it.
fn editor_in(state: Flags, data: &std::path::Path) -> (Session, Vec<Box<dyn std::any::Any>>) {
    let keep = |r: ReadOnly| -> (Session, Vec<Box<dyn std::any::Any>>) {
        let ReadOnly { tab, holder, dir, deck_dir, pack: _pack, h: _h } = r;
        (tab, vec![Box::new(holder), Box::new(dir), Box::new(deck_dir)])
    };
    match state {
        Flags::Held => {
            let (mut s, deck, acme) = opener_on(data);
            let t = pack_tab_from(&mut s, data, acme);
            (t, vec![Box::new(s), Box::new(deck)])
        }
        Flags::HeldElsewhere => keep(read_only_pack_tab(data)),
        Flags::HolderClosed | Flags::AfterASave | Flags::Retaken | Flags::BaseFilesAbsent => {
            let ReadOnly { mut tab, holder, dir, deck_dir, pack: _pack, h: _h } = read_only_pack_tab(data);
            drop(holder);
            match state {
                Flags::AfterASave => drop(tab.save_pack(false).unwrap()),
                Flags::Retaken => tab.apply("x", vec![rename_title("acme 4")], None).unwrap(),
                Flags::BaseFilesAbsent => {
                    let base = edit_of(&tab).base;
                    tab.doc.sources.packs.remove(&base);
                }
                _ => {}
            }
            (tab, vec![Box::new(dir), Box::new(deck_dir)])
        }
        Flags::Orphaned => {
            let dormant = dormant_editor(data, PackHost::Tab);
            delete_from_library(data, dormant.pack);
            let mut tab = session_on(data);
            assert_that!(tab.resume_checked(dormant.dir.path().to_path_buf())).is_equal_to(Resumed::Restored);
            (tab, vec![Box::new(dormant)])
        }
        Flags::DeckOnlyWorkshop => {
            let mut s = session_on(data);
            let d = crate::test_support::adopt_deck_only_pack(&mut s);
            s.edit_pack(d.id).unwrap();
            (s, Vec::new())
        }
        Flags::WithoutDirs => {
            let mut s = session_on(data);
            s.library = PackLibrary::new(None, None, s.themes.clone(), s.storage.clone());
            s.edit_pack(BASE_PACK).unwrap();
            (s, Vec::new())
        }
    }
}

#[rstest]
#[case::held(Flags::Held, (false, true))]
#[case::held_elsewhere(Flags::HeldElsewhere, (true, false))]
#[case::holder_closed(Flags::HolderClosed, (false, false))]
#[case::after_a_save(Flags::AfterASave, (false, true))]
#[case::retaken(Flags::Retaken, (false, true))]
#[case::orphaned(Flags::Orphaned, (true, false))]
#[case::deck_only_workshop(Flags::DeckOnlyWorkshop, (false, true))]
#[case::without_dirs(Flags::WithoutDirs, (false, true))]
#[case::base_files_absent(Flags::BaseFilesAbsent, (true, false))]
fn snapshot_reports_read_only_and_leased(#[case] state: Flags, #[case] expected: (bool, bool)) {
    let data = fresh_dir("oku-slides-read-only-");
    let (tab, _kept) = editor_in(state, data.path());
    assert_that!(flags(&tab)).is_equal_to(expected);
    let pack = edit_of(&tab).pack;
    match state {
        // The snapshot takes no lease: a third editor still can.
        Flags::HolderClosed => {
            assert_that!(PackLease::acquire(&scope_of(&tab), pack, "p", "probe").is_ok()).is_true();
        }
        Flags::AfterASave => {
            assert_that!(pack_leased(&scope_of(&tab), pack)).is_equal_to(Some(label(&edit_of(&tab).name)));
        }
        _ => {}
    }
}

#[test]
fn snapshot_display_name_is_a_label() {
    let data = fresh_dir("oku-slides-read-only-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let mut t = pack_tab_from(&mut s, data.path(), acme);
    t.apply("x", vec![rename_title("acme\n\u{2069}\"x")], None).unwrap();
    t.save_pack(false).unwrap();
    let view = t.editing_view().unwrap();
    assert_that!((view.name.clone(), view.display_name))
        .is_equal_to(("acme\n\u{2069}\"x".to_owned(), "acme \u{02BC}x".to_owned()));
}

#[test]
fn read_only_pack_tab_resume_never_freezes_the_holders_draft() {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { tab, h, pack, holder: _holder, dir: _dir, deck_dir: _deck_dir } = read_only_pack_tab(data.path());
    let session = edit_of(&tab).session_version;
    assert_that!(session.is_some()).is_true();
    assert_that!(draft_of(&tab, pack, &h)).is_equal_to(DraftWrite::Replace(h));
}

#[test]
fn read_only_pack_tab_freezes_its_base_when_it_takes_the_lease() {
    let data = fresh_dir("oku-slides-read-only-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let dir = fresh_dir("oku-slides-read-only-tab-");
    let mut t = pack_tab_from(&mut s, data.path(), acme);
    let base = edit_of(&t).base;
    assert_that!(t.resume_fresh(dir.path().to_path_buf())).is_true();
    drop(t);
    let mut holder = session_on(data.path());
    holder.edit_pack(acme).unwrap();
    unfreeze(data.path(), acme, &base);
    let mut tab = session_on(data.path());
    assert_that!(tab.resume_checked(dir.path().to_path_buf())).is_equal_to(Resumed::Restored);
    assert_that!(draft_of(&tab, acme, &base)).is_equal_to(DraftWrite::Replace(base.clone()));
    drop(holder);
    tab.apply("x", vec![rename_title("acme 5")], None).unwrap();
    assert_that!(draft_of(&tab, acme, &base)).is_equal_to(DraftWrite::Child(base));
}

#[test]
fn adopt_session_version_takes_a_free_lease() {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { mut tab, holder, pack, dir: _dir, deck_dir: _deck_dir, h: _h } = read_only_pack_tab(data.path());
    drop(holder);
    tab.save_pack(false).unwrap();
    assert_that!(matches!(&tab.mode, Mode::Pack { lease: Some(l), .. } if l.pack() == pack)).is_true();
}

#[test]
fn adopt_session_version_of_a_pack_held_elsewhere_is_refused() {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { mut tab, pack, holder: _holder, dir: _dir, deck_dir: _deck_dir, h: _h } =
        read_only_pack_tab(data.path());
    let listed = versions(&tab, pack);
    assert_that!(tab.save_pack(false).unwrap_err().kind()).is_equal_to(SessionErrorKind::PackOpenInTab);
    assert_that!(versions(&tab, pack)).is_equal_to(listed);
}

#[test]
fn adopt_session_version_with_a_foreign_manifest_id_is_refused() {
    let data = fresh_dir("oku-slides-read-only-");
    let (mut s, _deck, acme) = opener_on(data.path());
    let mut t = pack_tab_from(&mut s, data.path(), acme);
    let base = edit_of(&t).base;
    let files = t.doc.sources.packs[&base].clone();
    let mut forged = crate::pack::PackManifest::from_files(&files).unwrap();
    forged.id = Uuid::new_v4();
    let forged_files = forged.to_files(files.kind(), files.widgets().clone()).unwrap();
    t.doc.sources.packs.insert(base, forged_files);
    let listed = versions(&t, acme);
    let err = t.set_widget(None, "card", CARD_V2.into()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::LeaseMissing);
    assert_that!(err.localize(&t.localizer))
        .is_equal_to("Ce pack ne peut être enregistré que par l'onglet qui l'édite.".to_owned());
    assert_that!((versions(&t, acme), t.library.current(forged.id))).is_equal_to((listed, None));
}

/// The witness `library_writers_refuse_a_missing_witness` gives.
#[derive(Debug, Clone, Copy)]
enum Witness {
    None,
    OtherPack,
    OtherScope,
    Deleted,
}

/// A new version of `pack` (its current one renamed) the library does not have.
fn unwritten_version(s: &Session, pack: Uuid) -> oku_slides_core::PackFiles {
    let current = s.library.current_version(pack).unwrap();
    let mut m = current.manifest.clone();
    m.name = "acme jamais".into();
    m.parent = Some(current.version.clone());
    let files = m.to_files(current.files.kind(), current.files.widgets().clone()).unwrap();
    assert_that!(s.library.version(pack, &files.version())).is_none();
    files
}

#[rstest]
#[case::write_none(true, Witness::None)]
#[case::write_other_pack(true, Witness::OtherPack)]
#[case::write_other_scope(true, Witness::OtherScope)]
#[case::write_deleted(true, Witness::Deleted)]
#[case::restore_none(false, Witness::None)]
#[case::restore_other_pack(false, Witness::OtherPack)]
#[case::restore_other_scope(false, Witness::OtherScope)]
#[case::restore_deleted(false, Witness::Deleted)]
fn library_writers_refuse_a_missing_witness(#[case] write: bool, #[case] witness: Witness) {
    let data = fresh_dir("oku-slides-read-only-");
    let (s, _deck, acme) = opener_on(data.path());
    let files = unwritten_version(&s, acme);
    let other = fresh_dir("oku-slides-other-scope-");
    let lease = match witness {
        Witness::None => None,
        Witness::OtherPack => Some(PackLease::acquire(&scope_of(&s), Uuid::new_v4(), "q", "q").unwrap()),
        Witness::OtherScope => {
            let scope = LeaseScope::new(&other.path().join("widgets"), &other.path().join("layouts"));
            Some(PackLease::acquire(&scope, acme, "acme", "acme").unwrap())
        }
        Witness::Deleted => {
            let lease = PackLease::acquire(&scope_of(&s), acme, "acme", "acme").unwrap();
            s.library.delete(acme).unwrap();
            Some(lease)
        }
    };
    let expected = if matches!(witness, Witness::Deleted) {
        SessionErrorKind::UnknownPack
    } else {
        SessionErrorKind::LeaseMissing
    };
    if write {
        let err = write_session_version(&s.library, &draft_lock(), acme, &DraftWrite::New, &files, lease.as_ref())
            .unwrap_err();
        assert_that!(err.kind()).is_equal_to(expected);
        if expected == SessionErrorKind::LeaseMissing {
            assert_that!(err.localize(&s.localizer))
                .is_equal_to("Ce pack ne peut être enregistré que par l'onglet qui l'édite.".to_owned());
        }
    } else {
        restore_session_version(&s.library, acme, &files, lease.as_ref());
    }
    assert_that!(s.library.version(acme, &files.version())).is_none();
}

#[test]
fn library_writer_refusal_changes_nothing() {
    let data = fresh_dir("oku-slides-read-only-");
    let (s, _deck, acme) = opener_on(data.path());
    let files = unwritten_version(&s, acme);
    let writes = library_writes();
    let refused = write_session_version(&s.library, &draft_lock(), acme, &DraftWrite::New, &files, None);
    assert_that!(refused.is_err()).is_true();
    assert_that!((library_writes(), s.library.version(acme, &files.version()).is_none())).is_equal_to((writes, true));
}

#[test]
fn read_only_pack_tab_save_of_a_deleted_pack_is_unknown_pack() {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { mut tab, holder, pack, dir: _dir, deck_dir: _deck_dir, h: _h } = read_only_pack_tab(data.path());
    drop(holder);
    delete_from_library(data.path(), pack);
    assert_that!(tab.save_pack(false).unwrap_err().kind()).is_equal_to(SessionErrorKind::UnknownPack);
    assert_that!(tab.library.current(pack)).is_none();
}

/// A pack tab restored orphaned: `acme` was deleted while it was dormant.
fn orphaned_tab(data: &std::path::Path) -> (Session, crate::test_support_packs::Dormant) {
    let dormant = dormant_editor(data, PackHost::Tab);
    delete_from_library(data, dormant.pack);
    let mut tab = session_on(data);
    assert_that!(tab.resume_checked(dormant.dir.path().to_path_buf())).is_equal_to(Resumed::Restored);
    (tab, dormant)
}

#[test]
fn orphaned_pack_tab_stays_refused_after_the_pack_comes_back() {
    let data = fresh_dir("oku-slides-read-only-");
    let (mut tab, dormant) = orphaned_tab(data.path());
    // A deck-mode write brings the pack back under its id.
    let manifest = crate::pack::PackManifest { id: dormant.pack, ..manifest_ns(dormant.pack, "acme", &[]) };
    add_version(&tab.library, &manifest, &[]);
    assert_that!(tab.library.current(dormant.pack).is_some()).is_true();
    assert_that!(tab.save_pack(false).unwrap_err().kind()).is_equal_to(SessionErrorKind::UnknownPack);
}

#[test]
fn orphaned_save_as_new_then_edits_the_copy() {
    let data = fresh_dir("oku-slides-read-only-");
    let (mut tab, _dormant) = orphaned_tab(data.path());
    tab.save_pack(true).unwrap();
    tab.set_widget(None, "card", CARD_V2.replace("strong", "underline")).unwrap();
    tab.save_pack(false).unwrap();
    assert_that!(flags(&tab)).is_equal_to((false, true));
}

#[test]
fn pack_deleted_while_the_tab_is_awake_makes_it_read_only() {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { mut tab, holder, pack, dir: _dir, deck_dir: _deck_dir, h: _h } = read_only_pack_tab(data.path());
    drop(holder);
    delete_from_library(data.path(), pack);
    assert_that!(flags(&tab)).is_equal_to((true, false));
    let err = tab.apply("x", vec![rename_title("acme 6")], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownPack);
    assert_that!(matches!(&tab.mode, Mode::Pack { orphaned: true, .. })).is_true();
}

#[test]
fn resume_v3_deck_only_workshop_stays_writable() {
    let data = fresh_dir("oku-slides-read-only-");
    let dir = fresh_dir("oku-slides-read-only-tab-");
    let mut s = session_on(data.path());
    assert_that!(s.resume_fresh(dir.path().to_path_buf())).is_true();
    let d = crate::test_support::adopt_deck_only_pack(&mut s);
    s.handle(Request::EditPack { pack: d.id }).unwrap();
    s.flush_resume();
    drop(s);
    crate::test_support::edit_state(dir.path(), |state| {
        assert_that!(state["mode"]["pack"].as_object_mut().and_then(|m| m.remove("host"))).is_some();
    });
    let mut back = session_on(data.path());
    assert_that!(back.resume_checked(dir.path().to_path_buf())).is_equal_to(Resumed::Restored);
    assert_that!((flags(&back), back.snapshot().notice.is_none())).is_equal_to(((false, true), true));
    back.apply("x", vec![rename_title("importé 2")], None).unwrap();
    back.save_pack(false).unwrap();
}

#[test]
fn delete_pack_of_its_own_pack_in_a_read_only_tab_is_refused_as_in_pack_tab() {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { mut tab, pack, holder: _holder, dir: _dir, deck_dir: _deck_dir, h: _h } =
        read_only_pack_tab(data.path());
    assert_that!(tab.handle(Request::DeletePack { pack }).unwrap_err().kind()).is_equal_to(SessionErrorKind::InPackTab);
}

#[test]
fn read_only_pack_tab_deletes_an_unrelated_pack() {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { mut tab, holder: _holder, dir: _dir, deck_dir: _deck_dir, pack: _pack, h: _h } =
        read_only_pack_tab(data.path());
    let other = Uuid::new_v4();
    add_version(&tab.library, &manifest_ns(other, "autre", &[]), &[]);
    tab.handle(Request::DeletePack { pack: other }).unwrap();
    assert_that!(tab.library.current(other)).is_none();
}

#[test]
fn read_only_workshop_close_pack_returns_to_its_deck() {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { mut tab, holder: _holder, dir: _dir, deck_dir: _deck_dir, pack: _pack, h: _h } =
        read_only_workshop(data.path());
    tab.handle(Request::ClosePack).unwrap();
    assert_that!(matches!(tab.mode, Mode::Deck)).is_true();
    assert_that!(tab.doc.path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()))
        .is_equal_to(Some(format!("opener{}", crate::EXTENSION)));
}

#[test]
fn read_only_workshop_can_switch_pack() {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { mut tab, holder: _holder, dir: _dir, deck_dir: _deck_dir, pack: _pack, h: _h } =
        read_only_workshop(data.path());
    tab.handle(Request::EditPack { pack: BASE_PACK }).unwrap();
    assert_that!(
        matches!(&tab.mode, Mode::Pack { edit, lease: Some(l), .. } if edit.pack == BASE_PACK && l.pack() == BASE_PACK)
    )
    .is_true();
}

#[test]
fn read_only_pack_tab_save_as_new_takes_the_copy_lease() {
    let data = fresh_dir("oku-slides-read-only-");
    let ReadOnly { mut tab, holder, pack, dir: _dir, deck_dir: _deck_dir, h: _h } = read_only_pack_tab(data.path());
    tab.save_pack(true).unwrap();
    let copy = edit_of(&tab).pack;
    assert_that!(matches!(&tab.mode, Mode::Pack { lease: Some(l), .. } if l.pack() == copy && copy != pack)).is_true();
    assert_that!(matches!(&holder.mode, Mode::Pack { lease: Some(l), .. } if l.pack() == pack)).is_true();
}
