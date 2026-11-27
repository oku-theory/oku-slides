use oku_slides_core::Op;
use oku_slides_core::knowledge::DocumentType;
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use crate::agent::Upload;

use super::AttachmentRef;
use crate::pack::ESSENTIEL_PACK;
use crate::test_support::{png, session};
use crate::{Session, SessionErrorKind};

/// A minimal PDF header, enough for the magic sniff.
const PDF: &[u8] = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n1 0 obj\n<<>>\nendobj\n";

/// The id an attachment got, whatever its kind.
fn id_of(r: &AttachmentRef) -> String {
    r.id().to_owned()
}

#[test]
fn attach_image_png_returns_its_asset_id_type_and_size() {
    let mut s = session();
    let bytes = png(3, 2, 0);
    let r = s.attach(Upload::new(bytes.clone(), "image.png"), &[]).unwrap();
    let expected =
        AttachmentRef::Image { id: oku_slides_core::asset_id(&bytes), mime: "image/png".into(), width: 3, height: 2 };
    assert_that!(r).is_equal_to(expected);
}

#[test]
fn attach_image_unchanged_without_a_name() {
    let mut s = session();
    let r = s.attach(Upload::new(png(3, 2, 10), ""), &[]).unwrap();
    assert_that!((matches!(r, AttachmentRef::Image { .. }), r.name())).is_equal_to((true, None));
}

#[test]
fn attach_image_twice_keeps_one_copy() {
    let mut s = session();
    let bytes = png(2, 2, 1);
    let a = s.attach(Upload::new(bytes.clone(), ""), &[]).unwrap();
    let b = s.attach(Upload::new(bytes, ""), &[]).unwrap();
    assert_that!(a).is_equal_to(b);
}

#[test]
fn attach_same_bytes_second_name_keeps_first() {
    let mut s = session();
    let first = s.attach(Upload::new(PDF.to_vec(), "Rapport.pdf"), &[]).unwrap();
    let second = s.attach(Upload::new(PDF.to_vec(), "Autre nom.pdf"), &[]).unwrap();
    assert_that!(second.name()).is_equal_to(Some("Rapport.pdf"));
    assert_that!(second).is_equal_to(first);
}

#[rstest]
#[case::image(png(2, 2, 5), "", true, false)]
#[case::document(PDF.to_vec(), "a.pdf", false, true)]
#[case::mismatch(PDF.to_vec(), "a.md", false, false)]
#[case::svg(b"<svg xmlns='http://www.w3.org/2000/svg'/>".to_vec(), "a.svg", false, false)]
fn upload_hashes_and_sniffs_once_at_creation(
    #[case] bytes: Vec<u8>,
    #[case] name: &str,
    #[case] image: bool,
    #[case] document: bool,
) {
    let upload = Upload::new(bytes.clone(), name);
    assert_that!(upload.id()).is_equal_to(oku_slides_core::asset_id(&bytes).as_str());
    assert_that!((upload.is_image(), upload.is_document())).is_equal_to((image, document));
}

#[test]
fn attached_gives_back_the_bytes() {
    let mut s = session();
    let bytes = png(2, 2, 2);
    let r = s.attach(Upload::new(bytes.clone(), ""), &[]).unwrap();
    let (_, kept) = s.attached(r.id()).unwrap();
    assert_that!(kept.to_vec()).is_equal_to(bytes);
}

#[test]
fn attach_pdf_returns_document_ref() {
    let mut s = session();
    let r = s.attach(Upload::new(PDF.to_vec(), "Rapport annuel.pdf"), &[]).unwrap();
    let expected = AttachmentRef::Document {
        id: oku_slides_core::asset_id(PDF),
        name: "Rapport annuel.pdf".into(),
        kind: DocumentType::Pdf,
        bytes: oku_slides_core::num::count_u64(PDF.len()),
    };
    assert_that!(r).is_equal_to(expected);
}

#[test]
fn attach_document_ref_serializes_tagged() {
    let r =
        AttachmentRef::Document { id: "sha256:ab".into(), name: "a.md".into(), kind: DocumentType::Markdown, bytes: 3 };
    let value = serde_json::to_value(&r).unwrap();
    assert_that!(value).is_equal_to(json!({
        "type": "document", "id": "sha256:ab", "name": "a.md", "kind": "markdown", "bytes": 3
    }));
}

