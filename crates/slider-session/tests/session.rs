use serde_json::json;
use slider_core::{Deck, Frame, Op, Paragraph};
use slider_session::{Request, Session, paragraphs_to_text, text_to_paragraphs};

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

#[test]
fn ouvrir_modifier_annuler_enregistrer_rouvrir() {
    let mut s = Session::new(Vec::new());
    let snap = s.open(&fixture_path()).unwrap();
    assert_eq!(snap.slides.len(), 8);
    assert!(snap.path.is_none(), "un JSON n'est pas un fichier Slider : pas de chemin");

    let view = s.slide("s-libre").unwrap();
    let texte = view.targets.iter().find(|t| t.id == "e-texte").unwrap();
    assert!(texte.free);
    assert_eq!(texte.text.as_deref(), Some("Un bloc posé à 40 pt du coin"));
    assert!(view.targets.iter().any(|t| t.id == "@title" && t.target == "field"));
    let before_hash = view.hash.clone();

    let snap = s
        .apply(
            "Déplacer",
            vec![Op::SetFrame {
                slide: "s-libre".into(),
                element: "e-texte".into(),
                frame: Some(Frame { x: 100.0, y: 60.0, w: 720.0, h: 200.0 }),
            }],
        )
        .unwrap();
    assert!(snap.dirty && snap.can_undo);
    assert_eq!(snap.undo_label.as_deref(), Some("Déplacer"));
    let moved = s.slide("s-libre").unwrap();
    assert_ne!(moved.hash, before_hash);
    let t2 = moved.targets.iter().find(|t| t.id == "e-texte").unwrap();
    assert!((t2.x - texte.x - 60.0).abs() < 0.01);

    s.undo().unwrap();
    assert_eq!(s.slide("s-libre").unwrap().hash, before_hash);
    s.redo().unwrap();

    let dir = std::env::temp_dir().join(format!("slider-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let snap = s.save(Some(&dir.join("essai.pdf"))).unwrap();
    let path = snap.path.unwrap();
    assert!(path.to_string_lossy().ends_with("essai.slider.pdf"));
    assert!(!snap.dirty);

    let deck: Deck = s.deck().clone();
    let mut s2 = Session::new(Vec::new());
    s2.open(&path).unwrap();
    assert_eq!(s2.deck(), &deck);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn une_transaction_invalide_ne_change_rien() {
    let mut s = Session::new(Vec::new());
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
    assert!(err.to_string().contains("opération 1"));
    assert_eq!(s.deck(), &before);
}

#[test]
fn api_json() {
    let mut s = Session::new(Vec::new());
    let req: Request = serde_json::from_value(json!({"cmd": "snapshot"})).unwrap();
    let v = s.handle(req).unwrap();
    assert_eq!(v["slides"].as_array().unwrap().len(), 2);
    let req: Request = serde_json::from_value(json!({
        "cmd": "apply", "label": "Titre",
        "ops": [{"op": "set_slide_field", "slide": "s-1", "field": "title", "value": "Nouveau"}]
    }))
    .unwrap();
    let v = s.handle(req).unwrap();
    assert_eq!(v["slides"][1]["title"], "Nouveau");
    let svg = s.handle(serde_json::from_value(json!({"cmd": "svg", "id": "s-1"})).unwrap()).unwrap();
    assert!(svg.as_str().unwrap().starts_with("<svg"));
}

#[test]
fn texte_et_paragraphes() {
    let paras = vec![
        Paragraph { runs: vec![slider_core::Run { t: "Gras".into(), bold: true, ..Default::default() }], list: None },
        Paragraph { runs: vec![slider_core::Run::plain("Point")], list: Some(slider_core::ListKind::Bullet) },
    ];
    let text = paragraphs_to_text(&paras);
    assert_eq!(text, "Gras\n- Point");
    let back = text_to_paragraphs("Gras\n- Point\n1. Un", &paras);
    assert_eq!(back[0], paras[0], "un paragraphe inchangé garde sa mise en forme");
    assert_eq!(back[1], paras[1]);
    assert_eq!(back[2].list, Some(slider_core::ListKind::Number));
}

#[test]
fn l_apparence_suit_le_theme() {
    let mut s = Session::new(Vec::new());
    s.open(&fixture_path()).unwrap();
    let papier = s.snapshot().appearance;
    assert_eq!(papier.accent, "#3f6b5e");
    assert_eq!(papier.bg, "#fffdf9");
    assert_eq!(papier.font_display.first().map(String::as_str), Some("Newsreader"));

    let snap = s.apply("Thème", vec![Op::SetTheme { theme: Some("pro".into()) }]).unwrap();
    let pro = snap.appearance;
    assert_eq!(pro.accent, "#00bef4");
    assert_eq!(pro.accent_text, "#0072b0", "pro a une couleur d'accent pour le texte");
    assert_eq!(pro.font_body, vec!["Inter".to_owned()]);

    // Une surcharge du deck l'emporte sur le thème.
    let snap = s
        .apply("Accent", vec![Op::SetToken { path: vec!["color".into(), "accent".into()], value: json!("#123456") }])
        .unwrap();
    assert_eq!(snap.appearance.accent, "#123456");
}
