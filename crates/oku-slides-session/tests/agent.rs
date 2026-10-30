//! Agent forms and receipts: PNG previews, tool receipts, proposed rewrites.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
use std::collections::BTreeMap;

use oku_slides_core::{Op, Paragraph, Run, Target};
use oku_slides_i18n::{Localize, Localizer, english};
use oku_slides_session::SessionErrorKind;
use oku_slides_session::agent::{
    AgentFormReason, AgentUi, Anchor, AskRewrite, FieldKind, ParamField, Place, Report, ShowParams, ShowVariants,
    Surface, Variant,
};
use serde_json::json;
use speculoos::prelude::*;

#[allow(dead_code, reason = "aides partagées : chaque binaire de test n'en utilise qu'une partie")]
mod common;
use common::{fixture_path, session};

fn rewrite(slide: &str, owner: &str, field: Option<&str>, proposals: &[&str]) -> AskRewrite {
    AskRewrite {
        slide: slide.into(),
        owner: owner.into(),
        field: field.map(str::to_owned),
        proposals: proposals.iter().map(|p| (*p).to_owned()).collect(),
    }
}

fn anchor(slide: &str, owner: &str, field: Option<&str>) -> Anchor {
    Anchor { slide: slide.into(), owner: owner.into(), field: field.map(str::to_owned) }
}

#[test]
fn png_of_slide_returns_png_bytes() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let png = s.png("s-libre").unwrap();
    assert_that!(png[..4].to_vec()).is_equal_to(b"\x89PNG".to_vec());
}

#[test]
fn png_of_unknown_slide_returns_no_slide() {
    let mut s = session();
    assert_that!(s.png("inconnue").unwrap_err().kind()).is_equal_to(SessionErrorKind::NoSlide);
}

/// t-450 : le reçu désigne la slide (identifiant, empreinte de sa page) ; il ne porte pas son rendu.
#[test]
fn report_surface_on_a_slide_carries_its_reference_not_its_svg() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let snap = serde_json::to_value(s.snapshot()).unwrap();
    let hash = snap["slides"].as_array().unwrap().iter().find(|x| x["id"] == "s-libre").unwrap()["hash"].clone();
    assert_that!(hash.as_str().unwrap().len()).is_equal_to(32);
    let spec = Report {
        title: "Fait".into(),
        facts: vec!["a".into()],
        slide: Some("s-libre".into()),
        swatch: None,
        why: None,
        actions: vec![oku_slides_session::agent::NextStep { label: "Encore".into(), prompt: None }],
    };
    let r = s.report_surface("report-1", &spec).unwrap();
    for surface in [&r.surface, r.plain.as_ref().unwrap()] {
        let json = serde_json::to_value(surface).unwrap();
        let text = json.to_string();
        assert_that!(text.contains("<svg")).is_false();
        assert_that!(receipt_card(&json)["slide"]).is_equal_to(json!({ "id": "s-libre", "hash": hash }));
    }
}

