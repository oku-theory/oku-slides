//! Serveur de test : une session isolée, des réglages en mémoire, un client MCP.
#![allow(dead_code, reason = "each test file uses only part of the harness")]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

pub mod app;
mod harness;
pub mod kinds;
pub mod other_tabs;
pub mod refusals;
pub mod workspace_harness;

use async_trait::async_trait;
use oku_slides_core::Guides;
use oku_slides_mcp::{
    Applied, Checked, Diagnosis, HostPorts, McpEvent, Origin, Prefill, SettingsPort, SurfaceAction, SurfaceHub,
    TabPort, ToolServer, Tools, TuneRefusal, VoiceApplied, VoiceError, VoiceErrorKind, VoicePort,
};
use oku_slides_session::agent::{Sessions, Surface, TabGate, TabOpen, TabSession};
use oku_slides_session::{Fonts, Platform, Session, Settings};
use oku_slides_session::{VoiceEvent, VoiceView};
use rust_mcp_sdk::mcp_client::{ClientHandler, ClientRuntime, client_runtime};
use rust_mcp_sdk::schema::{
    CallToolRequestParams, CallToolResult, ClientCapabilities, Implementation, InitializeRequestParams, ProtocolVersion,
};
use rust_mcp_sdk::{McpClient, RequestOptions, StreamableTransportOptions};
use serde_json::{Map, Value, json};
use speculoos::prelude::*;
use tokio_util::sync::CancellationToken;

#[allow(unused_imports, reason = "each test file uses only some of the harness variants")]
pub use harness::{
    ClosedTabs, Gate, GatedTabs, HiddenTab, OneFileElsewhere, start_closed, start_gated, start_hidden,
    start_with_one_file_elsewhere,
};

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
    platform_with(|_| {})
}

/// [`platform`], réglages retouchés par `tweak`.
pub fn platform_with(tweak: impl FnOnce(&mut Settings)) -> (Platform, tempfile::TempDir) {
    let data = temp_dir("oku-slides-mcp-");
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    settings.paths.data = Some(data.path().to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
    settings.i18n.locale = Some("fr".into());
    tweak(&mut settings);
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
        Ok(Applied::NextLaunch)
    }
}

/// Verrouille un mutex de test, même empoisonné.
pub fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Port de dictée de test : note chaque demande ; `refuse` et `checked` règlent ses réponses.
#[derive(Debug, Default)]
pub struct FakeVoice {
    pub applied: Mutex<Vec<(VoiceEvent, Origin)>>,
    pub refuse: Mutex<Option<VoiceErrorKind>>,
    pub checked: Mutex<Checked>,
    pub answer: Mutex<Option<VoiceApplied>>,
    pub view: Mutex<VoiceView>,
    /// What `diagnosis` answers.
    pub diagnosis: Mutex<Diagnosis>,
    /// How many times the OS voices were asked to be read again.
    pub voices_refreshed: Mutex<u32>,
}

impl FakeVoice {
    /// Un port de test sur la vue de référence de la session.
    pub fn new() -> Self {
        Self { view: Mutex::new(oku_slides_session::voice_fixture::voice_view_fixture()), ..Self::default() }
    }
}

impl VoicePort for FakeVoice {
    fn view(&self) -> VoiceView {
        lock(&self.view).clone()
    }

    fn check(&self, _: &VoiceEvent, _: &Origin) -> Result<Checked, VoiceError> {
        match *lock(&self.refuse) {
            Some(kind) => Err(VoiceError::new(kind)),
            None => Ok(lock(&self.checked).clone()),
        }
    }

    fn apply(&self, event: VoiceEvent, origin: Origin) -> Result<VoiceApplied, VoiceError> {
        if let Some(kind) = *lock(&self.refuse) {
            return Err(VoiceError::new(kind));
        }
        lock(&self.applied).push((event, origin));
        Ok(lock(&self.answer).unwrap_or(VoiceApplied::Now))
    }

