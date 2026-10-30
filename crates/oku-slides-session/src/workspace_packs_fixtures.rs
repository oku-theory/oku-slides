//! Shared fixtures of the pack-tab workspace tests: an opener deck tab, its pack tab, a restart.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oku_slides_core::{Namespace, PackKind, PackPin, PackVersion};
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::{Created, Opener, PackOpened};
use crate::agent::{TabGate, TabOpen};
use crate::pack_lease::LeaseScope;
use crate::test_support::{adopt_deck_only_pack, apply_ws, fresh_dir, rename_title, resumed_on, workspace_on};
use crate::test_support_packs::{acme_of, assert_unleased, opener_deck, scope_of};
use crate::{Request, Result, Session, TabDocument, Workspace};

/// A workspace whose tab `t-1` is an opener deck pinning the library pack `acme`.
pub(in crate::workspace) struct Packs {
    /// The workspace; dropped first.
    pub(in crate::workspace) ws: Arc<Mutex<Workspace>>,
    /// The library pack the deck pins.
    pub(in crate::workspace) acme: Uuid,
    /// The namespace the deck pins it under.
    pub(in crate::workspace) acme_ns: Namespace,
    /// The library's lease scope.
    pub(in crate::workspace) scope: LeaseScope,
    /// The resume folder, for a workspace that resumes.
    pub(in crate::workspace) resume: Option<PathBuf>,
    /// The opener deck's file folder.
    pub(in crate::workspace) deck: tempfile::TempDir,
    /// The data folder; dropped last.
    pub(in crate::workspace) data: tempfile::TempDir,
}

/// The fixture without resume.
pub(in crate::workspace) fn packs() -> Packs {
    let data = fresh_dir("oku-slides-ws-packs-");
    let ws = workspace_on(data.path());
    seeded(ws, data, None)
}

/// The fixture on a resume folder: tabs are persisted and a restart reads them.
pub(in crate::workspace) fn packs_resumed() -> Packs {
    let data = fresh_dir("oku-slides-ws-packs-");
    let resume = data.path().join("resume");
    let ws = resumed_on(data.path(), &resume);
    seeded(ws, data, Some(resume))
}

/// `t-1` of `ws` made an opener deck.
fn seeded(ws: Mutex<Workspace>, data: tempfile::TempDir, resume: Option<PathBuf>) -> Packs {
    let (deck, acme, acme_ns, scope) = Workspace::with_session(&ws, "t-1", |s| {
        let deck = opener_deck(s, data.path());
        let acme = acme_of(s);
        let ns = s.user_deck().widgets.iter().find(|(_, p)| p.id == acme).map(|(ns, _)| ns.clone());
        (deck, acme, ns.unwrap_or_else(|| panic!("acme pinned")), scope_of(s))
    })
    .0
    .unwrap_or_else(|e| panic!("opener deck: {}", e.one_line()));
    Packs { ws: Arc::new(ws), acme, acme_ns, scope, resume, deck, data }
}

impl Packs {
    /// `f` on the session of `tab`.
    pub(in crate::workspace) fn with<R>(&self, tab: &str, f: impl FnOnce(&mut Session) -> R) -> R {
        Workspace::with_session(&self.ws, tab, f).0.unwrap_or_else(|e| panic!("{tab}: {}", e.one_line()))
    }

    /// Opens `pack` from `tab` as the user.
    pub(in crate::workspace) fn open(&self, tab: &str, pack: Uuid, widget: Option<&str>) -> Result<PackOpened> {
        Workspace::open_pack_with(&self.ws, tab, pack, widget, Opener::Ui, Created::No)
    }