#[rstest]
#[case::svg(b"<svg xmlns='http://www.w3.org/2000/svg'/>".to_vec(), "a.svg", "session-paste-unsupported")]
#[case::bmp(b"BM\x36\0\0\0\0\0\0\0".to_vec(), "a.bmp", "session-paste-unsupported")]
#[case::zip(b"PK\x03\x04rest".to_vec(), "a.zip", "session-paste-unsupported")]
#[case::truncated_png(b"\x89PNG\r\n\x1a\n\0\0".to_vec(), "a.png", "session-paste-unreadable")]
#[case::legacy(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1rest".to_vec(), "a.ppt", "session-attach-legacy")]
#[case::mismatch(b"bonjour".to_vec(), "a.pdf", "session-attach-type-mismatch")]
fn attach_refused_bytes_say_why(#[case] bytes: Vec<u8>, #[case] name: &str, #[case] key: &str) {
    let mut s = session();
    let e = s.attach(Upload::new(bytes, name), &[]).unwrap_err();
    assert_that!((e.kind(), e.paste_reason_key())).is_equal_to((SessionErrorKind::Paste, Some(key)));
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr(key));
}

#[test]
fn attach_text_document_is_taken_as_text() {
    let mut s = session();
    let r = s.attach(Upload::new(b"bonjour".to_vec(), "notes.txt"), &[]).unwrap();
    assert_that!(matches!(r, AttachmentRef::Document { kind: DocumentType::Text, .. })).is_true();
}

#[test]
fn attach_image_over_the_byte_limit_is_too_large() {
    let (_data, settings) = paste_settings(|p| p.max_image_mib = 0);
    let mut s = crate::test_support::session_with(settings);
    let e = s.attach(Upload::new(png(2, 2, 3), ""), &[]).unwrap_err();
    assert_that!(e.paste_reason_key()).is_equal_to(Some("session-paste-too-large"));
}

#[test]
fn attach_image_over_the_pixel_limit_has_too_many_pixels() {
    let (_data, settings) = paste_settings(|p| p.max_image_pixels = 3);
    let mut s = crate::test_support::session_with(settings);
    let e = s.attach(Upload::new(png(2, 2, 4), ""), &[]).unwrap_err();
    assert_that!(e.paste_reason_key()).is_equal_to(Some("session-paste-too-many-pixels"));
}

#[test]
fn snapshot_attach_limits_follow_the_paste_settings() {
    let (_data, settings) = paste_settings(|p| {
        p.max_image_mib = 3;
        p.max_draft_images = 5;
        p.max_document_mib = std::num::NonZeroUsize::new(99).unwrap();
        p.max_draft_documents = std::num::NonZeroUsize::new(2).unwrap();
    });
    let s = crate::test_support::session_with(settings);
    // The document size stays within its ceiling (47 MiB), as the host checks it.
    let expected =
        super::AttachLimits { image_bytes: 3 << 20, document_bytes: 47 << 20, draft_images: 5, draft_documents: 2 };
    assert_that!(s.snapshot().attach_limits).is_equal_to(expected);
}

#[test]
fn attach_document_past_limit_refused() {
    let (_data, settings) = paste_settings(|p| p.max_document_mib = std::num::NonZeroUsize::MIN);
    let mut s = crate::test_support::session_with(settings);
    let mut pdf = PDF.to_vec();
    pdf.resize((1 << 20) + 1, b' ');
    let e = s.attach(Upload::new(pdf, "a.pdf"), &[]).unwrap_err();
    assert_that!(e.paste_reason_key()).is_equal_to(Some("session-attach-document-too-large"));
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("session-attach-document-too-large"));
}

#[test]
fn attach_document_in_pack_tab_refused() {
    let data = crate::test_support::fresh_dir("oku-slides-paste-");
    let mut s = crate::test_support::session_on(data.path());
    s.open_pack_tab_here(ESSENTIEL_PACK, None).unwrap();
    let e = s.attach(Upload::new(PDF.to_vec(), "a.pdf"), &[]).unwrap_err();
    assert_that!(e.paste_reason_key()).is_equal_to(Some("session-attach-document-deck-only"));
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("session-attach-document-deck-only"));
}

