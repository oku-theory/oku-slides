//! Test helpers: the real MCP tools of a deck tab, for the manual model tests.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use oku_slides_mcp::{Applied, SettingsPort, SurfaceHub, ToolServer, Tools};
use oku_slides_platform::config::Settings;
use oku_slides_session::agent::TabSession;
use oku_slides_session::{Fonts, Platform, Session};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::compact::ToolSpec;
use crate::mcp::McpTools;
use crate::sessions::McpAccess;
use crate::turn::ToolPort;

/// Read-only settings: the manual tests change none.
#[derive(Debug)]
pub(crate) struct NoSettings;

impl SettingsPort for NoSettings {
    fn read(&self) -> std::collections::BTreeMap<String, Value> {
        std::collections::BTreeMap::default()
    }
    fn write(&self, _: &str, _: &Value) -> Result<Applied, String> {
        Err("read-only".into())
    }
}

/// The full tools of a deck tab on the roadmap fixture, listed over MCP as a turn lists them.
pub(crate) async fn deck_tools(dir: &std::path::Path, cancel: &CancellationToken) -> (Vec<ToolSpec>, ToolServer) {
    let data = dir.to_owned();
    let (session, platform) = tokio::task::spawn_blocking(move || {
        let mut settings = Settings::defaults().unwrap();
        settings.paths.data = Some(data.clone());
        settings.paths.decks = Some(data.join("decks"));
        settings.i18n.locale = Some("fr".into());
        let platform = Platform::with_settings(settings).unwrap();
        let mut session = Session::new(Fonts::default(), &platform).unwrap();
        session.open(&PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))).unwrap();
        (session, platform)
    })
    .await
    .unwrap();
    let kind = session.document_kind();
    let tab = TabSession::new("t-1", Arc::new(Mutex::new(session)));
    let hub = SurfaceHub::new(tab.clone(), platform.live_localizer.clone(), Arc::new(oku_slides_mcp::NoVoice));
    let tools = Tools::new(tab, hub, Arc::new(NoSettings), Arc::new(oku_slides_mcp::NoDrafts), &platform, kind);
    let server = ToolServer::start(tools, cancel.clone()).await.unwrap();
    let access = McpAccess { url: server.url().to_owned(), authorization: server.authorization() };
    let listed = McpTools::connect(&access).await.unwrap().list().await.unwrap();
    // `Storage` owns a runtime: its last copy must not drop inside the async test.
    std::mem::forget(platform);
    (listed, server)
}
