use oku_slides_core::PackFiles;
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::deck_packs::deck_packs_tests::files;
use crate::guide_approvals::VersionRef;
use crate::guide_origin::GuideOrigin;
use crate::pack::{BASE_PACK, ESSENTIEL_PACK};
use crate::test_support::{builtin, fresh_dir, session_on};
use crate::test_support_packs::pack_tab_of;
use crate::{Request, Session};

/// A fresh session, and a pack version none of its libraries holds yet.
fn session_and_pack() -> (tempfile::TempDir, Session, Uuid, PackFiles) {
    let data = fresh_dir("oku-slides-origin-");
    let s = session_on(data.path());
    let id = Uuid::new_v4();
    let pack = files(id, "P", None, "2026-10-01T10:00:00Z");
    (data, s, id, pack)
}

#[rstest]
#[case::base(BASE_PACK)]
#[case::essentiel(ESSENTIEL_PACK)]
fn guide_origin_builtin_pack_is_builtin(#[case] id: Uuid) {
    let (_d, s, _, _) = session_and_pack();
    let version = builtin(id).version.clone();
    assert_that!(s.guide_origin(id, &version, false)).is_equal_to(GuideOrigin::Builtin);
}

#[test]
fn guide_origin_builtin_theme_is_builtin() {
    let (_d, s, _, _) = session_and_pack();
    let theme = s.themes.entries().into_iter().next().unwrap();
    assert_that!(s.guide_origin(theme.manifest.id, &theme.version, true)).is_equal_to(GuideOrigin::Builtin);
}

#[test]
fn guide_origin_version_saved_in_the_library_is_local() {
    let (_d, s, id, pack) = session_and_pack();
    s.library.add_version(id, &pack).unwrap();
    assert_that!(s.guide_origin(id, &pack.version(), false)).is_equal_to(GuideOrigin::Local);
}

#[test]
fn guide_origin_version_received_through_receive_version_is_received() {
    let (_d, s, id, pack) = session_and_pack();
    s.library.receive_version(id, &pack).unwrap();
    assert_that!(s.guide_origin(id, &pack.version(), false)).is_equal_to(GuideOrigin::Received);
}

#[test]
fn guide_origin_version_held_only_by_a_deck_is_received() {
    let (_d, s, id, pack) = session_and_pack();
    assert_that!(s.guide_origin(id, &pack.version(), false)).is_equal_to(GuideOrigin::Received);
}

#[test]
fn guide_active_received_version_is_inactive_until_approved() {
    let (_d, mut s, id, pack) = session_and_pack();
    s.library.receive_version(id, &pack).unwrap();
    let before = s.guide_active(id, &pack.version(), false);
    s.handle(Request::ApproveGuides { versions: vec![VersionRef { id, version: pack.version() }] }).unwrap();
    assert_that!((before, s.guide_active(id, &pack.version(), false))).is_equal_to((false, true));
}

#[test]
fn guide_active_local_and_builtin_versions_need_no_approval() {
    let (_d, s, id, pack) = session_and_pack();
    s.library.add_version(id, &pack).unwrap();
    let base = builtin(BASE_PACK).version.clone();
    assert_that!((s.guide_active(id, &pack.version(), false), s.guide_active(BASE_PACK, &base, false)))
        .is_equal_to((true, true));
}

#[test]
fn guides_view_of_a_pack_tab_names_its_version_origin_and_approval() {
    let data = fresh_dir("oku-slides-origin-");
    let mut s = pack_tab_of(data.path(), ESSENTIEL_PACK);
    let view = s.snapshot().guides.unwrap();
    assert_that!((view.id, view.origin, view.approved)).is_equal_to((ESSENTIEL_PACK, GuideOrigin::Builtin, false));
    assert_that!(view.version).is_equal_to(builtin(ESSENTIEL_PACK).version.clone());
    s.set_guide(oku_slides_core::GuideKind::Use, "mine".into()).unwrap();
    let edited = s.snapshot().guides.unwrap();
    assert_that!((edited.id, edited.origin)).is_equal_to((ESSENTIEL_PACK, GuideOrigin::Local));
    assert_that!(edited.version == view.version).is_false();
}

#[test]
fn origin_names_are_the_serialized_ones() {
    let names = [GuideOrigin::Builtin, GuideOrigin::Local, GuideOrigin::Received].map(|o| o.as_str());
    assert_that!(names.to_vec()).is_equal_to(vec!["builtin", "local", "received"]);
    let json = serde_json::to_string(&GuideOrigin::Received).unwrap();
    assert_that!(json.as_str()).is_equal_to("\"received\"");
}

