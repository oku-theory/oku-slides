use std::sync::Arc;

use agent_client_protocol::schema::v1::ContentBlock;
use oku_slides_platform::config::PromptImageLimits;
use oku_slides_session::agent::AttachmentRef;
use oku_slides_session::agent::LABEL_MAX;
use rstest::rstest;
use speculoos::prelude::*;

use super::{document_block, image_blocks};

fn image(id: &str, len: usize) -> (AttachmentRef, Option<Arc<[u8]>>) {
    let r = AttachmentRef::Image { id: id.into(), mime: "image/png".into(), width: 4, height: 3 };
    (r, Some(vec![0u8; len].into()))
}

fn limits(max_bytes: usize) -> PromptImageLimits {
    PromptImageLimits { max_bytes, max_side: 8000, total_bytes: usize::MAX }
}

fn text_of(b: &ContentBlock) -> &str {
    match b {
        ContentBlock::Text(t) => &t.text,
        _ => "",
    }
}

#[test]
fn image_blocks_list_the_ids_then_one_image_block_each() {
    let blocks = image_blocks(&[image("sha256:aa", 8), image("sha256:bb", 8)], true, limits(200));
    assert_that!(blocks.len()).is_equal_to(3);
    assert_that!(text_of(&blocks[0])).contains("1. sha256:aa image/png 4×3");
    assert_that!(matches!(&blocks[2], ContentBlock::Image(i) if i.mime_type == "image/png")).is_true();
}

#[test]
fn image_blocks_for_an_agent_without_images_only_list_them() {
    let blocks = image_blocks(&[image("sha256:aa", 8)], false, limits(200));
    assert_that!(blocks.len()).is_equal_to(1);
    assert_that!(text_of(&blocks[0])).contains("not shown: this agent does not receive images");
}

#[test]
fn image_blocks_over_the_prompt_limit_are_only_listed() {
    let blocks = image_blocks(&[image("sha256:aa", 200)], true, limits(100));
    assert_that!(blocks.len()).is_equal_to(1);
    assert_that!(text_of(&blocks[0])).contains("not shown: too large to show you");
}

#[rstest]
#[case::encoded_just_over_raw_under(76, 4, "too large to show you")]
#[case::encoded_just_under(75, 4, "")]
#[case::side_over_limit(8, 8001, "too large to show you")]
fn image_blocks_are_bounded_by_encoded_size_and_side(#[case] len: usize, #[case] side: u32, #[case] note: &str) {
    // 76 bytes encode to 104 characters, above 100; 75 to 100.
    let (_, bytes) = image("sha256:aa", len);
    let r = AttachmentRef::Image { id: "sha256:aa".into(), mime: "image/png".into(), width: side, height: 3 };
    let blocks = image_blocks(&[(r, bytes)], true, limits(100));
    if note.is_empty() {
        assert_that!(blocks.len()).is_equal_to(2);
    } else {
        assert_that!(blocks.len()).is_equal_to(1);
        assert_that!(text_of(&blocks[0])).contains(note);
    }
}

#[test]
fn image_blocks_over_the_total_budget_are_only_listed() {
    // Chacune pèse 40 caractères encodés (30 octets) ; le budget de 100 n'en admet que deux.
    let l = PromptImageLimits { max_bytes: 100, max_side: 8000, total_bytes: 100 };
    let blocks = image_blocks(&[image("sha256:aa", 30), image("sha256:bb", 30), image("sha256:cc", 30)], true, l);
    assert_that!(blocks.len()).is_equal_to(3);
    assert_that!(text_of(&blocks[0]))
        .contains("3. sha256:cc image/png 4×3 (not shown: the images of this message are too heavy together)");
}

#[test]
fn image_blocks_of_a_forgotten_image_say_it_is_gone() {
    let gone = (AttachmentRef::Image { id: "sha256:cc".into(), mime: "image/png".into(), width: 1, height: 1 }, None);
    let blocks = image_blocks(&[gone], true, limits(200));
    assert_that!(text_of(&blocks[0])).contains("no longer available");
}

#[test]
fn image_blocks_skip_an_attached_document() {
    let doc = AttachmentRef::Document {
        id: "sha256:dd".into(),
        name: "a.pdf".into(),
        kind: oku_slides_core::knowledge::DocumentType::Pdf,
        bytes: 8,
    };
    let only_doc = image_blocks(&[(doc.clone(), Some(vec![0u8; 8].into()))], true, limits(200));
    assert_that!(only_doc).is_empty();
    let blocks = image_blocks(&[(doc, None), image("sha256:aa", 8)], true, limits(200));
    assert_that!(text_of(&blocks[0])).contains("\n1. sha256:aa image/png 4×3\n");
    assert_that!(text_of(&blocks[0]).contains("sha256:dd")).is_false();
}