    fn diagnosis(&self) -> Diagnosis {
        lock(&self.diagnosis).clone()
    }

    fn refresh_voices(&self) {
        *lock(&self.voices_refreshed) += 1;
    }
}

/// Tab port of the tests: records each prefill, page and tune.
#[derive(Debug, Default)]
pub struct RecordingTabs {
    /// `(tab, text)` of each prefill, in order.
    pub calls: Mutex<Vec<(String, String)>>,
    /// The answer to the next prefills; `None`: `Set`.
    pub answer: Mutex<Option<Prefill>>,
    /// `(tab, slide)` of each page, in order.
    pub pages: Mutex<Vec<(String, String)>>,
    /// True: no page shows the tab, every page is refused.
    pub no_page: Mutex<bool>,
    /// The tabs asked to fine-tune, in order.
    pub tunes: Mutex<Vec<String>>,
}

impl RecordingTabs {
    /// The prefills so far.
    pub fn calls(&self) -> Vec<(String, String)> {
        lock(&self.calls).clone()
    }
    /// The pages so far, `(tab, slide)`.
    pub fn pages(&self) -> Vec<(String, String)> {
        lock(&self.pages).clone()
    }
    /// The tabs asked to fine-tune so far.
    pub fn tunes(&self) -> Vec<String> {
        lock(&self.tunes).clone()
    }
}

impl TabPort for RecordingTabs {
    fn prefill(&self, tab: &str, text: String) -> Prefill {
        lock(&self.calls).push((tab.to_owned(), text));
        lock(&self.answer).unwrap_or(Prefill::Set)
    }

    fn page(&self, tab: &str, slide: String) -> bool {
        lock(&self.pages).push((tab.to_owned(), slide));
        !*lock(&self.no_page)
    }

    fn tune(&self, tab: &str) -> Result<(), TuneRefusal> {
        lock(&self.tunes).push(tab.to_owned());
        Ok(())
    }
}

pub struct Harness {
    pub server: ToolServer,
    pub hub: Arc<SurfaceHub>,
    /// Le port de la dictée et de l'agent que voit le hub.
    pub voice: Arc<FakeVoice>,
    /// La session, pour les assertions des tests (accès direct, hors `TabSession`).
    pub session: Arc<Mutex<Session>>,
    /// Le même onglet, tel que le voient les outils et le hub.
    pub tab: TabSession,
    pub settings: Arc<MemorySettings>,
    /// The tab port `open_pack`, `open_theme`, `help` and `import` act on other tabs through.
    pub tabs: Arc<RecordingTabs>,
    pub cancel: CancellationToken,
    /// The app port the tools ask for updates (unused when started on the dev server's).
    pub app: Arc<app::FakeApp>,
    pub localizer: Arc<oku_slides_i18n::Localizer>,
    /// La langue courante de l'interface, à changer à chaud.
    pub live: oku_slides_i18n::LiveLocalizer,
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
    prepared_with(|_| {}).await
}

