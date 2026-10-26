use std::path::PathBuf;
use std::sync::Mutex;

use rstest::rstest;
use serde::Deserialize;
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::*;
use crate::Session;
use crate::test_support::{apply_ws, fresh_dir, workspace_platform};
use crate::workspace::relock;

const PACK: Uuid = Uuid::from_u128(0xa7e11e40_0000_4000_8000_000000000001);

/// A saved layout holding `entries` as the info of tabs `t-1`, `t-2`, ...
fn layout_of(entries: &[Value]) -> SavedLayout {
    let info: serde_json::Map<String, Value> =
        entries.iter().enumerate().map(|(i, e)| (format!("t-{}", i + 1), e.clone())).collect();
    SavedLayout { layout: Layout::new(), info: Value::Object(info) }
}

/// A legacy entry (no `document`) with the given legacy keys.
fn legacy(path: Value, deck_uid: Value, help: Value) -> Value {
    let mut entry = json!({"title": "t", "dirty": false, "pristine": false});
    for (key, value) in [("path", path), ("deck_uid", deck_uid), ("help", help)] {
        entry[key] = value;
    }
    entry
}

fn deck(path: Option<&str>, uid: Option<&str>) -> TabDocument {
    TabDocument::Deck { path: path.map(PathBuf::from), deck_uid: uid.map(str::to_owned) }
}

fn info_of(document: TabDocument) -> TabInfo {
    TabInfo { title: "t".to_owned(), dirty: false, pristine: false, editing_pack: false, theme: None, document }
}

#[test]
fn layout_entry_without_document_becomes_a_help_document() {
    let mut file = layout_of(&[legacy(Value::Null, json!("u-1"), json!("fr"))]);
    let info = file.take_info("t-1");
    assert_that!(info.map(|i| i.document)).is_equal_to(Some(TabDocument::Help { locale: "fr".to_owned() }));
}

#[test]
fn layout_entry_without_document_keeps_path_and_uid() {
    let mut file = layout_of(&[legacy(json!("/d/a.oku-slides.pdf"), json!("u-1"), Value::Null)]);
    let info = file.take_info("t-1");
    assert_that!(info.map(|i| i.document)).is_equal_to(Some(deck(Some("/d/a.oku-slides.pdf"), Some("u-1"))));
}

#[test]
fn layout_entry_without_document_and_without_help_key_is_a_deck() {
    let mut entry = legacy(json!("/d/a.oku-slides.pdf"), json!("u-1"), Value::Null);
    entry.as_object_mut().map(|o| o.remove("help"));
    let info = layout_of(&[entry]).take_info("t-1");
    assert_that!(info.map(|i| i.document)).is_equal_to(Some(deck(Some("/d/a.oku-slides.pdf"), Some("u-1"))));
}

#[test]
fn layout_entry_without_document_with_editing_pack_stays_a_deck() {
    let mut entry = legacy(json!("/d/a.oku-slides.pdf"), json!("u-1"), Value::Null);
    entry["editing_pack"] = json!(true);
    let info = layout_of(&[entry]).take_info("t-1");
    assert_that!(info.as_ref().map(|i| i.editing_pack)).is_equal_to(Some(true));
    assert_that!(info.map(|i| i.document)).is_equal_to(Some(deck(Some("/d/a.oku-slides.pdf"), Some("u-1"))));
}

#[test]
fn layout_entry_with_a_non_string_help_costs_only_itself() {
    let mut file =
        layout_of(&[legacy(Value::Null, Value::Null, json!(3)), legacy(Value::Null, json!("u-2"), Value::Null)]);
    assert_that!(file.take_info("t-1")).is_none();
    assert_that!(file.take_info("t-2").map(|i| i.document)).is_equal_to(Some(deck(None, Some("u-2"))));
}

#[test]
fn layout_unparsable_entry_costs_only_itself() {
    let mut broken = legacy(Value::Null, Value::Null, Value::Null);
    broken["title"] = json!(5);
    let mut file = layout_of(&[broken, json!("not an object"), legacy(Value::Null, json!("u-3"), Value::Null)]);
    assert_that!(file.take_info("t-1")).is_none();
    assert_that!(file.take_info("t-2")).is_none();
    assert_that!(file.take_info("t-3").map(|i| i.document)).is_equal_to(Some(deck(None, Some("u-3"))));
}

