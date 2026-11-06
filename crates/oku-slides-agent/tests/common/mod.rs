//! Pile de test : sessions isolées (une par onglet), outils MCP réels par
//! onglet, un faux agent partagé.
#![allow(dead_code, reason = "chaque fichier de test n'utilise qu'une partie de l'outillage")]

pub mod app;
mod stack;

#[allow(unused_imports, reason = "each test file uses only some of the stack builders")]
pub use stack::{
    TestStack, stack_locked, stack_resumed, stack_resumed_from, stack_with_builtin, stack_with_command,
    stack_with_fallback, stack_with_sink, stack_with_sink_and_log, stack_with_speaker, stack_with_two_agents,
};

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oku_slides_agent::{AgentEvent, AgentHandle, AgentState, Keeper, Launch, Link, ToolsAccess, agent_fingerprint};
use oku_slides_mcp::{Applied, SettingsPort, SurfaceHub, ToolServer, Tools};
use oku_slides_platform::config::{AgentPreset, Builtin};
use oku_slides_session::agent::turn_view::{TurnState, TurnView, turn_surface};
use oku_slides_session::agent::{AgentWorkspace, FocusSource, FocusedTab, Surface, TabSession};
use oku_slides_session::{Fonts, Platform, Session, Settings};
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

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
    pub name: String,
    pub agent: AgentHandle,
    pub hub: Arc<SurfaceHub>,
    pub session: Arc<Mutex<Session>>,
    pub server: ToolServer,
    /// Le gardien de l'onglet : seul passage vers sa conversation.
    pub keeper: Keeper,
    /// Abonné dès la création : aucun évènement de l'onglet n'est perdu.
    pub events: tokio::sync::Mutex<broadcast::Receiver<AgentEvent>>,
}

pub struct Harness {
    pub tabs: Vec<Tab>,
    pub link: Link,
    pub log: PathBuf,
    pub cancel: CancellationToken,
    /// Scénario, journal et données : dernier champ, supprimé après les onglets.
    dir: tempfile::TempDir,
}

impl Harness {
    /// The data folder: script, log, sessions' data and resume folders.
    pub fn data(&self) -> &Path {
        self.dir.path()
    }

    /// The working folder of `tab`'s current conversation: `work/<uuid>` under its resume folder.
    ///
    /// Minted by the session if the conversation has none yet, as the agent's setup would.
    pub fn work_dir(&self, tab: &str) -> PathBuf {
        let mut session = self.session(tab).lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let work = session.agent_work_dir().unwrap_or_else(|e| panic!("work dir of {tab}: {}", e.one_line()));
        let saved = session.conversation().and_then(|c| c.work).unwrap_or_else(|| panic!("no saved work of {tab}"));
        let path = tab_resume_dir(self.dir.path(), tab).join("work").join(saved.to_string());
        assert_eq!(work.path(), path, "the session's folder is the saved conversation's");
        path
    }

    /// The folder the agent process runs in.
    pub fn process_dir(&self) -> PathBuf {
        self.dir.path().join(PROCESS_DIR)
    }

    pub fn tab(&self, name: &str) -> &Tab {
        self.tabs.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("onglet inconnu : {name}"))
    }

    pub fn session(&self, name: &str) -> &Arc<Mutex<Session>> {
        &self.tab(name).session
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// The agent process folder's name in a test's data folder.
const PROCESS_DIR: &str = "agent-process";

/// Un dossier neuf, supprimé au `drop` (t-273).
///
/// Créé atomiquement sous un nom aléatoire : jamais celui d'un test
/// précédent, dont le journal (ouvert en ajout) serait sinon relu.
fn test_dir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new().prefix(prefix).tempdir().unwrap_or_else(|e| panic!("dossier de test : {e}"))
}

/// The resume folder of tab `tab` under `data`, as the workspace names it.
pub fn tab_resume_dir(data: &Path, tab: &str) -> PathBuf {
    data.join("tabs").join(tab)
}

