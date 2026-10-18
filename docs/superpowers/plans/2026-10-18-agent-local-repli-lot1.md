# Local fallback agent, lot 1 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A built-in ACP agent (`oku-slides --local-agent`) that the host launches like any agent preset, which routes each turn to tool families, runs a bounded MCP tool loop and ends with `report`, on a scripted `fake` engine.

**Architecture:** A new crate `oku-slides-local-agent` implements the agent side of ACP (`agent-client-protocol` `Agent` builder over `Stdio`, as `src/bin/fake_agent.rs` does) and the MCP client (`rust-mcp-sdk` `client_runtime`). Every app binary calls `exit_if_local_agent()` at the top of `main`; the host resolves a preset with `builtin = "local"` to `<current exe> --local-agent`. The engine is a trait; lot 1 ships only the `fake` engine (debug builds), so no preset is added to `config.default.toml` yet.

**Tech Stack:** Rust 2024, `agent-client-protocol` 2.2.0, `rust-mcp-sdk` 1.1.0, Tokio, Snafu, Fluent (`oku-slides-i18n`), speculoos, rstest, nextest.

**Spec:** `docs/superpowers/specs/2026-10-18-agent-local-repli-design.md` (lot 1 = t-1020).

**Tasks:** t-1020; Task 1 to Task 7 = t-1027 to t-1033, titles verbatim.

## Global Constraints

- Every commit: `type(scope) : description (t-1020)` (space before the colon), message in French, written to a scratchpad file and committed with `git commit -F <file>`.
- No `unwrap`/`expect` outside tests; no silent fallback; Snafu errors with a `backtrace` in every variant; opaque public error types.
- No `std::fs` in production code (go through `Storage`); `std::fs` is fine in tests and in the debug-only fake engine's script/log files (test tooling, like `fake_agent.rs`).
- Never hold a `std::sync::Mutex` guard across an `.await`; lock with `lock().unwrap_or_else(PoisonError::into_inner)`.
- Never advertise `fs` or `terminal` capabilities; the agent acts only through MCP tools.
- Every `pub` item and module documented, first sentence under 15 words; source files ≤ 500 lines (hard cap 1000).
- Unit tests in sibling `x_tests.rs` files included by `#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`, speculoos assertions, names `unit_scenario_result`, rstest for tables.
- Dependency versions only in root `[workspace.dependencies]`, crates use `.workspace = true`; never `thiserror` or `chrono`.
- New Fluent keys: English first, then all 20 locales (`crates/oku-slides-i18n/src/locales/*/agent.ftl`), same key order, real translations, no space between a number and a CJK counter.
- Agent instructions in English only.
- Each `ARCHITECTURE.md` ≤ 50 lines, committed with the code it describes.
- Tests that start the app or the agent set `OKU_SLIDES_PATHS__DATA` / `OKU_SLIDES_PATHS__DECKS` to a fresh `tempfile` dir.
- Gate: `just check` and `just test` green before the last commit; `just ci` before merge.

## Deviations from the spec (fixed in the spec by Task 1's commit)

1. The local agent is started by a **`--local-agent` argument**, not an `OKU_SLIDES_LOCAL_AGENT` env var: the config crate reads every `OKU_SLIDES_<SECTION>` variable, and that name would collide with the planned `[local_agent]` section (`local_agent.engine`). It also matches `--render-worker`.
2. `Engine::complete` returns a whole reply and takes a `CancellationToken` that real engines check per token; it is not a stream (lot 1 has nothing to stream, progress is sent as ACP updates by the turn loop).
3. The `[agent.presets.local]` entry lands in `config.default.toml` with the first real engine (lot 3, t-1022): a preset whose only engine is the debug `fake` would be a half feature.

## File map

| File | Responsibility |
|---|---|
| `crates/oku-slides-platform/src/config.rs` (modify) | `AgentPreset.builtin: Option<Builtin>`, `Builtin`, exactly-one check |
| `crates/oku-slides-platform/src/lib.rs` (modify) | `LOCAL_AGENT_FLAG`, `Platform.program`, `with_program` |
| `crates/oku-slides-agent/src/config.rs` (modify) | `Launch::for_preset` |
| `crates/oku-slides-agent/src/stack.rs` (modify) | use `Launch::for_preset`; availability of a built-in preset |
| `crates/oku-slides-agent/src/picker.rs` (modify) | hint of a built-in preset |
| `crates/oku-slides-agent/src/error.rs` (modify) | `Unusable::NoProgram` |
| `crates/oku-slides-mcp/src/tools.rs`, `lib.rs` (modify) | `pub fn tool_names()` |
| `crates/oku-slides-local-agent/` (create) | the agent: `entry`, `acp`, `sessions`, `mcp`, `prompt`, `families`, `compact`, `engine`, `fake`, `turn`, `error` |
| `crates/oku-slides-local-agent/src/bin/oku-slides-local-agent-dev.rs` (create) | test-only binary: `exit_if_local_agent()` and nothing else |
| `crates/oku-slides-cli/src/main.rs`, `src-tauri/src/main.rs` (modify) | call `exit_if_local_agent()` after `exit_if_worker()` |
| `AGENTS.md`, `README.md`, `crates/oku-slides-local-agent/{ARCHITECTURE,AGENTS}.md` | rules, crate map, harness map |

---

### Task 1: Built-in presets in the platform settings

**Files:**
- Modify: `crates/oku-slides-platform/src/config.rs:350-385` (`AgentPreset`, `AgentSettings::check`)
- Modify: `crates/oku-slides-platform/src/lib.rs:40-120` (`LOCAL_AGENT_FLAG`, `Platform.program`)
- Test: `crates/oku-slides-platform/src/config_tests.rs`
- Modify: `docs/superpowers/specs/2026-10-18-agent-local-repli-design.md` (the three deviations above)

**Interfaces:**
- Produces:
  - `pub enum Builtin { Local }` (serde `rename_all = "snake_case"`, derives `Debug, Clone, Copy, PartialEq, Eq, Deserialize`)
  - `AgentPreset { name: String, command: String /* default "" */, args: Vec<String> /* default [] */, builtin: Option<Builtin> /* default None */ }`
  - `AgentPreset::builtin(name, Builtin) -> AgentPreset`
  - `pub const LOCAL_AGENT_FLAG: &str = "--local-agent";` in `oku_slides_platform`
  - `Platform.program: Option<PathBuf>` (this executable; `Some` from `Platform::load`, `None` from `with_settings`), `Platform::with_program(self, PathBuf) -> Self`

