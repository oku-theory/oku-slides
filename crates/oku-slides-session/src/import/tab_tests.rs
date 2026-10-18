use speculoos::prelude::*;

use oku_slides_core::{PackRef, WidgetFile};

use super::super::child_pack::child_id;
use super::super::{ImportMode, ImportRequest, ImportTarget, PackLookup, import};
use crate::test_support::{fresh_dir, imported_fixture, library_of, pptx_fixture, session_on, settings_on};
use crate::{DEFAULT_PACK, Fonts, PackManifest, PackScope, Platform, Request};

/// L'import hi-fi de la fixture `minimal`, chargé dans une session neuve sur `data`.
fn imported_session(data: &std::path::Path) -> crate::Session {
    let platform = Platform::with_settings(settings_on(data)).unwrap();
    let request = ImportRequest {
        pptx: pptx_fixture(data, "minimal"),
        target: crate::test_support::hifi(),
        system_fonts: false,
        packs: PackLookup::Library,
    };
    let imported = import(&platform, &Fonts::default(), &request).unwrap();
    let mut s = session_on(data);
    s.load_import(imported, "minimal").unwrap();
    s
}

#[test]
fn load_import_keeps_the_state_and_an_unsaved_deck_without_file() {
    let data = fresh_dir("oku-slides-import-tab-");
    let s = imported_session(data.path());
    assert_that!(s.import_state().map(|st| st.mode())).is_equal_to(Some(ImportMode::Hifi));
    assert_that!(s.deck().slides.len()).is_equal_to(3);
    let info = s.tab_info();
    assert_that!(info.path).is_none();
    assert_that!((info.dirty, info.pristine)).is_equal_to((true, false));
    // Le hi-fi ne reprend pas le titre de la présentation : le deck porte le nom du `.pptx`.
    assert_that!(info.title.as_str()).is_equal_to("minimal");
}

#[test]
fn load_import_undoes_the_import_in_one_step() {
    let data = fresh_dir("oku-slides-import-tab-undo-");
    let mut s = imported_session(data.path());
    s.handle(Request::Undo).unwrap();
    assert_that!(s.deck().slides).is_empty();
}

#[test]
fn opening_another_document_forgets_the_import_state() {
    let data = fresh_dir("oku-slides-import-tab-new-");
    let mut s = imported_session(data.path());
    s.new_deck().unwrap();
    assert_that!(s.import_state().is_none()).is_true();
}

#[test]
fn import_target_reads_its_mode_tagged_json() {
    let retrofit: ImportTarget =
        serde_json::from_value(serde_json::json!({ "mode": "retrofit", "pack": "Essentiel" })).unwrap();
    assert_that!(retrofit).is_equal_to(ImportTarget::Retrofit { pack: "Essentiel".into(), theme: None });
    let hifi: ImportTarget = serde_json::from_value(serde_json::json!({ "mode": "hifi" })).unwrap();
    assert_that!(hifi).is_equal_to(ImportTarget::Hifi { witness: None });
    assert_that!(serde_json::from_value::<ImportTarget>(serde_json::json!({ "mode": "retrofit" })).is_err()).is_true();
}

#[test]
fn opening_another_document_that_fails_keeps_the_import_state() {
    let data = fresh_dir("oku-slides-import-tab-failed-");
    let mut s = imported_session(data.path());
    s.failpoints.install_widgets = true;
    assert_that!(s.new_deck().is_err()).is_true();
    assert_that!(s.import_state().map(|st| st.mode())).is_equal_to(Some(ImportMode::Hifi));
}

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich()), (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

const QUOTE_V2: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich()), (args, ctx) => field(ctx, "body", strong(args.body)))
"#;

/// L'import retrofit de la fixture `minimal` sur Essentiel, chargé dans une session neuve sur `data`.
fn retrofit_session(data: &std::path::Path) -> crate::Session {
    let target = ImportTarget::Retrofit { pack: "Essentiel".into(), theme: None };
    imported_fixture(data, "minimal", target, |_| {})
}

/// La version courante du pack cible (Essentiel) dans la bibliothèque de `s`.
fn target_of(s: &crate::Session) -> PackRef {
    PackRef { id: DEFAULT_PACK, version: library_of(s).current(DEFAULT_PACK).unwrap() }
}

#[test]
fn retrofit_deck_stays_on_the_target_library_pack_until_a_widget_is_written() {
    let data = fresh_dir("oku-slides-import-tab-retrofit-");
    let s = retrofit_session(data.path());
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(target_of(&s)));
    // Le pack de la bibliothèque : l'atelier reste ouvert.
    assert_that!(s.snapshot().deck_pack).is_false();
}

