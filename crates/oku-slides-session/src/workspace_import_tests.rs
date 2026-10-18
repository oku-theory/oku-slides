use std::sync::Mutex;

use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::import::ImportMode;
use crate::test_support::{apply_ws, fresh_dir, pptx_fixture, workspace_on, workspace_platform};
use crate::{Localize, Localizer, Request, SessionErrorKind};
use oku_slides_render_worker::ReferenceWitness;

fn import_tab(ws: &Mutex<Workspace>, path: &Path, target: serde_json::Value, tune: bool) -> Result<WorkspaceOutcome> {
    let mut req = json!({ "cmd": "import_tab", "window": "w-1", "path": path, "tune": tune });
    if let (Some(req), Some(target)) = (req.as_object_mut(), target.as_object()) {
        req.extend(target.clone());
    }
    apply_ws(ws, req)
}

/// Le mode de l'import tenu par l'onglet `tab`, s'il en tient un.
fn import_mode(ws: &Mutex<Workspace>, tab: &str) -> Option<ImportMode> {
    Workspace::with_session(ws, tab, |s| s.import_state().map(|st| st.mode())).0.unwrap()
}

#[test]
fn import_tab_hifi_replaces_the_pristine_tab_with_an_unsaved_imported_deck() {
    let data = fresh_dir("oku-slides-workspace-import-");
    let ws = workspace_on(data.path());
    let pptx = pptx_fixture(data.path(), "minimal");
    let outcome = import_tab(&ws, &pptx, json!({ "mode": "hifi" }), false).unwrap();
    assert_that!(outcome.tune_tab()).is_none();
    let w = &Workspace::lock(&ws).view().windows[0];
    let tabs: Vec<&str> = w.tabs.iter().map(|t| t.id.as_str()).collect();
    assert_that!(tabs).is_equal_to(vec!["t-2"]);
    let info = &w.tabs[0].info;
    assert_that!(info.path).is_none();
    assert_that!(info.dirty).is_true();
    // Le hi-fi ne reprend pas le titre de la présentation : le deck porte le nom du `.pptx`.
    assert_that!(info.title.as_str()).is_equal_to("minimal");
    assert_that!(import_mode(&ws, "t-2")).is_equal_to(Some(ImportMode::Hifi));
}

#[test]
fn import_tab_retrofit_beside_an_edited_tab_adds_a_tab_holding_the_import() {
    let data = fresh_dir("oku-slides-workspace-import-retrofit-");
    let ws = workspace_on(data.path());
    let theme = json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": crate::test_support::theme_ref("pro")}]});
    Workspace::request(&ws, "t-1", serde_json::from_value::<Request>(theme).unwrap()).0.unwrap();
    let pptx = pptx_fixture(data.path(), "minimal");
    import_tab(&ws, &pptx, json!({ "mode": "retrofit", "pack": "Essentiel" }), false).unwrap();
    let w = &Workspace::lock(&ws).view().windows[0];
    let tabs: Vec<&str> = w.tabs.iter().map(|t| t.id.as_str()).collect();
    assert_that!(tabs).is_equal_to(vec!["t-1", "t-2"]);
    assert_that!(w.active.as_str()).is_equal_to("t-2");
    assert_that!(import_mode(&ws, "t-2")).is_equal_to(Some(ImportMode::Retrofit));
    assert_that!(import_mode(&ws, "t-1")).is_none();
}

#[test]
fn import_tab_with_tune_names_the_imported_tab_in_its_outcome() {
    let data = fresh_dir("oku-slides-workspace-import-tune-");
    let ws = workspace_on(data.path());
    let pptx = pptx_fixture(data.path(), "minimal");
    let outcome = import_tab(&ws, &pptx, json!({ "mode": "hifi" }), true).unwrap();
    assert_that!(outcome.tune_tab()).is_equal_to(Some("t-2"));
    let view: serde_json::Value = serde_json::from_str(&outcome.into_json()).unwrap();
    assert_that!(view["windows"][0]["active"]).is_equal_to(json!("t-2"));
}

#[test]
fn import_tab_twice_of_the_same_file_opens_two_tabs() {
    let data = fresh_dir("oku-slides-workspace-import-twice-");
    let ws = workspace_on(data.path());
    let pptx = pptx_fixture(data.path(), "minimal");
    import_tab(&ws, &pptx, json!({ "mode": "hifi" }), false).unwrap();
    import_tab(&ws, &pptx, json!({ "mode": "hifi" }), false).unwrap();
    assert_that!(Workspace::lock(&ws).tab_ids()).is_equal_to(vec!["t-2".to_owned(), "t-3".to_owned()]);
}

#[test]
fn import_tab_of_a_file_that_is_not_a_pptx_is_a_localized_import_error_and_keeps_the_tabs() {
    let data = fresh_dir("oku-slides-workspace-import-bad-");
    let ws = workspace_on(data.path());
    let bad = data.path().join("faux.pptx");
    std::fs::write(&bad, "pas un zip").unwrap();
    let err = import_tab(&ws, &bad, json!({ "mode": "hifi" }), true).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Import);
    // Le message est celui de l'import, en français (la langue des tests).
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr).is_not_equal_to(err.localize(&Localizer::new(Some("en"))));
    assert_that!(Workspace::lock(&ws).tab_ids()).is_equal_to(vec!["t-1".to_owned()]);
}