    /// Opens `pack` from `tab` for an agent whose gate is `gate`.
    pub(in crate::workspace) fn open_by_agent(
        &self,
        tab: &str,
        pack: Uuid,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> Result<TabOpen> {
        Workspace::open_pack(&self.ws, tab, pack, None, gate)
    }

    /// Opens `acme` from `t-1` as the user: the new pack tab.
    pub(in crate::workspace) fn pack_tab(&self) -> String {
        match self.open("t-1", self.acme, None) {
            Ok(PackOpened::Opened { tab, .. }) => tab,
            other => panic!("a new pack tab: {other:?}"),
        }
    }

    /// Sends a workspace request.
    pub(in crate::workspace) fn send(&self, v: Value) {
        apply_ws(&self.ws, v).unwrap_or_else(|e| panic!("workspace request: {}", e.one_line()));
    }

    /// A blank tab at the end of `w-1`: its id.
    pub(in crate::workspace) fn new_tab(&self) -> String {
        self.send(json!({"cmd": "new_tab", "window": "w-1"}));
        self.window(0).last().cloned().unwrap_or_else(|| panic!("a tab"))
    }

    /// Activates `tab`.
    pub(in crate::workspace) fn activate(&self, tab: &str) {
        self.send(json!({"cmd": "activate", "tab": tab}));
    }

    /// Closes `tab`, discarding its work.
    pub(in crate::workspace) fn close(&self, tab: &str) {
        self.send(json!({"cmd": "close", "tab": tab, "discard": true}));
    }

    /// The tabs of the `i`-th window, in bar order.
    pub(in crate::workspace) fn window(&self, i: usize) -> Vec<String> {
        Workspace::lock(&self.ws).layout.windows[i].tabs.clone()
    }

    /// The active tab of the `i`-th window.
    pub(in crate::workspace) fn active(&self, i: usize) -> String {
        Workspace::lock(&self.ws).layout.windows[i].active.clone()
    }

    /// Every open tab.
    pub(in crate::workspace) fn tab_count(&self) -> usize {
        Workspace::lock(&self.ws).tabs.len()
    }

    /// The opener of `tab`.
    pub(in crate::workspace) fn opener_of(&self, tab: &str) -> Option<String> {
        Workspace::lock(&self.ws).tabs.get(tab).and_then(|t| t.opener.clone())
    }

    /// The cached document of `tab`.
    pub(in crate::workspace) fn document(&self, tab: &str) -> Option<TabDocument> {
        Workspace::lock(&self.ws).tabs.get(tab).map(|t| t.info.document.clone())
    }

    /// Whether `tab`'s session still sleeps.
    pub(in crate::workspace) fn is_dormant(&self, tab: &str) -> bool {
        let slot = Workspace::lock(&self.ws).session(tab).unwrap_or_else(|e| panic!("{tab}: {}", e.one_line()));
        super::super::relock(&slot).is_dormant()
    }

    /// A version `tab`, a pack tab, wrote: a rename, saved.
    pub(in crate::workspace) fn write_in(&self, tab: &str) -> PackVersion {
        self.with(tab, |s| {
            s.apply("x", vec![rename_title("acme 2")], None).unwrap_or_else(|e| panic!("edit: {}", e.one_line()));
            s.save_pack(false).unwrap_or_else(|e| panic!("save: {}", e.one_line()))
        })
    }

    /// Settles and drops the workspace (no lease left), edits its `workspace.json`, then resumes it.
    pub(in crate::workspace) fn restart(self, forge: impl FnOnce(&Path, &mut Value)) -> Self {
        let Self { ws, acme, acme_ns, scope, resume, deck, data } = self;
        let resume = resume.unwrap_or_else(|| panic!("a resumed fixture"));
        Workspace::settle(&ws);
        drop(ws);
        assert_unleased(&scope, acme);
        let file = resume.join("workspace.json");
        let mut layout: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        forge(&resume, &mut layout);
        std::fs::write(&file, serde_json::to_vec(&layout).unwrap()).unwrap();
        let ws = Arc::new(resumed_on(data.path(), &resume));
        Self { ws, acme, acme_ns, scope, resume: Some(resume), deck, data }
    }
}

/// Copies tab folder `from` to `to` under `resume`, and cites `to` after it in `layout`'s first window.
pub(in crate::workspace) fn duplicate_tab(resume: &Path, layout: &mut Value, from: &str, to: &str) {
    copy_dir(&resume.join("tabs").join(from), &resume.join("tabs").join(to));
    let tabs = layout["windows"][0]["tabs"].as_array_mut().unwrap();
    let at = tabs.iter().position(|t| t == from).unwrap() + 1;
    tabs.insert(at, json!(to));
    let info = layout["info"][from].clone();
    layout["info"][to] = info;
}

/// Copies a folder tree.
pub(in crate::workspace) fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Marks `tab`'s saved entry in `layout` as a pack tab of `pack`.
pub(in crate::workspace) fn forge_pack_entry(layout: &mut Value, tab: &str, pack: Uuid) {
    layout["info"][tab]["document"] = json!({"kind": "pack", "pack": pack});
}

/// Runs `f` on another thread; panics if it takes more than 10 s (a deadlock).
pub(in crate::workspace) fn within<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    within_secs(10, f)
}

/// Runs `f` on another thread; panics if it takes more than `secs` (a deadlock).
///
/// For scenarios that restart a workspace: each restart renders every woken tab again.
pub(in crate::workspace) fn within_secs<R: Send + 'static>(secs: u64, f: impl FnOnce() -> R + Send + 'static) -> R {
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(f());
    });
    finished.recv_timeout(Duration::from_secs(secs)).unwrap_or_else(|_| panic!("deadlock: no answer within {secs} s"))
}

