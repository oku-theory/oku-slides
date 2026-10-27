use std::path::Path;

use oku_slides_core::PackVersion;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::super::{Resume, ResumeError};
use crate::pack::PackManifest;
use crate::resume::Resumed;
use crate::test_support::{adopt_deck_only_pack, edit_of, edit_state, fresh_dir, manifest_ns, session_on};
use crate::test_support_packs::{Dormant, Live, assert_unleased, live_editor, park, scope_of};
use crate::{Mode, PackHost, Request, Storage};

/// The refusal of restoring `dir` on `data`, from a session dropped right after.
fn refusal(data: &Path, dir: &Path) -> ResumeError {
    let mut s = session_on(data);
    let refused = s.restore(&Resume::new(Storage::local().unwrap(), dir.to_path_buf()), None);
    refused.err().unwrap_or_else(|| panic!("restore refused"))
}

/// Rewrites `dir`'s `sub/document.json` through `edit`.
fn edit_doc(dir: &Path, sub: &str, edit: impl FnOnce(&mut serde_json::Value)) {
    let path = dir.join(sub).join("document.json");
    let mut doc: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    edit(&mut doc);
    std::fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
}

/// A version no folder or library holds.
fn unknown_version() -> PackVersion {
    crate::test_support::files_of(&manifest_ns(Uuid::new_v4(), "inconnu", &[])).version()
}

/// The base of `live`'s working deck forged by `forge`, pinned and named in the state.
fn forge_base(live: &mut Live, forge: impl FnOnce(&mut PackManifest)) {
    let s = &mut live.editor;
    let base = edit_of(s).base;
    let files = s.doc.sources.packs[&base].clone();
    let mut manifest = PackManifest::from_files(&files).unwrap();
    forge(&mut manifest);
    let forged = manifest.to_files(files.kind(), files.widgets().clone()).unwrap();
    let version = forged.version();
    s.doc.sources.add_pack(forged);
    let mut deck = s.doc.sources.deck().clone();
    for pin in deck.widgets.values_mut().filter(|p| p.version == base) {
        pin.version = version.clone();
    }
    s.doc.sources.apply_silent(deck).unwrap();
    if let Mode::Pack { edit, .. } = &mut s.mode {
        edit.base = version;
        edit.session_version = None;
    }
}

/// A dormant pack tab on a deck-only pack: a blank deck's workshop marked as a pack tab.
fn deck_only_pack_tab(data: &Path) -> tempfile::TempDir {
    let dir = fresh_dir("oku-slides-refusal-");
    let mut s = session_on(data);
    assert_that!(s.resume_fresh(dir.path().to_path_buf())).is_true();
    let d = adopt_deck_only_pack(&mut s);
    s.handle(Request::EditPack { pack: d.id }).unwrap();
    s.flush_resume();
    drop(s);
    edit_state(dir.path(), |state| {
        state["version"] = json!(4);
        state["mode"]["pack"]["host"] = json!("tab");
    });
    dir
}

/// An inconsistency of `resume_inconsistent_pack_tab_is_set_aside`.
#[derive(Debug, Clone, Copy)]
enum Broken {
    PreviewPath,
    PreviewHash,
    PreviewAsset,
    DeckOnlyPack,
    KindDisagrees,
    DeckScopedLibraryId,
    ParentNotAncestor,
    AnotherPacksFiles,
    EditedPinElsewhere,
    UnreadablePack,
    UnreadablePreview,
    BaseMissing,
}

/// A dormant pack tab on `data` broken by `broken`: its folder, and its deck file's folder.
fn broken_tab(data: &Path, broken: Broken) -> (tempfile::TempDir, Option<tempfile::TempDir>) {
    if let Broken::DeckOnlyPack = broken {
        return (deck_only_pack_tab(data), None);
    }
    let mut live = live_editor(data, PackHost::Tab);
    match broken {
        Broken::DeckScopedLibraryId => forge_base(&mut live, |m| m.scope = oku_slides_core::PackScope::Deck),
        Broken::EditedPinElsewhere => {
            let card = Request::SetWidget {
                pack: None,
                name: "card".into(),
                source: crate::test_support_packs::CARD_V2.into(),
            };
            live.editor.handle(card).unwrap();
        }
        _ => {}
    }
    let parent = edit_of(&live.editor).parent;
    let opener_file = live.deck_dir.path().join(format!("opener{}", crate::EXTENSION));
    let Dormant { dir, deck_dir, .. } = park(live);
    let p = dir.path();
    match broken {
        Broken::PreviewPath => edit_doc(p, "deck", |d| d["path"] = json!(opener_file)),
        Broken::PreviewHash => edit_doc(p, "deck", |d| d["saved_hash"] = json!("sha256:00")),
        Broken::PreviewAsset => {
            std::fs::create_dir_all(p.join("deck").join("assets")).unwrap();
            std::fs::write(p.join("deck").join("assets").join("ab12"), b"image").unwrap();
        }
        Broken::KindDisagrees => edit_state(p, |s| s["mode"]["pack"]["kind"] = json!("layouts")),
        Broken::ParentNotAncestor => edit_state(p, |s| s["mode"]["pack"]["parent"] = json!(unknown_version().as_str())),
        Broken::AnotherPacksFiles => {
            let base = crate::test_support::builtin(crate::BASE_PACK).version.clone();
            edit_state(p, |s| s["mode"]["pack"]["base"] = json!(base.as_str()));
        }
        Broken::EditedPinElsewhere => {
            let old = parent.clone().unwrap_or_else(|| panic!("a parent"));
            assert_that!(old).is_not_equal_to(edit_of_state(p).0);
            edit_state(p, |s| {
                s["mode"]["pack"]["base"] = json!(old.as_str());
                s["mode"]["pack"]["session_version"] = serde_json::Value::Null;
            });
        }
        Broken::UnreadablePack => std::fs::write(p.join("pack").join("deck.loro"), b"illisible").unwrap(),
        Broken::UnreadablePreview => std::fs::write(p.join("deck").join("deck.loro"), b"illisible").unwrap(),
        Broken::BaseMissing => edit_state(p, |s| s["mode"]["pack"]["base"] = json!(unknown_version().as_str())),
        Broken::DeckOnlyPack | Broken::DeckScopedLibraryId => {}
    }
    (dir, Some(deck_dir))
}

