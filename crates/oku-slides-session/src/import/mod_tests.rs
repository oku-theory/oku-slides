use oku_slides_core::{Namespace, PackKind, PackManifest, PackPin, PackScope, WidgetFile, kinds};
use speculoos::prelude::*;

use super::*;
use crate::test_support::{fresh_dir, ns, pptx_fixture, settings_on};
use crate::{ESSENTIEL_PACK, PRO_THEME, PackLibrary, Settings, ThemeLibrary};

/// La plateforme de test sur `data`, en français.
fn platform(settings: Settings) -> Platform {
    Platform::with_settings(settings).unwrap()
}

/// Importe la fixture `minimal` vers `target`, avec les réglages de test sur `data`.
fn import_minimal(data: &std::path::Path, target: ImportTarget) -> Result<Imported> {
    let request =
        ImportRequest { pptx: pptx_fixture(data, "minimal"), target, system_fonts: false, packs: PackLookup::Library };
    import(&platform(settings_on(data)), &Fonts::default(), &request)
}

fn retrofit(layouts: &[&str], theme: Option<&str>) -> ImportTarget {
    ImportTarget::Retrofit {
        layouts: layouts.iter().map(|l| (*l).to_owned()).collect(),
        theme: theme.map(str::to_owned),
    }
}

/// Importe la fixture `name` en retrofit sur les packs de layouts `layouts`, sans thème choisi.
fn import_retrofit(data: &std::path::Path, layouts: &[&str]) -> Result<Imported> {
    import_minimal(data, retrofit(layouts, None))
}

#[test]
fn retrofit_without_layouts_targets_essentiel() {
    let data = fresh_dir("oku-slides-import-retrofit-default-");
    let imported = import_retrofit(data.path(), &[]).unwrap();
    let layouts: Vec<&str> = imported.sources.deck().layouts.keys().map(Namespace::as_str).collect();
    assert_that!(layouts).is_equal_to(vec!["essentiel"]);
}

#[test]
fn retrofit_on_a_widget_pack_is_refused() {
    let data = fresh_dir("oku-slides-import-retrofit-widgets-");
    let err = import_retrofit(data.path(), &["base"]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::NotLayoutPack);
}

#[test]
fn import_retrofit_is_one_history_step_on_the_target_pack_and_theme() {
    let data = fresh_dir("oku-slides-import-retrofit-");
    let mut imported = import_minimal(data.path(), retrofit(&["Essentiel"], Some("Pro"))).unwrap();
    let deck = imported.sources.deck();
    assert_that!(deck.slides.len()).is_equal_to(3);
    assert_that!(deck.theme.as_ref().and_then(|ns| deck.themes.get(ns)).map(|p| p.id)).is_equal_to(Some(PRO_THEME));
    assert_that!(imported.issues).is_empty();
    assert_that!(imported.state.mode()).is_equal_to(ImportMode::Retrofit);
    assert_that!(imported.sources.undo_steps()).is_equal_to(1);
    assert_that!(imported.sources.undo().unwrap()).is_true();
    assert_that!(imported.sources.deck().slides).is_empty();
}

#[test]
fn import_retrofit_embeds_the_target_pack_and_theme_versions() {
    let data = fresh_dir("oku-slides-import-retrofit-embed-");
    let imported = import_minimal(data.path(), retrofit(&["Essentiel"], Some("Pro"))).unwrap();
    let (deck, sources) = (imported.sources.deck(), &imported.sources);
    let pack = deck.layouts.get(&ns("essentiel")).unwrap();
    let theme = &deck.themes[deck.theme.as_ref().unwrap()];
    assert_that!(sources.packs.contains_key(&pack.version)).is_true();
    assert_that!(sources.themes.contains_key(&theme.version)).is_true();
}