#[test]
fn layout_help_entry_with_an_unknown_locale_costs_only_itself() {
    let mut unknown = legacy(Value::Null, Value::Null, Value::Null);
    unknown["document"] = json!({"kind": "help", "locale": "xx"});
    let mut file = layout_of(&[unknown, legacy(Value::Null, json!("u-2"), json!("en"))]);
    assert_that!(file.take_info("t-1")).is_none();
    assert_that!(file.take_info("t-2").map(|i| i.document))
        .is_equal_to(Some(TabDocument::Help { locale: "en".to_owned() }));
}

#[rstest]
#[case::absolute_normalized("/d/a.oku-slides.pdf", Some("/d/a.oku-slides.pdf"))]
#[case::relative("d/a.oku-slides.pdf", None)]
#[case::wrong_extension("/d/a.pdf", None)]
#[case::not_normalized("/x/a/../b.oku-slides.pdf", None)]
fn layout_deck_entry_path_is_kept_only_when_valid(#[case] path: &str, #[case] kept: Option<&str>) {
    let info = layout_of(&[legacy(json!(path), Value::Null, Value::Null)]).take_info("t-1");
    assert_that!(info.map(|i| i.document)).is_equal_to(Some(deck(kept, None)));
}

#[rstest]
#[case::sixty_four_valid_characters(&"0-aZ".repeat(16), true)]
#[case::sixty_five_characters(&format!("{}x", "0-aZ".repeat(16)), false)]
#[case::with_a_slash("ab/cd", false)]
fn layout_deck_entry_uid_is_kept_only_when_valid(#[case] uid: &str, #[case] kept: bool) {
    let info = layout_of(&[legacy(Value::Null, json!(uid), Value::Null)]).take_info("t-1");
    let expected = deck(None, kept.then_some(uid));
    assert_that!(info.map(|i| i.document)).is_equal_to(Some(expected));
}

#[test]
fn layout_entry_with_a_document_ignores_the_legacy_keys() {
    let mut entry = legacy(json!("/d/a.oku-slides.pdf"), json!("u-1"), json!("fr"));
    entry["document"] = json!({"kind": "pack", "pack": PACK});
    let info = layout_of(&[entry]).take_info("t-1");
    assert_that!(info.map(|i| i.document)).is_equal_to(Some(TabDocument::Pack { pack: PACK }));
}

#[test]
fn layout_entry_title_is_a_label() {
    let mut entry = legacy(Value::Null, Value::Null, Value::Null);
    entry["title"] = json!("a\u{202E}\nb \"c\"");
    let info = layout_of(&[entry]).take_info("t-1");
    assert_that!(info.map(|i| i.title)).is_equal_to(Some("a b \u{2bc}c\u{2bc}".to_owned()));
}

fn saved_entry(document: TabDocument) -> Value {
    serde_json::to_value(SavedTabInfo::new(&info_of(document))).unwrap_or_else(|e| panic!("entrée : {e}"))
}

#[rstest]
#[case::deck(
    deck(Some("/d/a.oku-slides.pdf"), Some("u-1")),
    json!({"kind": "deck", "path": "/d/a.oku-slides.pdf", "deck_uid": "u-1"}),
    json!("/d/a.oku-slides.pdf"), json!("u-1"), Value::Null
)]
#[case::help(
    TabDocument::Help { locale: "fr".to_owned() },
    json!({"kind": "help", "locale": "fr"}),
    Value::Null, Value::Null, json!("fr")
)]
#[case::pack(
    TabDocument::Pack { pack: PACK },
    json!({"kind": "pack", "pack": PACK}),
    Value::Null, Value::Null, Value::Null
)]
fn saved_layout_entry_carries_the_legacy_keys(
    #[case] document: TabDocument,
    #[case] written: Value,
    #[case] path: Value,
    #[case] deck_uid: Value,
    #[case] help: Value,
) {
    let expected = json!({
        "title": "t", "dirty": false, "pristine": false, "editing_pack": false, "theme": null,
        "document": written, "path": path, "deck_uid": deck_uid, "help": help,
    });
    assert_that!(saved_entry(document)).is_equal_to(expected);
}

