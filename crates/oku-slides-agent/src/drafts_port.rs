//! Starting a tab's entry, and the prefill and paging the host's tab port builds on it.
//!
//! A deck's agent that opens a pack or theme tab passes the user's request to that tab's
//! draft ([`Starter::prefill`]): the tab's entry starts (tools, keeper; never its ACP agent) and
//! an empty draft takes the text. The user reviews and sends it; no turn starts here. It also
//! pages another tab ([`Starter::page`]): a `Focus` event on that tab's hub, which its pages follow.

use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_mcp::{McpEvent, Prefill, TabPort};
use oku_slides_session::Platform;
use oku_slides_session::agent::{AgentWorkspace, AppPort, Sessions, TabSession};
use tokio::runtime::Handle;
use tokio::sync::mpsc;

use crate::Link;
use crate::after_turn::AfterTx;
use crate::settings_port::AppSettings;
use crate::tab_entry::{Out, TabEntry, TabLink, TabPorts};
use crate::table::Table;
use crate::voice_port::AppVoice;

/// What starts a tab's entry, shared by the stack and its tab port.
#[derive(Debug)]
pub(crate) struct Starter {
    sessions: Arc<dyn Sessions>,
    platform: Platform,
    /// The ports each started tab receives.
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
    /// The host's workspace port given to each tab.
    pub workspace: Arc<dyn AgentWorkspace>,
    /// Other tabs' draft, page and fine-tuning turn, for each tab's tools.
    pub tabs: Arc<dyn TabPort>,
    /// The host's app port given to each tab.
    pub app: Arc<dyn AppPort>,
    /// Where each tab's actor sends its ended turns' app actions.
    pub after: AfterTx,
    /// The agent's link to the interface.
    pub link: Link,
    /// The runtime the tabs' tasks run on.
    pub rt: Handle,
    /// Where the tabs broadcast their events.
    pub out_tx: mpsc::UnboundedSender<Out>,
    /// The stack's table of started tabs.
    pub table: Arc<Mutex<Table>>,
    /// The read-aloud, handed each tab's surfaces; `None` without a voice.
    pub reader: Option<crate::reader::ReaderTx>,
}

impl Starter {
    /// A starter whose tabs receive `parts`' ports.
    pub fn new(parts: StarterParts) -> Arc<Self> {
        let StarterParts {
            sessions,
            platform,
            settings,
            voice,
            workspace,
            tabs,
            app,
            after,
            link,
            rt,
            out_tx,
            table,
            reader,
        } = parts;
        let tab_ports = TabPorts { settings, voice, tabs, workspace, app, after, reader };
        Arc::new(Self { sessions, platform, ports: tab_ports, link, rt, out_tx, table })
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

impl Starter {
    /// Prefills `tab`'s draft with `text`, starting its entry if needed.
    ///
    /// `Unavailable` for an unknown or closed tab; otherwise `Set` when the draft took the text,
    /// `Kept` when it held one already. The tab is looked up first, so an unknown one never gets
    /// an entry. Blocking: call it off the async threads.
    pub fn prefill(&self, tab: &str, text: String) -> Prefill {
        if let Err(e) = self.sessions.document_kind(tab) {
            tracing::debug!(tab, error = %e.one_line(), "draft prefill: unknown tab");
            return Prefill::Unavailable;
        }
        let Some(link) = self.open(tab) else { return Prefill::Unavailable };
        if link.prefill_draft(&self.out_tx, text) { Prefill::Set } else { Prefill::Kept }
    }

    /// Shows slide `slide` on `tab`'s pages; false for an unknown or closed tab, looked up first.
    ///
    /// Blocking, as [`Starter::prefill`].
    pub fn page(&self, tab: &str, slide: String) -> bool {
        if let Err(e) = self.sessions.document_kind(tab) {
            tracing::debug!(tab, error = %e.one_line(), "page: unknown tab");
            return false;
        }
        let Some(link) = self.open(tab) else { return false };
        link.hub.emit(McpEvent::Focus { slide: Some(slide) });
        true
    }
}

#[cfg(test)]
#[path = "drafts_port_tests.rs"]
mod drafts_port_tests;
