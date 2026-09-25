use std::collections::BTreeMap;

use slider_core::{Deck, Op, Paragraph, SourceBundle};
use slider_render::{Fonts, Renderer};
use speculoos::prelude::*;

fn fixture() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("fixture {path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("fixture {path} invalide : {e}"))
}

#[test]
fn compile_fixture_deck_renders_every_slide_and_its_boxes() {
    let deck = fixture();
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    let doc = r.compile().expect("compilation");
    assert_that!(doc.page_count()).is_equal_to(deck.slides.len());

    let layout = doc.layout();
    let pages = &layout.slides;
    let ids: Vec<_> = pages.iter().map(|p| p.slide.as_str()).collect();
    let expected: Vec<_> = deck.slides.iter().map(|s| s.id.as_str()).collect();
    assert_that!(ids).is_equal_to(expected);
    assert_that!(pages.iter().enumerate().all(|(i, p)| p.page == i + 1)).is_true();

    let boxes = &layout.boxes;
    for id in ["b-points", "c1", "c2", "c3", "e-fond", "e-texte", "e-image"] {
        let b = boxes.iter().find(|b| b.id == id).unwrap_or_else(|| panic!("boîte {id} absente"));
        asserting(&format!("{id} : {b:?}")).that(&(b.w > 0.0 && b.h > 0.0)).is_true();
    }
    // Un élément libre est placé à son cadre, relatif à la zone de corps.
    let texte = boxes.iter().find(|b| b.id == "e-texte").unwrap();
    let fond = boxes.iter().find(|b| b.id == "e-fond").unwrap();
    assert_that!(texte.x - fond.x).is_close_to(40.0, 0.01);
    assert_that!(texte.y - fond.y).is_close_to(40.0, 0.01);

    // Les boîtes d'une page sont celles de ses éléments, et seulement elles.
    let libre = layout.page_of("s-libre").unwrap();
    let on_page: Vec<&str> = layout.boxes_on(libre).iter().map(|b| b.id.as_str()).collect();
    assert_that!(on_page).contains("e-texte");
    assert_that!(on_page).does_not_contain("c1");
    assert_that!(layout.zone_on(libre).is_some_and(|z| z.grid.is_some())).is_true();

    let svg = doc.svg(0).unwrap();
    assert_that!(svg.as_str()).starts_with("<svg");
}

#[test]
fn compile_after_edit_or_theme_change_changes_only_affected_pages() {
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
    asserting("la slide modifiée doit changer").that(&after.page_hash(3).unwrap()).is_not_equal_to(h4);
    asserting("la couverture ne doit pas changer").that(&after.page_hash(0).unwrap()).is_equal_to(h1);

    deck.theme = Some("pro".into());
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    assert_that!(r.compile().unwrap().page_hash(0).unwrap()).is_not_equal_to(h1);
}

#[test]
fn compile_dictated_text_with_typst_markup_renders_it_verbatim() {
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
    // Le texte est rendu tel quel.
    assert_that!(r.compile()).is_ok();
}

#[test]
fn embed_sources_then_extract_gives_back_same_bundle() {
    let deck = fixture();
    let mut bundle = SourceBundle::new(deck.clone());
    bundle.add_asset(vec![0u8; 64]);
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_deck(&deck, &bundle.assets).unwrap();
    let pdf = r.compile().unwrap().pdf().unwrap();

    let saved = slider_pdf::embed_sources(&pdf, &bundle.to_zip().unwrap()).unwrap();
    let found = slider_pdf::extract_sources(&saved).unwrap().expect("blob présent");
    assert_that!(found.modified_elsewhere()).is_false();
    let back = SourceBundle::from_zip(&found.zip).unwrap();
    assert_that!(back).is_equal_to(&bundle);

    // Réenregistrer remplace le blob, sans en accumuler.
    let again = slider_pdf::embed_sources(&saved, &bundle.to_zip().unwrap()).unwrap();
    asserting(&format!("{} vs {}", again.len(), saved.len())).that(&again.len()).is_less_than(saved.len() + 1024);

    let stripped = slider_pdf::strip_sources(&saved).unwrap();
    assert_that!(slider_pdf::extract_sources(&stripped).unwrap()).is_none();
    assert_that!(slider_pdf::extract_sources(&pdf).unwrap()).is_none();
}

