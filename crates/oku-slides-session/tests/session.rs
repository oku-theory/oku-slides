use oku_slides_core::{Deck, Frame, Op, Paragraph};
use oku_slides_session::{FieldKind, OwnerKind, Request, Session, Settings, paragraphs_to_text};
use serde_json::json;
use speculoos::prelude::*;

mod common;
use common::{
    Isolated, data_dir, fixture_path, req, session, session_in, session_with_settings, set_theme, snapshot_json,
};

/// Une session qui reprend l'état laissé dans `data`.
fn resumed(data: &std::path::Path) -> Session {
    let mut s = session_in(data);
    assert!(s.resume(data.join("resume")), "reprise refusée");
    s
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
    assert_that!(texte.kind).is_equal_to("text");
    let body = view.fields.iter().find(|f| f.owner == "e-texte" && f.field == "body").unwrap();
    assert_that!(body.text.as_ref()).is_equal_to("Un bloc posé à 40 pt du coin");
    assert_that!(body.kind).is_equal_to(FieldKind::Rich);
    let title = view.fields.iter().find(|f| f.owner == "s-libre" && f.field == "title").unwrap();
    assert_that!(title.owner_kind).is_equal_to(OwnerKind::Slide);
    let (before_hash, texte_x) = (view.hash, texte.x);

    s.apply(
        "Déplacer",
        vec![Op::SetFrame {
            slide: "s-libre".into(),
            element: "e-texte".into(),
            frame: Some(Frame { x: 100.0, y: 60.0, w: 720.0, h: 200.0 }),
        }],
        None,
    )
    .unwrap();
    let snap = s.snapshot();
    assert_that!(snap.dirty).is_true();
    assert_that!(snap.can_undo).is_true();
    assert_that!(snap.undo_label.as_deref()).is_equal_to(Some("Déplacer"));
    let moved = s.slide("s-libre").unwrap();
    assert_that!(moved.hash).is_not_equal_to(before_hash);
    let t2 = moved.targets.iter().find(|t| t.id == "e-texte").unwrap();
    assert_that!(t2.x - texte_x).is_close_to(60.0, 0.01);

    s.undo().unwrap();
    assert_that!(s.slide("s-libre").unwrap().hash).is_equal_to(before_hash);
    s.redo().unwrap();

    let dir = data_dir();
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
            None,
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
        "ops": [{"op": "set_param", "target": {"slide": "s-1"}, "param": "title", "value": "Nouveau"}]
    }))
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&s.handle(req).unwrap()).unwrap();
    assert_that!(v["slides"][1]["title"]).is_equal_to(json!("Nouveau"));
    asserting("empreinte en hex").that(&v["slides"][1]["hash"].as_str().map(str::len)).is_equal_to(Some(32));
    let svg = s.handle(serde_json::from_value(json!({"cmd": "svg", "id": "s-1"})).unwrap()).unwrap();
    let svg: String = serde_json::from_str(&svg).unwrap();
    assert_that!(svg.as_str()).starts_with("<svg");
    // Forme JSON : une cible est une boîte d'élément, un champ porte son objet.
    let view: serde_json::Value =
        serde_json::from_str(&s.handle(serde_json::from_value(json!({"cmd": "slide", "id": "s-1"})).unwrap()).unwrap())
            .unwrap();
    let text = view["targets"].as_array().unwrap().iter().find(|t| t["id"] == "b-1").unwrap();
    assert_that!(text["kind"]).is_equal_to(json!("text"));
    let fields = view["fields"].as_array().unwrap();
    let title = fields.iter().find(|f| f["owner"] == "s-1" && f["field"] == "title").unwrap();
    assert_that!(title["owner_kind"]).is_equal_to(json!("slide"));
    assert_that!(title["kind"]).is_equal_to(json!("plain"));
    assert_that!(title["text"]).is_equal_to(json!("Nouveau"));
    let body = fields.iter().find(|f| f["owner"] == "b-1").unwrap();
    assert_that!(body["owner_kind"]).is_equal_to(json!("element"));
    assert_that!(body["widget"]).is_equal_to(json!("text"));
    assert_that!(body["field"]).is_equal_to(json!("body"));
    assert_that!(body["kind"]).is_equal_to(json!("rich"));
}

#[test]
fn paragraphs_to_text_bold_run_and_bullet_gives_markdown_lines() {
    let paras = vec![
        Paragraph {
            runs: vec![oku_slides_core::Run { t: "Gras".into(), bold: true, ..Default::default() }],
            list: None,
        },
        Paragraph { runs: vec![oku_slides_core::Run::plain("Point")], list: Some(oku_slides_core::ListKind::Bullet) },
    ];
    assert_that!(paragraphs_to_text(&paras)).is_equal_to("**Gras**\n- Point".to_owned());
}

