use oku_slides_session::{DEFAULT_PACK, GRID_PACK, PackLibrary};
use speculoos::prelude::*;

use super::*;

/// Une bibliothèque de packs vide dans un dossier temporaire (les intégrés seulement).
fn library(dir: &Path) -> (Storage, PackLibrary) {
    let storage = Storage::local().unwrap();
    let library = PackLibrary::new(Some(dir.join("templates")), storage.clone());
    (storage, library)
}

/// La version intégrée du pack Grilles et sa référence.
fn grid_ref() -> (PackFiles, PackRef) {
    let files = PackFiles::from_bytes(include_str!("../../../templates/grilles/pack.json").as_bytes(), []).unwrap();
    let r = PackRef { id: GRID_PACK, version: files.version() };
    (files, r)
}

#[test]
fn deck_pack_version_without_copy_beside_the_deck_takes_the_library_one() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let (files, r) = grid_ref();
    let got = deck_pack_version(&storage, &library, (&tmp.path().join("deck"), 1 << 20), &r);
    assert_that!(got).is_equal_to(Some(files));
}

/// t-355 : la copie à côté du deck d'un autre pack est ignorée, et la
/// bibliothèque, consultée ensuite, passe par la même vérification d'id.
#[test]
fn deck_pack_version_of_another_pack_is_ignored_wherever_it_comes_from() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let (files, grid) = grid_ref();
    let r = PackRef { id: DEFAULT_PACK, version: grid.version };
    let at = tmp.path().join("deck").join("packs").join(r.version.hex());
    std::fs::create_dir_all(&at).unwrap();
    std::fs::write(at.join("pack.json"), files.pack_json().unwrap()).unwrap();
    assert_that!(deck_pack_version(&storage, &library, (&tmp.path().join("deck"), 1 << 20), &r)).is_none();
}
