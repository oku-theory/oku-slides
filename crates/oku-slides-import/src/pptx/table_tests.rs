use speculoos::prelude::*;

use super::*;
use crate::pptx::test_support::{TABLE_STYLES, TestPkg, with_table_styles};
use crate::pptx::xml::parse;
use crate::source::{Anchor, Fill, Origin, Warning, WarningKind};

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
    let (_, w) = table_of(&TBL.replace("{S}", "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}"));
    assert_that!(
        w.iter().any(|w| w.kind == WarningKind::UnknownTableStyle("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}".into()))
    )
    .is_true();
}
