use std::path::Path;

use oku_slides_core::{PackKind, PackVersion};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::library::DraftWrite;
use crate::pack::PackManifest;
use crate::pack_lease::pack_leased;
use crate::resume::Resumed;
use crate::test_support::{add_version, edit_of, edit_state, fresh_dir, manifest_ns, rename_title, session_on};
use crate::test_support_packs::{
    CARD_V2, Dormant, Live, acme_of, adopting_deck, delete_from_library, dormant_editor, dormant_workshop, draft_of,
    live_editor, live_workshop, park, scope_of, unfreeze,
};
use crate::{Mode, Session, SessionErrorKind};

/// A new session on `data` resuming `dir`, and what it gave.
fn resumed(data: &Path, dir: &Path) -> (Session, Resumed) {
    let mut s = session_on(data);
    let result = s.resume_checked(dir.to_path_buf(), None);
    (s, result)
}

/// The read-only and leased flags of `s`.
fn flags(s: &Session) -> (bool, bool) {
    let view = s.editing_view().unwrap_or_else(|| panic!("pack mode"));
    (view.read_only, view.leased)
}

/// The versions of `pack` the library of `s` has.
fn versions(s: &Session, pack: Uuid) -> Vec<PackVersion> {
    s.library.versions(pack).into_iter().map(|v| v.version).collect()
}

/// The pack `s` edits, if any.
fn editing(s: &Session) -> Option<Uuid> {
    crate::test_support_packs::editing(s)
}

/// The parent of `version` of `pack` in the library of `s`.
fn parent_of(s: &Session, pack: Uuid, version: &PackVersion) -> Option<PackVersion> {
    PackManifest::from_files(&s.library.version(pack, version).unwrap()).unwrap().parent
}

/// A version no folder or library holds.
fn unknown_version() -> PackVersion {
    crate::test_support::files_of(&manifest_ns(Uuid::new_v4(), "inconnu", &[])).version()
}

/// A deck tab persisted in `dir` on `data` after `work`, then gone.
fn dormant_deck(data: &Path, work: impl FnOnce(&mut Session)) -> tempfile::TempDir {
    let dir = fresh_dir("oku-slides-resume-pack-");
    let mut s = session_on(data);
    assert_that!(s.resume_fresh(dir.path().to_path_buf())).is_true();
    work(&mut s);
    s.persist(true);
    s.flush_resume();
    dir
}

#[test]
fn resume_v3_deck_state_with_unsaved_work_reopens_unchanged() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dir = dormant_deck(data.path(), |s| s.apply("x", vec![rename_title("Gardé")], None).unwrap());
    let (back, result) = resumed(data.path(), dir.path());
    assert_that!(result).is_equal_to(Resumed::Restored);
    assert_that!((back.deck().params.title.clone(), back.doc.dirty)).is_equal_to((Some("Gardé".to_owned()), true));
}

#[test]
fn resume_v3_help_state_reopens_unchanged() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dir = dormant_deck(data.path(), |s| s.open_help("fr", None).unwrap());
    let (back, result) = resumed(data.path(), dir.path());
    assert_that!((result, back.help())).is_equal_to((Resumed::Restored, Some("fr")));
}

#[test]
fn resume_v3_workshop_state_keeps_its_deck_dirty() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_workshop(data.path());
    let (back, result) = resumed(data.path(), dormant.dir.path());
    assert_that!((result, matches!(back.mode, Mode::Deck), back.doc.dirty)).is_equal_to((
        Resumed::Restored,
        true,
        true,
    ));
}

/// A dormant editor restored while another session holds its pack's lease.
///
/// A pack tab resumes read-only; a workshop reopens on its deck.
struct HeldElsewhere {
    back: Session,
    result: Resumed,
    holder: Session,
    _data: tempfile::TempDir,
    dormant: Dormant,
}

/// The `HeldElsewhere` fixture, its editor made by `dormant_of`.
fn held_elsewhere(dormant_of: fn(&Path) -> Dormant) -> HeldElsewhere {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_of(data.path());
    let holder = crate::test_support_packs::pack_tab_of(data.path(), dormant.pack);
    let (back, result) = resumed(data.path(), dormant.dir.path());
    HeldElsewhere { back, result, holder, _data: data, dormant }
}

