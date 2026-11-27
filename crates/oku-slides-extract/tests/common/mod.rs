//! Document builders for the extraction tests: zipped fixtures, generated workbooks,
//! spreadsheets and Word documents, image headers.

#![allow(dead_code, reason = "shared module: each test binary uses only part of it")]
#![expect(clippy::format_push_string, reason = "the XML builders read best as format! pieces")]

use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use oku_slides_core::knowledge::Extracted;
use oku_slides_core::knowledge::{DocumentType, ExtractLimits, ExtractRefusal};
use oku_slides_import::ReadLimits;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

/// The ODS media type, written first and stored by [`ods`].
pub const ODS_MIMETYPE: &str = "application/vnd.oasis.opendocument.spreadsheet";

/// Extraction limits of the tests: 2000 rows, 50 columns, 1 MiB of text, 20 images of at
/// most 2^26 pixels and 1 MiB in all.
pub fn limits() -> ExtractLimits {
    ExtractLimits {
        max_text_bytes: 1 << 20,
        sheet_rows: 2000,
        sheet_columns: 50,
        max_images: 20,
        max_image_pixels: 1 << 26,
        max_image_bytes: 1 << 20,
    }
}

/// [`oku_slides_extract::extract`], its worker side expected to succeed: the document's verdict.
pub fn extract(
    bytes: &[u8],
    kind: DocumentType,
    limits: &ExtractLimits,
    read: &ReadLimits,
    token: &str,
) -> Result<Extracted, ExtractRefusal> {
    oku_slides_extract::extract(bytes, kind, limits, read, token).unwrap_or_else(|e| panic!("worker: {e:?}"))
}

/// Read limits of the tests: the defaults.
pub fn read() -> ReadLimits {
    ReadLimits::default()
}

/// A zip of `(name, bytes, stored)` entries, in order.
pub fn zip_entries(entries: &[(&str, &[u8], bool)]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data, stored) in entries {
        let method = if *stored { CompressionMethod::Stored } else { CompressionMethod::Deflated };
        w.start_file(*name, SimpleFileOptions::default().compression_method(method))
            .unwrap_or_else(|e| panic!("zip: {e}"));
        w.write_all(data).unwrap_or_else(|e| panic!("zip: {e}"));
    }
    w.finish().unwrap_or_else(|e| panic!("zip: {e}")).into_inner()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let p = e.unwrap_or_else(|e| panic!("entry of {}: {e}", dir.display())).path();
        if p.file_name().is_some_and(|n| n == ".DS_Store") {
            continue;
        }
        if p.is_dir() { walk(&p, out) } else { out.push(p) }
    }
}

/// The fixture `fixtures/knowledge/<name>` (a directory) zipped, `mimetype` first and stored.
pub fn zip_dir(name: &str) -> Vec<u8> {
    zip_dir_with(name, &[])
}

/// [`zip_dir`] plus generated `extra` entries (images, never committed as binaries); an
/// extra entry replaces the fixture's entry of the same name.
pub fn zip_dir_with(name: &str, extra: &[(&str, &[u8])]) -> Vec<u8> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/knowledge").join(name);
    let mut files = Vec::new();
    walk(&dir, &mut files);
    files.sort_by_key(|p| (p.file_name() != Some("mimetype".as_ref()), p.clone()));
    let entries: Vec<(String, Vec<u8>)> = files
        .iter()
        .map(|f| {
            let rel = f.strip_prefix(&dir).unwrap_or_else(|e| panic!("{e}")).to_string_lossy().replace('\\', "/");
            (rel, fs::read(f).unwrap_or_else(|e| panic!("read {}: {e}", f.display())))
        })
        .collect();
    let mut refs: Vec<(&str, &[u8], bool)> = entries
        .iter()
        .filter(|(n, _)| !extra.iter().any(|(e, _)| e == n))
        .map(|(n, d)| (n.as_str(), d.as_slice(), n == "mimetype"))
        .collect();
    refs.extend(extra.iter().map(|(n, d)| (*n, *d, false)));
    zip_entries(&refs)
}

const WORKBOOK_RELS_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#;
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// One worksheet of [`xlsx_with`]: its name, its `state` (`visible`, `hidden`) and its
/// `<sheetData>` content.
pub struct Sheet<'a> {
    pub name: &'a str,
    pub state: &'a str,
    pub rows: String,
}

