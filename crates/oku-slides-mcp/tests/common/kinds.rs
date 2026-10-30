//! A server per tab kind: deck, help, pack and theme, and the tools each lists.

use std::sync::{Arc, Mutex};

use oku_slides_session::agent::TabSession;
use oku_slides_session::{DocumentKind, Fonts, PAPIER_THEME, Session, SessionErrorKind};
use rust_mcp_sdk::McpClient;
use rust_mcp_sdk::mcp_client::ClientRuntime;

use super::{Harness, lock, platform_with, start, start_pack_tab, start_theme_tab, start_with};

/// The library pack a pack-kind server edits: `base`.
pub const PACK: uuid::Uuid = oku_slides_session::pack::BASE_PACK;

/// The kind of tab a test server is bound to.
#[derive(Debug, Clone, Copy)]
pub enum Kind {
    /// The fixture deck ([`start`]).
    Deck,
    /// The shipped help ([`start_help_tab`]).
    Help,
    /// A pack tab of [`PACK`] ([`start_pack_tab`]).
    Pack,
    /// A theme tab of `papier` ([`start_theme_tab`]).
    Theme,
}

impl Kind {
    /// A server bound to a tab of this kind.
    pub async fn start(self) -> Harness {
        match self {
            Self::Deck => start().await,
            Self::Help => start_help_tab().await,
            Self::Pack => start_pack_tab(PACK).await,
            Self::Theme => start_theme_tab(PAPIER_THEME).await,
        }
    }
}

/// [`start`] on a help tab (the shipped help, in French) instead of the fixture deck.
pub async fn start_help_tab() -> Harness {
    let (platform, session, data) = tokio::task::spawn_blocking(|| {
        let (platform, data) = platform_with(|_| {});
        let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("test session: {e}"));
        session.open_help("fr", None).unwrap_or_else(|e| panic!("help tab: {}", e.one_line()));
        (platform, session, data)
    })
    .await
    .unwrap_or_else(|e| panic!("blocking setup task: {e}"));
    let session = Arc::new(Mutex::new(session));
    start_with((platform, data), session.clone(), session, "t-1").await
}

/// The names of the tools the server lists to `c`, in its order.
pub async fn list_tool_names(c: &Arc<ClientRuntime>) -> Vec<String> {
    let tools = c.request_tool_list(None).await.unwrap_or_else(|e| panic!("tool list: {e}")).tools;
    tools.into_iter().map(|t| t.name).collect()
}

/// The kind of `tab`, as its host reads it.
///
/// A closed tab has none (`UnknownTab`): the harness session's kind stands in for it (`start_closed`).
/// Any other host error is a harness bug and panics.
pub(super) async fn kind_of(tab: &TabSession, session: &Arc<Mutex<Session>>) -> DocumentKind {
    let (tab, session) = (tab.clone(), session.clone());
    tokio::task::spawn_blocking(move || match tab.document_kind() {
        Ok(kind) => kind,
        Err(e) if e.kind() == SessionErrorKind::UnknownTab => lock(&session).document_kind(),
        Err(e) => panic!("tab kind read: {}", e.one_line()),
    })
    .await
    .unwrap_or_else(|e| panic!("blocking kind read: {e}"))
}

/// The `UnknownTool` reply to a call of `name`, as the harness's localizer says it.
pub fn unknown_tool(h: &Harness, name: &str) -> String {
    h.localizer.tr_with("agent-error-unknown-tool", &[("name", name.into())])
}
