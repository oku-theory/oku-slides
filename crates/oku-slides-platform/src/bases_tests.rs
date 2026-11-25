use std::collections::BTreeSet;
use std::time::{Duration, SystemTime};

use oku_slides_data::BaseId;
use sha2::{Digest, Sha256};
use speculoos::prelude::*;
use tempfile::TempDir;

use super::{BaseStore, BaseStoreErrorKind, GC_GRACE, LEASE_STALE};
use crate::storage::Storage;

const MAX: u64 = 1 << 20;

fn store(dir: &TempDir) -> BaseStore {
    BaseStore::new(Storage::local().unwrap(), dir.path(), 1 << 30).unwrap()
}

fn id_of(bytes: &[u8]) -> BaseId {
    BaseId::from_sha256(&Sha256::digest(bytes).into())
}

/// A file outside the store holding `bytes`.
fn source(dir: &TempDir, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

fn later(by: Duration) -> SystemTime {
    SystemTime::now() + by
}

#[test]
fn adopt_hashes_while_copying() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);

    let id = store.adopt(&source(&dir, "a", b"base bytes"), MAX).unwrap();

    assert_that!(id).is_equal_to(id_of(b"base bytes"));
    assert_that!(std::fs::read(store.path(&id)).unwrap()).is_equal_to(b"base bytes".to_vec());
}

#[cfg(unix)]
#[test]
fn adopt_refuses_symlink() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let target = source(&dir, "target", b"secret");
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let error = store.adopt(&link, MAX).unwrap_err();

    assert_that!(error.kind()).is_equal_to(BaseStoreErrorKind::NotRegular);
}

#[test]
fn put_stream_refuses_hash_mismatch_and_oversize() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);

    let mismatch = store.put_stream(&b"other"[..], &id_of(b"bytes"), MAX).unwrap_err();
    let oversize = store.put_stream(&b"bytes"[..], &id_of(b"bytes"), 4).unwrap_err();
    store.put_stream(&b"bytes"[..], &id_of(b"bytes"), 5).unwrap();

    assert_that!(mismatch.kind()).is_equal_to(BaseStoreErrorKind::HashMismatch);
    assert_that!(oversize.kind()).is_equal_to(BaseStoreErrorKind::TooLarge);
    assert_that!(store.contains(&id_of(b"bytes")).unwrap()).is_true();
    let names: Vec<_> = std::fs::read_dir(store.dir()).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_that!(names.iter().any(|n| n.to_string_lossy().starts_with(".tmp-"))).is_false();
}

#[test]
fn adopted_uncommitted_file_survives_gc() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let id = store.adopt(&source(&dir, "a", b"fresh"), MAX).unwrap();

    let deleted = store.gc_at(&BTreeSet::new(), later(GC_GRACE * 2)).unwrap();

    assert_that!(deleted).is_equal_to(0);
    assert_that!(store.contains(&id).unwrap()).is_true();
}

#[test]
fn released_unrooted_base_is_collected() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let id = store.adopt(&source(&dir, "a", b"old"), MAX).unwrap();
    store.release([&id]).unwrap();

    let deleted = store.gc_at(&BTreeSet::new(), later(GC_GRACE * 2)).unwrap();

    assert_that!(deleted).is_equal_to(1);
    assert_that!(store.contains(&id).unwrap()).is_false();
}

#[test]
fn rooted_base_is_kept() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let id = store.adopt(&source(&dir, "a", b"rooted"), MAX).unwrap();
    store.release([&id]).unwrap();

    store.gc_at(&BTreeSet::from([id.clone()]), later(GC_GRACE * 2)).unwrap();

    assert_that!(store.contains(&id).unwrap()).is_true();
}

#[test]
fn second_process_lease_protects_its_bases() {
    let dir = TempDir::new().unwrap();
    let ours = store(&dir);
    let theirs = store(&dir);
    let id = theirs.adopt(&source(&dir, "a", b"theirs"), MAX).unwrap();

    ours.gc_at(&BTreeSet::new(), later(GC_GRACE * 2)).unwrap();

    assert_that!(ours.contains(&id).unwrap()).is_true();
}

#[test]
fn stale_lease_does_not_protect() {
    let dir = TempDir::new().unwrap();
    let ours = store(&dir);
    let theirs = store(&dir);
    let id = theirs.adopt(&source(&dir, "a", b"theirs"), MAX).unwrap();

    ours.gc_at(&BTreeSet::new(), later(LEASE_STALE * 2)).unwrap();

    assert_that!(ours.contains(&id).unwrap()).is_false();
}

#[test]
fn gc_keeps_files_younger_than_10_min() {
    let dir = TempDir::new().unwrap();
    let store = store(&dir);
    let id = store.adopt(&source(&dir, "a", b"young"), MAX).unwrap();
    store.release([&id]).unwrap();

    let deleted = store.gc_at(&BTreeSet::new(), later(GC_GRACE.checked_sub(Duration::from_secs(60)).unwrap())).unwrap();

    assert_that!(deleted).is_equal_to(0);
    assert_that!(store.contains(&id).unwrap()).is_true();
}

#[test]
fn base_id_with_dot_dot_is_refused_at_parse() {
    let forged = format!("sha256:../../{}", "a".repeat(58));

    assert_that!(BaseId::try_from(forged)).is_err();
}

#[test]
fn store_over_quota_skips_compaction_and_reports() {
    let dir = TempDir::new().unwrap();
    let store = BaseStore::new(Storage::local().unwrap(), dir.path(), 8).unwrap();
    store.adopt(&source(&dir, "a", b"12345"), MAX).unwrap();

    assert_that!(store.check_room(3)).is_ok();
    assert_that!(store.check_room(4).unwrap_err().kind()).is_equal_to(BaseStoreErrorKind::Full);
}

#[test]
fn open_of_an_absent_base_is_missing() {
    let dir = TempDir::new().unwrap();

    let error = store(&dir).open(&id_of(b"none")).unwrap_err();

    assert_that!(error.kind()).is_equal_to(BaseStoreErrorKind::Missing);
}

#[test]
fn lease_refresh_keeps_bases_past_stale_age() {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_time().build().unwrap();
    let dir = TempDir::new().unwrap();
    let ours = store(&dir);
    let theirs = store(&dir);
    let id = ours.adopt(&source(&dir, "a", b"ours"), MAX).unwrap();
    let lease = ours.inner.lease_file.clone();
    // Our lease file as a process leaves it after more than LEASE_STALE without a refresh.
    let aged = SystemTime::now() - (LEASE_STALE + Duration::from_mins(1));
    std::fs::File::options().write(true).open(&lease).unwrap().set_modified(aged).unwrap();
    // The `LEASE_REFRESH` loop, on a short period.
    let _refresh = ours.spawn_refresh_every(runtime.handle(), Duration::from_millis(10));

    for _ in 0..500 {
        std::thread::sleep(Duration::from_millis(10));
        if std::fs::metadata(&lease).unwrap().modified().unwrap() > aged {
            break;
        }
    }
    theirs.gc_at(&BTreeSet::new(), later(GC_GRACE * 2)).unwrap();

    // → without the refresh the lease is 51 min old at that `now`: stale, the base deleted.
    assert_that!(theirs.contains(&id).unwrap()).is_true();
}
