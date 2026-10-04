use oku_slides_a2ui::SlideRef;
use oku_slides_i18n::Localizer;
use speculoos::prelude::*;

use super::{NextStep, Report, fallback, receipt};

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn full() -> Report {
    Report {
        title: "Slide 4 : fond bleu nuit".into(),
        facts: vec!["titre en blanc".into(), "surtitre bleu clair".into()],
        slide: Some("s-4".into()),
        swatch: Some("#00154e".into()),
        why: Some("Le modèle `content` n'a pas de fond : rectangle pleine page ajouté.".into()),
        actions: vec![NextStep { label: "Autre couleur".into(), prompt: Some("Essaie un vert sapin".into()) }],
    }
}

#[test]
fn receipt_full_matches_snapshot() {
    let slide = SlideRef { id: "s-4".into(), hash: "0123456789abcdef0123456789abcdef".into() };
    let s = receipt("report-1", &full(), Some(&slide), &fr());
    assert_that!(s.live).is_false();
    insta::assert_json_snapshot!(s);
}

#[test]
fn receipt_minimal_has_no_disclosure_nor_actions() {
    let spec = Report {
        title: "Fait".into(),
        facts: vec!["ok".into()],
        slide: None,
        swatch: None,
        why: None,
        actions: vec![],
    };
    let json = serde_json::to_string(&receipt("report-1", &spec, None, &fr())).unwrap();
    assert_that!(json).does_not_contain("Disclosure");
    assert_that!(json).does_not_contain("suggest");
}

#[test]
fn receipt_action_sends_its_prompt_or_label() {
    let mut spec = full();
    spec.actions.push(NextStep { label: "Plus grand".into(), prompt: None });
    let json = serde_json::to_string(&receipt("report-1", &spec, None, &fr())).unwrap();
    assert_that!(json).contains("Essaie un vert sapin");
    assert_that!(json.matches("Plus grand").count()).is_equal_to(2);
}

#[test]
fn receipt_fallback_takes_first_sentence_as_title() {
    let s = fallback("report-1", "La slide 4 a un fond bleu nuit. Le titre est blanc.", &fr()).unwrap();
    let json = serde_json::to_string(&s).unwrap();
    assert_that!(json).contains("\"title\":\"La slide 4 a un fond bleu nuit.\"");
    assert_that!(json).contains("Le titre est blanc.");
}

#[test]
fn receipt_fallback_does_not_split_on_a_decimal_point() {
    let s = fallback("report-1", "Le titre passe à 4.5 cm. Le reste suit.", &fr()).unwrap();
    assert_that!(serde_json::to_string(&s).unwrap()).contains("\"title\":\"Le titre passe à 4.5 cm.\"");
}

#[test]
fn receipt_fallback_without_terminator_takes_the_whole_line() {
    let s = fallback("report-1", "Fond posé\nPuis le titre", &fr()).unwrap();
    let json = serde_json::to_string(&s).unwrap();
    assert_that!(json).contains("\"title\":\"Fond posé\"");
    assert_that!(json).contains("Puis le titre");
}

#[test]
fn receipt_fallback_long_first_sentence_keeps_it_whole_in_why() {
    let long = format!("{}.", "mot ".repeat(60).trim());
    let s = fallback("report-1", &long, &fr()).unwrap();
    let json = serde_json::to_string(&s).unwrap();
    assert_that!(json).contains("…");
    assert_that!(json).contains(&long);
}

#[test]
fn receipt_fallback_why_is_capped() {
    let long = format!("Titre. {}", "x".repeat(crate::agent::WHY_MAX * 2));
    let s = fallback("report-1", &long, &fr()).unwrap();
    let json = serde_json::to_string(&s).unwrap();
    assert_that!(json.len()).is_less_than(crate::agent::WHY_MAX + 2000);
}

#[test]
fn receipt_fallback_without_narration_is_none() {
    assert_that!(fallback("report-1", "   ", &fr())).is_none();
}

/// t-458 : une slide sans page rendue (son rendu a échoué, le dernier rendu installé ne la connaît
/// pas) donne un reçu sans référence de slide, et non une erreur (avant t-450 : l'échec du rendu
/// de la miniature faisait échouer `report`).
#[test]
fn report_surface_on_a_slide_without_a_rendered_page_has_no_slide_reference() {
    let mut s = crate::lib_tests_support::session_with_fixture();
    let mut deck = s.deck().clone();
    let mut slide = deck.slides[0].clone();
    slide.id = "s-sans-page".into();
    deck.slides.push(slide);
    // Les sources changent sans rendu : le rendu installé reste celui d'avant, sans cette slide.
    s.doc.sources.apply(deck, "Sans rendu", None).unwrap();
    let spec = Report { slide: Some("s-sans-page".into()), ..full() };
    let r = s.report_surface("report-1", &spec).unwrap();
    for surface in [&r.surface, r.plain.as_ref().unwrap()] {
        let json = serde_json::to_value(surface).unwrap();
        let messages = json["messages"].as_array().unwrap();
        let mut components = messages.iter().filter_map(|m| m["updateComponents"]["components"].as_array()).flatten();
        let card = components.find(|c| c["id"] == "card").unwrap();
        assert_that!(card["component"].as_str()).is_equal_to(Some("Receipt"));
        assert_that!(card.get("slide")).is_none();
        assert_that!(card["title"].as_str()).is_equal_to(Some("Slide 4 : fond bleu nuit"));
    }
}