- [ ] **Step 1: Write the failing tests** in `config_tests.rs`

```rust
#[test]
fn agent_settings_builtin_preset_loads() {
    let toml = r#"
[agent]
selected = "local"
[agent.presets.local]
name = "Local"
builtin = "local"
"#;
    let settings = settings_from_toml(toml).unwrap();
    assert_that!(settings.agent.active().builtin).is_equal_to(Some(Builtin::Local));
    assert_that!(settings.agent.active().command.as_str()).is_empty();
}

#[rstest]
#[case::both("name = \"X\"\ncommand = \"npx\"\nbuiltin = \"local\"")]
#[case::neither("name = \"X\"")]
fn agent_settings_preset_without_exactly_one_launch_is_refused(#[case] body: &str) {
    let toml = format!("[agent]\nselected = \"x\"\n[agent.presets.x]\n{body}\n");
    assert_that!(settings_from_toml(&toml)).is_err();
}
```

`settings_from_toml` is the helper this file already uses to load a TOML string over the defaults; if it is named differently, use the existing one (search `from_sources` in `config_tests.rs`). The default presets (`claude`, `codex`, `gemini`) stay in the merged table; that is fine, they are valid.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo nextest run -p oku-slides-platform agent_settings_`
Expected: compile error, `Builtin` not found.

- [ ] **Step 3: Implement**

In `config.rs`:

```rust
/// Un agent fourni par Oku Slides lui-même, sans programme externe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Builtin {
    /// L'agent local de repli (`oku-slides-local-agent`), lancé par ce binaire.
    Local,
}

/// Un agent proposé : son nom affiché et comment le lancer.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct AgentPreset {
    /// Nom propre affiché tel quel (pas traduit).
    pub name: String,
    /// Programme lancé (cherché dans le `PATH`) ; vide pour un agent intégré.
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Agent intégré : exclusif de `command`.
    #[serde(default)]
    pub builtin: Option<Builtin>,
}

impl AgentPreset {
    /// Un agent proposé (tests, outils).
    pub fn new(name: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
        Self { name: name.into(), command: command.into(), args, builtin: None }
    }

    /// Un agent intégré à Oku Slides.
    pub fn builtin(name: impl Into<String>, builtin: Builtin) -> Self {
        Self { name: name.into(), command: String::new(), args: Vec::new(), builtin: Some(builtin) }
    }
}
```

Extend `AgentSettings::check` (keep the existing `selected` check first):

```rust
for (id, p) in &self.presets {
    let external = !p.command.is_empty();
    if external == p.builtin.is_some() {
        return Err(format!("[agent.presets.{id}] needs exactly one of command or builtin"));
    }
}
```

In `lib.rs`, next to `RENDER_WORKER_ENV`:

```rust
/// Argument qui fait d'un binaire de l'app l'agent local (voir `oku-slides-local-agent`).
pub const LOCAL_AGENT_FLAG: &str = "--local-agent";
```

Add `pub program: Option<PathBuf>` to `Platform` (doc: "Ce binaire, relancé pour l'agent local ; `None` hors de l'app (tests)."), set it to `Some(program.clone())` in `load()` (the `program` already read there), `None` in `with_settings`, and:

```rust
/// Cette plateforme, dont l'agent local se lance avec `program` (tests).
pub fn with_program(mut self, program: PathBuf) -> Self {
    self.program = Some(program);
    self
}
```

Fix every `AgentPreset { .. }` struct literal the compiler reports (tests in other crates) by using `AgentPreset::new`.

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p oku-slides-platform` then `cargo check --workspace --tests`
Expected: PASS, workspace compiles.

- [ ] **Step 5: Fix the spec** — in the spec, replace the `OKU_SLIDES_LOCAL_AGENT` env marker with the `--local-agent` argument (Process and ACP section, and its reason), say `Engine` returns a whole reply and takes a cancellation token, and note under "Preset and offer" that the `[agent.presets.local]` default entry lands with lot 3.

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-platform docs/superpowers/specs/2026-10-18-agent-local-repli-design.md
# plus any test file fixed in step 3
git commit -F <scratchpad>/msg.txt   # feat(platform) : préréglage d'agent intégré et argument --local-agent (t-1020)
```

---

### Task 2: The host launches and checks a built-in preset

**Files:**
- Modify: `crates/oku-slides-agent/src/config.rs`
- Create: `crates/oku-slides-agent/src/config_tests.rs`
- Modify: `crates/oku-slides-agent/src/stack.rs:185-192, 334-346, 395-420`
- Modify: `crates/oku-slides-agent/src/picker.rs:28-38`, `picker_tests.rs`
- Modify: `crates/oku-slides-agent/src/error.rs:48-65`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (key `agent-local-no-program`)
- Test: `crates/oku-slides-agent/tests/availability.rs`

**Interfaces:**
- Consumes: `AgentPreset.builtin`, `Builtin::Local`, `LOCAL_AGENT_FLAG`, `Platform.program` (Task 1).
- Produces: `Launch::for_preset(preset: &AgentPreset, program: Option<&Path>, fallback_cwd: PathBuf) -> Option<Launch>` (`None`: built-in preset and no program); `Unusable::NoProgram` localized by `agent-local-no-program`.

- [ ] **Step 1: Write the failing tests** — `config_tests.rs` (declare it at the end of `config.rs` with the `#[cfg(test)] #[path]` pattern):

```rust
use std::path::{Path, PathBuf};

use oku_slides_platform::config::{AgentPreset, Builtin};
use speculoos::prelude::*;

use super::Launch;

#[test]
fn launch_for_preset_external_keeps_its_command() {
    let p = AgentPreset::new("Claude", "npx", vec!["-y".into()]);
    let l = Launch::for_preset(&p, None, PathBuf::from("/tmp")).unwrap();
    assert_that!(l.command.as_str()).is_equal_to("npx");
    assert_that!(l.args).is_equal_to(vec!["-y".to_owned()]);
}

#[test]
fn launch_for_preset_builtin_runs_this_program_with_the_flag() {
    let p = AgentPreset::builtin("Local", Builtin::Local);
    let l = Launch::for_preset(&p, Some(Path::new("/Apps/Oku")), PathBuf::from("/tmp")).unwrap();
    assert_that!(l.command.as_str()).is_equal_to("/Apps/Oku");
    assert_that!(l.args).is_equal_to(vec![oku_slides_platform::LOCAL_AGENT_FLAG.to_owned()]);
}

#[test]
fn launch_for_preset_builtin_without_program_is_none() {
    let p = AgentPreset::builtin("Local", Builtin::Local);
    assert_that!(Launch::for_preset(&p, None, PathBuf::from("/tmp"))).is_none();
}
```

