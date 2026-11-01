use oku_slides_core::PackKind;
use rstest::rstest;
use speculoos::prelude::*;
use time::OffsetDateTime;
use uuid::Uuid;

use super::ORPHAN_AGE;
use crate::Session;
use crate::test_support::{add_version, fresh_dir, manifest_ns, session_on};
use crate::test_support_packs::acme_pack;

/// A moment past the orphan age of anything created now.
fn later() -> OffsetDateTime {
    OffsetDateTime::now_utc() + ORPHAN_AGE + time::Duration::minutes(1)
}

/// Which empty draft a case creates in `s`.
#[derive(Debug, Clone, Copy)]
enum Draft {
    Pack,
    Theme,
}

/// Creates the empty draft `draft` in `s`'s library: its id.
fn create(s: &Session, draft: Draft) -> Uuid {
    match draft {
        Draft::Pack => s.library.create_empty_pack(PackKind::Widgets, "Nouveau").unwrap(),
        Draft::Theme => s.library.create_empty_theme("Nouveau").unwrap(),
    }
}

#[rstest]
#[case::pack(Draft::Pack)]
#[case::theme(Draft::Theme)]
fn collect_orphan_drafts_deletes_an_old_empty_draft(#[case] draft: Draft) {
    let data = fresh_dir("oku-slides-orphans-");
    let s = session_on(data.path());
    let id = create(&s, draft);
    assert_that!(s.library.collect_orphan_drafts(&[], later())).is_equal_to(vec![id]);
}

#[rstest]
#[case::pack(Draft::Pack)]
#[case::theme(Draft::Theme)]
fn collect_orphan_drafts_keeps_a_fresh_empty_draft(#[case] draft: Draft) {
    let data = fresh_dir("oku-slides-orphans-");
    let s = session_on(data.path());
    create(&s, draft);
    assert_that!(s.library.collect_orphan_drafts(&[], OffsetDateTime::now_utc())).is_empty();
}

#[test]
fn collect_orphan_drafts_keeps_a_draft_a_tab_shows() {
    let data = fresh_dir("oku-slides-orphans-");
    let s = session_on(data.path());
    let id = create(&s, Draft::Pack);
    assert_that!(s.library.collect_orphan_drafts(&[id], later())).is_empty();
}

#[test]
fn collect_orphan_drafts_keeps_a_pack_with_content() {
    let data = fresh_dir("oku-slides-orphans-");
    let mut s = session_on(data.path());
    s.new_deck().unwrap();
    acme_pack(&mut s, data.path());
    assert_that!(s.library.collect_orphan_drafts(&[], later())).is_empty();
}

/// An empty pack that `create_empty_pack` never made (an import, a CLI write) is the user's (t-1251).
#[test]
fn collect_orphan_drafts_keeps_an_empty_pack_not_created_as_a_draft() {
    let data = fresh_dir("oku-slides-orphans-");
    let s = session_on(data.path());
    add_version(&s.library, &manifest_ns(Uuid::new_v4(), "vide", &[]), &[]);
    assert_that!(s.library.collect_orphan_drafts(&[], later())).is_empty();
}

/// The folder holding the versions of draft `id` under `root`.
fn versions_dir(root: &std::path::Path, id: Uuid) -> std::path::PathBuf {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().map(Result::unwrap) {
            let path = entry.path();
            if path.ends_with(id.to_string()) {
                return path.join("versions");
            }
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
    panic!("no folder for {id}");
}

#[rstest]
#[case::pack(Draft::Pack)]
#[case::theme(Draft::Theme)]
fn collect_orphan_drafts_keeps_a_draft_with_an_unreadable_second_folder(#[case] draft: Draft) {
    let data = fresh_dir("oku-slides-orphans-");
    let s = session_on(data.path());
    let id = create(&s, draft);
    std::fs::create_dir(versions_dir(data.path(), id).join("illisible")).unwrap();
    assert_that!(s.library.collect_orphan_drafts(&[], later())).is_empty();
}

/// t-1378: resume runs this before the first window; an unmarked pack is ruled out by listings alone.
#[test]
fn collect_orphan_drafts_reads_no_version_of_an_unmarked_pack() {
    let data = fresh_dir("oku-slides-orphans-");
    let mut s = session_on(data.path());
    s.new_deck().unwrap();
    acme_pack(&mut s, data.path());
    let fresh = session_on(data.path());
    let before = fresh.library.widget_scans();
    fresh.library.collect_orphan_drafts(&[], later());
    let after_scan = fresh.library.widget_scans();
    // Positive control: a listing that does read versions moves the counter.
    assert_that!(fresh.library.packs(PackKind::Widgets)).is_not_empty();
    assert_that!((after_scan, fresh.library.widget_scans() > before)).is_equal_to((before, true));
}
