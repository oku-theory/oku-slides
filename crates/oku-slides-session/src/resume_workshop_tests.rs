use std::path::Path;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use super::{LegacyHost, PackResume, classify};
use crate::resume::Resumed;
use crate::test_support::{
    add_version, adopt_deck_only_pack, edit_state, fresh_dir, manifest_ns, rename_title, resumed_on, session_on,
};
use crate::test_support_packs::{
    acme_of, as_workshop, assert_unleased, deck_pack_tab_of, dormant_editor, live_workshop, ok, opener_deck, opener_on,
    pack_tab_from, pack_tab_of, park, scope_of,
};
use crate::{Mode, Session, TabDocument, Workspace};

/// Runs `f` on another thread; panics if it takes more than a minute (a deadlock).
fn within<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(f());
    });
    match finished.recv_timeout(Duration::from_secs(60)) {
        Ok(r) => r,
        Err(RecvTimeoutError::Timeout) => panic!("deadlock: no answer within 60 s"),
        Err(RecvTimeoutError::Disconnected) => panic!("the scenario panicked"),
    }
}

/// A new session on `data` resuming `dir`, and what it gave.
fn resumed(data: &Path, dir: &Path) -> (Session, Resumed) {
    let mut s = session_on(data);
    let result = s.resume_checked(dir.to_path_buf(), None);
    (s, result)
}

/// The resumed session's mode is the deck, its document the opener deck.
fn on_its_deck(s: &Session) -> (bool, String) {
    (matches!(s.mode, Mode::Deck), s.tab_info().title)
}

/// The notice a workshop reopened on its deck shows.
fn reopened_notice(s: &Session) -> String {
    s.localizer.tr("notice-workshop-reopened-on-deck")
}

#[rstest]
#[case::v3_without_host(3, json!(null), PackResume::Workshop)]
#[case::v3_deck(3, json!("deck"), PackResume::Workshop)]
#[case::v3_tab(3, json!("tab"), PackResume::Workshop)]
#[case::v4_tab(4, json!("tab"), PackResume::PackTab)]
#[case::v4_without_host(4, json!(null), PackResume::Unreadable)]
#[case::v4_deck(4, json!("deck"), PackResume::Unreadable)]
#[case::v5_deck(5, json!("deck"), PackResume::Workshop)]
#[case::v5_without_host(5, json!(null), PackResume::PackTab)]
#[case::v5_tab(5, json!("tab"), PackResume::PackTab)]
fn classify_pack_state_by_version_and_host_resumes_as(
    #[case] version: u32,
    #[case] host: Value,
    #[case] expected: PackResume,
) {
    let legacy: Option<LegacyHost> = serde_json::from_value(host).unwrap();
    assert_that!(classify(version, legacy)).is_equal_to(expected);
}

#[test]
fn resume_v3_workshop_reopens_on_deck() {
    within(|| {
        let data = fresh_dir("oku-slides-workshop-");
        let dormant = park(live_workshop(data.path()));
        let (back, result) = resumed(data.path(), dormant.dir.path());
        assert_that!(result).is_equal_to(Resumed::Restored);
        assert_that!(on_its_deck(&back)).is_equal_to((true, "opener".to_owned()));
        assert_unleased(&scope_of(&back), dormant.pack);
    });
}

#[test]
fn resume_v5_deck_host_workshop_reopens_on_deck() {
    within(|| {
        let data = fresh_dir("oku-slides-workshop-");
        let dormant = park(live_workshop(data.path()));
        edit_state(dormant.dir.path(), |state| {
            state["version"] = json!(5);
            state["mode"]["pack"]["host"] = json!("deck");
        });
        let (back, result) = resumed(data.path(), dormant.dir.path());
        assert_that!((result, on_its_deck(&back))).is_equal_to((Resumed::Restored, (true, "opener".to_owned())));
        assert_unleased(&scope_of(&back), dormant.pack);
    });
}

#[test]
fn resume_workshop_with_a_changed_pack_notices_the_dropped_edits() {
    within(|| {
        let data = fresh_dir("oku-slides-workshop-");
        // An edit of the working deck, never saved with SavePack: `pack/` is dirty.
        let mut live = live_workshop(data.path());
        ok(live.editor.apply("x", vec![rename_title("acme 2")], None), "edit");
        let dormant = park(live);
        let (back, _) = resumed(data.path(), dormant.dir.path());
        let expected = reopened_notice(&back);
        assert_that!(expected.as_str()).starts_with("Cet onglet éditait un pack dans sa présentation");
        assert_that!(back.snapshot().notice).is_equal_to(Some(expected.as_str()));
    });
}

