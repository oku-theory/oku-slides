use std::io::{Cursor, Write};

use speculoos::prelude::*;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use super::*;

fn package(first: &str, data: &[u8], method: CompressionMethod) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    w.start_file(first, SimpleFileOptions::default().compression_method(method)).unwrap();
    w.write_all(data).unwrap();
    w.finish().unwrap().into_inner()
}

#[test]
fn mimetype_first_stored_ods_type_accepted() {
    assert_that!(mimetype_first(&package("mimetype", MIMETYPE, CompressionMethod::Stored))).is_true();
}

#[test]
fn mimetype_first_compressed_wrong_name_or_type_refused() {
    assert_that!(mimetype_first(&package("mimetype", MIMETYPE, CompressionMethod::Deflated))).is_false();
    assert_that!(mimetype_first(&package("mimetypf", MIMETYPE, CompressionMethod::Stored))).is_false();
    let text = b"application/vnd.oasis.opendocument.text";
    assert_that!(mimetype_first(&package("mimetype", text, CompressionMethod::Stored))).is_false();
    assert_that!(mimetype_first(b"PK")).is_false();
}

#[test]
fn value_typed_attribute_then_paragraphs() {
    let xml = r#"<t xmlns:table="t" xmlns:office="o" xmlns:text="x">
        <table:table-cell office:value-type="float" office:value="3.5"><text:p>3,50</text:p></table:table-cell>
        <table:table-cell office:value-type="date" office:date-value="2024-01-02"><text:p>02/01</text:p></table:table-cell>
        <table:table-cell office:value-type="string"><text:p>a<text:s text:c="3"/>b<text:tab/>c</text:p><text:p>d<office:annotation><text:p>note</text:p></office:annotation></text:p></table:table-cell>
    </t>"#;
    let doc = roxmltree::Document::parse(xml).unwrap();
    let values: Vec<String> =
        doc.root_element().children().filter(Node::is_element).map(|n| value(n).into_owned()).collect();
    assert_that!(values).is_equal_to(vec!["3.5".to_owned(), "2024-01-02".to_owned(), "a   b c\nd".to_owned()]);
}
