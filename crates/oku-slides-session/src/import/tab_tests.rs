use speculoos::prelude::*;

use oku_slides_core::{Namespace, PackPin};

use super::super::{ImportMode, ImportRequest, ImportTarget, PackLookup, import};
use crate::test_support::{fresh_dir, imported_fixture, library_of, ns, pin, pptx_fixture, session_on, settings_on};
use crate::{ESSENTIEL_PACK, Fonts, PackManifest, Platform, Request};

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
        serde_json::from_value(serde_json::json!({ "mode": "retrofit", "layouts": ["Essentiel"] })).unwrap();
    assert_that!(retrofit).is_equal_to(ImportTarget::Retrofit { layouts: vec!["Essentiel".into()], theme: None });
    let bare: ImportTarget = serde_json::from_value(serde_json::json!({ "mode": "retrofit" })).unwrap();
    assert_that!(bare).is_equal_to(ImportTarget::Retrofit { layouts: Vec::new(), theme: None });
    let hifi: ImportTarget = serde_json::from_value(serde_json::json!({ "mode": "hifi" })).unwrap();
    assert_that!(hifi).is_equal_to(ImportTarget::Hifi { witness: None });
    assert_that!(serde_json::from_value::<ImportTarget>(serde_json::json!({ "mode": "autre" })).is_err()).is_true();
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

/// L'import retrofit de la fixture `minimal` sur Essentiel, chargé dans une session neuve sur `data`.
fn retrofit_session(data: &std::path::Path) -> crate::Session {
    let target = ImportTarget::Retrofit { layouts: Vec::new(), theme: None };
    imported_fixture(data, "minimal", target, |_| {})
}

/// La version courante du pack cible (Essentiel) dans la bibliothèque de `s`.
fn target_of(s: &crate::Session) -> PackPin {
    let current = library_of(s).current_version(ESSENTIEL_PACK).unwrap();
    PackPin { id: ESSENTIEL_PACK, version: current.version, requires: current.manifest.requires }
}

/// L'espace de noms du pack cible du retrofit.
fn essentiel() -> Namespace {
    ns("essentiel")
}

#[test]
fn retrofit_deck_pins_the_target_library_pack() {
    let data = fresh_dir("oku-slides-import-tab-retrofit-");
    let s = retrofit_session(data.path());
    assert_that!(pin(&s, "essentiel")).is_equal_to(Some(target_of(&s)));
    // Le pack de la bibliothèque : l'atelier reste ouvert.
    assert_that!(s.snapshot().deck_pack).is_false();
}

#[test]
fn retrofit_deck_on_an_older_target_version_is_an_older_pin_in_the_packs_panel() {
    let data = fresh_dir("oku-slides-import-tab-retrofit-notice-");
    let s = retrofit_session(data.path());
    let target = target_of(&s);
    let lib = library_of(&s);
    let mut m = PackManifest::from_files(&lib.current_version(ESSENTIEL_PACK).unwrap().files).unwrap();
    (m.parent, m.description) = (Some(target.version.clone()), "suivante".into());
    let current = lib.current_version(ESSENTIEL_PACK).unwrap().files;
    let next = m.to_files(current.kind(), current.widgets().clone()).unwrap();
    lib.add_version(ESSENTIEL_PACK, &next).unwrap();
    lib.set_current(ESSENTIEL_PACK, &next.version()).unwrap();
    let view = s.deck_packs().unwrap();
    let older = view.layouts.iter().find(|p| p.drift == Some(crate::Drift::Older));
    let found = older.map(|p| (p.namespace.clone(), p.version.clone()));
    assert_that!(found).is_equal_to(Some((essentiel(), target.version.hex().to_owned())));
}

#[test]
fn widget_write_on_a_retrofit_deck_follows_the_deck_mode_rule() {
    let data = fresh_dir("oku-slides-import-tab-retrofit-widget-");
    let mut s = retrofit_session(data.path());
    let target = target_of(&s);
    s.set_widget(None, "quote", QUOTE.into()).unwrap();
    // L'élément va au pack propre du deck ; le pack de layouts cible ne change pas.
    assert_that!((pin(&s, "essentiel"), s.registry.get("deck/quote").is_some())).is_equal_to((Some(target), true));
}

/// Un widget de slide des tests : un titre seul.
const COVER: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("cover", category: "slide", params: (title: p.plain()),
  (args, ctx) => field(ctx, "title", args.title))
"#;

#[test]
fn slide_widget_write_on_a_hifi_deck_stays_in_its_generated_pack() {
    let data = fresh_dir("oku-slides-import-tab-hifi-widget-");
    let mut s = imported_session(data.path());
    let before = pin(&s, "deck-layouts").unwrap();
    s.set_widget(None, "cover", COVER.into()).unwrap();
    assert_that!(pin(&s, "deck-layouts").map(|r| r.id)).is_equal_to(Some(before.id));
}