#[test]
fn resume_workshop_with_an_unchanged_pack_says_nothing() {
    within(|| {
        let data = fresh_dir("oku-slides-workshop-");
        let dormant = park(live_workshop(data.path()));
        let (back, result) = resumed(data.path(), dormant.dir.path());
        assert_that!((result, back.snapshot().notice)).is_equal_to((Resumed::Restored, None));
    });
}

#[test]
fn resume_v5_pack_tab_without_host_is_a_pack_tab() {
    within(|| {
        let data = fresh_dir("oku-slides-workshop-");
        let dormant = dormant_editor(data.path());
        edit_state(dormant.dir.path(), |state| assert_that!(state["version"]).is_equal_to(json!(5)));
        let (back, result) = resumed(data.path(), dormant.dir.path());
        assert_that!((result, back.is_pack_tab())).is_equal_to((Resumed::Restored, true));
    });
}

#[test]
fn resume_v4_deck_host_state_is_set_aside_as_unreadable() {
    within(|| {
        let data = fresh_dir("oku-slides-workshop-");
        let resume = data.path().join("resume");
        let ws = resumed_on(data.path(), &resume);
        let (deck, acme, scope) = Workspace::with_session(&ws, "t-1", |s| {
            let deck = opener_deck(s, data.path());
            (deck, acme_of(s), scope_of(s))
        })
        .0
        .unwrap_or_else(|e| panic!("opener: {}", e.one_line()));
        Workspace::settle(&ws);
        drop(ws);
        let tab_dir = fresh_dir("oku-slides-workshop-tab-");
        let mut t = pack_tab_of(data.path(), acme);
        assert_that!(t.resume_fresh(tab_dir.path().to_path_buf())).is_true();
        t.persist(true);
        t.flush_resume();
        drop(t);
        assert_unleased(&scope, acme);
        let tab = resume.join("tabs").join("t-1");
        as_workshop(&tab, tab_dir.path());
        edit_state(&tab, |state| {
            state["mode"]["pack"]["host"] = json!("deck");
            state["version"] = json!(4);
        });
        let ws = resumed_on(data.path(), &resume);
        let (notice, deck_mode, failed) = Workspace::with_session(&ws, "t-1", |s| {
            (s.doc.notice.clone(), matches!(s.mode, Mode::Deck), s.localizer.tr("notice-tab-resume-failed"))
        })
        .0
        .unwrap_or_else(|e| panic!("t-1: {}", e.one_line()));
        assert_that!((notice, deck_mode)).is_equal_to((Some(failed), true));
        let kept = std::fs::read_dir(resume.join("tabs"))
            .unwrap()
            .any(|e| e.unwrap().file_name().to_string_lossy().starts_with("t-1.failed-"));
        assert_that!(kept).is_true();
        drop(deck);
    });
}

/// A workshop of `pack` opened by `s` on `data`, persisted untouched and gone: an older version's state.
///
/// Its pack edits come from a pack tab, or a deck-pack tab for a deck-only pack, folded in by `as_workshop`.
fn untouched_workshop(mut s: Session, data: &Path, pack: uuid::Uuid) -> tempfile::TempDir {
    let (dir, tab_dir) = (fresh_dir("oku-slides-workshop-tab-"), fresh_dir("oku-slides-workshop-pack-"));
    let deck_only = !ok(s.pack_base(pack), "pack").manifest.scope.is_library();
    let mut t = if deck_only { deck_pack_tab_of(&mut s, data, pack) } else { pack_tab_from(&mut s, data, pack) };
    for (session, folder) in [(&mut s, &dir), (&mut t, &tab_dir)] {
        assert_that!(session.resume_fresh(folder.path().to_path_buf())).is_true();
        session.persist(true);
        session.flush_resume();
    }
    drop((s, t));
    as_workshop(dir.path(), tab_dir.path());
    dir
}

/// Clears `dir`'s `pack/` dirty flag, as a workshop that saved its pack left it.
fn pack_doc_clean(dir: &Path) {
    let path = dir.join("pack").join("document.json");
    let mut doc: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    doc["dirty"] = json!(false);
    std::fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
}

