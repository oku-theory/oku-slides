//! The draft's attachments (images and documents): attach, removal, refusals.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::io::Cursor;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use common::{cmd, drafted, is, logged, next, shown, stack_with_sink, stack_with_sink_and_log, wait_turn_done};
use serde_json::json;
use speculoos::prelude::*;

/// A valid 2 × 2 PNG, distinct for each `seed`, in base64.
fn png64(seed: u8) -> String {
    let mut out = Cursor::new(Vec::new());
    let mut e = png::Encoder::new(&mut out, 2, 2);
    e.set_color(png::ColorType::Grayscale);
    let mut w = e.write_header().unwrap_or_else(|e| panic!("png header: {e}"));
    w.write_image_data(&[seed, 1, 2, 3]).unwrap_or_else(|e| panic!("png pixels: {e}"));
    drop(w);
    STANDARD.encode(out.into_inner())
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_image_adds_it_to_the_broadcast_draft() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("pose-la"), from).await;
    stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(1) }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    let image = &draft["event"]["attachments"][0];
    assert_that!((image["mime"].clone(), image["width"].clone(), draft["event"]["text"].clone())).is_equal_to((
        json!("image/png"),
        json!(2),
        json!("pose-la"),
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_image_on_a_closed_draft_opens_it_empty() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", None, from).await;
    stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(2) }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["text"].clone()).is_equal_to(json!(""));
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_image_twice_lists_it_once() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(3) }))).unwrap();
    let (at, _) = next(&rx, from, is("agent", "draft", "t-1")).await;
    stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(3) }))).unwrap();
    let (_, draft) = next(&rx, at + 1, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["attachments"].as_array().map(Vec::len)).is_equal_to(Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_image_past_the_draft_limit_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    drafted(&stack, &rx, "t-1", Some("x"), from).await;
    for seed in 0..8 {
        stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(10 + seed) }))).unwrap();
    }
    let refused = stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(30) })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_unsupported_bytes_are_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1").await;
    // Neither an image nor a document: no magic, and not UTF-8 text.
    let data = STANDARD.encode([0u8, 0xFF, 0xFE, 0x00]);
    let refused = stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": data })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_image_on_a_tab_without_agent_is_refused() {
    let (stack, _rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let refused = stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(4) })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn remove_attachment_drops_it_from_the_draft() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(5) }))).unwrap();
    let (at, pasted) = next(&rx, from, is("agent", "draft", "t-1")).await;
    let id = pasted["event"]["attachments"][0]["id"].clone();
    stack.handle(cmd(json!({ "cmd": "remove_attachment", "tab": "t-1", "id": id }))).unwrap();
    let (_, draft) = next(&rx, at + 1, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["attachments"].clone()).is_equal_to(json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_null_drops_the_attachments() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(6) }))).unwrap();
    let (at, _) = next(&rx, from, is("agent", "draft", "t-1")).await;
    let from = drafted(&stack, &rx, "t-1", None, at + 1).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["attachments"].clone()).is_equal_to(json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_image_concurrently_never_passes_the_draft_limit() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    // `std::thread::scope`: one thread per image, `handle` blocks.
    let accepted = tokio::task::block_in_place(|| {
        std::thread::scope(|s| {
            let threads: Vec<_> = (0..9)
                .map(|seed| {
                    let (stack, data) = (&stack, png64(40 + seed));
                    s.spawn(move || {
                        stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": data })))
                    })
                })
                .collect();
            threads
                .into_iter()
                .map(std::thread::ScopedJoinHandle::join)
                .filter(|r| r.as_ref().is_ok_and(Result::is_ok))
                .count()
        })
    });
    let (_, last) = next(&rx, from, |(c, v)| {
        *c == "agent"
            && v["event"]["kind"] == "draft"
            && v["event"]["attachments"].as_array().is_some_and(|a| a.len() == 8)
    })
    .await;
    assert_that!((accepted, last["event"]["attachments"].as_array().map(Vec::len))).is_equal_to((8, Some(8)));
}