/// One storage for every test launch, never dropped: dropping its runtime in an async test panics.
pub fn launch_storage() -> oku_slides_platform::storage::Storage {
    static STORAGE: std::sync::LazyLock<oku_slides_platform::storage::Storage> = std::sync::LazyLock::new(|| {
        oku_slides_platform::storage::Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"))
    });
    STORAGE.clone()
}

/// What a test tab's session holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Deck {
    /// The `roadmap.json` fixture, saved, with a resume folder.
    Fixture,
    /// The deck file at this path, with a resume folder.
    File(PathBuf),
    /// A new deck never saved, with a resume folder.
    Unsaved,
    /// The fixture, without a resume folder: no working folder for the agent.
    NoResume,
}

/// The `roadmap.json` fixture.
pub fn roadmap() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

/// Une session de l'onglet `tab` selon `deck`, et la plateforme (locale `fr`).
async fn session(dir: PathBuf, tab: &str, deck: Deck) -> (Session, Platform) {
    let resume = tab_resume_dir(&dir, tab);
    tokio::task::spawn_blocking(move || {
        let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
        settings.paths.data = Some(dir);
        // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
        settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
        settings.i18n.locale = Some("fr".into());
        let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
        let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {e}"));
        let file = match &deck {
            Deck::Fixture | Deck::NoResume => Some(roadmap()),
            Deck::File(path) => Some(path.clone()),
            Deck::Unsaved => None,
        };
        if let Some(file) = file {
            session.open(&file).unwrap_or_else(|e| panic!("ouverture : {e}"));
        }
        if deck != Deck::NoResume {
            assert!(session.resume(resume), "reprise de l'onglet");
        }
        (session, platform)
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante : {e}"))
}

/// Démarre un onglet par nom de `tabs`, sur un seul faux agent (ou `command`) suivant `script`.
pub async fn start_tabs_with(script: Value, tabs: &[&str], command: Option<&str>) -> Harness {
    start_tabs_ready_within(script, tabs, command, oku_slides_agent::READY_WITHIN, Deck::Fixture).await
}

/// Un onglet `t-1` dont l'agent doit être prêt dans `ready_within`.
pub async fn start_ready_within(script: Value, ready_within: Duration) -> Harness {
    start_tabs_ready_within(script, &["t-1"], None, ready_within, Deck::Fixture).await
}

/// One tab `t-1` whose session holds `deck`, on the fake agent following `script`.
pub async fn start_deck(script: Value, deck: Deck) -> Harness {
    start_tabs_ready_within(script, &["t-1"], None, oku_slides_agent::READY_WITHIN, deck).await
}

/// Le scénario du faux agent : réécrit, il vaut pour le lancement suivant.
pub fn script_path(h: &Harness) -> PathBuf {
    h.log.with_file_name("script.json")
}

async fn start_tabs_ready_within(
    script: Value,
    tabs: &[&str],
    command: Option<&str>,
    ready_within: Duration,
    deck: Deck,
) -> Harness {
    let dir = test_dir("oku-slides-agent-");
    let (script_path, log) = (dir.path().join("script.json"), dir.path().join("log.jsonl"));
    std::fs::write(&script_path, script.to_string()).unwrap_or_else(|e| panic!("scénario : {e}"));
    let launch = Launch {
        command: command.unwrap_or(env!("CARGO_BIN_EXE_oku-slides-fake-agent")).to_owned(),
        args: vec![
            "--script".into(),
            script_path.to_string_lossy().into_owned(),
            "--log".into(),
            log.to_string_lossy().into_owned(),
        ],
        env: Vec::new(),
        path: None,
        process_dir: dir.path().join(PROCESS_DIR),
        ready_within,
    };
    let link = Link::for_preset("fake", launch, None, launch_storage());
    let cancel = CancellationToken::new();
    let mut started = Vec::with_capacity(tabs.len());
    for name in tabs {
        let (session, platform) = session(dir.path().to_owned(), name, deck.clone()).await;
        let session = Arc::new(Mutex::new(session));
        let tab = TabSession::new(*name, session.clone());
        let hub = SurfaceHub::new(tab.clone(), platform.live_localizer.clone(), Arc::new(oku_slides_mcp::NoVoice));
        let kind = session.lock().unwrap_or_else(std::sync::PoisonError::into_inner).document_kind();
        let tools = Tools::new(
            tab.clone(),
            hub.clone(),
            oku_slides_mcp::HostPorts {
                settings: Arc::new(NoSettings),
                tabs: Arc::new(oku_slides_mcp::NoTabs),
                workspace: Arc::new(oku_slides_session::agent::NoWorkspace),
                app: Arc::new(oku_slides_session::agent::UnavailableApp),
            },
            &platform,
            kind,
        );
        let server = ToolServer::start(tools, cancel.clone()).await.unwrap_or_else(|e| panic!("serveur MCP : {e}"));
        let access = ToolsAccess { url: server.url().to_owned(), authorization: server.authorization() };
        let keeper = Keeper::spawn(tab.clone(), hub.clone());
        keeper.restored().await;
        let agent = AgentHandle::spawn(
            tab,
            hub.clone(),
            access,
            link.clone(),
            platform.live_localizer.clone(),
            keeper.clone(),
            platform.settings.paste.prompt_limits(),
            kind,
        );
        let events = tokio::sync::Mutex::new(agent.subscribe());
        started.push(Tab { name: (*name).to_owned(), agent, hub, session, server, keeper, events });
    }
    Harness { tabs: started, link, log, cancel, dir }
}

pub async fn start_tabs(script: Value, tabs: &[&str]) -> Harness {
    start_tabs_with(script, tabs, None).await
}

/// Un seul onglet, `t-1` ; `command` remplace le faux agent.
pub async fn start_with(script: Value, command: Option<&str>) -> Harness {
    start_tabs_with(script, &["t-1"], command).await
}

pub async fn start(script: Value) -> Harness {
    start_with(script, None).await
}

/// Attend le premier évènement qui vérifie `pred` (20 s au plus).
pub async fn until(rx: &mut broadcast::Receiver<AgentEvent>, pred: impl Fn(&AgentEvent) -> bool) -> AgentEvent {
    within(Duration::from_secs(20), rx, pred).await.unwrap_or_else(|| panic!("évènement attendu non reçu"))
}

/// Le premier évènement qui vérifie `pred` dans `delay`, sinon `None`.
pub async fn within(
    delay: Duration,
    rx: &mut broadcast::Receiver<AgentEvent>,
    pred: impl Fn(&AgentEvent) -> bool,
) -> Option<AgentEvent> {
    tokio::time::timeout(delay, async {
        loop {
            match rx.recv().await {
                Ok(e) if pred(&e) => return e,
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => panic!("évènements fermés"),
            }
        }
    })
    .await
    .ok()
}

/// Attend la fin du prochain tour de l'onglet (10 s au plus).
pub async fn wait_turn_ended(tab: &Tab) -> AgentEvent {
    let mut rx = tab.events.lock().await;
    within(Duration::from_secs(10), &mut rx, |e| matches!(e, AgentEvent::TurnEnded { .. }))
        .await
        .unwrap_or_else(|| panic!("{} : fin de tour non reçue", tab.name))
}

/// Attend l'état `state` (10 s au plus).
pub async fn wait_status(rx: &mut broadcast::Receiver<AgentEvent>, state: AgentState) {
    within(Duration::from_secs(10), rx, |e| matches!(e, AgentEvent::Status { state: s, .. } if *s == state))
        .await
        .unwrap_or_else(|| panic!("état {state:?} non reçu"));
}

/// Attend une erreur (10 s au plus) ; rend son message.
pub async fn wait_error(rx: &mut broadcast::Receiver<AgentEvent>) -> String {
    match within(Duration::from_secs(10), rx, |e| matches!(e, AgentEvent::Error { .. })).await {
        Some(AgentEvent::Error { message, .. }) => message,
        _ => panic!("erreur attendue non reçue"),
    }
}

/// Attend la prochaine surface poussée par le hub (20 s au plus).
pub async fn wait_surface(
    rx: &mut broadcast::Receiver<oku_slides_mcp::McpEvent>,
) -> oku_slides_session::agent::Surface {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if let Ok(oku_slides_mcp::McpEvent::Surface { surface, .. }) = rx.recv().await {
                return surface;
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("surface attendue non reçue"))
}

/// La surface du tour `n`, clos (pour remplir un fil).
pub fn frozen_turn(n: u32) -> Surface {
    let view = TurnView {
        n,
        prompt: format!("consigne {n}"),
        plan: vec![],
        steps: vec![],
        tools: 0,
        notes: 0,
        narration: String::new(),
        images: Vec::new(),
        state: TurnState::Done { secs: 1 },
    };
    turn_surface(&view, &oku_slides_i18n::Localizer::new(Some("fr")))
}

/// La session de `t-1` connaît déjà `session` (relue au lancement, par l'agent `agent`).
pub async fn seed_session(h: &Harness, session: &str, agent: &str) {
    h.session("t-1")
        .lock()
        .unwrap_or_else(|e| panic!("session : {e}"))
        .keep_agent_session(session.into(), agent.into());
    h.tab("t-1").keeper.flush().await;
}

/// Les `sessionId` des messages `method` du journal du faux agent.
pub fn sessions_in(log: &Path, method: &str) -> Vec<String> {
    logged(log, method).iter().map(|m| m["sessionId"].as_str().unwrap_or_default().to_owned()).collect()
}

/// Tous les messages notés par le faux agent.
pub fn read_log(log: &Path) -> Vec<Value> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect()
}

/// La commande de l'interface décrite par `v` (JSON `{"cmd": …, "tab": …}`).
pub fn cmd(v: Value) -> oku_slides_agent::AgentCommand {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("commande : {e}"))
}

