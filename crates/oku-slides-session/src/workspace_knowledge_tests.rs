use std::path::Path;
use std::sync::Mutex;

use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::super::workspace_fixtures::{ids, view};
use super::super::{Slot, Workspace};
use crate::agent::TabOpen;
use crate::deck_usage::deck_usage_tests::record;
use crate::test_support::{apply_ws, fresh_dir, rename_title, resumed_on, workspace_on};
use crate::{DocumentKind, SessionErrorKind, TabDocument, TabInfo};

/// The knowledge tab of `owner`, opened; its id.
fn opened(ws: &Mutex<Workspace>, owner: &str) -> String {
    Workspace::lock(ws).open_knowledge(owner).unwrap_or_else(|e| panic!("open_knowledge: {}", e.one_line())).0
}

/// The cached info of `tab`.
fn info(ws: &Mutex<Workspace>, tab: &str) -> TabInfo {
    let v = view(ws);
    let t = v.windows.iter().flat_map(|w| &w.tabs).find(|t| t.id == tab);
    t.unwrap_or_else(|| panic!("{tab} not in the view")).info.clone()
}

/// Whether `tab`'s slot is still a view (no session).
fn is_view(ws: &Mutex<Workspace>, tab: &str) -> bool {
    let slot = Workspace::lock(ws).session(tab).unwrap_or_else(|e| panic!("{tab}: {}", e.one_line()));
    matches!(*super::super::relock(&slot), Slot::View)
}

fn tab_ids(ws: &Mutex<Workspace>) -> Vec<String> {
    Workspace::lock(ws).tab_ids()
}

#[test]
fn open_knowledge_new_placed_after_deck_tab() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let (tab, new) = Workspace::lock(&ws).open_knowledge("t-1").unwrap();
    let v = view(&ws);
    assert_that!(new).is_true();
    assert_that!(ids(&v.windows[0])).is_equal_to(vec!["t-1", tab.as_str(), "t-2"]);
    assert_that!(v.windows[0].active).is_equal_to(tab.clone());
    assert_that!(info(&ws, &tab).document).is_equal_to(TabDocument::Knowledge { deck_tab: "t-1".to_owned() });
    assert_that!(is_view(&ws, &tab)).is_true();
}

#[test]
fn open_knowledge_twice_focuses_same_tab() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    let first = opened(&ws, "t-1");
    apply_ws(&ws, json!({"cmd": "activate", "tab": "t-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "open_knowledge", "tab": "t-1"})).unwrap();
    let (again, new) = Workspace::lock(&ws).open_knowledge("t-1").unwrap();
    let v = view(&ws);
    assert_that!((again.as_str(), new)).is_equal_to((first.as_str(), false));
    assert_that!(v.windows[0].tabs).has_length(2);
    assert_that!(v.windows[0].active).is_equal_to(first);
}

#[test]
fn open_knowledge_from_help_tab_refused_wrong_document() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "open_help", "window": "w-1"})).unwrap();
    let help = view(&ws).windows[0].active.clone();
    let refused = Workspace::lock(&ws).open_knowledge(&help).map_err(|e| e.kind());
    assert_that!(refused).is_err_containing(SessionErrorKind::WrongDocument);
    assert_that!(tab_ids(&ws)).has_length(2);
}

#[test]
fn open_knowledge_from_knowledge_tab_refused_wrong_document() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    let tab = opened(&ws, "t-1");
    let refused = Workspace::lock(&ws).open_knowledge(&tab).map_err(|e| e.kind());
    assert_that!(refused).is_err_containing(SessionErrorKind::WrongDocument);
}

#[test]
fn open_knowledge_owner_closed_meanwhile_unknown_tab() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "close", "tab": "t-1", "discard": false})).unwrap();
    let refused = apply_ws(&ws, json!({"cmd": "open_knowledge", "tab": "t-1"})).map_err(|e| e.kind());
    assert_that!(refused).is_err_containing(SessionErrorKind::UnknownTab);
    assert_that!(tab_ids(&ws)).is_equal_to(vec!["t-2".to_owned()]);
}

#[test]
fn knowledge_tab_session_request_refused_wrong_document() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    let tab = opened(&ws, "t-1");
    let refused = Workspace::with_session(&ws, &tab, |s| s.tab_info()).0.map_err(|e| e.kind());
    assert_that!(refused).is_err_containing(SessionErrorKind::WrongDocument);
    assert_that!(is_view(&ws, &tab)).is_true();
}