/// [`prepared`], réglages retouchés par `tweak`.
async fn prepared_with(tweak: impl FnOnce(&mut Settings) + Send + 'static) -> (Platform, Session, tempfile::TempDir) {
    // `Storage` bloque sur son propre runtime (OpenDAL) : l'ouvrir depuis le
    // fil async du test (déjà dans le runtime de `#[tokio::test]`) paniquerait
    // ("Cannot start a runtime from within a runtime"), comme le fait
    // `Tools::with_session` pour les outils.
    tokio::task::spawn_blocking(|| {
        let (platform, data) = platform_with(tweak);
        let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session de test : {e}"));
        session.open(&fixture_path()).unwrap_or_else(|e| panic!("ouverture de la fixture : {e}"));
        (platform, session, data)
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante de préparation : {e}"))
}

/// A fresh session on its own data dir, turned into the theme tab of `theme`.
///
/// Blocking (`Storage`): call it from `spawn_blocking` or `block_in_place`.
pub fn theme_tab_session(theme: uuid::Uuid) -> (Platform, Session, tempfile::TempDir) {
    let (platform, data) = platform_with(|_| {});
    let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("test session: {e}"));
    session.open_theme_here(theme).unwrap_or_else(|e| panic!("theme tab: {e}"));
    (platform, session, data)
}

/// [`start`] on a theme tab of `theme` instead of the fixture deck.
pub async fn start_theme_tab(theme: uuid::Uuid) -> Harness {
    let (platform, session, data) = tokio::task::spawn_blocking(move || theme_tab_session(theme))
        .await
        .unwrap_or_else(|e| panic!("blocking setup task: {e}"));
    let session = Arc::new(Mutex::new(session));
    start_with((platform, data), session.clone(), session, "t-1").await
}

/// [`start`] on a pack tab of library pack `pack` instead of the fixture deck.
pub async fn start_pack_tab(pack: uuid::Uuid) -> Harness {
    let (platform, session, data) = tokio::task::spawn_blocking(move || {
        let (platform, data) = platform_with(|_| {});
        let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("test session: {e}"));
        session.open_pack_tab_here(pack, None).unwrap_or_else(|e| panic!("pack tab: {}", e.one_line()));
        (platform, session, data)
    })
    .await
    .unwrap_or_else(|e| panic!("blocking setup task: {e}"));
    let session = Arc::new(Mutex::new(session));
    start_with((platform, data), session.clone(), session, "t-1").await
}

/// Starts the server for a tab whose sessions `sessions` provides, in `serve`'s host.
///
/// The tools are those of the tab's kind ([`kinds::kind_of`]).
async fn start_on(
    (platform, data): (Platform, tempfile::TempDir),
    sessions: Arc<dyn Sessions>,
    session: Arc<Mutex<Session>>,
    tab: &str,
    serve: Host,
    workspace: Arc<dyn oku_slides_session::agent::AgentWorkspace>,
) -> Harness {
    let localizer = platform.localizer.clone();
    let tab = TabSession::new(tab, sessions);
    let live = platform.live_localizer.clone();
    let voice = Arc::new(FakeVoice::new());
    let hub = SurfaceHub::new(tab.clone(), live.clone(), voice.clone());
    let settings = Arc::new(MemorySettings::default());
    let tabs = Arc::new(RecordingTabs::default());
    let kind = kinds::kind_of(&tab, &session).await;
    let app = Arc::new(app::FakeApp::default());
    let port: Arc<dyn oku_slides_session::agent::AppPort> = match serve {
        Host::App => app.clone(),
        Host::Serve => Arc::new(oku_slides_session::agent::UnavailableApp),
    };
    let tools = Tools::new(
        tab.clone(),
        hub.clone(),
        HostPorts { settings: settings.clone(), tabs: tabs.clone(), workspace, app: port },
        &platform,
        kind,
    );
    let cancel = CancellationToken::new();
    let server = ToolServer::start(tools, cancel.clone()).await.unwrap_or_else(|e| panic!("serveur de test : {e}"));
    Harness { server, hub, voice, session, tab, settings, tabs, cancel, app, localizer, live, data }
}

/// Which host the tools run in: the app (a [`app::FakeApp`]) or the dev server.
#[derive(Debug, Clone, Copy)]
enum Host {
    App,
    Serve,
}

/// [`start`] as the dev server runs it: no updater, no quit (`UnavailableApp`).
pub async fn start_on_serve() -> Harness {
    let (platform, session, data) = prepared().await;
    let session = Arc::new(Mutex::new(session));
    start_on((platform, data), session.clone(), session, "t-1", Host::Serve, no_workspace()).await
}

