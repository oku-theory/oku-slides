use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::source::{Borders, Color, ColorMod, Rgba, Styled, TableFlags, TextBox};
use crate::test_support::{fixture, item, office_theme, para, run_with, source_deck, source_slide};

/// Couleur `slot` du thème Office, nuancée de `tint` (0 : pure).
fn scheme(slot: &str, tint: u8) -> Color {
    if tint == 0 {
        return Styled::scheme(office_theme().colors[slot], slot);
    }
    Styled::mapped(Rgba::new(tint, tint, 255), slot, slot, vec![ColorMod::Tint(f64::from(tint) / 255.0)])
}

/// Une cellule `text` en `size` pt, de couleur `color`, sur `fill`, au filet bas `rule`.
fn cell(text: &str, size: f64, color: &Color, fill: Fill, rule: Option<Stroke>) -> Cell {
    let r = run_with(text, |s| (s.size, s.color) = (size, color.clone()));
    Cell {
        text: TextBox { paragraphs: vec![para(vec![r])], ..TextBox::default() },
        fill,
        borders: Borders { b: rule, ..Borders::default() },
        row_span: 1,
        col_span: 1,
        covered: false,
    }
}

fn solid(c: Color) -> Fill {
    Fill::Solid { color: c }
}

fn white_rule() -> Option<Stroke> {
    Some(Stroke {
        width: 1.0,
        color: Styled::explicit(Rgba::new(255, 255, 255)),
        dash: Dash::Solid,
        head: None,
        tail: None,
    })
}

/// Le tableau de LibreOffice : en-tête accent1 en texte clair, lignes en
/// deux teintes d'accent1, filets blancs, colonnes 1:2, 16 pt.
fn libreoffice_like(widths: [f64; 2]) -> Table {
    let (ink, light) = (scheme("dk1", 0), scheme("lt1", 0));
    let row = |a: &str, b: &str, tint: u8| {
        vec![
            cell(a, 16.0, &ink, solid(scheme("accent1", tint)), white_rule()),
            cell(b, 16.0, &ink, solid(scheme("accent1", tint)), white_rule()),
        ]
    };
    let mut header = vec![cell("Titre", 16.0, &light, solid(scheme("accent1", 0)), white_rule())];
    header.push(Cell { covered: true, ..cell("", 16.0, &light, solid(scheme("accent1", 0)), None) });
    header[0].text.paragraphs[0].runs[0].style.bold = true;
    Table {
        col_widths: widths.to_vec(),
        row_heights: vec![30.0; 4],
        flags: TableFlags::default(),
        rows: vec![header, row("A", "1", 102), row("B", "2", 51), row("C", "3", 102)],
    }
}

fn look(t: &Table) -> TableLook {
    let page = Page::new(crate::source::Size { w: 960.0, h: 540.0 });
    TableLook::of(t, &ThemeColors::new(&office_theme()), &page)
}

#[test]
fn table_look_of_explicit_fills_finds_the_header_and_the_bands() {
    let l = look(&libreoffice_like([100.0, 100.0]));
    assert_that!((l.header_row, l.striped, l.irregular)).is_equal_to((true, true, false));
    assert_that!(Value::Object(l.tokens)).is_equal_to(json!({
        "header-bg": "accent",
        "header-color": "bg",
        "header-weight": "strong",
        "body-bg": "#6666ff",
        "stripe-bg": "#3333ff",
        "size": 32.0,
        "rule-color": "#ffffff",
        "rule-width": 2.0,
    }));
}

#[test]
fn table_look_of_unequal_columns_is_irregular() {
    assert_that!(look(&libreoffice_like([100.0, 200.0])).irregular).is_true();
}

#[test]
fn table_look_of_a_plain_table_has_no_header_and_no_fill() {
    let ink = scheme("dk1", 0);
    let row = |a: &str| vec![cell(a, 18.0, &ink, Fill::None, None), cell(a, 18.0, &ink, Fill::None, None)];
    let t = Table {
        col_widths: vec![50.0, 50.0],
        row_heights: Vec::new(),
        flags: TableFlags::default(),
        rows: vec![row("a"), row("b")],
    };
    let l = look(&t);
    assert_that!((l.header_row, l.striped, l.irregular)).is_equal_to((false, false, false));
    assert_that!(Value::Object(l.tokens)).is_equal_to(json!({ "body-bg": null, "size": 36.0, "rule-width": 0 }));
}

