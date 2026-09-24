use lopdf::{Document, Object, dictionary};

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
    doc.save_to(&mut out).unwrap();
    out
}

#[test]
fn ecrire_et_relire_le_blob() {
    let pdf = minimal_pdf();
    assert!(slider_pdf::extract_sources(&pdf).unwrap().is_none());
    let blob = b"PK zip factice".to_vec();
    let out = slider_pdf::embed_sources(&pdf, &blob).unwrap();
    let s = slider_pdf::extract_sources(&out).unwrap().unwrap();
    assert_eq!(s.zip, blob);
    assert_eq!(s.version, 1);
    assert!(!s.modified_elsewhere());
}

#[test]
fn un_blob_altere_est_detecte() {
    let out = slider_pdf::embed_sources(&minimal_pdf(), b"AAAAAAAAAAAAAAAAAAAA").unwrap();
    let pos = out.windows(20).position(|w| w == b"AAAAAAAAAAAAAAAAAAAA").unwrap();
    let mut bad = out.clone();
    bad[pos] = b'B';
    assert!(matches!(slider_pdf::extract_sources(&bad), Err(slider_pdf::PdfError::Corrupted)));
}

#[test]
fn une_modification_externe_est_signalee() {
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
    assert!(slider_pdf::extract_sources(&edited).unwrap().unwrap().modified_elsewhere());
}
