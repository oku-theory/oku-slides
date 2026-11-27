//! Excel workbooks (`.xlsx`): visible worksheets, cached cell values, shared strings.

use std::borrow::Cow;
use std::collections::BTreeMap;

use oku_slides_core::knowledge::{ExtractLimits, ExtractRefusal};
use oku_slides_import::ReadLimits;
use oku_slides_import::bounded::BoundedZip;
use roxmltree::Node;

use crate::archive::{self, attr, children};
use crate::finish;
use crate::sheet::{Book, Grid, Row};
use oku_slides_core::knowledge::Extracted;

/// Reads the workbook: each visible worksheet becomes a section, hidden ones are stated.
pub(crate) fn extract(bytes: &[u8], limits: &ExtractLimits, read: &ReadLimits) -> Result<Extracted, ExtractRefusal> {
    let zip = archive::open(bytes, read)?;
    let main = archive::main_part(&zip)?;
    let text = archive::text(&zip, &main)?;
    let doc = archive::xml(&zip, &text, &main)?;
    let workbook = doc.root_element();
    if workbook.tag_name().name() != "workbook" {
        return Err(ExtractRefusal::TypeMismatch);
    }
    let rels = archive::rels(&zip, &main)?;
    let related = |suffix: &str| rels.iter().find(|r| r.kind.ends_with(suffix)).map(|r| r.target.as_str());
    let strings = related("/sharedStrings").map_or(Ok(Vec::new()), |p| shared_strings(&zip, p))?;
    let dates = related("/styles").map_or(Ok(Vec::new()), |p| date_styles(&zip, p))?;
    let cells = Cells { strings: &strings, dates: &dates };
    let mut book = Book::new(limits);
    for sheet in children(workbook, "sheets").flat_map(|s| children(s, "sheet")) {
        let name = attr(sheet, "name").unwrap_or_default().to_owned();
        if matches!(attr(sheet, "state"), Some("hidden" | "veryHidden")) {
            book.skip(name, "hidden");
            continue;
        }
        let id = sheet
            .attributes()
            .find(|a| a.name() == "id" && a.namespace().is_some_and(|ns| ns.ends_with("/relationships")))
            .map(|a| a.value());
        let rel = rels.iter().find(|r| Some(r.id.as_str()) == id).ok_or(ExtractRefusal::Damaged)?;
        if !rel.kind.ends_with("/worksheet") {
            book.skip(name, "not a worksheet");
            continue;
        }
        if let Some(grid) = book.grid() {
            book.add(name, read_sheet(&zip, &rel.target, &cells, grid)?);
        }
    }
    let (body, cuts) = book.render();
    finish("", &body, "", limits, cuts, 0)
}

/// What cell values are read with: the shared strings, and which styles are dates.
struct Cells<'a> {
    strings: &'a [String],
    dates: &'a [bool],
}

/// The rows of a worksheet in order of appearance; `<dimension>` and row numbers are not
/// trusted, so gaps cost nothing.
fn read_sheet<'l>(
    zip: &BoundedZip<'_>,
    part: &str,
    cells: &Cells<'_>,
    mut grid: Grid<'l>,
) -> Result<Grid<'l>, ExtractRefusal> {
    let text = archive::text(zip, part)?;
    let doc = archive::xml(zip, &text, part)?;
    for row in children(doc.root_element(), "sheetData").flat_map(|d| children(d, "row")) {
        let mut out = Row::default();
        let mut next = 0u64;
        for c in children(row, "c") {
            let col = match attr(c, "r") {
                Some(r) => column(r).ok_or(ExtractRefusal::Damaged)?,
                None => next,
            };
            next = col.saturating_add(1);
            out.set(col, 1, &value(c, cells)?, &grid);
        }
        grid.push(&out, 1);
    }
    Ok(grid)
}

/// The zero-based column of a cell reference (`B3` is 1), saturating; `None` without letters.
pub(crate) fn column(reference: &str) -> Option<u64> {
    let letters = reference.bytes().take_while(u8::is_ascii_alphabetic);
    let n =
        letters.fold(0u64, |n, b| n.saturating_mul(26).saturating_add(u64::from(b.to_ascii_uppercase() - b'A' + 1)));
    n.checked_sub(1)
}

