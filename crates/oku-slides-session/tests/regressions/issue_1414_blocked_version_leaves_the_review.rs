//! t-1414: validating the review takes every answered version off the list, blocked ones too.
//!
//! A version the library refuses (here, its namespace is taken) stayed listed after
//! "Accept": the dialog never closed and its button turned into a no-op.

use std::path::{Path, PathBuf};

use oku_slides_core::{Op, PackKind, PackPin};
use oku_slides_session::{PackLibrary, Request, Storage, VersionRef};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{add_user_pack, data_dir, no_user_themes, ns, session_in, snapshot_json};

fn library(data: &Path) -> PackLibrary {
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    PackLibrary::new(Some(data.join("widgets")), None, no_user_themes(), storage)
}

/// A file saved by someone else whose deck pins their pack `quote`; its id and version.
fn received(them: &Path) -> (PathBuf, VersionRef) {
    let id = add_user_pack(them, "quote");
    let version = library(them).current(id).unwrap_or_else(|| panic!("pack"));
    let mut s = session_in(them);
    let pin = PackPin { id, version: version.clone(), requires: Vec::new() };
    s.apply("Pack", vec![Op::AddPack { kind: PackKind::Widgets, namespace: ns("quote"), pin }], None)
        .unwrap_or_else(|e| panic!("{e}"));
    let path = them.join("received.oku-slides.pdf");
    s.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap_or_else(|e| panic!("{e}"));
    (path, VersionRef { id, version })
}

#[test]
fn issue_1414_accept_of_a_blocked_version_empties_the_review() {
    let (me, them) = (data_dir(), data_dir());
    let (path, theirs) = received(&them);
    // My own pack already holds the namespace: theirs is blocked.
    add_user_pack(&me, "quote");
    let mut s = session_in(&me);
    s.open(&path).unwrap_or_else(|e| panic!("{e}"));
    let review = snapshot_json(&mut s)["received_to_review"].clone();
    assert_that!(review[0]["install"].as_bool()).is_equal_to(Some(false));
    s.handle(Request::AcceptReceived { versions: vec![theirs.clone()] }).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(snapshot_json(&mut s)["received_to_review"].as_array().map(Vec::len)).is_equal_to(Some(0));
    assert_that!(library(&me).versions(theirs.id)).is_empty();
}

#[test]
fn issue_1414_accept_of_a_version_not_held_is_refused_by_id_not_uuid_text() {
    let (me, them) = (data_dir(), data_dir());
    let (path, theirs) = received(&them);
    let mut s = session_in(&me);
    s.open(&path).unwrap_or_else(|e| panic!("{e}"));
    let forged = VersionRef { id: Uuid::new_v4(), version: theirs.version };
    let err = s.handle(Request::AcceptReceived { versions: vec![forged] }).err().map(|e| e.kind());
    assert_that!(err).is_equal_to(Some(oku_slides_session::SessionErrorKind::NotHeld));
}
