use oku_slides_core::{Deck, kinds};
use oku_slides_session::{DEFAULT_PACK, GRID_PACK, PackLibrary};
use speculoos::prelude::*;

use super::*;

/// Une bibliothèque de packs vide dans un dossier temporaire (les intégrés seulement).
fn library(dir: &Path) -> (Storage, PackLibrary) {
    let storage = Storage::local().unwrap();
    let library = PackLibrary::new(Some(dir.join("templates")), storage.clone());
    (storage, library)
}

/// Un deck qui épingle `pin` sous l'espace de noms `grilles`.
fn deck_pinning(pin: PackPin) -> Deck {
    let mut deck = Deck::empty();
    deck.widgets.insert(Namespace::parse("grilles").unwrap(), pin);
    deck
}

/// La version intégrée du pack Grilles et son épinglage.
fn grid_ref() -> (PackFiles, PackPin) {
    let files = PackFiles::from_bytes(include_str!("../../../templates/grilles/pack.json").as_bytes(), []).unwrap();
    // Grilles requiert `base` : le manifeste le dit, l'épinglage aussi.
    let r = PackPin { id: GRID_PACK, version: files.version(), requires: vec![kinds::base::namespace()] };
    (files, r)
}

#[test]
fn deck_pack_versions_without_copy_beside_the_deck_take_the_library_one() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let (files, r) = grid_ref();
    let got = deck_pack_versions(&storage, &library, (&tmp.path().join("deck"), 1 << 20), &deck_pinning(r.clone()));
    assert_that!(got).is_equal_to(Ok(BTreeMap::from([(r.version, files)])));
}

/// t-355 : la copie à côté du deck d'un autre pack est ignorée, et la
/// bibliothèque, consultée ensuite, passe par la même vérification d'id.
#[test]
fn deck_pack_versions_of_another_pack_is_ignored_wherever_it_comes_from() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let (files, grid) = grid_ref();
    let r = PackPin { id: DEFAULT_PACK, version: grid.version, requires: vec![] };
    let at = tmp.path().join("deck").join("widgets").join(r.version.hex());
    std::fs::create_dir_all(&at).unwrap();
    std::fs::write(at.join("widgets.json"), files.pack_json().unwrap()).unwrap();
    let got = deck_pack_versions(&storage, &library, (&tmp.path().join("deck"), 1 << 20), &deck_pinning(r));
    assert_that!(got.is_err()).is_true();
}
