use speculoos::prelude::*;

use super::{check_base_path, open_base};

fn bases() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let canonical = std::fs::canonicalize(dir.path()).unwrap();
    (dir, canonical)
}

#[test]
fn check_base_path_inside_folder_is_accepted() {
    let (_guard, dir) = bases();
    let path = dir.join("ab12.parquet");
    assert_that!(check_base_path(Some(&dir), &path)).is_ok_containing(path);
}

#[test]
fn worker_refuses_base_path_outside_its_dir() {
    let (_guard, dir) = bases();
    let (_other, elsewhere) = bases();
    assert_that!(check_base_path(Some(&dir), &elsewhere.join("ab12.parquet"))).is_err();
    assert_that!(check_base_path(Some(&dir), &dir.join("sub").join("ab12.parquet"))).is_err();
    assert_that!(check_base_path(Some(&dir), &dir.join("..").join("ab12.parquet"))).is_err();
    assert_that!(check_base_path(Some(&dir), &dir.join("ab12.csv"))).is_err();
    assert_that!(check_base_path(None, &dir.join("ab12.parquet"))).is_err();
}

#[cfg(unix)]
#[test]
fn open_base_through_symlink_is_refused() {
    let (_guard, dir) = bases();
    let target = dir.join("real.parquet");
    std::fs::write(&target, b"PAR1").unwrap();
    let link = dir.join("link.parquet");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert_that!(open_base(&link)).is_err();
    assert_that!(open_base(&target)).is_ok();
}
