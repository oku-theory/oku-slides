//! End-to-end session: editing, undo, saving, JSON requests, packs and themes.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
use std::collections::BTreeMap;

use oku_slides_core::{Deck, Frame, Op, Paragraph};
use oku_slides_session::{FieldKind, OwnerKind, Request, Session, Settings, paragraphs_to_text};
use serde_json::json;
use speculoos::prelude::*;

mod common;
use common::{data_dir, fixture_path, req, session, session_in, session_with_settings, set_theme, snapshot_json};

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
    assert_that!(texte.kind).is_equal_to(oku_slides_core::kinds::base::TEXT);
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
    assert_that!(text["kind"]).is_equal_to(json!(oku_slides_core::kinds::base::TEXT));
    let fields = view["fields"].as_array().unwrap();
    let title = fields.iter().find(|f| f["owner"] == "s-1" && f["field"] == "title").unwrap();
    assert_that!(title["owner_kind"]).is_equal_to(json!("slide"));
    assert_that!(title["kind"]).is_equal_to(json!("plain"));
    assert_that!(title["text"]).is_equal_to(json!("Nouveau"));
    let body = fields.iter().find(|f| f["owner"] == "b-1").unwrap();
    assert_that!(body["owner_kind"]).is_equal_to(json!("element"));
    assert_that!(body["widget"]).is_equal_to(json!(oku_slides_core::kinds::base::TEXT));
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

    s.apply(
        "Thème",
        vec![oku_slides_core::Op::SetTheme { theme: Some(common::ns("pro")), themes: BTreeMap::default() }],
        None,
    )
    .unwrap();
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
    s.apply("Thème par défaut", vec![Op::SetTheme { theme: None, themes: BTreeMap::default() }], None).unwrap();
    assert_that!(s.snapshot().appearance).is_equal_to(&papier);

    // Une surcharge du deck l'emporte sur le thème, pour les slides comme pour l'app.
    s.apply(
        "Accent",
        vec![
            oku_slides_core::Op::SetTheme { theme: Some(common::ns("pro")), themes: BTreeMap::default() },
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
fn layouts_view_renders_previews_in_deck_theme() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let papier = s.layouts_view().unwrap().packs.remove(0).layouts;
    assert_that!(papier.len()).is_equal_to(oku_slides_core::kinds::essentiel::ALL.len());
    let again = s.layouts_view().unwrap().packs.remove(0).layouts;
    asserting("rien n'a changé : les aperçus viennent du cache")
        .that(&std::sync::Arc::ptr_eq(&papier[0].svg, &again[0].svg))
        .is_true();
    assert_that!(papier.iter().all(|t| t.svg.starts_with("<svg"))).is_true();
    assert_that!(papier.iter().any(|t| t.kind == oku_slides_core::kinds::essentiel::CONTENT)).is_true();

    s.apply(
        "Thème",
        vec![oku_slides_core::Op::SetTheme { theme: Some(common::ns("pro")), themes: BTreeMap::default() }],
        None,
    )
    .unwrap();
    let pro = s.layouts_view().unwrap().packs.remove(0).layouts;
    asserting("l'aperçu suit le thème").that(&papier[0].svg).is_not_equal_to(&pro[0].svg);

    // Chaque layout s'insère dans le deck.
    for (i, t) in pro.iter().enumerate() {
        let mut slide = t.slide.clone();
        slide.id = format!("nouvelle-{i}");
        assert_that!(s.apply("Nouvelle slide", vec![Op::AddSlide { index: None, defaults: false, slide }], None))
            .is_ok();
    }
}

#[test]
fn edit_builtin_pack_then_save_as_copy_creates_a_new_pack_and_close_restores_deck() {
    use serde_json::json;
    let dir = data_dir();
    let mut s = session();
    s.set_pack_dirs(dir.join("widgets"), dir.join("layouts")).unwrap();
    let before = s.deck().clone();

    s.edit_pack(oku_slides_session::ESSENTIEL_PACK).unwrap();
    let snap = s.snapshot();
    assert_that!(snap.editing.as_ref().unwrap().pack).is_equal_to(oku_slides_session::ESSENTIEL_PACK);
    assert_that!(snap.editing.as_ref().unwrap().builtin).is_true();
    let first = snap.slides[0].id.to_owned();
    let notes = json!([{ "op": "set_notes", "slide": first, "notes": "Ma grille" }]);
    s.apply("Noter", serde_json::from_value(notes).unwrap(), None).unwrap();
    // Comme copie : un nouveau pack, sur lequel l'édition continue.
    s.save_pack(true).unwrap();
    let mode = s.snapshot().editing.map(|e| e.mode.clone()).unwrap();
    assert_that!(mode.builtin).is_false();
    assert_that!(mode.pack).is_not_equal_to(oku_slides_session::ESSENTIEL_PACK);
    assert_that!(mode.name.ends_with("(copie)")).is_true();
    let version = mode.session_version.clone().unwrap();
    let file =
        dir.join("layouts").join(mode.pack.to_string()).join("versions").join(version.hex()).join("layouts.json");
    assert_that!(file.exists()).is_true();

    s.close_pack().unwrap();
    assert_that!(s.snapshot().editing).is_none();
    // Le deck retrouve ses slides, et passe à la copie.
    assert_that!(s.deck().slides).is_equal_to(&before.slides);
    assert_that!(s.deck().layouts.values().any(|p| p.id == mode.pack)).is_true();
    let view = s.layouts_view().unwrap();
    assert_that!(view.packs.iter().any(|p| p.name == mode.name)).is_true();
}

#[test]
fn edit_pack_switching_set_keeps_deck_and_history_aside() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.apply("Notes", vec![Op::SetNotes { slide: "s-fin".into(), notes: "à garder".into() }], None).unwrap();
    let before = s.deck().clone();

    s.edit_pack(oku_slides_session::BASE_PACK).unwrap();
    s.edit_pack(oku_slides_session::ESSENTIEL_PACK).unwrap();
    let snap = s.snapshot();
    assert_that!(snap.editing.as_ref().unwrap().pack).is_equal_to(oku_slides_session::ESSENTIEL_PACK);
    asserting("l'édition des modèles a son propre historique").that(&snap.can_undo).is_false();

    s.close_pack().unwrap();
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
    s.apply("Thème", vec![serde_json::from_value(json!({ "op": "set_theme", "theme": "papier" })).unwrap()], None)
        .unwrap();
    assert_that!(s.snapshot().appearance.chart_palette.first().map(String::as_str)).is_equal_to(Some("#3f6b5e"));
    s.apply("Thème", vec![serde_json::from_value(json!({ "op": "set_theme", "theme": "pro" })).unwrap()], None)
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
        .is_equal_to(json!({"slide": "s-1", "grid": false, "ghosts": true, "pile": null, "notes": null, "widgets": [], "widget": null, "code": false, "code_width": null, "folded": []}));
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
        .is_equal_to(json!({"slide": null, "grid": true, "ghosts": false, "pile": null, "notes": null, "widgets": [], "widget": null, "code": false, "code_width": null, "folded": []}));
}