/// Attaches an image to `tab`'s draft and returns its id.
async fn pasted(stack: &common::TestStack, rx: &common::Relayed, tab: &'static str, seed: u8) -> String {
    let from = shown(stack, rx, tab).await;
    let from = drafted(stack, rx, tab, Some("x"), from).await;
    stack
        .handle(cmd(json!({ "cmd": "attach", "tab": tab, "name": "x.png", "data": png64(seed) })))
        .unwrap_or_else(|e| panic!("attach: {e}"));
    let (_, draft) = next(rx, from, is("agent", "draft", tab)).await;
    draft["event"]["attachments"][0]["id"].as_str().unwrap_or_else(|| panic!("image missing from the draft")).to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_an_image_lists_it_then_shows_it_before_the_text() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "prompt_image": true, "turns": [{}] }), &["t-1"]).await;
    let id = pasted(&stack, &rx, "t-1", 40).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "pose-la", "attachments": [id] }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let blocks = logged(&log, "session/prompt")[0]["prompt"].as_array().cloned().unwrap();
    let n = blocks.len();
    assert_that!(blocks[n - 3]["text"].as_str().unwrap()).contains(id.as_str());
    assert_that!((blocks[n - 2]["type"].clone(), blocks[n - 2]["mimeType"].clone()))
        .is_equal_to((json!("image"), json!("image/png")));
    assert_that!(blocks[n - 1]["text"].clone()).is_equal_to(json!("pose-la"));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_an_image_to_an_agent_without_images_sends_only_its_id() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    let id = pasted(&stack, &rx, "t-1", 41).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "pose-la", "attachments": [id] }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let prompt = logged(&log, "session/prompt")[0]["prompt"].to_string();
    assert_that!((prompt.contains(&id), prompt.contains("\"type\":\"image\""))).is_equal_to((true, false));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_an_unknown_image_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{}] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1").await;
    let refused =
        stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "x", "attachments": ["sha256:00"] })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_image_already_in_a_full_draft_is_a_no_op() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    drafted(&stack, &rx, "t-1", Some("x"), from).await;
    for seed in 0..8 {
        stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(50 + seed) }))).unwrap();
    }
    let again = stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(50) })));
    assert_that!(again).is_ok();
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_the_same_image_twice_lists_it_once() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "prompt_image": true, "turns": [{}] }), &["t-1"]).await;
    let id = pasted(&stack, &rx, "t-1", 60).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "pose-la", "attachments": [id, id] }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let prompt = logged(&log, "session/prompt")[0]["prompt"].to_string();
    let images = prompt.matches("\"type\":\"image\"").count();
    assert_that!((prompt.matches(id.as_str()).count(), images)).is_equal_to((1, 1));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_more_attachments_than_the_draft_allows_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{}] }), &["t-1"]).await;
    let first = pasted(&stack, &rx, "t-1", 70).await;
    let mut ids = vec![first];
    // 8 images and 4 documents fit in a draft: 13 distinct ids pass both caps.
    ids.extend((0..12).map(|n| format!("sha256:{n:02x}")));
    let many = stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "x", "attachments": ids })));
    let unknown =
        stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "x", "attachments": ["sha256:ff"] })));
    // The cap is checked before the lookup: its message differs from a forgotten image's.
    assert_that!(many).is_err();
    assert_that!(many.unwrap_err()).is_not_equal_to(unknown.unwrap_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_theme_with_an_image_goes_to_the_agent() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    let id = pasted(&stack, &rx, "t-1", 80).await;
    let prompt = json!({ "cmd": "prompt", "tab": "t-1", "text": "thème", "attachments": [id] });
    tokio::task::block_in_place(|| stack.handle(cmd(prompt))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let prompts = logged(&log, "session/prompt");
    assert_that!((prompts.len(), prompts[0].to_string().contains(id.as_str()))).is_equal_to((1, true));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_answering_a_waiting_question_with_an_image_goes_to_the_agent() {
    let ask = json!({ "tool": "ask_choice", "args": { "question": "Laquelle ?", "options": [
        { "id": "a", "label": "A" }, { "id": "b", "label": "B" } ] } });
    let script = json!({ "turns": [{ "mcp": [ask] }, { "chunks": ["ok"] }] });
    let (stack, rx, log) = stack_with_sink_and_log(script, &["t-1"]).await;
    let image = pasted(&stack, &rx, "t-1", 81).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "demande-moi" }))).unwrap();
    let asked = common::wait_for(&rx, |(_, v)| common::is_tool_surface(v, "t-1", "ask-")).await;
    let id = asked.1["event"]["surface"]["id"].clone();
    let answer = json!({ "cmd": "prompt", "tab": "t-1", "text": "la deuxième", "answer": id, "attachments": [image] });
    stack.handle(cmd(answer)).unwrap();
    wait_turn_done(&rx, "t-1", 2).await;
    let prompts = logged(&log, "session/prompt");
    let outs: Vec<String> = logged(&log, "mcp/result").iter().map(|r| r["out"].to_string()).collect();
    assert_that!((
        prompts.len(),
        prompts[1].to_string().contains("la deuxième"),
        outs.iter().any(|o| o.contains("la deuxième"))
    ))
    .is_equal_to((2, true, false));
}

#[tokio::test(flavor = "multi_thread")]
async fn remove_attachment_on_a_tab_without_agent_is_refused_as_no_agent() {
    let (stack, _rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let removed = stack.handle(cmd(json!({ "cmd": "remove_attachment", "tab": "t-1", "id": "sha256:00" })));
    let pasted = stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(82) })));
    // Both refusals come from `NoAgent`: the same localized message.
    assert_that!(removed).is_err();
    assert_that!(removed.unwrap_err()).is_equal_to(pasted.unwrap_err());
}

