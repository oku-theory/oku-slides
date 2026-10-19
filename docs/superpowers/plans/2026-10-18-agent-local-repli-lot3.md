# Local fallback agent, lot 3 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The built-in "Local" agent runs a real model: llama.cpp (Metal, Apple silicon) on a pinned
Qwen3-4B-Instruct-2507 Q4_K_M downloaded on demand from the agent panel, with honest availability
states, and the follow-ups of the lot 1 review.

**Architecture:** `oku-slides-platform` gains the shared pinned-file download loop (extracted from
dictation, SHA-256 checked) and the `[local_agent]` settings with `local_engine`, the one function
that decides whether the local agent can run. `oku-slides-local-agent` gains a pure ChatML renderer
and `<tool_call>` parser plus a thin `gguf` engine on a dedicated llama.cpp thread, and reads its
engine from the settings. `oku-slides-agent` reports the engine state in `availability`, offers the
download in the picker surface, and the UI forwards the two new picker actions.

**Tech Stack:** Rust 2024, `llama-cpp-2 =0.1.158` (Metal, `common`), `ureq` 3, `sha2`, `hex`,
`tiny_http` (tests), Snafu, Fluent, speculoos, rstest, nextest; React/TS (Bun) for the picker actions.

**Spec:** `docs/superpowers/specs/2026-10-18-agent-local-repli-design.md`, section
"Lot 3 design (t-1022)" (approved 2026-10-18), plus "Engines" and "Preset and offer".
Audit evidence: `llama-cpp-2-audit.md` in the shared notes folder of the t-1009 sessions; a working
llama.cpp probe is in this session's scratchpad (`llama-audit/probe/src/main.rs`).

**Tasks:** parent t-1022; Task 1 to Task 8 = t-1044 to t-1051, titles verbatim.

## Global Constraints

- Every commit: `type(scope) : description (t-N)` (space before the colon), message in French,
  citing the step's sub-task and t-1022, no `Co-Authored-By` line. Implementers stage only and
  write the message to the workspace file given in their brief; the controller commits with
  `git commit -F <file>` and a `Review-waived:` line naming the SDD review (`oku.review=deferred`).
- No `unwrap`/`expect` outside tests; no silent fallback; Snafu errors with a `backtrace` in
  every variant; opaque public error types; messages shown to users go through Fluent.
- No `std::fs` in production code (use `Storage`); `std::fs` is fine in tests and in the
  debug-only fake engine.
- Never hold a `std::sync::Mutex` guard across an `.await`; lock with
  `lock().unwrap_or_else(PoisonError::into_inner)`. Blocking work (download, llama.cpp) runs on
  its own thread, never on a runtime thread.
- Nothing in `oku-slides-local-agent` writes to stdout (ACP pipe); llama.cpp logs go to `tracing`.
- Every `pub` item and module documented, first sentence under 15 words; source files ≤ 500
  lines (hard cap 1000).
- Unit tests in sibling `x_tests.rs` files included by `#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`,
  speculoos assertions, names `unit_scenario_result`, rstest for tables. Localized errors are
  tested on the key or variant plus at least one non-English locale.
- Dependency versions only in root `[workspace.dependencies]`, crates use `.workspace = true`.
  `llama-cpp-2` is declared only under
  `[target.'cfg(all(target_os = "macos", target_arch = "aarch64"))'.dependencies]`.
- New Fluent keys: English first, then all 20 locales, same key order, real translations, no space
  between a number and a CJK counter, Fluent variables unchanged.
- Each `ARCHITECTURE.md` ≤ 50 lines and each `AGENTS.md` updated in the commit that changes what
  it describes (`just agents-lint`).
- Tests that start the app or the agent use a fresh `tempfile` dir for `OKU_SLIDES_PATHS__DATA`
  and `OKU_SLIDES_PATHS__DECKS`. The real 2.5 GB download never runs in CI: fake `tiny_http` hub
  with small files.
- Pinned model (copy verbatim): repo `unsloth/Qwen3-4B-Instruct-2507-GGUF`, revision
  `a06e946bb6b655725eafa393f4a9745d460374c9`, file `Qwen3-4B-Instruct-2507-Q4_K_M.gguf`,
  2497281120 bytes, SHA-256 `3605803b982cb64aead44f6c1b2ae36e3acdb41d8e46c8a94c6533bc4c67e597`,
  stored in `<data>/models/agent/`.
- Gate: `just check` and `just test` green at the end.

## Deviations from the spec

None beyond those recorded in "Lot 3 design (t-1022)". Two implementation choices the spec leaves
open, decided here:

1. `Launch` gains `env: Vec<(String, String)>` (extra variables for the agent process): the only way
   for the host-level test harness to inject `OKU_SLIDES_LOCAL_AGENT__ENGINE=fake` without mutating
   the test process's environment (`set_var` is `unsafe` in edition 2024 and `unsafe_code` is denied).
   Production presets leave it empty.
2. The agent's choice, availability and picker move from `Stack` into a shared `AgentChoice`
   (`choice.rs`, held in an `Arc`), so the download thread can re-emit the picker and the
   availability without a reference to the whole `Stack`.

## File map

| File | Responsibility |
|---|---|
| `crates/oku-slides-platform/src/models.rs` (create) | `PinnedFile`, `COMPLETE`, `is_complete`, `download` (feature `download`), `DownloadError`, `DownloadErrorKind` |
| `crates/oku-slides-platform/src/local_agent.rs` (create) | `AGENT_MODEL`, `AGENT_MODEL_SIZE`, `agent_model_dir`, `Target`, `ReadyEngine`, `Unavailable`, `resolve`, `local_engine` |
| `crates/oku-slides-platform/src/config.rs`, `config.default.toml` (modify) | `[models]` (`ModelsSettings`), `[local_agent]` (`LocalAgentSettings`, `LocalEngine`), `[agent.presets.local]` |
| `crates/oku-slides-stt/src/models.rs`, `error.rs`, `lib.rs` (modify) | Whisper downloads through the platform loop |
| `crates/oku-slides-local-agent/src/tool_error.rs` (create) | `ToolError`: rejected (repairable) vs transport |
| `crates/oku-slides-local-agent/src/chatml.rs` (create) | `render(&Completion) -> String` (Qwen ChatML with `<tools>`) |
| `crates/oku-slides-local-agent/src/reply.rs` (create) | `parse(&str) -> Reply` (`<tool_call>` JSON or text) |
| `crates/oku-slides-local-agent/src/gguf.rs` (create) | `GgufEngine` on a llama.cpp worker thread (macOS aarch64 only) |
| `crates/oku-slides-local-agent/src/entry.rs`, `engine.rs`, `turn.rs`, `mcp.rs`, `sessions.rs`, `families.rs` (modify) | engine from settings, `Message::Call`, `Reply::Invalid`, Snafu tool errors, route grammar |
| `crates/oku-slides-agent/src/choice.rs` (create) | `AgentChoice`: active preset, availability, picker surface, select |
| `crates/oku-slides-agent/src/model_download.rs` (create) | `ModelDownload`: one cancellable download thread, its state |
| `crates/oku-slides-agent/src/{stack,picker,error,config,link}.rs` (modify) | `DownloadModel`/`CancelDownload` commands, new `Unusable` variants, `Launch.env` |
| `ui/src/api.ts`, `ui/src/agent/{AgentPicker,useAgentPicker,AgentPanel}.tsx/ts` (modify) | the two picker actions |
| `ui/e2e/210-agent-picker.cjs` (modify) | choosing "Local" shows the download button |
| `crates/oku-slides-i18n/src/locales/*/agent.ftl` (modify) | new `agent-local-*` and `agent-model-*` keys |

---

### Task 1: Pinned model download in the platform

**Files:**
- Create: `crates/oku-slides-platform/src/models.rs`, `crates/oku-slides-platform/src/models_tests.rs`
- Modify: `crates/oku-slides-platform/src/lib.rs` (module + re-exports), `config.rs` (`ModelsSettings`, `Settings.models`, remove the three keys from `SttSettings`), `config.default.toml` (`[models]`), `Cargo.toml` (feature `download`, dev-dep `tiny_http`), `ARCHITECTURE.md`
- Modify: root `Cargo.toml` (nothing new: `ureq`, `sha2`, `hex`, `tiny_http` already declared)
- Modify: `crates/oku-slides-stt/src/models.rs`, `error.rs`, `lib.rs`, `Cargo.toml` (enable `oku-slides-platform/download`), `tests/dictation.rs`, `ARCHITECTURE.md`

**Interfaces:**
- Produces (in `oku_slides_platform::models`, re-exported at the crate root as `models::…` only — no glob):
  ```rust
  pub const COMPLETE: &str = ".complete";
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct PinnedFile { pub repo: &'static str, pub revision: &'static str, pub name: &'static str, pub sha256: Option<&'static str> }
  pub fn is_complete(storage: &Storage, dir: &Path) -> bool;
  #[cfg(feature = "download")]
  pub fn download(files: &[PinnedFile], dir: &Path, total: u64, settings: &ModelsSettings, storage: &Storage,
                  progress: impl FnMut(u64, u64) -> bool) -> Result<(), DownloadError>;
  #[cfg(feature = "download")] pub struct DownloadError(/* opaque */);
  #[cfg(feature = "download")] impl DownloadError { pub fn kind(&self) -> DownloadErrorKind }
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum DownloadErrorKind { Network, Write, Incomplete { got: u64, expected: u64 }, Integrity, Cancelled }
  ```
- Produces (config): `pub struct ModelsSettings { pub hub_url: String, pub download_retries: u32, pub download_backoff_secs: u64 }`, `Settings.models: ModelsSettings`.
- Consumes: `Storage::{writer, len, rename, delete, write, exists}` (existing).

- [ ] **Step 1: Write the failing tests** in `models_tests.rs` (`#![cfg(feature = "download")]` at the top; run with `--features download`). A fake hub serves each request from a map `name -> (status, body)`:

```rust
//! Téléchargement épinglé contre un faux dépôt local.
#![cfg(feature = "download")]

use std::collections::HashMap;
use std::path::Path;

use sha2::{Digest, Sha256};
use speculoos::prelude::*;

use super::{COMPLETE, DownloadErrorKind, PinnedFile, download, is_complete};
use crate::config::ModelsSettings;
use crate::storage::Storage;

/// Un faux dépôt : `/repo/resolve/rev/<nom>` rend le corps de `<nom>` ; les autres URL, 404.
fn hub(files: HashMap<&'static str, &'static [u8]>) -> String {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    std::thread::spawn(move || {
        for req in server.incoming_requests() {
            let name = req.url().rsplit('/').next().unwrap_or_default().to_owned();
            let ok = req.url().contains("/repo/resolve/rev/");
            let resp = match files.get(name.as_str()) {
                Some(body) if ok => tiny_http::Response::from_data(body.to_vec()),
                _ => tiny_http::Response::from_data(Vec::new()).with_status_code(404),
            };
            let _ = req.respond(resp);
        }
    });
    url
}

fn settings(hub_url: String) -> ModelsSettings {
    ModelsSettings { hub_url, download_retries: 0, download_backoff_secs: 0 }
}

fn sha(bytes: &[u8]) -> &'static str {
    Box::leak(hex::encode(Sha256::digest(bytes)).into_boxed_str())
}

fn file(name: &'static str, sha256: Option<&'static str>) -> PinnedFile {
    PinnedFile { repo: "repo", revision: "rev", name, sha256 }
}

#[test]
fn download_matching_sha_writes_file_and_marker() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::local();
    let url = hub(HashMap::from([("a.bin", b"hello".as_slice())]));
    let files = [file("a.bin", Some(sha(b"hello")))];
    download(&files, dir.path(), 5, &settings(url), &storage, |_, _| true).unwrap();
    assert_that!(std::fs::read(dir.path().join("a.bin")).unwrap()).is_equal_to(b"hello".to_vec());
    assert_that!(is_complete(&storage, dir.path())).is_true();
}

#[test]
fn download_sha_mismatch_deletes_file_and_fails_integrity() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::local();
    let url = hub(HashMap::from([("a.bin", b"tampered".as_slice())]));
    let files = [file("a.bin", Some(sha(b"hello")))];
    let err = download(&files, dir.path(), 8, &settings(url), &storage, |_, _| true).unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Integrity);
    assert_that!(dir.path().join("a.bin").exists()).is_false();
    assert_that!(dir.path().join("a.bin.part").exists()).is_false();
    assert_that!(dir.path().join(COMPLETE).exists()).is_false();
}

#[test]
fn download_progress_false_cancels_and_removes_part() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::local();
    let url = hub(HashMap::from([("a.bin", b"hello".as_slice())]));
    let err = download(&[file("a.bin", None)], dir.path(), 5, &settings(url), &storage, |_, _| false).unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Cancelled);
    assert_that!(dir.path().join("a.bin.part").exists()).is_false();
}

#[test]
fn download_existing_file_is_kept_and_not_fetched() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::local();
    std::fs::write(dir.path().join("a.bin"), b"hello").unwrap();
    // Le dépôt ne sert que b.bin : a.bin, déjà là, ne doit pas être redemandé.
    let url = hub(HashMap::from([("b.bin", b"world".as_slice())]));
    let mut seen = Vec::new();
    download(&[file("a.bin", None), file("b.bin", None)], dir.path(), 10, &settings(url), &storage, |r, t| {
        seen.push((r, t));
        true
    })
    .unwrap();
    assert_that!(seen.last().copied()).is_equal_to(Some((10, 10)));
}

#[test]
fn download_http_error_is_network() {
    let dir = tempfile::tempdir().unwrap();
    let url = hub(HashMap::new());
    let err = download(&[file("a.bin", None)], dir.path(), 5, &settings(url), &Storage::local(), |_, _| true)
        .unwrap_err();
    assert_that!(err.kind()).is_equal_to(DownloadErrorKind::Network);
}

#[test]
fn is_complete_without_marker_is_false() {
    let dir = tempfile::tempdir().unwrap();
    assert_that!(is_complete(&Storage::local(), dir.path())).is_false();
}
```

  Use the existing local `Storage` constructor (check `storage.rs`: the tests of `oku-slides-stt` build one; reuse that call instead of `Storage::local()` if its name differs). Path assertions use `std::fs` (tests only).

- [ ] **Step 2: Run to see them fail**

Run: `cargo nextest run -p oku-slides-platform --features download models`
Expected: compile error (`models` module missing).

- [ ] **Step 3: Implement `models.rs`** by moving the loop of `crates/oku-slides-stt/src/models.rs:98-161` and adding the hash:

```rust
//! Fichiers de modèles épinglés (dépôt, révision, SHA-256), téléchargés à la demande.
//!
//! Chaque fichier arrive en `.part`, haché au fil de l'eau, puis renommé ; le marqueur
//! [`COMPLETE`] n'est écrit qu'une fois tous les fichiers vérifiés. Octets non fiables :
//! un SHA-256 différent supprime le fichier.

use std::path::Path;

use crate::storage::Storage;

/// Marqueur écrit quand tous les fichiers d'un modèle sont là et vérifiés.
pub const COMPLETE: &str = ".complete";

/// Un fichier d'un dépôt Hugging Face, à une révision donnée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinnedFile {
    /// Dépôt, `organisation/nom`.
    pub repo: &'static str,
    /// Commit du dépôt (ou `main` pour un fichier non épinglé).
    pub revision: &'static str,
    /// Nom du fichier dans le dépôt et sur le disque.
    pub name: &'static str,
    /// SHA-256 attendu, en hexadécimal minuscule ; `None` : non vérifié.
    pub sha256: Option<&'static str>,
}

/// Le modèle rangé dans `dir` est-il complet (marqueur présent) ?
pub fn is_complete(storage: &Storage, dir: &Path) -> bool {
    storage.exists(&dir.join(COMPLETE)).unwrap_or(false)
}
```

  `is_complete` keeps the `unwrap_or(false)` of `Model::is_installed` (a storage error reads as "not installed", which only offers the download again; log it with `tracing::warn!` instead of hiding it: match the `Result`, warn on `Err`, return `false`).

  Then, under `#[cfg(feature = "download")]`, `download` and its errors (put them in `models/download.rs` if `models.rs` passes 300 lines):

```rust
#[derive(Debug, Snafu)]
pub struct DownloadError(DownloadInner);

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
enum DownloadInner {
    #[snafu(display("cannot fetch {file}"))]
    Http { file: String, source: Box<ureq::Error>, backtrace: Backtrace },
    #[snafu(display("cannot read {file} from the network"))]
    HttpRead { file: String, source: std::io::Error, backtrace: Backtrace },
    #[snafu(display("cannot write {}", path.display()))]
    Write { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("incomplete download: {got} of {expected} bytes"))]
    Incomplete { got: u64, expected: u64, backtrace: Backtrace },
    #[snafu(display("{file} does not match its pinned SHA-256"))]
    Integrity { file: String, backtrace: Backtrace },
    #[snafu(display("download cancelled"))]
    Cancelled { backtrace: Backtrace },
}

impl DownloadError {
    /// Ce qui a échoué, pour le message montré à l'utilisateur.
    pub fn kind(&self) -> DownloadErrorKind { /* one arm per variant; Http and HttpRead are Network */ }
}
```

  The loop is the stt one with these changes: URL `{hub}/{repo}/resolve/{revision}/{name}`; a
  `sha2::Sha256` hasher updated with every chunk when `sha256` is `Some`; after `out.close()` and the
  length check, compare `hex::encode(hasher.finalize())` with the pin — on mismatch `discard` the
  `.part` and return `Integrity`; then rename. `settings` is `&ModelsSettings`. The marker is
  written after the last file, then `progress(total, total)`.

  `Cargo.toml` of the platform:
  ```toml
  [features]
  # Téléchargement des modèles épinglés (dictée, agent local) : client HTTP et SHA-256.
  download = ["dep:ureq", "dep:sha2", "dep:hex"]
  [dependencies]
  ureq = { workspace = true, optional = true }
  sha2 = { workspace = true, optional = true }
  hex = { workspace = true, optional = true }
  [dev-dependencies]
  tiny_http.workspace = true
  sha2.workspace = true
  hex.workspace = true
  ```
  Make `just test` run these tests: check how nextest is invoked in the `justfile` (`--workspace`
  unifies features, so the stt crate enabling `download` turns it on for the platform tests too;
  confirm by running `just test` filtered on `models`).

- [ ] **Step 4: Settings `[models]`.** In `config.rs` add `ModelsSettings` (`#[derive(Debug, Clone, Deserialize)] #[non_exhaustive]`, documented fields) and `Settings.models`; remove `hub_url`, `download_retries`, `download_backoff_secs` from `SttSettings`. In `config.default.toml`, remove them from `[stt]` and add before `[agent]`:

```toml
[models]
# Dépôt des modèles téléchargés (dictée, agent local), tentatives et attente
# entre deux tentatives (s).
hub_url = "https://huggingface.co"
download_retries = 5
download_backoff_secs = 4
```

  Add a config test `settings_defaults_models_hub_is_hugging_face` asserting `settings.models.hub_url == "https://huggingface.co"`.

