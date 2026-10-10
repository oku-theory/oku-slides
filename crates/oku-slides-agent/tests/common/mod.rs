//! Pile de test : sessions isolées (une par onglet), outils MCP réels par
//! onglet, un faux agent partagé.
#![allow(dead_code, reason = "chaque fichier de test n'utilise qu'une partie de l'outillage")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oku_slides_agent::{AgentEvent, AgentHandle, AgentState, Keeper, Launch, Link, ToolsAccess, agent_fingerprint};
use oku_slides_mcp::{Applied, SettingsPort, SurfaceHub, ToolServer, Tools};
use oku_slides_session::agent::turn_view::{TurnState, TurnView, turn_surface};
use oku_slides_session::agent::{AgentScope, Surface, TabSession};
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
    /// Le gardien de l'onglet : seul passage vers ses conversations.
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
        let keeper = Keeper::spawn(tab.clone(), hub.clone());
        keeper.restored().await;
        let agent =
            AgentHandle::spawn(tab, hub.clone(), access, link.clone(), platform.localizer.clone(), keeper.clone());
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
            if let Ok(oku_slides_mcp::McpEvent::Surface { surface }) = rx.recv().await {
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
        state: TurnState::Done { secs: 1 },
    };
    turn_surface(&view, &oku_slides_i18n::Localizer::new(Some("fr")))
}

