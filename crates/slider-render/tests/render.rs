use std::collections::BTreeMap;

use slider_core::{Deck, Op, Paragraph, SourceBundle};
use slider_render::Renderer;

fn fixture() -> Deck {
    let s = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json")).unwrap();
    Deck::from_json(&s).unwrap()
}

#[test]
fn rend_chaque_slide_et_ses_boites() {
    let deck = fixture();
    let mut r = Renderer::new(Vec::new());
    r.set_deck(&deck, &BTreeMap::new());
    let doc = r.compile().expect("compilation");
    assert_eq!(doc.page_count(), deck.slides.len());

    let pages = doc.slide_pages();
    let ids: Vec<_> = pages.iter().map(|p| p.slide.as_str()).collect();
    let expected: Vec<_> = deck.slides.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, expected);
    assert!(pages.iter().enumerate().all(|(i, p)| p.page == i + 1));

    let boxes = doc.element_boxes();
    for id in ["b-points", "c1", "c2", "c3", "e-fond", "e-texte", "e-image"] {
        let b = boxes.iter().find(|b| b.id == id).unwrap_or_else(|| panic!("boîte {id} absente"));
        assert!(b.w > 0.0 && b.h > 0.0, "{id} : {b:?}");
    }
    // Un élément libre est placé à son cadre, relatif à la zone de corps.
    let texte = boxes.iter().find(|b| b.id == "e-texte").unwrap();
    let fond = boxes.iter().find(|b| b.id == "e-fond").unwrap();
    assert!((texte.x - fond.x - 40.0).abs() < 0.01 && (texte.y - fond.y - 40.0).abs() < 0.01);

    let svg = doc.svg(0).unwrap();
    assert!(svg.starts_with("<svg"));
}

#[test]
fn les_themes_et_une_modification_changent_le_rendu() {
    let mut deck = fixture();
    let mut r = Renderer::new(Vec::new());
    r.set_deck(&deck, &BTreeMap::new());
    let before = r.compile().unwrap();
    let h4 = before.page_hash(3).unwrap();
    let h1 = before.page_hash(0).unwrap();

    Op::SetText { slide: "s-constat".into(), element: "b-points".into(), text: vec![Paragraph::plain("Autre")] }
        .apply(&mut deck)
        .unwrap();
    r.set_deck(&deck, &BTreeMap::new());
    let after = r.compile().unwrap();
    assert_ne!(after.page_hash(3).unwrap(), h4, "la slide modifiée doit changer");
    assert_eq!(after.page_hash(0).unwrap(), h1, "la couverture ne doit pas changer");

    deck.theme = Some("pro".into());
    r.set_deck(&deck, &BTreeMap::new());
    assert_ne!(r.compile().unwrap().page_hash(0).unwrap(), h1);
}

#[test]
fn un_texte_dicte_ne_peut_pas_injecter_du_typst() {
    let mut deck = fixture();
    Op::SetText {
        slide: "s-constat".into(),
        element: "b-points".into(),
        text: vec![Paragraph::plain("#panic(\"x\") ] ) } #read(\"/etc/passwd\") $x$ <lbl> @ref")],
    }
    .apply(&mut deck)
    .unwrap();
    let mut r = Renderer::new(Vec::new());
    r.set_deck(&deck, &BTreeMap::new());
    r.compile().expect("le texte est rendu tel quel");
}

#[test]
fn enregistrer_puis_rouvrir_redonne_le_meme_deck() {
    let deck = fixture();
    let mut bundle = SourceBundle::new(deck.clone());
    bundle.add_asset(vec![0u8; 64]);
    let mut r = Renderer::new(Vec::new());
    r.set_deck(&deck, &bundle.assets);
    let pdf = r.compile().unwrap().pdf().unwrap();

    let saved = slider_pdf::embed_sources(&pdf, &bundle.to_zip().unwrap()).unwrap();
    let found = slider_pdf::extract_sources(&saved).unwrap().expect("blob présent");
    assert!(!found.modified_elsewhere());
    let back = SourceBundle::from_zip(&found.zip).unwrap();
    assert_eq!(back, bundle);

    // Réenregistrer remplace le blob, sans en accumuler.
    let again = slider_pdf::embed_sources(&saved, &bundle.to_zip().unwrap()).unwrap();
    assert!(again.len() < saved.len() + 1024, "{} vs {}", again.len(), saved.len());

    let stripped = slider_pdf::strip_sources(&saved).unwrap();
    assert!(slider_pdf::extract_sources(&stripped).unwrap().is_none());
    assert!(slider_pdf::extract_sources(&pdf).unwrap().is_none());
}