#[test]
fn snapshot_appearance_follows_theme_and_deck_overrides() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let papier = s.snapshot().appearance.clone();
    assert_that!(papier.scheme.as_str()).is_equal_to("light");
    assert_that!(papier.slide["accent"].as_str()).is_equal_to("#3f6b5e");
    assert_that!(papier.app["desk"].as_str()).is_equal_to("#ece8e0");
    asserting("les références par nom sont résolues").that(&papier.app["ink"].as_str()).is_equal_to("#2e2b26");
    assert_that!(papier.font_display.first().map(String::as_str)).is_equal_to(Some("Newsreader"));

    s.apply("Thème", vec![Op::SetTheme { theme: common::theme_ref("pro") }], None).unwrap();
    let pro = s.snapshot().appearance;
    assert_that!(pro.scheme.as_str()).is_equal_to("light");
    assert_that!(pro.app["desk"].as_str()).is_equal_to("#e1e5ea");
    assert_that!(pro.app["ink"].as_str()).is_equal_to("#00154e");
    assert_that!(pro.app["accent-text"].as_str()).is_equal_to("#0072b0");
    let fallback = [
        "Noto Sans Arabic",
        "Noto Sans Devanagari",
        "Noto Sans Bengali",
        "Noto Sans Thai",
        "Noto Sans SC",
        "Noto Sans JP",
        "Noto Sans TC",
        "Noto Sans KR",
    ];
    let expected: Vec<String> = std::iter::once("Inter").chain(fallback).map(str::to_owned).collect();
    assert_that!(pro.font_body).is_equal_to(expected);
    assert_that!(pro.metrics["radius-lg"]).is_close_to(16.0, f64::EPSILON);
    assert_that!(pro.metrics["elevation"]).is_close_to(1.5, f64::EPSILON);

    // `set_theme` à `null` : le thème par défaut, dans sa version courante.
    s.apply("Thème par défaut", vec![Op::SetTheme { theme: None }], None).unwrap();
    assert_that!(s.snapshot().appearance).is_equal_to(&papier);

    // Une surcharge du deck l'emporte sur le thème, pour les slides comme pour l'app.
    s.apply(
        "Accent",
        vec![
            Op::SetTheme { theme: common::theme_ref("pro") },
            Op::SetToken { path: vec!["color".into(), "accent".into()], value: json!("#123456") },
            Op::SetToken { path: vec!["app".into(), "desk".into()], value: json!("#222222") },
        ],
        None,
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

    s.apply("Thème", vec![Op::SetTheme { theme: common::theme_ref("pro") }], None).unwrap();
    let pro = s.templates_view(None).unwrap().templates;
    asserting("l'aperçu suit le thème").that(&papier[0].svg).is_not_equal_to(&pro[0].svg);

    // Chaque modèle s'insère dans le deck.
    for (i, t) in pro.iter().enumerate() {
        let mut slide = t.template.slide.clone();
        slide.id = format!("nouvelle-{i}");
        assert_that!(s.apply("Nouvelle slide", vec![Op::AddSlide { index: None, defaults: false, slide }], None))
            .is_ok();
    }
}

#[test]
fn edit_builtin_templates_then_save_as_copy_creates_a_new_pack_and_close_restores_deck() {
    use serde_json::json;
    let dir = data_dir();
    let mut s = session();
    s.set_templates_dir(&*dir);
    let before = s.deck().clone();

    s.edit_templates(oku_slides_session::GRID_PACK).unwrap();
    let snap = s.snapshot();
    assert_that!(snap.editing.as_ref().unwrap().set).is_equal_to(oku_slides_session::GRID_PACK);
    assert_that!(snap.editing.as_ref().unwrap().builtin).is_true();
    let first = snap.slides[0].id.to_owned();
    s.apply(
        "Renommer",
        serde_json::from_value(json!([
            { "op": "set_slide_field", "slide": first, "field": "name", "value": "Ma grille" }
        ]))
        .unwrap(),
        None,
    )
    .unwrap();
    // Comme copie : un nouveau pack, sur lequel l'édition continue.
    s.save_templates(true).unwrap();
    let mode = s.snapshot().editing.map(|e| e.mode.clone()).unwrap();
    assert_that!(mode.builtin).is_false();
    assert_that!(mode.set).is_not_equal_to(oku_slides_session::GRID_PACK);
    assert_that!(mode.name.ends_with("(copie)")).is_true();
    let version = mode.session_version.clone().unwrap();
    let file = dir.join(mode.set.to_string()).join("versions").join(version.hex()).join("pack.json");
    assert_that!(file.exists()).is_true();

    s.close_templates().unwrap();
    assert_that!(s.snapshot().editing).is_none();
    // Le deck retrouve ses slides, et passe à la copie.
    assert_that!(s.deck().slides).is_equal_to(&before.slides);
    assert_that!(s.deck().templates.as_ref().map(|r| r.id)).is_equal_to(Some(mode.set));
    let view = s.templates_view(Some(mode.set)).unwrap();
    assert_that!(view.templates[0].template.name.as_str()).is_equal_to("Ma grille");
    assert_that!(view.sets.iter().any(|x| x.id == mode.set && !x.builtin)).is_true();
}

#[test]
fn edit_templates_switching_set_keeps_deck_and_history_aside() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.apply("Notes", vec![Op::SetNotes { slide: "s-fin".into(), notes: "à garder".into() }], None).unwrap();
    let before = s.deck().clone();

    s.edit_templates(oku_slides_session::GRID_PACK).unwrap();
    s.edit_templates(oku_slides_session::DEFAULT_PACK).unwrap();
    let snap = s.snapshot();
    assert_that!(snap.editing.as_ref().unwrap().set).is_equal_to(oku_slides_session::DEFAULT_PACK);
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
    let refused = s.apply("Mauvais", vec![Op::RemoveElement { slide: "s-fin".into(), element: "absent".into() }], None);
    assert_that!(refused.unwrap_err().localize(&fr))
        .is_equal_to("Opération 0 refusée : Élément introuvable : absent".to_owned());
    let en = Localizer::new(Some("en"));
    assert_that!(s.slide("absente").unwrap_err().localize(&en)).is_equal_to("Slide not found: absente".to_owned());
}

