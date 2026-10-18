use std::ffi::OsString;
use std::path::{Path, PathBuf};

use speculoos::prelude::*;

use super::find_program;
use crate::storage::Storage;

/// Un dossier `bin` neuf contenant le fichier `name`.
fn bin_with(name: &str) -> (tempfile::TempDir, PathBuf) {
    let tmp = crate::test_support::fresh_dir("oku-slides-program-");
    let bin = tmp.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join(name), b"#!/bin/sh\n").unwrap();
    (tmp, bin)
}

/// Une variable `PATH` faite de `dirs`.
fn path_of(dirs: &[&Path]) -> OsString {
    std::env::join_paths(dirs).unwrap()
}

#[test]
fn find_program_bare_name_on_path_returns_its_full_path() {
    let (_tmp, bin) = bin_with("agent-acp");
    let empty = crate::test_support::fresh_dir("oku-slides-program-empty-");
    let path = path_of(&[empty.path(), &bin]);
    let found = find_program(&Storage::local().unwrap(), Path::new("agent-acp"), Some(&path));
    assert_that!(found).is_equal_to(Some(bin.join("agent-acp")));
}

#[test]
fn find_program_bare_name_missing_from_path_returns_none() {
    let (_tmp, bin) = bin_with("autre");
    let path = path_of(&[&bin]);
    let found = find_program(&Storage::local().unwrap(), Path::new("agent-acp"), Some(&path));
    assert_that!(found).is_none();
}

#[test]
fn find_program_bare_name_without_path_returns_none() {
    let found = find_program(&Storage::local().unwrap(), Path::new("agent-acp"), None);
    assert_that!(found).is_none();
}

#[test]
fn find_program_absolute_path_present_is_returned_as_is() {
    let (_tmp, bin) = bin_with("agent-acp");
    let program = bin.join("agent-acp");
    // Un chemin n'est jamais cherché dans `PATH`, même vide.
    let found = find_program(&Storage::local().unwrap(), &program, None);
    assert_that!(found).is_equal_to(Some(program));
}

#[test]
fn find_program_absolute_path_missing_returns_none() {
    let (_tmp, bin) = bin_with("autre");
    let path = path_of(&[&bin]);
    let found = find_program(&Storage::local().unwrap(), &bin.join("agent-acp"), Some(&path));
    assert_that!(found).is_none();
}

#[test]
fn find_program_skips_empty_path_entries() {
    // Le dossier courant des tests est la racine du crate, où est `Cargo.toml` : une entrée vide l'y chercherait.
    let (_tmp, bin) = bin_with("autre");
    let path = OsString::from(format!("::{}", bin.display()));
    let found = find_program(&Storage::local().unwrap(), Path::new("Cargo.toml"), Some(&path));
    assert_that!(found).is_none();
}

#[test]
fn find_program_skips_relative_path_entries() {
    // `src/lib.rs` existe sous le dossier courant des tests (la racine du crate).
    let (_tmp, bin) = bin_with("autre");
    let path = OsString::from(format!("src:{}", bin.display()));
    let found = find_program(&Storage::local().unwrap(), Path::new("lib.rs"), Some(&path));
    assert_that!(found).is_none();
}

#[test]
fn find_program_directory_named_like_the_program_is_not_found() {
    let (_tmp, bin) = bin_with("autre");
    std::fs::create_dir_all(bin.join("agent-acp")).unwrap();
    let path = path_of(&[&bin]);
    let found = find_program(&Storage::local().unwrap(), Path::new("agent-acp"), Some(&path));
    assert_that!(found).is_none();
}

#[test]
fn find_program_absolute_path_to_a_directory_is_not_found() {
    let (_tmp, bin) = bin_with("autre");
    let found = find_program(&Storage::local().unwrap(), &bin, None);
    assert_that!(found).is_none();
}