#[test]
fn wrong_document_knowledge_localized_fr() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    let tab = opened(&ws, "t-1");
    let e = Workspace::with_session(&ws, &tab, |s| s.tab_info()).0.err().unwrap_or_else(|| panic!("served"));
    let (fr, en) = (Localizer::new(Some("fr")), Localizer::new(Some("en")));
    let args = [("kind", "knowledge".into())];
    let shown = e.localize(&fr);
    assert_that!(shown.as_str()).is_equal_to(fr.tr_with("session-wrong-document", &args).as_str());
    assert_that!(shown.as_str()).is_not_equal_to(en.tr_with("session-wrong-document", &args).as_str());
    assert_that!(shown.as_str())
        .is_not_equal_to(fr.tr_with("session-wrong-document", &[("kind", "deck".into())]).as_str());
}

#[test]
fn knowledge_tab_title_follows_owner_rename() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    let tab = opened(&ws, "t-1");
    assert_that!(info(&ws, &tab).title).is_equal_to(info(&ws, "t-1").title);
    let op = rename_title("Feuille de route");
    Workspace::with_session(&ws, "t-1", |s| s.apply("x", vec![op], None)).0.unwrap().unwrap();
    let owner = info(&ws, "t-1").title;
    assert_that!(owner.as_str()).contains("Feuille de route");
    assert_that!(info(&ws, &tab).title).is_equal_to(owner);
}

#[test]
fn close_owner_closes_knowledge() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    opened(&ws, "t-1");
    apply_ws(&ws, json!({"cmd": "close", "tab": "t-1", "discard": false})).unwrap();
    assert_that!(tab_ids(&ws)).is_equal_to(vec!["t-2".to_owned()]);
}

#[test]
fn close_owner_in_other_window_closes_knowledge() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let tab = opened(&ws, "t-1");
    apply_ws(&ws, json!({"cmd": "move_tab", "tab": tab})).unwrap();
    assert_that!(view(&ws).windows).has_length(2);
    apply_ws(&ws, json!({"cmd": "close", "tab": "t-1", "discard": false})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(ids(&v.windows[0])).is_equal_to(vec!["t-2"]);
}

/// The last deck and its knowledge tab: both close, as a plain last tab does (the host quits).
#[test]
fn close_owner_with_knowledge_as_last_tabs_closes_both() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    opened(&ws, "t-1");
    let out = apply_ws(&ws, json!({"cmd": "close", "tab": "t-1", "discard": false})).unwrap();
    assert_that!(matches!(out, crate::WorkspaceOutcome::LastWindow(_))).is_false();
    assert_that!(tab_ids(&ws)).is_empty();
}

#[test]
fn close_knowledge_alone_keeps_owner_and_never_prompts() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let tab = opened(&ws, "t-1");
    // Its cached info claims unsaved work: a knowledge tab still closes without a question.
    Workspace::lock(&ws).tabs.get_mut(&tab).unwrap().info.dirty = true;
    apply_ws(&ws, json!({"cmd": "close", "tab": tab, "discard": false})).unwrap();
    let v = view(&ws);
    assert_that!(ids(&v.windows[0])).is_equal_to(vec!["t-1", "t-2"]);
    assert_that!(v.windows[0].active.as_str()).is_equal_to("t-1");
}