- [ ] **Step 5: Move stt onto it.** `oku-slides-stt/Cargo.toml`: `oku-slides-platform = { workspace = true, features = ["download"] }`; drop `ureq` if no other use remains (`rg ureq crates/oku-slides-stt`).
  `Model::files` returns `Vec<PinnedFile>` (revision `"main"`, `sha256: None`; the `base` preprocessor keeps repo `openai/whisper-base`). `Model::is_installed` calls `models::is_complete(storage, &self.path(dir))` (Apple stays `false`). `stt::download(model, dir, settings: &ModelsSettings, storage, progress)` calls `oku_slides_platform::models::download(&model.files(), &dest, model.size(), …)` and returns `Ok(dest)`.
  In `error.rs`, replace the variants that only the download produced (`Http`, `HttpRead`, `Incomplete`, `Cancelled` if unused elsewhere — check with `rg`) with `Download { source: DownloadError, backtrace: Backtrace }`; keep `ModelWrite` if `AssetInstall` or others still use it. Localize `Download` by `source.kind()`: `Cancelled` → `stt-cancelled`, `Incomplete { got, expected }` → `stt-download-incomplete`, others → `stt-download` (keys unchanged, no new Fluent text). `SttErrorKind`: `Cancelled` → `Cancelled`, others → the kind the old `Http` arm had. The `Stt` struct gets the `ModelsSettings` from `platform.settings.models` where it calls `download`. `tests/dictation.rs`: `settings.models.hub_url` / `settings.models.download_backoff_secs`.
  Update `error_tests.rs` of stt if it builds the removed variants.

- [ ] **Step 6: Run** `cargo nextest run -p oku-slides-platform --features download` and `cargo nextest run -p oku-slides-stt`. Expected: PASS (the existing dictation download tests prove Whisper still downloads).

- [ ] **Step 7: Docs.** Platform `ARCHITECTURE.md`: `models` bullet (feature `download`), `[models]` in the config bullet. stt `ARCHITECTURE.md`: Whisper models downloaded through `oku_slides_platform::models`. Add to `crates/oku-slides-platform/AGENTS.md` (create it if absent, with the standard header) the invariant: `- Download model files only through models::download, which checks pinned SHA-256 and writes .complete last. (plan:2026-10-18-agent-local-repli-lot3)`; add the file to the root harness map if created.

- [ ] **Step 8: Stage and write the commit message** `refactor(platform) : téléchargement des modèles épinglés partagé, SHA-256 vérifié (t-1044, t-1022)`.

---

### Task 2: Local agent settings and engine resolution

**Files:**
- Create: `crates/oku-slides-platform/src/local_agent.rs`, `local_agent_tests.rs`
- Modify: `crates/oku-slides-platform/src/config.rs` (`LocalAgentSettings`, `LocalEngine`, `Settings.local_agent`), `config.default.toml` (`[local_agent]`), `lib.rs`, `ARCHITECTURE.md`

**Interfaces:**
- Consumes: `models::{PinnedFile, is_complete}` (Task 1), `PathSettings::models()`.
- Produces (`oku_slides_platform::local_agent`):
  ```rust
  pub const AGENT_MODEL: PinnedFile;          // the pin of Global Constraints
  pub const AGENT_MODEL_SIZE: u64 = 2_497_281_120;
  pub const AGENT_MODEL_FILES: [PinnedFile; 1] = [AGENT_MODEL];   // the 'static slice ModelDownload takes
  pub fn agent_model_dir(settings: &Settings) -> PathBuf;   // <models>/agent
  #[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Target { AppleSilicon, IntelMac, Other }
  impl Target { pub fn current() -> Self }
  #[derive(Debug, Clone, PartialEq, Eq)] pub enum ReadyEngine { Gguf { model: PathBuf }, #[cfg(debug_assertions)] Fake }
  #[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Unavailable { IntelMac, UnsupportedOs, NeedsDownload { size: u64 } }
  pub fn resolve(engine: LocalEngine, target: Target, installed: bool, model: PathBuf) -> Result<ReadyEngine, Unavailable>;
  pub fn local_engine(settings: &Settings, storage: &Storage) -> Result<ReadyEngine, Unavailable>;
  ```
  (config) `#[serde(rename_all = "lowercase")] pub enum LocalEngine { #[default] Auto, Gguf, #[cfg(debug_assertions)] Fake }`,
  `pub struct LocalAgentSettings { pub engine: LocalEngine, pub context: u32, pub temperature: f32, pub max_reply_tokens: u32, pub fake_script: Option<PathBuf>, pub fake_log: Option<PathBuf> }`, `Settings.local_agent`.

- [ ] **Step 1: Failing tests** (`local_agent_tests.rs`):

```rust
use std::path::PathBuf;

use rstest::rstest;
use speculoos::prelude::*;

use super::{AGENT_MODEL, AGENT_MODEL_SIZE, ReadyEngine, Target, Unavailable, resolve};
use crate::config::LocalEngine;

fn model() -> PathBuf {
    PathBuf::from("/data/models/agent/Qwen3-4B-Instruct-2507-Q4_K_M.gguf")
}

#[rstest]
#[case::auto_ready(LocalEngine::Auto, Target::AppleSilicon, true, Ok(ReadyEngine::Gguf { model: model() }))]
#[case::gguf_ready(LocalEngine::Gguf, Target::AppleSilicon, true, Ok(ReadyEngine::Gguf { model: model() }))]
#[case::needs_download(LocalEngine::Auto, Target::AppleSilicon, false, Err(Unavailable::NeedsDownload { size: AGENT_MODEL_SIZE }))]
#[case::intel(LocalEngine::Gguf, Target::IntelMac, true, Err(Unavailable::IntelMac))]
#[case::intel_auto(LocalEngine::Auto, Target::IntelMac, false, Err(Unavailable::IntelMac))]
#[case::other_os(LocalEngine::Auto, Target::Other, false, Err(Unavailable::UnsupportedOs))]
fn resolve_engine_and_target_gives_state(
    #[case] engine: LocalEngine,
    #[case] target: Target,
    #[case] installed: bool,
    #[case] expected: Result<ReadyEngine, Unavailable>,
) {
    assert_that!(resolve(engine, target, installed, model())).is_equal_to(expected);
}

#[cfg(debug_assertions)]
#[test]
fn resolve_fake_is_ready_anywhere() {
    assert_that!(resolve(LocalEngine::Fake, Target::Other, false, model())).is_equal_to(Ok(ReadyEngine::Fake));
}

#[test]
fn agent_model_pin_is_the_audited_file() {
    assert_that!(AGENT_MODEL.revision).is_equal_to("a06e946bb6b655725eafa393f4a9745d460374c9");
    assert_that!(AGENT_MODEL.sha256).is_equal_to(Some("3605803b982cb64aead44f6c1b2ae36e3acdb41d8e46c8a94c6533bc4c67e597"));
}

#[test]
fn target_current_on_this_machine_matches_cfg() {
    let expected = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Target::AppleSilicon
    } else if cfg!(target_os = "macos") {
        Target::IntelMac
    } else {
        Target::Other
    };
    assert_that!(Target::current()).is_equal_to(expected);
}
```

  Plus in `config_tests.rs`: `settings_defaults_local_agent_engine_is_auto` (and `context == 16384`), and `#[cfg(debug_assertions)] local_agent_fake_engine_loads_from_toml` (`[local_agent]\nengine = "fake"\nfake_script = "s.json"` → `LocalEngine::Fake`, `fake_script == Some("s.json")`).

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-platform local_agent` → compile error.

- [ ] **Step 3: Implement.** `resolve`:

```rust
pub fn resolve(engine: LocalEngine, target: Target, installed: bool, model: PathBuf) -> Result<ReadyEngine, Unavailable> {
    match engine {
        #[cfg(debug_assertions)]
        LocalEngine::Fake => Ok(ReadyEngine::Fake),
        // Lot 4 ajoute Apple : `auto` le préférera quand il est disponible.
        LocalEngine::Auto | LocalEngine::Gguf => match target {
            Target::IntelMac => Err(Unavailable::IntelMac),
            Target::Other => Err(Unavailable::UnsupportedOs),
            Target::AppleSilicon if !installed => Err(Unavailable::NeedsDownload { size: AGENT_MODEL_SIZE }),
            Target::AppleSilicon => Ok(ReadyEngine::Gguf { model }),
        },
    }
}

pub fn local_engine(settings: &Settings, storage: &Storage) -> Result<ReadyEngine, Unavailable> {
    let dir = agent_model_dir(settings);
    let installed = crate::models::is_complete(storage, &dir);
    resolve(settings.local_agent.engine, Target::current(), installed, dir.join(AGENT_MODEL.name))
}
```

  `Target::current()` with `cfg!` as in the test. `config.default.toml`, after `[agent.presets.gemini]`:

```toml
[local_agent]
# Moteur de l'agent local de repli : "auto" (le meilleur disponible) ou "gguf"
# (llama.cpp sur Apple silicon, modèle Qwen3 4B téléchargé depuis le panneau de l'agent).
engine = "auto"
# Contexte du modèle (jetons), température, longueur maximale d'une réponse (jetons).
context = 16384
temperature = 0.3
max_reply_tokens = 1024
# Tests (builds de débogage) : engine = "fake" rejoue ce script et journalise chaque appel.
# fake_script = "script.json"
# fake_log = "log.jsonl"
```

- [ ] **Step 4: Run** the platform tests → PASS. Update platform `ARCHITECTURE.md` (`local_agent` module, `[local_agent]`).

- [ ] **Step 5: Stage, commit message** `feat(platform) : réglages de l'agent local et état de son moteur, modèle Qwen3 épinglé (t-1045, t-1022)`.

---

### Task 3: Tool errors and turn follow-ups of the lot 1 review

