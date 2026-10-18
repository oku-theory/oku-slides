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
use serde_json::{Map, Value, json};
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

/// Démarre le serveur pour un onglet dont les sessions sont fournies par `sessions`.
async fn start_with(
    (platform, data): (Platform, tempfile::TempDir),
    sessions: Arc<dyn Sessions>,
    session: Arc<Mutex<Session>>,
) -> Harness {
    let localizer = platform.localizer.clone();
    let tab = TabSession::new("t-1", sessions);
    let live = platform.live_localizer.clone();
    let hub = SurfaceHub::new(tab.clone(), live.clone());
    let settings = Arc::new(MemorySettings::default());
    let tools = Tools::new(tab.clone(), hub.clone(), settings.clone(), &platform);
    let cancel = CancellationToken::new();
    let server = ToolServer::start(tools, cancel.clone()).await.unwrap_or_else(|e| panic!("serveur de test : {e}"));
    Harness { server, hub, session, tab, settings, cancel, localizer, live, data }
}

pub async fn start() -> Harness {
    start_with_settings(|_| {}).await
}

/// [`start`], réglages retouchés par `tweak` (limites de rendu…).
pub async fn start_with_settings(tweak: impl FnOnce(&mut Settings) + Send + 'static) -> Harness {
    let (platform, session, data) = prepared_with(tweak).await;
    let session = Arc::new(Mutex::new(session));
    start_with((platform, data), session.clone(), session).await
}

/// Le `.pptx` de la fixture `fixtures/import/<name>` copié ou zippé dans `dir` ; son chemin.
///
/// Un fichier `<name>.pptx` est copié tel quel ; un dossier `<name>` est zippé.
pub fn pptx(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    use std::io::Write;
    let fixtures = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import"));
    let file = fixtures.join(format!("{name}.pptx"));
    if file.is_file() {
        let path = dir.join(format!("{name}.pptx"));
        std::fs::copy(&file, &path).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        return path;
    }
    fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let p = e.unwrap_or_else(|e| panic!("{e}")).path();
            if p.file_name().is_some_and(|n| n == ".DS_Store") {
                continue;
            }
            if p.is_dir() { files(&p, out) } else { out.push(p) }
        }
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
    start_with((platform, data), session.clone(), session).await
}

/// Un `Sessions` qui refuse toujours l'onglet : un vrai espace de travail,
/// interrogé sur un onglet qu'il n'a pas (fermé côté hôte).
#[derive(Debug)]
pub struct ClosedTabs(Mutex<oku_slides_session::Workspace>);

impl Sessions for ClosedTabs {
    /// Onglet toujours fermé : `f` n'est jamais appelée.
    fn with(&self, _tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        self.0.with("onglet-ferme", f)
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

/// Un `Sessions` qui retient le prochain accès à la session jusqu'à ce que le test le relâche
/// (voir [`Gate`]) : de quoi faire durer un appel d'outil pendant que les tours changent.
#[derive(Debug)]
pub struct GatedTabs {
    session: Arc<Mutex<Session>>,
    hold: Mutex<Option<Hold>>,
}

#[derive(Debug)]
struct Hold {
    entered: tokio::sync::oneshot::Sender<()>,
    release: std::sync::mpsc::Receiver<()>,
}

/// Côté test d'un accès retenu : `entered` l'attend, `release` le laisse passer.
#[derive(Debug)]
pub struct Gate {
    entered: tokio::sync::oneshot::Receiver<()>,
    release: std::sync::mpsc::Sender<()>,
}

impl Gate {
    /// Attend que l'accès retenu soit commencé (l'outil est donc déjà appelé).
    pub async fn entered(&mut self) {
        tokio::time::timeout(std::time::Duration::from_secs(10), &mut self.entered)
            .await
            .unwrap_or_else(|_| panic!("aucun accès à la session"))
            .unwrap_or_else(|e| panic!("accès retenu perdu : {e}"));
    }

    /// Laisse passer l'accès retenu.
    pub fn release(self) {
        let _ = self.release.send(());
    }
}

impl GatedTabs {
    /// Retient le prochain accès à la session.
    pub fn hold_next(&self) -> Gate {
        let (entered_tx, entered) = tokio::sync::oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        *self.hold.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some(Hold { entered: entered_tx, release: release_rx });
        Gate { entered, release }
    }
}

impl Sessions for GatedTabs {
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        let hold = self.hold.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take();
        if let Some(hold) = hold {
            let _ = hold.entered.send(());
            let _ = hold.release.recv();
        }
        self.session.with(tab, f)
    }
}

/// Un serveur dont les accès à la session peuvent être retenus (voir [`GatedTabs`]).
pub async fn start_gated() -> (Harness, Arc<GatedTabs>) {
    let (platform, session, data) = prepared().await;
    let session = Arc::new(Mutex::new(session));
    let tabs = Arc::new(GatedTabs { session: session.clone(), hold: Mutex::default() });
    let h = start_with((platform, data), tabs.clone(), session).await;
    (h, tabs)
}

/// Un serveur dont l'onglet est fermé : tout outil sur la session échoue.
pub async fn start_closed() -> Harness {
    let (platform, session, data) = prepared().await;
    let (platform, workspace) = tokio::task::spawn_blocking(move || {
        let workspace = oku_slides_session::Workspace::new(Fonts::default(), &platform, None)
            .unwrap_or_else(|e| panic!("espace de travail de test : {e}"));
        (platform, workspace)
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante de l'espace de travail : {e}"));
    let session = Arc::new(Mutex::new(session));
    let sessions: Arc<dyn Sessions> = Arc::new(ClosedTabs(Mutex::new(workspace)));
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
        McpEvent::Surface { surface } => Some(surface),
        _ => None,
    })
    .await
}

/// Une action de l'utilisateur sur une surface.
pub fn action(surface: &str, name: &str, context: Value) -> oku_slides_mcp::SurfaceAction {
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

/// La référence de la version intégrée du thème nommé `name` (sans casse).
#[allow(dead_code, reason = "pas utilisée par tous les binaires de test")]
pub fn theme_ref(name: &str) -> Option<oku_slides_core::ThemeRef> {
    use oku_slides_session::{LibraryKind, Themes};
    Themes::builtins()
        .iter()
        .find(|b| b.manifest.name.eq_ignore_ascii_case(name))
        .map(|b| oku_slides_core::ThemeRef { id: b.manifest.id, version: b.version.clone() })
}

/// Le nom court (`papier`, `pro`) du thème intégré de même id que `r`.
#[allow(dead_code, reason = "pas utilisée par tous les binaires de test")]
pub fn theme_name(r: &oku_slides_core::ThemeRef) -> Option<&'static str> {
    use oku_slides_session::{PAPIER_THEME, PRO_THEME};
    [(PAPIER_THEME, "papier"), (PRO_THEME, "pro")].into_iter().find(|(id, _)| *id == r.id).map(|(_, n)| n)
}