/// The holder's label of record: its pack tab's title.
fn holder_label(h: &HeldElsewhere) -> String {
    h.holder.tab_info().title
}

#[test]
fn resume_v3_workshop_of_a_leased_pack_keeps_the_holders_lease() {
    let h = held_elsewhere(dormant_workshop);
    assert_that!(h.result).is_equal_to(Resumed::Restored);
    assert_that!(pack_leased(&scope_of(&h.back), h.dormant.pack)).is_equal_to(Some(holder_label(&h)));
}

#[test]
fn resume_legacy_pack_tab_state_claiming_host_deck_is_set_aside() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_editor(data.path());
    edit_state(dormant.dir.path(), |state| {
        state["version"] = json!(crate::resume::PACK_TAB_STATE_VERSION);
        state["mode"]["pack"]["host"] = json!("deck");
    });
    let (back, result) = resumed(data.path(), dormant.dir.path());
    assert_that!((result, matches!(back.mode, Mode::Deck))).is_equal_to((Resumed::Unreadable, true));
}

#[test]
fn resume_pack_tab_reopens_as_a_pack_tab() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_editor(data.path());
    let (back, result) = resumed(data.path(), dormant.dir.path());
    assert_that!((result, editing(&back))).is_equal_to((Resumed::Restored, Some(dormant.pack)));
}

#[test]
fn resume_pack_tab_restores_its_ui() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let mut live = live_editor(data.path());
    live.editor.select_item("card").unwrap();
    let dormant = park(live);
    let (back, _) = resumed(data.path(), dormant.dir.path());
    assert_that!(back.ui.widget.as_deref()).is_equal_to(Some("card"));
}

#[test]
fn resume_pack_tab_holds_its_lease() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_editor(data.path());
    let (back, _) = resumed(data.path(), dormant.dir.path());
    assert_that!(pack_leased(&scope_of(&back), dormant.pack)).is_equal_to(Some("acme".to_owned()));
}

#[test]
fn resume_pack_tab_of_a_pack_held_elsewhere_reopens_without_a_lease() {
    let mut h = held_elsewhere(dormant_editor);
    assert_that!((h.result, h.back.is_pack_tab())).is_equal_to((Resumed::Restored, true));
    assert_that!(h.back.doc.notice.is_some()).is_true();
    h.holder.apply("x", vec![rename_title("acme holder")], None).unwrap();
    let theirs = h.holder.save_pack(false).unwrap();
    assert_that!(h.back.save_pack(false).unwrap_err().kind()).is_equal_to(SessionErrorKind::PackOpenInTab);
    assert_that!(h.back.library.current(h.dormant.pack)).is_equal_to(Some(theirs));
}

#[test]
fn resume_pack_tab_never_freezes_its_draft() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_editor(data.path());
    let (mut back, _) = resumed(data.path(), dormant.dir.path());
    let draft = edit_of(&back).session_version.unwrap();
    assert_that!(draft_of(&back, dormant.pack, &draft)).is_equal_to(DraftWrite::Replace(draft.clone()));
    back.apply("x", vec![rename_title("acme 7")], None).unwrap();
    let next = back.save_pack(false).unwrap();
    assert_that!(parent_of(&back, dormant.pack, &next)).is_equal_to(edit_of(&back).parent);
    assert_that!(edit_of(&back).parent).is_not_equal_to(Some(draft));
}

#[test]
fn resume_pack_tab_unsaved_refreezes_its_base() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let live = live_editor(data.path());
    let base = edit_of(&live.editor).base;
    let dormant = park(live);
    unfreeze(data.path(), dormant.pack, &base);
    let (back, _) = resumed(data.path(), dormant.dir.path());
    assert_that!(draft_of(&back, dormant.pack, &base)).is_equal_to(DraftWrite::Child(base));
}

#[test]
fn resume_pack_tab_with_a_tampered_base_blob_is_set_aside() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_editor(data.path());
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dormant.dir.path().join("state.json")).unwrap()).unwrap();
    let base = PackVersion::parse(state["mode"]["pack"]["base"].as_str().unwrap()).unwrap();
    let blob = dormant.dir.path().join("pack").join("packs").join(format!("{}.json", base.hex()));
    let mut json: serde_json::Value = serde_json::from_slice(&std::fs::read(&blob).unwrap()).unwrap();
    json["name"] = json!("altéré");
    std::fs::write(&blob, serde_json::to_vec(&json).unwrap()).unwrap();
    let before = versions(&session_on(data.path()), dormant.pack);
    let (back, result) = resumed(data.path(), dormant.dir.path());
    assert_that!((result, versions(&back, dormant.pack))).is_equal_to((Resumed::Unreadable, before));
}