/// A Markdown document named `name`, distinct for each `seed`, in base64.
fn markdown64(seed: u8) -> String {
    STANDARD.encode(format!("# Notes {seed}\n\nDu texte.\n"))
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_document_adds_it_to_the_broadcast_draft() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("lis-le"), from).await;
    let attach = json!({ "cmd": "attach", "tab": "t-1", "name": "notes.md", "data": markdown64(1) });
    stack.handle(cmd(attach)).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    let document = &draft["event"]["attachments"][0];
    assert_that!((document["type"].clone(), document["name"].clone(), document["kind"].clone())).is_equal_to((
        json!("document"),
        json!("notes.md"),
        json!("markdown"),
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_past_max_draft_documents_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    drafted(&stack, &rx, "t-1", Some("x"), from).await;
    for seed in 0..4 {
        let attach = json!({ "cmd": "attach", "tab": "t-1", "name": "n.md", "data": markdown64(10 + seed) });
        stack.handle(cmd(attach)).unwrap();
    }
    // An image still fits beside the documents: each kind has its own cap.
    stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": png64(90) }))).unwrap();
    let fifth = json!({ "cmd": "attach", "tab": "t-1", "name": "n.md", "data": markdown64(30) });
    let refused = stack.handle(cmd(fifth));
    let expected =
        oku_slides_i18n::Localizer::new(Some("fr")).tr_with("agent-error-too-many-documents", &[("max", 4u64.into())]);
    assert_that!(refused).is_equal_to(Err(expected));
}

/// A draft of `t-1` holding the four Markdown documents `markdown64(seed..seed + 4)`.
async fn four_documents(stack: &common::TestStack, rx: &common::Relayed, seed: u8) {
    let from = shown(stack, rx, "t-1").await;
    drafted(stack, rx, "t-1", Some("x"), from).await;
    for n in 0..4 {
        let attach = json!({ "cmd": "attach", "tab": "t-1", "name": "n.md", "data": markdown64(seed + n) });
        stack.handle(cmd(attach)).unwrap_or_else(|e| panic!("attach: {e}"));
    }
}

/// The French message of the session's refusal `key`.
fn refused_fr(key: &str) -> String {
    oku_slides_i18n::Localizer::new(Some("fr")).tr(key)
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_document_already_in_a_full_draft_is_allowed() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    four_documents(&stack, &rx, 100).await;
    let again = stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "n.md", "data": markdown64(100) })));
    assert_that!(again).is_ok();
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_svg_beside_a_full_draft_of_documents_gets_its_own_refusal() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    four_documents(&stack, &rx, 110).await;
    let svg = STANDARD.encode("<svg xmlns=\"http://www.w3.org/2000/svg\"/>");
    let refused = stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "logo.svg", "data": svg })));
    assert_that!(refused).is_equal_to(Err(refused_fr("session-paste-unsupported")));
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_unrecognised_bytes_beside_a_full_draft_of_documents_get_their_own_refusal() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    four_documents(&stack, &rx, 120).await;
    let unknown = STANDARD.encode([0u8, 0xFF, 0xFE, 0x00]);
    let refused = stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.bin", "data": unknown })));
    let named_png = STANDARD.encode("bonjour");
    let mismatch = stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x.png", "data": named_png })));
    assert_that!((refused, mismatch))
        .is_equal_to((Err(refused_fr("session-paste-unsupported")), Err(refused_fr("session-attach-type-mismatch"))));
}

#[tokio::test(flavor = "multi_thread")]
async fn attach_data_past_the_largest_limit_is_refused_before_decoding() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1").await;
    let paste = oku_slides_platform::config::Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}")).paste;
    let max = paste.max_image_bytes().max(paste.max_document_bytes()).div_ceil(3) * 4;
    // Not base64 at all: only a refusal before decoding says "too large" rather than "not whole".
    let at_max = stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x", "data": "!".repeat(max) })));
    let past = stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "x", "data": "!".repeat(max + 1) })));
    assert_that!((at_max, past)).is_equal_to((
        Err(refused_fr("agent-error-attachment-not-base64")),
        Err(refused_fr("agent-error-attachment-too-large")),
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_a_document_lists_it_just_before_the_text() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    stack.handle(cmd(json!({ "cmd": "attach", "tab": "t-1", "name": "notes.md", "data": markdown64(130) }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    let id = draft["event"]["attachments"][0]["id"].as_str().unwrap().to_owned();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "lis-le", "attachments": [id] }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let blocks = logged(&log, "session/prompt")[0]["prompt"].as_array().cloned().unwrap();
    let n = blocks.len();
    let listed = blocks[n - 2]["text"].as_str().unwrap();
    assert_that!(listed).starts_with("Attached documents");
    assert_that!(listed).contains(id.as_str());
    assert_that!(listed).contains("\"notes.md\"");
    assert_that!(blocks[n - 1]["text"].clone()).is_equal_to(json!("lis-le"));
}