#[test]
fn attach_image_in_pack_tab_accepted() {
    let data = crate::test_support::fresh_dir("oku-slides-paste-");
    let mut s = crate::test_support::session_on(data.path());
    s.open_pack_tab_here(ESSENTIEL_PACK, None).unwrap();
    assert_that!(s.attach(Upload::new(png(2, 2, 11), ""), &[])).is_ok();
}

#[rstest]
#[case::unix("/Users/x/Documents/plan.md", "plan.md")]
#[case::windows("C:\\Users\\x\\plan.md", "plan.md")]
#[case::hidden_control("pl\u{202E}an.md", "plan.md")]
#[case::line_control("pl\nan.md", "pl an.md")]
fn attach_name_with_path_keeps_last_segment(#[case] name: &str, #[case] kept: &str) {
    let mut s = session();
    let r = s.attach(Upload::new(b"# Plan\n".to_vec(), name), &[]).unwrap();
    assert_that!(r.name()).is_equal_to(Some(kept));
}

#[rstest]
#[case::empty("")]
#[case::trailing_slash("notes/")]
#[case::blank("   ")]
fn attach_empty_name_refused(#[case] name: &str) {
    let mut s = session();
    let e = s.attach(Upload::new(b"# Plan\n".to_vec(), name), &[]).unwrap_err();
    assert_that!(e.paste_reason_key()).is_equal_to(Some("session-attach-name-empty"));
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("session-attach-name-empty"));
}

/// A text document of `kib` KiB, distinct per `seed`.
fn text_doc(kib: usize, seed: char) -> Vec<u8> {
    seed.to_string().repeat(kib * 1024).into_bytes()
}

#[test]
fn attach_documents_past_kept_with_draft_refused() {
    let (_data, settings) = paste_settings(|p| p.max_kept_mib = 1);
    let mut s = crate::test_support::session_with(settings);
    let a = s.attach(Upload::new(text_doc(600, 'a'), "a.txt"), &[]).unwrap();
    let e = s.attach(Upload::new(text_doc(600, 'b'), "b.txt"), &[id_of(&a)]).unwrap_err();
    assert_that!(e.paste_reason_key()).is_equal_to(Some("session-attach-kept-full"));
    assert_that!(s.attached(a.id())).is_some();
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("session-attach-kept-full"));
}

#[test]
fn attach_document_evicted_like_images() {
    let (_data, settings) = paste_settings(|p| p.max_kept_mib = 1);
    let mut s = crate::test_support::session_with(settings);
    let old = s.attach(Upload::new(text_doc(400, 'a'), "a.txt"), &[]).unwrap();
    let kept = s.attach(Upload::new(text_doc(400, 'b'), "b.txt"), &[]).unwrap();
    let fresh = s.attach(Upload::new(text_doc(400, 'c'), "c.txt"), &[id_of(&kept)]).unwrap();
    let held = (s.attached(old.id()).is_none(), s.attached(kept.id()).is_some(), s.attached(fresh.id()).is_some());
    assert_that!(held).is_equal_to((true, true, true));
}

#[test]
fn attach_error_is_said_in_french_without_user_data() {
    let mut s = session();
    let e = s.attach(Upload::new(b"bonjour".to_vec(), "secret.pdf"), &[]).unwrap_err();
    let fr = e.localize(&Localizer::new(Some("fr")));
    assert_that!(fr.contains("secret")).is_false();
    assert_that!(fr.as_str()).is_not_equal_to("session-attach-type-mismatch");
}

/// Test settings whose `[paste]` is edited by `edit`, and the folder they point at (keep it).
fn paste_settings(
    edit: impl FnOnce(&mut oku_slides_platform::config::PasteSettings),
) -> (tempfile::TempDir, crate::Settings) {
    let data = crate::test_support::fresh_dir("oku-slides-paste-");
    let mut settings = crate::test_support::settings_on(data.path());
    edit(&mut settings.paste);
    (data, settings)
}

