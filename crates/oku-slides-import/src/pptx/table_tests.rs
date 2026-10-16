use speculoos::prelude::*;

use super::*;
use crate::pptx::test_support::{TABLE_STYLES, TestPkg, run_ctx, with_table_styles};
use crate::pptx::xml::parse;
use crate::source::{Anchor, Fill, Origin, Rgba, Styled, Warning, WarningKind};

/// Tableau 3×2 : en-tête fusionné, bandes de lignes.
const TBL: &str = r#"<a:tbl xmlns:a="urn:a"><a:tblPr firstRow="1" bandRow="1"><a:tableStyleId>{S}</a:tableStyleId></a:tblPr>
<a:tblGrid><a:gridCol w="1270000"/><a:gridCol w="2540000"/></a:tblGrid>
<a:tr h="381000"><a:tc gridSpan="2"><a:txBody><a:bodyPr/><a:p><a:r><a:t>Titre</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc hMerge="1"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr>
<a:tr h="381000"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>A</a:t></a:r></a:p></a:txBody><a:tcPr marL="0" anchor="ctr"/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>1</a:t></a:r></a:p></a:txBody><a:tcPr><a:lnL w="38100"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:lnL><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:tcPr></a:tc></a:tr>
<a:tr h="381000"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>B</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>2</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr>
</a:tbl>"#;

fn table_of(xml: &str) -> (Table, Vec<Warning>) {
    let t = TestPkg::with(&[]);
    let pkg = t.pkg();
    let doc = parse(xml, "t", 10_000).unwrap();
    with_table_styles(&pkg, TABLE_STYLES, |cx| parse_table(doc.root_element(), cx))
}

fn fill_hex(t: &Table, r: usize, c: usize) -> String {
    match &t.rows[r][c].fill {
        Fill::Solid { color } => color.value.to_hex(),
        f => panic!("plein attendu en ({r},{c}), trouvé {f:?}"),
    }
}

#[test]
fn grid_rows_and_flags_are_read() {
    let (t, _) = table_of(TBL);
    assert_that!((t.col_widths.clone(), t.row_heights.clone()))
        .is_equal_to((vec![100.0, 200.0], vec![30.0, 30.0, 30.0]));
    assert_that!((t.flags.first_row, t.flags.band_row, t.flags.first_col)).is_equal_to((true, true, false));
}

#[test]
fn merged_cells_are_spanned_and_covered() {
    let (t, _) = table_of(TBL);
    assert_that!((t.rows[0][0].col_span, t.rows[0][1].covered)).is_equal_to((2, true));
}

#[test]
fn first_row_takes_its_style_part() {
    let (t, _) = table_of(TBL);
    assert_that!(fill_hex(&t, 0, 0)).is_equal_to("#4472c4".to_owned());
    let run = &t.rows[0][0].text.paragraphs[0].runs[0].style;
    assert_that!((run.bold, run.color.origin.slot())).is_equal_to((true, Some("lt1")));
}

#[test]
fn bands_alternate_after_the_header_row_with_exact_colors() {
    let (t, _) = table_of(TBL);
    // ligne 1 = band1H (accent1 tint 40 %) ; ligne 2 = wholeTbl seul (accent1 tint 20 %).
    assert_that!((fill_hex(&t, 1, 0), fill_hex(&t, 2, 0), fill_hex(&t, 2, 1))).is_equal_to((
        "#cfd5ea".to_owned(),
        "#e9ebf5".to_owned(),
        "#e9ebf5".to_owned(),
    ));
}

#[test]
fn body_cell_text_takes_the_whole_table_color() {
    let (t, _) = table_of(TBL);
    assert_that!(t.rows[2][0].text.paragraphs[0].runs[0].style.color.origin.clone()).is_equal_to(Origin::scheme("dk1"));
}

#[test]
fn cell_properties_override_the_style() {
    let (t, _) = table_of(TBL);
    assert_that!(fill_hex(&t, 1, 1)).is_equal_to("#ff0000".to_owned());
    assert_that!((
        t.rows[1][0].text.insets.l,
        t.rows[1][0].text.anchor,
        t.rows[1][1].borders.l.as_ref().map(|s| s.width)
    ))
    .is_equal_to((0.0, Anchor::Middle, Some(3.0)));
}

#[test]
fn inside_horizontal_border_applies_between_rows_only() {
    let (t, _) = table_of(TBL);
    assert_that!((t.rows[1][0].borders.t.as_ref().map(|s| s.width), t.rows[0][0].borders.t.clone()))
        .is_equal_to((Some(1.0), None));
}

#[test]
fn band_columns_skip_the_first_column_and_count_after_it() {
    let xml = TBL.replace(
        r#"<a:tblPr firstRow="1" bandRow="1">"#,
        r#"<a:tblPr firstRow="1" bandRow="1" firstCol="1" bandCol="1">"#,
    );
    let (t, _) = table_of(&xml);
    // (2,0) : firstCol (accent2) ; (2,1) : première colonne en bande → band1V (accent3).
    assert_that!((fill_hex(&t, 2, 0), fill_hex(&t, 2, 1))).is_equal_to(("#ed7d31".to_owned(), "#a5a5a5".to_owned()));
}