#[test]
fn appearance_carries_the_chart_palette_of_the_deck_theme() {
    let mut s = session();
    s.apply(
        "Thème",
        vec![serde_json::from_value(json!({ "op": "set_theme", "theme": common::theme_ref("papier") })).unwrap()],
        None,
    )
    .unwrap();
    assert_that!(s.snapshot().appearance.chart_palette.first().map(String::as_str)).is_equal_to(Some("#3f6b5e"));
    s.apply(
        "Thème",
        vec![serde_json::from_value(json!({ "op": "set_theme", "theme": common::theme_ref("pro") })).unwrap()],
        None,
    )
    .unwrap();
    assert_that!(s.snapshot().appearance.chart_palette.first().map(String::as_str)).is_equal_to(Some("#0072b0"));
}

#[test]
fn chart_palette_request_no_longer_exists() {
    assert_that!(serde_json::from_value::<Request>(json!({ "cmd": "chart_palette" })).is_err()).is_true();
}

#[test]
fn edit_records_the_configured_author() {
    let data = data_dir();
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    settings.author.name = Some("Julie".into());
    let mut s = session_with_settings(&data, settings);
    s.handle(set_theme("pro")).unwrap();
    let r = s.sources_for_tests().replica();
    assert_that!(r.author_of(r.peer_id())).is_equal_to(Some("Julie".to_owned()));
}

#[test]
fn ui_request_is_kept_in_snapshot() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": "s-1", "grid": false, "ghosts": true}))).unwrap();
    assert_that!(snapshot_json(&mut s)["ui"])
        .is_equal_to(json!({"slide": "s-1", "grid": false, "ghosts": true, "pile": null, "notes": null, "widgets": [], "widget": null, "code": false, "code_width": null, "band": null, "folded": []}));
}

#[test]
fn ui_request_with_column_widths_keeps_them_in_snapshot() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": null, "grid": true, "ghosts": false, "pile": 200, "notes": 520})))
        .unwrap();
    let ui = snapshot_json(&mut s)["ui"].clone();
    assert_that!((ui["pile"].clone(), ui["notes"].clone())).is_equal_to((json!(200), json!(520)));
}

/// Le code source des widgets, déplié, est gardé.
#[test]
fn ui_request_with_code_shown_keeps_it_in_snapshot() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": null, "grid": true, "code": true}))).unwrap();
    assert_that!(snapshot_json(&mut s)["ui"]["code"]).is_equal_to(json!(true));
}

/// La largeur du code des widgets est gardée.
#[test]
fn ui_request_with_code_width_keeps_it_in_snapshot() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": null, "grid": true, "code_width": 640}))).unwrap();
    assert_that!(snapshot_json(&mut s)["ui"]["code_width"]).is_equal_to(json!(640));
}

/// La hauteur de la bande sous la scène est gardée.
#[test]
fn ui_request_with_band_height_keeps_it_in_snapshot() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": null, "grid": true, "band": 320}))).unwrap();
    assert_that!(snapshot_json(&mut s)["ui"]["band"]).is_equal_to(json!(320));
}

/// Les sections repliées de la pile sont gardées.
#[test]
fn ui_request_with_folded_sections_keeps_them_in_snapshot() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": null, "grid": true, "folded": ["widgets"]}))).unwrap();
    assert_that!(snapshot_json(&mut s)["ui"]["folded"]).is_equal_to(json!(["widgets"]));
}

/// Une section inconnue est refusée à la frontière.
#[test]
fn ui_request_with_unknown_folded_section_is_refused() {
    let r = serde_json::from_value::<Request>(json!({"cmd": "ui", "slide": null, "folded": ["notes"]}));
    assert_that!(r.is_err()).is_true();
}

/// Une interface d'avant les fantômes n'envoie pas `ghosts` : ils restent masqués.
#[test]
fn ui_request_without_ghosts_keeps_them_hidden() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": "s-1", "grid": false}))).unwrap();
    assert_that!(snapshot_json(&mut s)["ui"]["ghosts"]).is_equal_to(json!(false));
}

#[test]
fn snapshot_ui_defaults_to_grid_shown_ghosts_hidden_and_no_slide() {
    let mut s = session();
    assert_that!(snapshot_json(&mut s)["ui"])
        .is_equal_to(json!({"slide": null, "grid": true, "ghosts": false, "pile": null, "notes": null, "widgets": [], "widget": null, "code": false, "code_width": null, "band": null, "folded": []}));
}

#[test]
fn new_deck_forgets_the_remembered_slide_but_keeps_grid_ghosts_and_columns() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": "s-1", "grid": false, "ghosts": true, "pile": 200, "notes": 520})))
        .unwrap();
    s.new_deck().unwrap();
    assert_that!(snapshot_json(&mut s)["ui"]).is_equal_to(
        json!({"slide": null, "grid": false, "ghosts": true, "pile": 200, "notes": 520, "widgets": [], "widget": null, "code": false, "code_width": null, "band": null, "folded": []}),
    );
}

