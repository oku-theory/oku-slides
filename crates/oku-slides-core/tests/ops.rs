use oku_slides_core::*;
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

fn fixture() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("fixture {path} illisible : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("fixture {path} invalide : {e}"))
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
        Op::SetFrame {
            slide: "s-libre".into(),
            element: "e-image".into(),
            frame: Some(Frame { x: 900.0, y: 20.0, w: 700.0, h: 400.0 }),
        },
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
        Op::SetGrid { slide: "s-libre".into(), grid: Some(Grid { columns: 12, rows: 6, gap: None }) },
        Op::SetTemplates { templates: Some("grilles".into()) },
        Op::SetSlideField { slide: "s-constat".into(), field: SlideField::Name, value: json!("Constat") },
    ]
}

#[test]
fn apply_all_set_grid_and_set_cell_places_the_element() {
    let mut deck = fixture();
    let grid = Op::SetGrid { slide: "s-libre".into(), grid: Some(Grid { columns: 12, rows: 6, gap: None }) };
    let cell = Cell { col: 0, row: 0, cols: 8, rows: 4 };
    let ops = vec![grid, Op::SetCell { slide: "s-libre".into(), element: "e-texte".into(), cell: Some(cell) }];
    apply_all(&mut deck, ops).unwrap();
    let placed = deck.slide("s-libre").unwrap().elements.iter().find(|e| e.id == "e-texte").unwrap().cell;
    assert_that!(placed).is_equal_to(Some(cell));
    assert_that!(validate(&deck, None)).is_empty();
}

