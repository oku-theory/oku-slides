use oku_slides_core::PackManifest;
use oku_slides_i18n::Localize;
use speculoos::prelude::*;

use super::*;
use crate::import::ImportErrorKind;
use crate::{BASE_PACK, ESSENTIEL_PACK, PRO_THEME};

/// L'interface en français : les erreurs se testent hors de l'anglais.
fn fr() -> &'static oku_slides_i18n::Localizer {
    static FR: std::sync::LazyLock<oku_slides_i18n::Localizer> =
        std::sync::LazyLock::new(|| oku_slides_i18n::Localizer::new(Some("fr")));
    &FR
}

/// Une bibliothèque de packs vide dans un dossier temporaire (les intégrés seulement).
fn library(dir: &Path) -> (Storage, PackLibrary) {
    let storage = Storage::local().unwrap();
    let library = PackLibrary::new(
        Some(dir.join("widgets")),
        Some(dir.join("layouts")),
        crate::ThemeLibrary::builtin_only(),
        storage.clone(),
    );
    (storage, library)
}

/// Un budget de lecture qui ne gêne pas.
const MAX: usize = 1 << 20;

fn pack_id(files: &PackFiles) -> uuid::Uuid {
    PackManifest::from_files(files).unwrap().id
}

/// Écrit dans `dir` le manifeste du pack intégré `essentiel` (`layouts.json`) et un layout `citation`.
fn write_pack_dir(dir: &Path) {
    let json = crate::test_support::builtin(ESSENTIEL_PACK).files.pack_json().unwrap();
    std::fs::create_dir_all(dir.join("layouts")).unwrap();
    std::fs::write(dir.join("layouts.json"), json).unwrap();
    std::fs::write(dir.join("layouts").join("citation.typ"), "// widget").unwrap();
}

/// Des sources vides, sur le registre de base figé.
fn empty_bundle() -> SourceBundle {
    SourceBundle::new(oku_slides_core::Deck::empty(), crate::test_support::registry()).unwrap()
}

#[test]
fn read_pack_dir_reads_the_manifest_and_the_widget_files() {
    let tmp = tempfile::tempdir().unwrap();
    write_pack_dir(tmp.path());
    let (storage, _) = library(tmp.path());
    let files = read_pack_dir(&storage, tmp.path(), MAX).unwrap().unwrap();
    let names: Vec<String> = files.widgets().keys().map(oku_slides_core::WidgetFile::file_name).collect();
    assert_that!((pack_id(&files), files.kind(), names)).is_equal_to((
        ESSENTIEL_PACK,
        oku_slides_core::PackKind::Layouts,
        vec!["citation.typ".to_owned()],
    ));
}

#[test]
fn read_pack_dir_without_a_manifest_is_none() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, _) = library(tmp.path());
    assert_that!(read_pack_dir(&storage, tmp.path(), MAX).unwrap()).is_none();
}

#[test]
fn read_pack_dir_beyond_its_budget_is_too_large() {
    let tmp = tempfile::tempdir().unwrap();
    write_pack_dir(tmp.path());
    let (storage, _) = library(tmp.path());
    let size = usize::try_from(std::fs::metadata(tmp.path().join("layouts.json")).unwrap().len()).unwrap();
    // Le layouts.json tient, le layout (10 octets) non.
    let err = read_pack_dir(&storage, tmp.path(), size + 5).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::TooLarge);
    assert_that!(err.localize(fr())).contains("pack");
    assert_that!(err.localize(fr())).is_not_equal_to(err.localize(&oku_slides_i18n::Localizer::new(Some("en"))));
}

#[test]
fn read_pack_dir_with_a_manifest_that_is_not_an_object_is_a_bad_pack() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("widgets.json"), "[]").unwrap();
    let (storage, _) = library(tmp.path());
    let err = read_pack_dir(&storage, tmp.path(), MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::BadPack);
}

#[cfg(unix)]
#[test]
fn read_pack_dir_whose_widgets_cannot_be_listed_is_a_file_error() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    write_pack_dir(tmp.path());
    let widgets = tmp.path().join("layouts");
    // Un dossier qu'on ne lit pas : sa liste échoue, elle n'est pas vide.
    std::fs::set_permissions(&widgets, std::fs::Permissions::from_mode(0o000)).unwrap();
    let (storage, _) = library(tmp.path());
    let read = read_pack_dir(&storage, tmp.path(), MAX).map(|_| ()).map_err(|e| e.kind());
    std::fs::set_permissions(&widgets, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_that!(read).is_equal_to(Err(ImportErrorKind::File));
}

#[test]
fn resolve_pack_by_name_ignores_case() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let files = resolve_pack(&storage, &library, "essentiel", MAX).unwrap();
    assert_that!(pack_id(&files)).is_equal_to(ESSENTIEL_PACK);
}

#[test]
fn resolve_pack_by_id_takes_the_current_version() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let files = resolve_pack(&storage, &library, &BASE_PACK.to_string(), MAX).unwrap();
    assert_that!(files.version()).is_equal_to(library.current_version(BASE_PACK).unwrap().version);
}

