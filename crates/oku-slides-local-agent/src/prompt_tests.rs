use agent_client_protocol::schema::v1::{ContentBlock, TextContent};
use speculoos::prelude::*;

use super::read_prompt;

fn text(s: &str) -> ContentBlock {
    ContentBlock::Text(TextContent::new(s.to_owned()))
}

#[test]
fn read_prompt_takes_last_block_as_request_and_skips_host_instructions() {
    let p = read_prompt(&[
        text("You edit an Oku Slides presentation…"),
        text("<oku-slides-context>\ninterface language: French (fr) · deck language: …\n</oku-slides-context>"),
        text("mets le titre en rouge"),
    ]);
    assert_that!(p.request.as_str()).is_equal_to("mets le titre en rouge");
    assert_that!(p.locale.as_deref()).is_equal_to(Some("fr"));
    assert_that!(p.context.unwrap()).starts_with("<oku-slides-context>");
}

#[test]
fn read_prompt_without_context_has_no_locale() {
    assert_that!(read_prompt(&[text("bonjour")]).locale).is_none();
}

#[test]
fn read_prompt_without_text_has_an_empty_request() {
    assert_that!(read_prompt(&[]).request.as_str()).is_equal_to("");
}