In `picker_tests.rs`, add `picker_builtin_preset_has_no_command_hint` (a built-in preset's `ChoiceOption.hint` is `None`), following the file's existing pattern. In `tests/availability.rs`, add `availability_builtin_without_program_names_it` asserting, with locale `fr` (the harness default) and settings whose `selected` preset is `AgentPreset::builtin("Local", Builtin::Local)`, that `Stack::availability()` is `Some` and equals `Localizer::new(Some("fr")).tr("agent-local-no-program")`. Copy how that file builds a `Stack` with custom settings.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo nextest run -p oku-slides-agent launch_for_preset picker_builtin availability_builtin`
Expected: compile errors (`for_preset`, `NoProgram` missing).

- [ ] **Step 3: Implement** — in `config.rs`:

```rust
impl Launch {
    /// Le lancement de `preset` ; un agent intégré relance `program` avec son argument.
    ///
    /// `None` : agent intégré et pas de binaire de l'app connu (tests, outils).
    pub fn for_preset(preset: &AgentPreset, program: Option<&Path>, fallback_cwd: PathBuf) -> Option<Self> {
        let (command, args) = match preset.builtin {
            None => (preset.command.clone(), preset.args.clone()),
            Some(Builtin::Local) => {
                (program?.to_string_lossy().into_owned(), vec![oku_slides_platform::LOCAL_AGENT_FLAG.to_owned()])
            }
        };
        Some(Self { command, args, fallback_cwd, ready_within: READY_WITHIN })
    }
}
```

In `stack.rs`, `Stack::start` and `select_agent` build their `Launch` with `Launch::for_preset(preset, platform.program.as_deref(), platform.settings.paths.data())`. When it returns `None` (a built-in preset outside the app), keep a `Launch` whose command is empty, so a prompt fails on spawn exactly like a missing program, and `availability()` gives the reason:

```rust
let launch = Launch::for_preset(a, platform.program.as_deref(), platform.settings.paths.data())
    .unwrap_or_else(|| Launch { command: String::new(), args: Vec::new(),
        fallback_cwd: platform.settings.paths.data(), ready_within: crate::READY_WITHIN });
```

This is not a silent fallback: `availability()` reports `NoProgram` for that preset before any launch. In `availability()`, before the `find_program` check:

```rust
if preset.builtin.is_some() {
    return match self.platform.program {
        None => Some(Unusable::NoProgram.localize(&l)),
        Some(_) => self.link.failure().map(|u| u.localize(&l)),
    };
}
```

In `error.rs` add `NoProgram` (doc: "Agent intégré, mais ce processus n'est pas l'app (tests, outils).") localized with `l.tr("agent-local-no-program")`. In `picker.rs`, `hint` is `None` when `p.builtin.is_some()`.

Add `agent-local-no-program` to `en/agent.ftl` after the other `agent-*` unusable keys (`agent-no-http`), then the same position in the 19 other locales, translated. English: `agent-local-no-program = The local agent can only run inside the Oku Slides app.`; French: `agent-local-no-program = L'agent local ne tourne que dans l'app Oku Slides.`

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p oku-slides-agent -p oku-slides-i18n`
Expected: PASS (the i18n tests check every key exists in all locales).

- [ ] **Step 5: Commit** — `feat(agent) : l'hôte lance un agent intégré par son propre binaire (t-1020)`; update `crates/oku-slides-agent/ARCHITECTURE.md` (`config`: `Launch::for_preset`).

---

### Task 3: The local agent crate answers the ACP handshake

**Files:**
- Create: `crates/oku-slides-local-agent/Cargo.toml`, `src/lib.rs`, `src/entry.rs`, `src/entry_tests.rs`, `src/acp.rs`, `src/sessions.rs`, `src/sessions_tests.rs`, `src/error.rs`, `src/bin/oku-slides-local-agent-dev.rs`, `ARCHITECTURE.md`, `AGENTS.md`
- Modify: root `Cargo.toml` (`[workspace.dependencies] oku-slides-local-agent = { path = "crates/oku-slides-local-agent" }`)
- Modify: `crates/oku-slides-cli/Cargo.toml`, `crates/oku-slides-cli/src/main.rs:280-283`, `src-tauri/Cargo.toml`, `src-tauri/src/main.rs:84-87`
- Create: `crates/oku-slides-cli/tests/local_agent.rs`
- Modify: root `AGENTS.md` (invariant + harness map), `README.md` (crate map)

**Interfaces:**
- Consumes: `LOCAL_AGENT_FLAG` (Task 1).
- Produces:
  - `pub fn exit_if_local_agent()` — returns at once unless `std::env::args()` contains `LOCAL_AGENT_FLAG`; otherwise serves ACP on stdin/stdout and calls `std::process::exit`.
  - `pub(crate) struct Args { engine: EngineChoice, script: Option<PathBuf>, log: Option<PathBuf> }`, `pub(crate) fn parse_args(args: &[String]) -> Option<Result<Args, LocalAgentError>>` (`None`: flag absent).
  - `pub(crate) enum EngineChoice { Fake }` (lot 3 and 4 add `Gguf`, `Apple`).
  - `pub(crate) struct Sessions` with `open(&self, mcp: McpAccess) -> String` (ids `local-1`, `local-2`…), `get(&self, id) -> Option<Arc<SessionState>>`; `pub(crate) struct McpAccess { url: String, authorization: String }`; `SessionState { mcp: McpAccess, cancel: Mutex<CancellationToken>, summaries: Mutex<VecDeque<String>> }`.
  - `pub(crate) async fn serve(args: Args) -> Result<(), LocalAgentError>` in `acp.rs`; Task 5 changes it to `serve(engine: Arc<dyn Engine>) -> Result<(), LocalAgentError>`, the engine being built in `entry.rs` from `Args`.

- [ ] **Step 1: Write the failing tests**

`entry_tests.rs`:

```rust
use rstest::rstest;
use speculoos::prelude::*;

use super::{EngineChoice, parse_args};

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn parse_args_without_flag_is_none() {
    assert_that!(parse_args(&args(&["oku-slides", "check"]))).is_none();
}

#[test]
fn parse_args_fake_engine_takes_script_and_log() {
    let a = parse_args(&args(&["x", "--local-agent", "--engine", "fake", "--script", "/s.json", "--log", "/l"]))
        .unwrap()
        .unwrap();
    assert_that!(a.engine).is_equal_to(EngineChoice::Fake);
    assert_that!(a.script.as_deref()).is_equal_to(Some(std::path::Path::new("/s.json")));
}

#[rstest]
#[case::no_engine(&["x", "--local-agent"])]
#[case::unknown_engine(&["x", "--local-agent", "--engine", "gpt"])]
fn parse_args_without_known_engine_is_an_error(#[case] v: &[&str]) {
    assert_that!(parse_args(&args(v)).unwrap()).is_err();
}
```

Lot 1 has no default engine: `--local-agent` alone is an error ("no engine available"), which the host shows as a failed launch. Lot 3 makes `auto` the default.

`sessions_tests.rs`: `sessions_open_numbers_from_one` (`local-1`, then `local-2`) and `sessions_get_unknown_is_none`.

`crates/oku-slides-cli/tests/local_agent.rs`:

```rust
//! `oku-slides --local-agent` répond à la poignée de main ACP.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use speculoos::prelude::*;

#[test]
fn local_agent_flag_answers_initialize() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("script.json");
    std::fs::write(&script, r#"{"turns":[]}"#).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_oku-slides"))
        .args(["--local-agent", "--engine", "fake", "--script"])
        .arg(&script)
        .env("OKU_SLIDES_PATHS__DATA", dir.path())
        .env("OKU_SLIDES_PATHS__DECKS", dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":1,"clientCapabilities":{{}}}}}}"#).unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
    child.kill().unwrap();
    let reply: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_that!(reply["id"]).is_equal_to(serde_json::json!(1));
    assert_that!(reply["result"]["agentCapabilities"]["mcpCapabilities"]["http"]).is_equal_to(serde_json::json!(true));
}
```

Check the `initialize` params shape against what the host sends (log one from `fake_agent --log` in an existing test if unsure); keep `clientCapabilities` minimal.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo nextest run -p oku-slides-local-agent -p oku-slides-cli local_agent parse_args sessions_`
Expected: the crate does not exist yet → create `Cargo.toml` and an empty `lib.rs` first, then the tests fail on missing items.

- [ ] **Step 3: Implement**

`Cargo.toml`:

```toml
[package]
name = "oku-slides-local-agent"
description = "Agent local de repli : agent ACP intégré, outils MCP routés par familles, moteur local."
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
publish.workspace = true

[[bin]]
# Relance de test (`--local-agent`) pour les tests de bout en bout ; jamais livré.
name = "oku-slides-local-agent-dev"
path = "src/bin/oku-slides-local-agent-dev.rs"

[dependencies]
oku-slides-i18n.workspace = true
oku-slides-platform.workspace = true
agent-client-protocol.workspace = true
rust-mcp-sdk.workspace = true
async-trait.workspace = true
tokio = { workspace = true, features = ["sync", "macros", "rt", "time", "io-std"] }
tokio-util.workspace = true
serde.workspace = true
serde_json.workspace = true
snafu.workspace = true
tracing.workspace = true

[dev-dependencies]
oku-slides-agent.workspace = true
oku-slides-mcp.workspace = true
oku-slides-session.workspace = true
speculoos.workspace = true
rstest.workspace = true
tempfile.workspace = true

[lints]
workspace = true
```

`lib.rs`:

```rust
//! Agent local de repli : un agent ACP intégré à l'app, sans compte ni réseau.
//!
//! - [`exit_if_local_agent`] : à appeler en tête de `main`, après `exit_if_worker`.
//!
//! Le binaire de l'app, relancé avec `--local-agent`, sert ACP sur stdin/stdout ;
//! il n'agit que par les outils MCP de l'onglet, routés par familles (`families`).

mod acp;
mod entry;
mod error;
mod sessions;

pub use entry::exit_if_local_agent;
```

`entry.rs` — `parse_args` reads `--engine <name>`, `--script <path>`, `--log <path>` after the flag; `EngineChoice::Fake` is accepted only when `cfg!(debug_assertions)` (a release build refuses it with the same "no engine" error). `exit_if_local_agent`:

```rust
/// Si ce processus est l'agent local (`--local-agent`), sert ACP puis termine ; sinon ne fait rien.
///
/// À appeler en tête de `main`, juste après `exit_if_worker` : stdout est le tube ACP,
/// les journaux vont sur stderr.
pub fn exit_if_local_agent() {
    let argv: Vec<String> = std::env::args().collect();
    let Some(parsed) = parse_args(&argv) else { return };
    oku_slides_platform::telemetry::init();
    let code = match parsed.and_then(run) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!(error = %snafu::Report::from_error(&e), "agent local");
            1
        }
    };
    std::process::exit(code);
}