#[test]
fn vertical_merge_and_row_span_are_read_and_bounded() {
    let xml = r#"<a:tbl xmlns:a="urn:a"><a:tblGrid><a:gridCol w="12700"/></a:tblGrid><a:tr h="12700"><a:tc rowSpan="99"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr><a:tr h="12700"><a:tc vMerge="1"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl>"#;
    let (t, _) = table_of(xml);
    assert_that!((t.rows[0][0].row_span, t.rows[1][0].covered)).is_equal_to((2, true));
}

#[test]
fn table_without_style_has_no_fill_and_no_warning() {
    let (t, w) = table_of(&TBL.replace("<a:tableStyleId>{S}</a:tableStyleId>", ""));
    assert_that!((t.rows[2][0].fill.clone(), w.len())).is_equal_to((Fill::None, 0));
}

#[test]
fn unknown_style_id_warns() {
    let (_, w) = table_of(&TBL.replace("{S}", "{00000000-0000-0000-0000-000000000000}"));
    assert_that!(
        w.iter().any(|w| w.kind == WarningKind::UnknownTableStyle("{00000000-0000-0000-0000-000000000000}".into()))
    )
    .is_true();
}

/// `TBL` au style `id`, lu sans `tableStyles.xml`.
fn builtin_table(id: &str) -> (Table, Vec<Warning>) {
    let t = TestPkg::with(&[]);
    let pkg = t.pkg();
    let xml = TBL.replace("{S}", id);
    let doc = parse(&xml, "t", 10_000).unwrap();
    let (r, w, _) = run_ctx(&pkg, "ppt/slides/slide1.xml", None, |cx| parse_table(doc.root_element(), cx)).unwrap();
    (r, w)
}

#[test]
fn default_builtin_style_without_table_styles_part_fills_header_and_bands_from_the_accent() {
    let (t, w) = builtin_table("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}");
    assert_that!(w).is_empty();
    // En-tête accent1, band1H accent1 tint 40 %, corps accent1 tint 20 % ; texte d'en-tête lt1 en gras.
    assert_that!((fill_hex(&t, 0, 0), fill_hex(&t, 1, 0), fill_hex(&t, 2, 0))).is_equal_to((
        "#4472c4".to_owned(),
        "#cfd5ea".to_owned(),
        "#e9ebf5".to_owned(),
    ));
    let head = &t.rows[0][0].text.paragraphs[0].runs[0].style;
    assert_that!((head.bold, head.color.origin.slot())).is_equal_to((true, Some("lt1")));
    let header_bottom = t.rows[0][0].borders.b.as_ref().map(|s| (s.width, s.color.origin.slot().map(str::to_owned)));
    assert_that!(header_bottom).is_equal_to(Some((3.0, Some("lt1".to_owned()))));
}

#[test]
fn builtin_style_follows_its_accent() {
    let (t, _) = builtin_table("{93296810-A885-4BE3-A3E7-6D5BEEA58F35}");
    assert_that!(t.rows[0][0].fill.clone())
        .is_equal_to(Fill::Solid { color: Styled::scheme(Rgba::new(0x70, 0xad, 0x47), "accent6") });
}

#[test]
fn builtin_guid_is_matched_ignoring_case() {
    let (t, w) = builtin_table("{5c22544a-7ee6-4342-b048-85bdc9fd1c3a}");
    assert_that!((fill_hex(&t, 0, 0), w.len())).is_equal_to(("#4472c4".to_owned(), 0));
}

#[test]
fn table_background_shows_under_translucent_and_empty_cells() {
    // Thématique 2 - Accent 1 : fond accent1, band1H lt1 à 20 % d'opacité.
    let (t, _) = builtin_table("{D113A9D2-9D6B-4929-AA2D-F23B5EE8CBE7}");
    assert_that!((fill_hex(&t, 1, 0), fill_hex(&t, 2, 0))).is_equal_to(("#698ed0".to_owned(), "#4472c4".to_owned()));
}

#[test]
fn file_style_wins_over_the_builtin_of_the_same_guid() {
    let styles = TABLE_STYLES.replace("{S}", "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}");
    let t = TestPkg::with(&[]);
    let pkg = t.pkg();
    let xml = TBL.replace("{S}", "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}");
    let doc = parse(&xml, "t", 10_000).unwrap();
    let (t, _) = with_table_styles(&pkg, &styles, |cx| parse_table(doc.root_element(), cx));
    // `tableStyles.xml` donne au corps sans bande l'accent1 tint 20 % ; ses bords intérieurs seuls.
    assert_that!((fill_hex(&t, 2, 0), t.rows[2][0].borders.l.is_none())).is_equal_to(("#e9ebf5".to_owned(), true));
}
