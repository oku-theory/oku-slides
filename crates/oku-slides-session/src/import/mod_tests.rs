use oku_slides_core::{PackManifest, PackRef, PackScope, ThemeManifest};
use speculoos::prelude::*;

use super::*;
use crate::test_support::{fresh_dir, pptx_fixture, settings_on};
use crate::{DEFAULT_PACK, PRO_THEME, PackLibrary, Settings, ThemeLibrary};

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

fn retrofit(pack: &str, theme: Option<&str>) -> ImportTarget {
    ImportTarget::Retrofit { pack: pack.to_owned(), theme: theme.map(str::to_owned) }
}

#[test]
fn import_retrofit_is_one_history_step_on_the_target_pack_and_theme() {
    let data = fresh_dir("oku-slides-import-retrofit-");
    let mut imported = import_minimal(data.path(), retrofit("Essentiel", Some("Pro"))).unwrap();
    let deck = imported.sources.deck();
    assert_that!(deck.slides.len()).is_equal_to(3);
    assert_that!(deck.theme.as_ref().map(|r| r.id)).is_equal_to(Some(PRO_THEME));
    assert_that!(imported.issues).is_empty();
    assert_that!(imported.state.mode()).is_equal_to(ImportMode::Retrofit);
    assert_that!(imported.sources.undo_steps()).is_equal_to(1);
    assert_that!(imported.sources.undo().unwrap()).is_true();
    assert_that!(imported.sources.deck().slides).is_empty();
}

#[test]
fn import_retrofit_embeds_the_target_pack_and_theme_versions() {
    let data = fresh_dir("oku-slides-import-retrofit-embed-");
    let imported = import_minimal(data.path(), retrofit("Essentiel", Some("Pro"))).unwrap();
    let (deck, sources) = (imported.sources.deck(), &imported.sources);
    let pack = deck.templates.as_ref().unwrap();
    let theme = deck.theme.as_ref().unwrap();
    assert_that!(sources.packs.contains_key(&pack.version)).is_true();
    assert_that!(sources.themes.contains_key(&theme.version)).is_true();
}

#[test]
fn import_retrofit_keeps_the_target_library_pack_and_plans_its_child() {
    let data = fresh_dir("oku-slides-import-retrofit-child-");
    let library = PackLibrary::user(&platform(settings_on(data.path())));
    let target = library.current_version(DEFAULT_PACK).unwrap();
    let packs = library.packs();
    let imported = import_minimal(data.path(), retrofit("Essentiel", None)).unwrap();
    let r = imported.sources.deck().templates.clone().unwrap();
    assert_that!(r.clone()).is_equal_to(PackRef { id: DEFAULT_PACK, version: target.version.clone() });
    assert_that!(imported.sources.packs[&r.version]).is_equal_to(&target.files);
    // Le pack enfant n'existe pas encore : l'état sait seulement quelle version il prendra.
    assert_that!(imported.state.child().map(|c| c.target().clone())).is_equal_to(Some(r));
    assert_that!(library.current(DEFAULT_PACK)).is_equal_to(Some(target.version));
    assert_that!(library.packs()).is_equal_to(&packs);
}

#[test]
fn import_retrofit_on_a_pack_folder_through_the_library_lookup_is_an_unknown_pack() {
    let data = fresh_dir("oku-slides-import-retrofit-folder-");
    let dir = data.path().join("mon-pack");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("pack.json"), include_str!("../../../../templates/grilles/pack.json")).unwrap();
    let err = import_minimal(data.path(), retrofit(dir.to_str().unwrap(), None)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::UnknownPack);
    let request = ImportRequest {
        pptx: pptx_fixture(data.path(), "minimal"),
        target: retrofit(dir.to_str().unwrap(), None),
        system_fonts: false,
        packs: PackLookup::LibraryOrDir,
    };
    let imported = import(&platform(settings_on(data.path())), &Fonts::default(), &request).unwrap();
    assert_that!(imported.sources.deck().templates.as_ref().map(|r| r.id)).is_equal_to(Some(crate::GRID_PACK));
}

#[test]
fn import_retrofit_is_labelled_in_the_live_language() {
    let data = fresh_dir("oku-slides-import-retrofit-live-");
    let platform = platform(settings_on(data.path()));
    platform.live_localizer.set(std::sync::Arc::new(crate::Localizer::new(Some("de"))));
    let request = ImportRequest {
        pptx: pptx_fixture(data.path(), "minimal"),
        target: retrofit("Essentiel", None),
        system_fonts: false,
        packs: PackLookup::Library,
    };
    let imported = import(&platform, &Fonts::default(), &request).unwrap();
    let de = crate::Localizer::new(Some("de")).tr("session-import-label");
    assert_that!(imported.sources.replica().undo_label()).is_equal_to(Some(de));
}