#[test]
fn resume_after_edits_restores_deck_and_ui_but_not_undo() {
    let data = data_dir();
    let mut a = resumed(&data);
    a.handle(set_theme("pro")).unwrap();
    a.handle(req(json!({"cmd": "ui", "slide": "s-1", "grid": false, "ghosts": true, "pile": 200, "notes": 520})))
        .unwrap();
    let mut before = snapshot_json(&mut a);
    drop(a);
    let mut b = resumed(&data);
    // La pile d'annulation ne survit pas au relancement ; tout le reste, si.
    before["can_undo"] = json!(false);
    before["undo_label"] = json!(null);
    assert_that!(snapshot_json(&mut b)).is_equal_to(&before);
    b.undo().unwrap();
    assert_that!(b.deck().theme.as_ref().and_then(common::theme_name)).is_equal_to(Some("pro"));
}

#[test]
fn resume_while_editing_templates_restores_mode_and_stashed_deck() {
    let data = data_dir();
    let mut a = resumed(&data);
    a.handle(set_theme("pro")).unwrap();
    a.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_PACK}))).unwrap();
    drop(a);
    let mut b = resumed(&data);
    assert_that!(snapshot_json(&mut b)["editing"]["set"]).is_equal_to(json!(oku_slides_session::DEFAULT_PACK));
    b.handle(req(json!({"cmd": "close_templates"}))).unwrap();
    let snap = snapshot_json(&mut b);
    assert_that!(snap["theme"]["id"]).is_equal_to(json!(oku_slides_session::PRO_THEME));
    assert_that!(snap["dirty"]).is_equal_to(json!(true));
}

#[test]
fn resume_saved_deck_modified_elsewhere_reopens_from_disk() {
    let data = data_dir();
    let file = data.join("essai.oku-slides.pdf");
    let mut a = resumed(&data);
    a.handle(req(json!({"cmd": "save", "path": file}))).unwrap();
    drop(a);
    // Un autre lancement (sans reprise) change le fichier.
    let mut other = session();
    other.open(&file).unwrap();
    other.apply("Thème", vec![Op::SetTheme { theme: common::theme_ref("pro") }], None).unwrap();
    other.save(Some(&file)).unwrap();
    let mut b = resumed(&data);
    let snap = snapshot_json(&mut b);
    assert_that!(snap["theme"]["id"]).is_equal_to(json!(oku_slides_session::PRO_THEME));
    assert_that!(snap["can_undo"]).is_equal_to(json!(false));
}

#[test]
fn resume_with_corrupt_state_starts_blank() {
    let data = data_dir();
    std::fs::create_dir_all(data.join("resume")).unwrap();
    std::fs::write(data.join("resume/state.json"), b"pas du json").unwrap();
    let mut b = session_in(&data);
    assert_that!(b.resume(data.join("resume"))).is_false();
    let snap = snapshot_json(&mut b);
    assert_that!(snap["slides"].as_array().unwrap().len()).is_equal_to(2);
    assert_that!(snap["dirty"]).is_equal_to(json!(false));
}

#[test]
fn resume_with_corrupt_state_leaves_a_pristine_blank_deck() {
    let data = data_dir();
    std::fs::create_dir_all(data.join("resume")).unwrap();
    std::fs::write(data.join("resume/state.json"), b"pas du json").unwrap();
    let mut b = session_in(&data);
    assert_that!(b.resume(data.join("resume"))).is_false();
    assert_that!(b.tab_info().pristine).is_true();
}

#[test]
fn resume_in_empty_dir_returns_true_and_writes_state() {
    let data = data_dir();
    let mut s = session_in(&data);
    assert_that!(s.resume(data.join("resume"))).is_true();
    assert_that!(data.join("resume/state.json").exists()).is_true();
}

#[test]
fn tab_info_new_session_is_pristine_clean_and_named_like_the_snapshot() {
    let s = session();
    let info = s.tab_info();
    assert_that!(info.pristine).is_true();
    assert_that!(info.dirty).is_false();
    assert_that!(info.editing_templates).is_false();
    assert_that!(info.path).is_none();
    assert_that!(info.title).is_equal_to(s.snapshot().name);
}

#[test]
fn tab_info_after_new_deck_is_pristine() {
    let mut s = session();
    s.handle(set_theme("pro")).unwrap();
    s.new_deck().unwrap();
    assert_that!(s.tab_info().pristine).is_true();
}

#[test]
fn tab_info_after_edit_is_dirty_and_no_longer_pristine() {
    let mut s = session();
    s.handle(set_theme("pro")).unwrap();
    let info = s.tab_info();
    assert_that!(info.dirty).is_true();
    assert_that!(info.pristine).is_false();
}

#[test]
fn tab_info_after_undoing_every_edit_is_not_pristine() {
    let mut s = session();
    s.handle(set_theme("pro")).unwrap();
    s.undo().unwrap();
    assert_that!(s.tab_info().pristine).is_false();
}

/// Un deck JSON ouvert n'a pas de fichier associé : il n'est pas vierge
/// pour autant, et ouvrir un autre fichier ne doit pas le remplacer.
#[test]
fn tab_info_after_opening_a_json_deck_is_not_pristine() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let info = s.tab_info();
    assert_that!(info.path).is_none();
    assert_that!(info.pristine).is_false();
}

#[test]
fn tab_info_after_save_has_file_name_as_title_and_resolved_path() {
    let data = data_dir();
    let file = data.join("essai.oku-slides.pdf");
    let mut s = session_in(&data);
    s.handle(req(json!({"cmd": "save", "path": data.join("./x/../essai.oku-slides.pdf")}))).unwrap();
    let info = s.tab_info();
    assert_that!(info.title.as_str()).is_equal_to("essai");
    assert_that!(info.path).is_equal_to(Some(file));
    assert_that!(info.pristine).is_false();
}

