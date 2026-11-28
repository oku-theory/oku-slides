use oku_slides_core::knowledge::limits::MAX_FILE_BYTES;
use oku_slides_core::{KnowledgeFile, Op, asset_id, kinds};
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;
use tempfile::TempDir;

use crate::knowledge_fixture::{approve_knowledge, seed_knowledge};
use crate::knowledge_read::MAX_KNOWLEDGE_ASSET_BYTES;
use crate::pack::ESSENTIEL_PACK;
use crate::request::Request;
use crate::test_support::{fresh_dir, resumed_on, session_on, white_png};
use crate::theme::PAPIER_THEME;
use crate::{KnowledgeApproval, Session, SessionErrorKind, Workspace};

const NOTE: &str = "---\ntype: Note\ntitle: Hello\n---\nbody\n";
const RECEIVED: &str = "---\ntype: Note\n---\nreceived, maybe injected\n";

/// A session on a fresh data dir whose deck holds `files` (path, text) and `assets`, `approved` texts approved.
fn seeded(files: &[(&str, &str)], assets: Vec<Vec<u8>>, approved: &[&str]) -> (TempDir, Session) {
    let data = fresh_dir("oku-slides-knowledge-read-");
    approve_knowledge(data.path(), approved);
    let mut s = session_on(data.path());
    seed_knowledge(&mut s, files, assets);
    (data, s)
}

fn id_at(s: &Session, path: &str) -> String {
    s.deck().knowledge.iter().find(|(_, f)| f.path == path).map(|(id, _)| id.clone()).expect("no file at this path")
}

fn call(s: &mut Session, req: Request) -> Value {
    serde_json::from_str(&s.handle(req).unwrap()).unwrap()
}

fn refusal(s: &mut Session, req: Request) -> SessionErrorKind {
    s.handle(req).unwrap_err().kind()
}

fn page_of(id: &str) -> Request {
    Request::KnowledgePage { id: id.to_owned() }
}

fn asset_of(id: &str, asset: &str) -> Request {
    Request::KnowledgeAsset { id: id.to_owned(), asset: asset.to_owned() }
}

/// A text citing the image of `bytes`.
fn citing(bytes: &[u8]) -> String {
    let hex = asset_id(bytes).strip_prefix("sha256:").map(str::to_owned).unwrap_or_default();
    format!("---\ntype: Note\n---\n![alt](/references/assets/{hex}.png)\n")
}

fn slide_added() -> Op {
    Op::AddSlide { index: None, defaults: false, slide: oku_slides_core::Slide::new("neuve", kinds::essentiel::BLANK) }
}

#[test]
fn knowledge_tree_lists_every_file_with_awaiting_flag() {
    let (_data, mut s) = seeded(&[("/z.md", RECEIVED), ("/a.md", NOTE)], Vec::new(), &[NOTE]);
    let tree = call(&mut s, Request::KnowledgeTree);
    let files = tree["files"].as_array().unwrap();
    assert_that!(files.len()).is_equal_to(2);
    assert_that!(files[0]).is_equal_to(json!({
        "id": id_at(&s, "/a.md"), "path": "/a.md", "title": "Hello", "kind": "Note", "awaiting": false,
    }));
    assert_that!(files[1]).is_equal_to(json!({
        "id": id_at(&s, "/z.md"), "path": "/z.md", "title": null, "kind": null, "awaiting": true,
    }));
}

#[test]
fn knowledge_tree_and_page_long_path_comes_back_whole() {
    let path = format!("/references/{}/{}.md", "a".repeat(40), "b".repeat(60));
    let (_data, mut s) = seeded(&[(path.as_str(), NOTE)], Vec::new(), &[NOTE]);
    let tree = call(&mut s, Request::KnowledgeTree);
    assert_that!(tree["files"][0]["path"].as_str()).is_equal_to(Some(path.as_str()));
    let id = id_at(&s, &path);
    let page = call(&mut s, page_of(&id));
    assert_that!(page["path"].as_str()).is_equal_to(Some(path.as_str()));
}

