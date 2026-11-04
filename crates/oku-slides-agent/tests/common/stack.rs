//! The test stack: a real `Stack` over isolated tab sessions and a fake ACP agent, and its builders.

use super::*;

/// A test stack and its folder (script, log, data), removed
/// after it.
pub struct TestStack {
    // Declared before `dir`: the stack stops before its folder is removed.
    stack: Arc<oku_slides_agent::Stack>,
    /// The test host's focus, read by the voice router.
    pub workspace: Arc<FakeWorkspace>,
    /// The test host's app: update state, quits and installs.
    pub app: Arc<app::FakeApp>,
    sessions: Arc<TabSessions>,
    /// A resumed stack: once stopped by `shutdown`, its folder serves a restart.
    resumed: bool,
    /// The folder is kept on `drop` (a resumed stack, stopped).
    kept: AtomicBool,
    /// The folder of the previous stack whose session this one resumed; removed with it.
    adopted: Option<PathBuf>,
    dir: Option<tempfile::TempDir>,
}

impl TestStack {
    /// Runs `f` on `tab`'s session, off the async thread (the session's storage blocks).
    pub fn on_session<R>(&self, tab: &str, f: impl FnOnce(&mut Session) -> R) -> R {
        tokio::task::block_in_place(|| {
            let mut all = self.sessions.0.lock().unwrap_or_else(|e| panic!("sessions: {e}"));
            f(all.get_mut(tab).unwrap_or_else(|| panic!("unknown tab: {tab}")))
        })
    }

    /// Starts the voice router over a channel of the test's own: its sender.
    pub fn voice(&self) -> std::sync::mpsc::Sender<oku_slides_stt::Utterance> {
        self.voice_router().0
    }

    /// Like [`TestStack::voice`], with the router thread to join.
    pub fn voice_router(
        &self,
    ) -> (
        std::sync::mpsc::Sender<oku_slides_stt::Utterance>,
        Option<std::thread::JoinHandle<oku_slides_agent::RouterEnd>>,
    ) {
        let (tx, rx) = std::sync::mpsc::channel();
        let router = self.stack.route_utterances(rx);
        (tx, router)
    }

    /// Stops the stack (`Stack::shutdown`), then writes and drops the sessions.
    ///
    /// A resumed stack then keeps its folder: [`stack_resumed_from`] resumes it
    /// and removes it in turn.
    pub async fn shutdown(&self) {
        self.stack.shutdown().await;
        let sessions = self.sessions.clone();
        // Dropped outside the async context: their last resume state is written first.
        tokio::task::spawn_blocking(move || {
            let gone = std::mem::take(&mut *sessions.0.lock().unwrap_or_else(|e| panic!("sessions: {e}")));
            drop(gone);
        })
        .await
        .unwrap_or_else(|e| panic!("blocking task: {e}"));
        self.kept.store(self.resumed, Ordering::SeqCst);
    }
}

