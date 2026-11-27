//! Spreadsheets end to end: caps, bombs, hidden sheets and cell values.

#![expect(clippy::panic, reason = "test helpers fail loudly on a broken fixture")]

mod common;

use common::extract;
use common::{
    Sheet, limits, ods, ods_content, ods_raw, ods_with_styles, read, worksheet, xlsx, xlsx_with, zip_dir, zip_entries,
};
use oku_slides_core::knowledge::Extracted;
use oku_slides_core::knowledge::{Cut, CutKind, DocumentType, ExtractRefusal};
use oku_slides_import::ReadLimits;
use speculoos::prelude::*;

fn run(bytes: &[u8], kind: DocumentType) -> Result<Extracted, ExtractRefusal> {
    extract(bytes, kind, &limits(), &read(), "0123456789abcdef")
}

fn one_sheet(rows: &str) -> Vec<u8> {
    xlsx_with(&[Sheet { name: "Data", state: "visible", rows: rows.to_owned() }], None, None)
}

fn inline(r: &str, text: &str) -> String {
    format!(r#"<c r="{r}" t="inlineStr"><is><t>{text}</t></is></c>"#)
}

#[test]
fn xlsx_past_row_cap_states_cut() {
    let out = run(&xlsx(2001, 2), DocumentType::Xlsx).unwrap();
    assert_that!(out.markdown.lines().filter(|l| l.starts_with("| ") && !l.starts_with("| ---")).count())
        .is_equal_to(2000);
    assert_that!(out.markdown.as_str()).contains("… 1 rows not imported (limit 2000)\n");
    assert_that!(out.cuts).is_equal_to(vec![Cut { what: CutKind::Rows, count: 1 }]);
}

#[test]
fn xlsx_past_column_cap_states_cut() {
    let out = run(&xlsx(2, 53), DocumentType::Xlsx).unwrap();
    let header = out.markdown.lines().find(|l| l.starts_with("| ")).unwrap_or_default();
    assert_that!(header.matches(" |").count()).is_equal_to(50);
    assert_that!(out.markdown.as_str()).contains("… 3 columns not imported (limit 50)\n");
    assert_that!(out.cuts).is_equal_to(vec![Cut { what: CutKind::Columns, count: 3 }]);
}

#[test]
fn xlsx_far_corner_cell_bounded() {
    let sheet =
        worksheet(r#"<dimension ref="A1:XFD1048576"/>"#, r#"<row r="1048576"><c r="XFD1048576"><v>1</v></c></row>"#);
    let bytes = replace_part(&one_sheet(""), "xl/worksheets/sheet1.xml", &sheet);
    let out = run(&bytes, DocumentType::Xlsx).unwrap();
    assert_that!(out.markdown.len()).is_less_than(200);
    assert_that!(out.cuts).is_equal_to(vec![Cut { what: CutKind::Columns, count: 16_384 - 50 }]);
}

#[test]
fn xlsx_shared_strings_past_nodes_too_large() {
    let shared = (0..100).map(|i| format!("<si><t>s{i}</t></si>")).collect::<Vec<_>>().concat();
    let shared = format!(r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">{shared}</sst>"#);
    let bytes = xlsx_with(
        &[Sheet { name: "Data", state: "visible", rows: r#"<row><c t="s"><v>0</v></c></row>"#.to_owned() }],
        Some(&shared),
        None,
    );
    let tight = ReadLimits { max_xml_nodes: 120, ..read() };
    let err = extract(&bytes, DocumentType::Xlsx, &limits(), &tight, "t").err();
    assert_that!(err).is_equal_to(Some(ExtractRefusal::TooLarge));
}

#[test]
fn xlsx_shared_string_index_read_and_out_of_range_damaged() {
    let shared = r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>first</t></si><si><r><t>ri</t></r><r><t>ch</t></r><rPh><t>x</t></rPh></si></sst>"#;
    let rows = r#"<row><c t="s"><v>0</v></c><c t="s"><v>1</v></c></row>"#.to_owned();
    let out =
        run(&xlsx_with(&[Sheet { name: "Data", state: "visible", rows }], Some(shared), None), DocumentType::Xlsx)
            .unwrap();
    assert_that!(out.markdown.as_str()).contains("| first | rich |");
    let rows = r#"<row><c t="s"><v>7</v></c></row>"#.to_owned();
    let err =
        run(&xlsx_with(&[Sheet { name: "Data", state: "visible", rows }], Some(shared), None), DocumentType::Xlsx)
            .err();
    assert_that!(err).is_equal_to(Some(ExtractRefusal::Damaged));
}

#[test]
fn xlsx_hidden_sheet_skipped_and_stated() {
    let bytes = xlsx_with(
        &[
            Sheet { name: "Shown", state: "visible", rows: inline("A1", "public") },
            Sheet { name: "Secret", state: "hidden", rows: inline("A1", "private") },
            Sheet { name: "Deep", state: "veryHidden", rows: inline("A1", "private") },
        ],
        None,
        None,
    );
    let out = run(&bytes, DocumentType::Xlsx).unwrap();
    assert_that!(out.markdown.as_str()).contains("## Shown");
    assert_that!(out.markdown.as_str()).does_not_contain("private");
    assert_that!(out.markdown.as_str()).contains("… sheet \"Secret\" not imported (hidden)\n");
    assert_that!(out.markdown.as_str()).contains("… sheet \"Deep\" not imported (hidden)\n");
}

#[test]
fn xlsx_date_cell_rendered_as_serial() {
    let styles = r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts><numFmt numFmtId="164" formatCode="dd/mm/yyyy"/></numFmts><cellXfs><xf numFmtId="0"/><xf numFmtId="14"/><xf numFmtId="164"/></cellXfs></styleSheet>"#;
    let rows = r#"<row><c><v>1</v></c><c s="1"><v>45000</v></c><c s="2"><v>45001</v></c><c s="0"><v>7</v></c></row>"#
        .to_owned();
    let out =
        run(&xlsx_with(&[Sheet { name: "Data", state: "visible", rows }], None, Some(styles)), DocumentType::Xlsx)
            .unwrap();
    assert_that!(out.markdown.as_str()).contains("| 1 | 45000 (date serial) | 45001 (date serial) | 7 |");
}

#[test]
fn formula_cell_gives_cached_value() {
    let rows = r#"<row><c r="A1"><f>1+1</f><v>2</v></c><c r="B1" t="str"><f>"a"&amp;"b"</f><v>ab</v></c><c r="C1" t="b"><v>1</v></c><c r="D1" t="e"><v>#DIV/0!</v></c></row>"#;
    let out = run(&one_sheet(rows), DocumentType::Xlsx).unwrap();
    assert_that!(out.markdown.as_str()).contains("| 2 | ab | TRUE | #DIV/0\\! |");
    assert_that!(out.markdown.as_str()).does_not_contain("1+1");
}

#[test]
fn sheet_cell_with_pipe_and_newline_escaped() {
    let out = run(&one_sheet(&format!("<row>{}</row>", inline("A1", "a|b\nc &lt;x&gt;"))), DocumentType::Xlsx).unwrap();
    assert_that!(out.markdown.as_str()).contains("| a\\|b c \\<x\\> |");
    let ods = ods(
        r#"<table:table table:name="T"><table:table-row><table:table-cell><text:p>a|b</text:p><text:p>c</text:p></table:table-cell></table:table-row></table:table>"#,
    );
    assert_that!(run(&ods, DocumentType::Ods).unwrap().markdown.as_str()).contains("| a\\|b c |");
}

#[test]
fn xlsx_zip_bomb_too_large() {
    let bomb = worksheet("", &" ".repeat(4 << 20));
    let bytes = replace_part(&one_sheet(""), "xl/worksheets/sheet1.xml", &bomb);
    assert_that!(bytes.len()).is_less_than(64 << 10);
    let tight = ReadLimits { max_part_bytes: 1 << 20, ..read() };
    let err = extract(&bytes, DocumentType::Xlsx, &limits(), &tight, "t").err();
    assert_that!(err).is_equal_to(Some(ExtractRefusal::TooLarge));
}

/// t-1760: an ODS content part inflating past the part limit is refused, never read whole.
#[test]
fn ods_zip_bomb_too_large() {
    let content = ods_content("", &" ".repeat(4 << 20));
    let bytes = ods_raw(&content, true);
    assert_that!(bytes.len()).is_less_than(64 << 10);
    let tight = ReadLimits { max_part_bytes: 1 << 20, ..read() };
    let err = extract(&bytes, DocumentType::Ods, &limits(), &tight, "t").err();
    assert_that!(err).is_equal_to(Some(ExtractRefusal::TooLarge));
}

#[test]
fn ods_two_sheets_two_sections() {
    let out = run(&zip_dir("ods-two-sheets"), DocumentType::Ods).unwrap();
    assert_that!(out.markdown.matches("## ").count()).is_equal_to(2);
    assert_that!(out.markdown.as_str())
        .contains("## Budget\n\n| Item | Amount |\n| --- | --- |\n| Rent | 1200 |\n| Food | 350.5 |\n");
    assert_that!(out.markdown.as_str()).contains("## Team\n\n| Name | Since |\n| --- | --- |\n| Ada | 2024-03-01 |\n");
    assert_that!(out.markdown.as_str()).contains("… sheet \"Archive\" not imported (hidden)\n");
    assert_that!(out.cuts).is_empty();
}

/// t-1760: header rows and row groups are read in order; a table nested in a cell is not.
#[test]
fn ods_row_groups_read_in_order_nested_table_skipped() {
    let cell =
        |v: &str| format!(r#"<table:table-cell office:value-type="string"><text:p>{v}</text:p></table:table-cell>"#);
    let row = |v: &str| format!("<table:table-row>{}</table:table-row>", cell(v));
    let nested = format!(r#"<table:table table:name="Inner">{}</table:table>"#, row("Inner"));
    let tables = format!(
        r#"<table:table table:name="Groups"><table:table-header-rows>{}</table:table-header-rows><table:table-row-group>{}<table:table-rows>{}</table:table-rows></table:table-row-group><table:table-row><table:table-cell>{nested}</table:table-cell></table:table-row>{}</table:table>"#,
        row("Head"),
        row("One"),
        row("Two"),
        row("Three"),
    );
    let out = run(&ods(&tables), DocumentType::Ods).unwrap();
    assert_that!(out.markdown.as_str()).contains("| Head |\n| --- |\n| One |\n| Two |\n");
    assert_that!(out.markdown.as_str()).contains("| Three |\n");
    assert_that!(out.markdown.as_str()).does_not_contain("| Inner |");
}

#[test]
fn ods_repeat_bomb_bounded() {
    let bomb = r#"<table:table table:name="Bomb"><table:table-row table:number-rows-repeated="1048576"><table:table-cell table:number-columns-repeated="16384" office:value-type="float" office:value="1"/></table:table-row></table:table>"#;
    let out = run(&ods(bomb), DocumentType::Ods).unwrap();
    assert_that!(out.markdown.lines().filter(|l| l.starts_with("| ") && !l.starts_with("| ---")).count())
        .is_equal_to(2000);
    assert_that!(out.cuts).is_equal_to(vec![
        Cut { what: CutKind::Rows, count: 1_048_576 - 2000 },
        Cut { what: CutKind::Columns, count: 16_384 - 50 },
    ]);
}

#[test]
fn ods_many_bomb_tables_share_one_text_budget() {
    let text = "k".repeat(1024);
    let table = format!(
        r#"<table:table table:name="T"><table:table-row table:number-rows-repeated="1048576"><table:table-cell table:number-columns-repeated="50" office:value-type="string"><text:p>{text}</text:p></table:table-cell></table:table-row></table:table>"#
    );
    let bytes = ods(&table.repeat(1000));
    let out = run(&bytes, DocumentType::Ods).unwrap();
    assert_that!(out.markdown.len()).is_less_than_or_equal_to(limits().max_text_bytes);
    assert_that!(out.markdown.matches("## T\n").count()).is_equal_to(1);
    assert_that!(out.cuts.iter().find(|c| c.what == CutKind::Sheets).map(|c| c.count)).is_equal_to(Some(999));
    assert_that!(out.cuts.iter().any(|c| c.what == CutKind::Rows)).is_false();
}

#[test]
fn ods_many_sparse_far_column_tables_bounded() {
    let table = r#"<table:table table:name="T"><table:table-row table:number-rows-repeated="1048576"><table:table-cell table:number-columns-repeated="49"/><table:table-cell office:value-type="float" office:value="1"/></table:table-row></table:table>"#;
    let bytes = ods(&table.repeat(1000));
    let out = run(&bytes, DocumentType::Ods).unwrap();
    assert_that!(out.markdown.len()).is_less_than_or_equal_to(limits().max_text_bytes);
    assert_that!(out.markdown.matches("## T\n").count()).is_less_than(10);
    assert_that!(out.markdown.as_str()).contains(" sheets not imported (text budget spent)\n");
}

#[test]
fn ods_hidden_by_common_style_skipped() {
    let styles = r#"<style:style style:name="Hid" style:family="table"><style:table-properties table:display="false"/></style:style>"#;
    let content = ods_content(
        "",
        r#"<table:table table:name="Open"><table:table-row><table:table-cell office:value-type="string"><text:p>a</text:p></table:table-cell></table:table-row></table:table><table:table table:name="Secret" table:style-name="Hid"><table:table-row><table:table-cell office:value-type="string"><text:p>s</text:p></table:table-cell></table:table-row></table:table>"#,
    );
    let out = run(&ods_with_styles(&content, styles), DocumentType::Ods).unwrap();
    assert_that!(out.markdown.as_str()).contains("## Open\n");
    assert_that!(out.markdown.as_str()).does_not_contain("## Secret");
    assert_that!(out.markdown.as_str()).contains("… sheet \"Secret\" not imported (hidden)\n");
}

#[test]
fn ods_compressed_mimetype_type_mismatch() {
    let content = ods_content("", r#"<table:table table:name="T"/>"#);
    assert_that!(run(&ods_raw(&content, false), DocumentType::Ods).err())
        .is_equal_to(Some(ExtractRefusal::TypeMismatch));
}

#[test]
fn ods_encrypted_manifest_encrypted() {
    let manifest = r#"<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0"><manifest:file-entry manifest:full-path="content.xml"><manifest:encryption-data/></manifest:file-entry></manifest:manifest>"#;
    let bytes = zip_entries(&[
        ("mimetype", common::ODS_MIMETYPE.as_bytes(), true),
        ("META-INF/manifest.xml", manifest.as_bytes(), false),
        ("content.xml", b"\x01\x02 encrypted bytes", false),
    ]);
    assert_that!(run(&bytes, DocumentType::Ods).err()).is_equal_to(Some(ExtractRefusal::Encrypted));
}

#[test]
fn xlsx_large_shared_string_cited_by_many_cells_fast() {
    let shared = format!(
        r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>{}</t></si></sst>"#,
        "s".repeat(256 << 10)
    );
    let row = format!("<row>{}</row>", r#"<c t="s"><v>0</v></c>"#.repeat(50));
    let rows = row.repeat(800);
    let bytes = xlsx_with(&[Sheet { name: "Data", state: "visible", rows }], Some(&shared), None);
    let out = run(&bytes, DocumentType::Xlsx).unwrap();
    assert_that!(out.markdown.len()).is_less_than_or_equal_to(limits().max_text_bytes);
    assert_that!(out.cuts.iter().any(|c| c.what == CutKind::Text)).is_true();
}

/// `bytes` (a zip) with `part` replaced by `text`, other entries kept in order.
fn replace_part(bytes: &[u8], part: &str, text: &str) -> Vec<u8> {
    use std::io::Read;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap_or_else(|e| panic!("zip: {e}"));
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap_or_else(|e| panic!("zip: {e}"));
        let name = f.name().unwrap_or_else(|e| panic!("zip: {e}")).into_owned();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap_or_else(|e| panic!("zip: {e}"));
        if name == part {
            data = text.as_bytes().to_vec();
        }
        entries.push((name, data));
    }
    let refs: Vec<(&str, &[u8], bool)> = entries.iter().map(|(n, d)| (n.as_str(), d.as_slice(), false)).collect();
    zip_entries(&refs)
}
