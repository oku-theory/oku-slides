use oku_slides_core::{Deck, Frame, Op, Paragraph};
use oku_slides_session::{Fonts, Platform, Request, Session, Settings, TargetKind, paragraphs_to_text};
use serde_json::json;
use speculoos::prelude::*;

fn data_dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("oku-slides-session-{}-{}", std::process::id(), unique()))
}

/// Une session sur le dossier de données `data`.
fn session_in(data: &std::path::Path) -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    settings.paths.data = Some(data.to_owned());
    // Les messages attendus sont en français, quelle que soit la langue du système.
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"));
    Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session impossible : {}", e.one_line()))
}

/// Une session isolée : son propre dossier de données, vide.
fn session() -> Session {
    session_in(&data_dir())
}

/// Une session qui reprend l'état laissé dans `data`.
fn resumed(data: &std::path::Path, file: Option<&std::path::Path>) -> Session {
    let mut s = session_in(data);
    s.resume(data.join("resume"), file);
    s
}

fn set_theme(theme: &str) -> Request {
    req(json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": theme}]}))
}

fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

#[test]
fn open_edit_undo_save_reopen_round_trips_deck() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let snap = s.snapshot();
    assert_that!(snap.slides.len()).is_equal_to(8);
    asserting("un JSON n'est pas un fichier Oku Slides : pas de chemin").that(&snap.path).is_none();

    let view = s.slide("s-libre").unwrap();
    let texte = view.targets.iter().find(|t| t.id == "e-texte").unwrap();
    assert_that!(texte.free).is_true();
    assert_that!(texte.text.as_deref()).is_equal_to(Some("Un bloc posé à 40 pt du coin"));
    assert_that!(view.targets.iter().any(|t| t.id == "@title" && t.kind == TargetKind::Field("title"))).is_true();
    assert_that!(texte.kind).is_equal_to(TargetKind::Element("text"));
    let (before_hash, texte_x) = (view.hash, texte.x);

    s.apply(
        "Déplacer",
        vec![Op::SetFrame {
            slide: "s-libre".into(),
            element: "e-texte".into(),
            frame: Some(Frame { x: 100.0, y: 60.0, w: 720.0, h: 200.0 }),
        }],
    )
    .unwrap();
    let snap = s.snapshot();
    assert_that!(snap.dirty).is_true();
    assert_that!(snap.can_undo).is_true();
    assert_that!(snap.undo_label).is_equal_to(Some("Déplacer"));
    let moved = s.slide("s-libre").unwrap();
    assert_that!(moved.hash).is_not_equal_to(before_hash);
    let t2 = moved.targets.iter().find(|t| t.id == "e-texte").unwrap();
    assert_that!(t2.x - texte_x).is_close_to(60.0, 0.01);

    s.undo().unwrap();
    assert_that!(s.slide("s-libre").unwrap().hash).is_equal_to(before_hash);
    s.redo().unwrap();

    let dir = std::env::temp_dir().join(format!("oku-slides-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    s.save(Some(&dir.join("essai.pdf"))).unwrap();
    let snap = s.snapshot();
    let path = snap.path.unwrap().to_owned();
    assert_that!(path.to_string_lossy().into_owned()).ends_with("essai.oku-slides.pdf");
    assert_that!(snap.dirty).is_false();

    let deck: Deck = s.deck().clone();
    let mut s2 = session();
    s2.open(&path).unwrap();
    assert_that!(s2.deck()).is_equal_to(&deck);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn apply_invalid_transaction_leaves_deck_unchanged() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let before = s.deck().clone();
    let err = s
        .apply(
            "Mauvais",
            vec![
                Op::SetNotes { slide: "s-fin".into(), notes: "x".into() },
                Op::RemoveElement { slide: "s-fin".into(), element: "absent".into() },
            ],
        )
        .unwrap_err();
    assert_that!(err.transaction().map(oku_slides_core::TransactionError::index)).is_equal_to(Some(1));
    assert_that!(s.deck()).is_equal_to(&before);
}

#[test]
fn handle_json_requests_returns_expected_json() {
    let mut s = session();
    let req: Request = serde_json::from_value(json!({"cmd": "snapshot"})).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s.handle(req).unwrap()).unwrap();
    assert_that!(v["slides"].as_array().unwrap().len()).is_equal_to(2);
    let req: Request = serde_json::from_value(json!({
        "cmd": "apply", "label": "Titre",
        "ops": [{"op": "set_slide_field", "slide": "s-1", "field": "title", "value": "Nouveau"}]
    }))
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&s.handle(req).unwrap()).unwrap();
    assert_that!(v["slides"][1]["title"]).is_equal_to(json!("Nouveau"));
    asserting("empreinte en hex").that(&v["slides"][1]["hash"].as_str().map(str::len)).is_equal_to(Some(32));
    let svg = s.handle(serde_json::from_value(json!({"cmd": "svg", "id": "s-1"})).unwrap()).unwrap();
    let svg: String = serde_json::from_str(&svg).unwrap();
    assert_that!(svg.as_str()).starts_with("<svg");
    // Une cible garde sa forme JSON : { "target": "field", "kind": "title", … }.
    let view: serde_json::Value =
        serde_json::from_str(&s.handle(serde_json::from_value(json!({"cmd": "slide", "id": "s-1"})).unwrap()).unwrap())
            .unwrap();
    let title = view["targets"].as_array().unwrap().iter().find(|t| t["id"] == "@title").unwrap();
    assert_that!(title["target"]).is_equal_to(json!("field"));
    assert_that!(title["kind"]).is_equal_to(json!("title"));
}

