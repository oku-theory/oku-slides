//! Deck ops applied by `apply_all`: elements, slides and tokens.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]
#[path = "../common/mod.rs"]
mod common;

mod elements;
mod slides;
mod tokens;

use common::registry;
use oku_slides_core::kinds::base;
use oku_slides_core::kinds::essentiel::CONTENT;
use oku_slides_core::*;
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

fn fixture() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("fixture {path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("fixture {path} invalide : {e}"))
}

fn deck(value: Value) -> Deck {
    serde_json::from_value(value).unwrap_or_else(|e| panic!("deck invalide : {e}"))
}

fn element(value: Value) -> Element {
    serde_json::from_value(value).unwrap_or_else(|e| panic!("élément invalide : {e}"))
}

fn para(t: &str) -> Vec<Paragraph> {
    vec![Paragraph::plain(t)]
}

fn slide(id: &str) -> Target {
    Target { slide: id.into(), element: None }
}

fn at(slide: &str, element: &str) -> Target {
    Target { slide: slide.into(), element: Some(element.into()) }
}

fn slot(owner: Target, param: &str) -> SlotRef {
    SlotRef { owner, param: param.into() }
}

fn set_param(target: Target, param: &str, value: Value) -> Op {
    Op::SetParam { target, param: param.into(), value }
}

fn add(slot: SlotRef, index: Option<usize>, element: Element) -> Op {
    Op::AddElement { slot, index, defaults: false, element }
}

/// Applique `op` au deck avec le registre de base.
fn apply(deck: &mut Deck, op: Op) -> Result<(), OpError> {
    op.apply(deck, &registry())
}

fn apply_err(deck: &mut Deck, op: Op) -> OpError {
    let before = deck.clone();
    let Err(err) = apply(deck, op) else { panic!("l'op devait être refusée") };
    assert_that!(*deck).named("deck après refus").is_equal_to(before);
    err
}

fn sample_ops() -> Vec<Op> {
    vec![
        set_param(at("s-constat", "b-points"), "body", json!(para("Nouveau texte"))),
        set_param(at("s-priorites", "c2"), "body", json!(para("Carte réécrite"))),
        Op::SetStyle {
            slide: "s-libre".into(),
            element: "e-texte".into(),
            style: Style { color: Some("accent".into()), size: Some(TokenValue::Number(48.0)), ..Default::default() },
        },
        Op::SetFrame {
            slide: "s-libre".into(),
            element: "e-image".into(),
            frame: Some(Frame { x: 900.0, y: 20.0, w: 700.0, h: 400.0 }),
        },
        add(slot(slide("s-constat"), "blocks"), Some(0), Element::text("b-intro", para("Introduction"))),
        Op::RemoveElement { slide: "s-priorites".into(), element: "c1".into() },
        set_param(slide("s-constat"), "title", json!("Titre dicté")),
        set_param(slide("s-constat"), "layout", json!("columns")),
        set_param(slide("s-ch1"), "subtitle", json!(null)),
        set_param(slide("s-libre"), "grid", json!({ "columns": 12, "rows": 6 })),
        set_param(slide("s-title"), "title", json!("Titre lié")),
        Op::SetKind { target: slide("s-ch1"), kind: CONTENT.into() },
        Op::AddSlide { index: Some(3), defaults: false, slide: Slide::new("s-nouvelle", CONTENT) },
        Op::MoveSlide { slide: "s-fin".into(), index: 1 },
        Op::RemoveSlide { slide: "s-ch2".into() },
        Op::SetNotes { slide: "s-title".into(), notes: "Bonjour à tous".into() },
        Op::SetTheme {
            theme: Namespace::parse("marque"),
            themes: [(
                Namespace::parse("marque").unwrap_or_else(|| panic!("espace de noms invalide")),
                ThemePin {
                    id: uuid::Uuid::from_u128(9),
                    version: PackVersion::from_hex(&"d".repeat(64)).unwrap_or_else(|| panic!("hex invalide")),
                    extends: Vec::new(),
                },
            )]
            .into(),
        },
        Op::SetToken { path: vec!["color".into(), "accent".into()], value: json!("#2f5fb3") },
        Op::SetToken { path: vec!["component".into(), "header".into(), "left".into()], value: Value::Null },
        Op::SetTokens { tokens: json!({ "size": { "body": 30 } }) },
        Op::SetToken { path: vec!["component".into(), "card".into(), "height".into()], value: json!("fill") },
        Op::SetDeckParam { key: "author".into(), value: json!("Quelqu'un") },
        Op::SetDeckParam { key: "numbering".into(), value: json!("{n}") },
        Op::SetVar { key: "client".into(), value: json!("Globex") },
        Op::SetVar { key: "confidential".into(), value: json!(null) },
        Op::AddPack {
            kind: PackKind::Widgets,
            namespace: Namespace::parse("acme").unwrap_or_else(|| panic!("espace de noms invalide")),
            pin: PackPin {
                id: uuid::Uuid::from_u128(7),
                version: PackVersion::from_hex(&"c".repeat(64)).unwrap_or_else(|| panic!("hex invalide")),
                requires: Vec::new(),
            },
        },
        Op::AddPack {
            kind: PackKind::Layouts,
            namespace: Namespace::parse("grilles").unwrap_or_else(|| panic!("espace de noms invalide")),
            pin: PackPin {
                id: uuid::Uuid::from_u128(8),
                version: PackVersion::from_hex(&"e".repeat(64)).unwrap_or_else(|| panic!("hex invalide")),
                requires: Vec::new(),
            },
        },
        Op::SetCell {
            slide: "s-libre".into(),
            element: "e-texte".into(),
            cell: Some(Cell { col: 0, row: 0, cols: 8, rows: 4 }),
        },
    ]
}