#[test]
fn resolve_pack_by_folder_reads_it() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("mon-pack");
    write_pack_dir(&dir);
    let (storage, library) = library(tmp.path());
    let files = resolve_pack(&storage, &library, dir.to_str().unwrap(), MAX).unwrap();
    assert_that!(files.widgets().len()).is_equal_to(1);
}

#[test]
fn resolve_pack_unknown_fails_with_a_localized_message() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let err = resolve_pack(&storage, &library, "Nope", MAX).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::UnknownPack);
    assert_that!(err.localize(fr())).contains("Nope");
    assert_that!(err.localize(fr())).is_not_equal_to(err.localize(&oku_slides_i18n::Localizer::new(Some("en"))));
}

#[test]
fn resolve_pack_unknown_id_fails_too() {
    let tmp = tempfile::tempdir().unwrap();
    let (storage, library) = library(tmp.path());
    let id = "00000000-0000-4000-8000-000000000000";
    assert_that!(resolve_pack(&storage, &library, id, MAX)).is_err();
}

#[test]
fn library_pack_by_name_or_id_gives_the_current_version() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, library) = library(tmp.path());
    let by_name = library_pack(&library, "ESSENTIEL").unwrap();
    let by_id = library_pack(&library, &ESSENTIEL_PACK.to_string()).unwrap();
    assert_that!(by_name.version()).is_equal_to(by_id.version());
    assert_that!(pack_id(&by_id)).is_equal_to(ESSENTIEL_PACK);
}

#[test]
fn library_pack_never_reads_a_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("mon-pack");
    write_pack_dir(&dir);
    let (_, library) = library(tmp.path());
    let err = library_pack(&library, dir.to_str().unwrap()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::UnknownPack);
}

#[test]
fn embed_theme_by_name_or_id_embeds_the_same_version() {
    let tmp = tempfile::tempdir().unwrap();
    let themes = ThemeLibrary::new(Some(tmp.path().join("themes")), Storage::local().unwrap());
    let mut bundle = empty_bundle();
    let by_name = embed_theme(&themes, "PRO", &mut bundle).unwrap();
    let by_id = embed_theme(&themes, &PRO_THEME.to_string(), &mut bundle).unwrap();
    assert_that!(by_name).is_equal_to(by_id.clone());
    assert_that!(by_id.1.get(&by_id.0).map(|p| p.id)).is_equal_to(Some(PRO_THEME));
}

#[test]
fn embed_theme_unknown_fails_with_a_localized_message() {
    let tmp = tempfile::tempdir().unwrap();
    let themes = ThemeLibrary::new(Some(tmp.path().join("themes")), Storage::local().unwrap());
    let mut bundle = empty_bundle();
    let err = embed_theme(&themes, "Nope", &mut bundle).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::UnknownTheme);
    assert_that!(err.localize(fr())).contains("Nope");
    assert_that!(err.localize(fr())).is_not_equal_to(err.localize(&oku_slides_i18n::Localizer::new(Some("en"))));
}

#[test]
fn library_pack_by_namespace_gives_the_current_version_of_its_owner() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, library) = library(tmp.path());
    let manifest = crate::test_support::manifest_ns(uuid::Uuid::new_v4(), "acme", &[]);
    let pin = crate::test_support::add_version(&library, &manifest, &[]);
    let found = library_pack(&library, "acme").unwrap();
    assert_that!(found.version()).is_equal_to(pin.version);
    assert_that!(library_pack(&library, "absent").is_err()).is_true();
}

#[test]
fn embed_theme_by_namespace_embeds_its_closure() {
    let tmp = tempfile::tempdir().unwrap();
    let themes = ThemeLibrary::new(Some(tmp.path().join("themes")), Storage::local().unwrap());
    crate::test_support::add_user_theme(
        &themes,
        uuid::Uuid::new_v4(),
        "marque",
        &["pro"],
        serde_json::Map::default(),
        None,
    );
    let mut bundle = empty_bundle();
    let (ns, pins) = embed_theme(&themes, "marque", &mut bundle).unwrap();
    assert_that!(ns.as_str()).is_equal_to("marque");
    assert_that!(pins.keys().map(oku_slides_core::Namespace::as_str).collect::<Vec<_>>())
        .is_equal_to(vec!["marque", "pro"]);
    assert_that!(pins.values().all(|p| bundle.themes.contains_key(&p.version))).is_true();
}

#[test]
fn embed_theme_of_a_child_with_a_missing_parent_is_a_library_error() {
    let tmp = tempfile::tempdir().unwrap();
    let themes = ThemeLibrary::new(Some(tmp.path().join("themes")), Storage::local().unwrap());
    crate::test_support::add_user_theme(
        &themes,
        uuid::Uuid::new_v4(),
        "marque",
        &["absent"],
        serde_json::Map::default(),
        None,
    );
    let err = embed_theme(&themes, "marque", &mut empty_bundle()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::Library);
}