#[test]
fn paragraphs_to_text_bold_run_and_bullet_gives_plain_lines() {
    let paras = vec![
        Paragraph {
            runs: vec![oku_slides_core::Run { t: "Gras".into(), bold: true, ..Default::default() }],
            list: None,
        },
        Paragraph { runs: vec![oku_slides_core::Run::plain("Point")], list: Some(oku_slides_core::ListKind::Bullet) },
    ];
    assert_that!(paragraphs_to_text(&paras)).is_equal_to("Gras\n- Point".to_owned());
}

#[test]
fn snapshot_appearance_follows_theme_and_deck_overrides() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let papier = s.snapshot().appearance;
    assert_that!(papier.scheme.as_str()).is_equal_to("light");
    assert_that!(papier.slide["accent"].as_str()).is_equal_to("#3f6b5e");
    assert_that!(papier.app["desk"].as_str()).is_equal_to("#ece8e0");
    asserting("les références par nom sont résolues").that(&papier.app["ink"].as_str()).is_equal_to("#2e2b26");
    assert_that!(papier.font_display.first().map(String::as_str)).is_equal_to(Some("Newsreader"));

    s.apply("Thème", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    let pro = s.snapshot().appearance;
    assert_that!(pro.scheme.as_str()).is_equal_to("light");
    assert_that!(pro.app["desk"].as_str()).is_equal_to("#e1e5ea");
    assert_that!(pro.app["ink"].as_str()).is_equal_to("#00154e");
    assert_that!(pro.app["accent-text"].as_str()).is_equal_to("#0072b0");
    assert_that!(pro.font_body).is_equal_to(vec!["Inter".to_owned(), "Noto Sans Arabic".to_owned()]);
    assert_that!(pro.metrics["radius-lg"]).is_close_to(16.0, f64::EPSILON);
    assert_that!(pro.metrics["elevation"]).is_close_to(1.5, f64::EPSILON);

    // Sans thème, l'interface dérive ses couleurs : aucune couleur d'app imposée.
    s.apply("Sans thème", vec![Op::SetTheme { theme: None }]).unwrap();
    let plain = s.snapshot().appearance;
    assert_that!(plain.app.is_empty()).is_true();
    asserting("les formes par défaut restent définies")
        .that(&plain.metrics["radius-md"])
        .is_close_to(12.0, f64::EPSILON);

    // Une surcharge du deck l'emporte sur le thème, pour les slides comme pour l'app.
    s.apply(
        "Accent",
        vec![
            Op::SetTheme { theme: Some("pro".into()) },
            Op::SetToken { path: vec!["color".into(), "accent".into()], value: json!("#123456") },
            Op::SetToken { path: vec!["app".into(), "desk".into()], value: json!("#222222") },
        ],
    )
    .unwrap();
    let snap = s.snapshot();
    assert_that!(snap.appearance.slide["accent"].as_str()).is_equal_to("#123456");
    assert_that!(snap.appearance.app["accent"].as_str()).is_equal_to("#123456");
    assert_that!(snap.appearance.app["desk"].as_str()).is_equal_to("#222222");
}