/// The op that gives `asset` to the image `e-image` of `s-libre` (roadmap.json).
fn set_image(asset: &str) -> Op {
    serde_json::from_value(json!({
        "op": "set_param", "target": { "slide": "s-libre", "element": "e-image" }, "param": "asset", "value": asset
    }))
    .unwrap()
}

/// The op writing `/a.md`, whose body cites the image asset `id`.
fn knowledge_citing(id: &str) -> Op {
    let hex = id.trim_start_matches("sha256:");
    Op::KnowledgeWrite {
        path: "/a.md".into(),
        text: format!("---\ntype: Note\n---\n![x](/references/assets/{hex}.png)\n"),
        id: None,
    }
}

/// Sets `s`'s sources budget one byte above what its sources weigh now.
fn tight(s: &mut Session) {
    let total = crate::sources_budget::source_bytes(&s.doc.sources).unwrap();
    s.source_limits.total_bytes = usize::try_from(total).unwrap() + 1;
}

#[test]
fn apply_citing_an_attached_image_adds_it_to_the_bundle() {
    let mut s = session();
    let r = s.attach(Upload::new(png(4, 4, 8), ""), &[]).unwrap();
    s.apply("image", vec![set_image(r.id())], None).unwrap();
    assert_that!(s.doc.sources.assets.contains_key(r.id())).is_true();
}

#[test]
fn attach_alone_adds_nothing_to_the_bundle() {
    let mut s = session();
    let r = s.attach(Upload::new(png(4, 4, 9), ""), &[]).unwrap();
    assert_that!(s.doc.sources.assets.contains_key(r.id())).is_false();
}

#[test]
fn embed_cited_document_id_unknown_asset() {
    let mut s = session();
    let r = s.attach(Upload::new(PDF.to_vec(), "a.pdf"), &[]).unwrap();
    let e = s.apply("image", vec![set_image(r.id())], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::UnknownAsset);
    assert_that!(s.doc.sources.assets.contains_key(r.id())).is_false();
}

#[test]
fn embed_knowledge_image_ref_from_attachment_added() {
    let mut s = session();
    let r = s.attach(Upload::new(png(4, 4, 12), ""), &[]).unwrap();
    s.apply("knowledge", vec![knowledge_citing(r.id())], None).unwrap();
    assert_that!(s.doc.sources.assets.contains_key(r.id())).is_true();
}

#[test]
fn embed_knowledge_unknown_image_ref_stays_missing() {
    let mut s = session();
    let id = oku_slides_core::asset_id(&png(4, 4, 13));
    assert_that!(s.apply("knowledge", vec![knowledge_citing(&id)], None)).is_ok();
    assert_that!(s.doc.sources.assets.contains_key(&id)).is_false();
}

#[test]
fn embed_knowledge_image_past_budget_refused() {
    let mut s = session();
    let r = s.attach(Upload::new(png(4, 4, 14), ""), &[]).unwrap();
    tight(&mut s);
    let e = s.apply("knowledge", vec![knowledge_citing(r.id())], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::SourcesOverBudget);
    assert_that!(s.doc.sources.assets.contains_key(r.id())).is_false();
}

#[test]
fn apply_citing_an_unknown_asset_is_refused() {
    let mut s = session();
    let before: Vec<String> = s.doc.sources.assets.keys().cloned().collect();
    let e = s.apply("image", vec![set_image("sha256:1234")], None).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::UnknownAsset);
    // The UI shows this text; the agent has its own (`agent-error-asset-unknown`, t-1171).
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("session-asset-unknown"));
    let after: Vec<String> = s.doc.sources.assets.keys().cloned().collect();
    assert_that!(after).is_equal_to(before);
}

#[test]
fn apply_keeps_an_asset_already_missing_before_the_op() {
    let mut s = session();
    let fit = serde_json::from_value(json!({
        "op": "set_param", "target": { "slide": "s-libre", "element": "e-image" }, "param": "fit", "value": "contain"
    }))
    .unwrap();
    assert_that!(s.apply("fit", vec![fit], None)).is_ok();
}
