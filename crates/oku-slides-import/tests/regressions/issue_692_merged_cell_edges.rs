//! Régression (t-692) : une cellule fusionnée qui atteint la dernière colonne (ou ligne) prenait
//! les bordures intérieures et ignorait `lastCol`/`lastRow` : bord et parties se lisaient à l'ancre.

use oku_slides_import::read_pptx;
use oku_slides_import::source::{Cell, Fill, ItemKind, Table};
use speculoos::prelude::*;

use crate::common;

const STYLES: &str = r#"<?xml version="1.0"?><a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" def="{E}"><a:tblStyle styleId="{E}" styleName="Bords">
<a:wholeTbl><a:tcStyle><a:tcBdr>
<a:right><a:ln w="12700"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:ln></a:right>
<a:bottom><a:ln w="12700"><a:solidFill><a:srgbClr val="0000FF"/></a:solidFill></a:ln></a:bottom>
<a:insideV><a:ln w="12700"><a:solidFill><a:srgbClr val="00FF00"/></a:solidFill></a:ln></a:insideV>
<a:insideH><a:ln w="12700"><a:solidFill><a:srgbClr val="00FFFF"/></a:solidFill></a:ln></a:insideH>
</a:tcBdr></a:tcStyle></a:wholeTbl>
<a:lastCol><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="111111"/></a:solidFill></a:fill></a:tcStyle></a:lastCol>
<a:lastRow><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="222222"/></a:solidFill></a:fill></a:tcStyle></a:lastRow>
</a:tblStyle></a:tblStyleLst>"#;

/// 3 lignes × 2 colonnes : (0,0) s'étend sur toute la première ligne (`gridSpan="2"`) ;
/// (1,0) descend jusqu'à la dernière ligne (`rowSpan="2"`).
const TABLE: &str = r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="9" name="Tableau"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="2540000" cy="1143000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr lastCol="1" lastRow="1"><a:tableStyleId>{E}</a:tableStyleId></a:tblPr>
<a:tblGrid><a:gridCol w="1270000"/><a:gridCol w="1270000"/></a:tblGrid>
<a:tr h="381000"><a:tc gridSpan="2"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc><a:tc hMerge="1"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr>
<a:tr h="381000"><a:tc rowSpan="2"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr>
<a:tr h="381000"><a:tc vMerge="1"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr>
</a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#;

fn table() -> Table {
    let slide = common::text("minimal/ppt/slides/slide1.xml").replace("</p:spTree>", &format!("{TABLE}</p:spTree>"));
    let rels = common::text("minimal/ppt/_rels/presentation.xml.rels").replace(
        "</Relationships>",
        r#"<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableStyles" Target="tableStyles.xml"/></Relationships>"#,
    );
    let bytes = common::pptx_plus(
        "minimal",
        &[("ppt/slides/slide1.xml", Some(&slide)), ("ppt/_rels/presentation.xml.rels", Some(&rels))],
        &[("ppt/tableStyles.xml", STYLES.as_bytes())],
    );
    let deck = read_pptx(&bytes, &common::limits()).unwrap_or_else(|e| panic!("lecture : {e}"));
    let item = deck.slides[0].items.iter().find(|i| i.name == "Tableau").unwrap_or_else(|| panic!("pas de tableau"));
    match &item.kind {
        ItemKind::Table(t) => t.clone(),
        k => panic!("tableau attendu, trouvé {k:?}"),
    }
}

fn right(c: &Cell) -> Option<String> {
    c.borders.r.as_ref().map(|s| s.color.value.to_hex())
}

fn bottom(c: &Cell) -> Option<String> {
    c.borders.b.as_ref().map(|s| s.color.value.to_hex())
}

fn fill(c: &Cell) -> Option<String> {
    match &c.fill {
        Fill::Solid { color } => Some(color.value.to_hex()),
        _ => None,
    }
}

#[test]
fn cell_spanning_to_the_last_column_takes_the_right_border_and_last_col() {
    let t = table();
    let anchor = &t.rows[0][0];
    assert_that!((anchor.col_span, right(anchor), fill(anchor))).is_equal_to((
        2,
        Some("#ff0000".to_owned()),
        Some("#111111".to_owned()),
    ));
}

#[test]
fn cell_spanning_to_the_last_row_takes_the_bottom_border_and_last_row() {
    let t = table();
    let anchor = &t.rows[1][0];
    assert_that!((anchor.row_span, bottom(anchor), fill(anchor))).is_equal_to((
        2,
        Some("#0000ff".to_owned()),
        Some("#222222".to_owned()),
    ));
}