#[test]
fn templates_view_renders_previews_in_deck_theme() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let papier = s.templates_view(None).unwrap().templates;
    assert_that!(papier.len()).is_greater_than_or_equal_to(8);
    let again = s.templates_view(None).unwrap().templates;
    asserting("rien n'a changé : les aperçus viennent du cache")
        .that(&std::sync::Arc::ptr_eq(&papier, &again))
        .is_true();
    assert_that!(papier.iter().all(|t| t.svg.starts_with("<svg"))).is_true();
    assert_that!(papier.iter().any(|t| t.template.id == "cartes")).is_true();

    s.apply("Thème", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    let pro = s.templates_view(None).unwrap().templates;
    asserting("l'aperçu suit le thème").that(&papier[0].svg).is_not_equal_to(&pro[0].svg);

    // Chaque modèle s'insère dans le deck.
    for (i, t) in pro.iter().enumerate() {
        let mut slide = t.template.slide.clone();
        slide.id = format!("nouvelle-{i}");
        assert_that!(s.apply("Nouvelle slide", vec![Op::AddSlide { index: None, slide }])).is_ok();
    }
}

#[test]
fn edit_builtin_templates_then_save_creates_copy_and_close_restores_deck() {
    use serde_json::json;
    let dir = std::env::temp_dir().join(format!("oku-slides-session-tpl-{}", std::process::id()));
    let mut s = session();
    s.set_templates_dir(&dir);
    let before = s.deck().clone();

    s.edit_templates("grilles").unwrap();
    let snap = s.snapshot();
    assert_that!(snap.editing.unwrap().set.as_str()).is_equal_to("grilles");
    assert_that!(snap.editing.unwrap().builtin).is_true();
    let first = snap.slides[0].id.to_owned();
    s.apply(
        "Renommer",
        serde_json::from_value(json!([
            { "op": "set_slide_field", "slide": first, "field": "name", "value": "Ma grille" }
        ]))
        .unwrap(),
    )
    .unwrap();
    // Intégré : l'enregistrement crée une copie.
    s.save_templates(false).unwrap();
    let mode = s.snapshot().editing.unwrap();
    assert_that!(mode.builtin).is_false();
    assert_that!(mode.set.as_str()).is_equal_to("grilles-copie");
    assert_that!(dir.join("grilles-copie.json").exists()).is_true();

    s.close_templates().unwrap();
    assert_that!(s.snapshot().editing).is_none();
    assert_that!(s.deck()).is_equal_to(&before);
    let view = s.templates_view(Some("grilles-copie".into())).unwrap();
    assert_that!(view.templates[0].template.name.as_str()).is_equal_to("Ma grille");
    assert_that!(view.sets.iter().any(|x| x.id == "grilles-copie" && !x.builtin)).is_true();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn edit_templates_switching_set_keeps_deck_and_history_aside() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.apply("Notes", vec![Op::SetNotes { slide: "s-fin".into(), notes: "à garder".into() }]).unwrap();
    let before = s.deck().clone();

    s.edit_templates("grilles").unwrap();
    s.edit_templates("essentiel").unwrap();
    let snap = s.snapshot();
    assert_that!(snap.editing.unwrap().set.as_str()).is_equal_to("essentiel");
    asserting("l'édition des modèles a son propre historique").that(&snap.can_undo).is_false();

    s.close_templates().unwrap();
    assert_that!(s.deck()).is_equal_to(&before);
    let snap = s.snapshot();
    asserting("le deck retrouve son état").that(&snap.dirty).is_true();
    asserting("le deck retrouve son historique").that(&snap.can_undo).is_true();
}

#[test]
fn errors_localize_in_the_configured_language() {
    use oku_slides_session::{Localize, Localizer};
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let fr = Localizer::new(Some("fr"));
    assert_that!(s.slide("absente").unwrap_err().localize(&fr)).is_equal_to("Slide introuvable : absente".to_owned());
    let refused = s.apply("Mauvais", vec![Op::RemoveElement { slide: "s-fin".into(), element: "absent".into() }]);
    assert_that!(refused.unwrap_err().localize(&fr))
        .is_equal_to("Opération 0 refusée : Élément introuvable : absent".to_owned());
    let en = Localizer::new(Some("en"));
    assert_that!(s.slide("absente").unwrap_err().localize(&en)).is_equal_to("Slide not found: absente".to_owned());
}

/// Messages A2UI de la requête `panel` pour une slide.
fn panel(s: &mut Session, slide: &str) -> Vec<serde_json::Value> {
    let req = serde_json::from_value(json!({ "cmd": "panel", "slide": slide }))
        .unwrap_or_else(|e| panic!("requête panel invalide : {e}"));
    let reply = s.handle(req).unwrap_or_else(|e| panic!("panel refusé : {}", e.one_line()));
    serde_json::from_str(&reply).unwrap_or_else(|e| panic!("réponse panel illisible : {e}"))
}