/// La carte `Receipt` d'une surface de reçu, cherchée par son type (et non par sa position).
fn receipt_card(surface: &serde_json::Value) -> serde_json::Value {
    let components = surface["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["updateComponents"]["components"].as_array())
        .flatten();
    let cards: Vec<_> = components.filter(|c| c["component"] == "Receipt").collect();
    assert_that!(cards.len()).named("cartes Receipt").is_equal_to(1);
    cards[0].clone()
}

fn report(title: &str, facts: &[&str], swatch: Option<&str>) -> Report {
    Report {
        title: title.into(),
        facts: facts.iter().map(|f| (*f).into()).collect(),
        slide: None,
        swatch: swatch.map(str::to_owned),
        why: None,
        actions: Vec::new(),
    }
}

/// Les bornes de `report`, vérifiées au niveau de la session (là où l'outil les fait respecter).
#[rstest::rstest]
#[case::swatch_not_hex(report("Fait", &["a"], Some("bleu")), "swatch: “bleu” is not a hex color")]
#[case::swatch_without_hash(report("Fait", &["a"], Some("00154e")), "is not a hex color")]
#[case::swatch_bad_length(report("Fait", &["a"], Some("#00154")), "is not a hex color")]
#[case::long_title(report(&"x".repeat(121), &["a"], None), "title: 121 characters, at most 120")]
#[case::long_fact(report("Fait", &["a", &"é".repeat(121)], None), "facts[1]: 121 characters, at most 120")]
#[case::bidi_title(report("Fait\u{202E}", &["a"], None), "title: invisible or bidirectional")]
#[case::bidi_fact(report("Fait", &["\u{2066}a"], None), "facts[0]: invisible or bidirectional")]
#[case::blank_facts(report("Fait", &["  ", "", "\t"], None), "facts: must not be empty")]
fn report_surface_out_of_bounds_returns_agent_form(#[case] spec: Report, #[case] reason: &str) {
    let mut s = session();
    let err = s.report_surface("report-1", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.localize(english())).contains(reason);
}

/// t-458 : un fait blanc parmi des faits non blancs est écarté (pas de ligne vide sur la carte).
#[test]
fn report_surface_drops_blank_facts_among_others() {
    let mut s = session();
    let r = s.report_surface("report-1", &report("Fait", &["a", "  ", "b", "\t"], None)).unwrap();
    let json = serde_json::to_value(&r.surface).unwrap();
    assert_that!(receipt_card(&json)["facts"].clone()).is_equal_to(json!(["a", "b"]));
}

#[rstest::rstest]
#[case::six_digits("#00154e")]
#[case::three_digits("#fff")]
#[case::eight_digits("#00154eCC")]
fn report_surface_with_a_hex_swatch_shows_it(#[case] swatch: &str) {
    let mut s = session();
    let r = s.report_surface("report-1", &report(&"x".repeat(120), &["a"], Some(swatch))).unwrap();
    let json = serde_json::to_value(&r.surface).unwrap();
    assert_that!(receipt_card(&json)["swatch"].clone()).is_equal_to(json!(swatch));
    assert_that!(r.plain).is_none();
}

#[test]
fn rewrite_surface_of_text_element_diffs_each_proposal() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = AskRewrite {
        slide: "s-libre".into(),
        owner: "e-texte".into(),
        field: None,
        proposals: vec!["Un bloc posé tout près du coin".into(), "Un bloc libre".into()],
    };
    let surface = s.rewrite_surface("ask-9", &spec).unwrap();
    assert_that!(surface.place).is_equal_to(Place::Sheet);
    let json = serde_json::to_string(&surface).unwrap();
    assert_that!(json.matches("\"DiffText\"").count()).is_equal_to(2);
    assert_that!(json.contains("\"draft\":\"Un bloc posé tout près du coin\"")).is_true();
}

#[test]
fn rewrite_surface_of_title_field_reads_slide_title() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let title = s.deck().slide("s-constat").and_then(|sl| sl.params.text("title").map(str::to_owned)).unwrap();
    let spec = rewrite("s-constat", "s-constat", Some("title"), &["A", "B"]);
    let json = serde_json::to_string(&s.rewrite_surface("ask-10", &spec).unwrap()).unwrap();
    assert_that!(json.contains(&title)).is_true();
}

