use oku_slides_import::{ImportErrorKind, ReadLimits};
use speculoos::prelude::*;

use super::read_on_own_thread;

#[test]
fn read_on_own_thread_garbage_gives_the_reader_error() {
    let read = read_on_own_thread(b"not a zip", &ReadLimits::default()).map(|r| r.map(|_| ()).map_err(|e| e.kind()));
    assert_that!(read.ok()).is_equal_to(Some(Err(ImportErrorKind::Unreadable)));
}