/// Le composant `id` des messages `updateComponents`.
fn component<'a>(msgs: &'a [serde_json::Value], id: &str) -> Option<&'a serde_json::Value> {
    msgs.iter().filter_map(|m| m["updateComponents"]["components"].as_array()).flatten().find(|c| c["id"] == id)
}

#[test]
fn panel_outside_template_editing_is_empty() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    assert_that!(panel(&mut s, "s-libre")).is_empty();
}

#[test]
fn panel_while_editing_templates_binds_name_and_description() {
    let mut s = session();
    s.edit_templates("grilles").unwrap();
    let first = s.snapshot().slides[0].id.to_owned();
    let name = s.deck().slides[0].name.clone();
    let msgs = panel(&mut s, &first);

    assert_that!(msgs[0]["createSurface"]["catalogId"].as_str()).is_equal_to(Some(oku_slides_a2ui::CATALOG_ID));
    let data = msgs.iter().find_map(|m| m.get("updateDataModel")).map(|u| u["value"].clone());
    assert_that!(data.map(|d| d["template"]["name"].clone())).is_equal_to(Some(json!(name)));
    assert_that!(component(&msgs, "template-name").cloned()).is_some().is_equal_to(json!({
        "id": "template-name", "component": "TextField", "label": "Nom", "value": { "path": "/template/name" },
        "action": { "event": { "name": "rename_template",
            "context": { "slide": first, "value": { "path": "/template/name" } } } }
    }));
    assert_that!(component(&msgs, "template-title").map(|c| c["text"].clone())).is_equal_to(Some(json!("Ce modèle")));
}

#[test]
fn panel_while_editing_templates_binds_grid_steppers() {
    let mut s = session();
    s.edit_templates("grilles").unwrap();
    let first = s.snapshot().slides[0].id.to_owned();
    let columns = s.slide(&first).unwrap().zone.and_then(|z| z.grid.as_ref()).map(|g| g.columns);
    let msgs = panel(&mut s, &first);

    let data = msgs.iter().find_map(|m| m.get("updateDataModel")).map(|u| u["value"]["grid"]["columns"].clone());
    assert_that!(data).is_equal_to(columns.map(|c| json!(c)));
    let stepper = component(&msgs, "grid-columns").cloned();
    assert_that!(stepper.as_ref().map(|c| c["value"].clone())).is_equal_to(Some(json!({ "path": "/grid/columns" })));
    assert_that!(stepper.map(|c| c["action"]["event"]["name"].clone())).is_equal_to(Some(json!("set_grid")));
}

#[test]
fn appearance_carries_the_chart_palette_of_the_deck_theme() {
    let mut s = session();
    s.apply("Thème", vec![serde_json::from_value(json!({ "op": "set_theme", "theme": "papier" })).unwrap()]).unwrap();
    assert_that!(s.snapshot().appearance.chart_palette.first().map(String::as_str)).is_equal_to(Some("#3f6b5e"));
    s.apply("Thème", vec![serde_json::from_value(json!({ "op": "set_theme", "theme": "pro" })).unwrap()]).unwrap();
    assert_that!(s.snapshot().appearance.chart_palette.first().map(String::as_str)).is_equal_to(Some("#0072b0"));
}

#[test]
fn chart_palette_request_no_longer_exists() {
    assert_that!(serde_json::from_value::<Request>(json!({ "cmd": "chart_palette" })).is_err()).is_true();
}

fn req(v: serde_json::Value) -> Request {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("requête invalide : {e}"))
}

fn snapshot_json(s: &mut Session) -> serde_json::Value {
    let reply = s.handle(Request::Snapshot).unwrap_or_else(|e| panic!("snapshot refusé : {}", e.one_line()));
    serde_json::from_str(&reply).unwrap_or_else(|e| panic!("snapshot illisible : {e}"))
}

#[test]
fn ui_request_is_kept_in_snapshot() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": "s-1", "grid": false}))).unwrap();
    assert_that!(snapshot_json(&mut s)["ui"]).is_equal_to(json!({"slide": "s-1", "grid": false}));
}

#[test]
fn snapshot_ui_defaults_to_grid_shown_and_no_slide() {
    let mut s = session();
    assert_that!(snapshot_json(&mut s)["ui"]).is_equal_to(json!({"slide": null, "grid": true}));
}