#[test]
fn view_slot_settle_wake_set_aside_close_all_skipped() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let tab = opened(&ws, "t-1");
    Workspace::wake(&ws, std::slice::from_ref(&tab));
    assert_that!(is_view(&ws, &tab)).is_true();
    // A window closed with the deck and its knowledge tab: nothing asks, both go.
    apply_ws(&ws, json!({"cmd": "move_tab", "tab": "t-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "move_tab", "tab": tab, "window": "w-2"})).unwrap();
    apply_ws(&ws, json!({"cmd": "close_window", "window": "w-2", "discard": []})).unwrap();
    assert_that!(tab_ids(&ws)).is_equal_to(vec!["t-2".to_owned()]);
    // A set-aside and a settle leave a view slot as it is.
    let other = opened(&ws, "t-2");
    Workspace::settle(&ws);
    assert_that!(is_view(&ws, &other)).is_true();
    let slot = Workspace::lock(&ws).session(&other).unwrap();
    super::super::relock(&slot).stop_resume(&other);
    assert_that!(matches!(*super::super::relock(&slot), Slot::Closed(_))).is_true();
}

#[test]
fn knowledge_tab_never_dirty_and_skipped_by_compaction() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    let tab = opened(&ws, "t-1");
    assert_that!(info(&ws, &tab).dirty).is_false();
    assert_that!(Workspace::schedule_compactions(&ws, &tab)).is_empty();
    assert_that!(Workspace::usage_owner(&ws, &tab)).is_none();
    let refused = Workspace::record_usage(&ws, &tab, record(1)).map_err(|e| e.kind());
    assert_that!(refused).is_err_containing(SessionErrorKind::WrongDocument);
}

/// A resumed workspace whose first tab `t-1` has a written state; the resume dir.
fn resumable(data: &Path) -> (Mutex<Workspace>, std::path::PathBuf) {
    let dir = data.join("resume");
    let ws = resumed_on(data, &dir);
    Workspace::with_session(&ws, "t-1", |s| s.tab_info()).0.unwrap();
    (ws, dir)
}

/// Settles `ws`, lets `forge` change its `workspace.json`, and resumes it.
fn restart(ws: Mutex<Workspace>, data: &Path, dir: &Path, forge: impl FnOnce(&mut Value)) -> Mutex<Workspace> {
    Workspace::settle(&ws);
    drop(ws);
    let file = dir.join("workspace.json");
    let mut layout: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    forge(&mut layout);
    std::fs::write(&file, serde_json::to_vec(&layout).unwrap()).unwrap();
    resumed_on(data, dir)
}

#[test]
fn knowledge_tab_round_trips_workspace_json() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let (ws, dir) = resumable(data.path());
    let tab = opened(&ws, "t-1");
    let ws = restart(ws, data.path(), &dir, |_| {});
    let v = view(&ws);
    assert_that!(ids(&v.windows[0])).is_equal_to(vec!["t-1", tab.as_str()]);
    assert_that!(info(&ws, &tab).document).is_equal_to(TabDocument::Knowledge { deck_tab: "t-1".to_owned() });
    assert_that!(is_view(&ws, &tab)).is_true();
    assert_that!(dir.join("tabs").join(&tab).exists()).is_false();
}

/// A forged knowledge entry `id` on `owner`, cited at the end of the first window.
fn cite_knowledge(layout: &mut Value, id: &str, owner: &str) {
    layout["windows"][0]["tabs"].as_array_mut().unwrap().push(json!(id));
    layout["info"][id] = json!({
        "title": "forged", "dirty": true, "pristine": false,
        "document": {"kind": "knowledge", "deck_tab": owner},
    });
}

#[rstest]
#[case::restore_knowledge_with_deck_owner(false, &[("t-7", "t-1")], &["t-1", "t-7"])]
#[case::restore_knowledge_owner_not_deck_dropped(true, &[("t-7", "t-1")], &["t-1"])]
#[case::restore_knowledge_owner_failed_dropped_no_state_read(false, &[("t-7", "t-5")], &["t-1"])]
#[case::restore_second_knowledge_same_owner_dropped(false, &[("t-7", "t-1"), ("t-8", "t-1")], &["t-1", "t-7"])]
#[case::restore_knowledge_owner_is_knowledge_dropped(false, &[("t-7", "t-1"), ("t-8", "t-7")], &["t-1", "t-7"])]
fn restore_knowledge_owner_cases_kept_or_dropped(
    #[case] help_owner: bool,
    #[case] knowledge: &[(&str, &str)],
    #[case] expected: &[&str],
) {
    let data = fresh_dir("oku-slides-knowledge-restore-");
    let (ws, dir) = resumable(data.path());
    let ws = restart(ws, data.path(), &dir, |layout| {
        // The owner reads as a help tab: restored asleep, its state not read.
        if help_owner {
            layout["info"]["t-1"]["document"] = json!({"kind": "help", "locale": "fr"});
        }
        // `t-5` is cited with its info, but has no state: it is not restored.
        layout["windows"][0]["tabs"].as_array_mut().unwrap().push(json!("t-5"));
        layout["info"]["t-5"] = layout["info"]["t-1"].clone();
        for (id, owner) in knowledge {
            cite_knowledge(layout, id, owner);
        }
    });
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(expected.to_vec());
    if let Some(restored) = expected.get(1) {
        let restored_info = info(&ws, restored);
        assert_that!(restored_info.dirty).is_false();
        assert_that!(restored_info.title).is_equal_to(info(&ws, "t-1").title);
        assert_that!(is_view(&ws, restored)).is_true();
    }
}