#[test]
fn table_look_of_a_cell_fill_or_a_side_border_is_irregular() {
    let mut t = libreoffice_like([100.0, 100.0]);
    t.rows[2][1].fill = solid(scheme("accent2", 0));
    assert_that!(look(&t).irregular).is_true();
    let mut t = libreoffice_like([100.0, 100.0]);
    t.rows[1][0].borders.l = white_rule();
    assert_that!(look(&t).irregular).is_true();
}

#[test]
fn table_look_of_two_text_sizes_is_irregular() {
    let mut t = libreoffice_like([100.0, 100.0]);
    t.rows[3][0].text.paragraphs[0].runs[0].style.size = 12.0;
    let l = look(&t);
    assert_that!(l.irregular).is_true();
    // La taille dominante reste le jeton.
    assert_that!(l.tokens["size"].clone()).is_equal_to(json!(32.0));
}

#[test]
fn table_look_matches_only_the_tokens_of_the_deck() {
    let first = look(&libreoffice_like([100.0, 100.0]));
    assert_that!(first.matches(Some(&first))).is_true();
    let mut other = libreoffice_like([100.0, 100.0]);
    other.rows[0][0].fill = solid(scheme("accent2", 0));
    other.rows[0][1].fill = solid(scheme("accent2", 0));
    assert_that!(look(&other).matches(Some(&first))).is_false();
    assert_that!(look(&libreoffice_like([1.0, 3.0])).matches(None)).is_false();
}

#[test]
fn table_look_set_flags_overrides_the_tblpr_hints() {
    let mut option = json!({ "dataset": { "source": [] }, "header-row": false, "striped": false });
    look(&libreoffice_like([100.0, 100.0])).set_flags(&mut option);
    assert_that!((option["header-row"].clone(), option["striped"].clone())).is_equal_to((json!(true), json!(true)));
}

#[test]
fn first_table_looks_into_groups_in_slide_order() {
    let t = libreoffice_like([100.0, 100.0]);
    let table = item("s2/3", (0.0, 0.0, 100.0, 50.0), ItemKind::Table(t.clone()));
    let group = item("s2/2", (0.0, 0.0, 100.0, 50.0), ItemKind::Group { items: vec![table], smartart: None });
    let deck = source_deck(vec![source_slide(1, None, Vec::new()), source_slide(2, None, vec![group])]);
    assert_that!(first_table(&deck)).is_equal_to(Some(&t));
    assert_that!(first_table(&source_deck(Vec::new()))).is_none();
}

#[test]
fn table_look_of_the_libreoffice_fixture() {
    let deck = fixture("libreoffice.pptx");
    let Some(t) = first_table(&deck) else { panic!("aucun tableau") };
    let l = look(t);
    assert_that!((l.header_row, l.striped)).is_equal_to((true, true));
    assert_that!(l.tokens["header-bg"].clone()).is_equal_to(json!("accent"));
    assert_that!(l.tokens["size"].clone()).is_equal_to(json!(32.0));
    // Colonnes 1:2 : le widget ne les reprend pas.
    assert_that!(l.irregular).is_true();
}

