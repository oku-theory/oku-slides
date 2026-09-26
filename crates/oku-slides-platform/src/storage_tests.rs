use std::path::PathBuf;

use speculoos::prelude::*;

use super::*;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("oku-slides-storage-{name}-{}", std::process::id()))
}

#[test]
fn write_then_read_returns_the_bytes_and_creates_parents() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("rw");
    let file = dir.join("a/b/c.bin");
    s.write(&file, vec![1, 2, 3]).unwrap();
    assert_that!(s.read(&file).unwrap()).is_equal_to(vec![1, 2, 3]);
    assert_that!(s.len(&file).unwrap()).is_equal_to(Some(3));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn read_missing_file_returns_not_found() {
    let s = Storage::local().unwrap();
    let err = s.read(&temp_dir("absent").join("x")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(StorageErrorKind::NotFound);
    assert_that!(s.len(&temp_dir("absent").join("x")).unwrap()).is_none();
}

#[test]
fn write_atomic_replaces_content_and_leaves_no_temp_file() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("atomic");
    let file = dir.join("deck.oku-slides.pdf");
    s.write_atomic(&file, b"v1".to_vec()).unwrap();
    s.write_atomic(&file, b"v2".to_vec()).unwrap();
    assert_that!(s.read(&file).unwrap()).is_equal_to(b"v2".to_vec());
    assert_that!(s.list_files(&dir).unwrap()).is_equal_to(vec![file.clone()]);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn list_files_missing_dir_returns_empty_and_skips_subdirs() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("list");
    assert_that!(s.list_files(&dir).unwrap()).is_empty();
    s.write(&dir.join("x.json"), b"{}".to_vec()).unwrap();
    s.create_dir_all(&dir.join("sub")).unwrap();
    assert_that!(s.list_files(&dir).unwrap()).is_equal_to(vec![dir.join("x.json")]);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn read_fonts_keeps_only_font_files() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("fonts");
    s.write(&dir.join("Inter.TTF"), vec![0; 4]).unwrap();
    s.write(&dir.join("notes.txt"), vec![0; 4]).unwrap();
    let fonts = s.read_fonts(&dir).unwrap();
    assert_that!(fonts.keys().map(String::as_str).collect::<Vec<_>>()).is_equal_to(vec!["Inter.TTF"]);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn writer_streams_chunks_then_rename_and_delete_work() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("stream");
    let part = dir.join("model.bin.part");
    let mut w = s.writer(&part).unwrap();
    w.write_all(b"ab").unwrap();
    w.write_all(b"cd").unwrap();
    w.close().unwrap();
    let done = dir.join("model.bin");
    s.rename(&part, &done).unwrap();
    assert_that!(s.read(&done).unwrap()).is_equal_to(b"abcd".to_vec());
    assert_that!(s.exists(&part).unwrap()).is_false();
    s.delete(&done).unwrap();
    assert_that!(s.exists(&done).unwrap()).is_false();
    s.delete(&done).unwrap();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn read_path_with_parent_components_resolves_them() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("dotdot");
    s.write(&dir.join("x/file"), b"ok".to_vec()).unwrap();
    assert_that!(s.read(&dir.join("x/../x/./file")).unwrap()).is_equal_to(b"ok".to_vec());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn locate_relative_path_resolves_against_current_dir() {
    let located = locate(Path::new("a/./b/../c.txt")).unwrap();
    let cwd = std::env::current_dir().unwrap();
    assert_that!(located.key.ends_with("a/c.txt")).is_true();
    assert_that!(located.key.contains('\\')).is_false();
    assert_that!(cwd.starts_with(&located.root)).is_true();
}

#[test]
fn locate_root_alone_has_no_key() {
    assert_that!(locate(Path::new("/"))).is_none();
    assert_that!(locate(Path::new("/.."))).is_none();
}

#[test]
fn at_same_root_reuses_one_operator() {
    let s = Storage::local().unwrap();
    s.exists(Path::new("/tmp/a")).unwrap();
    s.exists(Path::new("/usr/b")).unwrap();
    assert_that!(s.roots.lock().unwrap().len()).is_equal_to(1);
}
