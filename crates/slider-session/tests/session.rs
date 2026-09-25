use serde_json::json;
use slider_core::{Deck, Frame, Op, Paragraph};
use slider_session::{Fonts, Platform, Request, Session, Settings, TargetKind, paragraphs_to_text};

/// Une session isolée : son propre dossier de données, vide.
fn session() -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    let data = std::env::temp_dir().join(format!("echo-session-{}-{}", std::process::id(), unique()));
    settings.paths.data = Some(data);
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"));
    Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session impossible : {}", e.one_line()))
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
fn ouvrir_modifier_annuler_enregistrer_rouvrir() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let snap = s.snapshot();
    assert_eq!(snap.slides.len(), 8);
    assert!(snap.path.is_none(), "un JSON n'est pas un fichier Écho : pas de chemin");

    let view = s.slide("s-libre").unwrap();
    let texte = view.targets.iter().find(|t| t.id == "e-texte").unwrap();
    assert!(texte.free);
    assert_eq!(texte.text.as_deref(), Some("Un bloc posé à 40 pt du coin"));
    assert!(view.targets.iter().any(|t| t.id == "@title" && t.kind == TargetKind::Field("title")));
    assert_eq!(texte.kind, TargetKind::Element("text"));
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
    assert!(snap.dirty && snap.can_undo);
    assert_eq!(snap.undo_label, Some("Déplacer"));
    let moved = s.slide("s-libre").unwrap();
    assert_ne!(moved.hash, before_hash);
    let t2 = moved.targets.iter().find(|t| t.id == "e-texte").unwrap();
    assert!((t2.x - texte_x - 60.0).abs() < 0.01);

    s.undo().unwrap();
    assert_eq!(s.slide("s-libre").unwrap().hash, before_hash);
    s.redo().unwrap();

    let dir = std::env::temp_dir().join(format!("slider-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    s.save(Some(&dir.join("essai.pdf"))).unwrap();
    let snap = s.snapshot();
    let path = snap.path.unwrap().to_owned();
    assert!(path.to_string_lossy().ends_with("essai.echo.pdf"));
    assert!(!snap.dirty);

    let deck: Deck = s.deck().clone();
    let mut s2 = session();
    s2.open(&path).unwrap();
    assert_eq!(s2.deck(), &deck);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn une_transaction_invalide_ne_change_rien() {
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
    assert_eq!(err.transaction().map(slider_core::TransactionError::index), Some(1));
    assert_eq!(s.deck(), &before);
}

#[test]
fn api_json() {
    let mut s = session();
    let req: Request = serde_json::from_value(json!({"cmd": "snapshot"})).unwrap();
    let v: serde_json::Value = serde_json::from_str(&s.handle(req).unwrap()).unwrap();
    assert_eq!(v["slides"].as_array().unwrap().len(), 2);
    let req: Request = serde_json::from_value(json!({
        "cmd": "apply", "label": "Titre",
        "ops": [{"op": "set_slide_field", "slide": "s-1", "field": "title", "value": "Nouveau"}]
    }))
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&s.handle(req).unwrap()).unwrap();
    assert_eq!(v["slides"][1]["title"], "Nouveau");
    assert_eq!(v["slides"][1]["hash"].as_str().map(str::len), Some(32), "empreinte en hex");
    let svg = s.handle(serde_json::from_value(json!({"cmd": "svg", "id": "s-1"})).unwrap()).unwrap();
    let svg: String = serde_json::from_str(&svg).unwrap();
    assert!(svg.starts_with("<svg"));
    // Une cible garde sa forme JSON : { "target": "field", "kind": "title", … }.
    let view: serde_json::Value =
        serde_json::from_str(&s.handle(serde_json::from_value(json!({"cmd": "slide", "id": "s-1"})).unwrap()).unwrap())
            .unwrap();
    let title = view["targets"].as_array().unwrap().iter().find(|t| t["id"] == "@title").unwrap();
    assert_eq!((&title["target"], &title["kind"]), (&json!("field"), &json!("title")));
}

#[test]
fn texte_et_paragraphes() {
    let paras = vec![
        Paragraph { runs: vec![slider_core::Run { t: "Gras".into(), bold: true, ..Default::default() }], list: None },
        Paragraph { runs: vec![slider_core::Run::plain("Point")], list: Some(slider_core::ListKind::Bullet) },
    ];
    assert_eq!(paragraphs_to_text(&paras), "Gras\n- Point");
}

#[test]
fn l_apparence_suit_le_theme() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let papier = s.snapshot().appearance;
    assert_eq!(papier.scheme, "light");
    assert_eq!(papier.slide["accent"], "#3f6b5e");
    assert_eq!(papier.app["desk"], "#ece8e0");
    assert_eq!(papier.app["ink"], "#2e2b26", "les références par nom sont résolues");
    assert_eq!(papier.font_display.first().map(String::as_str), Some("Newsreader"));

    s.apply("Thème", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    let pro = s.snapshot().appearance;
    assert_eq!(pro.scheme, "light");
    assert_eq!(pro.app["desk"], "#e1e5ea");
    assert_eq!(pro.app["ink"], "#00154e");
    assert_eq!(pro.app["accent-text"], "#0072b0");
    assert_eq!(pro.font_body, vec!["Inter".to_owned()]);
    assert_eq!(pro.metrics["radius-lg"], 16.0);
    assert_eq!(pro.metrics["elevation"], 1.5);

    // Sans thème, l'interface dérive ses couleurs : aucune couleur d'app imposée.
    s.apply("Sans thème", vec![Op::SetTheme { theme: None }]).unwrap();
    let plain = s.snapshot().appearance;
    assert!(plain.app.is_empty());
    assert_eq!(plain.metrics["radius-md"], 12.0, "les formes par défaut restent définies");

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
    assert_eq!(snap.appearance.slide["accent"], "#123456");
    assert_eq!(snap.appearance.app["accent"], "#123456");
    assert_eq!(snap.appearance.app["desk"], "#222222");
}

#[test]
fn les_modeles_se_rendent_dans_le_theme_du_deck() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let papier = s.templates_view(None).unwrap().templates;
    assert!(papier.len() >= 8);
    let again = s.templates_view(None).unwrap().templates;
    assert!(std::sync::Arc::ptr_eq(&papier, &again), "rien n'a changé : les aperçus viennent du cache");
    assert!(papier.iter().all(|t| t.svg.starts_with("<svg")));
    assert!(papier.iter().any(|t| t.template.id == "cartes"));

    s.apply("Thème", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    let pro = s.templates_view(None).unwrap().templates;
    assert_ne!(papier[0].svg, pro[0].svg, "l'aperçu suit le thème");

    // Chaque modèle s'insère dans le deck.
    for (i, t) in pro.iter().enumerate() {
        let mut slide = t.template.slide.clone();
        slide.id = format!("nouvelle-{i}");
        s.apply("Nouvelle slide", vec![Op::AddSlide { index: None, slide }]).unwrap();
    }
}

#[test]
fn edition_des_modeles_puis_retour_au_deck() {
    use serde_json::json;
    let dir = std::env::temp_dir().join(format!("echo-session-tpl-{}", std::process::id()));
    let mut s = session();
    s.set_templates_dir(&dir);
    let before = s.deck().clone();

    s.edit_templates("grilles").unwrap();
    let snap = s.snapshot();
    assert_eq!(snap.editing.unwrap().set, "grilles");
    assert!(snap.editing.unwrap().builtin);
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
    assert!(!mode.builtin);
    assert_eq!(mode.set, "grilles-copie");
    assert!(dir.join("grilles-copie.json").exists());

    s.close_templates().unwrap();
    assert!(s.snapshot().editing.is_none());
    assert_eq!(s.deck(), &before);
    let view = s.templates_view(Some("grilles-copie".into())).unwrap();
    assert_eq!(view.templates[0].template.name, "Ma grille");
    assert!(view.sets.iter().any(|x| x.id == "grilles-copie" && !x.builtin));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn changer_d_ensemble_en_cours_d_edition_garde_le_deck_de_cote() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.apply("Notes", vec![Op::SetNotes { slide: "s-fin".into(), notes: "à garder".into() }]).unwrap();
    let before = s.deck().clone();

    s.edit_templates("grilles").unwrap();
    s.edit_templates("essentiel").unwrap();
    let snap = s.snapshot();
    assert_eq!(snap.editing.unwrap().set, "essentiel");
    assert!(!snap.can_undo, "l'édition des modèles a son propre historique");

    s.close_templates().unwrap();
    assert_eq!(s.deck(), &before);
    let snap = s.snapshot();
    assert!(snap.dirty && snap.can_undo, "le deck retrouve son état et son historique");
}

#[test]
fn errors_localize_in_the_configured_language() {
    use slider_session::{Localize, Localizer};
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let fr = Localizer::new(Some("fr"));
    assert_eq!(s.slide("absente").unwrap_err().localize(&fr), "Slide introuvable : absente");
    let refused = s.apply("Mauvais", vec![Op::RemoveElement { slide: "s-fin".into(), element: "absent".into() }]);
    assert_eq!(refused.unwrap_err().localize(&fr), "Opération 0 refusée : Élément introuvable : absent");
    let en = Localizer::new(Some("en"));
    assert_eq!(s.slide("absente").unwrap_err().localize(&en), "Slide not found: absente");
}