**Files:**
- Create: `crates/oku-slides-local-agent/src/tool_error.rs`
- Modify: `src/turn.rs`, `src/mcp.rs`, `src/sessions.rs`, `src/acp.rs` (call site of `tools()`), `src/turn_tests.rs`, `src/lib.rs`, `Cargo.toml` (tokio features), `ARCHITECTURE.md`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Snafu)] pub(crate) struct ToolError(ToolInner);
  #[derive(Debug, Snafu)] #[snafu(visibility(pub(crate)))]
  pub(crate) enum ToolInner {
      /// Le serveur refuse l'appel (résultat en erreur, erreur JSON-RPC, outil inconnu, arguments) : le modèle peut le réparer.
      #[snafu(display("{message}"))] Rejected { message: String, backtrace: Backtrace },
      /// La connexion au serveur d'outils a échoué : le tour échoue.
      #[snafu(display("tool server unreachable"))]
      Transport { #[snafu(source(from(rust_mcp_sdk::error::McpSdkError, Box::new)))] source: Box<rust_mcp_sdk::error::McpSdkError>, backtrace: Backtrace },
      /// Un schéma d'outil n'est pas du JSON valide.
      #[snafu(display("invalid schema for tool {name}"))] Schema { name: String, source: serde_json::Error, backtrace: Backtrace },
  }
  impl ToolError { pub(crate) fn is_rejected(&self) -> bool; pub(crate) fn rejected(message: impl Into<String>) -> Self }
  impl From<rust_mcp_sdk::error::McpSdkError> for ToolError // RpcError → Rejected { message: rpc message }, others → Transport
  trait ToolPort { async fn list(&self) -> Result<Vec<ToolSpec>, ToolError>; async fn call(&self, name: &str, args: &Value) -> Result<String, ToolError>; }
  McpTools::connect(&McpAccess) -> Result<Self, ToolError>; SessionState::tools() -> Result<&McpTools, ToolError>
  ```

- [ ] **Step 1: Failing turn tests** in `turn_tests.rs` (extend `FakeTools`: `list_fails: bool`, `transport_on: Option<String>`; their `list`/`call` return `ToolError` — adapt the existing `failing` arm to `ToolError::rejected(...)`):

```rust
#[tokio::test]
async fn run_turn_tools_list_failure_fails_with_engine() {
    let tools = FakeTools { list_fails: true, ..FakeTools::default() };
    let engine = fake(json!({"turns": [{"route": "edit", "replies": []}]}));
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("fr"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Failed(FailKind::Engine));
}

#[tokio::test]
async fn run_turn_transport_error_fails_with_engine_not_repair() {
    let tools = FakeTools { transport_on: Some("apply".into()), ..FakeTools::default() };
    let engine = fake(json!({"turns": [{"route": "edit", "replies": [{"call": "apply", "args": {}}]}]}));
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("fr"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Failed(FailKind::Engine));
}

#[tokio::test]
async fn run_turn_success_between_failures_resets_repairs() {
    // apply échoue, save réussit, apply échoue encore, puis report : jamais deux échecs de suite.
    let tools = FakeTools::failing("apply");
    let replies = json!([{"call": "apply"}, {"call": "save"}, {"call": "apply"}, {"call": "report", "args": {"title": "ok", "facts": ["f"]}}]);
    let engine = fake(json!({"turns": [{"route": "edit, files", "replies": replies}]}));
    let end = run_turn(&engine, &tools, &NoUpdates, &prompt("fr"), &[], &CancellationToken::new()).await;
    assert_that!(end).is_equal_to(TurnEnd::Reported);
}

#[tokio::test]
async fn run_turn_text_reply_becomes_a_note() {
    let updates = RecordingUpdates::default();
    let replies = json!([{"text": "Je regarde."}, {"call": "report", "args": {"title": "ok", "facts": ["f"]}}]);
    let engine = fake(json!({"turns": [{"route": "edit", "replies": replies}]}));
    run_turn(&engine, &FakeTools::default(), &updates, &prompt("fr"), &[], &CancellationToken::new()).await;
    assert_that!(updates.notes()).is_equal_to(vec!["Je regarde.".to_owned()]);
}
```

  Reuse the helpers that already exist in `turn_tests.rs` (`fake`, `prompt`, an `Updates` recorder); if a recorder of notes does not exist, add `RecordingUpdates` with a `Mutex<Vec<String>>` of notes. Check `families.rs` for the family of `save` (files) so the route `"edit, files"` exposes it.

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-local-agent turn` → compile errors / FAIL (transport counted as repair).

- [ ] **Step 3: Implement.** `tool_error.rs` as above. In `turn.rs`: an unknown tool gives `ToolError::rejected(format!("unknown tool {name}"))`; in `act`, `Err(e) if !e.is_rejected()` → `Err(Stop::Fail(FailKind::Engine))` after a `tracing::warn!(error = %snafu::Report::from_error(&e), …)`; `Err(e)` (rejected) keeps the repair path with `format!("error: {e}")`. Also check cancel before returning `InvalidCall` on the second failure (lot 1 minor: `self.check_cancel()?` first). `mcp.rs`: `connect` maps `start()` errors with `?` (`From<McpSdkError>`), `list` maps schema errors to `Schema`, `call` returns `ToolError::rejected` for non-object arguments and for `is_error` results. `sessions.rs` / `acp.rs`: carry `ToolError`; log with `snafu::Report`.

- [ ] **Step 4: Tokio features.** Remove features from `tokio = { workspace = true, features = [...] }` in `[dependencies]` one at a time while `cargo check -p oku-slides-local-agent --all-targets` stays green (and `cargo check -p oku-slides-local-agent --release`); keep only those needed.

- [ ] **Step 5: Run** `cargo nextest run -p oku-slides-local-agent` → PASS. Update `ARCHITECTURE.md` (`tool_error`, transport vs rejected).

- [ ] **Step 6: Stage, commit message** `fix(local-agent) : erreurs d'outils Snafu, transport MCP distinct d'un appel à réparer (t-1046, t-1022)`.

---

### Task 4: ChatML prompt and tool-call parsing

**Files:**
- Create: `crates/oku-slides-local-agent/src/chatml.rs`, `chatml_tests.rs`, `reply.rs`, `reply_tests.rs`
- Modify: `src/engine.rs` (`Message::Call`, `Reply::Invalid`), `src/turn.rs` (push `Message::Call` before each `ToolResult`; handle `Reply::Invalid` as a repair), `src/fake.rs` (nothing beyond compiling), `src/families.rs` (`route_grammar`), `src/families_tests.rs`, `src/turn_tests.rs`, `src/lib.rs`, `ARCHITECTURE.md`

**Interfaces:**
- Produces:
  ```rust
  // engine.rs
  enum Message { User(String), Assistant(String), Call { name: String, args: serde_json::Value }, ToolResult { name: String, output: String } }
  enum Reply { Text(String), Call { name: String, args: Value }, Invalid(String) }
  // chatml.rs
  pub(crate) fn render(c: &Completion) -> String;
  // reply.rs
  pub(crate) fn parse(text: &str) -> Reply;
  // families.rs
  pub(crate) fn route_grammar() -> String;   // GBNF, root = one or two family keys
  ```

- [ ] **Step 1: Failing tests.** `chatml_tests.rs`:

```rust
use serde_json::json;
use speculoos::prelude::*;

use super::render;
use crate::compact::ToolSpec;
use crate::engine::{Completion, Message, Step};

fn completion(messages: Vec<Message>, tools: Vec<ToolSpec>) -> Completion {
    Completion { step: Step::Act, system: "Be brief.".into(), messages, tools }
}

#[test]
fn render_without_tools_is_plain_chatml_ending_with_assistant() {
    let text = render(&completion(vec![Message::User("Hi".into())], vec![]));
    assert_that!(text).is_equal_to(
        "<|im_start|>system\nBe brief.<|im_end|>\n<|im_start|>user\nHi<|im_end|>\n<|im_start|>assistant\n".to_owned(),
    );
}

#[test]
fn render_tools_lists_each_tool_as_one_json_line() {
    let tool = ToolSpec { name: "save".into(), description: "Save the deck.".into(), schema: json!({"type": "object"}) };
    let text = render(&completion(vec![Message::User("Save".into())], vec![tool]));
    assert_that!(text).contains("<tools>\n{\"type\":\"function\",\"function\":{\"name\":\"save\",\"description\":\"Save the deck.\",\"parameters\":{\"type\":\"object\"}}}\n</tools>");
    assert_that!(text).contains("<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call>");
}

#[test]
fn render_call_and_result_use_tool_call_and_tool_response() {
    let messages = vec![
        Message::User("Save".into()),
        Message::Call { name: "save".into(), args: json!({}) },
        Message::ToolResult { name: "save".into(), output: "saved".into() },
    ];
    let text = render(&completion(messages, vec![]));
    assert_that!(text).contains("<|im_start|>assistant\n<tool_call>\n{\"name\":\"save\",\"arguments\":{}}\n</tool_call><|im_end|>\n");
    assert_that!(text).contains("<|im_start|>user\n<tool_response>\nsaved\n</tool_response><|im_end|>\n");
}
```

  `reply_tests.rs`:

```rust
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::parse;
use crate::engine::Reply;

#[rstest]
#[case::text("Done.", Reply::Text("Done.".into()))]
#[case::call("<tool_call>\n{\"name\": \"save\", \"arguments\": {\"x\": 1}}\n</tool_call>", Reply::Call { name: "save".into(), args: json!({"x": 1}) })]
#[case::call_without_args("<tool_call>{\"name\": \"save\"}</tool_call>", Reply::Call { name: "save".into(), args: json!({}) })]
#[case::unclosed("<tool_call>{\"name\": \"save\", \"arguments\": {}}", Reply::Call { name: "save".into(), args: json!({}) })]
#[case::text_before_call("Saving.\n<tool_call>{\"name\": \"save\", \"arguments\": {}}</tool_call>", Reply::Call { name: "save".into(), args: json!({}) })]
fn parse_reply_gives_text_or_call(#[case] text: &str, #[case] expected: Reply) {
    assert_that!(parse(text)).is_equal_to(expected);
}

#[rstest]
#[case::bad_json("<tool_call>{name: save}</tool_call>")]
#[case::no_name("<tool_call>{\"arguments\": {}}</tool_call>")]
#[case::args_not_object("<tool_call>{\"name\": \"save\", \"arguments\": [1]}</tool_call>")]
fn parse_malformed_call_is_invalid(#[case] text: &str) {
    assert_that!(matches!(parse(text), Reply::Invalid(_))).is_true();
}
```

  `families_tests.rs`: `route_grammar_names_every_routable_family` (each `Family::ROUTABLE` key appears quoted in the grammar, and it starts with `root ::=`). `turn_tests.rs`: `run_turn_invalid_reply_is_repaired_once` — a fake whose Act replies are scripted; since the fake script cannot produce `Invalid`, write a tiny test engine in the test (`struct Scripted(Mutex<VecDeque<Reply>>)` implementing `Engine`, route reply `Reply::Text("edit")`, then `Reply::Invalid("bad".into())`, then a valid `report` call) and assert `TurnEnd::Reported` and that the `ToolResult` after the invalid reply starts with `error: invalid tool call`.

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-local-agent chatml reply families turn` → FAIL.

- [ ] **Step 3: Implement.**

```rust
//! Prompt ChatML de Qwen3 : système, bloc `<tools>`, appels et résultats d'outils.
//!
//! Rendu ici plutôt que par le modèle de chat du GGUF : llama.cpp n'y passe pas les outils.

use serde_json::json;

use crate::engine::{Completion, Message};

const TOOLS_HEAD: &str = "# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>\n";
const TOOLS_TAIL: &str = "</tools>\n\nFor each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:\n<tool_call>\n{\"name\": <function-name>, \"arguments\": <args-json-object>}\n</tool_call>";

/// Le prompt complet, terminé par l'ouverture du tour de l'assistant.
pub(crate) fn render(c: &Completion) -> String {
    let mut system = c.system.clone();
    if !c.tools.is_empty() {
        system.push_str("\n\n");
        system.push_str(TOOLS_HEAD);
        for t in &c.tools {
            let line = json!({"type": "function", "function": {"name": t.name, "description": t.description, "parameters": t.schema}});
            system.push_str(&line.to_string());
            system.push('\n');
        }
        system.push_str(TOOLS_TAIL);
    }
    let mut out = turn("system", &system);
    for m in &c.messages {
        out.push_str(&match m {
            Message::User(text) => turn("user", text),
            Message::Assistant(text) => turn("assistant", text),
            Message::Call { name, args } => {
                turn("assistant", &format!("<tool_call>\n{}\n</tool_call>", json!({"name": name, "arguments": args})))
            }
            Message::ToolResult { output, .. } => turn("user", &format!("<tool_response>\n{output}\n</tool_response>")),
        });
    }
    out.push_str("<|im_start|>assistant\n");
    out
}

fn turn(role: &str, content: &str) -> String {
    format!("<|im_start|>{role}\n{content}<|im_end|>\n")
}
```

  `json!` with `serde_json` default features keeps insertion order only with `preserve_order`; the test expects `name, description, parameters` and `name, arguments` order. Check `serde_json` features in the root `Cargo.toml`; if `preserve_order` is off, keys sort alphabetically (`description, name, parameters`; `arguments, name`) — then write the expected strings of the tests in that order instead (the model reads either).

  `reply.rs`: find `<tool_call>`; none → `Reply::Text(text.trim().to_owned())`; else take the slice after it up to `</tool_call>` or the end, trim, parse as `serde_json::Value`; require an object with a string `name`; `arguments` absent or `null` → `{}`; an object → it; anything else or a parse error → `Reply::Invalid(<short English reason with the serde error>)`.

  `families.rs`:

```rust
/// Grammaire GBNF de l'étape de routage : une ou deux clés de famille, séparées par une virgule.
pub(crate) fn route_grammar() -> String {
    let keys: Vec<String> = Family::ROUTABLE.iter().map(|f| format!("\"{}\"", f.key())).collect();
    format!("root ::= family (\", \" family)?\nfamily ::= {}\n", keys.join(" | "))
}
```

  `turn.rs`: before `self.call(...)` push `Message::Call { name: name.clone(), args: args.clone() }`; `Reply::Invalid(reason)` → same branch as a rejected call with `error: invalid tool call: {reason}` (counts as a repair; push a `Message::Assistant` of nothing? no — push only the `ToolResult` with name `"invalid"`). Update existing turn tests that count `messages` in the fake log (each call now adds one message).

- [ ] **Step 4: Run** `cargo nextest run -p oku-slides-local-agent` → PASS. `ARCHITECTURE.md`: `chatml`, `reply`, `route_grammar`.

- [ ] **Step 5: Stage, commit message** `feat(local-agent) : prompt ChatML de Qwen3 avec outils et lecture des appels d'outils (t-1047, t-1022)`.

---

### Task 5: Gguf engine on llama.cpp

**Files:**
- Modify: root `Cargo.toml` (`[workspace.dependencies]`), `crates/oku-slides-local-agent/Cargo.toml`, `Cargo.lock`
- Create: `crates/oku-slides-local-agent/src/gguf.rs`, `crates/oku-slides-local-agent/tests/gguf_manual.rs`
- Modify: `src/lib.rs`, `src/error.rs` (`Model` variant), `src/engine.rs` (remove the `dead_code` expects that a real engine now satisfies), `ARCHITECTURE.md`, `AGENTS.md`, `deny.toml` only if `cargo deny` (in `just ci` / `just audit`) rejects a new licence (the audit found them all allowed)

**Interfaces:**
- Consumes: `chatml::render`, `reply::parse`, `families::route_grammar`, `Completion`, `Step`, `EngineError` (`CancelledSnafu`, `FailedSnafu`), `LocalAgentSettings` (Task 2).
- Produces (only on `cfg(all(target_os = "macos", target_arch = "aarch64"))`):
  ```rust
  pub(crate) struct GgufEngine { jobs: std::sync::mpsc::Sender<Job> }
  impl GgufEngine { pub(crate) fn load(model: &Path, settings: &LocalAgentSettings) -> Result<Self, LocalAgentError> }
  impl Engine for GgufEngine
  ```

- [ ] **Step 1: Dependency.** Root `Cargo.toml`, next to the dictation deps:

```toml
# Agent local de repli (moteur Gguf, Apple silicon) : llama.cpp avec Metal. Audit du lot 3
# (t-1022) : MIT / Apache-2.0, minos 12.0, +6 Mo. `common` : grammaires GBNF ; pas d'openmp.
llama-cpp-2 = { version = "=0.1.158", default-features = false, features = ["metal", "common"] }
llama-cpp-sys-2 = { version = "=0.1.158", default-features = false }
```

  Crate `Cargo.toml`:

```toml
[target.'cfg(all(target_os = "macos", target_arch = "aarch64"))'.dependencies]
llama-cpp-2.workspace = true
# Épinglé avec llama-cpp-2 (qui accepte ^0.1.158) : même version de llama.cpp.
llama-cpp-sys-2.workspace = true
```

  Run `cargo check -p oku-slides-local-agent` (first build ≈ 2–3 min, in the background) and confirm `Cargo.lock` holds both at `0.1.158`. If clippy/`just check` complains that `llama-cpp-sys-2` is an unused direct dependency (`unused_crate_dependencies` is not enabled by default; check the workspace `[lints]`), add `use llama_cpp_sys_2 as _;` in `gguf.rs` with a comment.

- [ ] **Step 2: Error variant.** `error.rs`: `#[snafu(display("cannot load the local model {}", path.display()))] Model { path: PathBuf, message: String, backtrace: Backtrace }` (llama errors are converted with `to_string()`, not kept as source: they are not `'static + Error` uniformly — check `LlamaModelLoadError`; if it implements `Error`, keep it as `source` instead).

- [ ] **Step 3: Implement `gguf.rs`** (follow the working probe; one worker thread owns everything llama.cpp):

```rust
//! Moteur Gguf : llama.cpp (Metal) sur un fil dédié, propriétaire du modèle et du contexte.
//!
//! `LlamaContext` n'est pas `Send` : un seul fil le garde, reçoit les complétions par canal et
//! vérifie l'annulation à chaque jeton. Le cache KV est vidé à chaque complétion.

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::mpsc;

use async_trait::async_trait;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use oku_slides_platform::config::LocalAgentSettings;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::engine::{CancelledSnafu, Completion, Engine, EngineError, FailedSnafu, Reply, Step};

/// Jetons envoyés au modèle par appel à `decode` pendant la lecture du prompt.
const PREFILL_CHUNK: usize = 512;

struct Job {
    prompt: String,
    grammar: Option<String>,
    cancel: CancellationToken,
    reply: oneshot::Sender<Result<String, EngineError>>,
}

/// Le moteur Gguf ; le fil s'arrête quand le moteur est lâché (canal fermé).
pub(crate) struct GgufEngine {
    jobs: mpsc::Sender<Job>,
}

impl std::fmt::Debug for GgufEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GgufEngine")
    }
}

impl GgufEngine {
    /// Charge le modèle sur le fil du moteur ; rend la main quand il est prêt ou en échec.
    pub(crate) fn load(model: &Path, settings: &LocalAgentSettings) -> Result<Self, crate::error::LocalAgentError> {
        let (jobs, rx) = mpsc::channel::<Job>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let (path, settings) = (model.to_owned(), settings.clone());
        std::thread::Builder::new()
            .name("oku-gguf".into())
            .spawn(move || worker(&path, &settings, &ready_tx, &rx))
            .map_err(|e| /* ModelSnafu { path: model, message: e.to_string() } */)?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self { jobs }),
            Ok(Err(message)) => /* Err(ModelSnafu { path: model, message }) */,
            Err(_) => /* Err(ModelSnafu { path: model, message: "engine thread stopped".into() }) */,
        }
    }
}
```

  (Write the three `ModelSnafu { … }.build().into()` expressions in full; the comments above only mark where.)

  `worker`: `llama_cpp_2::send_logs_to_tracing(llama_cpp_2::LogOptions::default())`; `LlamaBackend::init()`; `LlamaModel::load_from_file(&backend, path, &LlamaModelParams::default().with_n_gpu_layers(999))`; `model.new_context(&backend, LlamaContextParams::default().with_n_ctx(NonZeroU32::new(settings.context)).with_n_batch(PREFILL_CHUNK as u32))`; any error → `ready_tx.send(Err(e.to_string()))` and return; else `ready_tx.send(Ok(()))`, then `while let Ok(job) = rx.recv()` run `generate` and `let _ = job.reply.send(result)` (a dropped receiver means the turn was abandoned).

  `generate(model, ctx, settings, job) -> Result<String, EngineError>`:
  1. `ctx.clear_kv_cache()`.
  2. `let tokens = model.str_to_token(&job.prompt, AddBos::Never)` (ChatML has its own specials; check the method name in 0.1.158: the probe used `model.vocab().tokenize(bytes, false, true)` — use whichever exists, with special tokens parsed). Fail with `Failed { message: "prompt too long" }` if `tokens.len() + max_reply_tokens > context`.
  3. Prefill in `PREFILL_CHUNK` chunks with a `LlamaBatch::new(PREFILL_CHUNK, 1)`: `batch.clear()`, `batch.add(token, pos, &[0], is_last_of_prompt)`, `ctx.decode(&mut batch)`; check `job.cancel.is_cancelled()` between chunks → `Cancelled`.
  4. Sampler: `LlamaSampler::chain_simple([grammar?, LlamaSampler::temp(settings.temperature), LlamaSampler::dist(seed)])` with the grammar sampler `LlamaSampler::grammar(model, &g, "root")` first when `job.grammar` is `Some`; for the Route step use `LlamaSampler::greedy()` instead of temp+dist (deterministic routing).
  5. Loop up to `max_reply_tokens`: `if job.cancel.is_cancelled() { return Cancelled }`; `let t = sampler.sample(ctx, batch.n_tokens() - 1)`; `sampler.accept(t)`; stop on `is_eog`; append the token's bytes (`token_to_piece` bytes, special tokens not rendered) to a `Vec<u8>`; next batch of one token at the next position; `ctx.decode`.
  6. `String::from_utf8_lossy(&bytes).into_owned()`.
  Every llama error → `FailedSnafu { message: e.to_string() }`.

  `Engine` impl:

```rust
#[async_trait]
impl Engine for GgufEngine {
    async fn complete(&self, c: &Completion, cancel: &CancellationToken) -> Result<Reply, EngineError> {
        let grammar = (c.step == Step::Route).then(crate::families::route_grammar);
        let (tx, rx) = oneshot::channel();
        let job = Job { prompt: crate::chatml::render(c), grammar, cancel: cancel.clone(), reply: tx };
        self.jobs.send(job).map_err(|_| EngineError::from(FailedSnafu { message: "engine thread stopped" }.build()))?;
        let text = tokio::select! {
            r = rx => r.map_err(|_| EngineError::from(FailedSnafu { message: "engine thread stopped" }.build()))??,
            () = cancel.cancelled() => return Err(CancelledSnafu.build().into()),
        };
        Ok(match c.step {
            Step::Route => Reply::Text(text),
            Step::Act => crate::reply::parse(&text),
        })
    }
}
```

  (On cancel the worker notices the same token at its next token and its reply is dropped; the next job waits for it, which is bounded by one token.)

- [ ] **Step 4: Manual test** `tests/gguf_manual.rs` — needs a crate-visible constructor: expose for tests only `#[doc(hidden)] pub fn gguf_smoke(model: &Path) -> Result<(String, String), String>` in `lib.rs` under the same `cfg`, which loads the engine with the default `LocalAgentSettings` (from `Settings::defaults()`), runs one Route completion ("Make the title bigger") and one Act completion exposing a `report` tool, and returns both raw replies as text. The test:

```rust
//! Charge le vrai modèle (2,5 Go) : manuel, `OKU_SLIDES_GGUF_MODEL=<fichier .gguf>`.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

#[test]
#[ignore = "manuel : charge le vrai modèle Qwen3 4B"]
fn gguf_real_model_routes_and_acts() {
    let model = std::env::var("OKU_SLIDES_GGUF_MODEL").unwrap();
    let (route, act) = oku_slides_local_agent::gguf_smoke(std::path::Path::new(&model)).unwrap();
    eprintln!("route: {route}\nact: {act}");
    assert!(route.split(", ").all(|k| ["edit", "present", "files", "templates", "configure", "troubleshoot"].contains(&k)));
    assert!(!act.is_empty());
}
```

  Use the real family keys from `Family::key` (check `families.rs` and adjust the list).

- [ ] **Step 5: Run** `cargo nextest run -p oku-slides-local-agent` (PASS, the manual test skipped) and `cargo build -p oku-slides-local-agent --release` (links). If a model file is available locally, run the manual test once with `--run-ignored only` and paste its output in the task report; otherwise say it was not run.

- [ ] **Step 6: Docs.** `ARCHITECTURE.md` `gguf` bullet; `AGENTS.md` invariant: `- Keep every llama.cpp call in gguf.rs, on its one worker thread; never share a LlamaContext. (plan:2026-10-18-agent-local-repli-lot3)`.

- [ ] **Step 7: Stage, commit message** `feat(local-agent) : moteur Gguf sur llama.cpp (Metal), fil dédié et annulation par jeton (t-1048, t-1022)`.

---

### Task 6: Engine chosen by the settings

**Files:**
- Modify: `crates/oku-slides-local-agent/src/entry.rs`, `entry_tests.rs`, `error.rs`, `src/bin/oku-slides-local-agent-dev.rs`, `tests/common/mod.rs`, `ARCHITECTURE.md`
- Modify: `crates/oku-slides-agent/src/config.rs` (`Launch.env`), `link.rs` (spawn with `.envs`), every `Launch { … }` literal (`stack.rs`, `link_tests.rs`, `tab_entry_tests.rs`, `tests/launch.rs`, local-agent `tests/common/mod.rs`)
- Modify: `crates/oku-slides-cli/tests/local_agent.rs`

**Interfaces:**
- Consumes: `local_engine`, `ReadyEngine`, `Unavailable`, `LocalAgentSettings` (Task 2), `GgufEngine::load` (Task 5), `FakeEngine::from_file` (lot 1).
- Produces: `Launch.env: Vec<(String, String)>`; `pub(crate) fn build_engine(platform: &Platform) -> Result<Arc<dyn Engine>, LocalAgentError>`; error variant `Unavailable { reason: String, backtrace }` ("local engine unavailable: {reason}") replacing `NoEngine` where an engine is unavailable, `NoScript` when `fake` has no `fake_script`.

- [ ] **Step 1: Failing tests** (`entry_tests.rs`, replacing the argument tests):

```rust
use speculoos::prelude::*;

use super::build_engine;

fn platform(edit: impl FnOnce(&mut oku_slides_platform::config::Settings)) -> (tempfile::TempDir, oku_slides_platform::Platform) {
    let dir = tempfile::tempdir().unwrap();
    let mut settings = oku_slides_platform::config::Settings::defaults().unwrap();
    settings.paths.data = Some(dir.path().to_owned());
    edit(&mut settings);
    (dir, oku_slides_platform::Platform::with_settings(settings).unwrap())
}

#[cfg(debug_assertions)]
#[test]
fn build_engine_fake_without_script_is_no_script() {
    let (_d, p) = platform(|s| s.local_agent.engine = oku_slides_platform::config::LocalEngine::Fake);
    let err = build_engine(&p).unwrap_err();
    assert_that!(matches!(err.0, crate::error::Inner::NoScript { .. })).is_true();
}

#[cfg(debug_assertions)]
#[test]
fn build_engine_fake_with_script_builds() {
    let (dir, p) = platform(|s| s.local_agent.engine = oku_slides_platform::config::LocalEngine::Fake);
    let script = dir.path().join("s.json");
    std::fs::write(&script, r#"{"turns":[]}"#).unwrap();
    let mut settings = p.settings.clone();
    settings.local_agent.fake_script = Some(script);
    let p = oku_slides_platform::Platform::with_settings(settings).unwrap();
    assert_that!(build_engine(&p)).is_ok();
}

#[test]
fn build_engine_gguf_without_model_is_unavailable() {
    let (_d, p) = platform(|s| s.local_agent.engine = oku_slides_platform::config::LocalEngine::Gguf);
    let err = build_engine(&p).unwrap_err();
    assert_that!(matches!(err.0, crate::error::Inner::Unavailable { .. })).is_true();
}
```

  (Adapt field access to what `Settings`/`Platform` actually expose: `Platform.settings` is public in lot 1's harness.)

- [ ] **Step 2: Run** → FAIL.

- [ ] **Step 3: Implement.** `exit_if_local_agent`: return unless `argv` contains `LOCAL_AGENT_FLAG`; `telemetry::init()`; `Platform::load()` (map its error into a new `Platform { source, backtrace }` variant, or log and exit 1); `build_engine(&platform)`; runtime; `acp::serve(engine)`. `build_engine`:

```rust
pub(crate) fn build_engine(platform: &Platform) -> Result<Arc<dyn Engine>, LocalAgentError> {
    let settings = &platform.settings.local_agent;
    match local_engine(&platform.settings, &platform.storage) {
        #[cfg(debug_assertions)]
        Ok(ReadyEngine::Fake) => {
            let script = settings.fake_script.as_deref().context(NoScriptSnafu)?;
            Ok(Arc::new(crate::fake::FakeEngine::from_file(script, settings.fake_log.clone())?))
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        Ok(ReadyEngine::Gguf { model }) => Ok(Arc::new(crate::gguf::GgufEngine::load(&model, settings)?)),
        // `resolve` ne rend Gguf que sur Apple silicon : ailleurs ce bras est inatteignable mais
        // doit exister pour la complétude ; il échoue avec sa raison plutôt que de paniquer.
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        Ok(ReadyEngine::Gguf { .. }) => UnavailableSnafu { reason: "gguf engine not built for this target" }.fail().map_err(Into::into),
        Err(why) => UnavailableSnafu { reason: format!("{why:?}") }.fail().map_err(Into::into),
    }
}
```

  Remove `parse_args`, `Args`, `EngineChoice`. Dev binary usage line: `usage: OKU_SLIDES_LOCAL_AGENT__ENGINE=fake OKU_SLIDES_LOCAL_AGENT__FAKE_SCRIPT=<file> oku-slides-local-agent-dev --local-agent`.

  `Launch`: add `/// Variables ajoutées à l'environnement de l'agent (tests : moteur factice de l'agent local).
  pub env: Vec<(String, String)>,`; `Launch::for_preset` and `Stack::launch_of` set `Vec::new()`; `link.rs` spawns with `.envs(env.iter().map(|(k, v)| (k, v)))`. Every `Launch { … }` literal gets `env: Vec::new()` except the local-agent harness, which sets `OKU_SLIDES_LOCAL_AGENT__ENGINE=fake`, `OKU_SLIDES_LOCAL_AGENT__FAKE_SCRIPT=<script>`, `OKU_SLIDES_LOCAL_AGENT__FAKE_LOG=<log>`, `OKU_SLIDES_PATHS__DATA=<dir>`, `OKU_SLIDES_PATHS__DECKS=<dir>/decks`, with `args: vec!["--local-agent".into()]`. CLI test: `.arg("--local-agent")` with the same `.env(...)` pairs.

- [ ] **Step 4: Run** `cargo nextest run -p oku-slides-local-agent -p oku-slides-agent -p oku-slides-cli` → PASS (the round trip and the CLI handshake now go through the settings).

- [ ] **Step 5: Docs.** local-agent `ARCHITECTURE.md` `entry` bullet (settings, `build_engine`, `Unavailable`, `NoScript`); `AGENTS.md`: the invariant "Accept the fake engine only in debug builds" stays; add `- Choose the engine only through local_engine; a forced engine that is unavailable fails with its reason. (spec:2026-10-18-agent-local-repli-design)`. agent `ARCHITECTURE.md`: `Launch.env`.

- [ ] **Step 6: Stage, commit message** `feat(local-agent)! : moteur choisi par le réglage local_agent.engine, fin des arguments --engine (t-1049, t-1022)`.

---

### Task 7: Availability states and the Local preset

**Files:**
- Create: `crates/oku-slides-agent/src/choice.rs`, `choice_tests.rs`
- Modify: `crates/oku-slides-agent/src/stack.rs` (delegate to `AgentChoice`), `picker.rs`, `error.rs`, `error_tests.rs`, `stack_tests.rs` (or wherever `availability` is tested today: `rg "fn .*availability" crates/oku-slides-agent`), `ARCHITECTURE.md`
- Modify: `crates/oku-slides-platform/src/config.default.toml` (`[agent.presets.local]`), `config_tests.rs` (exactly-one text)
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl` (20 files)
- Modify: `ui/e2e/210-agent-picker.cjs` only if the extra chip breaks it (it selects chips by name, so it should not)

**Interfaces:**
- Consumes: `local_engine`, `Unavailable` (Task 2); `Link::failure`, `picker_surface` (existing).
- Produces:
  ```rust
  pub(crate) struct AgentChoice { platform: Platform, link: Link, active: Mutex<String> }
  impl AgentChoice {
      pub(crate) fn new(platform: &Platform, link: Link) -> Self;
      pub(crate) fn active(&self) -> String;
      pub(crate) fn availability(&self) -> Option<String>;          // moved from Stack, plus engine states
      pub(crate) fn picker(&self, busy: bool) -> Surface;            // picker_surface with the current active
      pub(crate) fn select(&self, id: &str) -> Result<(), CommandError>;  // moved from Stack::select_agent
  }
  // error.rs
  enum Unusable { …, LocalNeedsDownload { size: u64 }, LocalIntelMac, LocalUnsupportedOs }
  impl From<Unavailable> for Unusable
  ```
  `Stack` holds `choice: Arc<AgentChoice>`; `Stack::availability()` stays public and delegates.

- [ ] **Step 1: Failing tests.** `error_tests.rs` (extend the existing table):

```rust
#[rstest]
#[case::needs_download(Unusable::LocalNeedsDownload { size: 2_497_281_120 }, "agent-local-needs-download")]
#[case::intel(Unusable::LocalIntelMac, "agent-local-intel-mac")]
#[case::other_os(Unusable::LocalUnsupportedOs, "agent-local-unsupported-os")]
fn unusable_local_states_localize_with_their_key(#[case] u: Unusable, #[case] key: &str) {
    let fr = Localizer::new(Some("fr"));
    let text = u.localize(&fr);
    assert_that!(text.as_str()).is_not_equal_to(key);       // la clé existe en français
    assert_that!(text).is_not_equal_to(u.localize(&Localizer::new(Some("en"))));
}

#[test]
fn unusable_needs_download_shows_size_in_gigabytes() {
    let text = Unusable::LocalNeedsDownload { size: 2_497_281_120 }.localize(&Localizer::new(Some("en")));
    assert_that!(text).contains("2.5");
}
```

  (Use the same localizer constructor as the existing tests in that file; if missing keys render as the key itself, the first assertion catches a missing translation.)

  `choice_tests.rs` (build a `Platform` with a temp data dir, `program: Some(..)` via `with_program`, `agent.selected = "local"`; a `Link` with a dummy `Launch`):

```rust
#[test]
fn availability_local_without_model_needs_download() { /* on Apple silicon: contains the fr text of agent-local-needs-download; elsewhere the IntelMac/UnsupportedOs text — compute the expected Unusable from Target::current() */ }

#[test]
fn availability_local_with_complete_marker_is_ready() { /* write models/agent/.complete with std::fs; on Apple silicon availability() is None */ }

#[cfg(debug_assertions)]
#[test]
fn availability_local_fake_engine_is_ready() { /* settings.local_agent.engine = Fake → None */ }

#[test]
fn availability_local_without_program_is_no_program() { /* program None → agent-local-no-program text */ }
```

  Write each body in full in the test file (the comments state what each asserts). `config_tests.rs`: `agent_settings_preset_without_exactly_one_launch_is_refused` asserts `err.to_string()` contains `"[agent.presets.x] needs exactly one of command or builtin"` (look at how `settings_from_toml`'s error displays; assert on the substring); add `settings_defaults_have_local_builtin_preset`.

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-agent -p oku-slides-platform` → FAIL.

- [ ] **Step 3: Implement.** `Unusable::localize`:
  - `LocalNeedsDownload { size }` → `l.tr_with("agent-local-needs-download", &[("gigabytes", ((size as f64 / 1e9 * 10.0).round() / 10.0).into())])` (`size as f64` on a `u64` needs an `#[expect(clippy::cast_precision_loss, reason = "taille affichée à 0,1 Go près")]` if that lint is on).
  - `LocalIntelMac` → `agent-local-intel-mac`, `LocalUnsupportedOs` → `agent-local-unsupported-os`.
  `AgentChoice::availability` for a built-in preset: `program` `None` → `NoProgram`; else `match local_engine(&settings, &storage) { Err(u) => Some(Unusable::from(u).localize(&l)), Ok(_) => self.link.failure().map(|u| u.localize(&l)) }`. Move `select_agent`'s body and the picker construction into `AgentChoice`; `Stack` keeps `busy` (it reads the table) and calls `choice.picker(busy)`.
  English keys in `en/agent.ftl`, after `agent-local-no-program`:

```ftl
agent-local-needs-download = The local agent needs its model ({ $gigabytes } GB) to be downloaded first.
agent-local-intel-mac = The local agent needs a Mac with Apple silicon.
agent-local-unsupported-os = The local agent is not available on this system yet.
```

  Then the 19 other locales, same order, translated (fr: `L'agent local a d'abord besoin de télécharger son modèle ({ $gigabytes } Go).`, `L'agent local nécessite un Mac avec puce Apple.`, `L'agent local n'est pas encore disponible sur ce système.`).
  `config.default.toml`, after `[agent.presets.gemini]`:

```toml
# Agent intégré : un petit modèle qui tourne sur ce Mac, sans compte ni réseau
# (moteur choisi par [local_agent]).
[agent.presets.local]
name = "Local"
builtin = "local"
```

- [ ] **Step 4: Run** `cargo nextest run -p oku-slides-agent -p oku-slides-platform -p oku-slides-i18n` → PASS; `just e2e 4973` for `210-agent-picker` and `060-agent` (fresh data dir, the extra chip must not break them).

- [ ] **Step 5: Docs.** agent `ARCHITECTURE.md`: `choice` module, availability states. agent `AGENTS.md` (or `crates/AGENTS.md` if it is cross-crate): `- Decide whether the local agent can run only through oku_slides_platform::local_agent::local_engine, in the host and the child alike. (plan:2026-10-18-agent-local-repli-lot3)`.

- [ ] **Step 6: Stage, commit message** `feat(agent) : états de disponibilité de l'agent local et préréglage « Local » par défaut (t-1050, t-1022)`.

---

### Task 8: Model download from the agent panel

**Files:**
- Create: `crates/oku-slides-agent/src/model_download.rs`, `model_download_tests.rs`
- Modify: `crates/oku-slides-agent/Cargo.toml` (`oku-slides-platform` feature `download`; dev-deps `tiny_http`, `sha2`, `hex` if the tests hash), `src/choice.rs`, `picker.rs`, `picker_tests.rs`, `stack.rs` (`AgentCommand::{DownloadModel, CancelDownload}`), `ARCHITECTURE.md`
- Modify: `crates/oku-slides-i18n/src/locales/*/agent.ftl`
- Modify: `ui/src/api.ts`, `ui/src/agent/AgentPicker.tsx`, `AgentPicker.render.test.tsx`, `useAgentPicker.ts`, `useAgentPicker.render.test.tsx`, `AgentPanel.tsx` (`AgentApi` type), every `AgentPicker`/`useAgentPicker` caller (`rg "AgentPicker|useAgentPicker" ui/src`) and test doubles of `AgentApi` (`selectAgent: async` occurrences)
- Modify: `ui/e2e/210-agent-picker.cjs`

**Interfaces:**
- Consumes: `models::{download, DownloadErrorKind}`, `AGENT_MODEL`, `AGENT_MODEL_SIZE`, `agent_model_dir`, `local_engine` (Tasks 1–2); `AgentChoice` (Task 7).
- Produces:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub(crate) enum DownloadState { Idle, Running { received: u64, total: u64 }, Failed(DownloadErrorKind) }
  #[derive(Debug, Clone, Default)]
  pub(crate) struct ModelDownload { state: Arc<Mutex<DownloadState>>, cancel: Arc<AtomicBool> }
  impl ModelDownload {
      /// Lance le téléchargement si rien ne tourne ; `on_change` est appelé à chaque pourcent et à la fin.
      pub(crate) fn start(&self, files: &'static [PinnedFile], dir: PathBuf, total: u64, settings: ModelsSettings,
                          storage: Storage, on_change: impl Fn() + Send + 'static) -> bool;
      pub(crate) fn cancel(&self);
      pub(crate) fn state(&self) -> DownloadState;
  }
  // AgentChoice gains `download: ModelDownload` and
  pub(crate) fn start_download(self: &Arc<Self>, on_change: impl Fn(&AgentChoice) + Send + 'static) -> bool;
  // AgentCommand
  DownloadModel { tab: String }, CancelDownload { tab: String }   // serde: "download_model", "cancel_download"
  ```
  UI: `AgentApi.downloadModel(): Promise<void>`, `AgentApi.cancelDownload(): Promise<void>`; `AgentPicker` props `onDownload: () => void`, `onCancelDownload: () => void`; `useAgentPicker` returns `download()` and `cancelDownload()`.

- [ ] **Step 1: Failing Rust tests.** `model_download_tests.rs` (fake hub as in Task 1, a two-file pin with SHA-256 of small bodies, `files` leaked to `'static` with `Box::leak`):

```rust
#[test]
fn start_runs_to_idle_and_writes_marker() { /* start → wait (poll state up to 5 s) until Idle; models::is_complete(dir) */ }

#[test]
fn start_twice_while_running_returns_false() { /* hub that sleeps 200 ms per request; second start is false */ }

#[test]
fn cancel_ends_in_failed_cancelled() { /* slow hub; cancel(); state ends Failed(DownloadErrorKind::Cancelled) */ }

#[test]
fn sha_mismatch_ends_in_failed_integrity() { /* wrong pin; state Failed(Integrity); no marker */ }

#[test]
fn on_change_called_at_least_at_the_end() { /* counter in an Arc<AtomicUsize> ≥ 1 after Idle */ }
```

  Write each body in full. `picker_tests.rs` (picker surface states, `AgentChoice`-independent: give `picker_surface` a new parameter `local: Option<LocalPanel>` — see Step 3 — and test it directly):

```rust
#[rstest]
#[case::needs(LocalPanel::NeedsDownload { reason: "r".into() }, &["local-reason", "download"])]
#[case::running(LocalPanel::Running { percent: 42 }, &["progress", "cancel-download"])]
#[case::failed(LocalPanel::Failed { reason: "r".into(), error: "e".into() }, &["local-reason", "download-error", "download"])]
fn picker_local_panel_shows_its_components(#[case] panel: LocalPanel, #[case] ids: &[&str]) {
    /* build the surface; collect component ids from the update_components message; assert each id is present */
}

#[test]
fn picker_download_button_sends_download_model() { /* the "download" Button's action event name is "download_model" */ }
```

- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-agent download picker` → FAIL.

- [ ] **Step 3: Implement Rust.**
  `model_download.rs`: `start` takes the state lock, refuses if `Running`, sets `Running { received: 0, total }`, resets `cancel`, releases the lock, spawns a `std::thread` that calls `models::download(files, &dir, total, &settings, &storage, |r, t| { …; !cancel.load(Relaxed) })`, updating `Running { received, total }` and calling `on_change()` only when `received * 100 / total` changes; at the end sets `Idle` (Ok) or `Failed(e.kind())` (Err, logged with `snafu::Report`), then `on_change()`.
  `picker.rs`:

```rust
/// Ce que le panneau montre de l'agent local quand il est choisi et pas prêt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LocalPanel {
    /// Modèle absent : la raison (avec la taille) et le bouton Télécharger.
    NeedsDownload { reason: String },
    /// Téléchargement en cours : la progression et le bouton Annuler.
    Running { percent: u8 },
    /// Le dernier téléchargement a échoué : la raison, l'erreur et le bouton Télécharger.
    Failed { reason: String, error: String },
}
```

  `picker_surface(presets, active, busy, locked, local: Option<&LocalPanel>, l)` appends, after `agents`: `Component::text("local-reason", reason, None)`; `Component::button("download", l.tr("agent-model-download"), true, Action::event("download_model", []))`; `Component::text("progress", l.tr_with("agent-model-downloading", &[("percent", percent.into())]), None)`; `Component::button("cancel-download", l.tr("agent-model-cancel"), false, Action::event("cancel_download", []))`; `Component::text("download-error", error, None)` — in the orders listed by the test table. Use the existing `Action::event` signature (check it accepts an empty context).
  `AgentChoice::local_panel()`: `None` unless the active preset is built-in and `local_engine` returns `Err(Unavailable::NeedsDownload { .. })`; then from `download.state()`: `Running` → `Running { percent }`, `Failed(kind)` → `Failed { reason, error: localize(kind) }`, `Idle` → `NeedsDownload { reason }` (reason = the availability text). Failure texts: new keys `agent-model-download-failed`, `agent-model-download-integrity`, `agent-model-download-incomplete` (with `$got`/`$expected`), `agent-model-download-cancelled`, chosen from `DownloadErrorKind`.
  `AgentChoice::start_download(self: &Arc<Self>, on_change)`: refuses (returns `false`) unless `local_panel()` is `NeedsDownload`/`Failed`; calls `download.start(&AGENT_MODEL_FILES, agent_model_dir(..), AGENT_MODEL_SIZE, settings.models.clone(), storage.clone(), move || on_change(&choice))`.
  `Stack::handle`: `DownloadModel { tab }` → `let (out, tab2) = (self.out_tx.clone(), tab.clone()); self.choice.start_download(move |c| { emit(&out, "agent", &tab2, &AgentEvent::Picker { surface: c.picker(false) }); emit(&out, "agent", &tab2, &AgentEvent::Availability { unusable: c.availability() }); })` then `self.emit_picker(&tab)`; `CancelDownload { tab }` → `self.choice.download.cancel()` (via a method) then `emit_picker`. Add both to `AgentCommand::tab`. Progress re-emits use `busy = false` (documented on the closure: the busy note comes back with the next `picker` request).
  English keys (agent.ftl, after the Task 7 keys), then 19 locales:

```ftl
agent-model-download = Download the model
agent-model-downloading = Downloading the model… { $percent } %
agent-model-cancel = Cancel
agent-model-download-failed = The model could not be downloaded.
agent-model-download-integrity = The downloaded model was damaged and has been deleted.
agent-model-download-incomplete = Incomplete download: { $got } bytes out of { $expected }.
agent-model-download-cancelled = Download cancelled.
```

- [ ] **Step 4: Run Rust** `cargo nextest run -p oku-slides-agent` → PASS.

- [ ] **Step 5: Failing UI tests.** `AgentPicker.render.test.tsx`: a surface with a Button whose action is `download_model` → clicking it calls `onDownload` once; `cancel_download` → `onCancelDownload`. `useAgentPicker.render.test.tsx`: `download()` calls `agent.downloadModel` and reports a rejection through `onError`. Follow the existing tests in these files for surface fixtures and rendering helpers.

- [ ] **Step 6: Implement UI.** `api.ts`: `| { cmd: "download_model"; tab: string } | { cmd: "cancel_download"; tab: string }` and `downloadModel: () => agentCall({ cmd: "download_model", tab })`, `cancelDownload: () => agentCall({ cmd: "cancel_download", tab })` with doc comments. `AgentApi` (in `AgentPanel.tsx`) gains both. `AgentPicker.onAction`: `download_model` → `onDownload()`, `cancel_download` → `onCancelDownload()`. `useAgentPicker`: `download` and `cancelDownload` like `select` (report errors via `onError`). Wire the callers; update every `AgentApi` test double.

- [ ] **Step 7: e2e.** In `210-agent-picker.cjs`, after step 3 (back to « Faux »), add: choose « Local », wait for the picker to show a button `Télécharger le modèle` (French label of `agent-model-download`) **or**, when the runner is not Apple silicon, the text of `agent-local-intel-mac` / `agent-local-unsupported-os` (read `process.arch`); never click it; choose « Faux » again. Run `just e2e 4973`.

- [ ] **Step 8: Run all** `just test` → PASS. Docs: agent `ARCHITECTURE.md` (`model_download`, picker local panel, two commands); `ui/AGENTS.md` only if a rule changes.

- [ ] **Step 9: Stage, commit message** `feat(agent) : téléchargement du modèle de l'agent local depuis le panneau, progression et annulation (t-1051, t-1022)`.

---

## Final gate (controller)

- `just check` and `just test` green; `just e2e` green.
- Spec coverage walk: every bullet of "Lot 3 design (t-1022)" maps to Tasks 1–8.
- Mark each sub-task done as its task completes; t-1022 done after the merge is approved.
