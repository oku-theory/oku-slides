use serde_json::json;
use slider_core::*;

fn fixture() -> Deck {
    let s = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json")).unwrap();
    Deck::from_json(&s).unwrap()
}

fn para(t: &str) -> Vec<Paragraph> {
    vec![Paragraph::plain(t)]
}

fn sample_ops() -> Vec<Op> {
    vec![
        Op::SetText { slide: "s-constat".into(), element: "b-points".into(), text: para("Nouveau texte") },
        Op::SetText { slide: "s-priorites".into(), element: "c2".into(), text: para("Carte réécrite") },
        Op::SetStyle {
            slide: "s-libre".into(),
            element: "e-texte".into(),
            style: Style { color: Some("accent".into()), size: Some(TokenValue::Number(48.0)), ..Default::default() },
        },
        Op::SetFrame { slide: "s-libre".into(), element: "e-image".into(), frame: Some(Frame { x: 900.0, y: 20.0, w: 700.0, h: 400.0 }) },
        Op::AddElement {
            slide: "s-constat".into(),
            zone: Zone::Blocks,
            parent: None,
            index: Some(0),
            element: Element::text("b-intro", para("Introduction")),
        },
        Op::RemoveElement { slide: "s-priorites".into(), element: "c1".into() },
        Op::SetSlideField { slide: "s-constat".into(), field: SlideField::Title, value: json!("Titre dicté") },
        Op::SetSlideField { slide: "s-constat".into(), field: SlideField::Layout, value: json!("columns") },
        Op::SetSlideField { slide: "s-ch1".into(), field: SlideField::Subtitle, value: json!(null) },
        Op::AddSlide { index: Some(3), slide: Slide::new("s-nouvelle", SlideKind::Content) },
        Op::MoveSlide { slide: "s-fin".into(), index: 1 },
        Op::RemoveSlide { slide: "s-ch2".into() },
        Op::SetNotes { slide: "s-title".into(), notes: "Bonjour à tous".into() },
        Op::SetTheme { theme: Some("pro".into()) },
        Op::SetToken { path: vec!["color".into(), "accent".into()], value: json!("#2f5fb3") },
        Op::SetToken { path: vec!["component".into(), "card".into(), "height".into()], value: json!("fill") },
        Op::SetParam { key: "author".into(), value: json!("Quelqu'un") },
        Op::SetParam { key: "numbering".into(), value: json!("{n}") },
        Op::SetVar { key: "client".into(), value: json!("Globex") },
        Op::SetVar { key: "confidential".into(), value: json!(null) },
    ]
}

#[test]
fn chaque_op_suivie_de_son_inverse_redonne_le_deck() {
    for op in sample_ops() {
        let mut deck = fixture();
        let before = deck.clone();
        let inv = op.apply(&mut deck).unwrap_or_else(|e| panic!("{op:?} : {e}"));
        assert_ne!(deck, before, "l'op {op:?} n'a rien changé");
        inv.apply(&mut deck).unwrap();
        assert_eq!(deck, before, "inverse incorrect pour {op:?}");
    }
}

#[test]
fn transaction_atomique_et_annulation() {
    let mut deck = fixture();
    let before = deck.clone();
    let mut h = History::default();
    h.commit(&mut deck, "Tout", sample_ops()).unwrap();
    assert_ne!(deck, before);
    let after = deck.clone();
    assert_eq!(h.undo(&mut deck).unwrap().as_deref(), Some("Tout"));
    assert_eq!(deck, before);
    assert_eq!(h.redo(&mut deck).unwrap().as_deref(), Some("Tout"));
    assert_eq!(deck, after);
}

#[test]
fn une_erreur_annule_toute_la_transaction() {
    let mut deck = fixture();
    let before = deck.clone();
    let mut ops = sample_ops();
    ops.push(Op::SetText { slide: "s-constat".into(), element: "inexistant".into(), text: vec![] });
    let mut h = History::default();
    let (i, err) = h.commit(&mut deck, "Échec", ops).unwrap_err();
    assert_eq!(i, sample_ops().len());
    assert!(matches!(err, OpError::ElementNotFound { .. }));
    assert_eq!(deck, before);
    assert!(!h.can_undo());
}

#[test]
fn identifiants_en_double_refuses() {
    let mut deck = fixture();
    let err = Op::AddElement {
        slide: "s-priorites".into(),
        zone: Zone::Blocks,
        parent: None,
        index: None,
        element: Element::text("c1", vec![]),
    }
    .apply(&mut deck)
    .unwrap_err();
    assert_eq!(err, OpError::DuplicateId("c1".into()));
}

#[test]
fn le_json_fait_l_aller_retour() {
    let deck = fixture();
    let again = Deck::from_json(&deck.to_json()).unwrap();
    assert_eq!(deck, again);
    assert!(validate(&deck, None).is_empty(), "{:?}", validate(&deck, None));
}

#[test]
fn zip_des_sources_aller_retour() {
    let mut b = SourceBundle::new(fixture());
    let id = b.add_asset(vec![1, 2, 3, 4]);
    assert!(id.starts_with("sha256:"));
    b.fonts.insert("Inter.ttf".into(), vec![9; 16]);
    b.history = Some("{}".into());
    let zip = b.to_zip().unwrap();
    let back = SourceBundle::from_zip(&zip).unwrap();
    assert_eq!(b, back);
}

#[test]
fn le_schema_des_ops_se_genere() {
    let schema = ops_schema();
    let s = schema.to_string();
    assert!(s.contains("set_text") && s.contains("set_token"));
}