impl Drop for TestStack {
    fn drop(&mut self) {
        if self.resumed {
            // A resumed session still writes as it goes: before its folder is removed.
            let gone = std::mem::take(&mut *self.sessions.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
            drop(gone);
        }
        if let Some(dir) = self.adopted.take()
            && let Err(e) = std::fs::remove_dir_all(&dir)
        {
            eprintln!("previous stack folder not removed ({}): {e}", dir.display());
        }
        if self.kept.load(Ordering::SeqCst)
            && let Some(dir) = self.dir.take()
        {
            // Kept for `stack_resumed_from`, which removes it.
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

/// What a resumed stack first writes into `t-1`'s session (with the agent's fingerprint).
type Seed = Box<dyn FnOnce(&mut Session, &str) + Send>;

/// How `t-1`'s session is born in a test stack.
enum First {
    /// Opened on the fixture, as the other tabs.
    Fixture,
    /// Opened in a fresh resume folder where `seed` writes (with the agent's fingerprint), then resumed.
    Seeded(Seed),
    /// Resumed from a folder a previous stack already filled.
    From(PathBuf),
}

/// A stack over the fake agent (`script`), one session per tab of `tabs`,
/// relayed into a list; and the fake agent's log.
pub async fn stack_with_sink_and_log(script: Value, tabs: &[&str]) -> (TestStack, Relayed, PathBuf) {
    let (stack, seen, log, _) = build_stack(script, tabs, First::Fixture).await;
    (stack, seen, log)
}

/// Like [`stack_with_sink_and_log`], reading aloud through `speaker`.
pub async fn stack_with_speaker(
    script: Value,
    tabs: &[&str],
    speaker: oku_slides_stt::Speaker,
) -> (TestStack, Relayed, PathBuf) {
    let options = Options { speaker: Some(speaker), ..Options::default() };
    let (stack, seen, log, _) = build_stack_with(script, None, tabs, First::Fixture, options).await;
    (stack, seen, log)
}

/// A stack whose `t-1` session was resumed from a folder `seed` already wrote.
///
/// `seed` gets the session and the stack's agent fingerprint; the session is then
/// dropped and resumed from the same folder, as after a restart. Also returns that folder.
pub async fn stack_resumed(
    script: Value,
    seed: impl FnOnce(&mut Session, &str) + Send + 'static,
) -> (TestStack, Relayed, PathBuf, PathBuf) {
    build_stack(script, &["t-1"], First::Seeded(Box::new(seed))).await
}

/// The same stack, without `seed`, resumed from folder `resume` of a stopped resumed stack.
pub async fn stack_resumed_from(script: Value, resume: PathBuf) -> (TestStack, Relayed, PathBuf, PathBuf) {
    build_stack(script, &["t-1"], First::From(resume)).await
}

/// A stack with two fake agents, `faux` (active) and `bis`; also returns the data folder.
pub async fn stack_with_two_agents(script: Value, bis: Value, tabs: &[&str]) -> (TestStack, Relayed, PathBuf, PathBuf) {
    let (stack, seen, log, _) = build_stack_with(script, Some(bis), tabs, First::Fixture, Options::default()).await;
    let data = log.parent().unwrap_or_else(|| panic!("folder")).to_owned();
    (stack, seen, log, data)
}

/// As [`stack_with_two_agents`] on `t-1`, the choice locked by the environment (`locked`).
pub async fn stack_locked(script: Value, bis: Value) -> (TestStack, Relayed) {
    let options = Options { locked: true, ..Options::default() };
    let (stack, seen, _, _) = build_stack_with(script, Some(bis), &["t-1"], First::Fixture, options).await;
    (stack, seen)
}

/// A stack on `t-1` whose active agent runs `command` (with the fake agent's arguments).
pub async fn stack_with_command(script: Value, command: &str) -> (TestStack, Relayed) {
    let options = Options { command: Some(command.to_owned()), ..Options::default() };
    let (stack, seen, _, _) = build_stack_with(script, None, &["t-1"], First::Fixture, options).await;
    (stack, seen)
}

/// A test stack whose chosen agent is the built-in preset, with no app program.
pub async fn stack_with_builtin(script: Value) -> (TestStack, Relayed) {
    let options = Options { builtin: true, ..Options::default() };
    let (stack, seen, _, _) = build_stack_with(script, None, &["t-1"], First::Fixture, options).await;
    (stack, seen)
}

/// A stack on `t-1` whose selected agent `faux` runs `command` (the fake agent if `None`) on
/// `script`, with a built-in `local` preset: the fake agent again, on `local_script` (log
/// `log-local.jsonl`, returned), through a launcher standing for the app binary.
///
/// `ready`: the local engine is the fake one (ready), else Gguf without its model. The launcher
/// is a shell script: on Unix only.
pub async fn stack_with_fallback(
    script: Value,
    command: Option<&str>,
    local_script: Value,
    ready: bool,
) -> (TestStack, Relayed, PathBuf) {
    let local = Some(LocalOptions { script: local_script, ready });
    let options = Options { command: command.map(str::to_owned), local, ..Options::default() };
    let (stack, seen, log, _) = build_stack_with(script, None, &["t-1"], First::Fixture, options).await;
    let local_log = log.with_file_name("log-local.jsonl");
    (stack, seen, local_log)
}

/// The fake agent launched as the app binary would be for the built-in preset (`--local-agent`).
fn local_launcher(dir: &Path, script: &Value) -> PathBuf {
    let (script_path, log) = (dir.join("script-local.json"), dir.join("log-local.jsonl"));
    std::fs::write(&script_path, script.to_string()).unwrap_or_else(|e| panic!("local script: {e}"));
    let fake = env!("CARGO_BIN_EXE_oku-slides-fake-agent");
    let launcher = dir.join("oku-slides-app");
    let body =
        format!("#!/bin/sh\nexec '{fake}' --script '{}' --log '{}' \"$@\"\n", script_path.display(), log.display());
    std::fs::write(&launcher, body).unwrap_or_else(|e| panic!("local launcher: {e}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755))
            .unwrap_or_else(|e| panic!("launcher permissions: {e}"));
    }
    launcher
}

/// The engine of a ready local agent: the fake one, in debug builds only.
fn ready_engine() -> oku_slides_platform::config::LocalEngine {
    #[cfg(debug_assertions)]
    return oku_slides_platform::config::LocalEngine::Fake;
    #[cfg(not(debug_assertions))]
    return oku_slides_platform::config::LocalEngine::Gguf;
}

/// A test stack; `t-1`'s session is born as `first` says. Also returns its resume folder.
async fn build_stack(script: Value, tabs: &[&str], first: First) -> (TestStack, Relayed, PathBuf, PathBuf) {
    build_stack_with(script, None, tabs, first, Options::default()).await
}

/// The agent settings of a test stack.
#[derive(Default)]
struct Options {
    /// The agent choice is locked by the environment.
    locked: bool,
    /// The program of the active agent `faux`; absent: the fake agent.
    command: Option<String>,
    /// The active agent is the built-in preset (no program).
    builtin: bool,
    /// A built-in `local` preset besides the selected one, run by the fake agent.
    local: Option<LocalOptions>,
    /// The read-aloud voice; `None`: nothing is read aloud.
    speaker: Option<oku_slides_stt::Speaker>,
}

/// The built-in `local` preset of a test stack.
struct LocalOptions {
    /// The fake agent's script when it runs as the local agent.
    script: Value,
    /// The local engine is ready (fake), else Gguf without its model.
    ready: bool,
}

/// The fake agent's arguments on `script`, logging into `log`.
fn fake_args(script: &Path, log: &Path) -> Vec<String> {
    vec!["--script".into(), script.to_string_lossy().into_owned(), "--log".into(), log.to_string_lossy().into_owned()]
}

/// Adds the built-in `local` preset to `settings`; returns the app binary standing in for it.
fn with_local(settings: &mut Settings, local: &LocalOptions) -> PathBuf {
    settings.local_agent.engine =
        if local.ready { ready_engine() } else { oku_slides_platform::config::LocalEngine::Gguf };
    settings
        .agent
        .insert_preset("local", AgentPreset::builtin("Local", Builtin::Local))
        .unwrap_or_else(|e| panic!("local agent: {e}"));
    local_launcher(settings.paths.data.as_deref().unwrap_or_else(|| panic!("folder")), &local.script)
}

/// As [`build_stack`]; `bis`: the script of a second fake agent, `bis` (log `log-bis.jsonl`);
/// `options`: locked choice, the active agent's program.
async fn build_stack_with(
    script: Value,
    bis: Option<Value>,
    tabs: &[&str],
    first: First,
    options: Options,
) -> (TestStack, Relayed, PathBuf, PathBuf) {
    let Options { locked, command: active, builtin, local, speaker } = options;
    let dir = test_dir("oku-slides-stack-");
    let (script_path, log) = (dir.path().join("script.json"), dir.path().join("log.jsonl"));
    let data = dir.path().to_owned();
    std::fs::write(&script_path, script.to_string()).unwrap_or_else(|e| panic!("script: {e}"));
    // Exactly what the stack launches: the fingerprint is its agent's.
    let command = env!("CARGO_BIN_EXE_oku-slides-fake-agent");
    let active = active.unwrap_or_else(|| command.to_owned());
    let args = fake_args(&script_path, &log);
    let args_bis = bis.map(|bis| {
        let script_bis = dir.path().join("script-bis.json");
        std::fs::write(&script_bis, bis.to_string()).unwrap_or_else(|e| panic!("bis script: {e}"));
        fake_args(&script_bis, &dir.path().join("log-bis.jsonl"))
    });
    let fingerprint = agent_fingerprint(&active, &args);
    let (resume, adopted) = match &first {
        First::From(resume) => (resume.clone(), resume.parent().map(Path::to_path_buf)),
        First::Fixture | First::Seeded(_) => (data.join("resume-t-1"), None),
    };
    let resumed = !matches!(first, First::Fixture);
    let names: Vec<String> = tabs.iter().map(|t| (*t).to_owned()).collect();
    let resume_dir = resume.clone();
    let tabs_data = data.clone();
    let (sessions, platform) = tokio::task::spawn_blocking(move || {
        let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
        settings.paths.data = Some(data);
        // Never the real Documents folder: decks without a file stay here.
        settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
        settings.i18n.locale = Some("fr".into());
        // Never the real model hub: a test download fails at once (port refused).
        settings.models.hub_url = "http://127.0.0.1:9".into();
        settings.models.download_retries = 0;
        // Gguf, not auto: Apple's model, ready on some test Macs, would leave nothing to download.
        settings.local_agent.engine = oku_slides_platform::config::LocalEngine::Gguf;
        let preset =
            if builtin { AgentPreset::builtin("Faux", Builtin::Local) } else { AgentPreset::new("Faux", active, args) };
        let agent = &mut settings.agent;
        agent.insert_preset("faux", preset).unwrap_or_else(|e| panic!("faux agent: {e}"));
        agent.select("faux").unwrap_or_else(|e| panic!("faux agent: {e}"));
        if let Some(args_bis) = args_bis {
            agent
                .insert_preset("bis", AgentPreset::new("Bis", command, args_bis))
                .unwrap_or_else(|e| panic!("bis agent: {e}"));
        }
        settings.agent.locked = locked;
        let launcher = local.map(|local| with_local(&mut settings, &local));
        let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}"));
        let platform = match launcher {
            Some(program) => platform.with_program(program),
            None => platform,
        };
        let fixture = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
        let new = || Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session: {e}"));
        // Each tab resumes in its own folder: its agent works in `work/` there.
        let opened = |name: &str| {
            let mut session = new();
            session.open(&fixture).unwrap_or_else(|e| panic!("open: {e}"));
            assert!(session.resume(tab_resume_dir(&tabs_data, name)), "reprise de l'onglet");
            session
        };
        let sessions = TabSessions::closed_on(&platform);
        let mut first = Some(first);
        for name in names {
            let session = match first.take_if(|_| name == "t-1") {
                None | Some(First::Fixture) => opened(&name),
                Some(First::Seeded(seed)) => {
                    let mut first = new();
                    assert!(first.resume(resume_dir.clone()));
                    first.open(&fixture).unwrap_or_else(|e| panic!("open: {e}"));
                    seed(&mut first, &fingerprint);
                    // Dropped: what is left to write for the resume is written first.
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
            sessions.0.lock().unwrap_or_else(|e| panic!("sessions: {e}")).insert(name, session);
        }
        (Arc::new(sessions), platform)
    })
    .await
    .unwrap_or_else(|e| panic!("blocking task: {e}"));
    let (stack, workspace, app, seen) = started(sessions.clone(), platform, speaker);
    let stack =
        TestStack { stack, workspace, app, sessions, resumed, kept: AtomicBool::new(false), adopted, dir: Some(dir) };
    (stack, seen, log, resume)
}

/// The stack over `sessions`, with the test host's focus and app, relayed into a list.
fn started(
    sessions: Arc<TabSessions>,
    platform: Platform,
    speaker: Option<oku_slides_stt::Speaker>,
) -> (Arc<oku_slides_agent::Stack>, Arc<FakeWorkspace>, Arc<app::FakeApp>, Relayed) {
    let shared: Arc<dyn oku_slides_session::agent::Sessions> = sessions;
    let workspace = Arc::new(FakeWorkspace::default());
    let ports: Arc<dyn AgentWorkspace> = workspace.clone();
    let app = Arc::new(app::FakeApp::default());
    let host: Arc<dyn oku_slides_session::agent::AppPort> = app.clone();
    let stack = oku_slides_agent::Stack::start(
        shared,
        &platform,
        oku_slides_agent::Voices { stt: None, speaker },
        ports,
        host,
        tokio::runtime::Handle::current(),
    );
    // `Storage` owns a runtime: its last copy must not be dropped
    // in the test (async context), where the stack is.
    std::mem::forget(platform);
    let seen: Relayed = Arc::default();
    let sink = seen.clone();
    stack.forward(move |channel, json| {
        let value = serde_json::from_str(&json).unwrap_or_else(|e| panic!("relayed JSON: {e}"));
        sink.lock().unwrap_or_else(|e| panic!("relay: {e}")).push((channel, value));
    });
    (stack, workspace, app, seen)
}

pub async fn stack_with_sink(script: Value, tabs: &[&str]) -> (TestStack, Relayed) {
    let (stack, seen, _) = stack_with_sink_and_log(script, tabs).await;
    (stack, seen)
}
