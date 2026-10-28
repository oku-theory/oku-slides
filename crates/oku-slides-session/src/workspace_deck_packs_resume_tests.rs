use std::path::Path;

use oku_slides_core::{Namespace, PackKind};
use rstest::rstest;
use serde::Deserialize;
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::super::packs::workspace_packs_fixtures::{Packs, deck_packs_resumed, duplicate_tab, within_secs};
use crate::deck_pack_tab::SendBase;
use crate::test_support::session_on;
use crate::test_support_packs::{CARD_SRC, ok};
use crate::{Request, Session, SessionErrorKind, TabDocument, Workspace};

/// Runs a restart scenario on another thread, at most a minute.
fn within<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    within_secs(60, f)
}

/// The `state.json` of tab `tab` under `resume`.
fn state_of(resume: &Path, tab: &str) -> Value {
    let bytes = std::fs::read(resume.join("tabs").join(tab).join("state.json")).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// The first object of `v` holding key `key`, at any depth.
fn holder<'a>(v: &'a mut Value, key: &str) -> Option<&'a mut serde_json::Map<String, Value>> {
    if v.as_object().is_some_and(|map| map.contains_key(key)) {
        return v.as_object_mut();
    }
    match v {
        Value::Object(map) => map.values_mut().find_map(|v| holder(v, key)),
        Value::Array(items) => items.iter_mut().find_map(|v| holder(v, key)),
        _ => None,
    }
}

/// The value of key `key` in `v`, at any depth; `Null` when absent.
fn value_at(mut v: Value, key: &str) -> Value {
    holder(&mut v, key).and_then(|map| map.get(key).cloned()).unwrap_or(Value::Null)
}

/// Rewrites tab `tab`'s `state.json` under `resume` through `edit`.
fn edit_state(resume: &Path, tab: &str, edit: impl FnOnce(&mut Value)) {
    crate::test_support::edit_state(&resume.join("tabs").join(tab), edit);
}

/// The localized `notice-tab-resume-failed`.
fn resume_failed(f: &Packs) -> String {
    Workspace::lock(&f.ws).platform.localizer.tr("notice-tab-resume-failed")
}

/// The notice `tab` shows once woken.
fn notice_of(f: &Packs, tab: &str) -> Option<String> {
    f.with(tab, |s| s.doc.notice.clone())
}

/// Whether the workspace still has tab `tab`.
fn has(f: &Packs, tab: &str) -> bool {
    Workspace::lock(&f.ws).tabs.contains_key(tab)
}

/// Whether a folder of tab `tab` (or its set-aside copy) is still under `resume`.
fn folder_kept(f: &Packs, tab: &str) -> bool {
    let tabs = f.resume.as_ref().unwrap_or_else(|| panic!("a resume folder")).join("tabs");
    std::fs::read_dir(tabs).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().starts_with(tab))
}

/// Parks a save of deck-pack tab `p`: its owner `t-1` is in an agent turn.
fn park_a_save(f: &Packs, p: &str, as_new: bool) {
    f.with("t-1", |s| s.begin_turn(1, "mise en page"));
    f.edit_in(p, "Importé 2");
    f.save_pack(p, as_new).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
    assert_that!(f.unsent(p)).is_true();
}

#[test]
fn resume_restores_a_linked_deck_pack_tab() {
    within(|| {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        f.save_in(&p, "Importé 2");
        let f = f.restart(|_, _| {});
        assert_that!((f.is_dormant(&p), f.document(&p)))
            .is_equal_to((true, Some(TabDocument::DeckPack { deck_tab: "t-1".to_owned(), pack: pin.id })));
        assert_that!(f.owner_write("t-1").map_err(|e| e.kind())).is_equal_to(Err(SessionErrorKind::PackOpenInTab));
    });
}

#[test]
fn resume_unforwarded_deck_pack_tab_forwards_on_flush() {
    within(|| {
        let (f, pin) = deck_packs_resumed();
        f.new_tab();
        let p = f.deck_pack_tab(pin.id);
        park_a_save(&f, &p, false);
        let f = f.restart(|resume, _| assert_that!(value_at(state_of(resume, &p), "unforwarded").is_null()).is_false());
        let closed = f.close_keeping("t-1").err().map(|e| e.kind());
        assert_that!(closed).is_equal_to(Some(SessionErrorKind::Unsaved));
        assert_that!(f.pinned("t-1", pin.id)).is_not_equal_to(Some(pin.version.clone()));
        assert_that!(f.unsent(&p)).is_false();
    });
}