#[test]
fn resume_pack_tab_is_not_reported_unsaved_after_a_save() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let mut live = live_editor(data.path());
    live.editor.apply("x", vec![rename_title("acme 8")], None).unwrap();
    live.editor.save_pack(false).unwrap();
    let dormant = park(live);
    let (back, _) = resumed(data.path(), dormant.dir.path());
    assert_that!(back.tab_info().dirty).is_false();
}

#[test]
fn resume_pack_tab_takes_the_library_name() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_editor(data.path());
    edit_state(dormant.dir.path(), |state| state["mode"]["pack"]["name"] = json!("forgé"));
    let (back, _) = resumed(data.path(), dormant.dir.path());
    assert_that!(edit_of(&back).name).is_equal_to("acme".to_owned());
}

#[test]
fn resume_pack_tab_ignores_a_saved_deck_only() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_editor(data.path());
    edit_state(dormant.dir.path(), |state| state["mode"]["pack"]["deck_only"] = json!(true));
    let (back, result) = resumed(data.path(), dormant.dir.path());
    assert_that!((result, edit_of(&back).deck_only)).is_equal_to((Resumed::Restored, false));
    assert_that!(pack_leased(&scope_of(&back), dormant.pack).is_some()).is_true();
}

#[test]
fn resume_pack_tab_keeps_a_copy_session_version() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let mut live = live_editor(data.path());
    let copy = live.editor.save_pack(true).unwrap();
    let dormant = park(live);
    let (mut back, _) = resumed(data.path(), dormant.dir.path());
    let id = edit_of(&back).pack;
    assert_that!(edit_of(&back).session_version).is_equal_to(Some(copy.clone()));
    back.apply("x", vec![rename_title("copie 2")], None).unwrap();
    let next = back.save_pack(false).unwrap();
    // Replaced, not a child: the copy's next version has no parent either.
    assert_that!(parent_of(&back, id, &next)).is_none();
    assert_that!(back.library.current(id)).is_equal_to(Some(next));
}

#[test]
fn resume_pack_tab_accepts_a_parent_ancestor_of_its_base() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_editor(data.path());
    let s = session_on(data.path());
    let oldest = s.library.versions(dormant.pack).into_iter().find(|v| v.manifest.parent.is_none()).unwrap().version;
    edit_state(dormant.dir.path(), |state| state["mode"]["pack"]["parent"] = json!(oldest.as_str()));
    let (_back, result) = resumed(data.path(), dormant.dir.path());
    assert_that!(result).is_equal_to(Resumed::Restored);
}

#[test]
fn resume_pack_tab_after_undo_to_the_parent_is_restored() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let mut live = live_editor(data.path());
    live.editor.set_widget(None, "card", CARD_V2.into()).unwrap();
    live.editor.undo().unwrap();
    assert_that!(edit_of(&live.editor).session_version).is_none();
    let dormant = park(live);
    let (_back, result) = resumed(data.path(), dormant.dir.path());
    assert_that!(result).is_equal_to(Resumed::Restored);
}

#[rstest]
#[case::unknown(false)]
#[case::sibling_draft(true)]
fn resume_pack_tab_clears_a_foreign_session_version(#[case] sibling: bool) {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_editor(data.path());
    let foreign = if sibling {
        let mut other = crate::test_support_packs::pack_tab_of(data.path(), dormant.pack);
        other.apply("x", vec![rename_title("acme sœur")], None).unwrap();
        let draft = other.save_pack(false).unwrap();
        drop(other);
        draft
    } else {
        unknown_version()
    };
    edit_state(dormant.dir.path(), |state| state["mode"]["pack"]["session_version"] = json!(foreign.as_str()));
    let (mut back, result) = resumed(data.path(), dormant.dir.path());
    assert_that!((result, edit_of(&back).session_version)).is_equal_to((Resumed::Restored, None));
    if sibling {
        back.apply("x", vec![rename_title("acme 9")], None).unwrap();
        back.save_pack(false).unwrap();
        assert_that!(back.library.version(dormant.pack, &foreign).is_some()).is_true();
    }
}