/// The previous release's `TabInfo`, copied from `views.rs` at 8a8d528d.
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct PreviousTabInfo {
    title: String,
    dirty: bool,
    #[serde(default)]
    editing_pack: bool,
    path: Option<PathBuf>,
    pristine: bool,
    #[serde(default)]
    deck_uid: Option<String>,
    #[serde(default)]
    help: Option<String>,
    #[serde(default)]
    theme: Option<uuid::Uuid>,
}

#[rstest]
#[case::deck(deck(Some("/d/a.oku-slides.pdf"), Some("u-1")), Some("/d/a.oku-slides.pdf"), Some("u-1"), None)]
#[case::help(TabDocument::Help { locale: "fr".to_owned() }, None, None, Some("fr"))]
#[case::pack(TabDocument::Pack { pack: PACK }, None, None, None)]
fn saved_layout_entry_parses_as_the_previous_tab_info(
    #[case] document: TabDocument,
    #[case] path: Option<&str>,
    #[case] deck_uid: Option<&str>,
    #[case] help: Option<&str>,
) {
    let previous: PreviousTabInfo =
        serde_json::from_value(saved_entry(document)).unwrap_or_else(|e| panic!("entrée illisible : {e}"));
    assert_that!(previous.path).is_equal_to(path.map(PathBuf::from));
    assert_that!(previous.deck_uid.as_deref()).is_equal_to(deck_uid);
    assert_that!(previous.help.as_deref()).is_equal_to(help);
}

/// A workspace left by the previous release: three tabs whose path, identity and locale all differ.
struct Previous {
    /// Keeps the folder alive.
    data: tempfile::TempDir,
    dir: PathBuf,
    /// Each tab's id and the document the previous release meant.
    tabs: Vec<(String, TabDocument)>,
}

/// A real `.oku-slides.pdf` named `name` in `data`.
fn saved_pdf(data: &Path, name: &str) -> PathBuf {
    let fixture = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    let mut s = Session::new(Fonts::default(), &workspace_platform(data)).unwrap_or_else(|e| panic!("session : {e}"));
    s.open(&fixture).unwrap_or_else(|e| panic!("ouverture : {e}"));
    let path = data.join(format!("{name}.oku-slides.pdf"));
    s.save(Some(&path)).unwrap_or_else(|e| panic!("enregistrement : {e}"));
    path
}

fn resumed(previous: &Previous) -> Mutex<Workspace> {
    let platform = workspace_platform(previous.data.path());
    Mutex::new(
        Workspace::resume(Fonts::default(), &platform, &previous.dir).unwrap_or_else(|e| panic!("reprise : {e}")),
    )
}

fn documents(ws: &Mutex<Workspace>) -> Vec<(String, TabDocument)> {
    let v = Workspace::lock(ws).view();
    v.windows.iter().flat_map(|w| &w.tabs).map(|t| (t.id.clone(), t.info.document.clone())).collect()
}