#[test]
fn rewrite_with_one_proposal_returns_agent_form_error() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-libre", "e-texte", None, &["Seule"]);
    assert_that!(s.rewrite_surface("ask-11", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn rewrite_ops_of_element_with_paragraphs_carries_their_runs() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-libre", "e-texte", None, &["a", "b"]);
    let bold = Paragraph { runs: vec![Run { bold: true, ..Run::plain("Gras") }], list: None, heading: None };
    let ops = s.rewrite_ops(&spec, "**Gras**", Some(vec![bold])).unwrap();
    s.apply("Reformulation", ops, None).unwrap();
    let deck = s.deck();
    let located = deck.slide("s-libre").unwrap().locate("e-texte").unwrap();
    let run = &located.element.params.rich("body").unwrap()[0].runs[0];
    assert_that!(run.bold).is_true();
    assert_that!(run.t.as_str()).is_equal_to("Gras");
}

#[test]
fn rewrite_ops_of_element_without_paragraphs_returns_agent_form_error() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-libre", "e-texte", None, &["a", "b"]);
    let err = s.rewrite_ops(&spec, "Nouveau", None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert!(matches!(err.agent_form_reason(), Some(AgentFormReason::Missing { at }) if at == "paragraphs"));
    // t-326 : la raison est dite dans la langue de l'utilisateur, pas en anglais.
    assert_that!(err.localize(&Localizer::new(Some("fr"))))
        .is_equal_to("La demande de l'agent est invalide : paragraphs : manquant".to_owned());
}

#[test]
fn agent_rewrite_of_the_slide_title_field_sets_the_slide_title_param() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-constat", "s-constat", Some("title"), &["A", "B"]);
    let ops = s.rewrite_ops(&spec, "Nouveau titre", None).unwrap();
    assert_that!(ops).is_equal_to(vec![Op::SetParam {
        target: Target { slide: "s-constat".into(), element: None },
        param: "title".into(),
        value: json!("Nouveau titre"),
    }]);
}

/// t-326 : un champ que le widget de la slide ne déclare pas est refusé.
#[test]
fn rewrite_surface_of_an_unknown_field_returns_unknown_field() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-constat", "s-constat", Some("nope"), &["A", "B"]);
    assert_that!(s.rewrite_surface("ask-nope", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::UnknownField);
}

/// t-326 : un paramètre déclaré mais pas texte (`layout`) n'est pas un champ à reformuler.
#[test]
fn rewrite_surface_of_a_non_text_param_returns_unknown_field() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-priorites", "s-priorites", Some("layout"), &["A", "B"]);
    assert_that!(s.rewrite_surface("ask-layout", &spec).unwrap_err().kind())
        .is_equal_to(SessionErrorKind::UnknownField);
}

/// t-326 : un nom de champ qui ressemble à un chemin ou à du Typst n'est jamais pris tel quel.
#[rstest::rstest]
#[case::list_path("blocks/b-points/body")]
#[case::typst("title)#panic(")]
#[case::old_prefix("@title")]
#[case::empty("")]
fn rewrite_surface_of_a_forged_field_returns_unknown_field(#[case] field: &str) {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-constat", "s-constat", Some(field), &["A", "B"]);
    assert_that!(s.rewrite_surface("ask-forged", &spec).unwrap_err().kind())
        .is_equal_to(SessionErrorKind::UnknownField);
}

/// t-326 : un élément d'une autre slide n'est pas un objet de celle-ci.
#[test]
fn rewrite_surface_of_an_owner_on_another_slide_returns_unknown_owner() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-constat", "c1", Some("title"), &["A", "B"]);
    assert_that!(s.rewrite_surface("ask-foreign", &spec).unwrap_err().kind())
        .is_equal_to(SessionErrorKind::UnknownOwner);
}

#[test]
fn rewrite_surface_of_an_unknown_owner_returns_unknown_owner() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-constat", "inconnu", None, &["A", "B"]);
    assert_that!(s.rewrite_surface("ask-unknown", &spec).unwrap_err().kind())
        .is_equal_to(SessionErrorKind::UnknownOwner);
}

#[test]
fn rewrite_surface_of_an_unknown_slide_returns_no_slide() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-ailleurs", "s-ailleurs", Some("title"), &["A", "B"]);
    assert_that!(s.rewrite_surface("ask-elsewhere", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::NoSlide);
}

/// t-326 : la slide n'a pas de « texte par défaut » : il faut nommer son champ.
#[test]
fn rewrite_surface_of_the_slide_without_field_returns_field_needed() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-constat", "s-constat", None, &["A", "B"]);
    assert_that!(s.rewrite_surface("ask-slide", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::FieldNeeded);
}