#[test]
fn tab_info_editing_templates_keeps_deck_title_and_reports_mode() {
    let mut s = session();
    let title = s.tab_info().title;
    s.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_PACK}))).unwrap();
    let info = s.tab_info();
    assert_that!(info.editing_templates).is_true();
    assert_that!(info.title).is_equal_to(title);
    assert_that!(info.pristine).is_false();
}

/// Un deck propre mis de côté ne rend pas l'onglet non enregistré : seul le
/// deck (ou l'ensemble de modèles) réellement modifié doit compter.
#[test]
fn tab_info_editing_templates_with_a_clean_stashed_deck_is_not_dirty() {
    let mut s = session();
    s.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_PACK}))).unwrap();
    assert_that!(s.tab_info().dirty).is_false();
}

/// Le deck mis de côté pendant l'édition des modèles garde son travail non
/// enregistré, que l'onglet doit continuer à signaler.
#[test]
fn tab_info_editing_templates_reports_dirty_from_the_stashed_deck() {
    let mut s = session();
    s.handle(set_theme("pro")).unwrap();
    s.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_PACK}))).unwrap();
    assert_that!(s.tab_info().dirty).is_true();
}

#[test]
fn set_notice_is_shown_once_in_the_snapshot() {
    let mut s = session();
    s.set_notice("Attention".into());
    assert_that!(snapshot_json(&mut s)["notice"]).is_equal_to(json!("Attention"));
    assert_that!(snapshot_json(&mut s)["notice"]).is_equal_to(json!(null));
}

/// Test de caractérisation (vert dès l'écriture, et doit le rester) : deux
/// onglets, un seul dossier de modèles ; le cache d'aperçus d'une session
/// suit ce qu'une autre enregistre, car il compare l'ensemble relu.
#[test]
fn templates_view_after_other_session_saves_the_set_shows_the_new_templates() {
    let data = data_dir();
    let mut a = session_in(&data);
    a.edit_templates(oku_slides_session::DEFAULT_PACK).unwrap();
    a.save_templates(true).unwrap();
    let id: uuid::Uuid = snapshot_json(&mut a)["editing"]["set"].as_str().unwrap().parse().unwrap();
    let mut b = session_in(&data);
    let before = b.templates_view(Some(id)).unwrap().templates.len();
    let first = a.deck().slides[0].id.clone();
    a.apply("Retirer", vec![Op::RemoveSlide { slide: first }], None).unwrap();
    a.save_templates(false).unwrap();
    let after = b.templates_view(Some(id)).unwrap().templates.len();
    assert_that!(after).is_equal_to(before - 1);
}

#[test]
fn resume_with_unreadable_state_moves_old_data_aside_instead_of_overwriting() {
    let data = data_dir();
    let resume_dir = data.join("resume");
    std::fs::create_dir_all(resume_dir.join("deck")).unwrap();
    // Version future : `read_state` échoue avant même de lire le deck.
    std::fs::write(resume_dir.join("state.json"), br#"{"version":99,"mode":"deck"}"#).unwrap();
    let doc_bytes = br#"{"deck":{"version":1,"theme":"papier","slides":[]}}"#.to_vec();
    std::fs::write(resume_dir.join("deck/document.json"), &doc_bytes).unwrap();

    let mut s = session_in(&data);
    assert_that!(s.resume(resume_dir.clone())).is_false();

    // La reprise a échoué : deck vierge, rien d'anormal en apparence.
    let snap = s.snapshot();
    assert_that!(snap.dirty).is_false();
    assert_that!(snap.slides.len()).is_equal_to(2);

    // Les anciennes données ne sont pas perdues : déplacées de côté.
    let moved: Vec<_> = std::fs::read_dir(&data)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("resume.failed-")))
        .collect();
    assert_that!(moved.len()).is_equal_to(1);
    let old_bytes = std::fs::read(moved[0].join("deck/document.json")).unwrap();
    assert_that!(old_bytes).is_equal_to(doc_bytes);
    // Rien n'est écrit à la place : l'onglet sera retiré par l'espace de travail.
    assert_that!(resume_dir.exists()).is_false();
}

/// Quand le déplacement de côté lui-même échoue (ici : dossier parent en
/// lecture seule, donc `rename` impossible), la reprise ne doit rien écrire
/// ce lancement-ci plutôt que de risquer d'écraser les anciennes données.
#[cfg(unix)]
#[test]
fn resume_with_unreadable_state_and_failed_quarantine_disables_resume_but_keeps_old_data() {
    use std::os::unix::fs::PermissionsExt;

    let data = data_dir();
    let resume_dir = data.join("resume");
    std::fs::create_dir_all(resume_dir.join("deck")).unwrap();
    std::fs::write(resume_dir.join("state.json"), br#"{"version":99,"mode":"deck"}"#).unwrap();
    let doc_bytes = br#"{"deck":{"version":1,"theme":"papier","slides":[]}}"#.to_vec();
    std::fs::write(resume_dir.join("deck/document.json"), &doc_bytes).unwrap();

    // Le dossier parent en lecture seule empêche de créer un dossier voisin :
    // le déplacement de côté échoue.
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o555)).unwrap();
    let mut s = session_in(&data);
    assert_that!(s.resume(resume_dir.clone())).is_false();
    // On restaure les permissions tout de suite, pour pouvoir nettoyer et
    // relire le dossier ensuite, échec de la reprise ou pas.
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Le deck est vierge malgré l'échec du déplacement.
    let snap = s.snapshot();
    assert_that!(snap.dirty).is_false();

    // Rien n'a été écrasé : les anciennes données sont toujours à leur place
    // (le déplacement de côté n'a pas eu lieu).
    let old_bytes = std::fs::read(resume_dir.join("deck/document.json")).unwrap();
    assert_that!(old_bytes).is_equal_to(doc_bytes);

    // Rien de nouveau n'a été écrit non plus (la reprise reste désactivée ce
    // lancement-ci) : `state.json` garde son contenu d'origine (version 99).
    let state = std::fs::read_to_string(resume_dir.join("state.json")).unwrap();
    assert_that!(state.as_str()).contains("99");
}