#[test]
fn knowledge_tree_and_page_blank_values_show_nothing() {
    let text = "---\ntype: \"  \"\ntitle: \" \"\ntags: [\" \", a]\n---\nb\n";
    let (_data, mut s) = seeded(&[("/a.md", text)], Vec::new(), &[text]);
    let tree = call(&mut s, Request::KnowledgeTree);
    assert_that!(tree["files"][0]["title"]).is_equal_to(&Value::Null);
    assert_that!(tree["files"][0]["kind"]).is_equal_to(&Value::Null);
    let id = id_at(&s, "/a.md");
    let page = call(&mut s, page_of(&id));
    assert_that!(page["header"]["title"]).is_equal_to(&Value::Null);
    assert_that!(page["header"]["kind"]).is_equal_to(&Value::Null);
    assert_that!(page["header"]["tags"]).is_equal_to(json!(["a"]));
}

#[test]
fn knowledge_tree_reuses_scan_cache() {
    let (_data, s) = seeded(&[("/a.md", NOTE), ("/b.md", RECEIVED)], Vec::new(), &[NOTE]);
    s.knowledge_tree().unwrap();
    s.knowledge_tree().unwrap();
    assert_that!(s.knowledge_summary.computations()).is_equal_to(1);
}

#[test]
fn knowledge_page_splits_header_and_body() {
    let text = "---\ntype: Note\ntitle: Hello\ntags: [a, b]\nverified: gold\n---\nthe body\n";
    let (_data, mut s) = seeded(&[("/a.md", text)], Vec::new(), &[text]);
    let id = id_at(&s, "/a.md");
    let page = call(&mut s, page_of(&id));
    assert_that!(page["id"].as_str()).is_equal_to(Some(id.as_str()));
    assert_that!(page["path"].as_str()).is_equal_to(Some("/a.md"));
    assert_that!(page["awaiting"].as_bool()).is_equal_to(Some(false));
    assert_that!(page["body"].as_str()).is_equal_to(Some("the body\n"));
    assert_that!(page["truncated_bytes"].as_u64()).is_equal_to(Some(0));
    assert_that!(page["header"]["kind"].as_str()).is_equal_to(Some("Note"));
    assert_that!(page["header"]["title"].as_str()).is_equal_to(Some("Hello"));
    assert_that!(page["header"]["tags"]).is_equal_to(json!(["a", "b"]));
    assert_that!(page["header"]["verified"].as_str()).is_equal_to(Some("gold"));
    assert_that!(page["header"]["stale"].as_bool()).is_equal_to(Some(false));
}

#[test]
fn knowledge_page_header_strings_are_cleaned() {
    let text = "---\ntitle: \"Plan \u{202e}secret\u{202c} d\u{2019}action\"\n---\nb\n";
    let (_data, mut s) = seeded(&[("/a.md", text)], Vec::new(), &[text]);
    let id = id_at(&s, "/a.md");
    let page = call(&mut s, page_of(&id));
    assert_that!(page["header"]["title"].as_str()).is_equal_to(Some("Plan secret dʼaction"));
}

#[test]
fn knowledge_page_unparsable_frontmatter_whole_text_as_body() {
    let text = "---\na: &x 1\nb: *x\n---\nbody\n";
    let (_data, mut s) = seeded(&[("/a.md", text)], Vec::new(), &[text]);
    let id = id_at(&s, "/a.md");
    let page = call(&mut s, page_of(&id));
    assert_that!(page["header"]).is_equal_to(&Value::Null);
    assert_that!(page["header_invalid"].as_bool()).is_equal_to(Some(true));
    assert_that!(page["body"].as_str()).is_equal_to(Some(text));
}

