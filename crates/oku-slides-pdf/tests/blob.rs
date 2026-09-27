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
    assert_that!(oku_slides_pdf::extract_sources(&pdf).unwrap()).is_none();
}

#[test]
fn embed_sources_then_extract_sources_returns_same_blob() {
    let pdf = minimal_pdf();
    let blob = b"PK zip factice".to_vec();
    let out = oku_slides_pdf::embed_sources(&pdf, &blob).unwrap();
    let s = oku_slides_pdf::extract_sources(&out).unwrap().unwrap();
    assert_that!(s.zip).is_equal_to(&blob);
    assert_that!(s.version).is_equal_to(2);
    assert_that!(s.modified_elsewhere()).is_false();
}

#[test]
fn extract_sources_with_altered_blob_returns_corrupted() {
    let out = oku_slides_pdf::embed_sources(&minimal_pdf(), b"AAAAAAAAAAAAAAAAAAAA").unwrap();
    let pos = out.windows(20).position(|w| w == b"AAAAAAAAAAAAAAAAAAAA").unwrap();
    let mut bad = out.clone();
    bad[pos] = b'B';
    assert_that!(oku_slides_pdf::extract_sources(&bad).unwrap_err().kind())
        .is_equal_to(oku_slides_pdf::PdfErrorKind::Corrupted);
}

#[test]
fn extract_sources_after_external_edit_reports_modified_elsewhere() {
    let out = oku_slides_pdf::embed_sources(&minimal_pdf(), b"zip").unwrap();
    let mut doc = Document::load_mem(&out).unwrap();
    let info = doc.trailer.get(b"Info").unwrap().as_reference().unwrap();
    doc.get_object_mut(info)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("ModDate", Object::string_literal("D:20990101000000Z"));
    let mut edited = Vec::new();
    doc.save_to(&mut edited).unwrap();
    assert_that!(oku_slides_pdf::extract_sources(&edited).unwrap().unwrap().modified_elsewhere()).is_true();
}

#[test]
fn extract_sources_blob_under_other_key_returns_none() {
    // Seule la clé /OkuSlides est lue : un blob sous /Echo (ancien nom) n'est pas des sources.
    let out = oku_slides_pdf::embed_sources(&minimal_pdf(), b"zip").unwrap();
    let mut doc = Document::load_mem(&out).unwrap();
    let root = doc.trailer.get(b"Root").unwrap().as_reference().unwrap();
    let catalog = doc.get_object_mut(root).unwrap().as_dict_mut().unwrap();
    let mut pieces = catalog.get(b"PieceInfo").unwrap().as_dict().unwrap().clone();
    let piece = pieces.remove(b"OkuSlides").unwrap();
    pieces.set("Echo", piece);
    catalog.set("PieceInfo", Object::Dictionary(pieces));
    let mut legacy = Vec::new();
    doc.save_to(&mut legacy).unwrap();
    assert_that!(oku_slides_pdf::extract_sources(&legacy).unwrap()).is_none();
}

/// Le même PDF, avec `/Version` réécrit dans l'enveloppe.
fn with_version(pdf: &[u8], version: i64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut doc = Document::load_mem(pdf)?;
    let root = doc.trailer.get(b"Root")?.as_reference()?;
    let mut piece = doc.get_object(root)?.as_dict()?.get(b"PieceInfo")?.as_dict()?.clone();
    let mut oku = piece.get(b"OkuSlides")?.as_dict()?.clone();
    let mut private = oku.get(b"Private")?.as_dict()?.clone();
    private.set("Version", Object::Integer(version));
    oku.set("Private", Object::Dictionary(private));
    piece.set("OkuSlides", Object::Dictionary(oku));
    doc.get_object_mut(root)?.as_dict_mut()?.set("PieceInfo", Object::Dictionary(piece));
    let mut out = Vec::new();
    doc.save_to(&mut out)?;
    Ok(out)
}

#[test]
fn extract_sources_version_1_returns_none() {
    let out = with_version(&oku_slides_pdf::embed_sources(&minimal_pdf(), b"zip").unwrap(), 1).unwrap();
    assert_that!(oku_slides_pdf::extract_sources(&out).unwrap()).is_none();
}

#[test]
fn extract_sources_version_3_returns_unsupported_version() {
    let out = with_version(&oku_slides_pdf::embed_sources(&minimal_pdf(), b"zip").unwrap(), 3).unwrap();
    assert_that!(oku_slides_pdf::extract_sources(&out).unwrap_err().kind())
        .is_equal_to(oku_slides_pdf::PdfErrorKind::UnsupportedVersion);
}