/// t-326 : un élément sans paramètre `rich` (une forme) ne se reformule pas sans champ.
#[test]
fn rewrite_surface_of_an_element_without_rich_text_returns_field_needed() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-libre", "e-fond", None, &["A", "B"]);
    assert_that!(s.rewrite_surface("ask-shape", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::FieldNeeded);
}

/// t-326 : le champ texte d'un élément (le titre d'une carte) se reformule comme celui d'une slide.
#[test]
fn rewrite_of_an_element_plain_field_sets_that_param_of_the_element() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let title =
        s.deck().slide("s-priorites").unwrap().locate("c1").unwrap().element.params.text("title").map(str::to_owned);
    let spec = rewrite("s-priorites", "c1", Some("title"), &["Premier pari", "Autre pari"]);
    let json = serde_json::to_value(s.rewrite_surface("ask-card", &spec).unwrap()).unwrap();
    assert_that!(json["anchor"].clone())
        .is_equal_to(json!({ "slide": "s-priorites", "owner": "c1", "field": "title" }));
    assert_that!(json.to_string().contains(title.as_deref().unwrap())).is_true();
    let ops = s.rewrite_ops(&spec, "Premier pari", None).unwrap();
    assert_that!(ops).is_equal_to(vec![Op::SetParam {
        target: Target { slide: "s-priorites".into(), element: Some("c1".into()) },
        param: "title".into(),
        value: json!("Premier pari"),
    }]);
}

/// t-326 : le champ `rich` nommé d'un élément prend les paragraphes, comme le champ par défaut.
#[test]
fn rewrite_ops_of_a_named_rich_field_carries_the_paragraphs() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-priorites", "c1", Some("body"), &["a", "b"]);
    let ops = s.rewrite_ops(&spec, "Corps", Some(vec![Paragraph::plain("Corps")])).unwrap();
    s.apply("Reformulation", ops, None).unwrap();
    let body =
        s.deck().slide("s-priorites").unwrap().locate("c1").unwrap().element.params.rich("body").unwrap().to_vec();
    assert_that!(body).is_equal_to(vec![Paragraph::plain("Corps")]);
}

/// t-326 : les ops d'une reformulation refusent elles aussi une adresse forgée (l'action peut
/// arriver après un changement du deck).
#[test]
fn rewrite_ops_of_a_foreign_owner_returns_unknown_owner() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-constat", "e-texte", None, &["a", "b"]);
    let err = s.rewrite_ops(&spec, "x", Some(vec![Paragraph::plain("x")])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownOwner);
}

#[test]
fn check_anchor_accepts_an_element_a_slide_field_and_an_element_field() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    for a in [
        anchor("s-libre", "e-texte", None),
        anchor("s-constat", "s-constat", Some("title")),
        anchor("s-priorites", "c2", Some("kicker")),
        anchor("s-libre", "e-fond", None),
    ] {
        assert_that!(s.check_anchor(&a)).named(&format!("{a:?}")).is_ok();
    }
}

#[rstest::rstest]
#[case::foreign_owner(anchor("s-constat", "c1", None), SessionErrorKind::UnknownOwner)]
#[case::unknown_owner(anchor("s-constat", "x", None), SessionErrorKind::UnknownOwner)]
#[case::unknown_slide(anchor("s-x", "s-x", Some("title")), SessionErrorKind::NoSlide)]
#[case::unknown_field(anchor("s-constat", "s-constat", Some("nope")), SessionErrorKind::UnknownField)]
#[case::element_unknown_field(anchor("s-libre", "e-texte", Some("title")), SessionErrorKind::UnknownField)]
#[case::slide_without_field(anchor("s-constat", "s-constat", None), SessionErrorKind::FieldNeeded)]
fn check_anchor_refuses_an_address_outside_the_deck(#[case] a: Anchor, #[case] kind: SessionErrorKind) {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    assert_that!(s.check_anchor(&a).unwrap_err().kind()).is_equal_to(kind);
}