#[test]
fn import_tab_with_a_pack_folder_as_target_is_a_localized_unknown_pack_and_keeps_the_tabs() {
    let data = fresh_dir("oku-slides-workspace-import-folder-");
    let ws = workspace_on(data.path());
    let dir = data.path().join("mon-pack");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("widgets.json"), include_str!("../../../templates/grilles/pack.json")).unwrap();
    let pptx = pptx_fixture(data.path(), "minimal");
    // Un dossier venu de l'interface n'est jamais lu : seule la bibliothèque compte.
    let err = import_tab(&ws, &pptx, json!({ "mode": "retrofit", "pack": dir }), false).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Import);
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr.as_str()).contains(dir.to_string_lossy().as_ref());
    assert_that!(fr).is_not_equal_to(err.localize(&Localizer::new(Some("en"))));
    assert_that!(Workspace::lock(&ws).tab_ids()).is_equal_to(vec!["t-1".to_owned()]);
}

#[test]
fn import_tab_with_retrofit_and_no_pack_is_refused_when_read() {
    let req = json!({ "cmd": "import_tab", "window": "w-1", "path": "/x.pptx", "mode": "retrofit" });
    assert_that!(serde_json::from_value::<WorkspaceRequest>(req).is_err()).is_true();
}

#[test]
fn import_tab_resumed_after_relaunch_keeps_its_deck_but_not_its_import_state() {
    let data = fresh_dir("oku-slides-workspace-import-resume-");
    let dir = data.path().join("resume");
    let platform = workspace_platform(data.path());
    let ws = Mutex::new(Workspace::resume(Fonts::default(), &platform, dir.clone()).unwrap());
    let pptx = pptx_fixture(data.path(), "minimal");
    import_tab(&ws, &pptx, json!({ "mode": "hifi" }), false).unwrap();
    Workspace::settle(&ws);
    drop(ws);
    let again = Mutex::new(Workspace::resume(Fonts::default(), &platform, dir).unwrap());
    let slides = Workspace::with_session(&again, "t-2", |s| s.deck().slides.len()).0.unwrap();
    assert_that!(slides).is_equal_to(3);
    assert_that!(import_mode(&again, "t-2")).is_none();
}

/// Un espace de travail neuf sur `data`, ses réglages changés par `f`.
fn workspace_with(data: &Path, f: impl FnOnce(&mut crate::Settings)) -> Mutex<Workspace> {
    let mut settings = workspace_platform(data).settings.as_ref().clone();
    f(&mut settings);
    let platform = crate::Platform::with_settings(settings).unwrap();
    Mutex::new(Workspace::new(crate::Fonts::default(), &platform, None).unwrap())
}

/// Le témoin du travail de référence de l'import de `tab`, et l'état de sa référence.
fn witness_of(
    ws: &Mutex<Workspace>,
    tab: &str,
) -> (Option<ReferenceWitness>, Option<Result<String, oku_slides_import::report::ReferenceCause>>) {
    Workspace::with_session(ws, tab, |s| {
        let st = s.import_state();
        let job = st.and_then(|st| st.reference_job()).map(|j| j.witness);
        (job, st.and_then(|st| st.reference_status()).map(|r| r.map(str::to_owned)))
    })
    .0
    .unwrap()
}

#[test]
fn import_tab_hifi_takes_the_office2pdf_witness_by_default() {
    let data = fresh_dir("oku-slides-workspace-import-witness-");
    let ws = workspace_on(data.path());
    import_tab(&ws, &pptx_fixture(data.path(), "minimal"), json!({ "mode": "hifi" }), false).unwrap();
    assert_that!(witness_of(&ws, "t-2")).is_equal_to((Some(ReferenceWitness::Office2Pdf), None));
}

#[test]
fn import_tab_hifi_with_an_absent_soffice_witness_says_it_is_missing_without_office2pdf() {
    let data = fresh_dir("oku-slides-workspace-import-soffice-");
    let ws = workspace_with(data.path(), |s| {
        s.import.witness = oku_slides_platform::config::ImportWitness::Soffice;
        s.import.soffice = vec![data.path().join("absent/soffice")];
    });
    import_tab(&ws, &pptx_fixture(data.path(), "minimal"), json!({ "mode": "hifi" }), false).unwrap();
    let missing = oku_slides_import::report::ReferenceCause::Missing;
    assert_that!(witness_of(&ws, "t-2")).is_equal_to((None, Some(Err(missing))));
}

#[test]
fn import_tab_retrofit_has_no_witness_even_with_soffice_set() {
    let data = fresh_dir("oku-slides-workspace-import-retrofit-witness-");
    let ws = workspace_with(data.path(), |s| s.import.witness = oku_slides_platform::config::ImportWitness::Soffice);
    let target = json!({ "mode": "retrofit", "pack": "Essentiel" });
    import_tab(&ws, &pptx_fixture(data.path(), "minimal"), target, false).unwrap();
    assert_that!(witness_of(&ws, "t-2")).is_equal_to((None, None));
}