#[test]
fn resume_pack_tab_ignores_dot_files_in_assets() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_editor(data.path());
    let assets = dormant.dir.path().join("deck").join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(assets.join(".DS_Store"), b"x").unwrap();
    let (_back, result) = resumed(data.path(), dormant.dir.path());
    assert_that!(result).is_equal_to(Resumed::Restored);
}

#[test]
fn resume_workshop_with_an_unreadable_pack_falls_back_to_its_deck() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_workshop(data.path());
    std::fs::write(dormant.dir.path().join("pack").join("deck.loro"), b"illisible").unwrap();
    let (back, result) = resumed(data.path(), dormant.dir.path());
    assert_that!((result, matches!(back.mode, Mode::Deck))).is_equal_to((Resumed::Restored, true));
    let notice = back.localizer.tr("notice-workshop-reopened-on-deck");
    assert_that!(back.snapshot().notice).is_equal_to(Some(notice.as_str()));
}

/// A dormant pack tab whose pack was deleted meanwhile, and recreated with other versions if `recreated`.
fn orphan(recreated: bool) -> (tempfile::TempDir, Dormant, Session, Resumed) {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_editor(data.path());
    delete_from_library(data.path(), dormant.pack);
    if recreated {
        let other = session_on(data.path());
        add_version(&other.library, &manifest_ns(dormant.pack, "acme", &[]), &[]);
    }
    let (back, result) = resumed(data.path(), dormant.dir.path());
    (data, dormant, back, result)
}

#[rstest]
#[case::tab_deleted(false)]
#[case::tab_recreated(true)]
fn resume_of_a_deleted_pack_is_restored(#[case] recreated: bool) {
    let (_data, _dormant, _back, result) = orphan(recreated);
    assert_that!(result).is_equal_to(Resumed::Restored);
}

#[rstest]
#[case::tab_deleted(false)]
#[case::tab_recreated(true)]
fn resume_of_a_deleted_pack_is_orphaned_read_only(#[case] recreated: bool) {
    let (_data, _dormant, back, _) = orphan(recreated);
    assert_that!(flags(&back)).is_equal_to((true, false));
}

#[rstest]
#[case::tab_deleted(false)]
#[case::tab_recreated(true)]
fn resume_of_a_deleted_pack_sets_the_deleted_notice(#[case] recreated: bool) {
    let (_data, _dormant, back, _) = orphan(recreated);
    let expected = back.localizer.tr_with("notice-pack-deleted", &[("pack", "acme".into())]);
    assert_that!(expected.as_str()).starts_with("Le pack acme n'est plus dans la bibliothèque");
    assert_that!(back.snapshot().notice).is_equal_to(Some(expected.as_str()));
}

#[rstest]
#[case::tab_deleted(false)]
#[case::tab_recreated(true)]
fn resume_of_a_deleted_pack_refuses_saves_with_unknown_pack(#[case] recreated: bool) {
    let (_data, _dormant, mut back, _) = orphan(recreated);
    assert_that!(back.save_pack(false).unwrap_err().kind()).is_equal_to(SessionErrorKind::UnknownPack);
}

#[rstest]
#[case::tab_deleted(false)]
#[case::tab_recreated(true)]
fn resume_of_a_deleted_pack_accepts_save_as_new(#[case] recreated: bool) {
    let (_data, dormant, mut back, _) = orphan(recreated);
    let copy = back.save_pack(true).unwrap();
    let id = edit_of(&back).pack;
    assert_that!(id).is_not_equal_to(dormant.pack);
    assert_that!(back.library.current(id)).is_equal_to(Some(copy));
}

/// Forges the base of `live`'s working deck: a version of `manifest` changed by `forge`, of `kind`,
/// pinned by the working deck and named the base in the state.
fn forge_base(live: &mut Live, kind: PackKind, forge: impl FnOnce(&mut PackManifest)) -> PackVersion {
    let s = &mut live.editor;
    let base = edit_of(s).base;
    let files = s.doc.sources.packs[&base].clone();
    let mut manifest = PackManifest::from_files(&files).unwrap();
    forge(&mut manifest);
    let forged = manifest.to_files(kind, files.widgets().clone()).unwrap();
    let version = forged.version();
    s.doc.sources.add_pack(forged);
    let mut deck = s.doc.sources.deck().clone();
    for pins in [&mut deck.widgets, &mut deck.layouts] {
        for pin in pins.values_mut().filter(|p| p.version == base) {
            pin.version = version.clone();
        }
    }
    s.doc.sources.apply_silent(deck).unwrap();
    if let Mode::Pack { edit, .. } = &mut s.mode {
        edit.base = version.clone();
        edit.session_version = None;
        edit.kind = kind;
    }
    version
}