#[test]
fn guide_origin_builtin_id_with_another_version_is_received() {
    let (_d, s, _, _) = session_and_pack();
    let other = files(BASE_PACK, "Base", None, "2026-10-01T10:00:00Z");
    assert_that!(s.guide_origin(BASE_PACK, &other.version(), false)).is_equal_to(GuideOrigin::Received);
}

#[test]
fn guide_origin_received_version_added_later_stays_received() {
    let (_d, s, id, pack) = session_and_pack();
    s.library.receive_version(id, &pack).unwrap();
    s.library.add_version(id, &pack).unwrap();
    assert_that!(s.guide_origin(id, &pack.version(), false)).is_equal_to(GuideOrigin::Received);
}

#[test]
fn guide_origin_unreadable_received_mark_counts_as_received() {
    let (d, s, id, pack) = session_and_pack();
    s.library.add_version(id, &pack).unwrap();
    // The version folder becomes a file: the mark beneath it cannot be read.
    let folder = d.path().join("widgets").join(id.to_string()).join("versions").join(pack.version().hex());
    std::fs::remove_dir_all(&folder).unwrap();
    std::fs::write(&folder, b"x").unwrap();
    assert_that!(s.library.is_received_or_unreadable(id, &pack.version())).is_true();
}

/// A pack version with an edit guide, none of the libraries holds yet.
fn pack_with_edit_guide(id: Uuid) -> PackFiles {
    let mut m = crate::test_support::manifest(id, "P", None, "2026-10-01T10:00:00Z");
    m.guides = oku_slides_core::Guides { edit: "hostile".into(), use_: String::new() };
    crate::test_support::files_of(&m)
}

/// T4: no predecessor shows the user wrote the guides, so the write approves nothing.
#[test]
fn approve_written_without_a_predecessor_leaves_its_guides_unapproved() {
    let (_d, s, id, _) = session_and_pack();
    let pack = pack_with_edit_guide(id);
    s.approve_written(id, &pack, None);
    assert_that!(s.library.guide_approvals().is_approved(id, &pack.version())).is_false();
}

#[test]
fn approve_written_over_an_active_predecessor_approves_new_guides() {
    let (_d, s, id, before) = session_and_pack();
    s.library.add_version(id, &before).unwrap();
    let predecessor = s.predecessor(id, &before.version(), false);
    let pack = pack_with_edit_guide(id);
    s.approve_written(id, &pack, predecessor.as_ref());
    assert_that!(s.library.guide_approvals().is_approved(id, &pack.version())).is_true();
}

/// Version `name` of pack `id` with use guide `use_`, received into the library.
fn received_with_guide(s: &Session, id: Uuid, name: &str, use_: &str) -> PackFiles {
    let mut m = crate::test_support::manifest(id, name, None, "2026-10-01T10:00:00Z");
    m.guides.set(oku_slides_core::GuideKind::Use, use_.into());
    let files = crate::test_support::files_of(&m);
    s.library.receive_version(id, &files).unwrap();
    files
}

#[test]
fn approve_guides_also_approves_received_versions_of_the_id_with_the_same_guides() {
    let (_d, mut s, id, _) = session_and_pack();
    let original = received_with_guide(&s, id, "P", "use me");
    let edited = received_with_guide(&s, id, "P, edited", "use me");
    s.handle(Request::ApproveGuides { versions: vec![VersionRef { id, version: original.version() }] }).unwrap();
    assert_that!(s.guide_active(id, &edited.version(), false)).is_true();
}

#[test]
fn approve_guides_leaves_a_received_version_with_other_guides_pending() {
    let (_d, mut s, id, _) = session_and_pack();
    let original = received_with_guide(&s, id, "P", "use me");
    let other = received_with_guide(&s, id, "P, other", "obey me");
    s.handle(Request::ApproveGuides { versions: vec![VersionRef { id, version: original.version() }] }).unwrap();
    assert_that!(s.guide_active(id, &other.version(), false)).is_false();
}

#[test]
fn approve_guides_leaves_the_same_guides_under_another_id_pending() {
    let (_d, mut s, id, _) = session_and_pack();
    let original = received_with_guide(&s, id, "P", "use me");
    let elsewhere = Uuid::new_v4();
    let copy = received_with_guide(&s, elsewhere, "Q", "use me");
    s.handle(Request::ApproveGuides { versions: vec![VersionRef { id, version: original.version() }] }).unwrap();
    assert_that!(s.guide_active(elsewhere, &copy.version(), false)).is_false();
}