#[test]
fn rewrite_ops_of_field_ignores_missing_paragraphs() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = rewrite("s-constat", "s-constat", Some("title"), &["A", "B"]);
    let ops = s.rewrite_ops(&spec, "Nouveau titre", None).unwrap();
    s.apply("Reformulation", ops, None).unwrap();
    assert_that!(s.deck().slide("s-constat").unwrap().params.text("title")).is_equal_to(Some("Nouveau titre"));
}

fn theme_field(template: serde_json::Value) -> ParamField {
    ParamField {
        label: "Thème".into(),
        kind: FieldKind::Select,
        value: json!("papier"),
        op_template: template,
        min: None,
        max: None,
        step: None,
        options: vec![],
        columns: Vec::new(),
        item_label: None,
    }
}

#[test]
fn params_surface_valid_layout_goes_to_the_thread() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowParams {
        title: "Apparence".into(),
        slide: "s-libre".into(),
        fields: vec![theme_field(json!({ "op": "set_var", "key": "theme", "value": "$value" }))],
    };
    let surface = s.params_surface("show-1", &spec).unwrap();
    assert_that!(surface.place).is_equal_to(Place::Thread);
    let json = serde_json::to_value(&surface).unwrap();
    let components = json["messages"][1]["updateComponents"]["components"].as_array().unwrap();
    let f0 = components.iter().find(|c| c["id"] == "f0").unwrap();
    assert_that!(f0["component"].as_str()).is_some().is_equal_to("Select");
    assert_that!(f0["action"]["event"]["name"].as_str()).is_some().is_equal_to("param");
    assert_that!(f0["action"]["event"]["context"]["field"].as_i64()).is_some().is_equal_to(0);
    assert_that!(json["messages"][2]["updateDataModel"]["value"]["f0"].as_str()).is_some().is_equal_to("papier");
}

fn rich_field(value: serde_json::Value) -> ParamField {
    ParamField {
        label: "Corps".into(),
        kind: FieldKind::Rich,
        value,
        op_template: json!({ "op": "set_param", "target": { "slide": "s-libre", "element": "e-texte" },
            "param": "body", "value": "$value" }),
        min: None,
        max: None,
        step: None,
        options: vec![],
        columns: Vec::new(),
        item_label: None,
    }
}

#[test]
fn params_surface_rich_field_with_paragraphs_is_ok() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let field = rich_field(json!([{ "runs": [{ "t": "Un", "bold": true }] }]));
    let spec = ShowParams { title: "Texte".into(), slide: "s-libre".into(), fields: vec![field] };
    assert_that!(s.params_surface("show-rich", &spec)).is_ok();
}

#[test]
fn params_surface_rich_field_without_paragraphs_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowParams { title: "Texte".into(), slide: "s-libre".into(), fields: vec![rich_field(json!("Un"))] };
    let err = s.params_surface("show-rich", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.localize(english())).contains("fields[0]: a rich value must be a list of paragraphs");
}

#[test]
fn params_surface_list_field_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let field = ParamField { kind: FieldKind::List, ..rich_field(json!([])) };
    let spec = ShowParams { title: "Liste".into(), slide: "s-libre".into(), fields: vec![field] };
    let err = s.params_surface("show-list", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert!(matches!(
        err.agent_form_reason(),
        Some(AgentFormReason::NotAvailable { at, tool: "show_params", .. }) if at == "fields[0]"
    ));
    assert_that!(err.localize(english())).contains("fields[0]: list is not available in show_params");
}