#[test]
fn session_without_resume_writes_nothing() {
    let data = data_dir();
    let mut s = session_in(&data);
    s.handle(set_theme("pro")).unwrap();
    assert_that!(data.join("resume").exists()).is_false();
}

/// t-145 : en mode modèles, un ensemble de modèles illisible ramène au deck,
/// sans rien perdre du deck mis de côté.
#[test]
fn resume_with_unreadable_templates_returns_to_the_deck_without_loss() {
    let data = data_dir();
    let mut a = resumed(&data);
    a.handle(set_theme("pro")).unwrap();
    a.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_PACK}))).unwrap();
    drop(a);
    let templates = data.join("resume/templates/document.json");
    assert_that!(templates.exists()).is_true();
    std::fs::write(&templates, b"pas du json").unwrap();
    let mut b = resumed(&data);
    let snap = snapshot_json(&mut b);
    assert_that!(snap["editing"]).is_equal_to(json!(null));
    assert_that!(snap["theme"]["id"]).is_equal_to(json!(oku_slides_session::PRO_THEME));
    assert_that!(snap["dirty"]).is_equal_to(json!(true));
}

/// t-394 : reprise en mode modèles, les sous-onglets de widgets reviennent.
#[test]
fn resume_in_templates_mode_returns_the_widget_tabs() {
    let data = data_dir();
    let mut a = resumed(&data);
    a.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_PACK}))).unwrap();
    a.handle(req(json!({"cmd": "ui", "slide": null, "grid": true, "widgets": ["_commun"], "widget": "_commun"})))
        .unwrap();
    drop(a);
    let mut b = resumed(&data);
    let ui = snapshot_json(&mut b)["ui"].clone();
    assert_that!((ui["widgets"].clone(), ui["widget"].clone())).is_equal_to((json!(["_commun"]), json!("_commun")));
}

/// t-394 : modèles illisibles, la reprise revient au deck sans sous-onglets.
#[test]
fn resume_with_unreadable_templates_returns_no_widget_tabs() {
    let data = data_dir();
    let mut a = resumed(&data);
    a.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_PACK}))).unwrap();
    a.handle(req(json!({"cmd": "ui", "slide": null, "grid": true, "widgets": ["_commun"], "widget": "_commun"})))
        .unwrap();
    drop(a);
    std::fs::write(data.join("resume/templates/document.json"), b"pas du json").unwrap();
    let mut b = resumed(&data);
    assert_that!(snapshot_json(&mut b)["ui"]["widgets"].clone()).is_equal_to(json!([]));
}

/// t-145 : un deck illisible fait repartir d'un deck vierge ; l'ancien
/// dossier est mis de côté, jamais écrasé.
#[test]
fn resume_with_unreadable_deck_starts_blank_and_sets_the_folder_aside() {
    let data = data_dir();
    let mut a = resumed(&data);
    a.handle(set_theme("pro")).unwrap();
    drop(a);
    let deck = data.join("resume/deck/document.json");
    assert_that!(deck.exists()).is_true();
    std::fs::write(&deck, b"pas du json").unwrap();
    let mut b = session_in(&data);
    assert_that!(b.resume(data.join("resume"))).is_false();
    assert_that!(b.tab_info().pristine).is_true();
    let aside = std::fs::read_dir(&data)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("resume.failed-"))
        .count();
    assert_that!(aside).is_equal_to(1);
}

/// t-145 : le fichier du deck supprimé du disque, la copie de travail reste.
#[test]
fn resume_with_the_deck_file_deleted_keeps_the_working_copy() {
    let data = data_dir();
    let file = data.join("parti.oku-slides.pdf");
    let mut a = resumed(&data);
    a.handle(set_theme("pro")).unwrap();
    a.handle(req(json!({"cmd": "save", "path": file}))).unwrap();
    drop(a);
    std::fs::remove_file(&file).unwrap();
    let mut b = resumed(&data);
    let snap = snapshot_json(&mut b);
    assert_that!(snap["theme"]["id"]).is_equal_to(json!(oku_slides_session::PRO_THEME));
    assert_that!(snap["dirty"]).is_equal_to(json!(false));
    assert_that!(b.tab_info().path).is_equal_to(Some(oku_slides_platform::storage::resolve(&file)));
}

