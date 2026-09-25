use lopdf::{Document, Object, dictionary};
use speculoos::prelude::*;

fn minimal_pdf() -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages_id, "MediaBox" => vec![0.into(), 0.into(), 100.into(), 100.into()] });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog);
    let mut out = Vec::new();
    doc.save_to(&mut out).unwrap_or_else(|e| panic!("PDF minimal impossible à écrire : {e}"));
    out
}

#[test]
fn extract_sources_without_blob_returns_none() {
    let pdf = minimal_pdf();
    assert_that!(slider_pdf::extract_sources(&pdf).unwrap()).is_none();
}

#[test]
fn embed_sources_then_extract_sources_returns_same_blob() {
    let pdf = minimal_pdf();
    let blob = b"PK zip factice".to_vec();
    let out = slider_pdf::embed_sources(&pdf, &blob).unwrap();
    let s = slider_pdf::extract_sources(&out).unwrap().unwrap();
    assert_that!(s.zip).is_equal_to(&blob);
    assert_that!(s.version).is_equal_to(1);
    assert_that!(s.modified_elsewhere()).is_false();
}

#[test]
fn extract_sources_with_altered_blob_returns_corrupted() {
    let out = slider_pdf::embed_sources(&minimal_pdf(), b"AAAAAAAAAAAAAAAAAAAA").unwrap();
    let pos = out.windows(20).position(|w| w == b"AAAAAAAAAAAAAAAAAAAA").unwrap();
    let mut bad = out.clone();
    bad[pos] = b'B';
    assert_that!(slider_pdf::extract_sources(&bad).unwrap_err().kind())
        .is_equal_to(slider_pdf::PdfErrorKind::Corrupted);
}

#[test]
fn extract_sources_after_external_edit_reports_modified_elsewhere() {
    let out = slider_pdf::embed_sources(&minimal_pdf(), b"zip").unwrap();
    let mut doc = Document::load_mem(&out).unwrap();
    let info = doc.trailer.get(b"Info").unwrap().as_reference().unwrap();
    doc.get_object_mut(info)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("ModDate", Object::string_literal("D:20990101000000Z"));
    let mut edited = Vec::new();
    doc.save_to(&mut edited).unwrap();
    assert_that!(slider_pdf::extract_sources(&edited).unwrap().unwrap().modified_elsewhere()).is_true();
}

#[test]
fn extract_sources_blob_under_other_key_returns_none() {
    // Seule la clé /Echo est lue : un blob sous /Slider n'est pas des sources Écho.
    let out = slider_pdf::embed_sources(&minimal_pdf(), b"zip").unwrap();
    let mut doc = Document::load_mem(&out).unwrap();
    let root = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    let catalog = doc.get_object_mut(root).unwrap().as_dict_mut().unwrap();
    let mut pieces = catalog.get(b"PieceInfo").unwrap().as_dict().unwrap().clone();
    let piece = pieces.remove(b"Echo").unwrap();
    pieces.set("Slider", piece);
    catalog.set("PieceInfo", Object::Dictionary(pieces));
    let mut legacy = Vec::new();
    doc.save_to(&mut legacy).unwrap();
    assert_that!(slider_pdf::extract_sources(&legacy).unwrap()).is_none();
}
