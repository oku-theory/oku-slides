//! `OpenDocument` spreadsheets (`.ods`): visible tables of `content.xml`, cached values, repeats
//! clamped to the caps.

use std::borrow::Cow;
use std::collections::BTreeSet;

use oku_slides_core::knowledge::{ExtractLimits, ExtractRefusal};
use oku_slides_import::ReadLimits;
use oku_slides_import::bounded::BoundedZip;
use roxmltree::Node;

use crate::archive::{self, attr, children};
use crate::finish;
use crate::sheet::{Book, Grid, Row};
use oku_slides_core::knowledge::Extracted;

/// The media type an ODS package stores, uncompressed, in its first entry `mimetype`.
const MIMETYPE: &[u8] = b"application/vnd.oasis.opendocument.spreadsheet";

/// Spaces one `text:s` gives at most: a longer run shows no differently in a table cell.
const MAX_SPACES: usize = 64;

/// Reads the spreadsheet: each visible table becomes a section, hidden ones are stated.
pub(crate) fn extract(bytes: &[u8], limits: &ExtractLimits, read: &ReadLimits) -> Result<Extracted, ExtractRefusal> {
    let zip = archive::open(bytes, read)?;
    if !mimetype_first(bytes) {
        return Err(ExtractRefusal::TypeMismatch);
    }
    if zip.exists("META-INF/manifest.xml") {
        let text = archive::text(&zip, "META-INF/manifest.xml")?;
        let doc = archive::xml(&zip, &text, "META-INF/manifest.xml")?;
        if doc.descendants().any(|n| n.tag_name().name() == "encryption-data") {
            return Err(ExtractRefusal::Encrypted);
        }
    }
    let mut hidden = common_hidden_styles(&zip)?;
    let text = archive::text(&zip, "content.xml")?;
    let doc = archive::xml(&zip, &text, "content.xml")?;
    hidden.extend(hidden_styles(doc.root_element(), "automatic-styles"));
    let mut book = Book::new(limits);
    let spreadsheet = doc.descendants().find(|n| n.is_element() && n.tag_name().name() == "spreadsheet");
    for table in spreadsheet.into_iter().flat_map(|s| children(s, "table")) {
        let name = attr(table, "name").unwrap_or_default().to_owned();
        if attr(table, "style-name").is_some_and(|s| hidden.contains(s)) {
            book.skip(name, "hidden");
            continue;
        }
        if let Some(grid) = book.grid() {
            book.add(name, read_table(table, grid)?);
        }
    }
    let (body, cuts) = book.render();
    finish("", &body, "", limits, cuts, 0)
}

/// True when the archive's first entry is `mimetype`, stored uncompressed, holding the ODS
/// media type: read on the raw bytes, as the format requires.
pub(crate) fn mimetype_first(bytes: &[u8]) -> bool {
    let le16 = |o: usize| bytes.get(o..o + 2).map(|b| usize::from(u16::from_le_bytes([b[0], b[1]])));
    let (Some(method), Some(name_len), Some(extra_len)) = (le16(8), le16(26), le16(28)) else {
        return false;
    };
    let data = 30 + name_len + extra_len;
    bytes.starts_with(b"PK\x03\x04")
        && method == 0
        && bytes.get(30..30 + name_len) == Some(b"mimetype".as_slice())
        && bytes.get(18..22) == Some(u32::try_from(MIMETYPE.len()).unwrap_or(u32::MAX).to_le_bytes().as_slice())
        && bytes.get(data..data + MIMETYPE.len()) == Some(MIMETYPE)
}

/// The table styles of `styles.xml` (common and automatic) that hide their table; none
/// without that part.
fn common_hidden_styles(zip: &BoundedZip<'_>) -> Result<BTreeSet<String>, ExtractRefusal> {
    if !zip.exists("styles.xml") {
        return Ok(BTreeSet::new());
    }
    let text = archive::text(zip, "styles.xml")?;
    let doc = archive::xml(zip, &text, "styles.xml")?;
    let root = doc.root_element();
    Ok(hidden_styles(root, "styles").chain(hidden_styles(root, "automatic-styles")).collect())
}

/// Names of the table styles under `root`'s `section` that hide their table
/// (`table:display="false"`).
fn hidden_styles<'a>(root: Node<'a, '_>, section: &'static str) -> impl Iterator<Item = String> + 'a {
    children(root, section)
        .flat_map(|s| children(s, "style"))
        .filter(|s| children(*s, "table-properties").any(|p| attr(p, "display") == Some("false")))
        .filter_map(|s| attr(s, "name").map(str::to_owned))
}

/// The rows of a table, row groups included; repeats are clamped to the caps.
///
/// An explicit stack walks the table in document order, without entering a row or a nested
/// table: each node is visited once (t-1760). The stack has no depth cap of its own: the
/// part's XML node limit bounds it.
fn read_table<'l>(table: Node<'_, '_>, mut grid: Grid<'l>) -> Result<Grid<'l>, ExtractRefusal> {
    let mut stack: Vec<Node<'_, '_>> = table.children().rev().collect();
    while let Some(node) = stack.pop() {
        if !node.is_element() {
            continue;
        }
        let row = match node.tag_name().name() {
            "table-row" => node,
            "table" => continue,
            _ => {
                stack.extend(node.children().rev());
                continue;
            }
        };
        let mut out = Row::default();
        let mut col = 0u64;
        for cell in row.children().filter(|n| matches!(n.tag_name().name(), "table-cell" | "covered-table-cell")) {
            let repeat = repeat(cell, "number-columns-repeated")?;
            out.set(col, repeat, &value(cell), &grid);
            col = col.saturating_add(repeat);
        }
        grid.push(&out, repeat(row, "number-rows-repeated")?);
    }
    Ok(grid)
}

/// A repeat count (at least one); zero or a non-number is a damaged document.
fn repeat(node: Node<'_, '_>, name: &str) -> Result<u64, ExtractRefusal> {
    match attr(node, name) {
        None => Ok(1),
        Some(n) => n.trim().parse().ok().filter(|&n| n > 0).ok_or(ExtractRefusal::Damaged),
    }
}

/// The cached value of a cell: its typed value attribute (borrowed), else its paragraphs.
fn value<'a>(cell: Node<'a, '_>) -> Cow<'a, str> {
    let typed = match attr(cell, "value-type") {
        Some("float" | "percentage" | "currency") => attr(cell, "value"),
        Some("date") => attr(cell, "date-value"),
        Some("time") => attr(cell, "time-value"),
        Some("boolean") => attr(cell, "boolean-value"),
        _ => None,
    };
    if let Some(v) = typed {
        return Cow::Borrowed(v);
    }
    let mut out = String::new();
    for (i, p) in children(cell, "p").enumerate() {
        if i > 0 {
            out.push('\n');
        }
        paragraph(p, &mut out);
    }
    Cow::Owned(out)
}

/// The text of a paragraph or span, with its spaces, tabs and line breaks; notes left out.
fn paragraph(node: Node<'_, '_>, out: &mut String) {
    for child in node.children() {
        if let Some(text) = child.is_text().then(|| child.text()).flatten() {
            out.push_str(text);
            continue;
        }
        match child.tag_name().name() {
            "s" => {
                let n = attr(child, "c").and_then(|c| c.parse::<usize>().ok()).unwrap_or(1);
                out.push_str(&" ".repeat(n.min(MAX_SPACES)));
            }
            "tab" => out.push(' '),
            "line-break" => out.push('\n'),
            "annotation" | "note" => {}
            _ => paragraph(child, out),
        }
    }
}

#[cfg(test)]
#[path = "ods_tests.rs"]
mod ods_tests;
