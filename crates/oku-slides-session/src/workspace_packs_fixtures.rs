//! Shared fixtures of the pack-tab workspace tests: an opener deck tab, its pack tab, a restart.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oku_slides_core::{Namespace, PackVersion};
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::{Opener, PackOpened};
use crate::agent::PackGate;
use crate::pack_lease::LeaseScope;
use crate::test_support::{apply_ws, fresh_dir, rename_title, resumed_on, workspace_on};
use crate::test_support_packs::{acme_of, assert_unleased, opener_deck, scope_of};
use crate::{Result, Session, TabDocument, Workspace};

/// A workspace whose tab `t-1` is an opener deck pinning the library pack `acme`.
pub(super) struct Packs {
    /// The workspace; dropped first.
    pub(super) ws: Arc<Mutex<Workspace>>,
    /// The library pack the deck pins.
    pub(super) acme: Uuid,
    /// The namespace the deck pins it under.
    pub(super) acme_ns: Namespace,
    /// The library's lease scope.
    pub(super) scope: LeaseScope,
    /// The resume folder, for a workspace that resumes.
    pub(super) resume: Option<PathBuf>,
    /// The opener deck's file folder.
    pub(super) deck: tempfile::TempDir,
    /// The data folder; dropped last.
    pub(super) data: tempfile::TempDir,
}

/// The fixture without resume.
pub(super) fn packs() -> Packs {
    let data = fresh_dir("oku-slides-ws-packs-");
    let ws = workspace_on(data.path());
    seeded(ws, data, None)
}

/// The fixture on a resume folder: tabs are persisted and a restart reads them.
pub(super) fn packs_resumed() -> Packs {
    let data = fresh_dir("oku-slides-ws-packs-");
    let resume = data.path().join("resume");
    let ws = resumed_on(data.path(), &resume);
    seeded(ws, data, Some(resume))
}

/// `t-1` of `ws` made an opener deck.
fn seeded(ws: Mutex<Workspace>, data: tempfile::TempDir, resume: Option<PathBuf>) -> Packs {
    let (deck, acme, acme_ns, scope) = Workspace::with_session(&ws, "t-1", |s| {
        let deck = opener_deck(s);
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
    pub(super) fn with<R>(&self, tab: &str, f: impl FnOnce(&mut Session) -> R) -> R {
        Workspace::with_session(&self.ws, tab, f).0.unwrap_or_else(|e| panic!("{tab}: {}", e.one_line()))
    }

    /// Opens `pack` from `tab` as the user.
    pub(super) fn open(&self, tab: &str, pack: Uuid, widget: Option<&str>) -> Result<PackOpened> {
        Workspace::open_pack_with(&self.ws, tab, pack, widget, Opener::Ui)
    }

    /// Opens `pack` from `tab` for an agent whose gate is `gate`.
    pub(super) fn open_by_agent(
        &self,
        tab: &str,
        pack: Uuid,
        gate: &mut dyn FnMut(PackGate) -> bool,
    ) -> Result<PackOpened> {
        Workspace::open_pack_with(&self.ws, tab, pack, None, Opener::Agent(gate))
    }

    /// Opens `acme` from `t-1` as the user: the new pack tab.
    pub(super) fn pack_tab(&self) -> String {
        match self.open("t-1", self.acme, None) {
            Ok(PackOpened::Opened(id)) => id,
            other => panic!("a new pack tab: {other:?}"),
        }
    }

    /// Sends a workspace request.
    pub(super) fn send(&self, v: Value) {
        apply_ws(&self.ws, v).unwrap_or_else(|e| panic!("workspace request: {}", e.one_line()));
    }

    /// A blank tab at the end of `w-1`: its id.
    pub(super) fn new_tab(&self) -> String {
        self.send(json!({"cmd": "new_tab", "window": "w-1"}));
        self.window(0).last().cloned().unwrap_or_else(|| panic!("a tab"))
    }

    /// Activates `tab`.
    pub(super) fn activate(&self, tab: &str) {
        self.send(json!({"cmd": "activate", "tab": tab}));
    }

    /// Closes `tab`, discarding its work.
    pub(super) fn close(&self, tab: &str) {
        self.send(json!({"cmd": "close", "tab": tab, "discard": true}));
    }

    /// The tabs of the `i`-th window, in bar order.
    pub(super) fn window(&self, i: usize) -> Vec<String> {
        Workspace::lock(&self.ws).layout.windows[i].tabs.clone()
    }

    /// The active tab of the `i`-th window.
    pub(super) fn active(&self, i: usize) -> String {
        Workspace::lock(&self.ws).layout.windows[i].active.clone()
    }

    /// Every open tab.
    pub(super) fn tab_count(&self) -> usize {
        Workspace::lock(&self.ws).tabs.len()
    }

    /// The opener of `tab`.
    pub(super) fn opener_of(&self, tab: &str) -> Option<String> {
        Workspace::lock(&self.ws).tabs.get(tab).and_then(|t| t.opener.clone())
    }

    /// The cached document of `tab`.
    pub(super) fn document(&self, tab: &str) -> Option<TabDocument> {
        Workspace::lock(&self.ws).tabs.get(tab).map(|t| t.info.document.clone())
    }

    /// Whether `tab`'s session still sleeps.
    pub(super) fn is_dormant(&self, tab: &str) -> bool {
        let slot = Workspace::lock(&self.ws).session(tab).unwrap_or_else(|e| panic!("{tab}: {}", e.one_line()));
        super::super::relock(&slot).is_dormant()
    }

    /// A version `tab`, a pack tab, wrote: a rename, saved.
    pub(super) fn write_in(&self, tab: &str) -> PackVersion {
        self.with(tab, |s| {
            s.apply("x", vec![rename_title("acme 2")], None).unwrap_or_else(|e| panic!("edit: {}", e.one_line()));
            s.save_pack(false).unwrap_or_else(|e| panic!("save: {}", e.one_line()))
        })
    }

    /// Settles and drops the workspace (no lease left), edits its `workspace.json`, then resumes it.
    pub(super) fn restart(self, forge: impl FnOnce(&Path, &mut Value)) -> Self {
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
pub(super) fn duplicate_tab(resume: &Path, layout: &mut Value, from: &str, to: &str) {
    copy_dir(&resume.join("tabs").join(from), &resume.join("tabs").join(to));
    let tabs = layout["windows"][0]["tabs"].as_array_mut().unwrap();
    let at = tabs.iter().position(|t| t == from).unwrap() + 1;
    tabs.insert(at, json!(to));
    let info = layout["info"][from].clone();
    layout["info"][to] = info;
}

/// Copies a folder tree.
pub(super) fn copy_dir(from: &Path, to: &Path) {
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
pub(super) fn forge_pack_entry(layout: &mut Value, tab: &str, pack: Uuid) {
    layout["info"][tab]["document"] = json!({"kind": "pack", "pack": pack});
}

/// Runs `f` on another thread; panics if it takes more than 10 s (a deadlock).
pub(super) fn within<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(f());
    });
    finished.recv_timeout(Duration::from_secs(10)).unwrap_or_else(|_| panic!("deadlock: no answer within 10 s"))
}

/// The kind of `r`'s error, or its outcome's name.
pub(super) fn outcome(r: Result<PackOpened>) -> String {
    match r {
        Ok(o) => format!("{o:?}"),
        Err(e) => format!("{:?}", e.kind()),
    }
}

/// Asserts the outcome of an open, by name.
pub(super) fn assert_outcome(r: Result<PackOpened>, expected: &str) {
    assert_that!(outcome(r)).is_equal_to(expected.to_owned());
}