fn run(args: Args) -> Result<(), LocalAgentError> {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().context(RuntimeSnafu)?;
    rt.block_on(crate::acp::serve(args))
}
```

`error.rs` — opaque `LocalAgentError(Inner)` with variants `NoEngine { backtrace }`, `Runtime { source: std::io::Error, backtrace }`, `Acp { message: String, backtrace }` (from `agent_client_protocol::Error` by `to_string()`, since that type is not `std::error::Error`; check, and use it as `source` if it is). Display messages lowercase English (logs only; nothing here reaches the user).

`acp.rs` — mirror `fake_agent.rs` `main`: `Agent.builder().name("oku-slides-local-agent")` with handlers:
- `InitializeRequest` → `InitializeResponse::new(req.protocol_version).agent_capabilities(AgentCapabilities::new().mcp_capabilities(McpCapabilities::new().http(true)))` (no `load_session`, no resume).
- `NewSessionRequest` → pick the `McpServer::Http` entry and its `Authorization` header (copy `State::remember_mcp`); none → `respond_with_internal_error("no http mcp server")`; else `NewSessionResponse::new(sessions.open(access))`.
- `PromptRequest` → in lot 1's Task 3, respond `PromptResponse::new(StopReason::EndTurn)` (Task 6 replaces it with the turn loop).
- `CancelNotification` → `sessions.get(id)` and cancel its token.
- `.connect_to(Stdio::new()).await`.

`src/bin/oku-slides-local-agent-dev.rs`:

```rust
//! Relance de test de l'agent local : `--local-agent` sert ACP, sinon rien.

