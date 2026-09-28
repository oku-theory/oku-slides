use std::path::Path;
use std::path::PathBuf;

use speculoos::prelude::*;

use super::*;

fn temp_dir(name: &str) -> tempfile::TempDir {
    crate::test_support::fresh_dir(&format!("oku-slides-storage-{name}-"))
}

#[test]
fn write_then_read_returns_the_bytes_and_creates_parents() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("rw");
    let dir = tmp.path();
    let file = dir.join("a/b/c.bin");
    s.write(&file, vec![1, 2, 3]).unwrap();
    assert_that!(s.read(&file).unwrap()).is_equal_to(vec![1, 2, 3]);
    assert_that!(s.len(&file).unwrap()).is_equal_to(Some(3));
}

#[test]
fn read_missing_file_returns_not_found() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("absent");
    let err = s.read(&tmp.path().join("x")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(StorageErrorKind::NotFound);
    assert_that!(s.len(&tmp.path().join("x")).unwrap()).is_none();
}

#[test]
fn write_atomic_replaces_content_and_leaves_no_temp_file() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("atomic");
    let dir = tmp.path();
    let file = dir.join("deck.oku-slides.pdf");
    s.write_atomic(&file, b"v1".to_vec()).unwrap();
    s.write_atomic(&file, b"v2".to_vec()).unwrap();
    assert_that!(s.read(&file).unwrap()).is_equal_to(b"v2".to_vec());
    assert_that!(s.list_files(dir).unwrap()).is_equal_to(vec![file.clone()]);
}

/// Deux écritures simultanées du même fichier (deux onglets qui enregistrent
/// le même ensemble de modèles) : aucune ne perd son fichier temporaire.
#[test]
fn write_atomic_concurrent_writes_both_succeed() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("atomic-race");
    let dir = tmp.path();
    let path = dir.join("set.json");
    let writers: Vec<_> = (0..8u8)
        .map(|n| {
            let (s, path) = (s.clone(), path.clone());
            std::thread::spawn(move || (0..20).try_for_each(|_| s.write_atomic(&path, vec![n; 4096])))
        })
        .collect();
    for w in writers {
        assert_that!(w.join().unwrap()).is_ok();
    }
    assert_that!(s.read(&path).unwrap().len()).is_equal_to(4096);
}

#[test]
fn delete_dir_removes_the_tree_and_ignores_missing_dirs() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("rmdir");
    let dir = tmp.path();
    s.write(&dir.join("t-1/deck/document.json"), vec![1]).unwrap();
    s.delete_dir(&dir.join("t-1")).unwrap();
    assert_that!(dir.join("t-1").exists()).is_false();
    assert_that!(dir.exists()).is_true();
    assert_that!(s.delete_dir(&dir.join("absent"))).is_ok();
}

/// Un lien dans l'arbre est retiré, jamais suivi : sa cible reste.
#[cfg(unix)]
#[test]
fn delete_dir_nested_symlink_removes_the_link_not_its_target() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("rmdir-nested-link");
    let dir = tmp.path();
    let victim = dir.join("victim");
    s.write(&victim.join("precious.txt"), b"keep".to_vec()).unwrap();
    s.write(&dir.join("t-1/deck/document.json"), vec![1]).unwrap();
    std::os::unix::fs::symlink(&victim, dir.join("t-1/deck/assets")).unwrap();
    s.delete_dir(&dir.join("t-1")).unwrap();
    assert_that!(dir.join("t-1").exists()).is_false();
    assert_that!(s.read(&victim.join("precious.txt")).unwrap()).is_equal_to(b"keep".to_vec());
}

/// Un lien vers un fichier n'est pas un dossier : seul le lien part.
#[cfg(unix)]
#[test]
fn delete_dir_symlink_to_file_removes_only_the_link() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("rmdir-file-link");
    let dir = tmp.path();
    s.write(&dir.join("target.txt"), b"keep".to_vec()).unwrap();
    std::os::unix::fs::symlink(dir.join("target.txt"), dir.join("t-2")).unwrap();
    s.delete_dir(&dir.join("t-2")).unwrap();
    assert_that!(dir.join("t-2").symlink_metadata().is_err()).is_true();
    assert_that!(s.read(&dir.join("target.txt")).unwrap()).is_equal_to(b"keep".to_vec());
}

#[test]
fn read_limited_within_the_limit_returns_the_bytes() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("limited");
    let dir = tmp.path();
    let file = dir.join("state.json");
    s.write(&file, b"{}".to_vec()).unwrap();
    assert_that!(s.read_limited(&file, 2).unwrap()).is_equal_to(b"{}".to_vec());
}

