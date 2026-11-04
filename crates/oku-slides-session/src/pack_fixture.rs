//! A deck pinning a pack the library holds a newer version of, for other crates' tests.
#![expect(clippy::panic, reason = "a test-support fixture: a setup that fails is the calling test failing")]

use oku_slides_core::{Guides, Namespace, Op, PackKind, PackPin, PackScope};

use crate::Session;
use crate::pack::PackManifest;

/// Pins a new user widget pack in an older version than the library's current one.
///
/// The library of `s` gets v0, then v1 (current); the deck pins v0, so the packs panel
/// offers "Use mine" and "Make current" on it. Returns the namespace it is pinned under.
///
/// # Panics
///
/// If the library or the deck refuses the pack: a broken test setup.
pub fn pin_behind_library(s: &mut Session) -> Namespace {
    let id = uuid::Uuid::new_v4();
    let namespace =
        Namespace::parse(&format!("t{}", id.simple())).unwrap_or_else(|| panic!("namespace of a test pack: {id}"));
    let manifest = |parent: Option<&oku_slides_core::PackVersion>, created: &str| PackManifest {
        id,
        name: "Charte".into(),
        namespace: namespace.clone(),
        requires: vec![],
        description: String::new(),
        parent: parent.cloned(),
        created: created.into(),
        scope: PackScope::Library,
        profiles: Vec::new(),
        examples: std::collections::BTreeMap::default(),
        styles: std::collections::BTreeMap::default(),
        tokens: Vec::new(),
        order: Vec::new(),
        guides: Guides::default(),
    };
    let files = |m: PackManifest| {
        m.to_files(PackKind::Widgets, std::collections::BTreeMap::new())
            .unwrap_or_else(|e| panic!("test pack files: {e}"))
    };
    let v0 = files(manifest(None, "2026-10-01T10:00:00Z"));
    let v1 = files(manifest(Some(&v0.version()), "2026-10-02T10:00:00Z"));
    s.library.add_version(id, &v0).unwrap_or_else(|e| panic!("test pack v0: {}", e.one_line()));
    let pin = PackPin { id, version: v0.version(), requires: Vec::new() };
    let op = Op::AddPack { kind: PackKind::Widgets, namespace: namespace.clone(), pin };
    s.apply("Pack", vec![op], None).unwrap_or_else(|e| panic!("test pack pin: {}", e.one_line()));
    s.library.add_version(id, &v1).unwrap_or_else(|e| panic!("test pack v1: {}", e.one_line()));
    s.library.set_current(id, &v1.version()).unwrap_or_else(|e| panic!("test pack current: {}", e.one_line()));
    namespace
}