fn main() {
    oku_slides_local_agent::exit_if_local_agent();
    eprintln!("usage: oku-slides-local-agent-dev --local-agent --engine fake --script <file>");
    std::process::exit(2);
}
```

Wire the binaries: add `oku-slides-local-agent.workspace = true` to `crates/oku-slides-cli/Cargo.toml` and `src-tauri/Cargo.toml`, and right after `oku_slides_render_worker::exit_if_worker();` in both `main`s:

```rust
    // Relancé en agent local (`--local-agent`) : sert ACP sur stdin/stdout, puis termine.
    oku_slides_local_agent::exit_if_local_agent();
```

Root `AGENTS.md`: change the invariant to `- Call exit_if_worker then exit_if_local_agent as the first lines of main in every app binary, before any other initialization. (spec:2026-09-29-compilation-bornee-design, spec:2026-10-18-agent-local-repli-design)`; add to the harness map `- crates/oku-slides-local-agent/AGENTS.md — Read before changing the local fallback agent, its engines or tool routing.` Crate `AGENTS.md` (rules sections, each rule ending with its source):

```markdown
# crates/oku-slides-local-agent — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Never write to stdout in this crate; stdout is the ACP pipe, logs go to stderr. (spec:2026-10-18-agent-local-repli-design)
- Map every MCP tool to a tool family; the completeness test fails otherwise. (spec:2026-10-18-agent-local-repli-design)
- Accept the fake engine only in debug builds. (plan:2026-10-18-agent-local-repli-lot1)

## Testing

- Test turns with the scripted fake engine and a fake tool port, never a real model. (plan:2026-10-18-agent-local-repli-lot1)
```

`ARCHITECTURE.md` (≤ 50 lines): what the crate is, modules (`entry`, `acp`, `sessions`, then the ones later tasks add), flow (handshake → session per tab with its MCP server → prompt → route → act → report), dependencies. README crate map: one line for the crate.

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p oku-slides-local-agent -p oku-slides-cli` then `just agents-lint`
Expected: PASS.

- [ ] **Step 5: Commit** — `feat(local-agent) : agent ACP intégré lancé par --local-agent (t-1020)`.

---

### Task 4: Tool families and compact schemas

**Files:**
- Modify: `crates/oku-slides-mcp/src/tools.rs` (add `tool_names`), `crates/oku-slides-mcp/src/lib.rs` (re-export)
- Create: `crates/oku-slides-local-agent/src/families.rs`, `families_tests.rs`, `src/compact.rs`, `compact_tests.rs`

**Interfaces:**
- Produces:
  - `oku_slides_mcp::tool_names() -> Vec<String>` (doc: "Noms de tous les outils MCP, dans l'ordre de la liste.")
  - `pub(crate) enum Family { Edit, Present, Files, Templates, Configure, Troubleshoot, AskReport }` with `fn key(self) -> &'static str` (`"edit"`, `"present"`, `"files"`, `"templates"`, `"configure"`, `"troubleshoot"`, `"ask_report"`), `fn summary(self) -> &'static str` (one English line for the route prompt), `const ROUTABLE: [Family; 6]` (all but `AskReport`).
  - `pub(crate) fn families_of(tool: &str) -> &'static [Family]` (empty: unknown tool)
  - `pub(crate) fn parse_route(reply: &str) -> Vec<Family>` — the first two known family keys found in `reply`, in order, deduplicated; empty if none.
  - `pub(crate) struct ToolSpec { pub name: String, pub description: String, pub schema: serde_json::Value }`
  - `pub(crate) fn compact(spec: &ToolSpec) -> ToolSpec` — description cut after its first sentence (first `". "` or end), every nested `"description"` key removed from the schema; `type`, `properties`, `required`, `enum`, `items`, `oneOf`/`anyOf` kept.
  - `pub(crate) fn exposed(all: &[ToolSpec], route: &[Family]) -> Vec<ToolSpec>` — compacted specs whose families intersect `route` ∪ {`AskReport`}, in `all`'s order.

Family table (a tool may sit in several families):

| Family | Tools |
|---|---|
| Edit | `apply`, `focus`, `deck_outline`, `slide`, `render_slide`, `widgets`, `chart_kind`, `show_as`, `show_params`, `show_variants` |
| Present | `present`, `focus`, `deck_outline` |
| Files | `open`, `new`, `save`, `move`, `merge`, `history`, `deck_outline` |
| Templates | `templates`, `edit_templates`, `save_templates`, `close_templates`, `delete_template_set`, `edit_theme`, `theme_tokens`, `widget_source`, `widget_sources`, `set_widget`, `remove_widget`, `set_widget_trial`, `preview_widget` |
| Configure | `settings`, `set_setting`, `set_interface` |
| Troubleshoot | `settings`, `render_slide`, `import_report`, `import_diff`, `diagnose` |
| AskReport | `ask_choice`, `ask_confirm`, `ask_rewrite`, `report`, `render` |

Tools that do not exist yet on this branch (`present`, `move`, `merge`, `history`, `set_interface` from t-988; `diagnose` from t-1021) stay in the table: an entry for a missing tool is harmless, and the completeness test only checks the other direction.

- [ ] **Step 1: Write the failing tests** — `families_tests.rs`:

```rust
use rstest::rstest;
use speculoos::prelude::*;

use super::{Family, families_of, parse_route};

#[test]
fn families_every_mcp_tool_has_a_family() {
    let orphans: Vec<String> =
        oku_slides_mcp::tool_names().into_iter().filter(|t| families_of(t).is_empty()).collect();
    assert_that!(orphans).is_empty();
}

#[rstest]
#[case("edit", vec![Family::Edit])]
#[case("configure, then troubleshoot", vec![Family::Configure, Family::Troubleshoot])]
#[case("edit edit files present", vec![Family::Edit, Family::Files])]
#[case("nothing useful", vec![])]
fn parse_route_keeps_two_known_families(#[case] reply: &str, #[case] expected: Vec<Family>) {
    assert_that!(parse_route(reply)).is_equal_to(expected);
}
```

`parse_route` must match whole keys: `"ask_report"` is never routable and is ignored if the model names it.

`compact_tests.rs`:

```rust
use serde_json::json;
use speculoos::prelude::*;

use super::{ToolSpec, compact, exposed};
use crate::families::Family;

fn spec(name: &str) -> ToolSpec {
    ToolSpec {
        name: name.into(),
        description: "Save the deck. The user confirms first.".into(),
        schema: json!({"type": "object", "properties": {"path": {"type": "string", "description": "long"}}, "required": ["path"]}),
    }
}

#[test]
fn compact_keeps_first_sentence() {
    assert_that!(compact(&spec("save")).description.as_str()).is_equal_to("Save the deck.");
}

#[test]
fn compact_drops_nested_descriptions_keeps_required() {
    let c = compact(&spec("save"));
    assert_that!(c.schema["properties"]["path"].get("description")).is_none();
    assert_that!(c.schema["required"]).is_equal_to(json!(["path"]));
}

#[test]
fn exposed_adds_ask_report_and_filters_by_route() {
    let all = vec![spec("save"), spec("apply"), spec("report")];
    let names: Vec<String> = exposed(&all, &[Family::Edit]).into_iter().map(|s| s.name).collect();
    assert_that!(names).is_equal_to(vec!["apply".to_owned(), "report".to_owned()]);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo nextest run -p oku-slides-local-agent families compact parse_route exposed`
Expected: compile errors.

- [ ] **Step 3: Implement** — `tool_names` in `tools.rs`: `pub fn tool_names() -> Vec<String> { list().into_iter().map(|t| t.name).collect() }`, re-exported in `lib.rs` (`pub use tools::{Tools, tool_names};`). `families.rs`: the table as a `match` in `families_of` returning `&'static [Family]` slices; `parse_route` tokenizes on non-`[a-z_]` characters and keeps the first two distinct routable keys. `compact.rs`: a recursive `fn strip(v: &mut Value)` removing `"description"` from every object except the top-level tool description. Add both modules to `lib.rs`.

- [ ] **Step 4: Run the tests** — `cargo nextest run -p oku-slides-local-agent -p oku-slides-mcp` → PASS.

- [ ] **Step 5: Commit** — `feat(local-agent) : familles d'outils et schémas compacts (t-1020)`; update the crate `ARCHITECTURE.md` and `crates/oku-slides-mcp/ARCHITECTURE.md` (`tool_names`).

---

### Task 5: Engine trait, fake engine and prompt reading

**Files:**
- Create: `crates/oku-slides-local-agent/src/engine.rs`, `src/fake.rs`, `fake_tests.rs`, `src/prompt.rs`, `prompt_tests.rs`, `src/instructions/local.md`
- Modify: `src/entry.rs` (build the engine from `Args`), `src/lib.rs`

**Interfaces:**
- Consumes: `ToolSpec` (Task 4), `Args`/`EngineChoice` (Task 3).
- Produces:
  - `pub(crate) enum Step { Route, Act }`
  - `pub(crate) enum Message { User(String), Assistant(String), ToolResult { name: String, output: String } }`
  - `pub(crate) struct Completion { pub step: Step, pub system: String, pub messages: Vec<Message>, pub tools: Vec<ToolSpec> }`
  - `pub(crate) enum Reply { Text(String), Call { name: String, args: serde_json::Value } }`
  - `#[async_trait] pub(crate) trait Engine: Send + Sync + Debug { async fn complete(&self, c: &Completion, cancel: &CancellationToken) -> Result<Reply, EngineError>; }` — doc: "Implement this trait to add a local model engine (Gguf, Apple)."
  - `pub(crate) struct EngineError` (opaque Snafu; variants `Cancelled`, `Failed { message }`, `ScriptExhausted`), with `fn is_cancelled(&self) -> bool`.
  - `pub(crate) struct FakeEngine` built by `FakeEngine::from_file(script: &Path, log: Option<PathBuf>) -> Result<Self, LocalAgentError>`; script JSON: `{"turns":[{"route":"edit","replies":[{"call":"apply","args":{…}},{"text":"…"}],"hang":false}]}` — each `complete` call with `Step::Route` returns turn N's `route` as `Reply::Text`; each `Step::Act` call returns the next entry of `replies`; `hang: true` waits for `cancel` then returns `Cancelled`. Each call appends one JSON line `{"step","tools":[names],"messages":n}` to `log`.
  - `pub(crate) struct Prompt { pub request: String, pub context: Option<String>, pub locale: Option<String> }`, `pub(crate) fn read_prompt(blocks: &[ContentBlock]) -> Prompt`
  - `pub(crate) const INSTRUCTIONS: &str = include_str!("instructions/local.md");`

- [ ] **Step 1: Write the failing tests** — `prompt_tests.rs`:

```rust
use agent_client_protocol::schema::v1::{ContentBlock, TextContent};
use speculoos::prelude::*;

use super::read_prompt;

fn text(s: &str) -> ContentBlock {
    ContentBlock::Text(TextContent::new(s.to_owned()))
}

#[test]
fn read_prompt_takes_last_block_as_request_and_skips_host_instructions() {
    let p = read_prompt(&[
        text("You edit an Oku Slides presentation…"),
        text("<oku-slides-context>\ninterface language: French (fr) · deck language: …\n</oku-slides-context>"),
        text("mets le titre en rouge"),
    ]);
    assert_that!(p.request.as_str()).is_equal_to("mets le titre en rouge");
    assert_that!(p.locale.as_deref()).is_equal_to(Some("fr"));
    assert_that!(p.context.unwrap()).starts_with("<oku-slides-context>");
}

#[test]
fn read_prompt_without_context_has_no_locale() {
    assert_that!(read_prompt(&[text("bonjour")]).locale).is_none();
}
```

The locale is the text between the parentheses after `interface language: `, up to `)`. The host's `agent.md`/`widgets.md` blocks are ignored: the local agent uses its own short `INSTRUCTIONS`.

`fake_tests.rs`: `fake_route_then_replies_in_order` (route text, then call, then text), `fake_exhausted_script_is_an_error`, `fake_hang_returns_cancelled_when_cancelled` (cancel the token from a spawned task after 10 ms; `is_cancelled()` true), `fake_logs_each_completion` (log file has 2 lines with the right `step` and tool names). Use `tempfile` for script and log.

- [ ] **Step 2: Run them to see them fail** — `cargo nextest run -p oku-slides-local-agent read_prompt fake_` → compile errors.

- [ ] **Step 3: Implement** — `engine.rs` and `fake.rs` per the interfaces (`fake.rs` reads its files with `std::fs`: debug-only test tooling, gated `#[cfg(debug_assertions)]` along with `EngineChoice::Fake`, mention the reason in its module doc). `prompt.rs` per the tests. `instructions/local.md` (English, short, no final newline):

