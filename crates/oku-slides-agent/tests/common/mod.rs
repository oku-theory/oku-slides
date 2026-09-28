//! Pile de test : sessions isolées (une par onglet), outils MCP réels par
//! onglet, un faux agent partagé.
#![allow(dead_code, reason = "chaque fichier de test n'utilise qu'une partie de l'outillage")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oku_slides_agent::{AgentEvent, AgentHandle, AgentState, Launch, Link, ToolsAccess};
use oku_slides_mcp::{Applied, SettingsPort, SurfaceHub, ToolServer, Tools};
use oku_slides_session::agent::TabSession;
use oku_slides_session::{Fonts, Platform, Session, Settings};
use serde_json::Value;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Default)]
pub struct NoSettings;

impl SettingsPort for NoSettings {
    fn read(&self) -> std::collections::BTreeMap<String, Value> {
        Default::default()
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

/// Un dossier neuf, supprimé au `drop` (t-273).
///
/// Créé atomiquement sous un nom aléatoire : jamais celui d'un test
/// précédent, dont le journal (ouvert en ajout) serait sinon relu.
fn test_dir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new().prefix(prefix).tempdir().unwrap_or_else(|e| panic!("dossier de test : {e}"))
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

/// Démarre un onglet par nom de `tabs`, sur un seul faux agent (ou `command`) suivant `script`.
pub async fn start_tabs_with(script: Value, tabs: &[&str], command: Option<&str>) -> Harness {
    start_tabs_ready_within(script, tabs, command, oku_slides_agent::READY_WITHIN).await
}

/// Un onglet `t-1` dont l'agent doit être prêt dans `ready_within`.
pub async fn start_ready_within(script: Value, ready_within: Duration) -> Harness {
    start_tabs_ready_within(script, &["t-1"], None, ready_within).await
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
        fallback_cwd: dir.path().to_owned(),
        ready_within,
    };
    let link = Link::new(launch);
    let cancel = CancellationToken::new();
    let mut started = Vec::with_capacity(tabs.len());
    for name in tabs {
        let (session, platform) = session(dir.path().to_owned()).await;
        let session = Arc::new(Mutex::new(session));
        let tab = TabSession::new(*name, session.clone());
        let hub = SurfaceHub::new(tab.clone(), platform.localizer.clone());
        let tools = Tools::new(tab.clone(), hub.clone(), Arc::new(NoSettings), &platform);
        let server = ToolServer::start(tools, cancel.clone()).await.unwrap_or_else(|e| panic!("serveur MCP : {e}"));
        let access = ToolsAccess { url: server.url().to_owned(), authorization: server.authorization() };
        let agent = AgentHandle::spawn(tab, hub.clone(), access, link.clone(), platform.localizer.clone());
        let events = tokio::sync::Mutex::new(agent.subscribe());
        started.push(Tab { name: (*name).to_owned(), agent, hub, session, server, events });
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
    within(Duration::from_secs(10), rx, |e| matches!(e, AgentEvent::Status { state: s } if *s == state))
        .await
        .unwrap_or_else(|| panic!("état {state:?} non reçu"));
}

/// Attend une erreur (10 s au plus) ; rend son message.
pub async fn wait_error(rx: &mut broadcast::Receiver<AgentEvent>) -> String {
    match within(Duration::from_secs(10), rx, |e| matches!(e, AgentEvent::Error { .. })).await {
        Some(AgentEvent::Error { message }) => message,
        _ => panic!("erreur attendue non reçue"),
    }
}

/// Attend la prochaine surface poussée par le hub (20 s au plus).
pub async fn wait_surface(
    rx: &mut broadcast::Receiver<oku_slides_mcp::McpEvent>,
) -> oku_slides_session::agent::Surface {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if let Ok(oku_slides_mcp::McpEvent::Surface { surface }) = rx.recv().await {
                return surface;
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("surface attendue non reçue"))
}

/// Tous les messages notés par le faux agent.
pub fn read_log(log: &PathBuf) -> Vec<Value> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect()
}

/// Paramètres des messages notés par le faux agent pour `method`.
pub fn logged(log: &PathBuf, method: &str) -> Vec<Value> {
    read_log(log).into_iter().filter(|v| v["method"] == method).map(|v| v["params"].clone()).collect()
}

/// Les sessions de test : une par onglet.
#[derive(Debug, Default)]
pub struct TabSessions(pub Mutex<std::collections::HashMap<String, Session>>);

impl oku_slides_session::agent::Sessions for TabSessions {
    /// Onglet inconnu : `f` n'est pas appelée (`TabSession::with` rend alors une erreur).
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        if let Some(s) = self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get_mut(tab) {
            f(s);
        }
        Ok(())
    }
}