#[test]
fn saved_knowledge_tab_has_no_legacy_keys() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let (ws, dir) = resumable(data.path());
    let tab = opened(&ws, "t-1");
    let saved: Value = serde_json::from_slice(&std::fs::read(dir.join("workspace.json")).unwrap()).unwrap();
    let entry = &saved["info"][tab.as_str()];
    assert_that!(entry["document"]).is_equal_to(json!({"kind": "knowledge", "deck_tab": "t-1"}));
    for key in ["path", "deck_uid", "help"] {
        assert_that!(entry.get(key).is_none_or(Value::is_null)).is_true();
    }
}

#[test]
fn agent_open_knowledge_existing_focuses_it() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    let tab = opened(&ws, "t-1");
    apply_ws(&ws, json!({"cmd": "activate", "tab": "t-1"})).unwrap();
    let out = Workspace::agent_open_knowledge(&ws, "t-1");
    assert_that!(out).is_ok_containing(TabOpen::Focused { tab: tab.clone(), shown: true });
    assert_that!(view(&ws).windows[0].active).is_equal_to(tab);
}

#[test]
fn agent_open_knowledge_unshown_requester_unshown() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let out = Workspace::agent_open_knowledge(&ws, "t-1").unwrap();
    let TabOpen::Opened { tab, shown } = out else { panic!("not opened: {out:?}") };
    let v = view(&ws);
    assert_that!(shown).is_false();
    assert_that!(ids(&v.windows[0])).is_equal_to(vec!["t-1", tab.as_str(), "t-2"]);
    assert_that!(v.windows[0].active.as_str()).is_equal_to("t-2");
}

#[test]
fn agent_open_knowledge_from_help_tab_refused() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "open_help", "window": "w-1"})).unwrap();
    let help = view(&ws).windows[0].active.clone();
    let out = Workspace::agent_open_knowledge(&ws, &help);
    let kind = out.err().and_then(|r| r.session_error().map(crate::SessionError::kind));
    assert_that!(kind).is_equal_to(Some(SessionErrorKind::WrongDocument));
    assert_that!(tab_ids(&ws)).has_length(2);
}

#[test]
fn open_tab_replacing_pristine_owner_closes_its_knowledge() {
    let data = super::super::workspace_fixtures::Data::new();
    let file = super::super::workspace_fixtures::saved_pdf(&data, "replacing");
    let ws = workspace_on(&data.0);
    let knowledge = opened(&ws, "t-1");
    apply_ws(&ws, json!({"cmd": "activate", "tab": "t-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    let ids = tab_ids(&ws);
    assert_that!(ids.contains(&"t-1".to_owned())).is_false();
    assert_that!(ids.contains(&knowledge)).is_false();
    let w = Workspace::lock(&ws);
    let orphans = w.tabs.values().filter(|t| t.info.document.kind() == DocumentKind::Knowledge).count();
    assert_that!(orphans).is_equal_to(0);
}

#[test]
fn tab_list_knowledge_tab_kind_knowledge() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    let tab = opened(&ws, "t-1");
    let listed = Workspace::agent_tabs(&ws, "t-1").unwrap();
    let knowledge = listed.iter().find(|t| t.tab == tab).unwrap_or_else(|| panic!("{tab} not listed"));
    assert_that!(knowledge.kind).is_equal_to(DocumentKind::Knowledge);
    assert_that!(knowledge.title).is_equal_to(info(&ws, "t-1").title);
    assert_that!(knowledge.dirty).is_false();
}

#[test]
fn document_kind_of_knowledge_tab_refused_wrong_document() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    let tab = opened(&ws, "t-1");
    let refused = Workspace::document_kind(&ws, &tab).map_err(|e| e.kind());
    assert_that!(refused).is_err_containing(SessionErrorKind::WrongDocument);
}

/// A restored knowledge entry whose id another tab already holds is dropped, never put over it.
#[test]
fn restore_knowledge_taken_id_dropped_keeps_tab() {
    let data = fresh_dir("oku-slides-knowledge-tab-");
    let ws = workspace_on(data.path());
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let mut restored: std::collections::HashSet<String> = ["t-1".to_owned(), "t-2".to_owned()].into();
    Workspace::lock(&ws).restore_knowledge(vec![("t-2".to_owned(), "t-1".to_owned())], &mut restored);
    assert_that!(is_view(&ws, "t-2")).is_false();
    assert_that!(Workspace::lock(&ws).knowledge_tab_of("t-1")).is_none();
}