/// The XML of a worksheet whose `<sheetData>` holds `rows`, behind `head` (`<dimension>`…).
pub fn worksheet(head: &str, rows: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">{head}<sheetData>{rows}</sheetData></worksheet>"#
    )
}

/// A workbook of `sheets`, with an optional shared strings part and styles part (their XML).
pub fn xlsx_with(sheets: &[Sheet<'_>], shared: Option<&str>, styles: Option<&str>) -> Vec<u8> {
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    parts.push((
        "_rels/.rels".to_owned(),
        format!(r#"{WORKBOOK_RELS_HEAD}<Relationship Id="rId1" Type="{REL}/officeDocument" Target="xl/workbook.xml"/></Relationships>"#).into_bytes(),
    ));
    let mut list = String::new();
    let mut rels = String::from(WORKBOOK_RELS_HEAD);
    for (i, s) in sheets.iter().enumerate() {
        let n = i + 1;
        list.push_str(&format!(r#"<sheet name="{}" sheetId="{n}" state="{}" r:id="rId{n}"/>"#, s.name, s.state));
        rels.push_str(&format!(
            r#"<Relationship Id="rId{n}" Type="{REL}/worksheet" Target="worksheets/sheet{n}.xml"/>"#
        ));
        parts.push((format!("xl/worksheets/sheet{n}.xml"), worksheet("", &s.rows).into_bytes()));
    }
    if let Some(xml) = shared {
        rels.push_str(&format!(r#"<Relationship Id="rS" Type="{REL}/sharedStrings" Target="sharedStrings.xml"/>"#));
        parts.push(("xl/sharedStrings.xml".to_owned(), xml.as_bytes().to_vec()));
    }
    if let Some(xml) = styles {
        rels.push_str(&format!(r#"<Relationship Id="rT" Type="{REL}/styles" Target="styles.xml"/>"#));
        parts.push(("xl/styles.xml".to_owned(), xml.as_bytes().to_vec()));
    }
    rels.push_str("</Relationships>");
    parts.push(("xl/_rels/workbook.xml.rels".to_owned(), rels.into_bytes()));
    parts.push((
        "xl/workbook.xml".to_owned(),
        format!(r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="{REL}"><sheets>{list}</sheets></workbook>"#).into_bytes(),
    ));
    let entries: Vec<(&str, &[u8], bool)> = parts.iter().map(|(n, d)| (n.as_str(), d.as_slice(), false)).collect();
    zip_entries(&entries)
}

/// A one-sheet workbook of `rows` × `cols` numbers (the cell at row r, column c holds r·cols+c).
pub fn xlsx(rows: usize, cols: usize) -> Vec<u8> {
    let mut xml = String::new();
    for r in 0..rows {
        xml.push_str(&format!(r#"<row r="{}">"#, r + 1));
        for c in 0..cols {
            xml.push_str(&format!("<c><v>{}</v></c>", r * cols + c));
        }
        xml.push_str("</row>");
    }
    xlsx_with(&[Sheet { name: "Data", state: "visible", rows: xml }], None, None)
}

/// The `content.xml` of a spreadsheet whose `<office:spreadsheet>` holds `tables`, after the
/// automatic styles `styles`.
pub fn ods_content(styles: &str, tables: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3"><office:automatic-styles>{styles}</office:automatic-styles><office:body><office:spreadsheet>{tables}</office:spreadsheet></office:body></office:document-content>"#
    )
}

/// A spreadsheet with this `content.xml`, its `mimetype` stored or not.
pub fn ods_raw(content: &str, mimetype_stored: bool) -> Vec<u8> {
    zip_entries(&[("mimetype", ODS_MIMETYPE.as_bytes(), mimetype_stored), ("content.xml", content.as_bytes(), false)])
}

/// A spreadsheet with this `content.xml` and the common styles `styles` in its `styles.xml`.
pub fn ods_with_styles(content: &str, styles: &str) -> Vec<u8> {
    let styles = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3"><office:styles>{styles}</office:styles></office:document-styles>"#
    );
    zip_entries(&[
        ("mimetype", ODS_MIMETYPE.as_bytes(), true),
        ("content.xml", content.as_bytes(), false),
        ("styles.xml", styles.as_bytes(), false),
    ])
}

/// A well-formed spreadsheet of `tables` (`<table:table>` elements).
pub fn ods(tables: &str) -> Vec<u8> {
    ods_raw(&ods_content("", tables), true)
}

/// The header of a `w` × `h` PNG: what its type and size are read from.
pub fn png(w: u32, h: u32) -> Vec<u8> {
    let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    b.extend_from_slice(&w.to_be_bytes());
    b.extend_from_slice(&h.to_be_bytes());
    b.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    b
}

/// The header of an EMF picture: a type that is not imported.
pub fn emf() -> Vec<u8> {
    let mut b = vec![1, 0, 0, 0];
    b.resize(40, 0);
    b.extend_from_slice(b" EMF");
    b.resize(88, 0);
    b
}

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// A run holding the image of relationship `rel`, described as `alt`.
pub fn docx_image(rel: &str, alt: &str) -> String {
    format!(
        r#"<w:r><w:drawing><wp:inline><wp:docPr id="1" name="p" descr="{alt}"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:blipFill><a:blip r:embed="{rel}"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"#
    )
}

/// A Word document whose `<w:body>` holds `body`; `media` are its images, each `(name,
/// bytes)` behind relationship `rImg<i>` to `media/<name>`.
pub fn docx(body: &str, media: &[(&str, &[u8])]) -> Vec<u8> {
    docx_styled(body, media, None)
}

/// [`docx`] with `styles`, the `<w:style>` elements of its styles part, when given.
pub fn docx_styled(body: &str, media: &[(&str, &[u8])], styles: Option<&str>) -> Vec<u8> {
    let mut rels = String::from(WORKBOOK_RELS_HEAD);
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    if let Some(styles) = styles {
        rels.push_str(&format!(r#"<Relationship Id="rSty" Type="{REL}/styles" Target="styles.xml"/>"#));
        parts.push((
            "word/styles.xml".to_owned(),
            format!(r#"<?xml version="1.0" encoding="UTF-8"?><w:styles xmlns:w="{W}">{styles}</w:styles>"#)
                .into_bytes(),
        ));
    }
    for (i, (name, bytes)) in media.iter().enumerate() {
        rels.push_str(&format!(r#"<Relationship Id="rImg{i}" Type="{REL}/image" Target="media/{name}"/>"#));
        parts.push((format!("word/media/{name}"), bytes.to_vec()));
    }
    rels.push_str("</Relationships>");
    parts.push(("word/_rels/document.xml.rels".to_owned(), rels.into_bytes()));
    parts.push((
        "_rels/.rels".to_owned(),
        format!(r#"{WORKBOOK_RELS_HEAD}<Relationship Id="rId1" Type="{REL}/officeDocument" Target="word/document.xml"/></Relationships>"#).into_bytes(),
    ));
    parts.push((
        "word/document.xml".to_owned(),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="{W}" xmlns:r="{REL}" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body>{body}</w:body></w:document>"#
        )
        .into_bytes(),
    ));
    let entries: Vec<(&str, &[u8], bool)> = parts.iter().map(|(n, d)| (n.as_str(), d.as_slice(), false)).collect();
    zip_entries(&entries)
}

/// A paragraph of plain `text`.
pub fn para(text: &str) -> String {
    format!(r#"<w:p><w:r><w:t xml:space="preserve">{text}</w:t></w:r></w:p>"#)
}

/// One page of [`pdf_pages`].
#[derive(Debug, Clone, Copy)]
pub enum PageSpec<'a> {
    /// A line of text in Helvetica.
    Text(&'a str),
    /// A 4 × 3 `DCTDecode` image (a JPEG header) and no text.
    Jpeg,
    /// A Flate-coded 8-bit `DeviceRGB` image of `w` × `h` and no text; its data is real when
    /// `w` × `h` × 3 is at most 1 MiB, a few bytes otherwise.
    FlateRgb { w: i64, h: i64 },
    /// A 64 × 64 Flate-coded RGB image whose data inflates to 64 MiB, and no text.
    FlateBomb,
    /// No content at all.
    Blank,
    /// A line of text only inside a chain of `depth` form `XObject`s, the last one showing it
    /// in a font of its own resources (`F9`, unknown to the page).
    Forms { depth: usize, text: &'a str },
    /// A line of text, then a 2 × 2 inline image.
    InlineImage(&'a str),
    /// A Flate-coded content stream of `bytes` bytes of graphics operators, and no text.
    BigContent { bytes: usize },
    /// `Contents` referring to an object that does not exist.
    BrokenContents,
    /// A line of text, then a `Do` of an `XObject` whose entry refers to a missing object.
    BrokenXObject(&'a str),
    /// A line of text, then a `Tj` in font `F2`, whose entry refers to a missing object.
    BrokenFont(&'a str),
    /// A form whose Flate-coded content is `bytes` bytes of graphics operators, and no text.
    BigForm { bytes: usize },
    /// `bytes` bytes of code 1 in a font whose `ToUnicode` map turns code 1 into 256 `A`s (lopdf's most).
    Amplified { bytes: usize },
    /// This content stream as written, Helvetica as `F1`.
    Raw(&'a str),
    /// A line of text in a font of its own whose Flate-coded `ToUnicode` map decodes to
    /// `bytes` bytes, mostly blank lines.
    BigFont { bytes: usize },
}

/// Byte `i` of a [`PageSpec::FlateRgb`] image: runs of 16, so that it compresses.
pub fn pixel(i: i64) -> u8 {
    u8::try_from((i / 16) % 251).unwrap_or_else(|e| panic!("{e}"))
}

/// The header of a `w` × `h` baseline JPEG: what its type and size are read from.
pub fn jpeg(w: u16, h: u16) -> Vec<u8> {
    let mut b = vec![0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08];
    b.extend_from_slice(&h.to_be_bytes());
    b.extend_from_slice(&w.to_be_bytes());
    b.extend_from_slice(&[3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1, 0xFF, 0xD9]);
    b
}

fn image_stream(spec: PageSpec<'_>) -> Option<lopdf::Stream> {
    use lopdf::{Stream, dictionary};
    let dict = |w: i64, h: i64, filter: &str| {
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => w,
            "Height" => h,
            "ColorSpace" => "DeviceRGB",
            "BitsPerComponent" => 8,
            "Filter" => filter,
        }
    };
    let flate = |w: i64, h: i64, raw: Vec<u8>| {
        let mut s = Stream::new(lopdf::Dictionary::new(), raw);
        s.compress().unwrap_or_else(|e| panic!("compress: {e}"));
        assert!(s.dict.has(b"Filter"), "image data too random to compress");
        let mut d = dict(w, h, "FlateDecode");
        d.set("Length", i64::try_from(s.content.len()).unwrap_or_else(|e| panic!("{e}")));
        Stream { dict: d, ..s }
    };
    match spec {
        PageSpec::Jpeg => Some(Stream::new(dict(4, 3, "DCTDecode"), jpeg(4, 3))),
        PageSpec::FlateRgb { w, h } => {
            let real = w.checked_mul(h).and_then(|p| p.checked_mul(3)).filter(|n| (1..=1 << 20).contains(n));
            let raw = real.map_or_else(|| vec![0; 64], |n| (0..n).map(pixel).collect());
            Some(flate(w, h, raw))
        }
        PageSpec::FlateBomb => Some(flate(64, 64, vec![0; 64 << 20])),
        PageSpec::Text(_)
        | PageSpec::BrokenContents
        | PageSpec::BrokenXObject(_)
        | PageSpec::BrokenFont(_)
        | PageSpec::BigForm { .. }
        | PageSpec::Amplified { .. }
        | PageSpec::Raw(_)
        | PageSpec::Blank
        | PageSpec::Forms { .. }
        | PageSpec::InlineImage(_)
        | PageSpec::BigContent { .. }
        | PageSpec::BigFont { .. } => None,
    }
}

/// The operators showing `text` in font `font`, at the top left.
fn text_ops(font: &str, text: &str) -> Vec<lopdf::content::Operation> {
    use lopdf::Object;
    use lopdf::content::Operation;
    vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec![font.into(), 12.into()]),
        Operation::new("Td", vec![72.into(), 720.into()]),
        Operation::new("Tj", vec![Object::string_literal(text)]),
        Operation::new("ET", vec![]),
    ]
}

/// The operators drawing the `XObject` named `name`.
fn draw_ops(name: &str) -> Vec<lopdf::content::Operation> {
    use lopdf::content::Operation;
    vec![Operation::new("q", vec![]), Operation::new("Do", vec![name.into()]), Operation::new("Q", vec![])]
}

/// A form `XObject` of `ops` with `resources`, stored as object `id`.
fn put_form(
    doc: &mut lopdf::Document,
    id: lopdf::ObjectId,
    ops: Vec<lopdf::content::Operation>,
    resources: lopdf::Dictionary,
) {
    use lopdf::{Object, Stream, dictionary};
    let content = lopdf::content::Content { operations: ops }.encode().unwrap_or_else(|e| panic!("content: {e}"));
    let dict = dictionary! {
        "Type" => "XObject",
        "Subtype" => "Form",
        "BBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => resources,
    };
    doc.objects.insert(id, Object::Stream(Stream::new(dict, content)));
}

/// A chain of `depth` forms, each drawing the next as `Fm`, the last showing `text` in its
/// own font `F9`: the first form's id.
fn form_chain(doc: &mut lopdf::Document, depth: usize, text: &str) -> lopdf::ObjectId {
    use lopdf::dictionary;
    let font = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Courier" });
    let ids: Vec<lopdf::ObjectId> = (0..depth).map(|_| doc.new_object_id()).collect();
    for (i, &id) in ids.iter().enumerate() {
        match ids.get(i + 1) {
            Some(&next) => put_form(doc, id, draw_ops("Fm"), dictionary! { "XObject" => dictionary! { "Fm" => next } }),
            None => put_form(doc, id, text_ops("F9", text), dictionary! { "Font" => dictionary! { "F9" => font } }),
        }
    }
    ids[0]
}

/// "q Q" operators repeated to `bytes` bytes.
fn graphics_ops(bytes: usize) -> Vec<u8> {
    b"q Q\n".iter().copied().cycle().take(bytes).collect()
}

/// A font whose `ToUnicode` map turns code 1 into 256 `A`s (lopdf's most).
fn amplifying_font(doc: &mut lopdf::Document) -> lopdf::ObjectId {
    use lopdf::{Stream, dictionary};
    let cmap = format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CMapName /Amp def\n\
         1 begincodespacerange\n<00> <FF>\nendcodespacerange\n1 beginbfchar\n<01> <{}>\nendbfchar\n\
         endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n",
        "0041".repeat(256)
    );
    let map = doc.add_object(Stream::new(lopdf::Dictionary::new(), cmap.into_bytes()));
    doc.add_object(
        dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "ToUnicode" => map },
    )
}

/// A font whose Flate-coded `ToUnicode` map, padded with blank lines to `bytes` bytes, maps
/// the printable ASCII codes to themselves.
fn big_font(doc: &mut lopdf::Document, bytes: usize) -> lopdf::ObjectId {
    use lopdf::{Stream, dictionary};
    let head = "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CMapName /Big def\n\
                1 begincodespacerange\n<00> <FF>\nendcodespacerange\n1 beginbfrange\n<20> <7E> <0020>\n\
                endbfrange\n";
    let tail = "endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";
    let mut cmap = head.as_bytes().to_vec();
    cmap.resize(bytes.saturating_sub(tail.len()).max(cmap.len()), b'\n');
    cmap.extend_from_slice(tail.as_bytes());
    let mut map = Stream::new(lopdf::Dictionary::new(), cmap);
    map.compress().unwrap_or_else(|e| panic!("compress: {e}"));
    let map = doc.add_object(map);
    doc.add_object(
        dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "ToUnicode" => map },
    )
}

/// The page objects of `pages` in a new document, under the page tree node `parent`.
fn add_pages(doc: &mut lopdf::Document, parent: lopdf::ObjectId, pages: &[PageSpec<'_>]) -> Vec<lopdf::Object> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Stream, dictionary};
    let font = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
    let mut kids = Vec::new();
    for spec in pages {
        let mut ops = Vec::new();
        let mut xobjects = lopdf::Dictionary::new();
        let mut raw = Vec::new();
        let mut fonts = dictionary! { "F1" => font };
        let mut contents = None;
        match *spec {
            PageSpec::Text(text) => ops.extend(text_ops("F1", text)),
            PageSpec::Forms { depth, text } => {
                xobjects.set("Fm", form_chain(doc, depth, text));
                ops.extend(draw_ops("Fm"));
            }
            PageSpec::InlineImage(text) => {
                ops.extend(text_ops("F1", text));
                raw.extend_from_slice(b"\nBI /W 2 /H 2 /CS /G /BPC 8 ID\n\x01\x02\x03\x04\nEI\n");
            }
            PageSpec::BigContent { bytes } => raw = graphics_ops(bytes),
            PageSpec::BrokenContents => contents = Some(doc.new_object_id()),
            PageSpec::BrokenXObject(text) => {
                ops.extend(text_ops("F1", text));
                xobjects.set("Gone", doc.new_object_id());
                ops.extend(draw_ops("Gone"));
            }
            PageSpec::BrokenFont(text) => {
                ops.extend(text_ops("F1", text));
                fonts.set("F2", doc.new_object_id());
                ops.extend(text_ops("F2", "Lost words"));
            }
            PageSpec::BigForm { bytes } => {
                let mut form = Stream::new(lopdf::Dictionary::new(), graphics_ops(bytes));
                form.compress().unwrap_or_else(|e| panic!("compress: {e}"));
                form.dict.set("Type", "XObject");
                form.dict.set("Subtype", "Form");
                form.dict.set("BBox", vec![0.into(), 0.into(), 612.into(), 792.into()]);
                xobjects.set("Fm", doc.add_object(form));
                ops.extend(draw_ops("Fm"));
            }
            PageSpec::Amplified { bytes } => {
                fonts.set("F3", amplifying_font(doc));
                ops.push(Operation::new("BT", vec![]));
                ops.push(Operation::new("Tf", vec!["F3".into(), 12.into()]));
                ops.push(Operation::new(
                    "Tj",
                    vec![lopdf::Object::String(vec![1; bytes], lopdf::StringFormat::Hexadecimal)],
                ));
                ops.push(Operation::new("ET", vec![]));
            }
            PageSpec::Raw(content) => raw.extend_from_slice(content.as_bytes()),
            PageSpec::BigFont { bytes } => {
                fonts.set("F4", big_font(doc, bytes));
                ops.extend(text_ops("F4", "Big font words"));
            }
            PageSpec::Jpeg | PageSpec::FlateRgb { .. } | PageSpec::FlateBomb | PageSpec::Blank => {}
        }
        if let Some(image) = image_stream(*spec) {
            xobjects.set("Im1", doc.add_object(image));
            ops.push(Operation::new("q", vec![]));
            ops.push(Operation::new("cm", vec![100.into(), 0.into(), 0.into(), 100.into(), 72.into(), 72.into()]));
            ops.push(Operation::new("Do", vec!["Im1".into()]));
            ops.push(Operation::new("Q", vec![]));
        }
        let mut content = Content { operations: ops }.encode().unwrap_or_else(|e| panic!("content: {e}"));
        content.extend_from_slice(&raw);
        let mut stream = Stream::new(lopdf::Dictionary::new(), content);
        if matches!(spec, PageSpec::BigContent { .. }) {
            stream.compress().unwrap_or_else(|e| panic!("compress: {e}"));
        }
        let content = contents.unwrap_or_else(|| doc.add_object(stream));
        let page = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => parent,
            "Contents" => content,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! { "Font" => fonts, "XObject" => xobjects },
        });
        kids.push(page.into());
    }
    kids
}

/// A document of `pages` under one page tree node, with an Info `Title` when given.
fn pdf_document(pages: &[PageSpec<'_>], title: Option<&str>) -> lopdf::Document {
    use lopdf::{Object, dictionary};
    let mut doc = lopdf::Document::with_version("1.5");
    let tree = doc.new_object_id();
    let kids = add_pages(&mut doc, tree, pages);
    let count = i64::try_from(kids.len()).unwrap_or_else(|e| panic!("{e}"));
    doc.objects.insert(tree, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }));
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => tree });
    doc.trailer.set("Root", catalog);
    if let Some(title) = title {
        let info = doc.add_object(dictionary! { "Title" => lopdf::text_string(title) });
        doc.trailer.set("Info", info);
    }
    doc
}

fn save(mut doc: lopdf::Document) -> Vec<u8> {
    let mut out = Vec::new();
    doc.save_to(&mut out).unwrap_or_else(|e| panic!("save pdf: {e}"));
    out
}

/// A PDF of `pages`, in order.
pub fn pdf_pages(pages: &[PageSpec<'_>]) -> Vec<u8> {
    save(pdf_document(pages, None))
}

/// A PDF of `pages` whose Info dictionary has this `Title` (a UTF-16 text string).
pub fn pdf_titled(pages: &[PageSpec<'_>], title: &str) -> Vec<u8> {
    save(pdf_document(pages, Some(title)))
}

/// A PDF whose page tree loops: the root's second kid is a `Pages` node whose kids are the
/// root itself and a text page.
pub fn pdf_kids_cycle() -> Vec<u8> {
    use lopdf::{Object, dictionary};
    let mut doc = pdf_document(&[PageSpec::Text("Before the loop")], None);
    let root = doc
        .catalog()
        .and_then(|c| c.get(b"Pages"))
        .and_then(Object::as_reference)
        .unwrap_or_else(|e| panic!("pages: {e}"));
    let node = doc.new_object_id();
    let looped = add_pages(&mut doc, node, &[PageSpec::Text("Inside the loop")]);
    let mut node_kids = vec![Object::Reference(root)];
    node_kids.extend(looped);
    doc.objects.insert(node, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => node_kids, "Count" => 2 }));
    let tree = doc.get_dictionary_mut(root).unwrap_or_else(|e| panic!("root: {e}"));
    let kids = tree.get_mut(b"Kids").and_then(Object::as_array_mut).unwrap_or_else(|e| panic!("kids: {e}"));
    kids.push(Object::Reference(node));
    save(doc)
}

/// A one-page text PDF encrypted (RC4, 128 bits) with a user password.
pub fn pdf_encrypted() -> Vec<u8> {
    let mut doc = pdf_document(&[PageSpec::Text("Secret page")], None);
    doc.trailer.set(
        "ID",
        vec![
            lopdf::Object::string_literal(b"0123456789abcdef".to_vec()),
            lopdf::Object::string_literal(b"0123456789abcdef".to_vec()),
        ],
    );
    let version = lopdf::EncryptionVersion::V2 {
        document: &doc,
        owner_password: "owner",
        user_password: "user",
        key_length: 128,
        permissions: lopdf::Permissions::all(),
    };
    let state = lopdf::EncryptionState::try_from(version).unwrap_or_else(|e| panic!("encryption: {e}"));
    doc.encrypt(&state).unwrap_or_else(|e| panic!("encrypt: {e}"));
    save(doc)
}

/// The root page tree node of `doc`.
fn tree_root(doc: &lopdf::Document) -> lopdf::ObjectId {
    doc.catalog()
        .and_then(|c| c.get(b"Pages"))
        .and_then(lopdf::Object::as_reference)
        .unwrap_or_else(|e| panic!("pages: {e}"))
}

/// A one-page PDF whose page draws form `Fm1`, which shows text and draws `Fm2`, which
/// draws `Fm1` again.
pub fn pdf_form_cycle() -> Vec<u8> {
    use lopdf::{Object, dictionary};
    let mut doc = pdf_document(&[PageSpec::Blank], None);
    let page = *doc.get_pages().values().next().unwrap_or_else(|| panic!("no page"));
    let font = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Courier" });
    let (one, two) = (doc.new_object_id(), doc.new_object_id());
    let mut ops = text_ops("F9", "Looping form");
    ops.extend(draw_ops("Fm"));
    put_form(
        &mut doc,
        one,
        ops,
        dictionary! { "Font" => dictionary! { "F9" => font }, "XObject" => dictionary! { "Fm" => two } },
    );
    put_form(&mut doc, two, draw_ops("Fm"), dictionary! { "XObject" => dictionary! { "Fm" => one } });
    let content = doc.add_object(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        lopdf::content::Content { operations: draw_ops("Fm") }.encode().unwrap_or_else(|e| panic!("{e}")),
    ));
    let page = doc.get_dictionary_mut(page).unwrap_or_else(|e| panic!("page: {e}"));
    page.set("Contents", content);
    page.set("Resources", dictionary! { "XObject" => dictionary! { "Fm" => Object::Reference(one) } });
    save(doc)
}

/// A PDF of `pages` pages each drawing the one form `Fm`, which shows `text` in its own font
/// then runs `pad` bytes of graphics operators.
pub fn pdf_shared_form(pages: usize, pad: usize, text: &str) -> Vec<u8> {
    use lopdf::{Object, dictionary};
    let mut doc = pdf_document(&vec![PageSpec::Blank; pages], None);
    let font = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Courier" });
    let form = doc.new_object_id();
    let mut content =
        lopdf::content::Content { operations: text_ops("F9", text) }.encode().unwrap_or_else(|e| panic!("{e}"));
    content.extend(graphics_ops(pad));
    let dict = dictionary! {
        "Type" => "XObject",
        "Subtype" => "Form",
        "BBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F9" => font } },
    };
    doc.objects.insert(form, Object::Stream(lopdf::Stream::new(dict, content)));
    let draw = doc.add_object(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        lopdf::content::Content { operations: draw_ops("Fm") }.encode().unwrap_or_else(|e| panic!("{e}")),
    ));
    let ids: Vec<lopdf::ObjectId> = doc.get_pages().into_values().collect();
    for id in ids {
        let page = doc.get_dictionary_mut(id).unwrap_or_else(|e| panic!("page: {e}"));
        page.set("Contents", draw);
        page.set("Resources", dictionary! { "XObject" => dictionary! { "Fm" => Object::Reference(form) } });
    }
    save(doc)
}

/// A PDF whose root lists the same `Pages` node twice, that node holding one text page.
pub fn pdf_shared_subtree() -> Vec<u8> {
    use lopdf::{Object, dictionary};
    let mut doc = pdf_document(&[], None);
    let root = tree_root(&doc);
    let node = doc.new_object_id();
    let kids = add_pages(&mut doc, node, &[PageSpec::Text("Shared page")]);
    doc.objects.insert(node, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => 1 }));
    let tree = doc.get_dictionary_mut(root).unwrap_or_else(|e| panic!("root: {e}"));
    tree.set("Kids", vec![Object::Reference(node), Object::Reference(node)]);
    tree.set("Count", 2);
    save(doc)
}

/// A one-page text PDF whose page tree root names the page as its `Parent`: a loop on the
/// way up to inherited resources.
pub fn pdf_parent_cycle() -> Vec<u8> {
    let mut doc = pdf_document(&[PageSpec::Text("Looped parent")], None);
    let root = tree_root(&doc);
    let page = *doc.get_pages().values().next().unwrap_or_else(|| panic!("no page"));
    doc.get_dictionary_mut(root).unwrap_or_else(|e| panic!("root: {e}")).set("Parent", page);
    save(doc)
}

/// A one-page text PDF whose cross-reference stream's `Prev` points at itself.
pub fn pdf_prev_loop() -> Vec<u8> {
    let bytes = pdf_pages(&[PageSpec::Text("Prev loop")]);
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let start = text.rfind("startxref").unwrap_or_else(|| panic!("no startxref"));
    let offset: String = text[start + 9..].trim_start().chars().take_while(char::is_ascii_digit).collect();
    let at = bytes.windows(10).rposition(|w| w == b"/Type/XRef").unwrap_or_else(|| panic!("no xref stream"));
    let mut out = bytes[..at].to_vec();
    out.extend_from_slice(format!("/Prev {offset}").as_bytes());
    // The startxref offset still points at the xref stream, before the inserted bytes.
    out.extend_from_slice(&bytes[at..]);
    out
}

/// 65 MiB of zeros, Flate-coded: past the default 64 MiB part limit.
fn zeros_bomb() -> Vec<u8> {
    let mut s = lopdf::Stream::new(lopdf::Dictionary::new(), vec![0; 65 << 20]);
    s.compress().unwrap_or_else(|e| panic!("compress: {e}"));
    s.content
}

/// A one-page text PDF holding an object stream that inflates to 65 MiB.
pub fn pdf_objstm_bomb() -> Vec<u8> {
    use lopdf::dictionary;
    let mut doc = pdf_document(&[PageSpec::Text("Beside a bomb")], None);
    let dict = dictionary! { "Type" => "ObjStm", "N" => 1, "First" => 4, "Filter" => "FlateDecode" };
    doc.add_object(lopdf::Stream::new(dict, zeros_bomb()).with_compression(false));
    save(doc)
}

/// A PDF whose only cross-reference stream inflates to 65 MiB.
pub fn pdf_xref_stream_bomb() -> Vec<u8> {
    let bomb = zeros_bomb();
    let mut out = b"%PDF-1.5\n".to_vec();
    let at = out.len();
    out.extend_from_slice(
        format!(
            "1 0 obj\n<< /Type /XRef /Size 2 /W [1 2 1] /Root 1 0 R /Filter /FlateDecode /Length {} >>\nstream\n",
            bomb.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&bomb);
    out.extend_from_slice(format!("\nendstream\nendobj\nstartxref\n{at}\n%%EOF\n").as_bytes());
    out
}