/// Ce que `Stack::forward` a relayé : `(canal, JSON lu)`.
pub type Relayed = Arc<Mutex<Vec<(&'static str, Value)>>>;

/// Une pile de test et son dossier (scénario, journal, données), supprimé
/// après elle.
pub struct TestStack {
    // Déclarée avant `dir` : la pile s'arrête avant que son dossier soit supprimé.
    stack: oku_slides_agent::Stack,
    dir: tempfile::TempDir,
}

impl std::ops::Deref for TestStack {
    type Target = oku_slides_agent::Stack;
    fn deref(&self) -> &oku_slides_agent::Stack {
        &self.stack
    }
}

/// Une pile sur le faux agent (`script`), une session par onglet de `tabs`,
/// relayée dans une liste ; et le journal du faux agent.
pub async fn stack_with_sink_and_log(script: Value, tabs: &[&str]) -> (TestStack, Relayed, PathBuf) {
    let dir = test_dir("oku-slides-stack-");
    let (script_path, log) = (dir.path().join("script.json"), dir.path().join("log.jsonl"));
    let data = dir.path().to_owned();
    std::fs::write(&script_path, script.to_string()).unwrap_or_else(|e| panic!("scénario : {e}"));
    let args = vec![
        "--script".into(),
        script_path.to_string_lossy().into_owned(),
        "--log".into(),
        log.to_string_lossy().into_owned(),
    ];
    let names: Vec<String> = tabs.iter().map(|t| (*t).to_owned()).collect();
    let (sessions, platform) = tokio::task::spawn_blocking(move || {
        let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
        settings.paths.data = Some(data);
        // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
        settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
        settings.i18n.locale = Some("fr".into());
        settings.agent.command = env!("CARGO_BIN_EXE_oku-slides-fake-agent").into();
        settings.agent.args = args;
        let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
        let fixture = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
        let sessions = TabSessions::default();
        for name in names {
            let mut session = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {e}"));
            session.open(&fixture).unwrap_or_else(|e| panic!("ouverture : {e}"));
            sessions.0.lock().unwrap_or_else(|e| panic!("sessions : {e}")).insert(name, session);
        }
        (sessions, platform)
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante : {e}"));
    let stack = oku_slides_agent::Stack::start(Arc::new(sessions), &platform, None, tokio::runtime::Handle::current());
    // `Storage` possède un runtime : sa dernière copie ne doit pas être lâchée
    // dans le test (contexte async), là où la pile l'est.
    std::mem::forget(platform);
    let seen: Relayed = Arc::default();
    let sink = seen.clone();
    stack.forward(move |channel, json| {
        let value = serde_json::from_str(&json).unwrap_or_else(|e| panic!("JSON relayé : {e}"));
        sink.lock().unwrap_or_else(|e| panic!("relais : {e}")).push((channel, value));
    });
    (TestStack { stack, dir }, seen, log)
}

pub async fn stack_with_sink(script: Value, tabs: &[&str]) -> (TestStack, Relayed) {
    let (stack, seen, _) = stack_with_sink_and_log(script, tabs).await;
    (stack, seen)
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