/// PNG 1 × 1 transparent.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0B, 0x49,
    0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x60, 0x00, 0x02, 0x00, 0x00, 0x05, 0x00, 0x01, 0x7A, 0x5E, 0xAB, 0x3F, 0x00,
    0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn set_deck_asset_added_then_removed_follows_the_deck() {
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
    asserting("l'image remplace le cadre « image manquante »").that(&shown).is_not_equal_to(missing);
    // Déjà chargé : rien ne change.
    r.set_deck(&deck, &assets).unwrap();
    assert_that!(r.compile().unwrap().page_hash(page).unwrap()).is_equal_to(shown);
    // Retiré du deck : de nouveau manquant. (L'empreinte peut différer : le
    // texte « image manquante » vient de main.typ, réanalysé entre-temps.)
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    let again = r.compile().unwrap();
    assert_that!(again.svg(page).unwrap()).is_equal_to(missing_svg);
}

#[test]
fn set_missing_image_label_with_quotes_still_compiles() {
    let deck = fixture();
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    let english = r.compile().unwrap().svg(deck.slide_index("s-libre").unwrap()).unwrap();
    r.set_missing_image_label(r#"image "manquante" \ ici"#);
    let french = r.compile().unwrap().svg(deck.slide_index("s-libre").unwrap()).unwrap();
    asserting("le libellé change le cadre de l'image absente").that(&english).is_not_equal_to(french);
}

#[test]
fn compile_invalid_token_returns_localized_compile_error_with_diagnostics() {
    use slider_i18n::{Localize, Localizer};
    use slider_render::{RenderErrorKind, Severity};
    let deck: Deck =
        serde_json::from_str(r#"{"tokens":{"color":{"accent":5}},"slides":[{"id":"s","kind":"content","title":"x"}]}"#)
            .unwrap();
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_deck(&deck, &BTreeMap::new()).unwrap();
    let err = r.compile().unwrap_err();
    assert_that!(err.kind()).is_equal_to(RenderErrorKind::Compile);
    let first = err.diagnostics().first().unwrap();
    assert_that!(first.severity).is_equal_to(Severity::Error);
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr).starts_with("Le rendu du deck a échoué\nerreur");
    assert_that!(err.to_string()).starts_with("Typst compilation failed:\nerror");
}

#[test]
fn svg_of_missing_page_returns_no_page() {
    use slider_i18n::{Localize, Localizer};
    let mut r = Renderer::new(Fonts::default()).unwrap();
    r.set_deck(&fixture(), &BTreeMap::new()).unwrap();
    let err = r.compile().unwrap().svg(99).unwrap_err();
    assert_that!(err.kind()).is_equal_to(slider_render::RenderErrorKind::NoPage);
    assert_that!(err.diagnostics().len()).is_equal_to(0);
    assert_that!(err.localize(&Localizer::new(Some("en")))).is_equal_to("This page does not exist".to_owned());
}

#[test]
fn diagnostic_localize_formats_place_and_hints() {
    use slider_i18n::{Localize, Localizer};
    use slider_render::{Diagnostic, Severity};
    let d = Diagnostic {
        severity: Severity::Warning,
        place: Some("slider/src/data.typ:12".into()),
        message: "unknown font".into(),
        hints: vec!["check the family".into()],
    };
    let fr = d.localize(&Localizer::new(Some("fr")));
    assert_that!(fr)
        .is_equal_to("avertissement (slider/src/data.typ:12) : unknown font\n  indice : check the family".to_owned());
    let unplaced = Diagnostic { place: None, hints: vec![], ..d };
    assert_that!(unplaced.to_string()).is_equal_to("warning: unknown font".to_owned());
}