/// Paramètres des messages notés par le faux agent pour `method`.
pub fn logged(log: &Path, method: &str) -> Vec<Value> {
    read_log(log).into_iter().filter(|v| v["method"] == method).map(|v| v["params"].clone()).collect()
}

/// Les sessions de test : une par onglet.
///
/// The second field, a settled workspace, answers a pack open from an unknown tab
/// (`UnknownTab`): session errors cannot be built outside the session crate.
#[derive(Debug)]
pub struct TabSessions(pub Mutex<std::collections::HashMap<String, Session>>, pub Mutex<oku_slides_session::Workspace>);

impl TabSessions {
    /// No session yet, and a closed workspace on `platform`.
    fn closed_on(platform: &Platform) -> Self {
        let workspace = oku_slides_session::Workspace::new(Fonts::default(), platform, None)
            .unwrap_or_else(|e| panic!("espace de travail de test : {e}"));
        let workspace = Mutex::new(workspace);
        oku_slides_session::Workspace::settle(&workspace);
        Self(Mutex::default(), workspace)
    }
}

impl oku_slides_session::agent::Sessions for TabSessions {
    /// Onglet inconnu : `f` n'est pas appelée (`TabSession::with` rend alors une erreur).
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        if let Some(s) = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get_mut(tab) {
            f(s);
        }
        Ok(())
    }

    /// A known tab answers as a single-document host; an unknown one is refused by the closed workspace.
    ///
    /// Its session already editing `pack` opens it `Here`; any other pack is `SingleDocument`.
    fn open_pack(
        &self,
        tab: &str,
        pack: uuid::Uuid,
        widget: Option<&str>,
        gate: &mut dyn FnMut(oku_slides_session::agent::TabGate) -> bool,
    ) -> oku_slides_session::Result<oku_slides_session::agent::TabOpen> {
        // The map stays locked while its session is out: no other call finds the tab missing.
        let mut sessions = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(s) = sessions.remove(tab) {
            let single = Mutex::new(s);
            let opened = oku_slides_session::agent::Sessions::open_pack(&single, tab, pack, widget, gate);
            sessions.insert(tab.to_owned(), single.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner));
            return opened;
        }
        drop(sessions);
        oku_slides_session::Workspace::open_pack(&self.1, "onglet-ferme", pack, widget, gate)
    }

    /// As `open_pack`: a known tab answers as a single-document host, an unknown one `UnknownTab`.
    fn open_theme(
        &self,
        tab: &str,
        theme: uuid::Uuid,
        gate: &mut dyn FnMut(oku_slides_session::agent::TabGate) -> bool,
    ) -> oku_slides_session::Result<oku_slides_session::agent::TabOpen> {
        // The map stays locked while its session is out: no other call finds the tab missing.
        let mut sessions = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(s) = sessions.remove(tab) {
            let single = Mutex::new(s);
            let opened = oku_slides_session::agent::Sessions::open_theme(&single, tab, theme, gate);
            sessions.insert(tab.to_owned(), single.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner));
            return opened;
        }
        drop(sessions);
        oku_slides_session::Workspace::open_theme_for_agent(&self.1, "onglet-ferme", theme, gate)
    }

    /// A known tab answers with its session's kind; an unknown one is refused by the closed workspace.
    fn document_kind(&self, tab: &str) -> oku_slides_session::Result<oku_slides_session::DocumentKind> {
        if let Some(s) = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(tab) {
            return Ok(s.document_kind());
        }
        oku_slides_session::Workspace::document_kind(&self.1, "onglet-ferme")
    }

    fn usage_owner(&self, tab: &str) -> Option<String> {
        if self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).contains_key(tab) {
            return Some(tab.to_owned());
        }
        oku_slides_session::Workspace::usage_owner(&self.1, tab)
    }
}