/// Starts the server for a tab whose sessions `sessions` provides, in the app.
async fn start_with(
    host: (Platform, tempfile::TempDir),
    sessions: Arc<dyn Sessions>,
    session: Arc<Mutex<Session>>,
    tab: &str,
) -> Harness {
    start_on(host, sessions, session, tab, Host::App, no_workspace()).await
}

/// [`start_with`] whose tools change tabs and windows through `workspace`.
async fn start_with_workspace(
    host: (Platform, tempfile::TempDir),
    sessions: Arc<dyn Sessions>,
    session: Arc<Mutex<Session>>,
    tab: &str,
    workspace: Arc<dyn oku_slides_session::agent::AgentWorkspace>,
) -> Harness {
    start_on(host, sessions, session, tab, Host::App, workspace).await
}

/// [`start`] whose tools change tabs and windows through `workspace`, a test's fake.
pub async fn start_with_port(workspace: Arc<dyn oku_slides_session::agent::AgentWorkspace>) -> Harness {
    let (platform, session, data) = prepared().await;
    let session = Arc::new(Mutex::new(session));
    start_on((platform, data), session.clone(), session, "t-1", Host::App, workspace).await
}

/// The workspace port of a host with no tab bar: every change refused.
fn no_workspace() -> Arc<dyn oku_slides_session::agent::AgentWorkspace> {
    Arc::new(oku_slides_session::agent::NoWorkspace)
}

pub async fn start() -> Harness {
    start_with_settings(|_| {}).await
}

/// [`start`], réglages retouchés par `tweak` (limites de rendu…).
pub async fn start_with_settings(tweak: impl FnOnce(&mut Settings) + Send + 'static) -> Harness {
    let (platform, session, data) = prepared_with(tweak).await;
    let session = Arc::new(Mutex::new(session));
    start_with((platform, data), session.clone(), session, "t-1").await
}

/// Le `.pptx` de la fixture `fixtures/import/<name>` copié ou zippé dans `dir` ; son chemin.
///
/// Un fichier `<name>.pptx` est copié tel quel ; un dossier `<name>` est zippé.
pub fn pptx(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    use std::io::Write;
    /// The files under `dir`, at any depth, without `.DS_Store`.
    fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let p = e.unwrap_or_else(|e| panic!("{e}")).path();
            if p.file_name().is_some_and(|n| n == ".DS_Store") {
                continue;
            }
            if p.is_dir() { files(&p, out) } else { out.push(p) }
        }
    }
    let fixtures = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import"));
    let file = fixtures.join(format!("{name}.pptx"));
    if file.is_file() {
        let path = dir.join(format!("{name}.pptx"));
        std::fs::copy(&file, &path).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        return path;
    }
    let root = fixtures.join(name);
    let mut paths = Vec::new();
    files(&root, &mut paths);
    // `[Content_Types].xml` en tête, comme l'écrit PowerPoint.
    paths.sort_by_key(|p| (p.file_name() != Some("[Content_Types].xml".as_ref()), p.clone()));
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for p in paths {
        let rel = p.strip_prefix(&root).unwrap_or_else(|e| panic!("{e}")).to_string_lossy().replace('\\', "/");
        w.start_file(rel, zip::write::SimpleFileOptions::default()).unwrap_or_else(|e| panic!("zip : {e}"));
        w.write_all(&std::fs::read(&p).unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("zip : {e}"));
    }
    let bytes = w.finish().unwrap_or_else(|e| panic!("zip : {e}")).into_inner();
    let path = dir.join(format!("{name}.pptx"));
    std::fs::write(&path, bytes).unwrap_or_else(|e| panic!("{e}"));
    path
}

/// La cible hi-fi des tests, témoin office2pdf.
pub fn hifi() -> oku_slides_session::import::ImportTarget {
    oku_slides_session::import::ImportTarget::Hifi {
        witness: Some(oku_slides_platform::config::ImportWitness::Office2pdf),
    }
}

