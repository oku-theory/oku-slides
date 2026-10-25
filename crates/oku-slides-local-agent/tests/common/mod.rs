//! Outillage de bout en bout : un onglet réel, ses outils MCP, et l'agent local de test.
#![allow(dead_code, reason = "chaque fichier de test n'utilise qu'une partie de l'outillage")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oku_slides_agent::{AgentEvent, AgentHandle, Keeper, Launch, Link, ToolsAccess};
use oku_slides_mcp::{Applied, SettingsPort, SurfaceHub, ToolServer, Tools};
use oku_slides_session::agent::TabSession;
use oku_slides_session::{Fonts, Platform, Session, Settings};
use serde_json::Value;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// Des réglages en lecture seule : ces tests n'en touchent aucun.
#[derive(Debug, Default)]
pub struct NoSettings;

impl SettingsPort for NoSettings {
    fn read(&self) -> std::collections::BTreeMap<String, Value> {
        std::collections::BTreeMap::default()
    }
    fn write(&self, _: &str, _: &Value) -> Result<Applied, String> {
        Err("read-only".into())
    }
}

/// Un onglet : sa session, ses outils, son agent, et ses évènements depuis le démarrage.
pub struct Tab {
    pub agent: AgentHandle,
    pub hub: Arc<SurfaceHub>,
    pub session: Arc<Mutex<Session>>,
    pub server: ToolServer,
    pub keeper: Keeper,
    /// Abonné dès la création : aucun évènement de l'onglet n'est perdu.
    pub events: tokio::sync::Mutex<broadcast::Receiver<AgentEvent>>,
    /// Abonné aux surfaces du hub dès la création.
    pub surfaces: tokio::sync::Mutex<broadcast::Receiver<oku_slides_mcp::McpEvent>>,
}

/// L'onglet `t-1`, le lien vers l'agent, et le journal du moteur factice.
pub struct Harness {
    pub tab: Tab,
    pub link: Link,
    pub log: PathBuf,
    pub cancel: CancellationToken,
    /// Scénario, journal et données : dernier champ, supprimé après l'onglet.
    dir: tempfile::TempDir,
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Une session sur la fixture `roadmap.json`, et la plateforme (locale `fr`).
async fn session(dir: PathBuf) -> (Session, Platform) {
    tokio::task::spawn_blocking(move || {
        let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
        settings.paths.data = Some(dir);
        // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
        settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
        settings.i18n.locale = Some("fr".into());
        let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
        let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {e}"));
        session
            .open(&PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json")))
            .unwrap_or_else(|e| panic!("ouverture : {e}"));
        (session, platform)
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante : {e}"))
}

/// Démarre l'onglet `t-1` sur l'agent local de test, moteur factice suivant `script`.
pub async fn start(script: Value) -> Harness {
    // Dossier neuf, nom aléatoire : le journal (ouvert en ajout) n'est jamais celui d'un autre test.
    let dir = tempfile::Builder::new()
        .prefix("oku-slides-local-agent-")
        .tempdir()
        .unwrap_or_else(|e| panic!("dossier de test : {e}"));
    let (script_path, log) = (dir.path().join("script.json"), dir.path().join("log.jsonl"));
    std::fs::write(&script_path, script.to_string()).unwrap_or_else(|e| panic!("scénario : {e}"));
    let launch = Launch {
        command: env!("CARGO_BIN_EXE_oku-slides-local-agent-dev").to_owned(),
        args: vec!["--local-agent".into()],
        env: vec![
            ("OKU_SLIDES_LOCAL_AGENT__ENGINE".into(), "fake".into()),
            ("OKU_SLIDES_LOCAL_AGENT__FAKE_SCRIPT".into(), script_path.to_string_lossy().into_owned()),
            ("OKU_SLIDES_LOCAL_AGENT__FAKE_LOG".into(), log.to_string_lossy().into_owned()),
            ("OKU_SLIDES_PATHS__DATA".into(), dir.path().to_string_lossy().into_owned()),
            ("OKU_SLIDES_PATHS__DECKS".into(), dir.path().join("decks").to_string_lossy().into_owned()),
        ],
        fallback_cwd: dir.path().to_owned(),
        ready_within: oku_slides_agent::READY_WITHIN,
    };
    let link = Link::new(launch);
    let cancel = CancellationToken::new();
    let (session, platform) = session(dir.path().to_owned()).await;
    let session = Arc::new(Mutex::new(session));
    let tab_session = TabSession::new("t-1", session.clone());
    let hub = SurfaceHub::new(tab_session.clone(), platform.live_localizer.clone(), Arc::new(oku_slides_mcp::NoVoice));
    let tools = Tools::new(tab_session.clone(), hub.clone(), Arc::new(NoSettings), &platform);
    let server = ToolServer::start(tools, cancel.clone()).await.unwrap_or_else(|e| panic!("serveur MCP : {e}"));
    let access = ToolsAccess { url: server.url().to_owned(), authorization: server.authorization() };
    let keeper = Keeper::spawn(tab_session.clone(), hub.clone());
    keeper.restored().await;
    let surfaces = tokio::sync::Mutex::new(hub.subscribe());
    let agent = AgentHandle::spawn(
        tab_session,
        hub.clone(),
        access,
        link.clone(),
        platform.live_localizer.clone(),
        keeper.clone(),
        platform.settings.paste.prompt_limits(),
    );
    let events = tokio::sync::Mutex::new(agent.subscribe());
    // `Storage` possède un runtime : sa dernière copie ne doit pas être lâchée dans le test (contexte async).
    std::mem::forget(platform);
    let tab = Tab { agent, hub, session, server, keeper, events, surfaces };
    Harness { tab, link, log, cancel, dir }
}

/// Attend le premier évènement qui vérifie `pred` (20 s au plus).
pub async fn until(rx: &mut broadcast::Receiver<AgentEvent>, pred: impl Fn(&AgentEvent) -> bool) -> AgentEvent {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            match rx.recv().await {
                Ok(e) if pred(&e) => return e,
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => panic!("évènements fermés"),
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("évènement attendu non reçu"))
}

/// Attend la surface du reçu (`report…`) du hub (20 s au plus) ; panique si le hub se ferme.
pub async fn receipt(rx: &mut broadcast::Receiver<oku_slides_mcp::McpEvent>) -> oku_slides_session::agent::Surface {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            match rx.recv().await {
                Ok(oku_slides_mcp::McpEvent::Surface { surface }) if surface.id.starts_with("report") => {
                    return surface;
                }
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => panic!("surfaces fermées"),
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("reçu attendu non reçu"))
}
