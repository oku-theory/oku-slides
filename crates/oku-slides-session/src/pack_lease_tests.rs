use std::sync::{Arc, Barrier};

use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::SessionErrorKind;
use crate::test_support::{fresh_dir, workspace_platform};
use crate::test_support_packs::assert_unleased;
use crate::{PackLibrary, ThemeLibrary};

/// A lease scope on fresh directories, kept by the caller.
fn scope() -> (tempfile::TempDir, LeaseScope) {
    let dir = fresh_dir("oku-slides-lease-");
    let scope = LeaseScope::new(&dir.path().join("widgets"), &dir.path().join("layouts"));
    (dir, scope)
}

#[test]
fn try_take_refuses_a_held_pack_naming_its_holder() {
    let (_dir, scope) = scope();
    let pack = Uuid::new_v4();
    let _held = PackLease::try_take(&draft_lock(), &scope, pack, "acme", "Tab A").unwrap();
    let err = PackLease::try_take(&draft_lock(), &scope, pack, "acme", "Tab B").unwrap_err();
    assert_that!((err.kind(), err.lease_holder())).is_equal_to((SessionErrorKind::PackOpenInTab, Some("Tab A")));
}

#[test]
fn acquire_refuses_a_held_pack() {
    let (_dir, scope) = scope();
    let pack = Uuid::new_v4();
    let _held = PackLease::acquire(&scope, pack, "acme", "Tab A").unwrap();
    let err = PackLease::acquire(&scope, pack, "acme", "Tab B").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::PackOpenInTab);
}

#[test]
fn try_take_after_a_drop_succeeds() {
    let (_dir, scope) = scope();
    let pack = Uuid::new_v4();
    drop(PackLease::acquire(&scope, pack, "acme", "Tab A").unwrap());
    assert_that!(PackLease::try_take(&draft_lock(), &scope, pack, "acme", "Tab B").is_ok()).is_true();
}

#[test]
fn leases_of_two_libraries_never_meet() {
    let ((_a, first), (_b, second)) = (scope(), scope());
    let pack = Uuid::new_v4();
    let _held = PackLease::acquire(&first, pack, "acme", "Tab A").unwrap();
    assert_that!(PackLease::acquire(&second, pack, "acme", "Tab B").is_ok()).is_true();
}

#[test]
fn leases_of_one_library_spelled_twice_meet() {
    let dir = fresh_dir("oku-slides-lease-");
    let library = |base: std::path::PathBuf| {
        let themes = ThemeLibrary::builtin_only();
        PackLibrary::new(
            Some(base.join("widgets")),
            Some(base.join("layouts")),
            themes,
            crate::Storage::local().unwrap(),
        )
    };
    let (plain, dotted) = (library(dir.path().to_path_buf()), library(dir.path().join(".")));
    let pack = Uuid::new_v4();
    let _held = PackLease::acquire(plain.lease_scope().unwrap(), pack, "acme", "Tab A").unwrap();
    assert_that!(pack_leased(dotted.lease_scope().unwrap(), pack)).is_equal_to(Some("Tab A".to_owned()));
}

#[test]
fn lease_drop_of_an_unknown_entry_never_panics() {
    let (_dir, scope) = scope();
    let pack = Uuid::new_v4();
    drop(PackLease { scope: scope.clone(), pack, token: u64::MAX });
    assert_unleased(&scope, pack);
}

#[test]
fn lease_drop_of_a_foreign_token_keeps_it() {
    let (_dir, scope) = scope();
    let pack = Uuid::new_v4();
    let old = PackLease::acquire(&scope, pack, "acme", "Tab A").unwrap();
    // The entry re-taken under another token, as a later lease would.
    PACK_TABS.lock().unwrap().get_mut(&scope).unwrap().insert(pack, (u64::MAX, "Tab B".into()));
    drop(old);
    assert_that!(pack_leased(&scope, pack)).is_equal_to(Some("Tab B".to_owned()));
}

#[test]
fn acquire_from_n_threads_grants_exactly_one() {
    let (_dir, scope) = scope();
    let pack = Uuid::new_v4();
    let barrier = Arc::new(Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|i| {
            let (scope, barrier) = (scope.clone(), Arc::clone(&barrier));
            std::thread::spawn(move || {
                barrier.wait();
                PackLease::acquire(&scope, pack, "acme", &format!("Tab {i}"))
            })
        })
        .collect();
    let results: Vec<Result<PackLease>> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    let granted = results.iter().filter(|r| r.is_ok()).count();
    let refused =
        results.iter().filter(|r| r.as_ref().is_err_and(|e| e.kind() == SessionErrorKind::PackOpenInTab)).count();
    assert_that!((granted, refused)).is_equal_to((1, 7));
}

#[test]
fn library_without_dirs_has_no_lease_scope() {
    let library = PackLibrary::new(None, None, ThemeLibrary::builtin_only(), crate::Storage::local().unwrap());
    assert_that!(library.lease_scope()).is_none();
}

#[test]
fn workspace_and_session_compute_one_scope() {
    let data = fresh_dir("oku-slides-lease-");
    let platform = workspace_platform(data.path());
    let s = crate::workspace::tab_session(&crate::Fonts::default(), &platform).unwrap();
    let scope = PackLibrary::user(&platform).lease_scope().cloned();
    assert_that!(scope.is_some()).is_true();
    assert_that!(scope).is_equal_to(s.library.lease_scope().cloned());
}
