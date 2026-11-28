//! A compaction meeting a peer's work: a concurrent edit is lost and reported on both sides,
//! and a base that never arrived is reported, not a panic.

use oku_slides_core::{Issue, SourceBundle};
use oku_slides_data::{BaseId, BaseRef, CellRead};

use super::*;

/// An independent copy of `bundle`, through the zip, its bases in `s`'s store.
fn copy(s: &Session, bundle: &SourceBundle) -> SourceBundle {
    let files = s.bases.files();
    let zip = bundle.to_zip_bases(false, Some(&files)).unwrap_or_else(|e| panic!("zip: {e:?}"));
    SourceBundle::from_zip_bases(&zip, s.source_limits, s.registry.clone(), Some(&files))
        .unwrap_or_else(|e| panic!("unzip: {e:?}"))
        .0
}

fn keys(issues: &[Issue]) -> Vec<&str> {
    issues.iter().map(|i| i.message.id()).collect()
}

/// Folds `d9` of `bundle` onto `base`, as the host commits a compaction.
fn compact(bundle: &mut SourceBundle, base: BaseRef) {
    let from = bundle.deck().data[&id("d9")].table().unwrap().clone();
    let done = bundle.compact_data(&id("d9"), &from, "e2".to_owned(), base).unwrap_or_else(|e| panic!("{e:?}"));
    assert_that!(done).is_true();
}

#[rstest]
#[case::compaction_first(true)]
#[case::edit_first(false)]
fn peer_edit_vs_compaction(#[case] compaction_first: bool) {
    let dir = fresh_dir("oku-slides-compact-peer-");
    let mut s = opened_with(dir.path(), 4);
    // The compacting peer starts from the same deck.
    let mut theirs = copy(&s, &s.doc.sources);
    let base = crate::bases::bases_tests::put_base(s.bases.store(), b"their base");
    if compaction_first {
        compact(&mut theirs, base.clone());
    }
    apply(&mut s, vec![set_cell("d9", "r1", 77)]);
    if !compaction_first {
        compact(&mut theirs, base.clone());
    }
    assert_that!(s.doc.sources.replica().undo_touches_data(&id("d9"))).is_true();
    // Each side merges the other's copy from before any merge.
    let mut ours = copy(&s, &s.doc.sources);
    let (_, ours_issues) = ours.merge(copy(&s, &theirs)).unwrap_or_else(|e| panic!("ours: {e:?}"));
    let (_, theirs_issues) = theirs.merge(copy(&s, &s.doc.sources)).unwrap_or_else(|e| panic!("theirs: {e:?}"));
    assert_that!(keys(&ours_issues)).contains("issue-data-edits-lost");
    assert_that!(keys(&theirs_issues)).contains("issue-data-edits-lost");
    s.doc.notice = None;
    let report = s.absorb(copy(&s, &theirs), "theirs.oku-slides.pdf", false).unwrap_or_else(|e| panic!("{e:?}"));
    assert_that!(report.lost).is_equal_to(vec![id("d9")]);
    // The edit is lost everywhere: every copy holds the compacted layer, and only it.
    assert_that!(ours.deck().data).is_equal_to(&theirs.deck().data);
    assert_that!(s.deck().data).is_equal_to(&theirs.deck().data);
    let table = s.deck().data[&id("d9")].table().unwrap();
    assert_that!(table.base.as_ref()).is_equal_to(Some(&base));
    assert_that!(table.cell("r1", "c2")).is_equal_to(CellRead::Base);
    // The editor's undo step on d9 would write into the dead layer: dropped, with the notice.
    assert_that!(s.doc.sources.replica().undo_touches_data(&id("d9"))).is_false();
    assert_that!(s.can_undo()).is_false();
    let cut = s.localizer.tr_with("session-data-undo-cut", &[("data", "ventes".into())]);
    assert_that!(s.doc.notice.clone().unwrap_or_default()).contains(cut.as_str());
}

#[test]
fn missing_base_during_merge_reports_without_panic() {
    let dir = fresh_dir("oku-slides-compact-missing-");
    let mut s = opened_with(dir.path(), 4);
    // Two copies of a peer whose base never reached this machine: no file in the store.
    let absent = BaseId::try_from(format!("sha256:{}", "b".repeat(64))).unwrap_or_else(|e| panic!("{e:?}"));
    let missing = BaseRef { id: absent.clone(), bytes: 10, rows: 4, columns: Vec::new() };
    let mut theirs = [copy(&s, &s.doc.sources), copy(&s, &s.doc.sources)];
    for bundle in &mut theirs {
        compact(bundle, missing.clone());
        bundle.bases.remove(&absent);
    }
    let [first, second] = theirs;
    let mut ours = copy(&s, &s.doc.sources);
    let (_, issues) = ours.merge(first).unwrap_or_else(|e| panic!("merge: {e:?}"));
    assert_that!(keys(&issues)).contains("issue-data-base-missing");
    s.absorb(second, "theirs.oku-slides.pdf", false).unwrap_or_else(|e| panic!("absorb: {e:?}"));
    let d9 = &s.deck().data[&id("d9")];
    assert_that!(d9.table().unwrap().base.as_ref().map(|b| &b.id)).is_equal_to(Some(&absent));
    assert_that!(s.rows_read(&id("d9"), d9, None, 0..4, usize::MAX).and_then(RowsRead::run).is_err()).is_true();
}
