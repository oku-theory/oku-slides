//! Serveur de test : une session isolée, des réglages en mémoire, un client MCP.
#![allow(dead_code, reason = "chaque fichier de test n'utilise qu'une partie de l'outillage")]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use oku_slides_mcp::{Applied, McpEvent, SettingsPort, SurfaceHub, ToolServer, Tools};
use oku_slides_session::agent::{Sessions, TabSession};
use oku_slides_session::{Fonts, Platform, Session, Settings};
use rust_mcp_sdk::mcp_client::{ClientHandler, ClientRuntime, client_runtime};
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, ClientCapabilities, Implementation, InitializeRequestParams, ProtocolVersion,
};
use rust_mcp_sdk::{McpClient, RequestOptions, StreamableTransportOptions};
use serde_json::{Map, Value};
use tokio_util::sync::CancellationToken;

/// Un dossier neuf, supprimé au `drop`.
///
/// Créé atomiquement sous un nom aléatoire (t-273) : jamais celui d'un
/// lancement précédent, même quand macOS recycle le PID.
pub fn temp_dir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new().prefix(prefix).tempdir().unwrap_or_else(|e| panic!("dossier de test impossible : {e}"))
}

/// Une plateforme de test (locale `fr`) et son dossier de données, à garder
/// vivant autant qu'elle.
pub fn platform() -> (Platform, tempfile::TempDir) {
    let data = temp_dir("oku-slides-mcp-");
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    settings.paths.data = Some(data.path().to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
    settings.i18n.locale = Some("fr".into());
    (Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme de test : {e}")), data)
}

pub fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

/// Réglages en mémoire ; `write` note ce qu'on lui demande.
#[derive(Debug, Default)]
pub struct MemorySettings {
    pub values: Mutex<BTreeMap<String, Value>>,
}

impl SettingsPort for MemorySettings {
    fn read(&self) -> BTreeMap<String, Value> {
        self.values.lock().map(|v| v.clone()).unwrap_or_default()
    }
    fn write(&self, key: &str, value: &Value) -> Result<Applied, String> {
        self.values.lock().map_err(|e| e.to_string())?.insert(key.to_owned(), value.clone());
        Ok(if key.starts_with("dictation.") { Applied::Now } else { Applied::NextLaunch })
    }
}

pub struct Harness {
    pub server: ToolServer,
    pub hub: Arc<SurfaceHub>,
    /// La session, pour les assertions des tests (accès direct, hors `TabSession`).
    pub session: Arc<Mutex<Session>>,
    /// Le même onglet, tel que le voient les outils et le hub.
    pub tab: TabSession,
    pub settings: Arc<MemorySettings>,
    pub cancel: CancellationToken,
    pub localizer: Arc<oku_slides_i18n::Localizer>,
    /// Le dossier de données : dernier champ, supprimé après la session.
    data: tempfile::TempDir,
}

impl Harness {
    /// Le dossier de données de la session.
    pub fn data(&self) -> &std::path::Path {
        self.data.path()
    }
}

/// `Platform` et une session de test, ouverte sur `fixtures/roadmap.json`,
/// hors du fil async (voir `start`).
async fn prepared() -> (Platform, Session, tempfile::TempDir) {
    // `Storage` bloque sur son propre runtime (OpenDAL) : l'ouvrir depuis le
    // fil async du test (déjà dans le runtime de `#[tokio::test]`) paniquerait
    // ("Cannot start a runtime from within a runtime"), comme le fait
    // `Tools::with_session` pour les outils.
    tokio::task::spawn_blocking(|| {
        let (platform, data) = platform();
        let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session de test : {e}"));
        session.open(&fixture_path()).unwrap_or_else(|e| panic!("ouverture de la fixture : {e}"));
        (platform, session, data)
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante de préparation : {e}"))
}

/// Démarre le serveur pour un onglet dont les sessions sont fournies par `sessions`.
async fn start_with(
    (platform, data): (Platform, tempfile::TempDir),
    sessions: Arc<dyn Sessions>,
    session: Arc<Mutex<Session>>,
) -> Harness {
    let localizer = platform.localizer.clone();
    let tab = TabSession::new("t-1", sessions);
    let hub = SurfaceHub::new(tab.clone(), localizer.clone());
    let settings = Arc::new(MemorySettings::default());
    let tools = Tools::new(tab.clone(), hub.clone(), settings.clone(), &platform);
    let cancel = CancellationToken::new();
    let server = ToolServer::start(tools, cancel.clone()).await.unwrap_or_else(|e| panic!("serveur de test : {e}"));
    Harness { server, hub, session, tab, settings, cancel, localizer, data }
}

pub async fn start() -> Harness {
    let (platform, session, data) = prepared().await;
    let session = Arc::new(Mutex::new(session));
    start_with((platform, data), session.clone(), session).await
}

/// Un `Sessions` qui refuse toujours l'onglet : simule un onglet fermé côté hôte.
#[derive(Debug)]
pub struct ClosedTabs(Arc<Mutex<Session>>);

impl Sessions for ClosedTabs {
    /// Onglet toujours fermé : `f` n'est jamais appelée, l'erreur vient
    /// d'un rendu sur une slide qui n'existe pas.
    fn with(&self, _tab: &str, _f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        let mut s = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        match s.png("closed-tab-has-no-slide") {
            Ok(_) => panic!("cette slide ne devrait pas exister dans la fixture de test"),
            Err(e) => Err(e),
        }
    }
}

/// Un `Sessions` à plusieurs onglets : tout fichier est déjà ouvert dans un
/// autre onglet, et `open_tab` note l'ouverture puis rend `t-7`.
#[derive(Debug)]
pub struct OtherTabs {
    session: Arc<Mutex<Session>>,
    /// `(onglet, fichier)` de chaque `open_tab`.
    pub opened: Mutex<Vec<(String, std::path::PathBuf)>>,
}

impl Sessions for OtherTabs {
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        self.session.with(tab, f)
    }

    fn open_elsewhere(&self, _tab: &str, _path: &std::path::Path) -> bool {
        true
    }

    fn open_tab(&self, tab: &str, path: &std::path::Path) -> oku_slides_session::Result<String> {
        self.opened.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push((tab.to_owned(), path.to_owned()));
        Ok("t-7".to_owned())
    }
}

/// Un serveur parmi d'autres onglets (voir [`OtherTabs`]).
pub async fn start_other_tabs() -> (Harness, Arc<OtherTabs>) {
    let (platform, session, data) = prepared().await;
    let session = Arc::new(Mutex::new(session));
    let tabs = Arc::new(OtherTabs { session: session.clone(), opened: Mutex::default() });
    let h = start_with((platform, data), tabs.clone(), session).await;
    (h, tabs)
}

/// Un serveur dont l'onglet est fermé : tout outil sur la session échoue.
pub async fn start_closed() -> Harness {
    let (platform, session, data) = prepared().await;
    let session = Arc::new(Mutex::new(session));
    let sessions: Arc<dyn Sessions> = Arc::new(ClosedTabs(session.clone()));
    start_with((platform, data), sessions, session).await
}

struct Noop;
#[async_trait]
impl ClientHandler for Noop {}

pub async fn client(url: &str, authorization: Option<String>) -> Result<Arc<ClientRuntime>, String> {
    let mut headers = HashMap::new();
    if let Some(a) = authorization {
        headers.insert("Authorization".to_owned(), a);
    }
    let transport = StreamableTransportOptions {
        mcp_url: url.to_owned(),
        request_options: RequestOptions { custom_headers: Some(headers), ..RequestOptions::default() },
    };
    let details = InitializeRequestParams {
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "test".into(),
            version: "0.1.0".into(),
            title: None,
            description: None,
            icons: vec![],
            website_url: None,
        },
        protocol_version: ProtocolVersion::V2025_11_25.into(),
        meta: None,
    };
    let client = client_runtime::with_transport_options(details, transport, Noop, None, None, None);
    client.clone().start().await.map_err(|e| e.to_string())?;
    Ok(client)
}

pub async fn call(client: &ClientRuntime, name: &str, args: Value) -> CallToolResult {
    let args: Map<String, Value> = args.as_object().cloned().unwrap_or_default();
    client
        .request_tool_call(CallToolRequestParams::new(name).with_arguments(args))
        .await
        .unwrap_or_else(|e| panic!("appel de l'outil {name} : {e}"))
}

/// Premier bloc texte d'un résultat, en JSON si possible.
pub fn text_of(r: &CallToolResult) -> String {
    serde_json::to_value(&r.content)
        .ok()
        .and_then(|v| v.get(0).and_then(|b| b.get("text")).and_then(|t| t.as_str().map(str::to_owned)))
        .unwrap_or_default()
}

pub fn json_of(r: &CallToolResult) -> Value {
    serde_json::from_str(&text_of(r)).unwrap_or(Value::Null)
}

/// Prochaine surface poussée par le hub.
pub async fn next_surface(rx: &mut tokio::sync::broadcast::Receiver<McpEvent>) -> oku_slides_session::agent::Surface {
    loop {
        if let McpEvent::Surface { surface } = rx.recv().await.unwrap_or_else(|e| panic!("évènement du hub : {e}")) {
            return surface;
        }
    }
}