// ---- Ensemble ---------------------------------------------------------------------

#[test]
fn apply_each_op_changes_the_deck() {
    for op in sample_ops() {
        let mut deck = fixture();
        let before = deck.clone();
        apply(&mut deck, op.clone()).unwrap_or_else(|e| panic!("{op:?} : {e}"));
        asserting(&format!("l'op {op:?} n'a rien changé")).that(&deck).is_not_equal_to(&before);
    }
}

#[test]
fn apply_all_sample_ops_returns_a_valid_deck() {
    let mut deck = fixture();
    apply_all(&mut deck, sample_ops(), &registry()).unwrap();
    assert_that!(validate(&deck, &registry(), None)).is_empty();
}

#[test]
fn apply_all_with_a_failing_op_rolls_back_whole_transaction() {
    let mut deck = fixture();
    let before = deck.clone();
    let mut ops = sample_ops();
    ops.push(set_param(at("s-constat", "inexistant"), "body", json!([])));
    let ops_len = ops.len();
    let err = apply_all(&mut deck, ops, &registry()).unwrap_err();
    assert_that!(err.index()).is_equal_to(ops_len - 1);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::ElementNotFound);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn apply_all_set_grid_and_set_cell_places_the_element() {
    let mut deck = fixture();
    let cell = Cell { col: 0, row: 0, cols: 8, rows: 4 };
    let ops = vec![
        set_param(slide("s-libre"), "grid", json!({ "columns": 12, "rows": 6 })),
        Op::SetCell { slide: "s-libre".into(), element: "e-texte".into(), cell: Some(cell) },
    ];
    apply_all(&mut deck, ops, &registry()).unwrap();
    let placed = deck.slide("s-libre").unwrap().locate("e-texte").unwrap().element.cell;
    assert_that!(placed).is_equal_to(Some(cell));
}

