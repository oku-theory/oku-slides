use std::path::PathBuf;

use super::*;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("echo-storage-{name}-{}", std::process::id()))
}

#[test]
fn write_then_read_returns_the_bytes_and_creates_parents() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("rw");
    let file = dir.join("a/b/c.bin");
    s.write(&file, vec![1, 2, 3]).unwrap();
    assert_eq!(s.read(&file).unwrap(), [1, 2, 3]);
    assert_eq!(s.len(&file).unwrap(), Some(3));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn read_missing_file_returns_not_found() {
    let s = Storage::local().unwrap();
    let err = s.read(&temp_dir("absent").join("x")).unwrap_err();
    assert_eq!(err.kind(), StorageErrorKind::NotFound);
    assert_eq!(s.len(&temp_dir("absent").join("x")).unwrap(), None);
}

#[test]
fn write_atomic_replaces_content_and_leaves_no_temp_file() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("atomic");
    let file = dir.join("deck.echo.pdf");
    s.write_atomic(&file, b"v1".to_vec()).unwrap();
    s.write_atomic(&file, b"v2".to_vec()).unwrap();
    assert_eq!(s.read(&file).unwrap(), b"v2");
    assert_eq!(s.list_files(&dir).unwrap(), std::slice::from_ref(&file));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn list_files_missing_dir_returns_empty_and_skips_subdirs() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("list");
    assert!(s.list_files(&dir).unwrap().is_empty());
    s.write(&dir.join("x.json"), b"{}".to_vec()).unwrap();
    s.create_dir_all(&dir.join("sub")).unwrap();
    assert_eq!(s.list_files(&dir).unwrap(), [dir.join("x.json")]);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn read_fonts_keeps_only_font_files() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("fonts");
    s.write(&dir.join("Inter.TTF"), vec![0; 4]).unwrap();
    s.write(&dir.join("notes.txt"), vec![0; 4]).unwrap();
    let fonts = s.read_fonts(&dir).unwrap();
    assert_eq!(fonts.keys().collect::<Vec<_>>(), ["Inter.TTF"]);
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
    assert_eq!(s.read(&done).unwrap(), b"abcd");
    assert!(!s.exists(&part).unwrap());
    s.delete(&done).unwrap();
    assert!(!s.exists(&done).unwrap());
    s.delete(&done).unwrap();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn read_path_with_parent_components_resolves_them() {
    let s = Storage::local().unwrap();
    let dir = temp_dir("dotdot");
    s.write(&dir.join("x/file"), b"ok".to_vec()).unwrap();
    assert_eq!(s.read(&dir.join("x/../x/./file")).unwrap(), b"ok");
    std::fs::remove_dir_all(&dir).ok();
}
