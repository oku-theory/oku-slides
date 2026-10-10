use oku_slides_core::PackManifest;
use oku_slides_i18n::english;
use oku_slides_session::{DEFAULT_PACK, GRID_PACK};
use speculoos::prelude::*;

use super::*;

/// Une bibliothèque de packs vide dans un dossier temporaire (les intégrés seulement).
fn library(dir: &Path) -> (Storage, PackLibrary) {
    let storage = Storage::local().unwrap();
    let library = PackLibrary::new(Some(dir.join("templates")), storage.clone());
    (storage, library)
}

fn pack_id(files: &PackFiles) -> uuid::Uuid {
    PackManifest::from_files(files).unwrap().id
}

/// Écrit dans `dir` le `pack.json` du pack intégré Grilles et un widget `citation`.
fn write_pack_dir(dir: &Path) {
    let json = include_str!("../../../templates/grilles/pack.json");
    std::fs::create_dir_all(dir.join("widgets")).unwrap();
    std::fs::write(dir.join("pack.json"), json).unwrap();
    std::fs::write(dir.join("widgets").join("citation.typ"), "// widget").unwrap();
}

/// Des sources vides, sur le registre de base figé.
fn empty_bundle() -> SourceBundle {
    let probe = serde_json::from_str(include_str!("../../../fixtures/widgets/base-registry.json")).unwrap();
    let registry = std::sync::Arc::new(oku_slides_core::WidgetRegistry::from_probe(probe).unwrap());
    SourceBundle::new(oku_slides_core::Deck::empty(), registry).unwrap()
}

#[test]
fn read_pack_dir_reads_the_manifest_and_the_widget_files() {
    let tmp = tempfile::tempdir().unwrap();
    write_pack_dir(tmp.path());
    let (storage, _) = library(tmp.path());
    let files = read_pack_dir(&storage, tmp.path()).unwrap();
    let names: Vec<String> = files.widgets().keys().map(|w| w.file_name()).collect();
    assert_that!((pack_id(&files), names)).is_equal_to((GRID_PACK, vec!["citation.typ".to_owned()]));
}

#[test]
fn read_pack_dir_without_pack_json_is_none() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, _) = library(tmp.path());
    assert_that!(read_pack_dir(&storage, tmp.path())).is_none();
}

#[test]
fn resolve_pack_by_name_ignores_case() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let files = resolve_pack(&storage, &library, english(), "essentiel").unwrap();
    assert_that!(pack_id(&files)).is_equal_to(DEFAULT_PACK);
}

#[test]
fn resolve_pack_by_id_takes_the_current_version() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let files = resolve_pack(&storage, &library, english(), &GRID_PACK.to_string()).unwrap();
    assert_that!(files.version()).is_equal_to(library.current_version(GRID_PACK).unwrap().version);
}

#[test]
fn resolve_pack_by_folder_reads_it() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("mon-pack");
    write_pack_dir(&dir);
    let (storage, library) = library(tmp.path());
    let files = resolve_pack(&storage, &library, english(), dir.to_str().unwrap()).unwrap();
    assert_that!(files.widgets().len()).is_equal_to(1);
}

#[test]
fn resolve_pack_unknown_fails_with_a_localized_message() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let err = resolve_pack(&storage, &library, english(), "Nope").unwrap_err();
    assert_that!(err.to_string()).is_equal_to("unknown pack: Nope".to_owned());
}

#[test]
fn resolve_pack_unknown_id_fails_too() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let id = "00000000-0000-4000-8000-000000000000";
    assert_that!(resolve_pack(&storage, &library, english(), id)).is_err();
}

#[test]
fn embed_theme_by_name_or_id_embeds_the_same_version() {
    let tmp = tempfile::tempdir().unwrap();
    let themes = ThemeLibrary::new(Some(tmp.path().join("themes")), Storage::local().unwrap());
    let mut bundle = empty_bundle();
    let by_name = embed_theme(&themes, english(), "PRO", &mut bundle).unwrap();
    let by_id = embed_theme(&themes, english(), &oku_slides_session::PRO_THEME.to_string(), &mut bundle).unwrap();
    assert_that!(by_name).is_equal_to(by_id.clone());
    assert_that!(by_id.id).is_equal_to(oku_slides_session::PRO_THEME);
}

#[test]
fn embed_theme_unknown_fails_with_a_localized_message() {
    let tmp = tempfile::tempdir().unwrap();
    let themes = ThemeLibrary::new(Some(tmp.path().join("themes")), Storage::local().unwrap());
    let mut bundle = empty_bundle();
    let err = embed_theme(&themes, english(), "Nope", &mut bundle).unwrap_err();
    assert_that!(err.to_string()).is_equal_to("Unknown theme: Nope".to_owned());
}