/// La session de `t-1` connaît déjà `scope` → `session` (relue au lancement, par l'agent `agent`).
pub async fn seed_session(h: &Harness, scope: AgentScope, session: &str, agent: &str) {
    h.session("t-1").lock().unwrap_or_else(|e| panic!("session : {e}")).keep_agent_session(
        scope,
        session.into(),
        agent.into(),
    );
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

/// Paramètres des messages notés par le faux agent pour `method`.
pub fn logged(log: &Path, method: &str) -> Vec<Value> {
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
    sessions: Arc<TabSessions>,
    /// Pile reprise : une fois arrêtée par `shutdown`, son dossier sert à un redémarrage.
    resumed: bool,
    /// Le dossier est gardé au `drop` (pile reprise arrêtée).
    kept: AtomicBool,
    /// Le dossier de la pile précédente dont celle-ci a repris la session ; supprimé avec elle.
    adopted: Option<PathBuf>,
    dir: Option<tempfile::TempDir>,
}

impl TestStack {
    /// `f` sur la session de `tab`, hors du fil async (le stockage de la session bloque).
    pub fn on_session<R>(&self, tab: &str, f: impl FnOnce(&mut Session) -> R) -> R {
        tokio::task::block_in_place(|| {
            let mut all = self.sessions.0.lock().unwrap_or_else(|e| panic!("sessions : {e}"));
            f(all.get_mut(tab).unwrap_or_else(|| panic!("onglet inconnu : {tab}")))
        })
    }

    /// Arrête la pile (`Stack::shutdown`), puis écrit et lâche les sessions.
    ///
    /// Une pile reprise garde ensuite son dossier : [`stack_resumed_from`] le reprend
    /// et le supprime à son tour.
    pub async fn shutdown(&self) {
        self.stack.shutdown().await;
        let sessions = self.sessions.clone();
        // Lâchées hors du contexte async : leur dernier état de reprise est écrit avant.
        tokio::task::spawn_blocking(move || {
            let gone = std::mem::take(&mut *sessions.0.lock().unwrap_or_else(|e| panic!("sessions : {e}")));
            drop(gone);
        })
        .await
        .unwrap_or_else(|e| panic!("tâche bloquante : {e}"));
        self.kept.store(self.resumed, Ordering::SeqCst);
    }
}

impl Drop for TestStack {
    fn drop(&mut self) {
        if self.resumed {
            // Une session reprise écrit encore en partant : avant que son dossier soit supprimé.
            let gone = std::mem::take(&mut *self.sessions.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
            drop(gone);
        }
        if let Some(dir) = self.adopted.take()
            && let Err(e) = std::fs::remove_dir_all(&dir)
        {
            eprintln!("dossier de la pile précédente non supprimé ({}) : {e}", dir.display());
        }
        if self.kept.load(Ordering::SeqCst)
            && let Some(dir) = self.dir.take()
        {
            // Gardé pour `stack_resumed_from`, qui le supprimera.
            let _kept = dir.keep();
        }
    }
}

impl std::ops::Deref for TestStack {
    type Target = oku_slides_agent::Stack;
    fn deref(&self) -> &oku_slides_agent::Stack {
        &self.stack
    }
}

/// Ce qu'une pile reprise écrit d'abord dans la session de `t-1` (avec l'empreinte de l'agent).
type Seed = Box<dyn FnOnce(&mut Session, &str) + Send>;

/// Comment naît la session de `t-1` dans une pile de test.
enum First {
    /// Ouverte sur la fixture, comme les autres onglets.
    Fixture,
    /// Ouverte dans un dossier de reprise neuf où `seed` écrit (avec l'empreinte de l'agent), puis reprise.
    Seeded(Seed),
    /// Reprise d'un dossier déjà rempli par une pile précédente.
    From(PathBuf),
}

/// Une pile sur le faux agent (`script`), une session par onglet de `tabs`,
/// relayée dans une liste ; et le journal du faux agent.
pub async fn stack_with_sink_and_log(script: Value, tabs: &[&str]) -> (TestStack, Relayed, PathBuf) {
    let (stack, seen, log, _) = build_stack(script, tabs, First::Fixture).await;
    (stack, seen, log)
}

/// Une pile dont la session `t-1` a été reprise d'un dossier où `seed` a déjà écrit.
///
/// `seed` reçoit la session et l'empreinte de l'agent de la pile ; la session est ensuite
/// lâchée puis reprise du même dossier, comme après un redémarrage. Rend aussi ce dossier.
pub async fn stack_resumed(
    script: Value,
    seed: impl FnOnce(&mut Session, &str) + Send + 'static,
) -> (TestStack, Relayed, PathBuf, PathBuf) {
    build_stack(script, &["t-1"], First::Seeded(Box::new(seed))).await
}

/// La même pile, sans `seed`, reprise du dossier `resume` d'une pile reprise arrêtée.
pub async fn stack_resumed_from(script: Value, resume: PathBuf) -> (TestStack, Relayed, PathBuf, PathBuf) {
    build_stack(script, &["t-1"], First::From(resume)).await
}

/// Une pile de test ; la session de `t-1` naît selon `first`. Rend aussi son dossier de reprise.
async fn build_stack(script: Value, tabs: &[&str], first: First) -> (TestStack, Relayed, PathBuf, PathBuf) {
    let dir = test_dir("oku-slides-stack-");
    let (script_path, log) = (dir.path().join("script.json"), dir.path().join("log.jsonl"));
    let data = dir.path().to_owned();
    std::fs::write(&script_path, script.to_string()).unwrap_or_else(|e| panic!("scénario : {e}"));
    // Exactement ce que la pile lance : l'empreinte est celle de son agent.
    let command = env!("CARGO_BIN_EXE_oku-slides-fake-agent");
    let args = vec![
        "--script".into(),
        script_path.to_string_lossy().into_owned(),
        "--log".into(),
        log.to_string_lossy().into_owned(),
    ];
    let fingerprint = agent_fingerprint(command, &args);
    let (resume, adopted) = match &first {
        First::From(resume) => (resume.clone(), resume.parent().map(Path::to_path_buf)),
        First::Fixture | First::Seeded(_) => (data.join("resume-t-1"), None),
    };
    let resumed = !matches!(first, First::Fixture);
    let names: Vec<String> = tabs.iter().map(|t| (*t).to_owned()).collect();
    let resume_dir = resume.clone();
    let (sessions, platform) = tokio::task::spawn_blocking(move || {
        let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
        settings.paths.data = Some(data);
        // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
        settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
        settings.i18n.locale = Some("fr".into());
        settings.agent.command = command.into();
        settings.agent.args = args;
        let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
        let fixture = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
        let new = || Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {e}"));
        let opened = || {
            let mut session = new();
            session.open(&fixture).unwrap_or_else(|e| panic!("ouverture : {e}"));
            session
        };
        let sessions = TabSessions::default();
        let mut first = Some(first);
        for name in names {
            let session = match first.take_if(|_| name == "t-1") {
                None | Some(First::Fixture) => opened(),
                Some(First::Seeded(seed)) => {
                    let mut first = new();
                    assert!(first.resume(resume_dir.clone()));
                    first.open(&fixture).unwrap_or_else(|e| panic!("ouverture : {e}"));
                    seed(&mut first, &fingerprint);
                    // Lâchée : ce qui reste à écrire pour la reprise l'est d'abord.
                    drop(first);
                    let mut session = new();
                    assert!(session.resume(resume_dir.clone()));
                    session
                }
                Some(First::From(resume)) => {
                    let mut session = new();
                    assert!(session.resume(resume));
                    session
                }
            };
            sessions.0.lock().unwrap_or_else(|e| panic!("sessions : {e}")).insert(name, session);
        }
        (Arc::new(sessions), platform)
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante : {e}"));
    let shared: Arc<dyn oku_slides_session::agent::Sessions> = sessions.clone();
    let stack = oku_slides_agent::Stack::start(shared, &platform, None, tokio::runtime::Handle::current());
    // `Storage` possède un runtime : sa dernière copie ne doit pas être lâchée
    // dans le test (contexte async), là où la pile l'est.
    std::mem::forget(platform);
    let seen: Relayed = Arc::default();
    let sink = seen.clone();
    stack.forward(move |channel, json| {
        let value = serde_json::from_str(&json).unwrap_or_else(|e| panic!("JSON relayé : {e}"));
        sink.lock().unwrap_or_else(|e| panic!("relais : {e}")).push((channel, value));
    });
    let stack = TestStack { stack, sessions, resumed, kept: AtomicBool::new(false), adopted, dir: Some(dir) };
    (stack, seen, log, resume)
}

pub async fn stack_with_sink(script: Value, tabs: &[&str]) -> (TestStack, Relayed) {
    let (stack, seen, _) = stack_with_sink_and_log(script, tabs).await;
    (stack, seen)
}

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

/// Le `nth` évènement `surfaces` relayé pour `tab` (1 = le premier ; sondé toutes les 20 ms, 10 s au plus).
///
/// Un onglet qui démarre en envoie un en relisant ses fils, avant celui de son premier `resync`.
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