/// The test host's windows: each one's active tab, the front one, and whether it has the focus.
#[derive(Debug, Default)]
pub struct FakeWorkspace(Mutex<FakeWindows>);

#[derive(Debug, Default)]
struct FakeWindows {
    active: std::collections::BTreeMap<String, String>,
    front: Option<String>,
    focused: bool,
    unsaved: usize,
}

impl FakeWorkspace {
    fn windows(&self) -> std::sync::MutexGuard<'_, FakeWindows> {
        self.0.lock().unwrap_or_else(|e| panic!("fenêtres : {e}"))
    }

    /// Window `window` shows `tab`.
    pub fn show(&self, window: &str, tab: &str) {
        self.windows().active.insert(window.to_owned(), tab.to_owned());
    }

    /// Window `window` gets the focus.
    pub fn focus(&self, window: &str) {
        let mut w = self.windows();
        w.front = Some(window.to_owned());
        w.focused = true;
    }

    /// Window `w-1` shows `tab` and gets the focus.
    pub fn focus_tab(&self, tab: &str) {
        self.show("w-1", tab);
        self.focus("w-1");
    }

    /// No window has the focus any more: the app is in the background.
    pub fn background(&self) {
        self.windows().focused = false;
    }

    /// From now on, `n` tabs hold unsaved work.
    pub fn set_unsaved(&self, n: usize) {
        self.windows().unsaved = n;
    }
}