/// What `resume_v3_workshop_with_a_foreign_pack_falls_back_to_its_deck` forges.
#[derive(Debug, Clone, Copy)]
enum Foreign {
    Id,
    Kind,
}

#[rstest]
#[case::id(Foreign::Id)]
#[case::kind(Foreign::Kind)]
fn resume_v3_workshop_with_a_foreign_pack_falls_back_to_its_deck(#[case] foreign: Foreign) {
    let data = fresh_dir("oku-slides-resume-pack-");
    let dormant = dormant_workshop(data.path());
    match foreign {
        Foreign::Id => {
            let base = crate::test_support::builtin(crate::BASE_PACK).version.clone();
            edit_state(dormant.dir.path(), |state| state["mode"]["pack"]["base"] = json!(base.as_str()));
        }
        Foreign::Kind => edit_state(dormant.dir.path(), |state| state["mode"]["pack"]["kind"] = json!("layouts")),
    }
    let s = session_on(data.path());
    let before = (versions(&s, dormant.pack), versions(&s, crate::BASE_PACK));
    drop(s);
    let (back, result) = resumed(data.path(), dormant.dir.path());
    assert_that!((result, matches!(back.mode, Mode::Deck))).is_equal_to((Resumed::Restored, true));
    assert_that!((versions(&back, dormant.pack), versions(&back, crate::BASE_PACK))).is_equal_to(before);
}

/// What a resume gives a pack tab (set aside) and an older workshop (its deck back).
#[rstest]
#[case::tab(live_editor, Resumed::Unreadable)]
#[case::deck(live_workshop, Resumed::Restored)]
fn resume_with_a_deck_scoped_pack_of_a_library_id_is_refused(
    #[case] live_of: fn(&Path) -> Live,
    #[case] expected: Resumed,
) {
    let data = fresh_dir("oku-slides-resume-pack-");
    let mut live = live_of(data.path());
    forge_base(&mut live, PackKind::Widgets, |m| m.scope = oku_slides_core::PackScope::Deck);
    let dormant = park(live);
    let before = versions(&session_on(data.path()), dormant.pack);
    let (back, result) = resumed(data.path(), dormant.dir.path());
    // Set aside or fallen back: either way the session is in deck mode.
    assert_that!((result, matches!(back.mode, Mode::Deck))).is_equal_to((expected, true));
    assert_that!(versions(&back, dormant.pack)).is_equal_to(before);
}

#[test]
fn resume_v3_workshop_with_a_foreign_kind_falls_back_to_its_deck() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let mut live = live_workshop(data.path());
    forge_base(&mut live, PackKind::Layouts, |_| {});
    let dormant = park(live);
    let (back, result) = resumed(data.path(), dormant.dir.path());
    assert_that!((result, matches!(back.mode, Mode::Deck))).is_equal_to((Resumed::Restored, true));
}

#[test]
fn resume_pack_tab_preview_adopts_nothing() {
    let data = fresh_dir("oku-slides-resume-pack-");
    let mut s = session_on(data.path());
    let mut seeded = adopting_deck(&mut s, data.path());
    let acme = acme_of(&s);
    let v = s.user_deck().widgets.values().find(|p| p.id == acme).map(|p| p.version.clone()).unwrap();
    let dir = fresh_dir("oku-slides-resume-pack-tab-");
    let mut t = crate::test_support_packs::pack_tab_from(&mut seeded, data.path(), crate::BASE_PACK);
    assert_that!(t.resume_fresh(dir.path().to_path_buf())).is_true();
    t.flush_resume();
    drop(t);
    let (_back, result) = resumed(data.path(), dir.path());
    assert_that!(result).is_equal_to(Resumed::Restored);
    assert_that!(draft_of(&s, acme, &v)).is_equal_to(DraftWrite::Replace(v));
}
