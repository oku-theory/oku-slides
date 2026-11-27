//! Documents and bounds for the extraction tests.
//!
//! Self-contained (only `oku_slides_core`, `oku_slides_import` and `zip`): the integration
//! tests include it by `#[path]`.

use std::fmt::Write as _;
use std::io::{Cursor, Write};

use oku_slides_core::knowledge::ExtractLimits;
use oku_slides_import::ReadLimits;
use zip::write::SimpleFileOptions;

/// The placeholder token of the tests' image citations.
pub(crate) const TOKEN: &str = "0123456789abcdef";

const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const RELS_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#;
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// Extraction limits of the tests: 1 MiB of text, 20 images of at most 2^26 pixels and 1 MiB in all.
pub(crate) fn limits() -> ExtractLimits {
    ExtractLimits {
        max_text_bytes: 1 << 20,
        sheet_rows: 2000,
        sheet_columns: 50,
        max_images: 20,
        max_image_pixels: 1 << 26,
        max_image_bytes: 1 << 20,
    }
}

/// Read limits of the tests: a document of at most 64 MiB.
pub(crate) fn read() -> ReadLimits {
    ReadLimits::for_extract(64 << 20, 4096 << 20)
}

/// The header of a `w` × `h` PNG: enough for the extractor, which reads only its header.
pub(crate) fn png(w: u32, h: u32) -> Vec<u8> {
    let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    b.extend_from_slice(&w.to_be_bytes());
    b.extend_from_slice(&h.to_be_bytes());
    b.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    b
}

/// A run holding the image of relationship `rel`, described as `alt`.
fn image_run(rel: &str, alt: &str) -> String {
    format!(
        r#"<w:p><w:r><w:drawing><wp:inline><wp:docPr id="1" name="p" descr="{alt}"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:blipFill><a:blip r:embed="{rel}"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>"#
    )
}

/// A Word document of a paragraph `Hello` and two images: a 2 × 2 PNG, then a 3 × 1 PNG.
pub(crate) fn docx_two_images() -> Vec<u8> {
    let images = [png(2, 2), png(3, 1)];
    let mut rels = String::from(RELS_HEAD);
    let mut body = String::from(r#"<w:p><w:r><w:t xml:space="preserve">Hello</w:t></w:r></w:p>"#);
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    for (i, bytes) in images.iter().enumerate() {
        write!(rels, r#"<Relationship Id="rImg{i}" Type="{REL}/image" Target="media/image{i}.png"/>"#)
            .expect("writing to a String");
        body.push_str(&image_run(&format!("rImg{i}"), &format!("picture {i}")));
        parts.push((format!("word/media/image{i}.png"), bytes.clone()));
    }
    rels.push_str("</Relationships>");
    parts.push(("word/_rels/document.xml.rels".to_owned(), rels.into_bytes()));
    parts.push((
        "_rels/.rels".to_owned(),
        format!(r#"{RELS_HEAD}<Relationship Id="rId1" Type="{REL}/officeDocument" Target="word/document.xml"/></Relationships>"#)
            .into_bytes(),
    ));
    parts.push((
        "word/document.xml".to_owned(),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="{W}" xmlns:r="{REL}" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body>{body}</w:body></w:document>"#
        )
        .into_bytes(),
    ));
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in &parts {
        w.start_file(name.as_str(), SimpleFileOptions::default()).expect("zip entry");
        w.write_all(data).expect("zip write");
    }
    w.finish().expect("zip finish").into_inner()
}
