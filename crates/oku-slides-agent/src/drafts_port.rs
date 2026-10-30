//! Starting a tab's entry, and the host's [`DraftPort`] built on it.
//!
//! A deck's agent that opens a pack or theme tab passes the user's request to that tab's
//! draft ([`StackDrafts`]): the tab's entry starts (tools, keeper; never its ACP agent) and an
//! empty draft takes the text. The user reviews and sends it; no turn starts here.

use std::sync::{Arc, Mutex, PoisonError, Weak};

use oku_slides_mcp::{DraftPort, Prefill};
use oku_slides_session::Platform;
use oku_slides_session::agent::{Sessions, TabSession};
use tokio::runtime::Handle;
use tokio::sync::mpsc;

use crate::Link;
use crate::settings_port::AppSettings;
use crate::tab_entry::{Out, TabEntry, TabLink, TabPorts};
use crate::table::Table;
use crate::voice_port::AppVoice;

/// What starts a tab's entry, shared by the stack and its drafts port.
#[derive(Debug)]
pub(crate) struct Starter {
    sessions: Arc<dyn Sessions>,
    platform: Platform,
    /// The ports each started tab receives, [`StackDrafts`] included.
    ports: TabPorts,
    link: Link,
    rt: Handle,
    out_tx: mpsc::UnboundedSender<Out>,
    table: Arc<Mutex<Table>>,
}

/// The stack's parts a [`Starter`] is made of.
pub(crate) struct StarterParts {
    /// The sessions each tab's tools work on.
    pub sessions: Arc<dyn Sessions>,
    /// The platform (storage, config, localizer) the tabs run on.
    pub platform: Platform,
    /// The settings port given to each tab.
    pub settings: Arc<AppSettings>,
    /// The voice and agent port given to each tab.
    pub voice: Arc<AppVoice>,
    /// The agent's link to the interface.
    pub link: Link,
    /// The runtime the tabs' tasks run on.
    pub rt: Handle,
    /// Where the tabs broadcast their events.
    pub out_tx: mpsc::UnboundedSender<Out>,
    /// The stack's table of started tabs.
    pub table: Arc<Mutex<Table>>,
}

impl Starter {
    /// A starter whose tabs' drafts port reaches back to it, weakly.
    ///
    /// Weak because the stack's table holds the tabs, whose tools hold the port.
    pub fn new(parts: StarterParts) -> Arc<Self> {
        let StarterParts { sessions, platform, settings, voice, link, rt, out_tx, table } = parts;
        Arc::new_cyclic(|starter: &Weak<Self>| {
            let drafts = Arc::new(StackDrafts { starter: starter.clone() });
            Self { sessions, platform, ports: TabPorts { settings, voice, drafts }, link, rt, out_tx, table }
        })
    }

    /// The tab, started if it has no entry yet; `None` if closed or the app over.
    ///
    /// Checked under the lock the entry would be inserted under.
    ///
    /// Starts the tools and keeper only: the ACP agent waits for a prompt.
    pub fn open(&self, tab: &str) -> Option<TabLink> {
        let mut table = self.table.lock().unwrap_or_else(PoisonError::into_inner);
        if table.refuses(tab) {
            return None;
        }
        if let Some(e) = table.tabs.get(tab) {
            return Some(e.link.clone());
        }
        let session = TabSession::new(tab, self.sessions.clone());
        let ports = self.ports.clone();
        let entry = TabEntry::start(session, &self.platform, ports, self.link.clone(), &self.rt, self.out_tx.clone());
        let link = entry.link.clone();
        table.tabs.insert(tab.to_owned(), entry);
        Some(link)
    }
}

/// The host's [`DraftPort`]: prefills another tab's draft, starting its entry if needed.
#[derive(Debug)]
pub(crate) struct StackDrafts {
    starter: Weak<Starter>,
}

impl DraftPort for StackDrafts {
    /// `Unavailable` once the stack is gone or for an unknown or closed tab.
    ///
    /// Otherwise `Set` when the draft took the text, `Kept` when it held one already.
    ///
    /// The tab is looked up first, so an unknown one never gets an entry.
    fn prefill(&self, tab: &str, text: String) -> Prefill {
        let Some(starter) = self.starter.upgrade() else { return Prefill::Unavailable };
        if let Err(e) = starter.sessions.document_kind(tab) {
            tracing::debug!(tab, error = %e.one_line(), "draft prefill: unknown tab");
            return Prefill::Unavailable;
        }
        let Some(link) = starter.open(tab) else { return Prefill::Unavailable };
        if link.prefill_draft(&starter.out_tx, text) { Prefill::Set } else { Prefill::Kept }
    }
}

#[cfg(test)]
#[path = "drafts_port_tests.rs"]
mod drafts_port_tests;