#[test]
fn knowledge_page_without_frontmatter_header_not_invalid() {
    let text = "just a body\n";
    let (_data, mut s) = seeded(&[("/a.md", text)], Vec::new(), &[text]);
    let id = id_at(&s, "/a.md");
    let page = call(&mut s, page_of(&id));
    assert_that!(page["header"]).is_equal_to(&Value::Null);
    assert_that!(page["header_invalid"].as_bool()).is_equal_to(Some(false));
}

#[test]
fn knowledge_page_awaiting_whole_text_no_header_no_images() {
    let png = white_png(2, 2);
    let text = citing(&png);
    let (_data, mut s) = seeded(&[("/a.md", &text)], vec![png], &[]);
    let id = id_at(&s, "/a.md");
    let page = call(&mut s, page_of(&id));
    assert_that!(page["awaiting"].as_bool()).is_equal_to(Some(true));
    assert_that!(page["header"]).is_equal_to(&Value::Null);
    assert_that!(page["body"].as_str()).is_equal_to(Some(text.as_str()));
    assert_that!(page["images"]).is_equal_to(json!([]));
}

#[test]
fn knowledge_page_past_max_bytes_truncated_counted() {
    let text = format!("---\ntype: Note\n---\n{}", "a".repeat(MAX_FILE_BYTES));
    let (_data, mut s) = seeded(&[("/big.md", &text)], Vec::new(), &[&text]);
    let id = id_at(&s, "/big.md");
    let page = call(&mut s, page_of(&id));
    let cut = "---\ntype: Note\n---\n".len();
    assert_that!(page["truncated_bytes"].as_u64()).is_equal_to(Some(cut as u64));
    assert_that!(page["body"].as_str().map(str::len)).is_equal_to(Some(MAX_FILE_BYTES - cut));
}

#[test]
fn knowledge_page_lists_cited_images() {
    let png = white_png(2, 2);
    let text = citing(&png);
    let (_data, mut s) = seeded(&[("/a.md", &text)], vec![png.clone()], &[&text]);
    let id = id_at(&s, "/a.md");
    let page = call(&mut s, page_of(&id));
    let asset = asset_id(&png);
    let hex = asset.strip_prefix("sha256:").unwrap();
    assert_that!(page["images"])
        .is_equal_to(json!([{"path": format!("/references/assets/{hex}.png"), "asset": asset}]));
}

#[test]
fn knowledge_page_unknown_id_refused() {
    let (_data, mut s) = seeded(&[("/a.md", NOTE)], Vec::new(), &[NOTE]);
    assert_that!(refusal(&mut s, page_of(&"f".repeat(32)))).is_equal_to(SessionErrorKind::KnowledgeIdNotFound);
}

#[test]
fn knowledge_asset_cited_png_answers_base64() {
    use base64::Engine;
    let png = white_png(2, 2);
    let text = citing(&png);
    let (_data, mut s) = seeded(&[("/a.md", &text)], vec![png.clone()], &[&text]);
    let id = id_at(&s, "/a.md");
    let reply = call(&mut s, asset_of(&id, &asset_id(&png)));
    assert_that!(reply["mime"].as_str()).is_equal_to(Some("image/png"));
    let data = base64::engine::general_purpose::STANDARD.decode(reply["data"].as_str().unwrap()).unwrap();
    assert_that!(data).is_equal_to(png);
}