/// Un serveur sur un onglet tout juste importé de la fixture `fixtures/import/<name>` vers `target`,
/// réglages retouchés par `tweak` ; le témoin y rend pour de vrai.
pub async fn start_imported(
    name: &'static str,
    target: oku_slides_session::import::ImportTarget,
    tweak: impl FnOnce(&mut Settings) + Send + 'static,
) -> Harness {
    let (platform, session, data) = tokio::task::spawn_blocking(move || {
        use oku_slides_session::import::{ImportRequest, PackLookup, import};
        let (platform, data) = platform_with(tweak);
        let request =
            ImportRequest { pptx: pptx(data.path(), name), target, system_fonts: false, packs: PackLookup::Library };
        let imported = import(&platform, &Fonts::default(), &request).unwrap_or_else(|e| panic!("import : {e}"));
        let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session de test : {e}"));
        session.load_import(imported, name).unwrap_or_else(|e| panic!("chargement de l'import : {e}"));
        (platform, session, data)
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante de l'import : {e}"));
    let session = Arc::new(Mutex::new(session));
    start_with((platform, data), session.clone(), session, "t-1").await
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

/// Attente bornée d'un évènement du hub.
pub const EVENT_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// Le premier évènement du hub que `pick` retient, dans `EVENT_WAIT` ; sinon le test échoue
/// (au lieu d'attendre sans fin un évènement qui ne vient pas).
pub async fn wait_event<T>(
    rx: &mut tokio::sync::broadcast::Receiver<McpEvent>,
    mut pick: impl FnMut(McpEvent) -> Option<T>,
) -> T {
    tokio::time::timeout(EVENT_WAIT, async {
        loop {
            let event = rx.recv().await.unwrap_or_else(|e| panic!("évènement du hub : {e}"));
            if let Some(found) = pick(event) {
                return found;
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("évènement du hub attendu non reçu en {EVENT_WAIT:?}"))
}

/// Prochaine surface poussée par le hub.
pub async fn next_surface(rx: &mut tokio::sync::broadcast::Receiver<McpEvent>) -> oku_slides_session::agent::Surface {
    wait_event(rx, |e| match e {
        McpEvent::Surface { surface, .. } => Some(surface),
        _ => None,
    })
    .await
}

/// Un client MCP connecté au serveur de `h`.
pub async fn server(h: &Harness) -> Arc<ClientRuntime> {
    client(h.server.url(), Some(h.server.authorization())).await.unwrap_or_else(|e| panic!("client de test : {e}"))
}

/// Un appel qui ne doit ouvrir aucune fenêtre : échoue au lieu d'attendre.
pub async fn call_within(c: &Arc<ClientRuntime>, tool: &str, args: Value) -> CallToolResult {
    tokio::time::timeout(EVENT_WAIT, call(c, tool, args))
        .await
        .unwrap_or_else(|_| panic!("{tool} attend une réponse : une confirmation a été ouverte"))
}

/// Le schéma d'entrée de l'outil `name`, tel que la liste des outils le montre.
pub async fn tool_schema(c: &Arc<ClientRuntime>, name: &str) -> Value {
    let tools = c.request_tool_list(None).await.unwrap_or_else(|e| panic!("liste des outils : {e}")).tools;
    let tool = tools.into_iter().find(|t| t.name == name).unwrap_or_else(|| panic!("outil {name} absent"));
    serde_json::to_value(tool.input_schema).unwrap_or_else(|e| panic!("schéma de {name} : {e}"))
}

/// La description de l'outil `name`, telle que la liste des outils la montre.
pub async fn tool_description(c: &Arc<ClientRuntime>, name: &str) -> String {
    let tools = c.request_tool_list(None).await.unwrap_or_else(|e| panic!("liste des outils : {e}")).tools;
    let tool = tools.into_iter().find(|t| t.name == name).unwrap_or_else(|| panic!("outil {name} absent"));
    tool.description.unwrap_or_default()
}

/// A confirmation's text and its `danger` flag (`forms::confirm`).
pub fn confirmation_of(surface: &Surface) -> (String, bool) {
    let text = component_of(surface, "message")["text"].as_str().unwrap_or_default().to_owned();
    (text, component_of(surface, "yes")["danger"] == Value::Bool(true))
}

/// The label of a confirmation's yes button, the verb to say for a danger one.
pub fn confirm_button_label(surface: &Surface) -> String {
    component_of(surface, "yes")["label"].as_str().unwrap_or_default().to_owned()
}

/// Component `id` of `surface`, as JSON; `Null` when absent.
fn component_of(surface: &Surface, id: &str) -> Value {
    let json = serde_json::to_value(surface).unwrap_or_else(|e| panic!("surface : {e}"));
    json["messages"]
        .as_array()
        .and_then(|m| m.iter().find_map(|m| m["updateComponents"]["components"].as_array().cloned()))
        .and_then(|c| c.into_iter().find(|c| c["id"] == id))
        .unwrap_or_default()
}

/// Répond à la confirmation `surface` : oui ou non.
pub fn answer(h: &Harness, surface: &str, confirmed: bool) {
    let mut context = Map::new();
    context.insert("confirmed".into(), Value::Bool(confirmed));
    h.hub
        .act(SurfaceAction { surface: surface.into(), name: "confirm".into(), context })
        .unwrap_or_else(|e| panic!("réponse à la surface : {e}"));
}

/// Appelle `tool`, vérifie que sa confirmation s'ouvre en fenêtre, y répond ; rend le résultat.
pub async fn guarded(h: &Harness, tool: &'static str, args: Value, confirmed: bool) -> CallToolResult {
    let mut rx = h.hub.subscribe();
    let c = server(h).await;
    let pending = tokio::spawn(async move { call(&c, tool, args).await });
    let surface = next_surface(&mut rx).await;
    let json = serde_json::to_value(&surface).unwrap_or_else(|e| panic!("sérialisation de la surface : {e}"));
    speculoos::assert_that!(json["place"].as_str()).is_equal_to(Some("modal"));
    answer(h, &surface.id, confirmed);
    pending.await.unwrap_or_else(|e| panic!("tâche de l'outil : {e}"))
}

/// Comme [`guarded`], et rend le texte de la confirmation et son drapeau `danger`.
pub async fn guarded_message(
    h: &Harness,
    tool: &'static str,
    args: Value,
    confirmed: bool,
) -> (String, bool, CallToolResult) {
    guarded_message_after(h, tool, args, confirmed, || ()).await
}

/// Comme [`guarded_message`], et lance `while_open` une fois la fenêtre ouverte, avant la réponse.
pub async fn guarded_message_after(
    h: &Harness,
    tool: &'static str,
    args: Value,
    confirmed: bool,
    while_open: impl FnOnce(),
) -> (String, bool, CallToolResult) {
    let mut rx = h.hub.subscribe();
    let c = server(h).await;
    let pending = tokio::spawn(async move { call(&c, tool, args).await });
    let surface = next_surface(&mut rx).await;
    let (text, danger) = confirmation_of(&surface);
    while_open();
    answer(h, &surface.id, confirmed);
    (text, danger, pending.await.unwrap_or_else(|e| panic!("tâche de l'outil : {e}")))
}

/// Une action de l'utilisateur sur une surface.
pub fn action(surface: &str, name: &str, context: &Value) -> oku_slides_mcp::SurfaceAction {
    oku_slides_mcp::SurfaceAction {
        surface: surface.into(),
        name: name.into(),
        context: context.as_object().cloned().unwrap_or_else(Map::new),
    }
}

/// Identifiants des surfaces que le hub tient pour ouvertes.
pub fn open_ids(h: &Harness) -> Vec<String> {
    h.hub.surfaces().into_iter().map(|s| s.id).collect()
}

/// Les composants d'un `render` minimal : un bouton qui suggère « Encore ».
pub fn render_args() -> Value {
    json!({ "components": [
        { "id": "root", "component": "Button", "label": "Encore", "action": { "event": { "name": "suggest", "context": { "prompt": "Encore" } } } }
    ] })
}

/// L'espace de noms du thème intégré nommé `name` (sans casse).
#[allow(dead_code, reason = "not used by every test binary")]
pub fn theme_ref(name: &str) -> Option<oku_slides_core::Namespace> {
    use oku_slides_session::{LibraryKind, Themes};
    Themes::builtins().iter().find(|b| b.manifest.name.eq_ignore_ascii_case(name)).map(|b| b.manifest.namespace.clone())
}

/// Le nom court (`papier`, `pro`) du thème intégré d'espace de noms `ns`.
#[allow(dead_code, reason = "not used by every test binary")]
pub fn theme_name(ns: &oku_slides_core::Namespace) -> Option<&'static str> {
    use oku_slides_session::{PAPIER_NAMESPACE, PRO_NAMESPACE};
    [PAPIER_NAMESPACE, PRO_NAMESPACE].into_iter().find(|n| ns.as_str() == *n)
}

/// [`start`], la bibliothèque tenant d'abord un pack de widgets vide sous `namespace`, qui requiert `base`.
pub async fn start_with_library_widget_pack(namespace: &'static str) -> Harness {
    let (platform, session, data) = tokio::task::spawn_blocking(move || {
        use oku_slides_core::{Namespace, PackKind, PackScope};
        let (platform, data) = platform_with(|_| {});
        let ns = |s: &str| Namespace::parse(s).unwrap_or_else(|| panic!("espace de noms invalide : {s}"));
        let manifest = oku_slides_session::PackManifest {
            id: uuid::Uuid::from_u128(0xac3e_0000_0000_4000_8000_0000_0000_0001),
            name: namespace.to_owned(),
            namespace: ns(namespace),
            requires: vec![ns("base")],
            description: String::new(),
            parent: None,
            created: "2026-10-01T10:00:00Z".into(),
            scope: PackScope::Library,
            profiles: Vec::new(),
            examples: BTreeMap::new(),
            styles: BTreeMap::new(),
            tokens: Vec::new(),
            order: Vec::new(),
            guides: Guides::default(),
        };
        let files = manifest.to_files(PackKind::Widgets, BTreeMap::new()).unwrap_or_else(|e| panic!("manifeste : {e}"));
        oku_slides_session::PackLibrary::user(&platform)
            .add_version(manifest.id, &files)
            .unwrap_or_else(|e| panic!("pack de la bibliothèque : {e}"));
        let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session de test : {e}"));
        session.open(&fixture_path()).unwrap_or_else(|e| panic!("ouverture de la fixture : {e}"));
        (platform, session, data)
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante de préparation : {e}"));
    let session = Arc::new(Mutex::new(session));
    start_with((platform, data), session.clone(), session, "t-1").await
}

/// La dernière surface d'id `id` déjà reçue sur `rx` ; le canal est vidé.
pub fn last_surface(
    rx: &mut tokio::sync::broadcast::Receiver<McpEvent>,
    id: &str,
) -> oku_slides_session::agent::Surface {
    let mut last = None;
    while let Ok(event) = rx.try_recv() {
        if let McpEvent::Surface { surface, .. } = event
            && surface.id == id
        {
            last = Some(surface);
        }
    }
    last.unwrap_or_else(|| panic!("aucune surface {id} reçue"))
}

/// Les ids des composants d'une surface, dans l'ordre de ses messages.
pub fn component_ids(surface: &oku_slides_session::agent::Surface) -> Vec<String> {
    surface
        .messages
        .iter()
        .filter_map(|m| match &m.body {
            oku_slides_a2ui::ServerBody::UpdateComponents(u) => Some(u.components.iter().map(|c| c.id.clone())),
            _ => None,
        })
        .flatten()
        .collect()
}
