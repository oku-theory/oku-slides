use speculoos::prelude::*;

use oku_slides_platform::storage::Storage;

use super::*;

#[test]
fn probe_canary_in_a_writable_dir_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let canary = probe_canary(&Storage::local().unwrap(), &dir.path().join("c"));
    assert_that!(canary).is_equal_to(Canary::Written);
}

#[cfg(unix)]
#[test]
fn probe_canary_in_a_read_only_dir_is_denied() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
    let canary = probe_canary(&Storage::local().unwrap(), &dir.path().join("c"));
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_that!(canary).is_equal_to(Canary::Denied);
}

#[test]
fn wipe_empties_the_work_dir_and_keeps_it() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
    std::fs::write(dir.path().join("a/b/f"), b"x").unwrap();
    std::fs::write(dir.path().join("g"), b"x").unwrap();
    wipe(&Storage::local().unwrap(), dir.path()).unwrap();
    assert_that!(dir.path().exists()).is_true();
    assert_that!(std::fs::read_dir(dir.path()).unwrap().count()).is_equal_to(0);
}