/// A scenario of an unavailable asset: the session, and the id and asset asked.
fn unavailable(case: &str) -> (TempDir, Session, String, String) {
    let img = white_png(2, 2);
    let other = white_png(3, 3);
    let cites = citing(&img);
    match case {
        "uncited" => {
            let (d, s) = seeded(&[("/a.md", NOTE)], vec![img.clone()], &[NOTE]);
            let id = id_at(&s, "/a.md");
            (d, s, id, asset_id(&img))
        }
        "awaiting_file" => {
            let (d, s) = seeded(&[("/a.md", &cites)], vec![img.clone()], &[]);
            let id = id_at(&s, "/a.md");
            (d, s, id, asset_id(&img))
        }
        "cited_only_by_an_awaiting_file" => {
            let (d, s) = seeded(&[("/a.md", NOTE), ("/b.md", &cites)], vec![img.clone()], &[NOTE]);
            let id = id_at(&s, "/a.md");
            (d, s, id, asset_id(&img))
        }
        "edited_since" => {
            let (d, mut s) = seeded(&[("/a.md", &cites)], vec![img.clone()], &[&cites]);
            let id = id_at(&s, "/a.md");
            assert_that!(s.knowledge_asset(&id, &asset_id(&img)).is_ok()).is_true();
            let mut deck = s.deck().clone();
            deck.knowledge.insert(id.clone(), KnowledgeFile { path: "/a.md".into(), text: format!("{cites}more\n") });
            s.doc.sources.apply_silent(deck).unwrap();
            (d, s, id, asset_id(&img))
        }
        "svg" => {
            let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".to_vec();
            let text = citing(&svg);
            let (d, s) = seeded(&[("/a.md", &text)], vec![svg.clone()], &[&text]);
            let id = id_at(&s, "/a.md");
            (d, s, id, asset_id(&svg))
        }
        "missing" => {
            let (d, s) = seeded(&[("/a.md", &cites)], Vec::new(), &[&cites]);
            let id = id_at(&s, "/a.md");
            (d, s, id, asset_id(&img))
        }
        "too_large" => {
            let mut big = img.clone();
            big.resize(MAX_KNOWLEDGE_ASSET_BYTES + 1, 0);
            let text = citing(&big);
            let (d, s) = seeded(&[("/a.md", &text)], vec![big.clone()], &[&text]);
            let id = id_at(&s, "/a.md");
            (d, s, id, asset_id(&big))
        }
        "malformed_id" => {
            let (d, s) = seeded(&[("/a.md", &cites)], vec![img.clone()], &[&cites]);
            (d, s, "../a".to_owned(), asset_id(&img))
        }
        "malformed_asset" => {
            let (d, s) = seeded(&[("/a.md", &cites)], vec![img.clone()], &[&cites]);
            let id = id_at(&s, "/a.md");
            (d, s, id, "sha256:zz".to_owned())
        }
        "uppercase_asset" => {
            let (d, s) = seeded(&[("/a.md", &cites)], vec![img.clone()], &[&cites]);
            let id = id_at(&s, "/a.md");
            (d, s, id, asset_id(&img).to_uppercase().replace("SHA256:", "sha256:"))
        }
        "short_asset" => {
            let (d, s) = seeded(&[("/a.md", &cites)], vec![img.clone()], &[&cites]);
            let id = id_at(&s, "/a.md");
            (d, s, id, asset_id(&img)[..20].to_owned())
        }
        "unprefixed_asset" => {
            let (d, s) = seeded(&[("/a.md", &cites)], vec![img.clone()], &[&cites]);
            let id = id_at(&s, "/a.md");
            (d, s, id, asset_id(&img).replace("sha256:", ""))
        }
        "unknown_asset" => {
            let (d, s) = seeded(&[("/a.md", &cites)], vec![img.clone(), other.clone()], &[&cites]);
            let id = id_at(&s, "/a.md");
            (d, s, id, asset_id(&other))
        }
        other_case => unreachable!("{other_case}"),
    }
}

#[rstest]
#[case::uncited("uncited")]
#[case::awaiting_file("awaiting_file")]
#[case::cited_only_by_an_awaiting_file("cited_only_by_an_awaiting_file")]
#[case::edited_since("edited_since")]
#[case::svg("svg")]
#[case::missing("missing")]
#[case::too_large("too_large")]
#[case::malformed_id("malformed_id")]
#[case::malformed_asset("malformed_asset")]
#[case::uppercase_asset("uppercase_asset")]
#[case::short_asset("short_asset")]
#[case::unprefixed_asset("unprefixed_asset")]
#[case::unknown_asset("unknown_asset")]
fn knowledge_asset_unavailable_cases_are_refused(#[case] case: &str) {
    let (_data, mut s, id, asset) = unavailable(case);
    assert_that!(refusal(&mut s, asset_of(&id, &asset))).is_equal_to(SessionErrorKind::KnowledgeAssetUnavailable);
}

