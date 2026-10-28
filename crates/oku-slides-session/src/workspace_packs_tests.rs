use std::sync::Arc;
use std::time::Duration;

use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::workspace_packs_fixtures::{
    Packs, assert_outcome, copy_dir, duplicate_tab, forge_pack_entry, outcome, packs, packs_resumed, within,
};
use super::{Opener, PackOpened, opening_entry};
use crate::agent::PackGate;
use crate::library::DraftWrite;
use crate::pack_lease::{PackLease, pack_leased};
use crate::test_support::{adopt_deck_only_pack, fresh_dir, rename_title, workspace_on};
use crate::test_support_packs::{CARD_V2, as_workshop, assert_unleased, draft_of, editing, pack_tab_of};
use crate::workspace::{Opening, TABS_DIR, TabPlace};
use crate::{BASE_PACK, Request, SessionErrorKind, TabDocument, Workspace, WorkspaceOutcome};

/// `tab`'s deck in a widget session on `acme`: whether it is one, and its draft's library write.
fn widget_session(f: &Packs, tab: &str) -> (bool, DraftWrite) {
    let (acme, ns) = (f.acme, f.acme_ns.clone());
    f.with(tab, |s| {
        if s.widget_session.is_none() {
            s.set_widget(Some(&ns), "card", CARD_V2.into()).unwrap_or_else(|e| panic!("card: {}", e.one_line()));
        }
        let v = s.user_deck().widgets[&ns].version.clone();
        (s.widget_session.is_some(), draft_of(s, acme, &v))
    })
}

/// `apply` on a JSON request, its outcome.
fn send(f: &Packs, v: serde_json::Value) -> WorkspaceOutcome {
    Workspace::apply(&f.ws, serde_json::from_value(v).unwrap()).unwrap_or_else(|e| panic!("{}", e.one_line()))
}

/// The count of tabs and the next tab number.
fn counters(f: &Packs) -> (usize, u64) {
    (f.tab_count(), Workspace::lock(&f.ws).layout.next_tab)
}

#[test]
fn open_pack_from_a_deck_tab_opens_a_pack_tab_right_after_it() {
    let f = packs();
    let other = f.new_tab();
    f.activate("t-1");
    let p = f.pack_tab();
    assert_that!(f.window(0)).is_equal_to(vec!["t-1".to_owned(), p.clone(), other]);
    assert_that!(f.document(&p)).is_equal_to(Some(TabDocument::Pack { pack: f.acme }));
}

#[test]
fn open_pack_from_a_deck_tab_activates_the_pack_tab() {
    let f = packs();
    f.new_tab();
    f.activate("t-1");
    let p = f.pack_tab();
    assert_that!(f.active(0)).is_equal_to(p);
}

#[test]
fn open_pack_from_the_last_tab_appends_after_it() {
    let f = packs();
    let other = f.new_tab();
    f.send(json!({"cmd": "reorder", "tab": "t-1", "index": 1}));
    let p = f.pack_tab();
    assert_that!(f.window(0)).is_equal_to(vec![other, "t-1".to_owned(), p]);
}

#[test]
fn open_pack_twice_focuses_the_existing_pack_tab() {
    let f = packs();
    let p = f.pack_tab();
    f.activate("t-1");
    assert_outcome(f.open("t-1", f.acme, None), &format!("Focused({p:?})"));
    assert_that!(f.active(0)).is_equal_to(p);
}

#[test]
fn open_pack_twice_keeps_the_tab_count_and_next_tab() {
    let f = packs();
    f.pack_tab();
    let before = counters(&f);
    f.open("t-1", f.acme, None).unwrap();
    assert_that!(counters(&f)).is_equal_to(before);
}

/// Window `[B, A, P, X]`: P opened from A, then focused from B; the neighbour rule would give X.
#[test]
fn open_pack_from_a_second_deck_retargets_the_opener() {
    let f = packs();
    let b = f.new_tab();
    f.send(json!({"cmd": "reorder", "tab": b, "index": 0}));
    let p = f.pack_tab();
    f.new_tab();
    assert_outcome(f.open(&b, f.acme, None), &format!("Focused({p:?})"));
    f.close(&p);
    assert_that!(f.active(0)).is_equal_to(b);
}

#[test]
fn agent_open_pack_never_retargets_the_opener() {
    let f = packs();
    let b = f.new_tab();
    f.send(json!({"cmd": "reorder", "tab": b, "index": 0}));
    let p = f.pack_tab();
    assert_outcome(f.open_by_agent(&b, f.acme, &mut |_| true), &format!("Focused({p:?})"));
    assert_that!(f.opener_of(&p)).is_equal_to(Some("t-1".to_owned()));
}

