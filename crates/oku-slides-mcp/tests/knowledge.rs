//! The knowledge tools of a deck tab, on a real server: list, read with its images, search.
#![expect(clippy::panic, reason = "test failures panic")]
mod common;

use std::fmt::Write as _;
use std::sync::PoisonError;

use base64::Engine as _;
use common::kinds::{Kind, list_tool_names};
use common::*;
use oku_slides_core::asset_id;
use oku_slides_i18n::Localizer;
use oku_slides_session::Settings;
use oku_slides_session::knowledge_fixture::{approve_knowledge, seed_knowledge, unstamped};
use rstest::rstest;
use rust_mcp_sdk::schema::{CallToolResult, ContentBlock};
use serde_json::{Value, json};
use speculoos::prelude::*;

/// Taken by each test that imports: the import slot is the process's, so imports running at once
/// in this binary (cargo test threads) would see a spurious busy.
static IMPORTS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const TOOLS: [&str; 8] = [
    "knowledge_import",
    "knowledge_list",
    "knowledge_read",
    "knowledge_search",
    "knowledge_write",
    "knowledge_edit",
    "knowledge_move",
    "knowledge_delete",
];

const ROADMAP: &str = "/references/roadmap.md";
const SECRET: &str = "/notes/secret.md";
const ROADMAP_TEXT: &str =
    "---\ntype: Note\ntitle: Roadmap\ndescription: Q1 goals\nstatus: draft\n---\nThe ROADMAP for Q1\nnothing here\n";
const SECRET_TEXT: &str = "---\ntype: Note\ntitle: Hidden\n---\nroadmap secret\n";

/// A PNG header of `width`×`height`, padded to `len` bytes with `fill` (its bytes differ by `fill`).
fn png(width: u32, height: u32, len: usize, fill: u8) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.resize(len, fill);
    bytes
}

/// The hex of `bytes`' asset id, as a bundle path cites it.
fn hex_of(bytes: &[u8]) -> String {
    asset_id(bytes).trim_start_matches("sha256:").to_owned()
}

/// A deck server whose knowledge is `files`, `approved` approved, holding `assets`.
async fn start_knowledge(
    tweak: impl FnOnce(&mut Settings) + Send + 'static,
    approved: Option<Vec<String>>,
    files: Vec<(String, String)>,
    assets: Vec<Vec<u8>>,
) -> Harness {
    start_seeded(
        tweak,
        move |data| {
            if let Some(texts) = approved {
                let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
                approve_knowledge(data, &texts);
            }
        },
        move |s| {
            let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect();
            seed_knowledge(s, &files, assets);
        },
    )
    .await
}

/// The roadmap (approved) and the secret (awaiting), no image.
async fn two_files() -> Harness {
    start_knowledge(
        |_| {},
        Some(vec![ROADMAP_TEXT.to_owned()]),
        vec![(ROADMAP.to_owned(), ROADMAP_TEXT.to_owned()), (SECRET.to_owned(), SECRET_TEXT.to_owned())],
        Vec::new(),
    )
    .await
}

/// A file citing `cited` images, approved, the deck holding `held`.
async fn citing(tweak: impl FnOnce(&mut Settings) + Send + 'static, cited: &[Vec<u8>], held: Vec<Vec<u8>>) -> Harness {
    let mut text = String::from("---\ntype: Note\n---\nimages\n");
    for (n, bytes) in cited.iter().enumerate() {
        let _ = writeln!(text, "![i{n}](assets/{}.png)", hex_of(bytes));
    }
    start_knowledge(tweak, Some(vec![text.clone()]), vec![(ROADMAP.to_owned(), text)], held).await
}

/// The image blocks of a result, after its text.
fn images_of(r: &CallToolResult) -> Vec<(String, String)> {
    r.content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::ImageContent(i) => Some((i.mime_type.clone(), i.data.clone())),
            _ => None,
        })
        .collect()
}