#[test]
fn appearance_carries_the_dark_interface_palette_of_papier_and_pro() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let papier = s.snapshot().appearance.clone();
    asserting("papier : table sombre")
        .that(&papier.variants["dark"].get("desk").map(String::as_str))
        .is_equal_to(Some("#1f1d1a"));
    s.apply("Thème", vec![Op::SetTheme { theme: common::theme_ref("pro") }], None).unwrap();
    let pro = s.snapshot().appearance;
    asserting("pro : table navy")
        .that(&pro.variants["dark"].get("desk").map(String::as_str))
        .is_equal_to(Some("#000c2e"));
}

const KPI_WIDGET: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", params: (body: p.plain()),
  (args, ctx) => field(ctx, "body", args.body))
"#;

/// Le snapshot nomme les widgets du pack du deck, sans `_commun`.
#[test]
fn snapshot_lists_the_pack_widget_files_without_common() {
    let mut s = session();
    s.handle(req(json!({"cmd": "set_widget", "name": "_commun", "source": "#let x = 1"}))).unwrap();
    s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": KPI_WIDGET}))).unwrap();
    assert_that!(snapshot_json(&mut s)["pack_widgets"]).is_equal_to(json!(["kpi"]));
}

/// En mode modèles, le snapshot nomme les widgets du pack édité, sans `_commun`.
#[test]
fn snapshot_lists_the_edited_pack_widgets_in_template_mode() {
    let mut s = session();
    s.handle(req(json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_PACK}))).unwrap();
    s.handle(req(json!({"cmd": "set_widget", "name": "_commun", "source": "#let x = 1"}))).unwrap();
    s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": KPI_WIDGET}))).unwrap();
    let snap = snapshot_json(&mut s);
    assert_that!(snap["editing"].is_null()).is_false();
    assert_that!(snap["pack_widgets"]).is_equal_to(json!(["kpi"]));
}

/// Sans widget embarqué, la liste est vide.
#[test]
fn snapshot_pack_widgets_is_empty_without_an_embedded_pack() {
    let mut s = session();
    assert_that!(snapshot_json(&mut s)["pack_widgets"]).is_equal_to(json!([]));
}

const KPI_LABELLED: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", label: "KPI", params: (body: p.plain()),
  (args, ctx) => field(ctx, "body", args.body))
"#;

const KPI_DESCRIBED: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("kpi", label: "KPI", description: "Une valeur en grand", params: (body: p.plain()),
  (args, ctx) => field(ctx, "body", args.body))
"#;

const COVER_SLIDE: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("cover", category: "slide", params: (title: p.plain()),
  (args, ctx) => args.title)
"#;

/// La barre d'ajout propose les widgets d'élément du pack, avec leur label.
#[test]
fn snapshot_add_widgets_lists_the_pack_element_widgets_with_their_label() {
    let mut s = session();
    s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": KPI_LABELLED}))).unwrap();
    assert_that!(snapshot_json(&mut s)["add_widgets"])
        .is_equal_to(json!([{"name": "kpi", "label": "KPI", "description": null}]));
}

/// La barre d'ajout porte la description d'un widget du pack, pour le filtre.
#[test]
fn snapshot_add_widgets_carries_the_widget_description() {
    let mut s = session();
    s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": KPI_DESCRIBED}))).unwrap();
    assert_that!(snapshot_json(&mut s)["add_widgets"][0]["description"]).is_equal_to(json!("Une valeur en grand"));
}

/// Un fichier au label mal typé est écarté seul : les autres widgets du pack restent proposés.
#[test]
fn snapshot_add_widgets_keeps_the_pack_when_one_file_has_a_non_text_label() {
    let mut s = session();
    s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": KPI_DESCRIBED}))).unwrap();
    let change = s.set_widget("casse", NON_TEXT_LABEL.into()).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let set_aside = change.status.schema.is_none()
        && change.status.diagnostics.iter().any(|d| d.message.contains("le label doit être un texte"));
    let names: Vec<serde_json::Value> =
        snapshot_json(&mut s)["add_widgets"].as_array().into_iter().flatten().map(|w| w["name"].clone()).collect();
    assert_that!((change.saved, set_aside, names)).is_equal_to((true, true, vec![json!("kpi")]));
}

const NON_TEXT_LABEL: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("casse", label: 42, params: (body: p.plain()),
  (args, ctx) => field(ctx, "body", args.body))
"#;

/// Un widget de slide du pack ne s'ajoute pas à une slide : absent de la liste.
#[test]
fn snapshot_add_widgets_leaves_out_slide_widgets() {
    let mut s = session();
    s.handle(req(json!({"cmd": "set_widget", "name": "cover", "source": COVER_SLIDE}))).unwrap();
    assert_that!(snapshot_json(&mut s)["add_widgets"]).is_equal_to(json!([]));
}

/// Un widget du pack qui ne compile pas n'est pas au registre : absent de la liste.
#[test]
fn snapshot_add_widgets_leaves_out_broken_widgets() {
    let mut s = session();
    let _ = s.handle(req(json!({"cmd": "set_widget", "name": "kpi", "source": "#let widget = ("})));
    assert_that!(snapshot_json(&mut s)["add_widgets"]).is_equal_to(json!([]));
}

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

fn editing() -> (Isolated, String) {
    let mut s = session();
    s.edit_templates(oku_slides_session::GRID_PACK)
        .unwrap_or_else(|e| panic!("édition des modèles refusée : {}", e.one_line()));
    let slide = s.snapshot().slides[0].id.to_owned();
    (s, slide)
}