impl AgentWorkspace for FakeWorkspace {
    fn focused_tab(&self) -> Option<FocusedTab> {
        let w = self.windows();
        let tab = w.active.get(w.front.as_deref()?)?.clone();
        Some(FocusedTab { tab, source: FocusSource::Host, focused: w.focused })
    }

    fn unsaved_tabs(&self) -> usize {
        self.windows().unsaved
    }
}

/// A closed utterance numbered `seq`, as the dictation hands it to the voice router.
pub fn utterance(seq: u64, text: &str) -> oku_slides_stt::Utterance {
    let now = std::time::Instant::now();
    oku_slides_stt::Utterance {
        seq: oku_slides_stt::UtteranceSeq(seq),
        text: text.to_owned(),
        started: now,
        ended: now,
        speech_secs: 1.0,
    }
}

/// Ce que `Stack::forward` a relayé : `(canal, JSON lu)`.
pub type Relayed = Arc<Mutex<Vec<(&'static str, Value)>>>;

/// Le tour `n` de l'onglet `tab` est fini : sa surface (canal `mcp`) ne court plus.
pub async fn wait_turn_done(rx: &Relayed, tab: &str, n: u32) -> (&'static str, Value) {
    let id = format!("turn-{n}");
    wait_for(rx, |(ch, v)| {
        *ch == "mcp"
            && v["tab"] == tab
            && v["event"]["kind"] == "surface_patch"
            && v["event"]["id"] == id.as_str()
            && v["event"]["messages"].as_array().is_some_and(|ms| {
                ms.iter().any(|m| m["updateDataModel"]["path"] == "/running" && m["updateDataModel"]["value"] == false)
            })
    })
    .await
}

/// Une surface d'outil (`ask-…`, `show-…`, `report-…`) relayée pour `tab` ; ignore les surfaces de tour.
pub fn is_tool_surface(v: &Value, tab: &str, prefix: &str) -> bool {
    v["tab"] == tab
        && v["event"]["kind"] == "surface"
        && v["event"]["surface"]["id"].as_str().is_some_and(|i| i.starts_with(prefix))
}

/// Les évènements `surfaces` relayés pour `tab`, dans l'ordre.
pub fn surfaces_of(rx: &Relayed, tab: &str) -> Vec<Value> {
    rx.lock()
        .unwrap_or_else(|e| panic!("relais : {e}"))
        .iter()
        .filter(|(ch, v)| *ch == "mcp" && v["tab"] == tab && v["event"]["kind"] == "surfaces")
        .map(|(_, v)| v.clone())
        .collect()
}

/// The `nth` `surfaces` event relayed for `tab` (1 = the first).
///
/// Polled every 20 ms, for 10 s at most. A starting tab sends one while reading
/// back its thread, before the one of its first `resync`.
pub async fn wait_surfaces(rx: &Relayed, tab: &str, nth: usize) -> Value {
    for _ in 0..500 {
        if let Some(v) = surfaces_of(rx, tab).into_iter().nth(nth - 1) {
            return v;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("{nth}e liste de surfaces de {tab} non reçue");
}

/// Le premier relais qui vérifie `pred` (sondé toutes les 20 ms, 10 s au plus).
pub async fn wait_for(rx: &Relayed, pred: impl Fn(&(&'static str, Value)) -> bool) -> (&'static str, Value) {
    for _ in 0..500 {
        let found = rx.lock().unwrap_or_else(|e| panic!("relais : {e}")).iter().find(|r| pred(r)).cloned();
        if let Some(r) = found {
            return r;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("relais attendu non reçu : {:?}", rx.lock().map(|v| v.len()));
}

/// Le nombre de relais reçus jusqu'ici.
pub fn relayed_len(rx: &Relayed) -> usize {
    rx.lock().unwrap_or_else(|e| panic!("relais : {e}")).len()
}

/// Attend `n` relais vérifiant `pred` parmi ceux reçus après les `from` premiers ; rend leurs positions et valeurs.
pub async fn wait_n_after(
    rx: &Relayed,
    from: usize,
    n: usize,
    pred: impl Fn(&(&'static str, Value)) -> bool,
) -> Vec<(usize, (&'static str, Value))> {
    for _ in 0..500 {
        let found: Vec<_> = rx
            .lock()
            .unwrap_or_else(|e| panic!("relais : {e}"))
            .iter()
            .enumerate()
            .skip(from)
            .filter(|(_, r)| pred(r))
            .map(|(i, r)| (i, r.clone()))
            .collect();
        if found.len() >= n {
            return found;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("{n} relais attendus après les {from} premiers, non reçus");
}

/// L'espace de noms du thème intégré nommé `name` (sans casse).
#[allow(dead_code, reason = "pas utilisée par tous les binaires de test")]
pub fn theme_ref(name: &str) -> Option<oku_slides_core::Namespace> {
    use oku_slides_session::{LibraryKind, Themes};
    Themes::builtins().iter().find(|b| b.manifest.name.eq_ignore_ascii_case(name)).map(|b| b.manifest.namespace.clone())
}

/// Le nom court (`papier`, `pro`) du thème intégré d'espace de noms `ns`.
#[allow(dead_code, reason = "pas utilisée par tous les binaires de test")]
pub fn theme_name(ns: &oku_slides_core::Namespace) -> Option<&'static str> {
    use oku_slides_session::{PAPIER_NAMESPACE, PRO_NAMESPACE};
    [PAPIER_NAMESPACE, PRO_NAMESPACE].into_iter().find(|n| ns.as_str() == *n)
}

pub fn is(
    ch: &'static str,
    kind: &'static str,
    tab: &'static str,
) -> impl Fn(&(&'static str, serde_json::Value)) -> bool {
    move |(c, v)| *c == ch && v["event"]["kind"] == kind && v["tab"] == tab
}

/// Le premier relais vérifiant `pred` après `from` : sa position et sa valeur.
pub async fn next(
    rx: &Relayed,
    from: usize,
    pred: impl Fn(&(&'static str, serde_json::Value)) -> bool,
) -> (usize, serde_json::Value) {
    let (i, (_, v)) = wait_n_after(rx, from, 1, pred).await.remove(0);
    (i, v)
}

/// Un `resync` de `tab` : rend la position juste après sa liste.
///
/// Un onglet qui démarre envoie d'abord une liste en relisant son fil, avant celle du `resync` :
/// sans liste relayée avant, on attend donc la deuxième.
pub async fn shown(stack: &TestStack, rx: &Relayed, tab: &'static str) -> usize {
    let before = surfaces_of(rx, tab).len();
    stack.handle(cmd(json!({ "cmd": "resync", "tab": tab }))).unwrap_or_else(|e| panic!("resync : {e}"));
    let nth = if before == 0 { 2 } else { before + 1 };
    for _ in 0..500 {
        let at = rx
            .lock()
            .unwrap_or_else(|e| panic!("relais : {e}"))
            .iter()
            .enumerate()
            .filter(|(_, (ch, v))| *ch == "mcp" && v["tab"] == tab && v["event"]["kind"] == "surfaces")
            .nth(nth - 1)
            .map(|(i, _)| i);
        if let Some(i) = at {
            return i + 1;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("liste du resync de {tab} non reçue");
}

/// Écrit le brouillon de `tab` depuis la vue `v-1`, et rend la position juste après son écho.
pub async fn drafted(stack: &TestStack, rx: &Relayed, tab: &'static str, text: Option<&str>, from: usize) -> usize {
    let req = json!({ "cmd": "set_draft", "tab": tab, "text": text, "origin": "v-1" });
    stack.handle(cmd(req)).unwrap_or_else(|e| panic!("set_draft : {e}"));
    let (echo, _) = next(rx, from, move |(c, v)| {
        *c == "agent"
            && v["event"]["kind"] == "draft"
            && v["tab"] == tab
            && v["event"]["origin"] == "v-1"
            && v["event"]["text"] == json!(text)
    })
    .await;
    echo + 1
}