/// The kind of `r`'s error, or its outcome as a [`TabOpen`]: `Focused("t-2")`, `Opened("t-2", hidden)`.
pub(in crate::workspace) fn outcome(r: Result<impl Into<TabOpen>>) -> String {
    let moved = |name: &str, tab: &str, shown: bool| {
        if shown { format!("{name}({tab:?})") } else { format!("{name}({tab:?}, hidden)") }
    };
    match r.map(Into::into) {
        Ok(TabOpen::Opened { tab, shown }) => moved("Opened", &tab, shown),
        Ok(TabOpen::Focused { tab, shown }) => moved("Focused", &tab, shown),
        Ok(o @ (TabOpen::Here | TabOpen::Refused)) => format!("{o:?}"),
        Err(e) => format!("{:?}", e.kind()),
    }
}

/// Asserts the outcome of an open, by name.
pub(in crate::workspace) fn assert_outcome(r: Result<impl Into<TabOpen>>, expected: &str) {
    assert_that!(outcome(r)).is_equal_to(expected.to_owned());
}

/// The fixture with a deck-only pack adopted by `t-1`, saved: the fixture and the pack's pin.
pub(in crate::workspace) fn deck_packs() -> (Packs, PackPin) {
    let f = packs();
    let pin = f.adopt_deck_only();
    (f, pin)
}

/// `deck_packs` on a resume folder.
pub(in crate::workspace) fn deck_packs_resumed() -> (Packs, PackPin) {
    let f = packs_resumed();
    let pin = f.adopt_deck_only();
    (f, pin)
}

impl Packs {
    /// `t-1` adopts a deck-only pack and saves its file: the pack's pin.
    pub(in crate::workspace) fn adopt_deck_only(&self) -> PackPin {
        self.with("t-1", |s| {
            let pin = adopt_deck_only_pack(s);
            // Through `handle`, which persists the tab's state as the app does.
            s.handle(Request::Save { path: None, overwrite: false })
                .unwrap_or_else(|e| panic!("save: {}", e.one_line()));
            pin
        })
    }

    /// Opens deck-only `pack` from `t-1` as the user: the new deck-pack tab.
    pub(in crate::workspace) fn deck_pack_tab(&self, pack: Uuid) -> String {
        match self.open("t-1", pack, None) {
            Ok(PackOpened::Opened { tab, .. }) => tab,
            other => panic!("a new deck-pack tab: {other:?}"),
        }
    }

    /// An unsaved rename in deck-pack tab `tab`.
    pub(in crate::workspace) fn edit_in(&self, tab: &str, title: &str) {
        self.with(tab, |s| s.apply("x", vec![rename_title(title)], None))
            .unwrap_or_else(|e| panic!("edit: {}", e.one_line()));
    }

    /// Saves pack tab `tab` through a public request (`as_new`: a copy).
    pub(in crate::workspace) fn save_pack(&self, tab: &str, as_new: bool) -> Result<String> {
        Workspace::request(&self.ws, tab, Request::SavePack { as_new }).0
    }

    /// A rename then a save in deck-pack tab `tab`, which must succeed.
    pub(in crate::workspace) fn save_in(&self, tab: &str, title: &str) {
        self.edit_in(tab, title);
        self.save_pack(tab, false).unwrap_or_else(|e| panic!("save pack: {}", e.one_line()));
    }

    /// The version of `pack` the deck of `tab` pins.
    pub(in crate::workspace) fn pinned(&self, tab: &str, pack: Uuid) -> Option<PackVersion> {
        self.with(tab, |s| s.user_deck().all_pins().find(|(_, _, p)| p.id == pack).map(|(_, _, p)| p.version.clone()))
    }

    /// Whether `tab`'s deck is dirty.
    pub(in crate::workspace) fn dirty(&self, tab: &str) -> bool {
        self.with(tab, |s| s.doc.dirty)
    }

    /// Whether deck-pack tab `tab` holds work its owner has not taken.
    pub(in crate::workspace) fn unsent(&self, tab: &str) -> bool {
        self.with(tab, |s| s.unsent())
    }

    /// An owner write of the deck-only pack of `tab`: its removal from the deck.
    pub(in crate::workspace) fn owner_write(&self, tab: &str) -> Result<()> {
        self.with(tab, |s| s.remove_deck_pack(PackKind::Widgets, &Namespace::deck()))
    }

    /// Whether every window's `active` is one of its tabs.
    pub(in crate::workspace) fn actives_valid(&self) -> bool {
        Workspace::lock(&self.ws).layout.windows.iter().all(|w| w.tabs.contains(&w.active))
    }
}

impl Packs {
    /// Closes `tab`, keeping its work: the outcome.
    pub(in crate::workspace) fn close_keeping(&self, tab: &str) -> Result<crate::WorkspaceOutcome> {
        let req = serde_json::from_value(json!({"cmd": "close", "tab": tab})).unwrap_or_else(|e| panic!("{e}"));
        Workspace::apply(&self.ws, req)
    }
}
