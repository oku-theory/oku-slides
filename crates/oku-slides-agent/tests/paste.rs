//! Images collées dans le brouillon : dépôt, retrait, refus.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::io::Cursor;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use common::{cmd, drafted, is, logged, next, shown, stack_with_sink, stack_with_sink_and_log, wait_turn_done};
use serde_json::json;
use speculoos::prelude::*;

/// Un PNG valide de 2 × 2 pixels, propre à `seed`, en base64.
fn png64(seed: u8) -> String {
    let mut out = Cursor::new(Vec::new());
    let mut e = png::Encoder::new(&mut out, 2, 2);
    e.set_color(png::ColorType::Grayscale);
    let mut w = e.write_header().unwrap_or_else(|e| panic!("en-tête png : {e}"));
    w.write_image_data(&[seed, 1, 2, 3]).unwrap_or_else(|e| panic!("pixels png : {e}"));
    drop(w);
    STANDARD.encode(out.into_inner())
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_adds_it_to_the_broadcast_draft() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("pose-la"), from).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(1) }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    let image = &draft["event"]["images"][0];
    assert_that!((image["mime"].clone(), image["width"].clone(), draft["event"]["text"].clone())).is_equal_to((
        json!("image/png"),
        json!(2),
        json!("pose-la"),
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_on_a_closed_draft_opens_it_empty() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", None, from).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(2) }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["text"].clone()).is_equal_to(json!(""));
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_twice_lists_it_once() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(3) }))).unwrap();
    let (at, _) = next(&rx, from, is("agent", "draft", "t-1")).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(3) }))).unwrap();
    let (_, draft) = next(&rx, at + 1, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["images"].as_array().map(Vec::len)).is_equal_to(Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_past_the_draft_limit_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    drafted(&stack, &rx, "t-1", Some("x"), from).await;
    for seed in 0..8 {
        stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(10 + seed) }))).unwrap();
    }
    let refused = stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(30) })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_that_is_not_an_image_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1").await;
    let data = STANDARD.encode(b"bonjour");
    let refused = stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": data })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_on_a_tab_without_agent_is_refused() {
    let (stack, _rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let refused = stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(4) })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn remove_image_drops_it_from_the_draft() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(5) }))).unwrap();
    let (at, pasted) = next(&rx, from, is("agent", "draft", "t-1")).await;
    let id = pasted["event"]["images"][0]["id"].clone();
    stack.handle(cmd(json!({ "cmd": "remove_image", "tab": "t-1", "id": id }))).unwrap();
    let (_, draft) = next(&rx, at + 1, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["images"].clone()).is_equal_to(json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_draft_null_drops_the_images() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(6) }))).unwrap();
    let (at, _) = next(&rx, from, is("agent", "draft", "t-1")).await;
    let from = drafted(&stack, &rx, "t-1", None, at + 1).await;
    stack.handle(cmd(json!({ "cmd": "follow", "tab": "t-1" }))).unwrap();
    let (_, draft) = next(&rx, from, is("agent", "draft", "t-1")).await;
    assert_that!(draft["event"]["images"].clone()).is_equal_to(json!([]));
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_concurrently_never_passes_the_draft_limit() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    let from = drafted(&stack, &rx, "t-1", Some("x"), from).await;
    // `std::thread::scope` : un fil par image, `handle` est bloquant.
    let accepted = tokio::task::block_in_place(|| {
        std::thread::scope(|s| {
            let threads: Vec<_> = (0..9)
                .map(|seed| {
                    let (stack, data) = (&stack, png64(40 + seed));
                    s.spawn(move || stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": data }))))
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
        *c == "agent" && v["event"]["kind"] == "draft" && v["event"]["images"].as_array().is_some_and(|a| a.len() == 8)
    })
    .await;
    assert_that!((accepted, last["event"]["images"].as_array().map(Vec::len))).is_equal_to((8, Some(8)));
}

/// Dépose une image dans le brouillon de `tab` et rend son id.
async fn pasted(stack: &common::TestStack, rx: &common::Relayed, tab: &'static str, seed: u8) -> String {
    let from = shown(stack, rx, tab).await;
    let from = drafted(stack, rx, tab, Some("x"), from).await;
    stack
        .handle(cmd(json!({ "cmd": "paste_image", "tab": tab, "data": png64(seed) })))
        .unwrap_or_else(|e| panic!("collage : {e}"));
    let (_, draft) = next(rx, from, is("agent", "draft", tab)).await;
    draft["event"]["images"][0]["id"].as_str().unwrap_or_else(|| panic!("image absente du brouillon")).to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_an_image_lists_it_then_shows_it_before_the_text() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "prompt_image": true, "turns": [{}] }), &["t-1"]).await;
    let id = pasted(&stack, &rx, "t-1", 40).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "pose-la", "images": [id] }))).unwrap();
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
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "pose-la", "images": [id] }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let prompt = logged(&log, "session/prompt")[0]["prompt"].to_string();
    assert_that!((prompt.contains(&id), prompt.contains("\"type\":\"image\""))).is_equal_to((true, false));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_an_unknown_image_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{}] }), &["t-1"]).await;
    shown(&stack, &rx, "t-1").await;
    let refused = stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "x", "images": ["sha256:00"] })));
    assert_that!(refused).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn paste_image_already_in_a_full_draft_is_a_no_op() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let from = shown(&stack, &rx, "t-1").await;
    drafted(&stack, &rx, "t-1", Some("x"), from).await;
    for seed in 0..8 {
        stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(50 + seed) }))).unwrap();
    }
    let again = stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(50) })));
    assert_that!(again).is_ok();
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_the_same_image_twice_lists_it_once() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "prompt_image": true, "turns": [{}] }), &["t-1"]).await;
    let id = pasted(&stack, &rx, "t-1", 60).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "pose-la", "images": [id, id] }))).unwrap();
    wait_turn_done(&rx, "t-1", 1).await;
    let prompt = logged(&log, "session/prompt")[0]["prompt"].to_string();
    let images = prompt.matches("\"type\":\"image\"").count();
    assert_that!((prompt.matches(id.as_str()).count(), images)).is_equal_to((1, 1));
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_with_more_images_than_the_draft_allows_is_refused() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [{}] }), &["t-1"]).await;
    let first = pasted(&stack, &rx, "t-1", 70).await;
    let mut ids = vec![first];
    ids.extend((0..8).map(|n| format!("sha256:{n:02x}")));
    let many = stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "x", "images": ids })));
    let unknown = stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "x", "images": ["sha256:ff"] })));
    // Le plafond est contrôlé avant la recherche : son message diffère de celui d'une image oubliée.
    assert_that!(many).is_err();
    assert_that!(many.unwrap_err()).is_not_equal_to(unknown.unwrap_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_theme_with_an_image_goes_to_the_agent() {
    let (stack, rx, log) = stack_with_sink_and_log(json!({ "turns": [{}] }), &["t-1"]).await;
    let id = pasted(&stack, &rx, "t-1", 80).await;
    let prompt = json!({ "cmd": "prompt", "tab": "t-1", "text": "thème", "images": [id] });
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
    let answer = json!({ "cmd": "prompt", "tab": "t-1", "text": "la deuxième", "answer": id, "images": [image] });
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
async fn remove_image_on_a_tab_without_agent_is_refused_as_no_agent() {
    let (stack, _rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    let removed = stack.handle(cmd(json!({ "cmd": "remove_image", "tab": "t-1", "id": "sha256:00" })));
    let pasted = stack.handle(cmd(json!({ "cmd": "paste_image", "tab": "t-1", "data": png64(82) })));
    // Both refusals come from `NoAgent`: the same localized message.
    assert_that!(removed).is_err();
    assert_that!(removed.unwrap_err()).is_equal_to(pasted.unwrap_err());
}