#[test]
fn resume_unforwarded_copy_is_restored_and_applied() {
    within(|| {
        let (f, pin) = deck_packs_resumed();
        f.new_tab();
        let p = f.deck_pack_tab(pin.id);
        park_a_save(&f, &p, true);
        let f = f.restart(|_, _| {});
        assert_that!(f.document(&p))
            .is_equal_to(Some(TabDocument::DeckPack { deck_tab: "t-1".to_owned(), pack: pin.id }));
        assert_that!(f.close_keeping("t-1").err().map(|e| e.kind())).is_equal_to(Some(SessionErrorKind::Unsaved));
        let Some(TabDocument::DeckPack { pack: copy, .. }) = f.document(&p) else { panic!("a deck-pack tab") };
        assert_that!((copy != pin.id, f.pinned("t-1", copy).is_some())).is_equal_to((true, true));
        f.close("t-1");
        assert_that!((has(&f, "t-1"), has(&f, &p))).is_equal_to((false, false));
    });
}

#[test]
fn unacknowledged_copy_then_save_is_applied() {
    within(|| {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        park_a_save(&f, &p, true);
        f.save_in(&p, "Importé 3");
        f.with("t-1", Session::end_turn);
        let Some(TabDocument::DeckPack { pack: copy, .. }) = f.document(&p) else { panic!("a deck-pack tab") };
        assert_that!((copy != pin.id, f.pinned("t-1", copy).is_some(), f.unsent(&p))).is_equal_to((true, true, false));
    });
}

#[test]
fn resume_unforwarded_whose_pin_is_not_its_base_notices() {
    within(|| {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        park_a_save(&f, &p, false);
        let opened = serde_json::to_value(&pin.version).unwrap();
        let f = f.restart(|resume, _| {
            edit_state(resume, &p, |state| {
                // The saved version stays pinned, but the state names the opened one as its base.
                let edit = holder(state, "session_version").unwrap_or_else(|| panic!("a pack state"));
                edit.insert("base".to_owned(), opened);
            });
        });
        let lost = Workspace::lock(&f.ws).platform.localizer.tr("notice-deck-pack-unforwarded-lost");
        assert_that!(notice_of(&f, &p).is_some_and(|n| n.contains(&lost))).is_true();
        assert_that!(f.pinned("t-1", pin.id)).is_equal_to(Some(pin.version.clone()));
    });
}

#[test]
fn resume_unforwarded_undo_to_the_parent_is_forwarded() {
    within(|| {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        f.with(&p, |s| s.set_widget(None, "card", CARD_SRC.into())).unwrap_or_else(|e| panic!("{}", e.one_line()));
        f.save_pack(&p, false).unwrap_or_else(|e| panic!("save: {}", e.one_line()));
        assert_that!(f.pinned("t-1", pin.id)).is_not_equal_to(Some(pin.version.clone()));
        f.with("t-1", |s| s.begin_turn(1, "mise en page"));
        // Back to the version the tab opened on: its session version is gone, its save parked.
        assert_that!(Workspace::request(&f.ws, &p, Request::Undo).0).is_ok();
        f.with(&p, Session::mark_due);
        assert_that!(f.unsent(&p)).is_true();
        // A quit whose flush did not run (`closed` already set): the tab stays unforwarded.
        Workspace::lock(&f.ws).closed.store(true, std::sync::atomic::Ordering::SeqCst);
        let f = f.restart(|_, _| {});
        let lost = Workspace::lock(&f.ws).platform.localizer.tr("notice-deck-pack-unforwarded-lost");
        assert_that!(notice_of(&f, &p).is_some_and(|n| n.contains(&lost))).is_false();
        assert_that!((f.pinned("t-1", pin.id), f.unsent(&p))).is_equal_to((Some(pin.version.clone()), false));
    });
}

#[test]
fn resume_owner_closes_a_tab_whose_link_it_refused() {
    within(|| {
        let (f, pin) = deck_packs_resumed();
        f.new_tab();
        let p = f.deck_pack_tab(pin.id);
        let path = f.with("t-1", |s| s.doc.path.clone()).unwrap_or_else(|| panic!("a file"));
        let data = f.data.path().to_owned();
        let f = f.restart(|_, _| {
            let mut other = session_on(&data);
            ok(other.open(&path), "open elsewhere");
            ok(other.remove_deck_pack(PackKind::Widgets, &Namespace::deck()), "remove elsewhere");
            ok(other.save(None), "save elsewhere");
        });
        let notice = notice_of(&f, "t-1");
        assert_that!(notice.is_some_and(|n| n.contains(&resume_failed(&f)))).is_true();
        assert_that!((has(&f, &p), folder_kept(&f, &p), f.actives_valid())).is_equal_to((false, true, true));
    });
}