#[test]
fn resume_workshop_of_a_library_pack_the_deck_pins_older_says_nothing() {
    within(|| {
        let data = fresh_dir("oku-slides-workshop-");
        let (s, _deck, acme) = opener_on(data.path());
        let mut other = pack_tab_of(data.path(), acme);
        ok(other.apply("x", vec![rename_title("acme 2")], None), "edit");
        let newer = ok(other.save_pack(false), "save");
        drop(other);
        assert_that!(s.library.current(acme)).is_equal_to(Some(newer));
        let dir = untouched_workshop(s, data.path(), acme);
        let (back, result) = resumed(data.path(), dir.path());
        assert_that!((result, back.snapshot().notice)).is_equal_to((Resumed::Restored, None));
    });
}

/// `host_deck`: the v3 shape 6.0.0 wrote, its workshop's `host` saved as `deck`.
#[rstest]
#[case::without_host(false)]
#[case::host_deck(true)]
fn resume_workshop_of_a_library_pack_the_deck_does_not_pin_says_nothing(#[case] host_deck: bool) {
    within(move || {
        let data = fresh_dir("oku-slides-workshop-");
        let (s, _deck, _) = opener_on(data.path());
        let other = uuid::Uuid::new_v4();
        add_version(&s.library, &manifest_ns(other, "autre", &[]), &[]);
        let dir = untouched_workshop(s, data.path(), other);
        if host_deck {
            edit_state(dir.path(), |state| state["mode"]["pack"]["host"] = json!("deck"));
        }
        let (back, result) = resumed(data.path(), dir.path());
        assert_that!((result, back.snapshot().notice)).is_equal_to((Resumed::Restored, None));
    });
}

#[test]
fn resume_deck_only_workshop_with_an_unpinned_version_notices_the_dropped_edits() {
    within(|| {
        let data = fresh_dir("oku-slides-workshop-");
        let (mut s, _deck, _) = opener_on(data.path());
        let d = adopt_deck_only_pack(&mut s);
        let dir = untouched_workshop(s, data.path(), d.id);
        pack_doc_clean(dir.path());
        // As an older version left it: a session version the deck does not pin.
        let unpinned = crate::test_support::files_of(&manifest_ns(d.id, "importe", &[])).version();
        edit_state(dir.path(), |state| state["mode"]["pack"]["session_version"] = json!(unpinned.as_str()));
        let (back, _) = resumed(data.path(), dir.path());
        assert_that!(back.snapshot().notice.map(str::to_owned)).is_equal_to(Some(reopened_notice(&back)));
    });
}

#[test]
fn resume_workshop_with_a_pack_tab_document_is_set_aside() {
    within(|| {
        let data = fresh_dir("oku-slides-workshop-");
        let dormant = park(live_workshop(data.path()));
        let cached = TabDocument::Pack { pack: dormant.pack };
        let mut back = session_on(data.path());
        let result = back.resume_checked(dormant.dir.path().to_path_buf(), Some(&cached));
        assert_that!((result, back.tab_info().title == "opener")).is_equal_to((Resumed::Unreadable, false));
    });
}

/// t-145: an older workshop whose `pack/` is unreadable reopens on its deck, losing nothing of it.
#[test]
fn resume_with_unreadable_pack_returns_to_the_deck_without_loss() {
    within(|| {
        let data = fresh_dir("oku-slides-workshop-");
        let dormant = park(live_workshop(data.path()));
        std::fs::write(dormant.dir.path().join("pack").join("document.json"), b"pas du json").unwrap();
        let (back, result) = resumed(data.path(), dormant.dir.path());
        let snap = back.snapshot();
        let theme = back.deck().theme.as_ref().map(ToString::to_string);
        assert_that!((result, snap.editing.is_none(), snap.dirty)).is_equal_to((Resumed::Restored, true, true));
        assert_that!(theme).is_equal_to(Some("pro".to_owned()));
    });
}

/// t-394: an older workshop whose `pack/` is unreadable reopens on its deck without sub-tabs.
#[test]
fn resume_with_unreadable_pack_returns_no_widget_tabs() {
    within(|| {
        let data = fresh_dir("oku-slides-workshop-");
        let mut live = live_workshop(data.path());
        let ui = crate::views::UiState {
            widgets: vec!["_commun".into()],
            widget: Some("_commun".into()),
            ..crate::views::UiState::default()
        };
        ok(live.editor.handle(crate::Request::Ui { ui, order: None }), "ui");
        let dormant = park(live);
        edit_state(dormant.dir.path(), |state| assert_that!(state["ui"]["widgets"]).is_equal_to(json!(["_commun"])));
        std::fs::write(dormant.dir.path().join("pack").join("document.json"), b"pas du json").unwrap();
        let (back, _) = resumed(data.path(), dormant.dir.path());
        assert_that!(back.ui.widgets.is_empty()).is_true();
    });
}