#[rstest]
#[case::deck(Kind::Deck, true)]
#[case::help(Kind::Help, false)]
#[case::pack(Kind::Pack, false)]
#[case::theme(Kind::Theme, false)]
#[tokio::test(flavor = "multi_thread")]
async fn tool_list_knowledge_tools_only_in_a_deck_tab(#[case] kind: Kind, #[case] listed: bool) {
    let h = kind.start().await;
    let c = server(&h).await;
    let names = list_tool_names(&c).await;
    for tool in TOOLS {
        assert_that!(names.contains(&tool.to_owned())).named(tool).is_equal_to(listed);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_search_description_names_the_configured_hit_cap() {
    let h = start_with_settings(|s| s.knowledge.search_hits = std::num::NonZeroUsize::new(7).unwrap()).await;
    let c = server(&h).await;
    assert_that!(tool_description(&c, "knowledge_search").await.as_str()).contains("At most 7 hits;");
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_list_approved_gives_frontmatter_awaiting_only_its_path() {
    let h = two_files().await;
    let c = server(&h).await;
    let r = call(&c, "knowledge_list", json!({})).await;
    assert_that!(r.is_error).is_none();
    assert_that!(json_of(&r)).is_equal_to(json!({ "files": [
        { "path": SECRET, "approval": "awaiting" },
        { "path": ROADMAP, "type": "Note", "title": "Roadmap", "description": "Q1 goals", "status": "draft",
          "stale": false, "approval": "approved" },
    ]}));
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_list_of_a_dir_lists_its_subtree_only() {
    let h = two_files().await;
    let c = server(&h).await;
    let r = call(&c, "knowledge_list", json!({ "dir": "/references" })).await;
    let paths: Vec<Value> = json_of(&r)["files"].as_array().into_iter().flatten().map(|f| f["path"].clone()).collect();
    assert_that!(paths).is_equal_to(vec![json!(ROADMAP)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_read_approved_gives_its_whole_text_and_no_image() {
    let h = two_files().await;
    let c = server(&h).await;
    let r = call(&c, "knowledge_read", json!({ "path": ROADMAP })).await;
    assert_that!(r.is_error).is_none();
    assert_that!(json_of(&r)).is_equal_to(json!({ "path": ROADMAP, "text": ROADMAP_TEXT, "images": [] }));
    assert_that!(r.content.len()).is_equal_to(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_read_awaiting_is_an_error_in_the_user_language() {
    let h = two_files().await;
    let c = server(&h).await;
    let r = call(&c, "knowledge_read", json!({ "path": SECRET })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let fr = Localizer::new(Some("fr")).tr_with("session-knowledge-awaiting", &[("path", "§".into())]);
    let (head, _) = fr.split_once('§').unwrap_or_else(|| panic!("no path in {fr}"));
    assert_that!(text_of(&r).as_str()).contains(head.trim_end_matches(['\u{2068}', ' ']));
    assert_that!(text_of(&r).as_str()).contains(SECRET);
    assert_that!(text_of(&r).as_str()).does_not_contain("roadmap secret");
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_read_present_and_missing_image_shows_one_lists_both() {
    let present = png(800, 600, 1 << 20, 1);
    let missing = png(10, 10, 100, 2);
    let h = citing(|_| {}, &[present.clone(), missing.clone()], vec![present.clone()]).await;
    let c = server(&h).await;
    let r = call(&c, "knowledge_read", json!({ "path": ROADMAP })).await;
    let images = json_of(&r)["images"].clone();
    assert_that!(images[0]).is_equal_to(json!({
        "asset": asset_id(&present), "path": format!("/references/assets/{}.png", hex_of(&present)),
        "mime": "image/png", "width": 800, "height": 600, "shown": true,
    }));
    assert_that!(images[1]["shown"]).is_equal_to(json!(false));
    assert_that!(images[1]["reason"]).is_equal_to(json!("missing from the deck"));
    let shown = images_of(&r);
    assert_that!(shown)
        .is_equal_to(vec![("image/png".to_owned(), base64::engine::general_purpose::STANDARD.encode(&present))]);
}

#[rstest]
#[case::too_large(&[(2 << 20, 1)], "too large to show you")]
#[case::too_heavy_together(&[(3 << 19, 1), (3 << 19, 2)], "too heavy together with the other images of this read")]
#[tokio::test(flavor = "multi_thread")]
async fn knowledge_read_over_the_prompt_limits_lists_the_last_with_its_reason(
    #[case] sizes: &[(usize, u8)],
    #[case] reason: &str,
) {
    let images: Vec<Vec<u8>> = sizes.iter().map(|(len, fill)| png(100, 100, *len, *fill)).collect();
    // 2 MiB per image and per read. Too large: 2 MiB raw is 8/3 MiB encoded. Too heavy: 1.5 MiB raw
    // is 2 MiB encoded, which fits alone, not twice.
    let tweak = |s: &mut Settings| {
        s.paste.prompt_image_mib = 2;
        s.paste.prompt_total_mib = 2;
    };
    let h = citing(tweak, &images, images.clone()).await;
    let c = server(&h).await;
    let r = call(&c, "knowledge_read", json!({ "path": ROADMAP })).await;
    let listed = json_of(&r)["images"].as_array().cloned().unwrap_or_default();
    let last = listed.last().cloned().unwrap_or(Value::Null);
    assert_that!((last["shown"].clone(), last["reason"].clone())).is_equal_to((json!(false), json!(reason)));
    assert_that!(images_of(&r).len()).is_equal_to(sizes.len() - 1);
}

/// A PNG signature whose first chunk is not `IHDR`: its size in pixels cannot be read.
fn corrupt_png() -> Vec<u8> {
    let mut bytes = png(100, 100, 200, 3);
    bytes[12..16].copy_from_slice(b"JUNK");
    bytes
}

#[rstest]
#[case::svg(br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"/>"#.to_vec(), "image/svg+xml")]
#[case::bmp(b"BM\x46\0\0\0\0\0\0\0\x36\0\0\0\x28\0\0\0\x0a\0\0\0\x0a\0\0\0".to_vec(), "image/bmp")]
#[case::corrupt_png_header(corrupt_png(), "image/png")]
#[case::zero_width_png(png(0, 100, 200, 4), "image/png")]
#[tokio::test(flavor = "multi_thread")]
async fn knowledge_read_of_an_image_it_cannot_show_lists_it_not_visible(#[case] image: Vec<u8>, #[case] mime: &str) {
    let h = citing(|_| {}, std::slice::from_ref(&image), vec![image.clone()]).await;
    let c = server(&h).await;
    let r = call(&c, "knowledge_read", json!({ "path": ROADMAP })).await;
    let listed = json_of(&r)["images"][0].clone();
    assert_that!((listed["mime"].clone(), listed["shown"].clone(), listed["reason"].clone())).is_equal_to((
        json!(mime),
        json!(false),
        json!("not an image format you can see"),
    ));
    assert_that!(images_of(&r)).is_empty();
}

#[rstest]
#[case::zero_width_png(png(0, 100, 3 << 19, 5))]
#[case::corrupt_png_header({
    let mut bytes = png(100, 100, 3 << 19, 6);
    bytes[12..16].copy_from_slice(b"JUNK");
    bytes
})]
#[tokio::test(flavor = "multi_thread")]
async fn knowledge_read_of_an_unshown_image_leaves_the_total_to_the_next(#[case] unshown: Vec<u8>) {
    // 2 MiB per read: 1.5 MiB raw is 2 MiB encoded, so only one of the two could fit.
    let tweak = |s: &mut Settings| {
        s.paste.prompt_image_mib = 2;
        s.paste.prompt_total_mib = 2;
    };
    // A read lists its images by asset id: the shown one is picked to come after the unshown one.
    let shown = (7..=u8::MAX)
        .map(|fill| png(100, 100, 3 << 19, fill))
        .find(|p| asset_id(p) > asset_id(&unshown))
        .unwrap_or_else(|| panic!("no fill sorts after the unshown image"));
    let cited = vec![unshown, shown.clone()];
    let h = citing(tweak, &cited, cited.clone()).await;
    let c = server(&h).await;
    let r = call(&c, "knowledge_read", json!({ "path": ROADMAP })).await;
    let listed = json_of(&r)["images"].clone();
    assert_that!((listed[0]["reason"].clone(), listed[1]["shown"].clone()))
        .is_equal_to((json!("not an image format you can see"), json!(true)));
    assert_that!(images_of(&r))
        .is_equal_to(vec![("image/png".to_owned(), base64::engine::general_purpose::STANDARD.encode(&shown))]);
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_search_finds_approved_lines_only() {
    let h = two_files().await;
    let c = server(&h).await;
    let r = call(&c, "knowledge_search", json!({ "query": "roadmap" })).await;
    assert_that!(json_of(&r)).is_equal_to(json!({
        "hits": [
            { "path": ROADMAP, "line": 3, "text": "title: Roadmap" },
            { "path": ROADMAP, "line": 7, "text": "The ROADMAP for Q1" },
        ],
        "more": 0,
    }));
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_tools_without_approvals_file_everything_awaits() {
    let h = start_knowledge(
        |_| {},
        None,
        vec![(ROADMAP.to_owned(), ROADMAP_TEXT.to_owned()), (SECRET.to_owned(), SECRET_TEXT.to_owned())],
        Vec::new(),
    )
    .await;
    let c = server(&h).await;
    let list = json_of(&call(&c, "knowledge_list", json!({})).await);
    let approvals: Vec<Value> = list["files"].as_array().into_iter().flatten().map(|f| f["approval"].clone()).collect();
    assert_that!(approvals).is_equal_to(vec![json!("awaiting"), json!("awaiting")]);
    assert_that!(call(&c, "knowledge_read", json!({ "path": ROADMAP })).await.is_error).is_equal_to(Some(true));
    let search = json_of(&call(&c, "knowledge_search", json!({ "query": "roadmap" })).await);
    assert_that!(search).is_equal_to(json!({ "hits": [], "more": 0 }));
}

/// A concept file of the agent's.
const PRICING: &str = "/notes/pricing.md";
const PRICING_TEXT: &str = "---\ntype: Note\ntitle: Pricing\n---\nPlans start at 10 euros\n";

/// The paths of every knowledge file of the harness's deck, sorted.
fn paths_of(h: &Harness) -> Vec<String> {
    let s = h.session.lock().unwrap_or_else(PoisonError::into_inner);
    let mut paths: Vec<String> = s
        .knowledge_entries(None)
        .unwrap_or_else(|e| panic!("knowledge entries: {e}"))
        .into_iter()
        .map(|e| e.path)
        .collect();
    paths.sort();
    paths
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_write_then_read_returns_the_approved_text() {
    let h = two_files().await;
    let c = server(&h).await;
    let w = call(&c, "knowledge_write", json!({ "path": PRICING, "text": PRICING_TEXT })).await;
    assert_that!(w.is_error).is_none();
    assert_that!(json_of(&w)).is_equal_to(json!({ "path": PRICING }));
    let r = call(&c, "knowledge_read", json!({ "path": PRICING })).await;
    assert_that!(r.is_error).is_none();
    assert_that!(json_of(&r)["text"].as_str().map(unstamped)).is_equal_to(Some(PRICING_TEXT.to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_edit_of_an_approved_file_replaces_the_passage() {
    let h = two_files().await;
    let c = server(&h).await;
    let e = call(&c, "knowledge_edit", json!({ "path": ROADMAP, "old": "nothing here", "new": "ship it" })).await;
    assert_that!(e.is_error).is_none();
    assert_that!(json_of(&e)).is_equal_to(json!({ "path": ROADMAP }));
    let r = call(&c, "knowledge_read", json!({ "path": ROADMAP })).await;
    assert_that!(json_of(&r)["text"].as_str().map(unstamped))
        .is_equal_to(Some(ROADMAP_TEXT.replace("nothing here", "ship it")));
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_move_returns_the_new_path_and_delete_the_deleted_one() {
    let h = two_files().await;
    let c = server(&h).await;
    let m = call(&c, "knowledge_move", json!({ "from": ROADMAP, "to": "/references/plan.md" })).await;
    assert_that!(m.is_error).is_none();
    assert_that!(json_of(&m)).is_equal_to(json!({ "path": "/references/plan.md" }));
    let d = call(&c, "knowledge_delete", json!({ "path": "/references/plan.md" })).await;
    assert_that!(d.is_error).is_none();
    assert_that!(json_of(&d)).is_equal_to(json!({ "path": "/references/plan.md" }));
    assert_that!(paths_of(&h)).is_equal_to(vec![SECRET.to_owned()]);
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_edit_of_an_awaiting_file_is_an_error_in_the_user_language() {
    let h = two_files().await;
    let c = server(&h).await;
    let r = call(&c, "knowledge_edit", json!({ "path": SECRET, "old": "roadmap", "new": "plan" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(h.localizer.locale()).is_equal_to("fr");
    let fr = Localizer::new(Some("fr")).tr_with("session-knowledge-change-awaiting", &[("path", "§".into())]);
    let (head, _) = fr.split_once('§').unwrap_or_else(|| panic!("no path in {fr}"));
    assert_that!(text_of(&r).as_str()).contains(head.trim_end_matches(['\u{2068}', ' ']));
    assert_that!(text_of(&r).as_str()).does_not_contain("roadmap secret");
    // The refused edit changed nothing: the file still awaits.
    let l = call(&c, "knowledge_list", json!({ "dir": "/notes" })).await;
    assert_that!(json_of(&l)).is_equal_to(json!({ "files": [{ "path": SECRET, "approval": "awaiting" }] }));
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_move_of_an_awaiting_file_is_an_error_and_moves_nothing() {
    let h = two_files().await;
    let c = server(&h).await;
    let m = call(&c, "knowledge_move", json!({ "from": SECRET, "to": "/notes/public.md" })).await;
    assert_that!(m.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&m).as_str()).does_not_contain("roadmap secret");
    let l = call(&c, "knowledge_list", json!({ "dir": "/notes" })).await;
    assert_that!(json_of(&l)).is_equal_to(json!({ "files": [{ "path": SECRET, "approval": "awaiting" }] }));
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_write_over_an_awaiting_path_approves_it_and_leaves_the_others_awaiting() {
    let other = "/notes/other.md";
    let h = start_knowledge(
        |_| {},
        Some(vec![ROADMAP_TEXT.to_owned()]),
        vec![
            (ROADMAP.to_owned(), ROADMAP_TEXT.to_owned()),
            (SECRET.to_owned(), SECRET_TEXT.to_owned()),
            (other.to_owned(), "---\ntype: Note\n---\nstill hidden\n".to_owned()),
        ],
        Vec::new(),
    )
    .await;
    let c = server(&h).await;
    let written = call(&c, "knowledge_write", json!({ "path": SECRET, "text": PRICING_TEXT })).await;
    assert_that!(written.is_error).is_none();
    let read = call(&c, "knowledge_read", json!({ "path": SECRET })).await;
    assert_that!(json_of(&read)["text"].as_str().map(unstamped)).is_equal_to(Some(PRICING_TEXT.to_owned()));
    let listed = call(&c, "knowledge_list", json!({ "dir": "/notes" })).await;
    let approvals: Vec<(Value, Value)> = listed_files(&listed);
    assert_that!(approvals).is_equal_to(vec![(json!(other), json!("awaiting")), (json!(SECRET), json!("approved"))]);
}

/// The (path, approval) of each file a `knowledge_list` result lists, sorted by path.
fn listed_files(r: &CallToolResult) -> Vec<(Value, Value)> {
    let mut files: Vec<(Value, Value)> = json_of(r)["files"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| (f["path"].clone(), f["approval"].clone()))
        .collect();
    files.sort_by_key(|(p, _)| p.to_string());
    files
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_turn_writes_undo_in_one_step() {
    let h = two_files().await;
    let before = paths_of(&h);
    let turn = 4;
    h.session.lock().unwrap_or_else(PoisonError::into_inner).begin_turn(turn, "knowledge");
    let c = server(&h).await;
    for (path, text) in [("/notes/a.md", PRICING_TEXT), ("/notes/b.md", PRICING_TEXT)] {
        let w = call(&c, "knowledge_write", json!({ "path": path, "text": text })).await;
        assert_that!(w.is_error).is_none();
    }
    h.session.lock().unwrap_or_else(PoisonError::into_inner).end_turn();
    assert_that!(paths_of(&h).len()).is_equal_to(before.len() + 2);
    let u = call(&c, "undo_turn", json!({ "turn": turn })).await;
    assert_that!(u.is_error).is_not_equal_to(Some(true));
    assert_that!(paths_of(&h)).is_equal_to(before);
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_import_of_an_attached_markdown_writes_a_readable_reference() {
    let _one = IMPORTS.lock().await;
    let h = two_files().await;
    let id = attach(&h, b"# Budget\n\nForty thousand.\n", "Budget 2026.md");
    let c = server(&h).await;
    let r = call(&c, "knowledge_import", json!({ "attachment": id })).await;
    assert_that!(r.is_error).is_none();
    assert_that!(json_of(&r))
        .is_equal_to(json!({ "paths": ["/references/budget-2026.md"], "images": 0, "cut": false }));
    let read = call(&c, "knowledge_read", json!({ "path": "/references/budget-2026.md" })).await;
    assert_that!(read.is_error).is_none();
    assert_that!(json_of(&read)["text"].as_str().unwrap_or_default()).contains("Forty thousand.");
}

/// Attaches `bytes` named `name` to the harness's tab; its id.
fn attach(h: &Harness, bytes: &[u8], name: &str) -> String {
    let upload = oku_slides_session::agent::Upload::new(bytes.to_vec(), name);
    let mut s = h.session.lock().unwrap_or_else(PoisonError::into_inner);
    s.attach(upload, &[]).unwrap_or_else(|e| panic!("attach: {e}")).id().to_owned()
}

/// The text of the knowledge file `path`, read through `knowledge_read`.
async fn read_text(c: &rust_mcp_sdk::mcp_client::ClientRuntime, path: &str) -> String {
    let read = call(c, "knowledge_read", json!({ "path": path })).await;
    assert_that!(read.is_error).is_none();
    json_of(&read)["text"].as_str().unwrap_or_default().to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_import_with_a_path_replaces_the_earlier_import() {
    let _one = IMPORTS.lock().await;
    let h = two_files().await;
    let first = attach(&h, b"# Budget\n\nForty thousand.\n", "Budget 2026.md");
    let second = attach(&h, b"# Budget\n\nFifty thousand.\n", "Budget v2.md");
    let c = server(&h).await;
    let r = call(&c, "knowledge_import", json!({ "attachment": first })).await;
    assert_that!(r.is_error).is_none();
    let path = "/references/budget-2026.md";
    let r = call(&c, "knowledge_import", json!({ "attachment": second, "path": path })).await;
    assert_that!(r.is_error).is_none();
    assert_that!(json_of(&r)).is_equal_to(json!({ "paths": [path], "images": 0, "cut": false }));
    let text = read_text(&c, path).await;
    assert_that!(text.as_str()).contains("Fifty thousand.");
    assert_that!(text.as_str()).does_not_contain("Forty thousand.");
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_import_onto_an_edited_import_is_an_error_in_the_user_language() {
    let _one = IMPORTS.lock().await;
    let h = two_files().await;
    let first = attach(&h, b"# Budget\n\nForty thousand.\n", "Budget 2026.md");
    let second = attach(&h, b"# Budget\n\nFifty thousand.\n", "Budget v2.md");
    let c = server(&h).await;
    let path = "/references/budget-2026.md";
    assert_that!(call(&c, "knowledge_import", json!({ "attachment": first })).await.is_error).is_none();
    let edit = call(&c, "knowledge_edit", json!({ "path": path, "old": "Forty", "new": "Sixty" })).await;
    assert_that!(edit.is_error).is_none();
    let r = call(&c, "knowledge_import", json!({ "attachment": second, "path": path })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    let expected = Localizer::new(Some("fr")).tr_with("session-import-target-edited", &[("path", path.into())]);
    assert_that!(text_of(&r).as_str()).contains(expected.as_str());
    assert_that!(read_text(&c, path).await.as_str()).contains("Sixty thousand.");
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_import_of_an_unknown_attachment_is_an_error_in_the_user_language() {
    let h = two_files().await;
    let before = paths_of(&h);
    let c = server(&h).await;
    let r = call(&c, "knowledge_import", json!({ "attachment": "doc-absent" })).await;
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(&r).as_str()).contains(Localizer::new(Some("fr")).tr("session-attachment-gone").as_str());
    assert_that!(paths_of(&h)).is_equal_to(before);
}