#[test]
fn new_deck_forgets_the_remembered_slide_but_keeps_grid_ghosts_and_columns() {
    let mut s = session();
    s.handle(req(json!({"cmd": "ui", "slide": "s-1", "grid": false, "ghosts": true, "pile": 200, "notes": 520})))
        .unwrap();
    s.new_deck().unwrap();
    assert_that!(snapshot_json(&mut s)["ui"]).is_equal_to(
        json!({"slide": null, "grid": false, "ghosts": true, "pile": 200, "notes": 520, "widgets": [], "widget": null, "code": false, "code_width": null, "folded": []}),
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
fn resume_while_editing_pack_restores_mode_and_stashed_deck() {
    let data = data_dir();
    let mut a = resumed(&data);
    a.handle(set_theme("pro")).unwrap();
    a.handle(req(json!({"cmd": "edit_pack", "pack": oku_slides_session::ESSENTIEL_PACK}))).unwrap();
    drop(a);
    let mut b = resumed(&data);
    assert_that!(snapshot_json(&mut b)["editing"]["pack"]).is_equal_to(json!(oku_slides_session::ESSENTIEL_PACK));
    b.handle(req(json!({"cmd": "close_pack"}))).unwrap();
    let snap = snapshot_json(&mut b);
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
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
    other
        .apply(
            "Thème",
            vec![oku_slides_core::Op::SetTheme { theme: Some(common::ns("pro")), themes: BTreeMap::default() }],
            None,
        )
        .unwrap();
    other.save(Some(&file)).unwrap();
    let mut b = resumed(&data);
    let snap = snapshot_json(&mut b);
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
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
    assert_that!(info.editing_pack).is_false();
    assert_that!(info.document.path()).is_none();
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
    assert_that!(info.document.path()).is_none();
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
    assert_that!(info.document.path()).is_equal_to(Some(file.as_path()));
    assert_that!(info.pristine).is_false();
}

#[test]
fn tab_info_editing_pack_keeps_deck_title_and_reports_mode() {
    let mut s = session();
    let title = s.tab_info().title;
    s.handle(req(json!({"cmd": "edit_pack", "pack": oku_slides_session::ESSENTIEL_PACK}))).unwrap();
    let info = s.tab_info();
    assert_that!(info.editing_pack).is_true();
    assert_that!(info.title).is_equal_to(title);
    assert_that!(info.pristine).is_false();
}

/// Un deck propre mis de côté ne rend pas l'onglet non enregistré : seul le
/// deck (ou l'ensemble de modèles) réellement modifié doit compter.
#[test]
fn tab_info_editing_pack_with_a_clean_stashed_deck_is_not_dirty() {
    let mut s = session();
    s.handle(req(json!({"cmd": "edit_pack", "pack": oku_slides_session::ESSENTIEL_PACK}))).unwrap();
    assert_that!(s.tab_info().dirty).is_false();
}

/// Le deck mis de côté pendant l'édition des modèles garde son travail non
/// enregistré, que l'onglet doit continuer à signaler.
#[test]
fn tab_info_editing_pack_reports_dirty_from_the_stashed_deck() {
    let mut s = session();
    s.handle(set_theme("pro")).unwrap();
    s.handle(req(json!({"cmd": "edit_pack", "pack": oku_slides_session::ESSENTIEL_PACK}))).unwrap();
    assert_that!(s.tab_info().dirty).is_true();
}

#[test]
fn set_notice_is_shown_once_in_the_snapshot() {
    let mut s = session();
    s.set_notice("Attention".into());
    assert_that!(snapshot_json(&mut s)["notice"]).is_equal_to(json!("Attention"));
    assert_that!(snapshot_json(&mut s)["notice"]).is_equal_to(json!(null));
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
        .filter_map(std::result::Result::ok)
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

/// t-145 : en mode pack, un pack de reprise illisible ramène au deck,
/// sans rien perdre du deck mis de côté.
#[test]
fn resume_with_unreadable_pack_returns_to_the_deck_without_loss() {
    let data = data_dir();
    let mut a = resumed(&data);
    a.handle(set_theme("pro")).unwrap();
    a.handle(req(json!({"cmd": "edit_pack", "pack": oku_slides_session::ESSENTIEL_PACK}))).unwrap();
    drop(a);
    let pack = data.join("resume/pack/document.json");
    assert_that!(pack.exists()).is_true();
    std::fs::write(&pack, b"pas du json").unwrap();
    let mut b = resumed(&data);
    let snap = snapshot_json(&mut b);
    assert_that!(snap["editing"]).is_equal_to(json!(null));
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
    assert_that!(snap["dirty"]).is_equal_to(json!(true));
}

/// t-394 : reprise en mode pack, les sous-onglets de widgets reviennent.
#[test]
fn resume_in_pack_mode_returns_the_widget_tabs() {
    let data = data_dir();
    let mut a = resumed(&data);
    a.handle(req(json!({"cmd": "edit_pack", "pack": oku_slides_session::ESSENTIEL_PACK}))).unwrap();
    a.handle(req(json!({"cmd": "ui", "slide": null, "grid": true, "widgets": ["_commun"], "widget": "_commun"})))
        .unwrap();
    drop(a);
    let mut b = resumed(&data);
    let ui = snapshot_json(&mut b)["ui"].clone();
    assert_that!((ui["widgets"].clone(), ui["widget"].clone())).is_equal_to((json!(["_commun"]), json!("_commun")));
}

/// t-394 : modèles illisibles, la reprise revient au deck sans sous-onglets.
#[test]
fn resume_with_unreadable_pack_returns_no_widget_tabs() {
    let data = data_dir();
    let mut a = resumed(&data);
    a.handle(req(json!({"cmd": "edit_pack", "pack": oku_slides_session::ESSENTIEL_PACK}))).unwrap();
    a.handle(req(json!({"cmd": "ui", "slide": null, "grid": true, "widgets": ["_commun"], "widget": "_commun"})))
        .unwrap();
    drop(a);
    std::fs::write(data.join("resume/pack/document.json"), b"pas du json").unwrap();
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
        .filter_map(std::result::Result::ok)
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
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
    assert_that!(snap["dirty"]).is_equal_to(json!(false));
    assert_that!(b.tab_info().document.path())
        .is_equal_to(Some(oku_slides_platform::storage::resolve(&file).as_path()));
}

#[test]
fn appearance_carries_the_dark_interface_palette_of_papier_and_pro() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let papier = s.snapshot().appearance.clone();
    asserting("papier : table sombre")
        .that(&papier.variants["dark"].get("desk").map(String::as_str))
        .is_equal_to(Some("#1f1d1a"));
    s.apply(
        "Thème",
        vec![oku_slides_core::Op::SetTheme { theme: Some(common::ns("pro")), themes: BTreeMap::default() }],
        None,
    )
    .unwrap();
    let pro = s.snapshot().appearance;
    asserting("pro : table navy")
        .that(&pro.variants["dark"].get("desk").map(String::as_str))
        .is_equal_to(Some("#000c2e"));
}

#[test]
fn snapshot_carries_the_deck_language() {
    let mut s = session();
    s.new_deck().unwrap();
    assert_that!(snapshot_json(&mut s)["lang"]).is_equal_to(json!("fr"));
    s.open(&fixture_path()).unwrap();
    assert_that!(snapshot_json(&mut s)["lang"]).is_equal_to(json!(null));
}