#[test]
fn import_retrofit_pins_the_target_library_layout_pack_unchanged() {
    let data = fresh_dir("oku-slides-import-retrofit-target-");
    let library = PackLibrary::user(&platform(settings_on(data.path())));
    let target = library.current_version(ESSENTIEL_PACK).unwrap();
    let packs = library.packs(PackKind::Layouts);
    let imported = import_minimal(data.path(), retrofit(&["Essentiel"], None)).unwrap();
    let r = imported.sources.deck().layouts.get(&ns("essentiel")).cloned().unwrap();
    assert_that!(r.clone()).is_equal_to(PackPin {
        id: ESSENTIEL_PACK,
        version: target.version.clone(),
        requires: target.manifest.requires.clone(),
    });
    assert_that!(imported.sources.packs[&r.version]).is_equal_to(&target.files);
    assert_that!(library.current(ESSENTIEL_PACK)).is_equal_to(Some(target.version));
    assert_that!(library.packs(PackKind::Layouts)).is_equal_to(&packs);
}

/// Écrit dans `dir` les fichiers du pack `files`, comme une version de la bibliothèque.
fn write_pack_dir(dir: &std::path::Path, files: &oku_slides_core::PackFiles) {
    let kind = files.kind();
    std::fs::create_dir_all(dir.join(kind.files_dir())).unwrap();
    std::fs::write(dir.join(kind.manifest_file()), files.pack_json().unwrap()).unwrap();
    for (name, source) in files.widgets() {
        std::fs::write(dir.join(kind.files_dir()).join(name.file_name()), source).unwrap();
    }
}

#[test]
fn import_retrofit_on_a_pack_folder_through_the_library_lookup_is_an_unknown_pack() {
    let data = fresh_dir("oku-slides-import-retrofit-folder-");
    let dir = data.path().join("mon-pack");
    write_pack_dir(&dir, &crate::test_support::builtin(ESSENTIEL_PACK).files);
    let err = import_minimal(data.path(), retrofit(&[dir.to_str().unwrap()], None)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::UnknownPack);
    let request = ImportRequest {
        pptx: pptx_fixture(data.path(), "minimal"),
        target: retrofit(&[dir.to_str().unwrap()], None),
        system_fonts: false,
        packs: PackLookup::LibraryOrDir,
    };
    let imported = import(&platform(settings_on(data.path())), &Fonts::default(), &request).unwrap();
    let essentiel = imported.sources.deck().layouts.get(&ns("essentiel")).map(|r| r.id);
    assert_that!(essentiel).is_equal_to(Some(ESSENTIEL_PACK));
}

#[test]
fn import_retrofit_is_labelled_in_the_live_language() {
    let data = fresh_dir("oku-slides-import-retrofit-live-");
    let platform = platform(settings_on(data.path()));
    platform.live_localizer.set(std::sync::Arc::new(crate::Localizer::new(Some("de"))));
    let request = ImportRequest {
        pptx: pptx_fixture(data.path(), "minimal"),
        target: retrofit(&["Essentiel"], None),
        system_fonts: false,
        packs: PackLookup::Library,
    };
    let imported = import(&platform, &Fonts::default(), &request).unwrap();
    let de = crate::Localizer::new(Some("de")).tr("session-import-label");
    assert_that!(imported.sources.replica().undo_label()).is_equal_to(Some(de));
}

#[test]
fn import_hifi_is_one_history_step_on_its_generated_pack() {
    let data = fresh_dir("oku-slides-import-hifi-");
    let mut imported = import_minimal(data.path(), crate::test_support::hifi()).unwrap();
    let pack_id = hifi::identity(&std::fs::read(data.path().join("minimal.pptx")).unwrap());
    let deck = imported.sources.deck();
    let pack = deck.layouts.get(&Namespace::deck_layouts()).cloned().unwrap();
    assert_that!(pack.id).is_equal_to(pack_id);
    let manifest = PackManifest::from_files(&imported.sources.packs[&pack.version]).unwrap();
    assert_that!(manifest.name).is_equal_to("minimal".to_owned());
    assert_that!(imported.issues).is_empty();
    assert_that!(imported.sources.undo_steps()).is_equal_to(1);
    assert_that!(imported.sources.undo().unwrap()).is_true();
    assert_that!(imported.sources.deck().slides).is_empty();
}

