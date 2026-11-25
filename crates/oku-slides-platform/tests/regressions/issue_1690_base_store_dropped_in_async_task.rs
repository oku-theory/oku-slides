//! t-1690: a session dropped on an async task (an agent turn ending with its tab) panicked:
//! releasing its bases rewrote the lease file, and the store's last handle deleted it, both
//! blocking on the storage runtime from inside another: "Cannot start a runtime from within a
//! runtime". Holding the storage's last handle, the store then dropped that runtime there:
//! "Cannot drop a runtime in a context where blocking is not allowed".

use oku_slides_platform::bases::BaseStore;
use oku_slides_platform::storage::Storage;
use speculoos::prelude::*;

use crate::temp_dir;

#[test]
fn base_store_released_and_dropped_in_async_task_deletes_its_lease_file() {
    let dir = temp_dir("base-store-drop");
    let storage = Storage::local().unwrap_or_else(|e| panic!("storage: {e}"));
    // The store holds the storage's only handle, so its runtime goes with it.
    let store = BaseStore::new(storage, dir.path(), 1 << 30).unwrap_or_else(|e| panic!("store: {e}"));
    let src = dir.path().join("b.parquet");
    std::fs::write(&src, b"base").unwrap_or_else(|e| panic!("{e}"));
    // Adopting leases the base: the store now holds a lease file.
    let id = store.adopt(&src, 1 << 20).unwrap_or_else(|e| panic!("adopt: {e}"));
    let leases = dir.path().join("bases").join("leases");
    let count = || std::fs::read_dir(&leases).map_or(0, Iterator::count);
    assert_that!(count()).is_equal_to(1);
    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap_or_else(|e| panic!("runtime: {e}"));
    runtime.block_on(async move {
        store.release([&id]).unwrap_or_else(|e| panic!("release: {e}"));
        drop(store);
    });
    assert_that!(count()).is_equal_to(0);
}
