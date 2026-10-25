//! t-1091 : les besoins d'un pack se résolvaient par espace de noms parmi toutes les entrées
//! de la bibliothèque, sans la règle des espaces de noms intégrés (R15).
//!
//! Une ancienne copie de l'utilisateur sous un espace de noms intégré n'est jamais un résultat
//! de recherche : un pack qui requiert `base` épingle le `base` intégré, et un besoin
//! `essentiel` (pack de layouts intégré, qu'un besoin ne peut nommer) est refusé au lieu de
//! prendre la copie rangée parmi les widgets (comme celle de t-1068).

use std::collections::BTreeMap;
use std::path::Path;

use oku_slides_core::{Namespace, PackFiles, PackKind, PackScope};
use oku_slides_session::{BASE_PACK, PackLibrary, PackManifest, Storage};
use speculoos::prelude::*;
use uuid::Uuid;

use crate::common::{data_dir, no_user_themes, ns, session_in};

/// L'id de l'ancienne copie d'un pack intégré.
const LEGACY_ID: Uuid = Uuid::from_u128(0x10910000_0000_4000_8000_000000000001);

fn manifest(id: Uuid, namespace: &str, name: &str, requires: &[&str]) -> PackManifest {
    PackManifest {
        id,
        name: name.into(),
        namespace: ns(namespace),
        requires: requires.iter().map(|r| ns(r)).collect(),
        description: String::new(),
        parent: None,
        created: "2026-10-01T00:00:00Z".into(),
        scope: PackScope::Library,
        profiles: Vec::new(),
        examples: BTreeMap::default(),
        styles: BTreeMap::default(),
        tokens: Vec::new(),
    }
}

/// Range sous `<data>/widgets/` une copie d'un autre id sous l'espace de noms `namespace`, comme l'écrivaient les lots 1-2.
fn store_legacy_copy(data: &Path, namespace: &str) {
    // « A… » : rangée avant tout autre pack de l'utilisateur par nom.
    let files: PackFiles = manifest(LEGACY_ID, namespace, "A copie", &[])
        .to_files(PackKind::Widgets, BTreeMap::new())
        .unwrap_or_else(|e| panic!("{e}"));
    let dir = data.join("widgets").join(LEGACY_ID.to_string());
    let at = dir.join("versions").join(files.version().hex());
    std::fs::create_dir_all(&at).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(at.join("widgets.json"), files.pack_json().unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(dir.join("current"), files.version().as_str()).unwrap_or_else(|e| panic!("{e}"));
}

/// Une bibliothèque sous `data` qui tient la copie sous `legacy` et le pack `acme`, qui requiert `required`.
fn library_with_acme(data: &Path, legacy: &str, required: &str) -> Uuid {
    store_legacy_copy(data, legacy);
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    let library = PackLibrary::new(Some(data.join("widgets")), Some(data.join("layouts")), no_user_themes(), storage);
    let acme = manifest(Uuid::new_v4(), "acme", "Acme", &[required]);
    let files = acme.to_files(PackKind::Widgets, BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    library.add_version(acme.id, &files).unwrap_or_else(|e| panic!("{}", e.one_line()));
    acme.id
}

#[test]
fn issue_1091_pack_requiring_base_pins_the_builtin_base_despite_a_legacy_copy() {
    let data = data_dir();
    let acme = library_with_acme(&data, "base", "base");
    let mut s = session_in(&data);
    s.edit_pack(acme).unwrap_or_else(|e| panic!("atelier : {}", e.one_line()));
    let base = Namespace::parse("base").map(|b| s.deck().widgets.get(&b).map(|p| p.id));
    assert_that!(base).is_equal_to(Some(Some(BASE_PACK)));
}

#[test]
fn issue_1091_requirement_on_a_legacy_essentiel_copy_is_refused_not_resolved() {
    let data = data_dir();
    let acme = library_with_acme(&data, "essentiel", "essentiel");
    let mut s = session_in(&data);
    let edited = s.edit_pack(acme).map_err(|e| e.kind());
    assert_that!(edited).is_equal_to(Err(oku_slides_session::SessionErrorKind::UnknownNamespace));
    assert_that!(s.deck().all_pins().any(|(_, _, pin)| pin.id == LEGACY_ID)).is_false();
}