#[test]
fn import_hifi_generates_a_deck_only_pack() {
    let data = fresh_dir("oku-slides-import-hifi-scope-");
    let imported = import_minimal(data.path(), crate::test_support::hifi()).unwrap();
    let pack = imported.sources.deck().layouts.get(&Namespace::deck_layouts()).cloned().unwrap();
    let pack = PackManifest::from_files(&imported.sources.packs[&pack.version]).unwrap();
    assert_that!(pack.scope).is_equal_to(PackScope::Deck);
}

#[test]
fn hifi_imports_pin_neutre_with_deck_tokens() {
    let data = fresh_dir("oku-slides-import-hifi-neutre-");
    let imported = import_minimal(data.path(), crate::test_support::hifi()).unwrap();
    let deck = imported.sources.deck();
    assert_that!(deck.theme.as_ref().map(|t| t.as_str())).is_equal_to(Some(crate::NEUTRE_NAMESPACE));
    let pin = &deck.themes[&ns(crate::NEUTRE_NAMESPACE)];
    assert_that!((deck.themes.len(), pin.id)).is_equal_to((1, crate::NEUTRE_THEME));
    assert_that!(imported.sources.themes.contains_key(&pin.version)).is_true();
    assert_that!(deck.tokens.as_object().is_some_and(|t| !t.is_empty())).is_true();
}

#[test]
fn import_retrofit_keeps_the_library_scope_of_its_target_pack() {
    let data = fresh_dir("oku-slides-import-retrofit-scope-");
    let imported = import_minimal(data.path(), retrofit(&["Essentiel"], Some("Pro"))).unwrap();
    let pack = imported.sources.deck().layouts.get(&ns("essentiel")).cloned().unwrap();
    let pack = PackManifest::from_files(&imported.sources.packs[&pack.version]).unwrap();
    assert_that!(pack.scope).is_equal_to(PackScope::Library);
}

#[test]
fn import_hifi_leaves_the_pack_and_theme_libraries_untouched() {
    let data = fresh_dir("oku-slides-import-hifi-library-");
    let imported = import_minimal(data.path(), crate::test_support::hifi()).unwrap();
    let library = PackLibrary::user(&platform(settings_on(data.path())));
    let id = imported.sources.deck().layouts.get(&Namespace::deck_layouts()).unwrap().id;
    assert_that!(library.current_version(id)).is_none();
    let themes = ThemeLibrary::user(&platform(settings_on(data.path())));
    assert_that!(themes.entries().len()).is_equal_to(3);
}

#[test]
fn import_hifi_with_a_negative_threshold_is_refused_before_reading() {
    let data = fresh_dir("oku-slides-import-threshold-");
    let mut settings = settings_on(data.path());
    settings.import.score_threshold = -1.0;
    let request = ImportRequest {
        pptx: data.path().join("absent.pptx"),
        target: crate::test_support::hifi(),
        system_fonts: false,
        packs: PackLookup::Library,
    };
    let err = import(&platform(settings), &Fonts::default(), &request).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::Threshold);
}

#[test]
fn import_of_a_missing_file_is_a_file_error() {
    let data = fresh_dir("oku-slides-import-missing-");
    let request = ImportRequest {
        pptx: data.path().join("absent.pptx"),
        target: retrofit(&["Essentiel"], None),
        system_fonts: false,
        packs: PackLookup::Library,
    };
    let err = import(&platform(settings_on(data.path())), &Fonts::default(), &request).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::File);
}