#[test]
fn apply_all_cell_outside_new_grid_returns_error_and_keeps_deck() {
    let mut deck = fixture();
    let before = deck.clone();
    let bad = vec![
        set_param(slide("s-libre"), "grid", json!({ "columns": 4, "rows": 3 })),
        Op::SetCell {
            slide: "s-libre".into(),
            element: "e-texte".into(),
            cell: Some(Cell { col: 3, row: 0, cols: 2, rows: 1 }),
        },
    ];
    assert_that!(apply_all(&mut deck, bad, &registry())).is_err();
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_cell_without_explicit_grid_uses_default_grid() {
    let mut deck = fixture();
    let cell = |col, cols| Op::SetCell {
        slide: "s-libre".into(),
        element: "e-texte".into(),
        cell: Some(Cell { col, row: 0, cols, rows: 1 }),
    };
    assert_that!(apply(&mut deck, cell(0, 8))).is_ok();
    assert_that!(apply(&mut deck, cell(10, 4))).is_err();
}

#[test]
fn set_cell_in_flow_slot_returns_invalid_value() {
    let mut deck = fixture();
    let op = Op::SetCell {
        slide: "s-priorites".into(),
        element: "c1".into(),
        cell: Some(Cell { col: 0, row: 0, cols: 1, rows: 1 }),
    };
    assert_that!(apply_err(&mut deck, op).kind()).is_equal_to(OpErrorKind::InvalidValue);
}

#[test]
fn set_style_with_valign_and_inset_anchors_the_text() {
    let mut deck = fixture();
    let style = Style { valign: Some("middle".into()), inset: Some([7.2, 3.6, 7.2, 3.6]), ..Default::default() };
    apply(&mut deck, Op::SetStyle { slide: "s-libre".into(), element: "e-texte".into(), style }).unwrap();
    let e = deck.slide("s-libre").and_then(|s| s.locate("e-texte")).unwrap().element;
    assert_that!(e.style.valign.as_deref()).is_equal_to(Some("middle"));
    assert_that!(e.style.inset).is_equal_to(Some([7.2, 3.6, 7.2, 3.6]));
}

#[rstest]
#[case::set_valign(false, Style { valign: Some("center".into()), ..Default::default() }, "style.valign")]
#[case::set_inset(false, Style { inset: Some([0.0, f64::NAN, 0.0, 0.0]), ..Default::default() }, "style.inset")]
#[case::replace_inset(true, Style { inset: Some([1e9, 0.0, 0.0, 0.0]), ..Default::default() }, "style.inset")]
#[case::set_tracking(false, Style { tracking: Some(-1e9), ..Default::default() }, "style.tracking")]
#[case::set_spacing(false, Style { spacing: Some(-2.0), ..Default::default() }, "style.spacing")]
#[case::replace_indent(true, Style { indent: Some([1e9, 0.0]), ..Default::default() }, "style.indent")]
fn set_or_replace_style_out_of_its_domain_returns_invalid_value(
    #[case] replace: bool,
    #[case] style: Style,
    #[case] field: &str,
) {
    let mut deck = fixture();
    let (slide, element) = ("s-libre".to_owned(), "e-texte".to_owned());
    let op = if replace { Op::ReplaceStyle { slide, element, style } } else { Op::SetStyle { slide, element, style } };
    let err = apply_err(&mut deck, op);
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
    assert_that!(err.subject()).is_equal_to(Some(field));
}

/// Le patch se contrôle avant la cible, comme `SetFrame` : un style invalide
/// est refusé pour lui-même, même sur un élément absent.
#[rstest]
#[case::set(false)]
#[case::replace(true)]
fn set_or_replace_invalid_style_on_a_missing_element_reports_the_value_first(#[case] replace: bool) {
    let mut deck = fixture();
    let style = Style { valign: Some("center".into()), ..Default::default() };
    let (slide, element) = ("s-libre".to_owned(), "absent".to_owned());
    let op = if replace { Op::ReplaceStyle { slide, element, style } } else { Op::SetStyle { slide, element, style } };
    assert_that!(apply_err(&mut deck, op).kind()).is_equal_to(OpErrorKind::InvalidValue);
}

#[test]
fn target_without_element_serializes_without_the_field() {
    assert_that!(serde_json::to_value(slide("s1")).unwrap()).is_equal_to(json!({ "slide": "s1" }));
    let back: Target = serde_json::from_value(json!({ "slide": "s1", "element": "e1" })).unwrap();
    assert_that!(back).is_equal_to(at("s1", "e1"));
}

#[test]
fn ops_schema_generates_schema_naming_ops() {
    let s = ops_schema().unwrap().to_string();
    for name in ["set_param", "set_deck_param", "set_kind", "add_element", "set_token", "unset_token", "set_tokens"] {
        assert_that!(s.as_str()).contains(name);
    }
    assert_that!(s.as_str()).does_not_contain("\"set_text\"");
    // t-423 : `set_grid` redevient une op (grille et recadrage des éléments).
    assert_that!(s.as_str()).contains("\"set_grid\"");
}

#[test]
fn op_errors_localize_with_their_subject_and_reason() {
    let fr = Localizer::new(Some("fr"));
    let cases: Vec<(Op, &str)> = vec![
        (Op::SetNotes { slide: "absente".into(), notes: String::new() }, "Slide introuvable : absente"),
        (set_param(at("s-constat", "x"), "body", json!([])), "Élément introuvable : x"),
        (Op::MoveSlide { slide: "s-fin".into(), index: 99 }, "Position hors limites"),
        (set_param(slide("s-fin"), "text", json!(3)), "Valeur invalide pour text : il faut un texte ou null"),
        (Op::SetToken { path: vec![], value: json!(1) }, "Valeur invalide pour path : le chemin est vide"),
        (Op::UnsetToken { path: vec![] }, "Valeur invalide pour path : le chemin est vide"),
        (Op::SetTokens { tokens: json!([1]) }, "Valeur invalide pour tokens : un objet ou null est attendu"),
        (
            set_param(slide("s-libre"), "grid", json!({ "columns": 0, "rows": 6 })),
            "Valeur invalide pour grid : une grille a de 1 à 24 colonnes et lignes",
        ),
        (
            Op::SetFrame {
                slide: "s-libre".into(),
                element: "e-texte".into(),
                frame: Some(Frame { x: 0.0, y: 0.0, w: 0.0, h: 1.0 }),
            },
            "Valeur invalide pour frame : la largeur et la hauteur doivent être supérieures à 0",
        ),
        (Op::SetKind { target: slide("s-fin"), kind: "inconnu".into() }, "Widget inconnu : inconnu"),
        (set_param(slide("s-fin"), "titre", json!("x")), "Paramètre inconnu : titre"),
        (set_param(slide("s-constat"), "layout", json!("x")), "Valeur non permise pour layout"),
        (Op::SetKind { target: slide("s-fin"), kind: base::TEXT.into() }, "base/text ne peut pas être utilisé ici"),
        (
            Op::AddSlide { index: None, defaults: false, slide: Slide::new("s-fin", CONTENT) },
            "Identifiant déjà utilisé : s-fin",
        ),
    ];
    for (op, expected) in cases {
        let mut deck = fixture();
        let err = apply(&mut deck, op).unwrap_err();
        asserting(expected).that(&err.localize(&fr)).is_equal_to(expected.to_owned());
    }
}

/// Un type refusé dit ce qui était attendu, dans la langue de l'utilisateur (t-676).
#[rstest]
#[case::text_or_null(slide("s-fin"), "text", json!(3), "il faut un texte ou null")]
#[case::paragraphs_or_null(at("s-priorites", "c1"), "body", json!("texte"), "il faut une liste de paragraphes ou null")]
#[case::flag_or_null(at("s-priorites", "c1"), "highlight", json!("oui"), "il faut un booléen ou null")]
#[case::slot_by_value(slide("s-constat"), "blocks", json!([]), "les éléments s'ajoutent avec add_element")]
fn param_type_says_what_was_expected_in_the_user_language(
    #[case] target: Target,
    #[case] param: &str,
    #[case] value: Value,
    #[case] reason: &str,
) {
    let mut deck = fixture();
    let err = apply_err(&mut deck, set_param(target, param, value));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
    let fr = err.localize(&Localizer::new(Some("fr")));
    assert_that!(fr).is_equal_to(format!("Valeur invalide pour {param} : {reason}"));
}

#[test]
fn param_type_of_a_number_param_says_a_number_is_expected_in_french() {
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "essentiel/content",
        "params": { "blocks": [{ "id": "p", "kind": "base/stack" }] } }] }));
    let err = apply_err(&mut deck, set_param(at("s", "p"), "count", json!("4")));
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
    assert_that!(err.localize(&Localizer::new(Some("fr"))))
        .is_equal_to("Valeur invalide pour count : il faut un nombre ou null".to_owned());
}

#[test]
fn slot_rejects_localizes_with_the_kind() {
    let probe = json!({ "packs": [{ "namespace": "t", "kind": "layouts", "widgets": [
        { "name": "only-text", "category": "slide",
          "params": [{ "name": "items", "type": "slot", "mode": "flow", "accepts": ["base/text"] }] },
    ] }, { "namespace": "base", "kind": "widgets", "widgets": [
        { "name": "text", "category": "element", "params": [] },
        { "name": "shape", "category": "element", "params": [] }
    ] }] });
    let registry = WidgetRegistry::from_probe(probe).unwrap();
    let mut deck = deck(json!({ "slides": [{ "id": "s", "kind": "t/only-text" }] }));
    let err =
        add(slot(slide("s"), "items"), None, Element::new("f", base::SHAPE)).apply(&mut deck, &registry).unwrap_err();
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_equal_to("base/shape n'est pas accepté ici".to_owned());
    assert_that!(err.to_string()).is_equal_to("items does not accept base/shape".to_owned());
}