#[test]
fn params_surface_no_fields_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![] };
    assert_that!(s.params_surface("show-1b", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn params_surface_missing_slide_returns_no_slide() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowParams {
        title: "x".into(),
        slide: "inconnue".into(),
        fields: vec![theme_field(json!({ "op": "set_var", "key": "theme", "value": "$value" }))],
    };
    assert_that!(s.params_surface("show-1c", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::NoSlide);
}

#[test]
fn params_surface_does_not_mutate_the_real_deck() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let before = s.deck().clone();
    let spec = ShowParams {
        title: "x".into(),
        slide: "s-libre".into(),
        fields: vec![theme_field(json!({ "op": "set_var", "key": "theme", "value": "$value" }))],
    };
    s.params_surface("show-1d", &spec).unwrap();
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(s.snapshot().can_undo).is_false();
}

#[test]
fn params_surface_slider_with_inverted_bounds_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let field = ParamField {
        label: "Espacement".into(),
        kind: FieldKind::Slider,
        value: json!(4.0),
        op_template: json!({ "op": "set_var", "key": "theme", "value": "$value" }),
        min: Some(10.0),
        max: Some(2.0),
        step: None,
        options: vec![],
        columns: Vec::new(),
        item_label: None,
    };
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![field] };
    let err = s.params_surface("show-1e", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("fields[0]")).is_true();
}

#[test]
fn params_surface_slider_with_only_one_bound_still_rejects_an_implicit_inverted_range() {
    // `forms::params` retombe sur 0..100 quand une borne manque : fournir
    // seul un `min` au-delà de 100 (ou seul un `max` en-dessous de 0)
    // produirait, sans cette vérification, une plage inversée à l'affichage.
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let field = ParamField {
        label: "Espacement".into(),
        kind: FieldKind::Slider,
        value: json!(4.0),
        op_template: json!({ "op": "set_var", "key": "theme", "value": "$value" }),
        min: Some(150.0),
        max: None,
        step: None,
        options: vec![],
        columns: Vec::new(),
        item_label: None,
    };
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![field] };
    let err = s.params_surface("show-1g", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("fields[0]")).is_true();
}

#[test]
fn params_surface_stepper_with_non_finite_bound_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let field = ParamField {
        label: "Colonnes".into(),
        kind: FieldKind::Stepper,
        value: json!(4),
        op_template: json!({ "op": "set_var", "key": "theme", "value": "$value" }),
        min: Some(f64::NAN),
        max: None,
        step: None,
        options: vec![],
        columns: Vec::new(),
        item_label: None,
    };
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![field] };
    let err = s.params_surface("show-1f", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("fields[0]")).is_true();
}

#[test]
fn params_surface_template_that_is_not_an_op_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowParams {
        title: "x".into(),
        slide: "s-libre".into(),
        fields: vec![theme_field(json!({ "op": "no_such_op", "v": "$value" }))],
    };
    let err = s.params_surface("show-2", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("fields[0]")).is_true();
    // La raison ne doit pas répéter le préfixe « invalid surface » : `param_op`
    // et `params_surface` construisaient chacun leur propre `AgentForm`, en
    // enveloppant le message déjà rendu de l'autre.
    assert_that!(err.one_line().matches("invalid surface").count()).is_equal_to(1);
}