```markdown
You are Oku Slides' local assistant. You act on the open presentation and on the app only through the tools you are given.
Route step: answer with one or two family names from the list, nothing else.
Act step: call one tool at a time. Read before you change (deck_outline, slide, settings). Never confirm on the user's behalf: guarded tools ask the user themselves.
Write every text the user reads in the interface language given in the context.
End every turn with report: a one-line title saying what changed, and at most 4 short facts.
```

`entry.rs`: `Args` → `Arc<dyn Engine>` (`EngineChoice::Fake` → `FakeEngine::from_file(script?, log)`; missing `--script` is `NoEngine`), passed to `acp::serve`.

- [ ] **Step 4: Run the tests** — `cargo nextest run -p oku-slides-local-agent` → PASS.

- [ ] **Step 5: Commit** — `feat(local-agent) : moteur abstrait, moteur factice et lecture du prompt (t-1020)`.

---

### Task 6: The turn loop over MCP

**Files:**
- Create: `crates/oku-slides-local-agent/src/mcp.rs`, `src/turn.rs`, `turn_tests.rs`
- Modify: `src/acp.rs` (prompt handler runs a turn), `src/sessions.rs` (summaries), `src/lib.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (3 keys)

**Interfaces:**
- Consumes: `Engine`, `Completion`, `Reply`, `Step`, `Message` (Task 5); `exposed`, `parse_route`, `Family`, `ToolSpec` (Task 4); `Prompt` (Task 5); `SessionState` (Task 3).
- Produces:
  - `#[async_trait] pub(crate) trait ToolPort: Send + Sync { async fn list(&self) -> Result<Vec<ToolSpec>, String>; async fn call(&self, name: &str, args: &Value) -> Result<String, String>; }` — `Err` carries the MCP error text, also for a result with `is_error: true`.
  - `pub(crate) struct McpTools` implementing `ToolPort` over one `client_runtime` started once per session (`McpTools::connect(&McpAccess) -> Result<Self, String>`), with `shut_down` on drop of the session.
  - `pub(crate) trait Updates: Send + Sync { fn tool_started(&self, id: &str, name: &str, args: &Value); fn tool_finished(&self, id: &str, ok: bool); fn note(&self, text: &str); }` — implemented in `acp.rs` with `SessionUpdate::ToolCall` / `ToolCallUpdate` / `AgentMessageChunk` notifications.
  - `pub(crate) const MAX_CALLS: usize = 8;`
  - `pub(crate) async fn run_turn(engine: &dyn Engine, tools: &dyn ToolPort, updates: &dyn Updates, prompt: &Prompt, summaries: &[String], cancel: &CancellationToken) -> TurnEnd`
  - `pub(crate) enum TurnEnd { Reported, Cancelled, Failed(FailKind) }`, `pub(crate) enum FailKind { NoRoute, ToolLimit, InvalidCall, Engine }`

Turn algorithm (`run_turn`):
1. `tools.list()`; on error → send the failure report (step 6) with `FailKind::Engine`.
2. Route: `Completion { step: Route, system: INSTRUCTIONS + "\n" + one line per `Family::ROUTABLE` (`key: summary`), messages: [summaries joined as one User message if any, then User(context + "\n\n" + request)], tools: [] }`. `parse_route` of the text reply; a `Call` reply or an empty route → retry once; still empty → `FailKind::NoRoute`.
3. Act: `exposed(&all, &route)`; loop while calls < `MAX_CALLS`: `complete(Act)`.
   - `Reply::Call { name, args }` with `name` not in the exposed set, or `tools.call` returning `Err` → push `ToolResult { name, output: "error: " + err }` and count a repair; a second consecutive repair → `FailKind::InvalidCall`. Otherwise push the result, reset the repair count. Each call is wrapped in `updates.tool_started` / `tool_finished` with id `call-{n}`.
   - A successful `report` call ends the turn: `TurnEnd::Reported`.
   - `Reply::Text(t)` → `updates.note(t)` (shown as one progress line), push `Assistant(t)`, continue.
4. Calls exhausted → `FailKind::ToolLimit`.
5. `cancel` fired (checked before each engine call and after each tool call), or `EngineError::is_cancelled` → `TurnEnd::Cancelled` without a report.
6. Failure: call `report` with `{"title": <localized>}` using `Localizer::new(prompt.locale.as_deref())` and keys `local-agent-no-route`, `local-agent-tool-limit` (`$n` = `MAX_CALLS`), `local-agent-invalid-call`, `local-agent-engine-failed`; return `TurnEnd::Failed(kind)` even if that report call fails (log it).

After the turn, `acp.rs` pushes a one-line summary `"<request> → <report title or failure key>"` into the session's `summaries`, keeping the last 3, and answers `PromptResponse::new(StopReason::EndTurn)` (`Cancelled` → `StopReason::Cancelled`). A new prompt on a session cancels that session's running turn first (replace the token).

- [ ] **Step 1: Write the failing tests** — `turn_tests.rs` with a `FakeTools` (records calls; `list` returns specs for `apply`, `save`, `report`; `call("apply", …)` returns `Ok`, a configurable tool returns `Err`) and a `NoUpdates`, plus `FakeEngine` built from an inline script written to a tempfile:

```rust
#[tokio::test]
async fn run_turn_routes_calls_and_ends_on_report() {
    let engine = engine(json!({"turns":[{"route":"edit","replies":[
        {"call":"apply","args":{"ops":[]}},
        {"call":"report","args":{"title":"Titre en rouge"}}]}]}));
    let tools = FakeTools::default();
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("mets le titre en rouge"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Reported);
    assert_that!(tools.called()).is_equal_to(vec!["apply".to_owned(), "report".to_owned()]);
}
```

Also, one behaviour each:
- `run_turn_act_exposes_only_routed_families` — route `edit`: the fake engine's log shows the `Act` completion's tools include `apply` and `report`, not `save`.
- `run_turn_unknown_tool_is_repaired_once` — replies `[call "nope", call "report"]` → `Reported`, and the second `Act` completion received a `ToolResult` starting with `error:`.
- `run_turn_two_invalid_calls_fail_with_report` — `[call "nope", call "nope"]` → `Failed(InvalidCall)`, and `tools.called()` ends with `report` whose `title` equals `Localizer::new(Some("de")).tr("local-agent-invalid-call")` (prompt locale `de`: a non-English locale, asserted by key, not English text).
- `run_turn_stops_after_max_calls` — 9 `apply` calls scripted → `Failed(ToolLimit)`, 8 `apply` calls made.
- `run_turn_empty_route_fails_after_retry` — route `"nothing"` twice → `Failed(NoRoute)`.
- `run_turn_cancel_ends_without_report` — `hang: true`, token cancelled after 10 ms → `Cancelled`, no `report` call.