#[test]
fn read_limited_empty_file_returns_no_bytes() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("limited-empty");
    let dir = tmp.path();
    let file = dir.join("state.json");
    s.write(&file, Vec::new()).unwrap();
    assert_that!(s.read_limited(&file, 10).unwrap()).is_empty();
}

#[test]
fn read_limited_one_byte_over_returns_too_large() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("limited-over");
    let dir = tmp.path();
    let file = dir.join("state.json");
    s.write(&file, b"{ }".to_vec()).unwrap();
    assert_that!(s.read_limited(&file, 2).unwrap_err().kind()).is_equal_to(StorageErrorKind::TooLarge);
}

#[test]
fn read_limited_missing_file_returns_not_found() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("limited-absent");
    let err = s.read_limited(&tmp.path().join("x"), 10).unwrap_err();
    assert_that!(err.kind()).is_equal_to(StorageErrorKind::NotFound);
}

#[test]
fn list_files_missing_dir_returns_empty_and_skips_subdirs() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("list");
    let dir = tmp.path();
    assert_that!(s.list_files(dir).unwrap()).is_empty();
    s.write(&dir.join("x.json"), b"{}".to_vec()).unwrap();
    s.create_dir_all(&dir.join("sub")).unwrap();
    assert_that!(s.list_files(dir).unwrap()).is_equal_to(vec![dir.join("x.json")]);
}

#[test]
fn list_dirs_returns_only_subdirectories() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("dirs");
    let dir = tmp.path();
    s.write(&dir.join("a/x.bin"), vec![1]).unwrap();
    s.write(&dir.join("b/y.bin"), vec![1]).unwrap();
    s.write(&dir.join("z.bin"), vec![1]).unwrap();
    let mut names: Vec<String> =
        s.list_dirs(dir).unwrap().iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    names.sort();
    assert_that!(names).is_equal_to(vec!["a".to_owned(), "b".to_owned()]);
    assert_that!(s.list_dirs(&dir.join("absent")).unwrap()).is_empty();
}

#[test]
fn read_fonts_keeps_only_font_files() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("fonts");
    let dir = tmp.path();
    s.write(&dir.join("Inter.TTF"), vec![0; 4]).unwrap();
    s.write(&dir.join("notes.txt"), vec![0; 4]).unwrap();
    let fonts = s.read_fonts(dir).unwrap();
    assert_that!(fonts.keys().map(String::as_str).collect::<Vec<_>>()).is_equal_to(vec!["Inter.TTF"]);
}

#[test]
fn writer_streams_chunks_then_rename_and_delete_work() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("stream");
    let dir = tmp.path();
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
}

#[test]
fn read_path_with_parent_components_resolves_them() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("dotdot");
    let dir = tmp.path();
    s.write(&dir.join("x/file"), b"ok".to_vec()).unwrap();
    assert_that!(s.read(&dir.join("x/../x/./file")).unwrap()).is_equal_to(b"ok".to_vec());
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
fn resolve_makes_the_path_absolute_and_drops_dot_segments() {
    let cwd = std::env::current_dir().unwrap();
    assert_that!(resolve(Path::new("a/./b/../c.pdf"))).is_equal_to(cwd.join("a/c.pdf"));
    assert_that!(resolve(Path::new("/x/y/../z"))).is_equal_to(PathBuf::from("/x/z"));
}

#[test]
fn resolve_never_climbs_above_the_root() {
    assert_that!(resolve(Path::new("/../x"))).is_equal_to(PathBuf::from("/x"));
}

#[test]
fn at_same_root_reuses_one_operator() {
    let s = Storage::local().unwrap();
    s.exists(Path::new("/tmp/a")).unwrap();
    s.exists(Path::new("/usr/b")).unwrap();
    assert_that!(s.roots.lock().unwrap().len()).is_equal_to(1);
}

/// Clone sans Git LFS : les polices ne sont que des pointeurs texte. Jamais
/// passées au rendu comme des polices (une erreur, avec le remède, est
/// journalisée) ; les vraies polices du dossier restent.
#[test]
fn read_fonts_skips_git_lfs_pointers() {
    let s = Storage::local().unwrap();
    let tmp = temp_dir("fonts-lfs");
    let dir = tmp.path();
    let pointer = b"version https://git-lfs.github.com/spec/v1\noid sha256:5d3c\nsize 66592\n".to_vec();
    s.write(&dir.join("inter-400.ttf"), pointer).unwrap();
    s.write(&dir.join("noto.otf"), vec![0; 4]).unwrap();
    let fonts = s.read_fonts(dir).unwrap();
    assert_that!(fonts.keys().map(String::as_str).collect::<Vec<_>>()).is_equal_to(vec!["noto.otf"]);
}
