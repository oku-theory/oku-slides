//! t-1782: `BaseStore::gc` listed the leases folder while another store rewrote its lease
//! file through `Storage::write_atomic`, and read that write's temporary file: half-written,
//! it did not parse and the round collected nothing; renamed in between, the read failed
//! `NotFound` and `gc` returned an error.

use std::collections::BTreeSet;
use std::time::{Duration, SystemTime};

use oku_slides_platform::bases::BaseStore;
use oku_slides_platform::storage::Storage;
use speculoos::prelude::*;

use crate::temp_dir;

#[test]
fn gc_ignores_a_lease_write_in_flight() {
    let dir = temp_dir("gc-lease-temp");
    let store = BaseStore::new(Storage::local().unwrap_or_else(|e| panic!("storage: {e}")), dir.path(), 1 << 30)
        .unwrap_or_else(|e| panic!("store: {e}"));
    let src = dir.path().join("b.parquet");
    std::fs::write(&src, b"base").unwrap_or_else(|e| panic!("{e}"));
    let id = store.adopt(&src, 1 << 20).unwrap_or_else(|e| panic!("adopt: {e}"));
    store.release([&id]).unwrap_or_else(|e| panic!("release: {e}"));
    let bases = dir.path().join("bases");
    // Past the grace period, the released base is due for collection.
    let base = bases.join(format!("{}.parquet", id.hex()));
    let aged = SystemTime::now() - Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(&base)
        .and_then(|f| f.set_modified(aged))
        .unwrap_or_else(|e| panic!("{e}"));
    // Another store's lease rewrite, caught halfway: `write_atomic`'s temporary name.
    std::fs::write(bases.join("leases").join(".other.json.4242-7.tmp"), b"[\"ab").unwrap_or_else(|e| panic!("{e}"));

    let deleted = store.gc(&BTreeSet::new()).unwrap_or_else(|e| panic!("gc: {e}"));

    assert_that!(deleted).is_equal_to(1);
    assert_that!(store.contains(&id).unwrap_or_else(|e| panic!("contains: {e}"))).is_false();
}