#[test]
fn image_blocks_without_images_are_empty() {
    assert_that!(image_blocks(&[], true, limits(200))).is_empty();
}

fn document(id: &str, name: &str, bytes: Option<usize>) -> (AttachmentRef, Option<Arc<[u8]>>) {
    let r = AttachmentRef::Document {
        id: id.into(),
        name: name.into(),
        kind: oku_slides_core::knowledge::DocumentType::Pdf,
        bytes: 2049,
    };
    (r, bytes.map(|n| vec![0u8; n].into()))
}

#[test]
fn prompt_lists_documents_with_ids() {
    let attached = [document("sha256:dd", "Rapport \"final\"\u{202E}.pdf", Some(2049))];
    let block = document_block(&attached);
    let text = block.as_ref().map(text_of).unwrap_or_default();
    assert_that!(text).starts_with("Attached documents (attached by the user with this message; import one with knowledge_import {\"attachment\": \"<id>\"} before quoting it):\n");
    assert_that!(text).contains("- sha256:dd \"Rapport \u{2BC}final\u{2BC}.pdf\" (PDF, 3 KB)");
    assert_that!(text).does_not_contain("\u{202E}");
}

#[test]
fn prompt_document_name_is_clipped_to_label_max() {
    let name = format!("{}.pdf", "n".repeat(251));
    let block = document_block(&[document("sha256:dd", &name, Some(8))]);
    let text = block.as_ref().map(text_of).unwrap_or_default();
    assert_that!(text).contains(format!("- sha256:dd \"{}…\" (PDF", "n".repeat(LABEL_MAX - 1)).as_str());
    assert_that!(text).does_not_contain("n".repeat(LABEL_MAX).as_str());
}

#[test]
fn prompt_forgotten_document_says_so() {
    let block = document_block(&[document("sha256:dd", "a.pdf", None)]);
    let text = block.as_ref().map(text_of).unwrap_or_default();
    assert_that!(text)
        .contains("- sha256:dd \"a.pdf\" (PDF, 3 KB) (no longer available: ask the user to attach it again)");
}

#[test]
fn images_still_shown_beside_documents() {
    let attached = [image("sha256:aa", 8), document("sha256:dd", "a.pdf", Some(8))];
    let images = image_blocks(&attached, true, limits(200));
    assert_that!(images.len()).is_equal_to(2);
    assert_that!(text_of(&images[0])).does_not_contain("sha256:dd");
    assert_that!(matches!(&images[1], ContentBlock::Image(_))).is_true();
    let documents = document_block(&attached);
    let text = documents.as_ref().map(text_of).unwrap_or_default();
    assert_that!(text).contains("sha256:dd");
    assert_that!(text).does_not_contain("sha256:aa");
}

#[test]
fn document_block_without_document_is_none() {
    assert_that!(document_block(&[image("sha256:aa", 8)]).is_none()).is_true();
}

/// The path the prompt gives the agent for a document: attach, list, import, read back.
#[test]
fn attach_prompt_import_end_to_end() {
    use oku_slides_session::{Fonts, Platform, Session, Settings};
    let data = tempfile::Builder::new().prefix("oku-slides-agent-import-").tempdir().unwrap();
    let mut settings = Settings::defaults().unwrap();
    settings.paths.data = Some(data.path().to_owned());
    settings.paths.decks = Some(data.path().join("decks"));
    let platform = Platform::with_settings(settings).unwrap();
    let mut s = Session::new(Fonts::default(), &platform).unwrap();
    let fixture = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    s.open(&fixture).unwrap();
    let upload = oku_slides_session::agent::Upload::new(b"# Minutes\n\nShip in March.\n".to_vec(), "Minutes.md");
    let attached = s.attach(upload, &[]).unwrap();
    let kept = s.attached(attached.id()).map(|(_, bytes)| bytes);
    let block = document_block(&[(attached.clone(), kept)]);
    let listed = block.as_ref().map(text_of).unwrap_or_default();
    assert_that!(listed).contains(format!("- {} \"Minutes.md\"", attached.id()).as_str());
    let job = s.knowledge_import_job(attached.id(), None).unwrap();
    let extracted = job.run().unwrap();
    let outcome = s.knowledge_import_apply("Agent", job.ticket(), extracted).unwrap();
    assert_that!(outcome.paths).is_equal_to(vec!["/references/minutes.md".to_owned()]);
    let read = s.knowledge_file("/references/minutes.md").unwrap();
    assert_that!(read.text.as_str()).contains("Ship in March.");
}