/// Real tab states, then a `workspace.json` written by hand as the previous release did.
fn previous_release() -> Previous {
    let data = fresh_dir("oku-slides-resume-documents-");
    let dir = data.path().join("resume");
    let (a, b) = (saved_pdf(data.path(), "a"), saved_pdf(data.path(), "b"));
    let first = Mutex::new(
        Workspace::resume(Fonts::default(), &workspace_platform(data.path()), &dir).unwrap_or_else(|e| panic!("{e}")),
    );
    for path in [&a, &b] {
        apply_ws(&first, json!({"cmd": "open_tab", "window": "w-1", "path": path})).unwrap_or_else(|e| panic!("{e}"));
    }
    apply_ws(&first, json!({"cmd": "open_help", "window": "w-1"})).unwrap_or_else(|e| panic!("{e}"));
    let ids: Vec<String> = documents(&first).into_iter().map(|(id, _)| id).collect();
    let uid_of = |id: &str| {
        Workspace::with_session(&first, id, |s| s.tab_info().document.deck_uid().map(str::to_owned)).0.ok().flatten()
    };
    let tabs: Vec<(String, TabDocument)> = ids
        .iter()
        .zip([Some(&a), Some(&b), None])
        .map(|(id, path)| {
            let document = match path {
                Some(p) => TabDocument::Deck { path: Some(resolve(p)), deck_uid: uid_of(id) },
                None => TabDocument::Help { locale: "fr".to_owned() },
            };
            (id.clone(), document)
        })
        .collect();
    Workspace::settle(&first);
    drop(first);
    let file = dir.join("workspace.json");
    let mut layout: Value = serde_json::from_slice(&std::fs::read(&file).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    let entry = |title: &str, path: Option<&PathBuf>, uid: &str, help: Option<&str>| {
        json!({"title": title, "dirty": false, "pristine": false, "editing_pack": false, "theme": null,
               "path": path.map(|p| resolve(p)), "deck_uid": uid, "help": help})
    };
    layout["info"] = json!({
        ids[0].as_str(): entry("a", Some(&a), tabs[0].1.deck_uid().unwrap_or_default(), None),
        ids[1].as_str(): entry("b", Some(&b), tabs[1].1.deck_uid().unwrap_or_default(), None),
        ids[2].as_str(): entry("Aide", None, "help-uid", Some("fr")),
    });
    layout["kept"] = json!(["t-7"]);
    layout["next_tab"] = json!(9);
    std::fs::create_dir_all(dir.join(TABS_DIR).join("t-7")).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(&file, serde_json::to_vec(&layout).unwrap_or_default()).unwrap_or_else(|e| panic!("{e}"));
    Previous { data, dir, tabs }
}

#[test]
fn restore_of_a_previous_layout_reads_every_document_before_waking() {
    let previous = previous_release();
    let ws = resumed(&previous);
    for (id, _) in &previous.tabs {
        let slot = Workspace::lock(&ws).session(id).unwrap_or_else(|e| panic!("{e}"));
        assert_that!(relock(&slot).is_dormant()).is_true();
    }
    assert_that!(documents(&ws)).is_equal_to(previous.tabs.clone());
}

#[test]
fn restore_of_a_previous_layout_keeps_every_document_after_waking() {
    let previous = previous_release();
    let file = previous.dir.join("workspace.json");
    let mut layout: Value = serde_json::from_slice(&std::fs::read(&file).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    let first = previous.tabs[0].0.as_str();
    layout["info"][first]["deck_uid"] = json!("stale-uid");
    std::fs::write(&file, serde_json::to_vec(&layout).unwrap_or_default()).unwrap_or_else(|e| panic!("{e}"));
    let ws = resumed(&previous);
    for (id, _) in &previous.tabs {
        Workspace::with_session(&ws, id, |s| s.tab_info()).0.unwrap_or_else(|e| panic!("{e}"));
        let slot = Workspace::lock(&ws).session(id).unwrap_or_else(|e| panic!("{e}"));
        assert_that!(relock(&slot).is_dormant()).is_false();
    }
    assert_that!(documents(&ws)).is_equal_to(previous.tabs.clone());
}

#[test]
fn restore_of_a_previous_layout_keeps_next_tab_and_kept() {
    let previous = previous_release();
    let ws = resumed(&previous);
    let w = Workspace::lock(&ws);
    assert_that!(w.layout.next_tab).is_equal_to(9);
    assert_that!(w.layout.kept.clone()).is_equal_to(vec!["t-7".to_owned()]);
}

#[test]
fn restore_then_save_writes_documents_and_legacy_keys() {
    let previous = previous_release();
    let ws = resumed(&previous);
    drop(ws);
    let bytes = std::fs::read(previous.dir.join("workspace.json")).unwrap_or_else(|e| panic!("{e}"));
    let saved: Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{e}"));
    let written: Vec<(String, Value, Value, Value, Value)> = previous
        .tabs
        .iter()
        .map(|(id, _)| {
            let e = &saved["info"][id.as_str()];
            (id.clone(), e["document"].clone(), e["path"].clone(), e["deck_uid"].clone(), e["help"].clone())
        })
        .collect();
    let expected: Vec<(String, Value, Value, Value, Value)> = previous
        .tabs
        .iter()
        .map(|(id, d)| {
            let (path, uid, help) = (json!(d.path()), json!(d.deck_uid()), json!(d.help_locale()));
            (id.clone(), serde_json::to_value(d).unwrap_or_default(), path, uid, help)
        })
        .collect();
    assert_that!(written).is_equal_to(expected);
}