#[test]
fn knowledge_asset_malformed_asset_localized_fr() {
    let (_data, s, id, asset) = unavailable("uppercase_asset");
    let err = s.knowledge_asset(&id, &asset).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::KnowledgeAssetUnavailable);
    let (fr, en) = (Localizer::new(Some("fr")), Localizer::new(Some("en")));
    assert_that!(err.localize(&fr)).contains("n'est pas disponible");
    assert_that!(err.localize(&fr)).is_not_equal_to(err.localize(&en));
}

#[test]
fn knowledge_asset_unavailable_localized_fr() {
    let (_data, s, id, asset) = unavailable("uncited");
    let err = s.knowledge_asset(&id, &asset).unwrap_err();
    let (fr, en) = (Localizer::new(Some("fr")), Localizer::new(Some("en")));
    assert_that!(err.localize(&fr)).contains("sha256:");
    assert_that!(err.localize(&fr)).is_not_equal_to(err.localize(&en));
}

/// The three reading requests, each once.
fn reading_requests(s: &Session) -> [Request; 3] {
    let id = s.deck().knowledge.keys().next().cloned().unwrap_or_else(|| "0".repeat(32));
    [Request::KnowledgeTree, page_of(&id), asset_of(&id, "sha256:00")]
}

#[test]
fn knowledge_requests_in_history_refused() {
    let (_data, mut s) = seeded(&[("/a.md", NOTE)], Vec::new(), &[NOTE]);
    s.apply("x", vec![slide_added()], None).unwrap();
    s.history_open().unwrap();
    for req in reading_requests(&s) {
        assert_that!(refusal(&mut s, req)).is_equal_to(SessionErrorKind::InHistory);
    }
}

#[test]
fn knowledge_requests_help_tab_refused() {
    let data = fresh_dir("oku-slides-knowledge-read-tabs-");
    let mut pack = session_on(data.path());
    pack.open_pack_tab_here(ESSENTIEL_PACK, None).unwrap();
    let mut theme = session_on(data.path());
    theme.open_theme_here(PAPIER_THEME).unwrap();
    let mut help = session_on(data.path());
    help.open_help("fr", None).unwrap();
    for s in [&mut pack, &mut theme, &mut help] {
        for req in reading_requests(s) {
            assert_that!(refusal(s, req)).is_equal_to(SessionErrorKind::WrongDocument);
        }
    }
}

#[test]
fn tab_info_knowledge_changes_on_write_and_on_approval() {
    let (_data, mut s) = seeded(&[("/a.md", RECEIVED)], Vec::new(), &[]);
    let first = s.tab_info().knowledge.unwrap();
    let write = Op::KnowledgeWrite { path: "/b.md".into(), text: NOTE.into(), id: None };
    s.apply("x", vec![write], None).unwrap();
    let written = s.tab_info().knowledge.unwrap();
    assert_that!(written.generation).is_not_equal_to(first.generation);
    let id = id_at(&s, "/a.md");
    let approval = KnowledgeApproval { id, hash: asset_id(RECEIVED.as_bytes()) };
    s.handle(Request::ApproveKnowledge { files: vec![approval] }).unwrap();
    let approved = s.tab_info().knowledge.unwrap();
    assert_that!(approved.generation).is_equal_to(written.generation);
    assert_that!(approved.approvals).is_equal_to(written.approvals + 1);
}