#[test]
fn new_deck_forgets_the_remembered_slide_but_keeps_grid() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": "s-1", "grid": false}))).unwrap();
    s.handle(req(json!({"cmd": "new"}))).unwrap();
    assert_that!(snapshot_json(&mut s)["ui"]).is_equal_to(json!({"slide": null, "grid": false}));
}

#[test]
fn resume_after_edits_restores_deck_history_and_ui() {
    let data = data_dir();
    let mut a = resumed(&data, None);
    a.handle(set_theme("pro")).unwrap();
    a.handle(req(json!({"cmd": "ui", "slide": "s-1", "grid": false}))).unwrap();
    let before = snapshot_json(&mut a);
    drop(a);
    let mut b = resumed(&data, None);
    assert_that!(snapshot_json(&mut b)).is_equal_to(&before);
    b.undo().unwrap();
    assert_that!(b.deck().theme.as_deref()).is_equal_to(Some("papier"));
}

#[test]
fn resume_while_editing_templates_restores_mode_and_stashed_deck() {
    let data = data_dir();
    let mut a = resumed(&data, None);
    a.handle(set_theme("pro")).unwrap();
    a.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_SET}))).unwrap();
    drop(a);
    let mut b = resumed(&data, None);
    assert_that!(snapshot_json(&mut b)["editing"]["set"]).is_equal_to(json!(oku_slides_session::DEFAULT_SET));
    b.handle(req(json!({"cmd": "close_templates"}))).unwrap();
    let snap = snapshot_json(&mut b);
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
    assert_that!(snap["dirty"]).is_equal_to(json!(true));
}

#[test]
fn resume_saved_deck_modified_elsewhere_reopens_from_disk() {
    let data = data_dir();
    let file = data.join("essai.oku-slides.pdf");
    let mut a = resumed(&data, None);
    a.handle(req(json!({"cmd": "save", "path": file}))).unwrap();
    drop(a);
    // Un autre lancement (sans reprise) change le fichier.
    let mut other = session();
    other.open(&file).unwrap();
    other.apply("Thème", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    other.save(Some(&file)).unwrap();
    let mut b = resumed(&data, None);
    let snap = snapshot_json(&mut b);
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
    assert_that!(snap["can_undo"]).is_equal_to(json!(false));
}

#[test]
fn resume_with_corrupt_state_starts_blank() {
    let data = data_dir();
    std::fs::create_dir_all(data.join("resume")).unwrap();
    std::fs::write(data.join("resume/state.json"), b"pas du json").unwrap();
    let mut b = resumed(&data, None);
    let snap = snapshot_json(&mut b);
    assert_that!(snap["slides"].as_array().unwrap().len()).is_equal_to(2);
    assert_that!(snap["dirty"]).is_equal_to(json!(false));
}

#[test]
fn resume_with_file_and_unsaved_work_keeps_work_with_notice() {
    let data = data_dir();
    let mut a = resumed(&data, None);
    a.handle(set_theme("pro")).unwrap();
    drop(a);
    let mut b = resumed(&data, Some(&fixture_path()));
    let snap = snapshot_json(&mut b);
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
    assert_that!(snap["notice"].as_str()).is_some();
}

#[test]
fn resume_with_file_and_no_unsaved_work_opens_file() {
    let data = data_dir();
    drop(resumed(&data, None));
    let mut b = resumed(&data, Some(&fixture_path()));
    assert_that!(snapshot_json(&mut b)["slides"].as_array().unwrap().len()).is_equal_to(8);
}

#[test]
fn resume_with_file_while_editing_templates_and_dirty_deck_keeps_notice_on_deck() {
    let data = data_dir();
    let mut a = resumed(&data, None);
    a.handle(set_theme("pro")).unwrap();
    a.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_SET}))).unwrap();
    drop(a);
    let mut b = resumed(&data, Some(&fixture_path()));
    b.handle(req(json!({"cmd": "close_templates"}))).unwrap();
    let snap = snapshot_json(&mut b);
    asserting("l'avis suit le deck mis de côté, pas les modèles en cours").that(&snap["notice"].as_str()).is_some();
}

#[test]
fn session_without_resume_writes_nothing() {
    let data = data_dir();
    let mut s = session_in(&data);
    s.handle(set_theme("pro")).unwrap();
    assert_that!(data.join("resume").exists()).is_false();
}