#[test]
fn table_look_of_a_mixed_header_is_irregular() {
    let mut color = libreoffice_like([100.0, 100.0]);
    color.rows[0][1] =
        Cell { covered: false, ..cell("Suite", 16.0, &scheme("dk1", 0), solid(scheme("accent1", 0)), None) };
    assert_that!(look(&color).irregular).is_true();
    let mut weight = libreoffice_like([100.0, 100.0]);
    weight.rows[0][1] =
        Cell { covered: false, ..cell("Suite", 16.0, &scheme("lt1", 0), solid(scheme("accent1", 0)), None) };
    assert_that!(look(&weight).irregular).is_true();
    let mut italic = libreoffice_like([100.0, 100.0]);
    italic.rows[0][0].text.paragraphs[0].runs[0].style.italic = true;
    assert_that!(look(&italic).irregular).is_true();
    let mut font = libreoffice_like([100.0, 100.0]);
    font.rows[0][1] =
        Cell { covered: false, ..cell("Suite", 16.0, &scheme("lt1", 0), solid(scheme("accent1", 0)), None) };
    font.rows[0][1].text.paragraphs[0].runs[0].style.bold = true;
    font.rows[0][1].text.paragraphs[0].runs[0].style.font = Styled::explicit("Arial".to_owned());
    assert_that!(look(&font).irregular).is_true();
}

#[test]
fn table_look_of_a_styled_body_is_irregular() {
    let bold = |mut t: Table| {
        t.rows[1][0].text.paragraphs[0].runs[0].style.bold = true;
        t
    };
    assert_that!(look(&bold(libreoffice_like([100.0, 100.0]))).irregular).is_true();
    let mut italic = libreoffice_like([100.0, 100.0]);
    italic.rows[2][0].text.paragraphs[0].runs[0].style.italic = true;
    assert_that!(look(&italic).irregular).is_true();
    let mut font = libreoffice_like([100.0, 100.0]);
    font.rows[2][0].text.paragraphs[0].runs[0].style.font = Styled::explicit("Arial".to_owned());
    assert_that!(look(&font).irregular).is_true();
}

#[test]
fn table_look_never_writes_null_for_a_non_nullable_token() {
    let mut t = libreoffice_like([100.0, 100.0]);
    for c in &mut t.rows[2] {
        c.fill = Fill::None;
    }
    let l = look(&t);
    assert_that!(l.striped).is_true();
    assert_that!(l.tokens["stripe-bg"].clone()).is_equal_to(json!("bg"));
    for (k, v) in &l.tokens {
        assert_that!(k == "body-bg" || !v.is_null()).is_true();
    }
}

/// Deux lignes d'un même fond (accent1 nuancé à 102), sans en-tête.
fn headless() -> Table {
    let ink = scheme("dk1", 0);
    let row = |a: &str| {
        let c = || cell(a, 16.0, &ink, solid(scheme("accent1", 102)), white_rule());
        vec![c(), c()]
    };
    Table {
        col_widths: vec![100.0, 100.0],
        row_heights: vec![30.0; 2],
        flags: TableFlags::default(),
        rows: vec![row("A"), row("B")],
    }
}

#[test]
fn table_look_without_header_matches_a_deck_style_with_one() {
    let deck = look(&libreoffice_like([100.0, 100.0]));
    let l = look(&headless());
    assert_that!(l.header_row).is_false();
    assert_that!(l.matches(Some(&deck))).is_true();
}

#[test]
fn table_look_without_header_but_another_body_does_not_match() {
    let deck = look(&libreoffice_like([100.0, 100.0]));
    let mut t = headless();
    for c in t.rows.iter_mut().flatten() {
        c.fill = solid(scheme("accent2", 0));
    }
    assert_that!(look(&t).matches(Some(&deck))).is_false();
}

#[test]
fn table_look_of_rows_taller_than_their_text_is_irregular() {
    // 16 pt sur une ligne : 19,2 pt de texte et 7,2 pt de marges ; 80 pt n'en vient pas.
    let mut t = libreoffice_like([100.0, 100.0]);
    t.row_heights[2] = 80.0;
    assert_that!(look(&t).irregular).is_true();
}

#[test]
fn table_look_of_rows_as_tall_as_their_wrapped_text_is_regular() {
    // 40 caractères de 16 pt dans 85,6 pt utiles : 4 lignes, 76,8 + 7,2 pt.
    let mut t = libreoffice_like([100.0, 100.0]);
    t.rows[2][0].text.paragraphs[0].runs[0].text = "x".repeat(40);
    t.row_heights[2] = 84.0;
    assert_that!(look(&t).irregular).is_false();
}
