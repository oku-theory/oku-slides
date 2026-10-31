//! A multi-tab `Sessions` fake: files open elsewhere, pack and theme opens recorded.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use oku_slides_session::agent::{Sessions, TabGate, TabOpen};
use oku_slides_session::{Fonts, Session};
use speculoos::prelude::*;

use super::{Harness, lock, prepared, start_with};

/// A multi-tab `Sessions`: every file is already open in another tab, and
/// `open_tab` records the open, then returns `t-7`.
///
/// An allowed `open_pack` opens `t-7` (or lands `Here` for [`OtherTabs::HERE_PACK`]); an allowed
/// `open_theme` opens `t-9` (or lands `Here` once [`OtherTabs::theme_here_next`] is set).
#[derive(Debug)]
pub struct OtherTabs {
    session: Arc<Mutex<Session>>,
    /// `(tab, file)` of each `open_tab`.
    pub opened: Mutex<Vec<(String, std::path::PathBuf)>>,
    /// `(tab, pack, widget)` of each `open_pack` whose gate allowed a new tab.
    pub opened_packs: Mutex<Vec<(String, uuid::Uuid, Option<String>)>>,
    /// Each theme an `open_theme` whose gate allowed a new tab opened.
    pub opened_themes: Mutex<Vec<uuid::Uuid>>,
    /// Every answer of the gate to `NewTab`, `false` ones included.
    pub gate_answers: Mutex<Vec<bool>>,
    /// The next allowed `open_pack` fails after its gate (`UnknownTab`).
    pub fail_next: std::sync::atomic::AtomicBool,
    /// The next allowed `open_theme` lands `Here`, as an open that found the theme in the requesting tab.
    pub theme_here_next: std::sync::atomic::AtomicBool,
    /// The next allowed `open_pack` waits here, after its gate, for a message or a drop.
    pub hold: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
    /// A workspace without the tab the pack is opened from: the source of the failure.
    closed: Mutex<oku_slides_session::Workspace>,
}

impl OtherTabs {
    /// The pack this tab already edits: its open asks the gate, then lands `Here`.
    pub const HERE_PACK: uuid::Uuid = uuid::Uuid::from_u128(0x4e7e_0000_0000_4000_8000_0000_0000_0001);

    /// The themes opened so far, in order.
    pub fn opened_themes(&self) -> Vec<uuid::Uuid> {
        lock(&self.opened_themes).clone()
    }

    /// Holds the next allowed `open_pack` after its gate; send on, or drop, the sender to release it.
    pub fn hold_next(&self) -> std::sync::mpsc::Sender<()> {
        let (release, held) = std::sync::mpsc::channel();
        *lock(&self.hold) = Some(held);
        release
    }
}

impl Sessions for OtherTabs {
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        self.session.with(tab, f)
    }

    fn open_elsewhere(&self, _tab: &str, _path: &std::path::Path) -> bool {
        true
    }

    fn open_tab(&self, tab: &str, path: &std::path::Path) -> oku_slides_session::Result<String> {
        lock(&self.opened).push((tab.to_owned(), path.to_owned()));
        Ok("t-7".to_owned())
    }

    /// A new pack tab `t-7`, if the gate allows it: the gate, the record, the hold, then the result.
    ///
    /// [`OtherTabs::HERE_PACK`] lands `Here` once its gate allowed it, as an open that found the
    /// pack in the requesting tab after claiming.
    fn open_pack(
        &self,
        tab: &str,
        pack: uuid::Uuid,
        widget: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        let allowed = gate(TabGate::NewTab);
        lock(&self.gate_answers).push(allowed);
        if !allowed {
            return Ok(TabOpen::Refused);
        }
        if pack == Self::HERE_PACK {
            return Ok(TabOpen::Here);
        }
        lock(&self.opened_packs).push((tab.to_owned(), pack, widget.map(str::to_owned)));
        // Taken out of its mutex: the wait holds no lock.
        let held = lock(&self.hold).take();
        if let Some(held) = held {
            let _ = held.recv_timeout(std::time::Duration::from_secs(10));
        }
        if self.fail_next.swap(false, std::sync::atomic::Ordering::SeqCst) {
            return self.closed.open_pack("onglet-ferme", pack, widget, &mut |_| false);
        }
        Ok(TabOpen::Opened { tab: "t-7".to_owned(), shown: true })
    }

    /// A new theme tab `t-9`, if the gate allows it: the gate, the record, then the result.
    fn open_theme(
        &self,
        _tab: &str,
        theme: uuid::Uuid,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        let allowed = gate(TabGate::NewTab);
        lock(&self.gate_answers).push(allowed);
        if !allowed {
            return Ok(TabOpen::Refused);
        }
        if self.theme_here_next.swap(false, std::sync::atomic::Ordering::SeqCst) {
            return Ok(TabOpen::Here);
        }
        lock(&self.opened_themes).push(theme);
        Ok(TabOpen::Opened { tab: "t-9".to_owned(), shown: true })
    }

    fn document_kind(&self, tab: &str) -> oku_slides_session::Result<oku_slides_session::DocumentKind> {
        self.session.document_kind(tab)
    }

    fn usage_owner(&self, tab: &str) -> Option<String> {
        self.session.usage_owner(tab)
    }
}

/// A server among other tabs (see [`OtherTabs`]).
pub async fn start_other_tabs() -> (Harness, Arc<OtherTabs>) {
    let (platform, session, data) = prepared().await;
    let (platform, closed) = tokio::task::spawn_blocking(move || {
        let closed = oku_slides_session::Workspace::new(Fonts::default(), &platform, None)
            .unwrap_or_else(|e| panic!("test workspace: {e}"));
        (platform, closed)
    })
    .await
    .unwrap_or_else(|e| panic!("workspace blocking task: {e}"));
    let session = Arc::new(Mutex::new(session));
    let tabs = Arc::new(OtherTabs {
        session: session.clone(),
        opened: Mutex::default(),
        opened_packs: Mutex::default(),
        opened_themes: Mutex::default(),
        gate_answers: Mutex::default(),
        fail_next: std::sync::atomic::AtomicBool::new(false),
        theme_here_next: std::sync::atomic::AtomicBool::new(false),
        hold: Mutex::default(),
        closed: Mutex::new(closed),
    });
    let h = start_with((platform, data), tabs.clone(), session, "t-1").await;
    (h, tabs)
}

/// Polls until `done`, at most 10 s: a held open reaching its hold, for instance.
pub async fn until(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert_that!(Instant::now() < deadline).named(what).is_true();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