/// The cached value of a cell as text, borrowed from the sheet or the shared strings; a
/// date-styled number is its serial plus `(date serial)`.
fn value<'a>(c: Node<'a, '_>, cells: &'a Cells<'_>) -> Result<Cow<'a, str>, ExtractRefusal> {
    let v = children(c, "v").next().and_then(|v| v.text()).unwrap_or_default();
    Ok(match attr(c, "t") {
        Some("s") => {
            let i: usize = v.trim().parse().map_err(|_| ExtractRefusal::Damaged)?;
            Cow::Borrowed(cells.strings.get(i).ok_or(ExtractRefusal::Damaged)?.as_str())
        }
        Some("inlineStr") => Cow::Owned(children(c, "is").next().map(rich_text).unwrap_or_default()),
        Some("b") => match v.trim() {
            "1" => Cow::Borrowed("TRUE"),
            "0" => Cow::Borrowed("FALSE"),
            _ => return Err(ExtractRefusal::Damaged),
        },
        Some("str" | "e" | "d") => Cow::Borrowed(v),
        _ => {
            let style: usize = attr(c, "s").map_or(Ok(0), str::parse).map_err(|_| ExtractRefusal::Damaged)?;
            if !v.is_empty() && cells.dates.get(style).copied().unwrap_or(false) {
                Cow::Owned(format!("{v} (date serial)"))
            } else {
                Cow::Borrowed(v)
            }
        }
    })
}

/// The text of a string item (`si`, `is`): its runs, phonetic guides (`rPh`) left out.
fn rich_text(item: Node<'_, '_>) -> String {
    item.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "t")
        .filter(|n| !n.ancestors().any(|a| a.tag_name().name() == "rPh"))
        .filter_map(|n| n.text())
        .collect()
}

/// The shared strings, in order; past `max_xml_nodes` the workbook is `TooLarge`.
fn shared_strings(zip: &BoundedZip<'_>, part: &str) -> Result<Vec<String>, ExtractRefusal> {
    let text = archive::text(zip, part)?;
    let doc = archive::xml(zip, &text, part)?;
    Ok(children(doc.root_element(), "si").map(rich_text).collect())
}

/// For each cell style (`cellXfs` index), whether its number format shows a date or a time.
fn date_styles(zip: &BoundedZip<'_>, part: &str) -> Result<Vec<bool>, ExtractRefusal> {
    let text = archive::text(zip, part)?;
    let doc = archive::xml(zip, &text, part)?;
    let root = doc.root_element();
    let custom: BTreeMap<&str, &str> = children(root, "numFmts")
        .flat_map(|f| children(f, "numFmt"))
        .filter_map(|f| Some((attr(f, "numFmtId")?, attr(f, "formatCode")?)))
        .collect();
    Ok(children(root, "cellXfs")
        .flat_map(|x| children(x, "xf"))
        .map(|xf| {
            let id = attr(xf, "numFmtId").unwrap_or("0");
            custom.get(id).map_or_else(|| id.parse().is_ok_and(is_date_format_id), |code| is_date_format(code))
        })
        .collect())
}

/// True for a built-in number format that shows a date or a time.
pub(crate) fn is_date_format_id(id: u32) -> bool {
    matches!(id, 14..=22 | 27..=36 | 45..=47 | 50..=58)
}

/// True for a format code with a date or time part once its literals (`"…"`, `[…]`, escaped
/// and padding characters) are removed.
pub(crate) fn is_date_format(code: &str) -> bool {
    let mut chars = code.chars();
    let mut found = false;
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                chars.by_ref().find(|&c| c == '"');
            }
            '[' => {
                chars.by_ref().find(|&c| c == ']');
            }
            '\\' | '_' | '*' => {
                chars.next();
            }
            c => found |= matches!(c.to_ascii_lowercase(), 'y' | 'm' | 'd' | 'h' | 's'),
        }
    }
    found
}

#[cfg(test)]
#[path = "xlsx_tests.rs"]
mod xlsx_tests;
