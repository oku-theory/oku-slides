use oku_slides_core::{Deck, kinds};
use oku_slides_session::{BASE_PACK, ESSENTIEL_PACK, PackLibrary};
use speculoos::prelude::*;

use super::*;

/// Une bibliothèque de packs vide dans un dossier temporaire (les intégrés seulement).
fn library(dir: &Path) -> (Storage, PackLibrary) {
    let storage = Storage::local().unwrap();
    let library = PackLibrary::new(Some(dir.join("widgets")), Some(dir.join("layouts")), storage.clone());
    (storage, library)
}

/// Un deck qui épingle `pin` sous l'espace de noms `essentiel`, parmi ses packs de layouts.
fn deck_pinning(pin: PackPin) -> Deck {
    let mut deck = Deck::empty();
    deck.layouts.insert(kinds::essentiel::namespace(), pin);
    deck
}

/// La version intégrée du pack de layouts `essentiel` et son épinglage.
fn essentiel_ref(library: &PackLibrary) -> (PackFiles, PackPin) {
    let files = library.current_version(ESSENTIEL_PACK).unwrap().files;
    // `essentiel` requiert `base` : le manifeste le dit, l'épinglage aussi.
    let r = PackPin { id: ESSENTIEL_PACK, version: files.version(), requires: vec![kinds::base::namespace()] };
    (files, r)
}

#[test]
fn deck_pack_versions_of_a_layout_pack_without_copy_beside_the_deck_take_the_library_one() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let (files, r) = essentiel_ref(&library);
    let got = deck_pack_versions(&storage, &library, (&tmp.path().join("deck"), 1 << 20), &deck_pinning(r.clone()));
    assert_that!(got).is_equal_to(Ok(BTreeMap::from([(r.version, files)])));
}

/// t-355 : la copie à côté du deck d'un autre pack est ignorée, et la
/// bibliothèque, consultée ensuite, passe par la même vérification d'id.
#[test]
fn deck_pack_versions_of_another_pack_is_ignored_wherever_it_comes_from() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let (files, essentiel) = essentiel_ref(&library);
    let r = PackPin { id: BASE_PACK, version: essentiel.version, requires: vec![] };
    let at = tmp.path().join("deck").join("layouts").join(r.version.hex());
    std::fs::create_dir_all(at.join("layouts")).unwrap();
    std::fs::write(at.join("layouts.json"), files.pack_json().unwrap()).unwrap();
    for (name, source) in files.widgets() {
        std::fs::write(at.join("layouts").join(name.file_name()), source).unwrap();
    }
    let got = deck_pack_versions(&storage, &library, (&tmp.path().join("deck"), 1 << 20), &deck_pinning(r));
    assert_that!(got.is_err()).is_true();
}