Make `TurnEnd` and `FailKind` derive `Debug, PartialEq, Eq`.

- [ ] **Step 2: Run them to see them fail** — `cargo nextest run -p oku-slides-local-agent run_turn` → compile errors.

- [ ] **Step 3: Implement** — `turn.rs` per the algorithm (keep it under 300 lines: one function per step: `route`, `act`, `fail`). `mcp.rs`: `McpTools::connect` builds `StreamableTransportOptions` with the `Authorization` header and `InitializeRequestParams` exactly as `fake_agent.rs` `call_tool` does (client name `oku-slides-local-agent`), `start()` once; `list` = `request_tool_list(None)` mapped to `ToolSpec { name, description: description.unwrap_or_default(), schema: serde_json::to_value(input_schema) }`; `call` = `request_tool_call(CallToolRequestParams::new(name).with_arguments(obj))`, joining the text content blocks; `is_error == Some(true)` → `Err(text)`. Check `request_tool_list`'s exact parameter type in `rust-mcp-sdk-1.1.0/src/mcp_traits/mcp_client.rs:363`. `acp.rs`: the prompt handler reads the prompt (`read_prompt`), gets or connects the session's `McpTools`, and spawns `run_turn` with `cx.spawn` as `fake_agent.rs` does, then responds.

Fluent keys, appended after `agent-local-no-program` in `en/agent.ftl` then translated in the 19 other locales:

```ftl
local-agent-no-route = The local agent did not understand what to do. Try saying it another way.
local-agent-tool-limit = The local agent stopped after { $n } steps without finishing.
local-agent-invalid-call = The local agent could not use the tools correctly.
local-agent-engine-failed = The local agent's model failed.
```

French: `L'agent local n'a pas compris quoi faire. Essayez de le dire autrement.` / `L'agent local s'est arrêté après { $n } étapes sans finir.` / `L'agent local n'a pas su utiliser les outils.` / `Le modèle de l'agent local a échoué.`

- [ ] **Step 4: Run the tests** — `cargo nextest run -p oku-slides-local-agent -p oku-slides-i18n` → PASS.

- [ ] **Step 5: Commit** — `feat(local-agent) : tour routé, boucle d'outils bornée et reçu (t-1020)`; update the crate `ARCHITECTURE.md` (modules `mcp`, `turn`, flow).

---

### Task 7: End-to-end round trip through the host

**Files:**
- Create: `crates/oku-slides-local-agent/tests/roundtrip.rs`, `tests/common/mod.rs`

**Interfaces:**
- Consumes: everything above; from `oku-slides-agent`: `Launch`, `Link`, `AgentHandle`, `Keeper`, `ToolsAccess`, `AgentEvent`; from `oku-slides-mcp`: `SurfaceHub`, `Tools`, `ToolServer`, `SettingsPort`, `Applied`; from `oku-slides-session`: `TabSession`, `Session`, `Fonts`, `Platform`, `Settings`.
- Produces: nothing for later tasks.

- [ ] **Step 1: Write the test** — `tests/common/mod.rs`: a one-tab harness copied from `crates/oku-slides-agent/tests/common/mod.rs` (`test_dir`, `session`, the tab start in `start_tabs_ready_within`, `until`), reduced to one tab, with:

```rust
let launch = Launch {
    command: env!("CARGO_BIN_EXE_oku-slides-local-agent-dev").to_owned(),
    args: vec!["--local-agent".into(), "--engine".into(), "fake".into(),
               "--script".into(), script.to_string_lossy().into_owned(),
               "--log".into(), log.to_string_lossy().into_owned()],
    fallback_cwd: dir.path().to_owned(),
    ready_within: oku_slides_agent::READY_WITHIN,
};
```

`tests/roundtrip.rs`:

```rust
//! L'hôte lance l'agent local, qui modifie le deck par MCP et finit par un reçu.

mod common;

use oku_slides_agent::AgentEvent;
use serde_json::json;
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn roundtrip_local_agent_edits_deck_and_reports() {
    let h = common::start(json!({"turns":[{"route":"edit","replies":[
        {"call":"apply","args":{"ops":[{"op":"set_deck_param","name":"title","value":"Feuille de route 2027"}]}},
        {"call":"report","args":{"title":"Titre changé"}}]}]})).await;
    h.tab.agent.prompt("renomme le deck".into());
    let mut rx = h.tab.events.lock().await;
    common::until(&mut rx, |e| matches!(e, AgentEvent::TurnEnded { .. })).await;
    let title = h.tab.session.lock().unwrap().deck().title().to_owned();
    assert_that!(title.as_str()).is_equal_to("Feuille de route 2027");
}
```

Adapt three things to the real APIs before running, by reading `crates/oku-slides-agent/tests/stack.rs` and the MCP `apply` schema (`crates/oku-slides-mcp/src/edit_tools.rs`): how a prompt is sent to an `AgentHandle`, the exact `set_deck_param` op shape, and how a test reads the deck title from `Session`. Add a second test, `roundtrip_report_shows_in_the_turn`, that waits for the turn surface carrying a receipt titled `Titre changé` (copy how `crates/oku-slides-agent/tests/turns.rs` finds a receipt).

- [ ] **Step 2: Run them** — `cargo nextest run -p oku-slides-local-agent roundtrip`
Expected: PASS. If it fails, debug with the fake engine's `--log` file and the host's tracing (`RUST_LOG=debug`), fix the cause in the earlier task's code with a unit test first.

- [ ] **Step 3: Full gate** — `just check` then `just test`
Expected: all green (fmt, clippy `-D warnings`, tsc, agents-lint, nextest, UI tests).

- [ ] **Step 4: Commit** — `test(local-agent) : aller-retour hôte, agent local et outils MCP (t-1020)`.

---

## After the plan

- Mark t-1020's sub-tasks done one by one as each task lands; t-1020 is done when Task 7 is green.
- Lot 2 (t-1021, `diagnose`) is independent and can run in parallel; lot 3 (t-1022) starts from this branch's `Engine` trait and adds the default `[agent.presets.local]`.