#[test]
fn retrofit_deck_on_an_older_target_version_gets_the_pack_notice() {
    let data = fresh_dir("oku-slides-import-tab-retrofit-notice-");
    let s = retrofit_session(data.path());
    let target = target_of(&s);
    let lib = library_of(&s);
    let mut m = PackManifest::from_files(&lib.current_version(DEFAULT_PACK).unwrap().files).unwrap();
    (m.parent, m.description) = (Some(target.version.clone()), "suivante".into());
    let next = m.to_files(lib.current_version(DEFAULT_PACK).unwrap().files.widgets().clone()).unwrap();
    lib.add_version(DEFAULT_PACK, &next).unwrap();
    lib.set_current(DEFAULT_PACK, &next.version()).unwrap();
    assert_that!(s.pack_notice_for_deck().map(|n| n.pack)).is_equal_to(Some(target));
}

#[test]
fn first_widget_write_on_a_retrofit_deck_creates_its_deck_only_child_pack_in_one_undo_step() {
    let data = fresh_dir("oku-slides-import-tab-retrofit-child-");
    let mut s = retrofit_session(data.path());
    let target = target_of(&s);
    let (lib_versions, lib_packs) = (library_of(&s).versions(DEFAULT_PACK).len(), library_of(&s).packs());
    let steps = s.doc.sources.undo_steps();
    s.set_widget("quote", QUOTE.into()).unwrap();
    let r = s.deck().templates.clone().unwrap();
    let pptx = std::fs::read(data.path().join("minimal.pptx")).unwrap();
    assert_that!(r.id).is_equal_to(child_id(&pptx, &target.version));
    let files = &s.doc.sources.packs[&r.version];
    let manifest = PackManifest::from_files(files).unwrap();
    let target_files = library_of(&s).version(DEFAULT_PACK, &target.version).unwrap();
    assert_that!(manifest.scope).is_equal_to(PackScope::Deck);
    assert_that!(manifest.parent).is_equal_to(Some(target.version.clone()));
    let name = |locale| {
        let args = [("pack", "Essentiel".into()), ("deck", "minimal".into())];
        crate::Localizer::new(Some(locale)).tr_with("session-import-retrofit-pack", &args)
    };
    assert_that!(name("fr")).is_not_equal_to(name("de"));
    assert_that!(manifest.name).is_equal_to(name("fr"));
    assert_that!(manifest.templates).is_equal_to(PackManifest::from_files(&target_files).unwrap().templates);
    assert_that!(files.widget(&WidgetFile::parse("quote").unwrap())).is_equal_to(Some(QUOTE));
    assert_that!(files.widgets().len()).is_equal_to(target_files.widgets().len() + 1);
    assert_that!(s.snapshot().deck_pack).is_true();
    assert_that!(s.doc.sources.undo_steps()).is_equal_to(steps + 1);
    // La bibliothèque et le pack cible n'ont pas changé.
    let lib = library_of(&s);
    assert_that!(lib.current(DEFAULT_PACK)).is_equal_to(Some(target.version.clone()));
    assert_that!(lib.versions(DEFAULT_PACK).len()).is_equal_to(lib_versions);
    assert_that!(lib.packs()).is_equal_to(&lib_packs);
    assert_that!(lib.versions(r.id).is_empty()).is_true();
    s.undo().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(target));
}

#[test]
fn second_widget_write_on_a_retrofit_deck_stays_in_its_child_pack_and_its_undo_step() {
    let data = fresh_dir("oku-slides-import-tab-retrofit-again-");
    let mut s = retrofit_session(data.path());
    let target = target_of(&s);
    s.set_widget("quote", QUOTE.into()).unwrap();
    let (child, steps) = (s.deck().templates.clone().unwrap(), s.doc.sources.undo_steps());
    s.set_widget("quote", QUOTE_V2.into()).unwrap();
    let r = s.deck().templates.clone().unwrap();
    assert_that!(r.id).is_equal_to(child.id);
    assert_that!(s.doc.sources.packs[&r.version].widget(&WidgetFile::parse("quote").unwrap()))
        .is_equal_to(Some(QUOTE_V2));
    assert_that!(s.doc.sources.undo_steps()).is_equal_to(steps);
    s.undo().unwrap();
    assert_that!(s.deck().templates.clone()).is_equal_to(Some(target));
}

#[test]
fn widget_write_on_a_hifi_deck_stays_in_its_generated_pack() {
    let data = fresh_dir("oku-slides-import-tab-hifi-widget-");
    let mut s = imported_session(data.path());
    let before = s.deck().templates.clone().unwrap();
    s.set_widget("quote", QUOTE.into()).unwrap();
    assert_that!(s.deck().templates.as_ref().map(|r| r.id)).is_equal_to(Some(before.id));
}