/// One bad deck-pack entry the resume sets aside.
#[derive(Debug, Clone, Copy)]
enum Bad {
    OwnerAbsent,
    OwnerAPackTab,
    DeckTabNotATabId,
    DeckTabItsOwnId,
    Duplicate,
    CachedPackDiffers,
    NotDeckOnly,
}

#[rstest]
#[case::owner_absent(Bad::OwnerAbsent)]
#[case::owner_a_pack_tab(Bad::OwnerAPackTab)]
#[case::deck_tab_not_a_tab_id(Bad::DeckTabNotATabId)]
#[case::deck_tab_its_own_id(Bad::DeckTabItsOwnId)]
#[case::duplicate_deck_tab_and_pack(Bad::Duplicate)]
#[case::cached_pack_not_the_states(Bad::CachedPackDiffers)]
#[case::not_a_deck_only_state(Bad::NotDeckOnly)]
fn resume_sets_aside_a_bad_deck_pack_entry(#[case] bad: Bad) {
    within(move || {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        let pack_tab = matches!(bad, Bad::OwnerAPackTab).then(|| f.pack_tab());
        let (aside, shown_by) = match bad {
            Bad::Duplicate => ("t-50".to_owned(), "t-1".to_owned()),
            Bad::OwnerAPackTab => (p.clone(), pack_tab.clone().unwrap_or_default()),
            _ => (p.clone(), "t-1".to_owned()),
        };
        let f = f.restart(|resume, layout| match bad {
            Bad::Duplicate => duplicate_tab(resume, layout, &p, "t-50"),
            Bad::NotDeckOnly => edit_state(resume, &p, |state| {
                let edit = holder(state, "deck_only").unwrap_or_else(|| panic!("a pack state"));
                edit.insert("deck_only".to_owned(), json!(false));
            }),
            _ => {
                let document = &mut layout["info"][&p]["document"];
                match bad {
                    Bad::OwnerAbsent => document["deck_tab"] = json!("t-77"),
                    Bad::OwnerAPackTab => document["deck_tab"] = json!(pack_tab),
                    Bad::DeckTabNotATabId => document["deck_tab"] = json!("nope"),
                    Bad::DeckTabItsOwnId => document["deck_tab"] = json!(p),
                    _ => document["pack"] = json!(Uuid::new_v4()),
                }
            }
        });
        // A tab restored dormant is set aside when it wakes.
        drop(Workspace::with_session(&f.ws, &aside, |_| ()));
        assert_that!((has(&f, &aside), folder_kept(&f, &aside))).is_equal_to((false, true));
        let notice = notice_of(&f, &shown_by);
        assert_that!(notice.is_some_and(|n| n.contains(&resume_failed(&f)))).is_true();
        assert_that!(f.actives_valid()).is_true();
    });
}

/// A document as the previous release read it: no `deck_pack` kind.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PreviousDocument {
    Deck {},
    Help {},
    Pack {},
    Theme {},
}

/// A layout entry as the previous release read it.
#[derive(Debug, Deserialize)]
struct PreviousEntry {
    #[expect(dead_code, reason = "parsed only: the previous release's reader")]
    document: PreviousDocument,
}

#[test]
fn legacy_reader_drops_only_the_deck_pack_entry() {
    within(|| {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        let mut info = Value::Null;
        let _f = f.restart(|_, layout| info = layout["info"].clone());
        let entries = info.as_object().unwrap_or_else(|| panic!("an info map"));
        let dropped: Vec<&String> = entries
            .iter()
            .filter(|(_, entry)| serde_json::from_value::<PreviousEntry>((*entry).clone()).is_err())
            .map(|(id, _)| id)
            .collect();
        assert_that!((entries.len(), dropped)).is_equal_to((2, vec![&p]));
    });
}

#[test]
fn pack_state_unforwarded_round_trips() {
    within(|| {
        let (f, pin) = deck_packs_resumed();
        let p = f.deck_pack_tab(pin.id);
        park_a_save(&f, &p, false);
        let mut saved = Value::Null;
        let f = f.restart(|resume, _| saved = value_at(state_of(resume, &p), "unforwarded"));
        let base = SendBase { pack: pin.id, parent: Some(pin.version.clone()) };
        assert_that!(serde_json::from_value::<SendBase>(saved).ok()).is_equal_to(Some(base));
        assert_that!(f.unsent(&p)).is_true();
    });
}