/// La surface de `host_settings`, en JSON.
fn host(s: &mut Session, slide: &str, widget: Option<&str>) -> serde_json::Value {
    let reply = s
        .handle(Request::HostSettings { slide: slide.into(), widget: widget.map(Into::into) })
        .unwrap_or_else(|e| panic!("réglages de l'hôte refusés : {}", e.one_line()));
    serde_json::from_str(&reply).unwrap_or_else(|e| panic!("réglages de l'hôte illisibles : {e}"))
}

fn host_param(s: &mut Session, slide: &str, key: &str, value: serde_json::Value) -> oku_slides_session::Result<String> {
    s.handle(Request::HostParam { slide: slide.into(), key: key.into(), value })
}

#[test]
fn host_settings_outside_templates_mode_is_null() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    assert_that!(host(&mut s, "s-libre", None)).is_equal_to(json!(null));
}

#[test]
fn host_settings_shows_the_template_name() {
    let (mut s, slide) = editing();
    host_param(&mut s, &slide, "name", json!("Grille de démonstration")).unwrap();
    assert_that!(host(&mut s, &slide, None).to_string().as_str()).contains("Grille de démonstration");
}

#[test]
fn host_settings_of_a_template_with_blocks_has_no_grid_fields() {
    let dir = data_dir();
    let id = uuid::Uuid::new_v4();
    let pack = json!({ "id": id, "name": "À blocs", "created": "2026-10-01T00:00:00Z", "templates": [{
        "id": "cartes", "name": "Cartes", "slide": { "id": "cartes", "kind": "content", "params": {
            "title": "Titre", "blocks": [{ "id": "b", "kind": "text", "params": { "body": [{ "runs": [{ "t": "x" }] }] } }]
        } }
    }] });
    let files = oku_slides_core::PackFiles::from_bytes(pack.to_string().as_bytes(), []).unwrap();
    let library =
        oku_slides_session::PackLibrary::new(Some(dir.to_path_buf()), oku_slides_session::Storage::local().unwrap());
    library.add_version(id, &files).unwrap();
    let mut s = session();
    s.set_templates_dir(&*dir);
    s.edit_templates(id).unwrap();
    let cards = s
        .deck()
        .slides
        .iter()
        .find(|sl| sl.params.slot("blocks").is_some_and(|b| !b.is_empty()))
        .map(|sl| sl.id.clone())
        .unwrap();
    assert_that!(host(&mut s, &cards, None).to_string().contains("\"columns\"")).is_false();
}

#[test]
fn host_settings_of_a_widget_shows_its_example_values() {
    let (mut s, slide) = editing();
    s.set_widget("quote", QUOTE.into()).unwrap();
    s.handle(Request::SetExampleValue { name: "quote".into(), param: "source".into(), value: json!("Essai") }).unwrap();
    assert_that!(host(&mut s, &slide, Some("quote")).to_string().as_str()).contains("Essai");
}

#[test]
fn host_settings_of_a_broken_widget_keeps_the_widget_card() {
    let (mut s, slide) = editing();
    s.set_widget("quote", "#let widget = (".into()).unwrap();
    assert_that!(host(&mut s, &slide, Some("quote"))["id"].as_str()).is_equal_to(Some("host-widget"));
}

#[test]
fn host_settings_of_a_stale_widget_shows_the_template() {
    let (mut s, slide) = editing();
    assert_that!(host(&mut s, &slide, Some("absent"))["id"].as_str()).is_equal_to(Some("host-template"));
}

#[test]
fn host_param_name_renames_the_template() {
    let (mut s, slide) = editing();
    host_param(&mut s, &slide, "name", json!("Renommé")).unwrap();
    assert_that!(s.deck().slide(&slide).unwrap().name.as_deref()).is_equal_to(Some("Renommé"));
}

#[test]
fn host_param_columns_undoes_one_change_at_a_time() {
    let (mut s, slide) = editing();
    host_param(&mut s, &slide, "columns", json!(6)).unwrap();
    let after_first = s.deck().slide(&slide).unwrap().clone();
    host_param(&mut s, &slide, "columns", json!(4)).unwrap();
    s.handle(Request::Undo).unwrap();
    assert_that!(s.deck().slide(&slide).unwrap().clone()).is_equal_to(after_first);
}

#[test]
fn host_param_with_an_unknown_key_is_refused() {
    let (mut s, slide) = editing();
    let err = host_param(&mut s, &slide, "couleur", json!(1)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_session::SessionErrorKind::HostKey);
}

#[test]
fn host_param_outside_templates_mode_is_refused() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let err = host_param(&mut s, "s-libre", "name", json!("x")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_session::SessionErrorKind::NotEditingTemplates);
}

#[test]
fn set_example_value_request_leaves_the_templates_unchanged() {
    let (mut s, _) = editing();
    s.set_widget("quote", QUOTE.into()).unwrap();
    let before = s.deck().slides.clone();
    s.handle(Request::SetExampleValue { name: "quote".into(), param: "source".into(), value: json!("Essai") }).unwrap();
    assert_that!(s.deck().slides.clone()).is_equal_to(before);
}

#[test]
fn snapshot_carries_the_deck_language() {
    let mut s = session();
    s.new_deck().unwrap();
    assert_that!(snapshot_json(&mut s)["lang"]).is_equal_to(json!("fr"));
    s.open(&fixture_path()).unwrap();
    assert_that!(snapshot_json(&mut s)["lang"]).is_equal_to(json!(null));
}