#[test]
fn params_surface_template_on_missing_element_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let t = json!({ "op": "set_param", "target": { "slide": "s-libre", "element": "absent" }, "param": "body",
                "value": [{ "runs": [{ "t": "$value" }] }] });
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![theme_field(t)] };
    assert_that!(s.params_surface("show-3", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn param_op_fills_template_with_chosen_value() {
    let s = session();
    let field = theme_field(json!({ "op": "set_var", "key": "theme", "value": "$value" }));
    assert_that!(s.param_op(&field, &json!("pro")).unwrap())
        .is_equal_to(Op::SetVar { key: "theme".into(), value: json!("pro") });
}

fn retitle(title: &str) -> Vec<Op> {
    vec![Op::SetParam {
        target: Target { slide: "s-libre".into(), element: Some("e-texte".into()) },
        param: "body".into(),
        value: json!([{ "runs": [{ "t": title }] }]),
    }]
}

#[test]
fn variants_surface_renders_each_variant_without_touching_deck() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let before = s.deck().clone();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Un".into(), ops: retitle("Un") },
            Variant { label: "Deux".into(), ops: retitle("Deux") },
        ],
    };
    let surface = s.variants_surface("show-4", &spec).unwrap();
    assert_that!(surface.place).is_equal_to(Place::Drawer);
    let json = serde_json::to_value(&surface).unwrap();
    let components = json["messages"][1]["updateComponents"]["components"].as_array().unwrap();
    assert_that!(components.iter().filter(|c| c["component"] == "SlideThumb").count()).is_equal_to(2);
    let v0 = components.iter().find(|c| c["id"] == "v0").unwrap();
    let v1 = components.iter().find(|c| c["id"] == "v1").unwrap();
    assert_that!(v0["svg"].as_str().unwrap().contains("<svg")).is_true();
    assert_that!(v0["label"].as_str()).is_some().is_equal_to("1 · Un");
    assert_that!(v1["label"].as_str()).is_some().is_equal_to("2 · Deux");
    assert_that!(v0["action"]["event"]["context"]["index"].as_i64()).is_some().is_equal_to(0);
    assert_that!(v1["action"]["event"]["context"]["index"].as_i64()).is_some().is_equal_to(1);
    assert_that!(s.deck()).is_equal_to(&before);
    assert_that!(s.snapshot().can_undo).is_false();
}

#[test]
fn variants_surface_with_one_variant_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec =
        ShowVariants { slide: "s-libre".into(), variants: vec![Variant { label: "Seule".into(), ops: retitle("Un") }] };
    assert_that!(s.variants_surface("show-4b", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn variants_surface_missing_slide_returns_no_slide() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowVariants {
        slide: "inconnue".into(),
        variants: vec![
            Variant { label: "Un".into(), ops: retitle("Un") },
            Variant { label: "Deux".into(), ops: retitle("Deux") },
        ],
    };
    assert_that!(s.variants_surface("show-4c", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::NoSlide);
}

#[test]
fn variants_surface_variant_that_removes_the_target_slide_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Ok".into(), ops: retitle("Ok") },
            Variant { label: "Supprime".into(), ops: vec![Op::RemoveSlide { slide: "s-libre".into() }] },
        ],
    };
    let err = s.variants_surface("show-4d", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("variants[1]")).is_true();
}

#[test]
fn variants_surface_variant_with_too_many_ops_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let too_many = (0..=oku_slides_session::agent::forms::OPS.1).map(|_| retitle("x").remove(0)).collect();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Bonne".into(), ops: retitle("Ok") },
            Variant { label: "Trop".into(), ops: too_many },
        ],
    };
    let err = s.variants_surface("show-4e", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("variants[1]")).is_true();
}

#[test]
fn variants_surface_with_invalid_ops_fails_whole_call() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let bad = vec![Op::SetParam {
        target: Target { slide: "s-libre".into(), element: Some("absent".into()) },
        param: "body".into(),
        value: serde_json::json!([]),
    }];
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Bonne".into(), ops: retitle("Ok") },
            Variant { label: "Mauvaise".into(), ops: bad },
        ],
    };
    let err = s.variants_surface("show-5", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("variants[1]")).is_true();
}

#[test]
fn variants_surface_with_too_many_variants_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let variants = (0..=oku_slides_session::agent::forms::VARIANTS.1)
        .map(|i| Variant { label: format!("v{i}"), ops: retitle("x") })
        .collect();
    let spec = ShowVariants { slide: "s-libre".into(), variants };
    assert_that!(s.variants_surface("show-6", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn variants_surface_with_empty_ops_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Un".into(), ops: retitle("Un") },
            Variant { label: "Vide".into(), ops: vec![] },
        ],
    };
    let err = s.variants_surface("show-7", &spec).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert_that!(err.one_line().contains("variants[1]")).is_true();
}