#[test]
fn apply_all_cell_outside_new_grid_returns_error_and_keeps_deck() {
    // Hors de la grille : refusé, deck intact.
    let mut deck = fixture();
    let before = deck.clone();
    let bad = vec![
        Op::SetGrid { slide: "s-libre".into(), grid: Some(Grid { columns: 4, rows: 3, gap: None }) },
        Op::SetCell {
            slide: "s-libre".into(),
            element: "e-texte".into(),
            cell: Some(Cell { col: 3, row: 0, cols: 2, rows: 1 }),
        },
    ];
    assert_that!(apply_all(&mut deck, bad)).is_err();
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn set_cell_without_explicit_grid_uses_default_grid() {
    // Sans grille explicite, la slide a la grille par défaut (12 × 6).
    let mut deck = fixture();
    let cell = Cell { col: 0, row: 0, cols: 8, rows: 4 };
    let default_grid = Op::SetCell { slide: "s-libre".into(), element: "e-texte".into(), cell: Some(cell) };
    assert_that!(default_grid.apply(&mut deck)).is_ok();
    let outside = Op::SetCell {
        slide: "s-libre".into(),
        element: "e-texte".into(),
        cell: Some(Cell { col: 10, row: 0, cols: 4, rows: 1 }),
    };
    assert_that!(outside.apply(&mut deck)).is_err();
}

#[test]
fn apply_each_op_changes_the_deck() {
    for op in sample_ops() {
        let mut deck = fixture();
        let before = deck.clone();
        op.clone().apply(&mut deck).unwrap_or_else(|e| panic!("{op:?} : {e}"));
        asserting(&format!("l'op {op:?} n'a rien changé")).that(&deck).is_not_equal_to(&before);
    }
}

#[test]
fn apply_all_with_a_failing_op_rolls_back_whole_transaction() {
    let mut deck = fixture();
    let before = deck.clone();
    let mut ops = sample_ops();
    ops.push(Op::SetText { slide: "s-constat".into(), element: "inexistant".into(), text: vec![] });
    let ops_len = ops.len();
    let err = apply_all(&mut deck, ops).unwrap_err();
    assert_that!(err.index()).is_equal_to(ops_len - 1);
    assert_that!(err.op_error().kind()).is_equal_to(OpErrorKind::ElementNotFound);
    assert_that!(deck).is_equal_to(before);
}

#[test]
fn add_element_with_duplicate_id_returns_duplicate_id() {
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
    assert_that!(err.kind()).is_equal_to(OpErrorKind::DuplicateId);
    assert_that!(err.subject()).is_equal_to(Some("c1"));
}

#[test]
fn deck_to_json_then_from_json_returns_equal_valid_deck() {
    let deck = fixture();
    let again = Deck::from_json(&deck.to_json().unwrap()).unwrap();
    assert_that!(deck).is_equal_to(&again);
    assert_that!(validate(&deck, None)).is_empty();
}

#[test]
fn source_bundle_to_zip_then_from_zip_returns_equal_bundle() {
    let mut b = SourceBundle::new(fixture()).unwrap();
    let id = b.add_asset(vec![1, 2, 3, 4]);
    assert_that!(id.as_str()).starts_with("sha256:");
    b.fonts.insert("Inter.ttf".into(), vec![9; 16]);
    let zip = b.to_zip().unwrap();
    let (back, _) =
        SourceBundle::from_zip(&zip, SourceLimits { replica_bytes: 1 << 24, total_bytes: 1 << 26 }).unwrap();
    assert_that!(b).is_equal_to(back);
}

#[test]
fn ops_schema_generates_schema_naming_ops() {
    let schema = ops_schema().unwrap();
    let s = schema.to_string();
    assert_that!(s.as_str()).contains("set_text");
    assert_that!(s.as_str()).contains("set_token");
}

#[test]
fn op_errors_localize_with_their_subject() {
    use oku_slides_i18n::{Localize, Localizer};
    let fr = Localizer::new(Some("fr"));
    let cases: Vec<(Op, &str)> = vec![
        (Op::SetNotes { slide: "absente".into(), notes: String::new() }, "Slide introuvable : absente"),
        (Op::SetText { slide: "s-constat".into(), element: "x".into(), text: vec![] }, "Élément introuvable : x"),
        (Op::MoveSlide { slide: "s-fin".into(), index: 99 }, "Position hors limites"),
        (
            Op::SetSlideField { slide: "s-fin".into(), field: SlideField::Title, value: json!(3) },
            "Valeur invalide pour title",
        ),
        (
            Op::SetSlideField { slide: "s-fin".into(), field: SlideField::Kind, value: json!("inconnu") },
            "Valeur invalide pour kind",
        ),
        (Op::SetToken { path: vec![], value: json!(1) }, "Valeur invalide pour path"),
        (
            Op::SetGrid { slide: "s-libre".into(), grid: Some(Grid { columns: 0, rows: 6, gap: None }) },
            "Valeur invalide pour grid",
        ),
        (
            Op::SetFrame {
                slide: "s-libre".into(),
                element: "e-texte".into(),
                frame: Some(Frame { x: 0.0, y: 0.0, w: 0.0, h: 1.0 }),
            },
            "Valeur invalide pour frame",
        ),
        (
            Op::AddElement {
                slide: "s-libre".into(),
                zone: Zone::Elements,
                parent: Some("e-texte".into()),
                index: None,
                element: Element::text("nouveau", vec![]),
            },
            "e-texte n'est pas un groupe",
        ),
        (
            Op::AddSlide { index: None, slide: Slide::new("s-fin", SlideKind::Content) },
            "Identifiant déjà utilisé : s-fin",
        ),
    ];
    for (op, expected) in cases {
        let mut deck = fixture();
        let err = op.apply(&mut deck).unwrap_err();
        asserting(expected).that(&err.localize(&fr)).is_equal_to(expected.to_owned());
    }
}

#[rstest]
#[case("")]
#[case("   ")]
fn set_slide_field_blank_title_stores_none(#[case] blank: &str) {
    let mut deck = fixture();
    let id = deck.slides[2].id.clone();
    apply_all(&mut deck, vec![Op::SetSlideField { slide: id.clone(), field: SlideField::Title, value: json!(blank) }])
        .unwrap();
    assert_that!(deck.slide(&id).unwrap().title).is_none();
}

#[test]
fn set_slide_field_padded_title_stores_it_trimmed() {
    let mut deck = fixture();
    let id = deck.slides[2].id.clone();
    apply_all(
        &mut deck,
        vec![Op::SetSlideField { slide: id.clone(), field: SlideField::Title, value: json!("  Deux colonnes ") }],
    )
    .unwrap();
    assert_that!(deck.slide(&id).unwrap().title).is_equal_to(Some("Deux colonnes".to_owned()));
}

#[rstest]
#[case("")]
#[case("   ")]
fn set_param_blank_author_stores_none(#[case] blank: &str) {
    let mut deck = fixture();
    apply_all(&mut deck, vec![Op::SetParam { key: "author".into(), value: json!(blank) }]).unwrap();
    assert_that!(deck.params.author).is_none();
}

#[test]
fn apply_all_add_slide_with_blank_prose_returns_canonical_deck() {
    let mut deck = fixture();
    let mut slide = Slide::new("s-neuve", SlideKind::Content);
    slide.title = Some(" ".into());
    slide.notes = "\n".into();
    apply_all(&mut deck, vec![Op::AddSlide { index: None, slide }]).unwrap();
    let added = deck.slide("s-neuve").unwrap();
    assert_that!(added.title).is_none();
    assert_that!(added.notes).is_empty();
    let mut canonical = deck.clone();
    canonicalize(&mut canonical);
    assert_that!(deck).is_equal_to(canonical);
}

#[test]
fn set_text_on_image_returns_no_text() {
    use oku_slides_i18n::{Localize, Localizer};
    let mut deck = fixture();
    let err =
        Op::SetText { slide: "s-libre".into(), element: "e-image".into(), text: vec![] }.apply(&mut deck).unwrap_err();
    assert_that!(err.kind()).is_equal_to(OpErrorKind::NoText);
    assert_that!(err.localize(&Localizer::new(Some("en")))).is_equal_to("e-image has no text".to_owned());
}
