use std::collections::BTreeMap;

use slider_core::{Deck, Op, Paragraph, SourceBundle};
use slider_render::{Fonts, Renderer};

fn fixture() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("fixture {path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("fixture {path} invalide : {e}"))
}

#[test]
fn rend_chaque_slide_et_ses_boites() {
    let deck = fixture();
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    let doc = r.compile().expect("compilation");
    assert_eq!(doc.page_count(), deck.slides.len());

    let layout = doc.layout();
    let pages = &layout.slides;
    let ids: Vec<_> = pages.iter().map(|p| p.slide.as_str()).collect();
    let expected: Vec<_> = deck.slides.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, expected);
    assert!(pages.iter().enumerate().all(|(i, p)| p.page == i + 1));

    let boxes = &layout.boxes;
    for id in ["b-points", "c1", "c2", "c3", "e-fond", "e-texte", "e-image"] {
        let b = boxes.iter().find(|b| b.id == id).unwrap_or_else(|| panic!("boîte {id} absente"));
        assert!(b.w > 0.0 && b.h > 0.0, "{id} : {b:?}");
    }
    // Un élément libre est placé à son cadre, relatif à la zone de corps.
    let texte = boxes.iter().find(|b| b.id == "e-texte").unwrap();
    let fond = boxes.iter().find(|b| b.id == "e-fond").unwrap();
    assert!((texte.x - fond.x - 40.0).abs() < 0.01 && (texte.y - fond.y - 40.0).abs() < 0.01);

    // Les boîtes d'une page sont celles de ses éléments, et seulement elles.
    let libre = layout.page_of("s-libre").unwrap();
    let on_page: Vec<&str> = layout.boxes_on(libre).iter().map(|b| b.id.as_str()).collect();
    assert!(on_page.contains(&"e-texte") && !on_page.contains(&"c1"), "{on_page:?}");
    assert!(layout.zone_on(libre).is_some_and(|z| z.grid.is_some()));

    let svg = doc.svg(0).unwrap();
    assert!(svg.starts_with("<svg"));
}

#[test]
fn les_themes_et_une_modification_changent_le_rendu() {
    let mut deck = fixture();
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    let before = r.compile().unwrap();
    let h4 = before.page_hash(3).unwrap();
    let h1 = before.page_hash(0).unwrap();

    Op::SetText { slide: "s-constat".into(), element: "b-points".into(), text: vec![Paragraph::plain("Autre")] }
        .apply(&mut deck)
        .unwrap();
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    let after = r.compile().unwrap();
    assert_ne!(after.page_hash(3).unwrap(), h4, "la slide modifiée doit changer");
    assert_eq!(after.page_hash(0).unwrap(), h1, "la couverture ne doit pas changer");

    deck.theme = Some("pro".into());
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
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
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    r.compile().expect("le texte est rendu tel quel");
}

#[test]
fn enregistrer_puis_rouvrir_redonne_le_meme_deck() {
    let deck = fixture();
    let mut bundle = SourceBundle::new(deck.clone());
    bundle.add_asset(vec![0u8; 64]);
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_deck(&deck, &bundle.assets).unwrap();
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

/// PNG 1 × 1 transparent.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0B, 0x49,
    0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x60, 0x00, 0x02, 0x00, 0x00, 0x05, 0x00, 0x01, 0x7A, 0x5E, 0xAB, 0x3F, 0x00,
    0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn un_asset_ajoute_puis_retire_suit_le_deck() {
    let deck = fixture();
    let page = deck.slide_index("s-libre").unwrap();
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    let first = r.compile().unwrap();
    let missing = first.page_hash(page).unwrap();
    let missing_svg = first.svg(page).unwrap();

    let assets = BTreeMap::from([("sha256:0000".to_owned(), PNG.to_vec())]);
    r.set_deck(&deck, &assets).unwrap();
    let shown = r.compile().unwrap().page_hash(page).unwrap();
    assert_ne!(shown, missing, "l'image remplace le cadre « image manquante »");
    // Déjà chargé : rien ne change.
    r.set_deck(&deck, &assets).unwrap();
    assert_eq!(r.compile().unwrap().page_hash(page).unwrap(), shown);
    // Retiré du deck : de nouveau manquant. (L'empreinte peut différer : le
    // texte « image manquante » vient de main.typ, réanalysé entre-temps.)
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    let again = r.compile().unwrap();
    assert_eq!(again.svg(page).unwrap(), missing_svg);
}

#[test]
fn set_missing_image_label_with_quotes_still_compiles() {
    let deck = fixture();
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    let english = r.compile().unwrap().svg(deck.slide_index("s-libre").unwrap()).unwrap();
    r.set_missing_image_label(r#"image "manquante" \ ici"#);
    let french = r.compile().unwrap().svg(deck.slide_index("s-libre").unwrap()).unwrap();
    assert_ne!(english, french, "le libellé change le cadre de l'image absente");
}
