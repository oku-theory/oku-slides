use std::path::Path;

use serde_json::json;
use speculoos::prelude::*;
use uuid::Uuid;

use super::Request;
use crate::Session;
use crate::library::draft_lock;
use crate::library_index::library_writes;
use crate::test_support::{add_version, edit_state, fresh_dir, manifest_ns, rename_title, session_on};
use crate::test_support_packs::{acme_of, acme_pack, adopting_deck};

/// A second version of `pack` made current by another session on `data`.
fn newer_elsewhere(data: &Path, pack: Uuid) {
    let mut other = crate::test_support_packs::pack_tab_of(data, pack);
    other.apply("x", vec![rename_title("acme 2")], None).unwrap();
    other.save_pack(false).unwrap();
}

/// Copies the folder `from` over `to`, files replaced.
fn copy_over(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
            copy_over(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn use_pack_version_request_parses_the_ui_json() {
    let pack = Uuid::new_v4();
    let version = crate::test_support::builtin(crate::BASE_PACK).version.clone();
    let request: Request = serde_json::from_value(json!({
        "cmd": "use_pack_version", "pack": pack, "version": version.as_str(), "deck_uid": "d-1",
    }))
    .unwrap();
    let parsed = matches!(request, Request::UsePackVersion { pack: p, version: v, deck_uid } if p == pack && v == version && deck_uid == "d-1");
    assert_that!(parsed).is_true();
}

#[test]
fn snapshot_request_relights_the_badge_after_a_library_write() {
    let data = fresh_dir("oku-slides-badge-");
    let mut a = session_on(data.path());
    let (acme, _) = acme_pack(&mut a, data.path());
    a.handle(Request::Snapshot).unwrap();
    assert_that!(a.snapshot().pack_updates).is_equal_to(0);
    newer_elsewhere(data.path(), acme);
    a.handle(Request::Snapshot).unwrap();
    assert_that!(a.snapshot().pack_updates).is_equal_to(1);
}

/// Characterization of an accepted limit (t-1157): a library write of another process, which
/// moves no in-process count, leaves the badge unlit until the pins change. Invert it when
/// cross-process badges come.
#[test]
fn snapshot_request_without_an_in_process_write_keeps_the_badge_unlit() {
    let (data, scratch) = (fresh_dir("oku-slides-badge-"), fresh_dir("oku-slides-badge-scratch-"));
    let mut a = session_on(data.path());
    let (acme, _) = acme_pack(&mut a, data.path());
    copy_over(&data.path().join("widgets"), &scratch.path().join("widgets"));
    newer_elsewhere(scratch.path(), acme);
    a.handle(Request::Snapshot).unwrap();
    assert_that!(a.snapshot().pack_updates).is_equal_to(0);
    copy_over(&scratch.path().join("widgets"), &data.path().join("widgets"));
    a.handle(Request::Snapshot).unwrap();
    assert_that!(a.snapshot().pack_updates).is_equal_to(0);
}

#[test]
fn snapshot_request_after_a_foreign_write_takes_no_drafts() {
    let data = fresh_dir("oku-slides-badge-");
    let mut a = session_on(data.path());
    acme_pack(&mut a, data.path());
    a.handle(Request::Snapshot).unwrap();
    let other = session_on(data.path());
    add_version(crate::test_support::library_of(&other), &manifest_ns(Uuid::new_v4(), "autre", &[]), &[]);
    let (held, release) = (std::sync::mpsc::channel(), std::sync::mpsc::channel::<()>());
    std::thread::spawn(move || {
        let lock = draft_lock();
        held.0.send(()).unwrap();
        let _ = release.1.recv();
        drop(lock);
    });
    held.1.recv().unwrap();
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let answered = a.handle(Request::Snapshot).is_ok();
        let _ = done.send(answered);
    });
    let answered = finished.recv_timeout(std::time::Duration::from_secs(10));
    release.0.send(()).unwrap();
    assert_that!(answered).is_equal_to(Ok(true));
}

#[test]
fn snapshot_after_adopting_an_unfrozen_pinned_version_settles() {
    let data = fresh_dir("oku-slides-badge-");
    let mut s = session_on(data.path());
    let mut t: Session = adopting_deck(&mut s, data.path());
    let acme = acme_of(&s);
    t.handle(Request::Snapshot).unwrap();
    let v = t.user_deck().widgets.values().find(|p| p.id == acme).map(|p| p.version.clone()).unwrap();
    assert_that!(t.library.draft_write(&draft_lock(), acme, Some(&v)))
        .is_equal_to(crate::library::DraftWrite::Child(v));
    t.handle(Request::Snapshot).unwrap();
    let settled = library_writes();
    t.handle(Request::Snapshot).unwrap();
    assert_that!(library_writes()).is_equal_to(settled);
}

#[test]
fn snapshot_request_changes_nothing_persisted() {
    let data = fresh_dir("oku-slides-badge-");
    let dir = fresh_dir("oku-slides-badge-tab-");
    let mut a = session_on(data.path());
    assert_that!(a.resume_fresh(dir.path().to_path_buf())).is_true();
    let (acme, _) = acme_pack(&mut a, data.path());
    a.flush_resume();
    edit_state(dir.path(), |state| state["ui"]["grid"] = json!(false));
    newer_elsewhere(data.path(), acme);
    a.handle(Request::Snapshot).unwrap();
    a.flush_resume();
    let state: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("state.json")).unwrap()).unwrap();
    assert_that!(state["ui"]["grid"]).is_equal_to(json!(false));
}