#[test]
fn import_of_a_file_over_max_input_mib_is_too_large() {
    let data = fresh_dir("oku-slides-import-too-large-");
    let pptx = data.path().join("gros.pptx");
    std::fs::write(&pptx, vec![0u8; (1 << 20) + 1]).unwrap();
    let mut settings = settings_on(data.path());
    settings.import.max_input_mib = std::num::NonZeroU64::MIN;
    let request =
        ImportRequest { pptx, target: retrofit(&["Essentiel"], None), system_fonts: false, packs: PackLookup::Library };
    let err = import(&platform(settings), &Fonts::default(), &request).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn import_of_a_file_of_exactly_max_input_mib_is_read() {
    let data = fresh_dir("oku-slides-import-at-limit-");
    let pptx = data.path().join("juste.pptx");
    std::fs::write(&pptx, vec![0u8; 1 << 20]).unwrap();
    let mut settings = settings_on(data.path());
    settings.import.max_input_mib = std::num::NonZeroU64::MIN;
    let request =
        ImportRequest { pptx, target: retrofit(&["Essentiel"], None), system_fonts: false, packs: PackLookup::Library };
    // Lu jusqu'au lecteur, qui le refuse : ce n'est pas un `.pptx`.
    let err = import(&platform(settings), &Fonts::default(), &request).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::Presentation);
}

#[test]
fn import_on_an_unknown_pack_is_refused() {
    let data = fresh_dir("oku-slides-import-unknown-pack-");
    let err = import_minimal(data.path(), retrofit(&["Nope"], None)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::UnknownPack);
}

#[test]
fn import_mode_serializes_in_snake_case() {
    assert_that!(serde_json::to_string(&ImportMode::Hifi).unwrap()).is_equal_to("\"hifi\"".to_owned());
    assert_that!(crate::test_support::hifi().mode()).is_equal_to(ImportMode::Hifi);
}

#[test]
fn hifi_import_pins_generated_pack_under_deck_layouts() {
    let data = fresh_dir("oku-slides-import-hifi-deck-pack-");
    let imported = import_minimal(data.path(), crate::test_support::hifi()).unwrap();
    let deck = imported.sources.deck();
    let layouts: Vec<&str> = deck.layouts.keys().map(Namespace::as_str).collect();
    assert_that!(layouts).is_equal_to(vec!["deck-layouts", "essentiel"]);
    let generated = |k: &str| k.starts_with("deck-layouts/") || k == kinds::essentiel::BLANK;
    let kinds = deck.slides.iter().map(|s| s.kind.as_str());
    assert_that!(kinds.clone().count() > 0 && kinds.into_iter().all(generated)).is_true();
}

#[test]
fn hifi_import_pins_the_builtin_base_widgets() {
    let data = fresh_dir("oku-slides-import-hifi-base-");
    let imported = import_minimal(data.path(), crate::test_support::hifi()).unwrap();
    let pins: Vec<String> = imported.sources.deck().widgets.keys().map(ToString::to_string).collect();
    let shape = imported.widgets.registry.get(kinds::base::SHAPE).is_some();
    assert_that!((pins, shape)).is_equal_to((vec!["base".into()], true));
}

/// Un pack de layouts `acme` de la bibliothèque, sans besoins : le layout `content` d'`essentiel` et son premier profil.
fn acme_without_base(data: &std::path::Path) {
    let essentiel = crate::test_support::builtin(ESSENTIEL_PACK);
    let content = WidgetFile::parse("content").unwrap();
    let profile = essentiel.manifest.profiles.iter().find(|p| p.layout == content).cloned().unwrap();
    let source = essentiel.files.widget(&content).unwrap();
    let common = essentiel.files.widget(&WidgetFile::common()).unwrap();
    let acme =
        PackManifest { profiles: vec![profile], ..crate::test_support::manifest_ns(uuid::Uuid::new_v4(), "acme", &[]) };
    let library = PackLibrary::user(&platform(settings_on(data)));
    crate::test_support::add_version_of(
        &library,
        PackKind::Layouts,
        &acme,
        &[("content", source), ("_commun", common)],
    );
}

#[test]
fn retrofit_onto_a_library_pack_without_base_pins_base() {
    let data = fresh_dir("oku-slides-import-retrofit-acme-");
    acme_without_base(data.path());
    let imported = import_minimal(data.path(), retrofit(&["acme"], None)).unwrap();
    let deck = imported.sources.deck();
    assert_that!(deck.widgets.contains_key(&kinds::base::namespace())).is_true();
    assert_that!(oku_slides_core::validate(deck, &imported.widgets.registry, None).is_empty()).is_true();
}