#[test]
fn params_surface_with_too_many_fields_returns_agent_form() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let fields = (0..=oku_slides_session::agent::forms::FIELDS.1)
        .map(|_| theme_field(json!({ "op": "set_var", "key": "theme", "value": "$value" })))
        .collect();
    let spec = ShowParams { title: "x".into(), slide: "s-libre".into(), fields };
    assert_that!(s.params_surface("show-8", &spec).unwrap_err().kind()).is_equal_to(SessionErrorKind::AgentForm);
}

#[test]
fn variants_surface_called_twice_in_a_row_gives_the_same_result() {
    // Ne prouve pas, en boîte noire, que le second appel réutilise
    // effectivement `variant_renderer` plutôt que d'en reconstruire un
    // (les deux chemins produiraient le même rendu) : ce test garde surtout
    // la branche « réutilisation » du `match &mut self.variant_renderer` de
    // `variants_surface` exercée et sans régression observable (pas de
    // panique, pas de changement de résultat), pas la preuve de la
    // réutilisation elle-même.
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Un".into(), ops: retitle("Un") },
            Variant { label: "Deux".into(), ops: retitle("Deux") },
        ],
    };
    let first = s.variants_surface("show-9a", &spec).unwrap();
    let second = s.variants_surface("show-9b", &spec).unwrap();
    let json = |s: &Surface| serde_json::to_value(&s.messages[1]).unwrap()["updateComponents"]["components"].clone();
    assert_that!(json(&second)).is_equal_to(json(&first));
}

#[test]
fn variants_surface_still_works_after_reopening_the_document() {
    // Même limite que ci-dessus : rouvrir le même fichier ne change pas les
    // polices de rendu ici (`render_fonts` identique), donc la branche de
    // remise à zéro de `variant_renderer` (`lib.rs`, sur changement de
    // polices) n'est pas déclenchée par ce test — il n'y a pas de fixture à
    // polices distinctes dans ce dépôt pour l'exercer directement. Ce test
    // garde seulement la non-régression : un appel après réouverture reste
    // sans erreur.
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    let spec = ShowVariants {
        slide: "s-libre".into(),
        variants: vec![
            Variant { label: "Un".into(), ops: retitle("Un") },
            Variant { label: "Deux".into(), ops: retitle("Deux") },
        ],
    };
    s.variants_surface("show-10a", &spec).unwrap();
    s.open(&fixture_path()).unwrap();
    let surface = s.variants_surface("show-10b", &spec).unwrap();
    assert_that!(surface.place).is_equal_to(Place::Drawer);
}

#[test]
fn agent_context_describes_mode_slide_selection_and_drains_notes() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.note("variant picked: \"image à gauche\" (s-libre)");
    let ui = AgentUi {
        slide: Some("s-libre".into()),
        element: Some("e-texte".into()),
        field: None,
        editing: false,
        widget: None,
        locale: None,
    };
    let ctx = s.agent_context(&ui);
    insta::assert_snapshot!(ctx);
    assert_that!(s.agent_context(&ui).contains("since_last_turn")).is_false();
}

#[test]
fn turn_groups_every_apply_into_one_undo() {
    let mut s = session();
    s.open(&fixture_path()).unwrap();
    s.begin_turn(1, "titre plus percutant");
    s.apply(
        "a",
        vec![oku_slides_core::Op::SetTheme { theme: Some(common::ns("pro")), themes: BTreeMap::default() }],
        None,
    )
    .unwrap();
    s.apply(
        "b",
        vec![oku_slides_core::Op::SetTheme { theme: Some(common::ns("papier")), themes: BTreeMap::default() }],
        None,
    )
    .unwrap();
    s.end_turn();
    assert_that!(s.snapshot().undo_label.as_deref()).is_equal_to(Some("Agent : titre plus percutant"));
    s.undo().unwrap();
    assert_that!(s.snapshot().can_undo).is_false();
}

#[test]
fn document_dir_of_unsaved_deck_is_none() {
    assert_that!(session().document_dir()).is_none();
}
