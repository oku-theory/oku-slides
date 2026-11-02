//! t-1414: a received pack enters the library only on the user's validation.
//!
//! Before t-1414, opening or merging a file added every library pack it embedded to the
//! user's library, silently: a deck could plant packs offered to every other deck.

use std::path::{Path, PathBuf};

use oku_slides_core::{Op, PackKind, PackPin};
use oku_slides_session::{PackLibrary, Request, Storage};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{add_user_pack, data_dir, no_user_themes, session_in, snapshot_json};

fn library(data: &Path) -> PackLibrary {
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    PackLibrary::new(Some(data.join("widgets")), None, no_user_themes(), storage)
}

/// A file saved by someone else whose deck pins their pack `id`, named `quote`.
fn received(them: &Path) -> (PathBuf, Uuid) {
    pinning_their_pack(them, session_in(them), "received")
}

/// `s`, on `them`'s data, pins their new pack `quote` and saves as `name`: the file and the pack id.
fn pinning_their_pack(them: &Path, mut s: oku_slides_session::Session, name: &str) -> (PathBuf, Uuid) {
    let id = add_user_pack(them, "quote");
    let version = library(them).current(id).unwrap_or_else(|| panic!("pack"));
    let pin = PackPin { id, version, requires: Vec::new() };
    let namespace = crate::common::ns("quote");
    s.apply("Pack", vec![Op::AddPack { kind: PackKind::Widgets, namespace, pin }], None)
        .unwrap_or_else(|e| panic!("{e}"));
    let path = them.join(format!("{name}.oku-slides.pdf"));
    s.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap_or_else(|e| panic!("{e}"));
    (path, id)
}

#[test]
fn issue_1414_open_leaves_the_library_without_the_received_pack() {
    let (me, them) = (data_dir(), data_dir());
    let (path, id) = received(&them);
    let mut s = session_in(&me);
    s.open(&path).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(library(&me).versions(id)).is_empty();
    let review = snapshot_json(&mut s)["received_to_review"].clone();
    assert_that!(review[0]["id"].as_str()).is_equal_to(Some(id.to_string().as_str()));
    assert_that!(review[0]["install"].as_bool()).is_equal_to(Some(true));
}

/// The merge alone brings the pack: the deck opened first never held it.
#[test]
fn issue_1414_merge_leaves_the_library_without_the_received_pack() {
    let (me, them) = (data_dir(), data_dir());
    let base = them.join("base.oku-slides.pdf");
    session_in(&them)
        .handle(Request::Save { path: Some(base.clone()), overwrite: false })
        .unwrap_or_else(|e| panic!("{e}"));
    let mut s = session_in(&me);
    s.open(&base).unwrap_or_else(|e| panic!("{e}"));
    let mut theirs = session_in(&them);
    theirs.open(&base).unwrap_or_else(|e| panic!("{e}"));
    let (path, id) = pinning_their_pack(&them, theirs, "next");
    s.handle(Request::Merge { path }).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(library(&me).versions(id)).is_empty();
    let review = snapshot_json(&mut s)["received_to_review"].clone();
    assert_that!(review[0]["id"].as_str()).is_equal_to(Some(id.to_string().as_str()));
}