/// The only candidate is a dormant entry cached as a pack tab of P whose state is a deck.
#[test]
fn open_pack_asks_focus_then_new_tab_when_no_candidate_holds_it() {
    let f = packs_resumed();
    let (decoy, acme) = (f.new_tab(), f.acme);
    let f = f.restart(|_, layout| forge_pack_entry(layout, &decoy, acme));
    let mut asked = Vec::new();
    f.open_by_agent("t-1", acme, &mut |g| {
        asked.push(g);
        true
    })
    .unwrap();
    assert_that!(asked).is_equal_to(vec![PackGate::Focus, PackGate::NewTab]);
}

#[test]
fn open_pack_asks_the_gate_once_with_no_other_lock() {
    let f = packs();
    let (ws, acme) = (Arc::clone(&f.ws), f.acme);
    let calls = within(move || {
        let mut calls = Vec::new();
        let mut gate = |g: PackGate| {
            let free = ws.try_lock().is_ok();
            let other = Arc::clone(&ws);
            let (done, answered) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = done.send(Workspace::request(&other, "t-1", Request::Snapshot).0.is_ok());
            });
            calls.push((g, free, answered.recv_timeout(Duration::from_secs(10)).ok()));
            true
        };
        Workspace::open_pack_with(&ws, "t-1", acme, None, Opener::Agent(&mut gate)).unwrap();
        calls
    });
    assert_that!(calls).is_equal_to(vec![(PackGate::NewTab, true, Some(true))]);
}

#[test]
fn open_pack_asks_focus_before_focusing_another_tab() {
    let f = packs();
    let p = f.pack_tab();
    f.activate("t-1");
    let mut asked = Vec::new();
    let r = f.open_by_agent("t-1", f.acme, &mut |g| {
        asked.push(g);
        true
    });
    assert_that!((outcome(r), asked)).is_equal_to((format!("Focused({p:?})"), vec![PackGate::Focus]));
}

#[rstest]
#[case::deck_only_pack(true)]
#[case::own_pack(false)]
fn open_pack_asks_nothing_in_place(#[case] deck_only: bool) {
    let f = packs();
    let (tab, pack) = if deck_only {
        let pack = f.adopt_deck_only().id;
        (f.deck_pack_tab(pack), pack)
    } else {
        (f.pack_tab(), f.acme)
    };
    let r = f.open_by_agent(&tab, pack, &mut |g| panic!("asked {g:?}"));
    assert_outcome(r, "Here");
}

#[test]
fn open_pack_refused_by_focus_activates_nothing() {
    let f = packs_resumed();
    let p = f.pack_tab();
    f.activate("t-1");
    let f = f.restart(|_, _| {});
    assert_that!(f.is_dormant(&p)).is_true();
    let r = f.open_by_agent("t-1", f.acme, &mut |g| g != PackGate::Focus);
    assert_that!((outcome(r), f.active(0), f.opener_of(&p), f.is_dormant(&p))).is_equal_to((
        "Refused".to_owned(),
        "t-1".to_owned(),
        None,
        true,
    ));
}

#[test]
fn open_pack_refused_by_the_gate_changes_nothing() {
    let f = packs();
    let before = widget_session(&f, "t-1");
    assert_that!(before.0).is_true();
    let count = f.tab_count();
    let r = f.open_by_agent("t-1", f.acme, &mut |_| false);
    assert_unleased(&f.scope, f.acme);
    assert_that!((outcome(r), f.tab_count(), widget_session(&f, "t-1"))).is_equal_to((
        "Refused".to_owned(),
        count,
        before,
    ));
}

#[test]
fn open_pack_with_a_widget_on_an_open_pack_tab_opens_its_sub_tab_before_activating() {
    let f = packs();
    let p = f.pack_tab();
    f.activate("t-1");
    let r = f.open("t-1", f.acme, Some("card"));
    let widget = f.with(&p, |s| s.ui.widget.clone());
    assert_that!((outcome(r), widget, f.active(0))).is_equal_to((
        format!("Focused({p:?})"),
        Some("card".to_owned()),
        p,
    ));
}

#[test]
fn open_pack_with_an_unknown_widget_on_an_open_pack_tab_activates_nothing() {
    let f = packs();
    f.pack_tab();
    f.activate("t-1");
    let r = f.open("t-1", f.acme, Some("nope"));
    assert_that!((outcome(r), f.active(0))).is_equal_to(("UnknownWidget".to_owned(), "t-1".to_owned()));
}

#[test]
fn open_pack_of_the_pack_the_tab_edits_is_applied() {
    let f = packs();
    let p = f.pack_tab();
    let out = send(&f, json!({"cmd": "open_pack", "tab": p, "pack": f.acme}));
    assert_that!(matches!(out, WorkspaceOutcome::Applied(_))).is_true();
}