#[test]
fn retrofit_child_id_depends_on_the_pptx_and_the_target_version() {
    let version = |name: &str| {
        let m = serde_json::json!({ "name": name });
        let serde_json::Value::Object(m) = m else { unreachable!() };
        oku_slides_core::PackFiles::new(m, Default::default()).version()
    };
    let (a, b) = (version("a"), version("b"));
    assert_that!(child_pack::child_id(b"pptx", &a)).is_equal_to(child_pack::child_id(b"pptx", &a));
    assert_that!(child_pack::child_id(b"pptx", &a)).is_not_equal_to(child_pack::child_id(b"pptx", &b));
    assert_that!(child_pack::child_id(b"pptx", &a)).is_not_equal_to(child_pack::child_id(b"other", &a));
}

#[test]
fn import_hifi_is_one_history_step_on_its_generated_pack_and_theme() {
    let data = fresh_dir("oku-slides-import-hifi-");
    let mut imported = import_minimal(data.path(), crate::test_support::hifi()).unwrap();
    let (pack_id, theme_id) = hifi::identity(&std::fs::read(data.path().join("minimal.pptx")).unwrap());
    let deck = imported.sources.deck();
    let pack = deck.templates.clone().unwrap();
    let theme = deck.theme.clone().unwrap();
    assert_that!((pack.id, theme.id)).is_equal_to((pack_id, theme_id));
    let manifest = PackManifest::from_files(&imported.sources.packs[&pack.version]).unwrap();
    assert_that!(manifest.name).is_equal_to("minimal".to_owned());
    assert_that!(imported.sources.themes.contains_key(&theme.version)).is_true();
    assert_that!(imported.issues).is_empty();
    assert_that!(imported.sources.undo_steps()).is_equal_to(1);
    assert_that!(imported.sources.undo().unwrap()).is_true();
    assert_that!(imported.sources.deck().slides).is_empty();
}

#[test]
fn import_hifi_generates_a_deck_only_pack_and_theme() {
    let data = fresh_dir("oku-slides-import-hifi-scope-");
    let imported = import_minimal(data.path(), crate::test_support::hifi()).unwrap();
    let deck = imported.sources.deck();
    let (pack, theme) = (deck.templates.clone().unwrap(), deck.theme.clone().unwrap());
    let pack = PackManifest::from_files(&imported.sources.packs[&pack.version]).unwrap();
    let theme = ThemeManifest::from_files(&imported.sources.themes[&theme.version]).unwrap();
    assert_that!((pack.scope, theme.scope)).is_equal_to((PackScope::Deck, PackScope::Deck));
}

#[test]
fn import_retrofit_keeps_the_library_scope_of_its_target_pack_and_theme() {
    let data = fresh_dir("oku-slides-import-retrofit-scope-");
    let imported = import_minimal(data.path(), retrofit("Essentiel", Some("Pro"))).unwrap();
    let deck = imported.sources.deck();
    let (pack, theme) = (deck.templates.clone().unwrap(), deck.theme.clone().unwrap());
    let pack = PackManifest::from_files(&imported.sources.packs[&pack.version]).unwrap();
    let theme = ThemeManifest::from_files(&imported.sources.themes[&theme.version]).unwrap();
    assert_that!((pack.scope, theme.scope)).is_equal_to((PackScope::Library, PackScope::Library));
}

#[test]
fn import_hifi_leaves_the_pack_and_theme_libraries_untouched() {
    let data = fresh_dir("oku-slides-import-hifi-library-");
    let imported = import_minimal(data.path(), crate::test_support::hifi()).unwrap();
    let library = PackLibrary::user(&platform(settings_on(data.path())));
    let id = imported.sources.deck().templates.as_ref().unwrap().id;
    assert_that!(library.current_version(id)).is_none();
    let themes = ThemeLibrary::user(&platform(settings_on(data.path())));
    let theme = imported.sources.deck().theme.as_ref().unwrap().id;
    assert_that!(themes.current_version(theme)).is_none();
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
        target: retrofit("Essentiel", None),
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
        ImportRequest { pptx, target: retrofit("Essentiel", None), system_fonts: false, packs: PackLookup::Library };
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
        ImportRequest { pptx, target: retrofit("Essentiel", None), system_fonts: false, packs: PackLookup::Library };
    // Lu jusqu'au lecteur, qui le refuse : ce n'est pas un `.pptx`.
    let err = import(&platform(settings), &Fonts::default(), &request).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::Presentation);
}

#[test]
fn import_on_an_unknown_pack_is_refused() {
    let data = fresh_dir("oku-slides-import-unknown-pack-");
    let err = import_minimal(data.path(), retrofit("Nope", None)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::UnknownPack);
}

#[test]
fn import_mode_serializes_in_snake_case() {
    assert_that!(serde_json::to_string(&ImportMode::Hifi).unwrap()).is_equal_to("\"hifi\"".to_owned());
    assert_that!(crate::test_support::hifi().mode()).is_equal_to(ImportMode::Hifi);
}