#[test]
fn tab_info_knowledge_none_outside_deck() {
    let data = fresh_dir("oku-slides-knowledge-read-tabs-");
    let mut pack = session_on(data.path());
    pack.open_pack_tab_here(ESSENTIEL_PACK, None).unwrap();
    let mut theme = session_on(data.path());
    theme.open_theme_here(PAPIER_THEME).unwrap();
    let mut help = session_on(data.path());
    help.open_help("fr", None).unwrap();
    for s in [&pack, &theme, &help] {
        assert_that!(s.tab_info().knowledge).is_none();
    }
}

#[test]
fn tab_info_knowledge_in_history_reads_live_deck() {
    let (_data, mut s) = seeded(&[("/a.md", NOTE)], Vec::new(), &[NOTE]);
    s.apply("x", vec![slide_added()], None).unwrap();
    let live = s.tab_info().knowledge;
    assert_that!(live).is_some();
    assert_that!(live.map(|k| k.in_history)).is_equal_to(Some(false));
    s.history_open().unwrap();
    let past = s.tab_info().knowledge;
    assert_that!(past.map(|k| k.in_history)).is_equal_to(Some(true));
    assert_that!(past.map(|k| (k.generation, k.approvals))).is_equal_to(live.map(|k| (k.generation, k.approvals)));
    s.history_close().unwrap();
    assert_that!(s.tab_info().knowledge).is_equal_to(live);
}

/// An awake deck tab on a resumed workspace, whose layout file is `dir/workspace.json`.
fn awake_tab(data: &std::path::Path) -> (std::sync::Mutex<Workspace>, String, std::path::PathBuf) {
    let dir = data.join("resume");
    let ws = resumed_on(data, &dir);
    let tab = Workspace::lock(&ws).view().windows[0].tabs[0].id.clone();
    Workspace::with_session(&ws, &tab, |s| s.tab_info()).0.unwrap();
    let file = dir.join("workspace.json");
    (ws, tab, file)
}

/// Renames the deck of `tab`: a change the saved layout keeps.
fn rename(ws: &std::sync::Mutex<Workspace>, tab: &str, title: &str) {
    let op = crate::test_support::rename_title(title);
    Workspace::with_session(ws, tab, |s| s.apply("x", vec![op], None)).0.unwrap().unwrap();
}

#[test]
fn workspace_view_serializes_knowledge_saved_layout_omits_it() {
    let data = fresh_dir("oku-slides-knowledge-read-layout-");
    let (ws, tab, file) = awake_tab(data.path());
    Workspace::with_session(&ws, &tab, |s| seed_knowledge(s, &[("/a.md", NOTE)], Vec::new())).0.unwrap();
    let view = serde_json::to_value(Workspace::lock(&ws).view()).unwrap();
    let revision = &view["windows"][0]["tabs"][0]["knowledge"];
    assert_that!(revision["approvals"].is_u64()).is_true();
    assert_that!(revision["generation"].is_u64()).is_true();
    rename(&ws, &tab, "Avant");
    let saved: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_that!(saved["info"][tab.as_str()].as_object().unwrap().contains_key("knowledge")).is_false();
}

#[test]
fn knowledge_revision_change_alone_does_not_save_layout() {
    let data = fresh_dir("oku-slides-knowledge-read-layout-");
    let (ws, tab, file) = awake_tab(data.path());
    rename(&ws, &tab, "Avant");
    std::fs::remove_file(&file).unwrap();
    let (result, changed) = Workspace::with_session(&ws, &tab, |s| seed_knowledge(s, &[("/a.md", NOTE)], Vec::new()));
    result.unwrap();
    assert_that!(changed).is_true();
    assert_that!(file.exists()).is_false();
    let (renamed, changed) = Workspace::with_session(&ws, &tab, |s| {
        s.apply("x", vec![crate::test_support::rename_title("Autre")], None).unwrap();
    });
    renamed.unwrap();
    assert_that!(changed).is_true();
    assert_that!(file.exists()).is_true();
}