#[test]
fn open_pack_from_a_pack_tab_of_another_library_pack_opens_a_second_pack_tab() {
    let f = packs();
    let p = f.pack_tab();
    let PackOpened::Opened(q) = f.open(&p, BASE_PACK, None).unwrap() else { panic!("a second pack tab") };
    assert_that!((f.window(0), f.document(&q)))
        .is_equal_to((vec!["t-1".to_owned(), p, q.clone()], Some(TabDocument::Pack { pack: BASE_PACK })));
}

#[test]
fn open_pack_of_an_unknown_pack_is_refused_and_leaves_no_tab() {
    let f = packs();
    let before = counters(&f);
    let r = f.open("t-1", uuid::Uuid::new_v4(), None);
    assert_that!((outcome(r), counters(&f))).is_equal_to(("UnknownPack".to_owned(), before));
}

#[test]
fn open_pack_of_an_unknown_pack_leaves_no_opening_entry() {
    let f = packs();
    let unknown = uuid::Uuid::new_v4();
    f.open("t-1", unknown, None).unwrap_err();
    assert_that!(opening_entry(unknown)).is_false();
}

#[test]
fn open_pack_leaves_no_opening_entry_after_success() {
    let f = packs();
    f.pack_tab();
    assert_that!(opening_entry(f.acme)).is_false();
}

#[test]
fn open_pack_from_an_unknown_tab_is_refused_and_leaves_no_tab() {
    let f = packs();
    let before = counters(&f);
    let r = f.open("t-99", f.acme, None);
    assert_that!((outcome(r), counters(&f))).is_equal_to(("UnknownTab".to_owned(), before));
}

#[test]
fn open_pack_never_replaces_a_pristine_tab() {
    let data = fresh_dir("oku-slides-ws-packs-");
    let ws = workspace_on(data.path());
    assert_that!(Workspace::lock(&ws).tabs["t-1"].info.pristine).is_true();
    Workspace::open_pack_with(&ws, "t-1", BASE_PACK, None, Opener::Ui).unwrap();
    assert_that!(Workspace::lock(&ws).layout.windows[0].tabs.clone())
        .is_equal_to(vec!["t-1".to_owned(), "t-2".to_owned()]);
}

#[test]
fn open_pack_refused_by_a_held_lease_keeps_the_widget_session() {
    let f = packs();
    widget_session(&f, "t-1");
    // Another editor of acme, outside this workspace's tabs, holds its lease.
    let _holder = pack_tab_of(f.data.path(), f.acme);
    let before = widget_session(&f, "t-1");
    assert_that!(before.0).is_true();
    let r = f.open("t-1", f.acme, None);
    assert_that!((outcome(r), widget_session(&f, "t-1"))).is_equal_to(("PackOpenInTab".to_owned(), before));
}

/// `Pending`-level: a pack tab of a deck-only pack fails to build, and leaves nothing leased.
#[test]
fn open_pack_whose_build_fails_leaves_no_lease() {
    let f = packs_resumed();
    let d = f.with("t-1", |s| adopt_deck_only_pack(s).id);
    let seed = f.with("t-1", |s| s.take_preview(f.acme)).unwrap();
    let lease = PackLease::acquire(&f.scope, d, "d", "d").unwrap();
    let place = TabPlace::After("t-1".into());
    let pending = Workspace::lock(&f.ws).pending(Some("w-1".into()), Opening::Pack { pack: d, widget: None }, place);
    let err = pending.build_pack(*seed, lease).unwrap_err();
    let folder = f.resume.as_ref().unwrap().join(TABS_DIR).join(&pending.id);
    assert_unleased(&f.scope, d);
    assert_that!((err.kind(), f.tab_count(), folder.exists())).is_equal_to((SessionErrorKind::DeckOnly, 1, false));
}

#[test]
fn pack_tab_survives_a_workspace_restore_unwoken() {
    let f = packs_resumed();
    let p = f.pack_tab();
    f.activate("t-1");
    let f = f.restart(|_, _| {});
    assert_that!(f.is_dormant(&p)).is_true();
    let count = f.tab_count();
    assert_outcome(f.open("t-1", f.acme, None), &format!("Focused({p:?})"));
    assert_that!(f.tab_count()).is_equal_to(count);
}

#[test]
fn pack_tab_survives_a_workspace_restore_as_a_pack_tab() {
    let f = packs_resumed();
    let p = f.pack_tab();
    let f = f.restart(|_, _| {});
    let document = f.with(&p, |s| s.tab_info().document);
    assert_that!(document).is_equal_to(TabDocument::Pack { pack: f.acme });
}

