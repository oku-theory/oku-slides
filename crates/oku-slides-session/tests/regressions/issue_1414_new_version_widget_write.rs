//! t-1414: a widget edit never writes a received new version of a known pack into the library.
//!
//! The write gate once checked that the library knew the pack's id, not the pinned
//! version: editing a widget of a received new version wrote its child into the
//! library and made it current, as if the user had validated the received version.

use std::path::{Path, PathBuf};

use oku_slides_core::{Op, PackKind, PackPin};
use oku_slides_session::{PackLibrary, PackManifest, Request, SessionErrorKind, Storage};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{add_user_pack, data_dir, no_user_themes, ns, session_in};

const WIDGET: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich()),
  (args, ctx) => field(ctx, "body", strong(args.body)))
"#;

fn library(data: &Path) -> PackLibrary {
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    PackLibrary::new(Some(data.join("widgets")), None, no_user_themes(), storage)
}

/// A file saved by someone else whose deck pins their pack `quote`; its id.
fn received(them: &Path) -> (PathBuf, Uuid) {
    let id = add_user_pack(them, "quote");
    let version = library(them).current(id).unwrap_or_else(|| panic!("pack"));
    let mut s = session_in(them);
    let pin = PackPin { id, version, requires: Vec::new() };
    s.apply("Pack", vec![Op::AddPack { kind: PackKind::Widgets, namespace: ns("quote"), pin }], None)
        .unwrap_or_else(|e| panic!("{e}"));
    let path = them.join("received.oku-slides.pdf");
    s.handle(Request::Save { path: Some(path.clone()), overwrite: false }).unwrap_or_else(|e| panic!("{e}"));
    (path, id)
}

/// Puts in `me`'s library another version of their pack `id`: the received one is then a new version.
fn known_to_me(me: &Path, them: &Path, id: Uuid) -> oku_slides_core::PackVersion {
    let theirs = library(them).current_version(id).unwrap_or_else(|| panic!("pack"));
    let mine = PackManifest { created: "2026-09-01T10:00:00Z".into(), ..theirs.manifest };
    let files = mine.to_files(PackKind::Widgets, theirs.files.widgets().clone()).unwrap_or_else(|e| panic!("{e}"));
    library(me).add_version(id, &files).unwrap_or_else(|e| panic!("{e}"));
    files.version()
}

#[test]
fn issue_1414_widget_write_on_a_received_new_version_is_refused_and_current_unchanged() {
    let (me, them) = (data_dir(), data_dir());
    let (path, id) = received(&them);
    let mine = known_to_me(&me, &them, id);
    let mut s = session_in(&me);
    s.open(&path).unwrap_or_else(|e| panic!("{e}"));
    let write = Request::SetWidget { pack: Some(ns("quote")), name: "quote".into(), source: WIDGET.into() };
    let err = s.handle(write).err().map(|e| e.kind());
    assert_that!(err).is_equal_to(Some(SessionErrorKind::PackNotInstalled));
    let lib = library(&me);
    assert_that!(lib.current(id)).is_equal_to(Some(mine));
    assert_that!(lib.versions(id).len()).is_equal_to(1);
}
