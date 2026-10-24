use std::sync::Arc;

use agent_client_protocol::schema::v1::ContentBlock;
use oku_slides_platform::config::PromptImageLimits;
use oku_slides_session::agent::AttachmentRef;
use rstest::rstest;
use speculoos::prelude::*;

use super::image_blocks;

fn image(id: &str, len: usize) -> (AttachmentRef, Option<Arc<[u8]>>) {
    let r = AttachmentRef { id: id.into(), mime: "image/png".into(), width: 4, height: 3 };
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
    // 76 octets s'encodent en 104 caractères, au-dessus de 100 ; 75 en 100.
    let (mut r, bytes) = image("sha256:aa", len);
    r.width = side;
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
    let gone = (AttachmentRef { id: "sha256:cc".into(), mime: "image/png".into(), width: 1, height: 1 }, None);
    let blocks = image_blocks(&[gone], true, limits(200));
    assert_that!(text_of(&blocks[0])).contains("no longer available");
}

#[test]
fn image_blocks_without_images_are_empty() {
    assert_that!(image_blocks(&[], true, limits(200))).is_empty();
}