#[test]
fn restored_pack_tab_has_no_opener() {
    let f = packs_resumed();
    let p = f.pack_tab();
    assert_that!(f.opener_of(&p)).is_equal_to(Some("t-1".to_owned()));
    let f = f.restart(|_, _| {});
    assert_that!(f.opener_of(&p)).is_none();
}

/// No dormant lease (K3): a restored pack tab that sleeps holds nothing.
#[test]
fn restored_pack_tab_holds_no_lease_while_dormant() {
    let f = packs_resumed();
    let p = f.pack_tab();
    let f = f.restart(|_, _| {});
    let ns = f.acme_ns.clone();
    let write = f.with("t-1", |s| s.set_widget(Some(&ns), "card", CARD_V2.into()).map(|_| ()).map_err(|e| e.kind()));
    assert_that!((f.is_dormant(&p), write)).is_equal_to((true, Ok(())));
}

#[test]
fn restored_pack_tab_takes_the_lease_when_it_wakes() {
    let f = packs_resumed();
    let p = f.pack_tab();
    let f = f.restart(|_, _| {});
    Workspace::request(&f.ws, &p, Request::Snapshot).0.unwrap();
    let ns = f.acme_ns.clone();
    let err = f.with("t-1", |s| s.set_widget(Some(&ns), "card", CARD_V2.into()).unwrap_err());
    assert_that!((err.kind(), err.lease_holder())).is_equal_to((SessionErrorKind::PackOpenInTab, Some("acme")));
}

/// Two forged entries of P: the first woken holds the lease, the second waits read-only.
#[test]
fn second_restored_pack_tab_of_one_pack_reopens_without_a_lease() {
    let f = packs_resumed();
    let p = f.pack_tab();
    let f = f.restart(|resume, layout| duplicate_tab(resume, layout, &p, "t-9"));
    Workspace::request(&f.ws, &p, Request::Snapshot).0.unwrap();
    let notice = f.with("t-9", |s| s.snapshot().notice.map(str::to_owned));
    let expected = f.with("t-9", |s| {
        s.localizer.tr_with("notice-pack-edited-elsewhere", &[("pack", "acme".into()), ("tab", "acme".into())])
    });
    let edit = || f.with("t-9", |s| s.apply("y", vec![rename_title("acme 3")], None).map_err(|e| e.kind()));
    let refused = edit();
    f.close(&p);
    assert_that!((notice, refused, edit())).is_equal_to((Some(expected), Err(SessionErrorKind::PackOpenInTab), Ok(())));
}

/// An older version's workshop `t-1` (its state saved before the pack tab) and pack tab P, restored.
fn workshop_and_pack_tab() -> (Packs, String) {
    let f = packs_resumed();
    f.with("t-1", |s| {
        s.persist(true);
        s.flush_resume();
    });
    let saved = fresh_dir("oku-slides-ws-workshop-");
    let resume = f.resume.clone().unwrap();
    copy_dir(&resume.join(TABS_DIR).join("t-1"), saved.path());
    let tab_dir = fresh_dir("oku-slides-ws-workshop-pack-");
    let mut t = pack_tab_of(f.data.path(), f.acme);
    assert_that!(t.resume_fresh(tab_dir.path().to_path_buf())).is_true();
    t.persist(true);
    t.flush_resume();
    drop(t);
    as_workshop(saved.path(), tab_dir.path());
    let p = f.pack_tab();
    let f = f.restart(|resume, _| {
        let t1 = resume.join(TABS_DIR).join("t-1");
        std::fs::remove_dir_all(&t1).unwrap();
        copy_dir(saved.path(), &t1);
    });
    (f, p)
}

#[test]
fn restored_pack_tab_woken_first_holds_the_lease() {
    let (f, p) = workshop_and_pack_tab();
    Workspace::request(&f.ws, &p, Request::Snapshot).0.unwrap();
    // The workshop reopens on its deck: it takes no lease.
    let editing = f.with("t-1", |s| s.editing_view().is_some());
    assert_that!((pack_leased(&f.scope, f.acme), editing)).is_equal_to((Some("acme".to_owned()), false));
}

#[test]
fn pack_tab_after_save_as_new_focuses_on_the_copy() {
    let f = packs();
    let p = f.pack_tab();
    let copy = f.with(&p, |s| {
        s.apply("x", vec![rename_title("acme copie")], None).unwrap();
        s.save_pack(true).unwrap();
        editing(s)
    });
    let copy = copy.unwrap_or_else(|| panic!("the copy is edited"));
    assert_that!(copy).is_not_equal_to(f.acme);
    f.activate("t-1");
    assert_outcome(f.open("t-1", copy, None), &format!("Focused({p:?})"));
}