/// The base and session version a dormant state names.
fn edit_of_state(dir: &Path) -> (PackVersion, Option<PackVersion>) {
    let state: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("state.json")).unwrap()).unwrap();
    let version = |v: &serde_json::Value| v.as_str().and_then(PackVersion::parse);
    (version(&state["mode"]["pack"]["base"]).unwrap(), version(&state["mode"]["pack"]["session_version"]))
}

/// Whether `e` is the refusal `broken` expects.
fn is_expected(e: &ResumeError, broken: Broken) -> bool {
    let reason = match broken {
        Broken::PreviewPath => "preview with a path",
        Broken::PreviewHash => "preview with a saved hash",
        Broken::PreviewAsset => "preview with assets",
        Broken::DeckOnlyPack => "deck-only pack in a pack tab",
        Broken::KindDisagrees => "kind differs",
        Broken::DeckScopedLibraryId => "deck-scoped base of a library pack",
        Broken::ParentNotAncestor => "parent not an ancestor of the base",
        Broken::AnotherPacksFiles => "base of another pack",
        Broken::EditedPinElsewhere => "edited pin is neither the base nor the saved version",
        Broken::UnreadablePack => "unreadable pack",
        Broken::UnreadablePreview => "unreadable preview",
        Broken::BaseMissing => "base missing",
    };
    e.pack_tab_reason() == Some(reason)
}

#[rstest]
#[case::preview_path(Broken::PreviewPath)]
#[case::preview_hash(Broken::PreviewHash)]
#[case::preview_asset(Broken::PreviewAsset)]
#[case::deck_only_pack(Broken::DeckOnlyPack)]
#[case::kind_disagrees(Broken::KindDisagrees)]
#[case::deck_scoped_library_id(Broken::DeckScopedLibraryId)]
#[case::parent_not_ancestor(Broken::ParentNotAncestor)]
#[case::another_packs_files(Broken::AnotherPacksFiles)]
#[case::edited_pin_elsewhere(Broken::EditedPinElsewhere)]
#[case::unreadable_pack(Broken::UnreadablePack)]
#[case::unreadable_preview(Broken::UnreadablePreview)]
#[case::base_missing(Broken::BaseMissing)]
fn resume_inconsistent_pack_tab_is_set_aside(#[case] broken: Broken) {
    let data = fresh_dir("oku-slides-refusal-");
    let (dir, _deck) = broken_tab(data.path(), broken);
    let e = refusal(data.path(), dir.path());
    assert_that!(is_expected(&e, broken)).named(&format!("{e}")).is_true();
    let mut back = session_on(data.path());
    assert_that!(back.resume_checked(dir.path().to_path_buf(), None)).is_equal_to(Resumed::Unreadable);
    assert_that!(back.tab_info().title).is_not_equal_to("opener".to_owned());
}

#[test]
fn resume_refused_pack_tab_leaves_a_blank_deck() {
    let data = fresh_dir("oku-slides-refusal-");
    let (dir, _deck) = broken_tab(data.path(), Broken::PreviewHash);
    let mut back = session_on(data.path());
    assert_that!(back.resume_checked(dir.path().to_path_buf(), None)).is_equal_to(Resumed::Unreadable);
    assert_that!((matches!(back.mode, Mode::Deck), back.doc.blank)).is_equal_to((true, true));
}

#[test]
fn resume_refused_pack_tab_leaves_no_lease() {
    let data = fresh_dir("oku-slides-refusal-");
    let (dir, _deck) = broken_tab(data.path(), Broken::BaseMissing);
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("state.json")).unwrap()).unwrap();
    let pack: Uuid = serde_json::from_value(state["mode"]["pack"]["pack"].clone()).unwrap();
    let mut back = session_on(data.path());
    assert_that!(back.resume_checked(dir.path().to_path_buf(), None)).is_equal_to(Resumed::Unreadable);
    assert_unleased(&scope_of(&back), pack);
}
