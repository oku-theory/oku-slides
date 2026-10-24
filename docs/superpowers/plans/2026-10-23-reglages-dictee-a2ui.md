# Dictation and agent settings box — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the ⚙ dictation popup with a live A2UI host box `host-voice` in the agent dock, and let the agent drive the dictation and app-agent settings over MCP — except the dictation API address and key variable, which only the user sets.

**Architecture:** `oku-slides-stt` gains pure helpers (model ids, silence range, API field validation and normalization), a key-presence check that never copies the key, and a hardened API client. `oku-slides-session` builds the box from a text-free `VoiceView` (`VoicePanel`) and parses its actions (`voice_event`). `oku-slides-mcp` defines a three-method `VoicePort` (`view`, `check`, `apply`) with an opaque `VoiceError`; every per-tab `SurfaceHub` shows, applies and refreshes the box through one send path. `oku-slides-agent` implements the port (`AppVoice`), counts running ACP prompts with RAII `PromptGuard`s taken in the tab actor, switches the shared agent only when no prompt runs and then persists the agent actually active, watches `Stt`, and refreshes every tab once per change. Seven MCP tools check through the port before any confirmation. The UI loses the popup and the agent-picker protocol.

**Tech Stack:** Rust (snafu, speculoos, rstest, tokio `watch`, `url` 2.5), rust-mcp-sdk 1.x, Fluent (20 locales), React/TS (Bun, happy-dom), Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-10-23-reglages-dictee-a2ui-design.md` (task t-1144; sub-tasks t-1146…t-1151), rewritten together with this plan.

## Global Constraints

- Every commit: `type(scope) : description (t-NNNN)` citing t-1144 and the task's sub-task id, French message, **no `Co-Authored-By` line**; `!` after the scope for Tasks 4 and 5. Commit with `git commit -F <file>`, message file in the scratchpad.
- Every commit passes `just check` (fmt, clippy `-D warnings`, `clippy.toml` `too-many-arguments-threshold = 6`, tsc, agents-lint) and the tests of the crates it touches; Tasks 5 and 6 also pass `just e2e 4999`.
- Errors: Snafu, a backtrace in every variant, opaque public types, Fluent-localized; no `unwrap`/`expect` outside tests; no silent fallback (a refused or no-op request says so; an invariant violation is logged at `error!` and refused).
- **Locks.** `AppVoice.state` covers memory only (never I/O, never `view()`, never `agents_view()`). Two locks may cover I/O and are never taken while `state` is held: `AppVoice.edit` (the dictation config read-modify-write, `Stt::configure` writes Storage) and `AgentChoice.saving` (writing `agent.selected` to `config.toml`). Lock order when nested: `state` → `AgentChoice.active` → `Link` mutexes. Mutexes `lock().unwrap_or_else(PoisonError::into_inner)` (tests too); no std guard across `.await`; blocking work off the async runtime.
- No `unsafe` anywhere (workspace `unsafe_code = "deny"`), never `std::env::set_var` in tests.
- Localized errors take only safe arguments; any agent-supplied string shown to the user goes through `oku_slides_session::agent::bounds::clip`.
- The dictation API key value is never copied outside `ApiTranscriber`, and never in a surface, tool result, log or confirmation.
- Tests: sibling `x_tests.rs` via `#[cfg(test)] #[path]`, speculoos **matchers** (`contains`, `is_none`, `is_empty`, `does_not_contain_key`, `is_err_containing` — never `assert_that!(a && b).is_true()`), names `unit_scenario_result`, one behaviour per test (rstest for tables), localized errors asserted on the kind plus the literal French text; regressions in `tests/regressions/issue_1144_*.rs`; nothing waits without a deadline; no timing assertion that the starting state also satisfies.
- Every pub item, field and new module documented, first sentence **at most 14 words**. Files ≤ 500 lines target, 1000 max.
- No new `ComponentKind`; `CATALOG_ID` unchanged.
- Each crate's `ARCHITECTURE.md` is updated **in the commit that changes the crate**, by merging the lines named in the step (session, mcp and agent are at 50 lines; each task runs `wc -l crates/*/ARCHITECTURE.md` and keeps ≤ 50). Each rule change goes to the nearest `AGENTS.md` in the same commit.
- Fluent: the Rust `Localizer` has no `NUMBER()`; numbers are formatted with `oku_slides_chart::NumberFormat::for_lang(Some(l.locale()))` and passed as strings. New keys in all 20 locales, same key order everywhere; CJK classifiers stick to their number (`{ $value }秒`). `ui-*` keys are UI-only.
- Runs outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir.

## Decisions (user-approved; also in the spec)

1. **The agent never chooses where dictation goes nor which secret it sends.** `api.url` and `api.key_env` are not tool arguments; any of the keys `api`, `api_url`, `url`, `api_key_env`, `key_env` in `set_dictation` arguments is refused before parsing with `ApiReservedToUser` ("open the box with show_dictation and ask the user"). The port also refuses them for `Origin::Agent`. The agent may change `api.model`, the model, the language, auto stop, silence, download models and switch the app agent.
2. **Validation on every write and at use:** `url` is ASCII, without whitespace, controls, `\`, userinfo, query or fragment, contains `://`; scheme `https`, or `http` only to a loopback host (`localhost`, `127.0.0.1`, `[::1]` after normalization); host ≤ 253 chars, labels 1–63 chars of `[A-Za-z0-9_-]` (or a bracketed IPv6); the **normalized** form (`url::Url::as_str`, trailing `/` trimmed) is ≤ 512 chars, idempotent, and is what is stored, shown and called. `key_env` matches `^[A-Z_][A-Z0-9_]{0,63}$`; `api.model` `^[A-Za-z0-9._:/-]{1,128}$`. The API client refuses invalid stored settings, follows no redirect and uses no proxy for loopback. Key presence is checked without copying the key.
3. **Confirmations:** switching the model to `api` asks one `danger` confirmation naming the user-configured host, URL and variable and the effective API model (stored, or the one in the same request). Switching to a model to download (local or Apple language) asks the download confirmation and then starts the download. A download and an agent switch are confirmed. A confirmed agent request is applied with `confirmed: true`; if the state changed so that a confirmation would now be needed but was not given, `apply` refuses (`Changed`). A request that changes nothing answers `{"applied": "unchanged"}`; a cancel with nothing to cancel is refused (`NothingToCancel`).
4. **Agent switch:** one shared ACP agent serves every tab; "busy" = at least one `PromptGuard` alive (each guard has a unique id; the tab actor holds one while it prompts, across a queued follow-up). An agent's `select_agent` is always pending while a prompt runs (`when_idle`) and applied when the last guard drops; the switch re-checks under the `AppVoice` lock (epoch, shutdown, no guard) and only then calls `Link::switch`; afterwards the agent **actually active** is persisted. Selecting the active agent never restarts it: it only clears a pending choice. A user click applies at once, bumps the epoch and clears any pending choice. Closing the requesting tab or quitting drops a pending choice. The box shows it. A pending switch can wait as long as replies keep running: shown in the box, accepted.
5. **Refresh:** `AppVoice` runs one watcher thread on `Stt` (400 ms; a typed fingerprint of state, config, model availability, download percent and detected language; bumps only on change). Busy, pending and progress lines are always present and bound to the data model, so their changes are data-only `SurfacePatch`es; the box is rebuilt only when its layout changes. Clicks and refreshes go through one hub send path under the state lock, the shown box never leaves `shows` during a click. The stack builds one panel per change for all tabs, emits `AgentEvent::Voice` only when the dictation part changed and `Availability` only when it changed (seeded at start); the UI also re-reads the mic status after an SSE reconnect.
6. **Read-only views** have no dock: their ⚙ is hidden. `SttRequest::{Configure, Download}` disappear. The box shows model states as chip hints, not `Badge`s. Trust boundary: on loopback, `oku-slides serve` accepts UI commands without a token, so a local process (including an agent allowed to run shell commands) can act as the user; such an agent could read the environment anyway. The guarantee is against MCP-only agents.

## File map

| File | Task | Responsibility |
|---|---|---|
| `crates/oku-slides-stt/src/{models.rs,lib.rs,api.rs}`, new `api_check.rs` (+ tests) | 1 | `Model::{id, from_id}`, `SILENCE_RANGE`, `configure -> bool`, `download -> bool`, `normalized_url`, `url_host`, `is_loopback`, `valid_key_env`, `valid_api_model`, `ApiConfig::key_present`, hardened `ApiTranscriber` |
| `crates/oku-slides-session/{Cargo.toml,src/voice_view.rs,src/voice_panel.rs,src/voice_fixture.rs}` (+ tests) | 2 | text-free view, events, `VoicePanel`, `model_label`, `size_text`, `seconds_text`, `gigabytes_text`; fixture behind `test-support` |
| `crates/oku-slides-mcp/{Cargo.toml,src/voice.rs,src/hub/settings.rs,src/hub.rs,src/hub/actions.rs,src/hub/error.rs}` | 3 | `VoicePort`, `VoiceError`, `Checked`, `NoVoice` behind `test-support`, the box in the hub |
| `crates/oku-slides-agent/src/{voice_port.rs,agent_switch.rs,stt_watch.rs,voice_refresh.rs,table.rs,stack.rs,tab_entry.rs,tab_agent.rs,choice.rs,command_error.rs,error.rs,events.rs,turns.rs}` | 3 | `AppVoice`, `PromptGuard`, switch, watcher, refresh, `ShowSettings`, `AgentEvent::Voice` |
| `crates/oku-slides-mcp/src/{voice_tools.rs,tools.rs,handler.rs,tool_error.rs,settings.rs,guarded_tools.rs}`, agent `settings_port.rs`, `instructions/agent.md`, local-agent `families.rs` | 4 | tools, whitelist, instructions |
| `ui/src/…`, `ui/e2e/210-settings-box.cjs`, `ui/e2e/lib/`, `justfile`, agent `{stack,events,turns,choice,model_download}.rs`, stt `SttRequest` | 5 | UI, old protocol removed, e2e |
| `docs/aide/{en,fr}.json`, `README.md` | 6 | help, README, final gate |

---

### Task 1: `oku-slides-stt` helpers and a hardened API client (t-1146)

**Files:**
- Create: `crates/oku-slides-stt/src/api_check.rs`, `api_check_tests.rs`
- Modify: `src/models.rs` (+ tests), `src/lib.rs` (`SILENCE_RANGE`, `configure`, `download`, `model_info`, `handle`, re-exports), `src/lib_tests.rs`, `src/dictation_stream_tests.rs`, `src/api.rs`, `src/api_tests.rs`, `src/error.rs` (+ tests), `examples/transcribe.rs`, `Cargo.toml`, root `Cargo.toml` (`url = "2.5.8"` in `[workspace.dependencies]`, already in `Cargo.lock`), `stt.ftl` × 20, `crates/oku-slides-stt/{ARCHITECTURE.md,AGENTS.md}`
- Read first: `crates/oku-slides-stt/AGENTS.md`, `src/lib.rs` (`configure`, `download`, `model_info` line ~366, `handle`), `src/models.rs`, `src/api.rs` (`ApiConfig::key`, `ApiTranscriber::new`), `src/api_tests.rs` (`tiny_http` fakes), `src/test_support.rs`, `src/dictation_stream_tests.rs` (reaching `Recording`: `Stt::with_engine` + `ScriptedEngine`, `handle(Start)`, `wait_until`)

**Interfaces (re-exported at the crate root):**
- `impl Model { pub fn id(self) -> &'static str; pub fn from_id(id: &str) -> Option<Model> }` (same names as serde).
- `pub const SILENCE_RANGE: RangeInclusive<f32> = 0.5..=10.0;` (`/// Durées de silence acceptées pour l'arrêt seul (s).`)
- `#[must_use = "faux : la dictée était occupée, rien n'a changé"] pub fn configure(&self, config: Config) -> bool`.
- `#[must_use = "faux : rien n'a démarré (occupée, déjà installé, API)"] pub fn download(&self, model: Model, then_start: bool) -> bool`.
- `pub fn normalized_url(url: &str) -> Option<String>`, `pub fn url_host(url: &str) -> Option<String>`, `pub fn is_loopback(url: &str) -> bool`, `pub fn valid_key_env(name: &str) -> bool`, `pub fn valid_api_model(name: &str) -> bool`.
- `ApiConfig::key_present(&self) -> bool` (`pub(crate)`): the variable is set and not blank, read with `std::env::var_os`, never copied into a `String`.
- `SttError` gains an inner variant `ApiConfig` (kind `Model`, key `stt-api-config`).

- [ ] **Step 1: Write the failing tests.** `api_check_tests.rs`:

```rust
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case("https://api.openai.com/v1", "https://api.openai.com/v1")]
#[case("https://api.openai.com/v1/", "https://api.openai.com/v1")]
#[case("https://api.openai.com", "https://api.openai.com")]
#[case("HTTPS://API.groq.com/openai/v1", "https://api.groq.com/openai/v1")]
#[case("http://localhost:8000/v1", "http://localhost:8000/v1")]
#[case("http://127.0.0.1:9000", "http://127.0.0.1:9000")]
#[case("http://127.1:9000", "http://127.0.0.1:9000")]
#[case("http://0x7f.1/v1", "http://127.0.0.1/v1")]
#[case("http://2130706433/v1", "http://127.0.0.1/v1")]
#[case("http://[::1]:9000/v1", "http://[::1]:9000/v1")]
fn normalized_url_accepted_form_is_canonical(#[case] url: &str, #[case] expected: &str) {
    assert_that!(normalized_url(url)).is_equal_to(Some(expected.to_owned()));
}

#[rstest]
#[case("http://api.openai.com/v1")]
#[case("http://localhost.evil.example/v1")]
#[case("http://localhost./v1")]
#[case("http://127.0.0.1@evil.example/v1")]
#[case("http://[::ffff:127.0.0.1]/v1")]
#[case("http://[::1%25lo]/v1")]
#[case("https://api.openai.com@evil.example/v1")]
#[case("https://user:pw@api.openai.com/v1")]
#[case("https://api.openai.com/v1?x=1")]
#[case("https://api.openai.com/v1?")]
#[case("https://api.openai.com/v1#f")]
#[case("https://api.openai.com/\u{202e}1v")]
#[case("https://аpi.openai.com/v1")]
#[case(" https://api.openai.com/v1")]
#[case("https://api.openai.com/v 1")]
#[case("https:api.openai.com/v1")]
#[case("http://localhost\\@evil.example/v1")]
#[case("https://a..b/v1")]
#[case("https://.../v1")]
#[case("https://a{b}.example/v1")]
#[case("ftp://api.openai.com")]
#[case("api.openai.com/v1")]
#[case("")]
fn normalized_url_refused_form_is_none(#[case] url: &str) {
    assert_that!(normalized_url(url)).is_none();
}

#[test]
fn normalized_url_over_512_characters_is_none() {
    assert_that!(normalized_url(&format!("https://a.example/{}", "x".repeat(512)))).is_none();
}

#[test]
fn normalized_url_grown_past_512_by_encoding_is_none() {
    // `"` devient `%22` : la forme normalisée dépasse la borne.
    assert_that!(normalized_url(&format!("https://a.example/{}", "\"".repeat(200)))).is_none();
}

#[test]
fn normalized_url_with_a_64_character_label_is_none() {
    assert_that!(normalized_url(&format!("https://{}.example/v1", "a".repeat(64)))).is_none();
}

#[rstest]
#[case("https://api.openai.com/v1/")]
#[case("HTTP://127.1:9000/")]
#[case("https://api.groq.com/openai/v1")]
fn normalized_url_is_idempotent(#[case] url: &str) {
    let once = normalized_url(url).unwrap();
    assert_that!(normalized_url(&once)).is_equal_to(Some(once));
}

#[rstest]
#[case("https://api.groq.com/openai/v1", Some("api.groq.com"))]
#[case("http://[::1]:9000/v1", Some("[::1]"))]
#[case("http://evil.example", None)]
fn url_host_names_the_host_of_a_valid_url(#[case] url: &str, #[case] host: Option<&str>) {
    assert_that!(url_host(url)).is_equal_to(host.map(str::to_owned));
}

#[rstest]
#[case("http://127.1:9000", true)]
#[case("http://localhost:8000/v1", true)]
#[case("https://api.openai.com/v1", false)]
fn is_loopback_follows_the_normalized_host(#[case] url: &str, #[case] loopback: bool) {
    assert_that!(is_loopback(url)).is_equal_to(loopback);
}

#[rstest]
#[case("OPENAI_API_KEY", true)]
#[case("_KEY2", true)]
#[case("openai_api_key", false)]
#[case("sk-proj-abc123", false)]
#[case("A B", false)]
#[case("", false)]
fn valid_key_env_follows_the_name_rule(#[case] name: &str, #[case] ok: bool) {
    assert_that!(valid_key_env(name)).is_equal_to(ok);
}

#[rstest]
#[case(64, true)]
#[case(65, false)]
fn valid_key_env_name_length_is_capped_at_64(#[case] len: usize, #[case] ok: bool) {
    assert_that!(valid_key_env(&"A".repeat(len))).is_equal_to(ok);
}

#[rstest]
#[case("gpt-4o-transcribe", true)]
#[case("whisper-1", true)]
#[case("openai/whisper-large-v3", true)]
#[case("model\r\nX: y", false)]
#[case("m\"x", false)]
#[case("", false)]
fn valid_api_model_follows_the_name_rule(#[case] name: &str, #[case] ok: bool) {
    assert_that!(valid_api_model(name)).is_equal_to(ok);
}
```

Other tests:
- models: `model_serde_name_is_its_id` (rstest over `Model::ALL`), `model_from_id_round_trips` (rstest over `Model::ALL`), `model_from_id_unknown_is_none`.
- `lib_tests.rs`: `configure_while_idle_returns_true`; `download_of_an_installed_model_returns_false` (a model whose files the test creates, as the existing installed-model tests do); `download_of_the_api_model_returns_false`; `api_model_installed_follows_the_key_variable` (`key_env: "HOME"` → installed; a unique unset name → not).
- `dictation_stream_tests.rs` (it already reaches `Recording`): `configure_while_recording_returns_false`, `configure_while_recording_keeps_the_config`, `download_while_recording_returns_false`.
- `api_tests.rs`, with the existing `tiny_http` helpers:
  - `api_transcriber_invalid_stored_url_is_refused` (url `http://192.168.1.5/v1`; assert `e.kind() == SttErrorKind::Model` and `e.localize(&fr)` equals the French text below);
  - `api_transcriber_invalid_key_env_is_refused` (key_env `sk-x`, same assertions);
  - `api_transcriber_redirect_is_not_followed`: a first fake answers `307` with `Location: http://127.0.0.1:<second>/x`; the second fake runs `recv_timeout(Duration::from_millis(500))` in a thread and its result is asserted `None`; the call fails with the HTTP status 307, asserted through the existing `ApiStatus` kind.
- `error_tests.rs`: `ApiConfig` key and French literal.

- [ ] **Step 2: Run them to see them fail.** `cargo nextest run -p oku-slides-stt`. Expected: compile errors.

- [ ] **Step 3: Write `api_check.rs`.**

```rust
//! Règles des réglages de l'API de dictée, sur chaque écriture et à l'usage.
//!
//! L'adresse et la variable décident quel secret part vers quel hôte : la forme normalisée
//! est la seule stockée, montrée et appelée (spec t-1144).

/// Longueur maximale d'une adresse d'API, normalisée.
const MAX_URL: usize = 512;
/// Longueurs maximales d'un hôte, puis d'une de ses étiquettes (DNS).
const MAX_HOST: usize = 253;
const MAX_LABEL: usize = 63;
/// Hôtes autorisés en `http://` : cette machine, après normalisation.
const LOOPBACK: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];

/// La forme normalisée d'une adresse d'API acceptable ; `None` si elle est refusée.
pub fn normalized_url(url: &str) -> Option<String> {
    let plain = !url.is_empty() && url.bytes().all(|b| b.is_ascii_graphic()) && url.contains("://") && !url.contains('\\');
    if !plain {
        return None;
    }
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    let scheme_ok = match parsed.scheme() {
        "https" => true,
        "http" => LOOPBACK.contains(&host),
        _ => false,
    };
    let host_ok = host.len() <= MAX_HOST
        && (host.starts_with('[')
            || host
                .split('.')
                .all(|l| (1..=MAX_LABEL).contains(&l.len()) && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')));
    let clean = parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.query().is_none()
        && parsed.fragment().is_none();
    let normal = parsed.as_str().trim_end_matches('/').to_owned();
    (scheme_ok && host_ok && clean && normal.len() <= MAX_URL).then_some(normal)
}

/// L'hôte d'une adresse valide, montré seul dans les confirmations.
pub fn url_host(url: &str) -> Option<String> {
    let normal = normalized_url(url)?;
    url::Url::parse(&normal).ok()?.host_str().map(str::to_owned)
}

/// L'adresse valide vise cette machine (pas de proxy pour elle).
pub fn is_loopback(url: &str) -> bool {
    url_host(url).is_some_and(|h| LOOPBACK.contains(&h.as_str()))
}

/// Le nom est celui d'une variable d'environnement : majuscules, chiffres et `_`.
pub fn valid_key_env(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else { return false };
    name.len() <= 64
        && (first.is_ascii_uppercase() || first == b'_')
        && bytes.all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

/// Le nom est celui d'un modèle d'API : lettres, chiffres et `._:/-`.
pub fn valid_api_model(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'/' | b'-'))
}

#[cfg(test)]
#[path = "api_check_tests.rs"]
mod api_check_tests;
```

(`is_ascii_graphic` refuses spaces, controls and every non-ASCII byte, so bidi controls and lookalike letters never reach the parser; `://` and `\` checks refuse forms the WHATWG parser would repair; IPv6 zone ids and `[::ffff:…]` fail the loopback list; the checked url 2.5.8 behaviour matches every case above.)

- [ ] **Step 4: `Model::id`/`from_id`** in `models.rs`:

```rust
    /// Identifiant du modèle, le même que son nom serde.
    pub fn id(self) -> &'static str {
        match self {
            Model::Apple => "apple",
            Model::Turbo => "turbo",
            Model::Large => "large",
            Model::Base => "base",
            Model::Api => "api",
        }
    }

    /// Le modèle d'identifiant `id` ; `None` s'il est inconnu.
    pub fn from_id(id: &str) -> Option<Model> {
        Model::ALL.into_iter().find(|m| m.id() == id)
    }
```

- [ ] **Step 5: `lib.rs`.**
  - `SILENCE_RANGE`; `configure` clamps with it, returns `false` when not idle, `true` at the end.
  - `download` returns `bool`: `false` on the existing early returns (not idle or installed; `Api` after its `report`), the result of `download_language` for Apple (make it return `bool` too: `false` when it reports and returns), `true` once the thread is started.
  - `model_info`: `Model::Api => (g.config.api.key_present(), true)` — the key is no longer copied on every status.
  - `Stt::handle(SttRequest::Configure(c))` (removed in Task 5): `if !self.configure(c) { tracing::info!("réglages de dictée ignorés : dictée en cours"); }`; `SttRequest::Download` arm: `if !self.download(model, false) { tracing::info!(model = model.id(), "téléchargement non lancé"); }`. Every other caller (`rg -n '\.configure\(|\.download\(' crates src-tauri`; `examples/transcribe.rs`, the stt tests): `let _applied = …;` with a one-line French comment saying why the result does not matter there.
  - Re-export the new items (explicit list; update the module doc's table of contents).

- [ ] **Step 6: `api.rs`.**
  - `ApiConfig::key_present`:

```rust
    /// La variable de la clé est posée et non vide ; la clé n'est jamais copiée.
    pub(crate) fn key_present(&self) -> bool {
        std::env::var_os(&self.key_env).is_some_and(|v| v.as_encoded_bytes().iter().any(|b| !b.is_ascii_whitespace()))
    }
```

  - `ApiTranscriber::new` refuses the config unless `normalized_url(&config.url)`, `valid_key_env(&config.key_env)` and `valid_api_model(&config.model)` all pass: `SttInner::ApiConfig { backtrace }` (kind `Model`; key `stt-api-config`, en "The dictation API settings are invalid: correct them in the settings box." / fr "Les réglages de l'API de dictée sont invalides : corrigez-les dans la boîte des réglages."); endpoint built from the normalized URL; agent config `.max_redirects(0)`, and `.proxy(None)` when `is_loopback(&config.url)`, else `ureq::Proxy::try_from_env()` as today.

- [ ] **Step 7: Dependencies.** Root `[workspace.dependencies]`: `url = "2.5.8"`; `crates/oku-slides-stt/Cargo.toml`: `url.workspace = true`.

- [ ] **Step 8: Docs and rules.** `crates/oku-slides-stt/ARCHITECTURE.md`: rewrite the API line to name `api_check` (`normalized_url`, `is_loopback`, `valid_key_env`, `valid_api_model`), `Model::id`, `key_present`, and that the client refuses invalid settings, follows no redirect and skips the proxy for loopback. `crates/oku-slides-stt/AGENTS.md` Invariants: `- Store, show and call only the normalized_url form of the dictation API address; refuse invalid settings at use. (spec:2026-10-23-reglages-dictee-a2ui-design)` and `- Check the API key's presence with key_present; read the key itself only in ApiTranscriber. (spec:2026-10-23-reglages-dictee-a2ui-design)`.

- [ ] **Step 9: Run.** `cargo nextest run -p oku-slides-stt -p oku-slides-i18n && just check`. Expected: PASS.

- [ ] **Step 10: Commit** `feat(stt) : identifiants des modèles, plage du silence, règles de l'API de dictée, clé jamais copiée, client sans redirection ni proxy local (t-1144, t-1146)`.

---

### Task 2: The box surface, its view and its events (`oku-slides-session`) (t-1147)

**Files:**
- Create: `crates/oku-slides-session/src/voice_view.rs`, `voice_panel.rs`, `voice_fixture.rs`, `voice_view_tests.rs`, `voice_panel_tests.rs`
- Modify: `Cargo.toml` (`[features] test-support = []`), `src/lib.rs` (modules + explicit re-exports), `src/agent/forms_tests.rs` (`every_form`), `ui/src/a2ui/fixtures/agent-surfaces.json` (regenerated), `session.ftl` × 20, `crates/oku-slides-session/ARCHITECTURE.md`
- Read first: `src/deck_packs_panel.rs` (pattern; close button lines 221-223), `src/agent/forms.rs` (`surface`, `lit`, `Place`, `forms.rs:411` for `Vec<String>` → `&[&str]`), `crates/oku-slides-a2ui/src/message.rs` (`Component::column(&str, &[&str])`, `ComponentKind::Text`), `crates/oku-slides-agent/src/picker.rs`, `crates/oku-slides-chart/src/numbers.rs` (`NumberFormat::{for_lang, format, localize}`), `crates/oku-slides-core/Cargo.toml` (its `test-support` feature, the pattern)

**Interfaces (re-exported from the crate root):**

```rust
pub const HOST_VOICE: &str = "host-voice";
pub const DICTATION_LANGUAGES: [(&str, &str); 7];
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ModelKind { Apple, Local, Api }
pub struct ModelView { pub id: String, pub kind: ModelKind, pub size: u64, pub installed: bool, pub available: bool }
pub struct ApiView { pub url: String, pub model: String, pub key_env: String, pub key_found: bool }
pub struct DownloadView { pub model: String, pub received: u64, pub total: u64 }
pub struct DictationView { pub model: String, pub models: Vec<ModelView>, pub language: Option<String>,
    pub detected: Option<String>, pub auto_stop: bool, pub silence: f32, pub api: ApiView,
    pub download: Option<DownloadView>, pub idle: bool }
pub enum LocalFailure { Failed, Integrity, Incomplete { got: u64, expected: u64 }, Cancelled }
pub enum LocalModelView { NeedsDownload { size: u64 }, Running { percent: u8 }, Failed { size: u64, failure: LocalFailure } }
pub struct AgentPresetView { pub id: String, pub name: String, pub hint: Option<String> }
#[derive(Default)] pub struct AgentsView { pub presets: Vec<AgentPresetView>, pub active: String, pub busy: bool,
    pub locked: bool, pub local: Option<LocalModelView>, pub pending: Option<String> }
#[derive(Default)] pub struct VoiceView { pub dictation: Option<DictationView>, pub agents: AgentsView }
pub enum DictationLanguage { Auto, Code(String) }
#[derive(Default)] pub struct DictationPatch { pub model: Option<String>, pub language: Option<DictationLanguage>,
    pub auto_stop: Option<bool>, pub silence: Option<f32>, pub api_url: Option<String>, pub api_model: Option<String>,
    pub api_key_env: Option<String> }
impl DictationPatch { pub fn is_empty(&self) -> bool; pub fn touches_api_target(&self) -> bool }
pub enum VoiceEvent { Dictation(DictationPatch), DownloadDictation { model: String }, CancelDictationDownload,
    SelectAgent { id: String }, DownloadAgentModel, CancelAgentModelDownload }
pub fn voice_event(name: &str, context: &Map<String, Value>) -> Option<VoiceEvent>;
#[derive(Debug, Clone, PartialEq)] pub struct VoicePanel;   // components + data
impl VoicePanel { pub fn build(view: &VoiceView, l: &Localizer) -> Self; pub fn surface(&self) -> Surface;
    pub fn same_layout(&self, other: &Self) -> bool; pub fn data_messages(&self) -> Vec<ServerMessage> }
pub fn model_label(model: &ModelView, api_model: &str, l: &Localizer) -> String;   // short name: voice-model-<id>
pub fn size_text(bytes: u64, l: &Localizer) -> String;                              // "818 MB", "3,1 Go"
pub fn seconds_text(seconds: f32, l: &Localizer) -> String;                         // "1,5 s", "1,2 s"
pub fn gigabytes_text(bytes: u64, l: &Localizer) -> String;                         // "2,5"
// behind #[cfg(any(test, feature = "test-support"))]:
pub mod voice_fixture { pub fn voice_view_fixture() -> VoiceView; }
```

All structs derive `Debug, Clone, PartialEq` (plus `Default` where marked); `ModelKind` also `Copy, Eq`. Each field and variant field has a `///` line. `touches_api_target` is true when `api_url` or `api_key_env` is set (doc: those decide where the secret goes). `ModelView.id`, `DictationView.model` and `AgentsView.active` are strings on purpose: session stays free of `oku-slides-stt` (doc line on `ModelView`).

Layout (`Place::Thread`, `live: true`; ids fixed):

```
root Column: [d-title, d-intro, d-models, d-unavailable?, d-api-url?, d-api-model?, d-api-key-env?, d-api-note?,
              d-language, d-silence, d-note, d-progress, d-cancel?, d-download?,
              a-title, a-agents, a-local-reason?, a-error?, a-download?, a-progress, a-cancel?,
              a-pending, a-busy, a-locked?, close]
```

Data: `{ "model", "language", "silence", "api": { "url", "model", "key_env" }, "choice", "progress": { "dictation", "agent" }, "notice": { "busy", "pending" } }`. `d-progress`, `a-progress`, `a-busy`, `a-pending` are **always present** `Text`s bound to `/progress/dictation`, `/progress/agent`, `/notice/busy`, `/notice/pending` (empty string when off).

Actions: `model` {`id`: `/model`}; `language` {`code`: `/language`}; `silence` {`value`: `/silence`}; `api` {`field`: literal `url|model|key_env`, `value`: `/api/<field>`}; `download` {`model`: literal}; `cancel_download`; `select` {`id`: `/choice`}; `download_agent_model`; `cancel_agent_model_download`; `dismiss` (button `close`, handled by the hub).

- [ ] **Step 1: Fluent keys (`session.ftl`).** Copy, per locale, the value of the left key into the right key (same locale), keeping the key order identical in all 20 files (UI keys deleted in Task 5, `agent-picker-*` in Task 5):

| From | To |
|---|---|
| `ui-dictation-model-title` | `voice-dictation-title` |
| `ui-dictation-model-intro` / `-intro-apple` | `voice-dictation-intro` / `-intro-apple` |
| `ui-dictation-note-{apple,turbo,large,base,api}` | `voice-note-{…}` |
| `ui-dictation-installed` / `-not-installed` | `voice-installed` / `voice-not-installed` |
| `ui-dictation-api-ready` / `-api-key-missing` | `voice-api-ready` / `voice-api-key-missing` |
| `ui-dictation-language` / `-language-auto` / `-language-auto-resolved` | `voice-language` / `-language-auto` / `-language-auto-resolved` |
| `ui-dictation-auto-stop-label` | `voice-silence` |
| `ui-dictation-phrases-note` | `voice-phrases-note` |
| `ui-dictation-download-now` / `-download-language` | `voice-download-now` / `voice-download-language` |
| `ui-dictation-api-url` / `-model` / `-key-env` / `-note` | `voice-api-url` / `-model` / `-key-env` / `-note` |
| `ui-dictation-cancel` | `voice-cancel` |
| `agent-picker-title` / `-busy` / `-locked` (`agent.ftl`) | `voice-agent-title` / `-busy` / `-locked` |

New keys (en / fr given; translate the en text for the 18 other locales):

| Key | en | fr |
|---|---|---|
| `voice-model-apple` | Apple (system) | Apple (système) |
| `voice-model-turbo` | Large v3 Turbo | Large v3 Turbo |
| `voice-model-large` | Large v3 | Large v3 |
| `voice-model-base` | Base | Base |
| `voice-model-api` | API ({ $model }) | API ({ $model }) |
| `voice-silence-off` | Never | Jamais |
| `voice-seconds` | { $value } s | { $value } s |
| `voice-size-mb` | { $value } MB | { $value } Mo |
| `voice-size-gb` | { $value } GB | { $value } Go |
| `voice-unavailable` | Not available for this language: { $models } | Indisponible pour cette langue : { $models } |
| `voice-downloading` | Downloading { $name } · { $percent }% | Téléchargement de { $name } · { $percent } % |
| `voice-agent-pending` | Switching to { $name } as soon as no reply is running. | Passage à { $name } dès qu'aucune réponse ne tourne. |

(ja, zh-Hans, zh-Hant: `voice-seconds = { $value }秒`; ko: `{ $value }초`.) Do the copy with a short Python script kept in the scratchpad and run over `crates/oku-slides-i18n/src/locales/*/`, appending the new block after the last existing key; check with `cargo nextest run -p oku-slides-i18n`.

- [ ] **Step 2: The fixture.** `Cargo.toml`: `[features]` `test-support = []` (doc comment: "Expose `voice_fixture` to other crates' tests."). `src/voice_fixture.rs` (module doc `//! La vue de référence de la boîte des réglages, pour les tests d'autres crates.`), declared in `lib.rs` as `#[cfg(any(test, feature = "test-support"))] pub mod voice_fixture;` — it uses only `voice_view` types, no `tempfile`, no I/O:

```rust
use crate::voice_view::{AgentPresetView, AgentsView, ApiView, DictationView, ModelKind, ModelView, VoiceView};

/// La vue de référence : Turbo installé, Large à télécharger, Base indisponible, API sans clé.
pub fn voice_view_fixture() -> VoiceView {
    let model = |id: &str, kind, size, installed, available| ModelView { id: id.into(), kind, size, installed, available };
    VoiceView {
        dictation: Some(DictationView {
            model: "turbo".into(),
            models: vec![
                model("turbo", ModelKind::Local, 817_887_676, true, true),
                model("large", ModelKind::Local, 3_090_835_702, false, true),
                model("base", ModelKind::Local, 148_067_931, false, false),
                model("api", ModelKind::Api, 0, false, true),
            ],
            language: None,
            detected: None,
            auto_stop: true,
            silence: 1.5,
            api: ApiView {
                url: "https://api.openai.com/v1".into(),
                model: "gpt-4o-transcribe".into(),
                key_env: "OPENAI_API_KEY".into(),
                key_found: false,
            },
            download: None,
            idle: true,
        }),
        agents: AgentsView {
            presets: vec![
                AgentPresetView { id: "faux".into(), name: "Faux".into(), hint: Some("fake --script s".into()) },
                AgentPresetView { id: "local".into(), name: "Local".into(), hint: None },
            ],
            active: "faux".into(),
            busy: false,
            locked: false,
            local: None,
            pending: None,
        },
    }
}
```

- [ ] **Step 3: Write the failing tests.** `voice_panel_tests.rs`:

```rust
use oku_slides_i18n::Localizer;
use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::*;
use crate::voice_fixture::voice_view_fixture;
use crate::voice_view::*;

fn en() -> Localizer {
    Localizer::new(Some("en"))
}

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn json_of(p: &VoicePanel) -> Value {
    serde_json::to_value(p.surface()).unwrap()
}

fn components(p: &VoicePanel) -> Vec<Value> {
    let messages = json_of(p)["messages"].as_array().cloned().unwrap();
    messages.iter().find_map(|m| m["updateComponents"]["components"].as_array().cloned()).unwrap()
}

fn ids(p: &VoicePanel) -> Vec<String> {
    components(p).iter().map(|c| c["id"].as_str().unwrap().to_owned()).collect()
}

fn component(p: &VoicePanel, id: &str) -> Value {
    components(p).into_iter().find(|c| c["id"] == id).unwrap()
}

fn data(p: &VoicePanel) -> Value {
    let messages = json_of(p)["messages"].as_array().cloned().unwrap();
    messages.iter().find_map(|m| m.get("updateDataModel").cloned()).unwrap()["value"].clone()
}

fn with(f: impl FnOnce(&mut DictationView)) -> VoiceView {
    let mut v = voice_view_fixture();
    f(v.dictation.as_mut().unwrap());
    v
}

fn agents(f: impl FnOnce(&mut AgentsView)) -> VoiceView {
    let mut v = voice_view_fixture();
    f(&mut v.agents);
    v
}

fn option_ids(c: &Value) -> Vec<String> {
    c["options"].as_array().unwrap().iter().map(|o| o["id"].as_str().unwrap().to_owned()).collect()
}

fn option(p: &VoicePanel, chips: &str, id: &str) -> Value {
    let c = component(p, chips);
    c["options"].as_array().unwrap().iter().find(|o| o["id"] == id).cloned().unwrap()
}

/// Les vues qui couvrent toutes les actions de la boîte.
pub(crate) fn covering_views() -> Vec<VoiceView> {
    let api = with(|d| {
        d.model = "api".into();
        d.api.key_found = true;
        d.download = Some(DownloadView { model: "large".into(), received: 1, total: 4 });
    });
    let local = with(|d| {
        d.model = "large".into();
        d.auto_stop = false;
        d.silence = 2.5;
    });
    let mut agent = voice_view_fixture();
    agent.agents.locked = true;
    agent.agents.busy = true;
    agent.agents.pending = Some("local".into());
    agent.agents.local = Some(LocalModelView::Failed { size: 2_500_000_000, failure: LocalFailure::Integrity });
    let mut running = voice_view_fixture();
    running.agents.local = Some(LocalModelView::Running { percent: 40 });
    vec![api, local, agent, running]
}

#[test]
fn voice_panel_surface_is_live_outside_any_turn() {
    let s = VoicePanel::build(&voice_view_fixture(), &en()).surface();
    assert_that!((s.id.as_str(), s.live, s.turn)).is_equal_to((HOST_VOICE, true, None));
}

#[test]
fn voice_panel_ends_with_a_close_button() {
    assert_that!(ids(&VoicePanel::build(&voice_view_fixture(), &en())).last().cloned()).is_equal_to(Some("close".to_owned()));
}

#[test]
fn voice_panel_close_button_sends_dismiss() {
    let close = component(&VoicePanel::build(&voice_view_fixture(), &en()), "close");
    assert_that!(close["action"]["event"]["name"].clone()).is_equal_to(json!("dismiss"));
}

#[test]
fn voice_panel_unavailable_model_is_not_a_chip() {
    let p = VoicePanel::build(&voice_view_fixture(), &en());
    assert_that!(option_ids(&component(&p, "d-models"))).is_equal_to(vec!["turbo".to_owned(), "large".to_owned(), "api".to_owned()]);
}

#[test]
fn voice_panel_unavailable_model_is_named_below_the_chips() {
    let p = VoicePanel::build(&voice_view_fixture(), &fr());
    assert_that!(component(&p, "d-unavailable")["text"].clone()).is_equal_to(json!("Indisponible pour cette langue : Base"));
}

#[test]
fn voice_panel_chosen_unavailable_model_stays_a_chip() {
    let p = VoicePanel::build(&with(|d| d.model = "base".into()), &en());
    assert_that!(option_ids(&component(&p, "d-models"))).contains(&"base".to_owned());
}

#[test]
fn voice_panel_model_chip_label_is_the_short_name() {
    let p = VoicePanel::build(&voice_view_fixture(), &fr());
    assert_that!(option(&p, "d-models", "large")["label"].clone()).is_equal_to(json!("Large v3"));
}

#[test]
fn voice_panel_model_chip_hint_names_the_size() {
    let p = VoicePanel::build(&voice_view_fixture(), &fr());
    assert_that!(option(&p, "d-models", "large")["hint"].as_str().unwrap_or("")).contains("3,1 Go");
}

#[rstest]
#[case(false, "OPENAI_API_KEY")]
#[case(true, "Clé trouvée")]
fn voice_panel_api_chip_hint_says_whether_the_key_is_set(#[case] found: bool, #[case] text: &str) {
    let p = VoicePanel::build(&with(|d| d.api.key_found = found), &fr());
    assert_that!(option(&p, "d-models", "api")["hint"].as_str().unwrap_or("")).contains(text);
}

#[rstest]
#[case("d-api-url")]
#[case("d-api-model")]
#[case("d-api-key-env")]
fn voice_panel_api_model_shows_its_field(#[case] id: &str) {
    assert_that!(ids(&VoicePanel::build(&with(|d| d.model = "api".into()), &en()))).contains(&id.to_owned());
}

#[test]
fn voice_panel_local_model_hides_the_api_fields() {
    assert_that!(ids(&VoicePanel::build(&voice_view_fixture(), &en()))).does_not_contain(&"d-api-url".to_owned());
}

#[rstest]
#[case(true, 1.5, "1.5")]
#[case(false, 1.5, "off")]
#[case(true, 2.5, "2.5")]
#[case(true, 1.2, "1.2")]
fn voice_panel_silence_value_is_the_short_seconds(#[case] on: bool, #[case] secs: f32, #[case] value: &str) {
    let v = with(|d| {
        d.auto_stop = on;
        d.silence = secs;
    });
    assert_that!(data(&VoicePanel::build(&v, &en()))["silence"].clone()).is_equal_to(json!(value));
}

#[test]
fn voice_panel_custom_silence_is_offered_in_order() {
    let p = VoicePanel::build(&with(|d| d.silence = 2.5), &en());
    let expected: Vec<String> = ["off", "1", "1.5", "2", "2.5", "3", "5"].into_iter().map(String::from).collect();
    assert_that!(option_ids(&component(&p, "d-silence"))).is_equal_to(expected);
}

#[rstest]
#[case(1.5, "1,5 s")]
#[case(1.2, "1,2 s")]
fn seconds_text_is_short_with_the_french_comma(#[case] secs: f32, #[case] text: &str) {
    assert_that!(seconds_text(secs, &fr())).is_equal_to(text.to_owned());
}

#[test]
fn voice_panel_uninstalled_chosen_model_offers_its_download() {
    let p = VoicePanel::build(&with(|d| d.model = "large".into()), &fr());
    assert_that!(component(&p, "d-download")["label"].clone()).is_equal_to(json!("Télécharger maintenant (3,1 Go)"));
}

#[test]
fn voice_panel_installed_chosen_model_offers_no_download() {
    assert_that!(ids(&VoicePanel::build(&voice_view_fixture(), &en()))).does_not_contain(&"d-download".to_owned());
}

#[test]
fn voice_panel_busy_dictation_offers_no_download() {
    let v = with(|d| {
        d.model = "large".into();
        d.idle = false;
    });
    assert_that!(ids(&VoicePanel::build(&v, &en()))).does_not_contain(&"d-download".to_owned());
}

#[test]
fn voice_panel_progress_is_bound_to_the_data_model() {
    let p = VoicePanel::build(&voice_view_fixture(), &en());
    assert_that!(component(&p, "d-progress")["text"].clone()).is_equal_to(json!({ "path": "/progress/dictation" }));
}

#[test]
fn voice_panel_downloading_puts_the_percent_in_the_data_model() {
    let v = with(|d| d.download = Some(DownloadView { model: "large".into(), received: 1, total: 4 }));
    assert_that!(data(&VoicePanel::build(&v, &fr()))["progress"]["dictation"].clone())
        .is_equal_to(json!("Téléchargement de Large v3 · 25 %"));
}

#[test]
fn voice_panel_progress_change_keeps_the_layout() {
    let at = |received| VoicePanel::build(&with(|d| d.download = Some(DownloadView { model: "large".into(), received, total: 4 })), &en());
    assert_that!(at(1).same_layout(&at(2))).is_true();
}

#[rstest]
#[case::busy(|a: &mut AgentsView| a.busy = true)]
#[case::pending(|a: &mut AgentsView| a.pending = Some("local".into()))]
fn voice_panel_notice_change_keeps_the_layout(#[case] change: fn(&mut AgentsView)) {
    let a = VoicePanel::build(&voice_view_fixture(), &en());
    let b = VoicePanel::build(&agents(change), &en());
    assert_that!(a.same_layout(&b)).is_true();
}

#[test]
fn voice_panel_model_change_changes_the_layout() {
    let a = VoicePanel::build(&voice_view_fixture(), &en());
    let b = VoicePanel::build(&with(|d| d.model = "api".into()), &en());
    assert_that!(a.same_layout(&b)).is_false();
}

#[test]
fn voice_panel_data_messages_are_one_root_data_update() {
    let p = VoicePanel::build(&voice_view_fixture(), &en());
    let messages = serde_json::to_value(p.data_messages()).unwrap();
    assert_that!(messages[0]["updateDataModel"]["value"].clone()).is_equal_to(data(&p));
}

#[test]
fn voice_panel_busy_text_is_in_the_data_model() {
    let p = VoicePanel::build(&agents(|a| a.busy = true), &fr());
    assert_that!(data(&p)["notice"]["busy"].clone()).is_equal_to(json!("Changer d'agent interrompra le tour en cours."));
}

#[test]
fn voice_panel_pending_text_names_the_agent() {
    let p = VoicePanel::build(&agents(|a| a.pending = Some("local".into())), &fr());
    assert_that!(data(&p)["notice"]["pending"].clone()).is_equal_to(json!("Passage à Local dès qu'aucune réponse ne tourne."));
}

#[test]
fn voice_panel_apple_auto_language_names_the_detected_one() {
    let v = with(|d| {
        d.model = "apple".into();
        d.models.insert(0, ModelView { id: "apple".into(), kind: ModelKind::Apple, size: 0, installed: true, available: true });
        d.detected = Some("fr".into());
    });
    let options = component(&VoicePanel::build(&v, &en()), "d-language")["options"].clone();
    assert_that!(options[0]["label"].as_str().unwrap_or("")).contains("Français");
}

#[test]
fn voice_panel_unlisted_language_is_still_an_option() {
    let p = VoicePanel::build(&with(|d| d.language = Some("ja".into())), &en());
    assert_that!(option_ids(&component(&p, "d-language"))).contains(&"ja".to_owned());
}

#[rstest]
#[case(LocalModelView::NeedsDownload { size: 2_500_000_000 }, "a-download")]
#[case(LocalModelView::Running { percent: 40 }, "a-cancel")]
#[case(LocalModelView::Failed { size: 2_500_000_000, failure: LocalFailure::Integrity }, "a-error")]
fn voice_panel_local_agent_state_shows_its_component(#[case] local: LocalModelView, #[case] id: &str) {
    assert_that!(ids(&VoicePanel::build(&agents(|a| a.local = Some(local)), &en()))).contains(&id.to_owned());
}

#[test]
fn voice_panel_local_agent_reason_formats_gigabytes_in_french() {
    let p = VoicePanel::build(&agents(|a| a.local = Some(LocalModelView::NeedsDownload { size: 2_500_000_000 })), &fr());
    assert_that!(component(&p, "a-local-reason")["text"].as_str().unwrap_or("")).contains("2,5");
}

#[rstest]
#[case(false, true)]
#[case(true, false)]
fn voice_panel_agent_chips_send_select_unless_locked(#[case] locked: bool, #[case] sends: bool) {
    let chips = component(&VoicePanel::build(&agents(|a| a.locked = locked), &en()), "a-agents");
    assert_that!(chips.get("action").is_some()).is_equal_to(sends);
}

#[test]
fn voice_panel_builtin_agent_chip_has_no_hint() {
    let p = VoicePanel::build(&voice_view_fixture(), &en());
    assert_that!(option(&p, "a-agents", "local").get("hint").cloned()).is_none();
}

#[test]
fn voice_panel_without_dictation_has_no_dictation_component() {
    let mut v = voice_view_fixture();
    v.dictation = None;
    assert_that!(ids(&VoicePanel::build(&v, &en())).iter().filter(|i| i.starts_with("d-")).count()).is_equal_to(0);
}

#[test]
fn voice_panel_french_title_is_translated() {
    let p = VoicePanel::build(&voice_view_fixture(), &fr());
    assert_that!(component(&p, "d-title")["text"].clone()).is_equal_to(json!("Modèle de reconnaissance"));
}

#[rstest]
#[case(818_000_000, "818 MB")]
#[case(999_600_000, "1 GB")]
#[case(3_090_835_702, "3.1 GB")]
fn size_text_rounds_to_mb_or_tenths_of_gb(#[case] bytes: u64, #[case] text: &str) {
    assert_that!(size_text(bytes, &en())).is_equal_to(text.to_owned());
}

#[test]
fn size_text_french_uses_go_and_a_comma() {
    assert_that!(size_text(3_090_835_702, &fr())).is_equal_to("3,1 Go".to_owned());
}

/// Chaque action de la boîte, résolue contre son modèle de données, redonne un évènement.
#[test]
fn voice_panel_every_action_parses_back_into_an_event() {
    let mut seen = std::collections::BTreeSet::new();
    for view in covering_views() {
        let p = VoicePanel::build(&view, &en());
        let model = data(&p);
        for c in components(&p) {
            let Some(event) = c.get("action").and_then(|a| a.get("event")) else { continue };
            let name = event["name"].as_str().unwrap().to_owned();
            if name == "dismiss" {
                continue;
            }
            let empty = Map::new();
            let ctx: Map<String, Value> = event
                .get("context")
                .and_then(Value::as_object)
                .unwrap_or(&empty)
                .iter()
                .map(|(k, v)| match v.get("path").and_then(Value::as_str) {
                    Some(path) => (k.clone(), model.pointer(path).cloned().unwrap_or(Value::Null)),
                    None => (k.clone(), v.clone()),
                })
                .collect();
            assert_that!(voice_event(&name, &ctx)).named(&name).is_some();
            seen.insert(name);
        }
    }
    let expected: std::collections::BTreeSet<String> = [
        "model", "language", "silence", "api", "download", "cancel_download", "select", "download_agent_model",
        "cancel_agent_model_download",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    assert_that!(seen).is_equal_to(expected);
}
```

(`covering_views`: the API view has a dictation download running (`cancel_download`); the local view offers `download`; the locked view has no `select`, so the unlocked views provide it; the `running` view provides `cancel_agent_model_download`; the failed one `download_agent_model`. `Dynamic` is `#[serde(untagged)]`: literals serialize bare, paths as `{"path": …}`. Check the French literals against `fr/ui.ftl` and `fr/agent.ftl` before writing.)

`voice_view_tests.rs`:

```rust
use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

use super::*;

fn ctx(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

fn patch(p: DictationPatch) -> Option<VoiceEvent> {
    Some(VoiceEvent::Dictation(p))
}

#[rstest]
#[case("model", json!({"id": "large"}), patch(DictationPatch { model: Some("large".into()), ..Default::default() }))]
#[case("language", json!({"code": ""}), patch(DictationPatch { language: Some(DictationLanguage::Auto), ..Default::default() }))]
#[case("language", json!({"code": "fr"}), patch(DictationPatch { language: Some(DictationLanguage::Code("fr".into())), ..Default::default() }))]
#[case("silence", json!({"value": "off"}), patch(DictationPatch { auto_stop: Some(false), ..Default::default() }))]
#[case("silence", json!({"value": "3"}), patch(DictationPatch { auto_stop: Some(true), silence: Some(3.0), ..Default::default() }))]
#[case("api", json!({"field": "url", "value": "http://localhost:8000/v1"}), patch(DictationPatch { api_url: Some("http://localhost:8000/v1".into()), ..Default::default() }))]
#[case("api", json!({"field": "model", "value": "whisper-1"}), patch(DictationPatch { api_model: Some("whisper-1".into()), ..Default::default() }))]
#[case("api", json!({"field": "key_env", "value": "GROQ_API_KEY"}), patch(DictationPatch { api_key_env: Some("GROQ_API_KEY".into()), ..Default::default() }))]
#[case("download", json!({"model": "base"}), Some(VoiceEvent::DownloadDictation { model: "base".into() }))]
#[case("cancel_download", json!({}), Some(VoiceEvent::CancelDictationDownload))]
#[case("select", json!({"id": "bis"}), Some(VoiceEvent::SelectAgent { id: "bis".into() }))]
#[case("download_agent_model", json!({}), Some(VoiceEvent::DownloadAgentModel))]
#[case("cancel_agent_model_download", json!({}), Some(VoiceEvent::CancelAgentModelDownload))]
fn voice_event_known_action_parses(#[case] name: &str, #[case] context: Value, #[case] expected: Option<VoiceEvent>) {
    assert_that!(voice_event(name, &ctx(context))).is_equal_to(expected);
}

#[rstest]
#[case("model", json!({}))]
#[case("language", json!({}))]
#[case("download", json!({}))]
#[case("select", json!({}))]
#[case("silence", json!({"value": "0"}))]
#[case("silence", json!({"value": "-1"}))]
#[case("silence", json!({"value": "inf"}))]
#[case("silence", json!({"value": "NaN"}))]
#[case("silence", json!({"value": "abc"}))]
#[case("api", json!({"field": "key", "value": "x"}))]
#[case("api", json!({"field": "url"}))]
#[case("dismiss", json!({}))]
#[case("nope", json!({}))]
fn voice_event_malformed_action_is_none(#[case] name: &str, #[case] context: Value) {
    assert_that!(voice_event(name, &ctx(context))).is_none();
}

#[rstest]
#[case(DictationPatch { model: Some("api".into()), ..Default::default() }, false)]
#[case(DictationPatch { api_model: Some("m".into()), ..Default::default() }, false)]
#[case(DictationPatch { api_url: Some("https://a.example".into()), ..Default::default() }, true)]
#[case(DictationPatch { api_key_env: Some("K".into()), ..Default::default() }, true)]
fn dictation_patch_touches_api_target_with_url_or_variable(#[case] p: DictationPatch, #[case] touches: bool) {
    assert_that!(p.touches_api_target()).is_equal_to(touches);
}

#[rstest]
#[case(DictationPatch::default(), true)]
#[case(DictationPatch { auto_stop: Some(true), ..Default::default() }, false)]
fn dictation_patch_is_empty_only_without_fields(#[case] p: DictationPatch, #[case] empty: bool) {
    assert_that!(p.is_empty()).is_equal_to(empty);
}
```

- [ ] **Step 4: Run them to see them fail.** `cargo nextest run -p oku-slides-session voice_`. Expected: compile error.

- [ ] **Step 5: Write `voice_view.rs`** (module doc `//! Ce que montre la boîte des réglages de la dictée et de l'agent.` then "Aucun texte traduit : la boîte le traduit à chaque construction."): the types above, `DICTATION_LANGUAGES = [("fr", "Français"), ("en", "English"), ("de", "Deutsch"), ("es", "Español"), ("it", "Italiano"), ("pt", "Português"), ("nl", "Nederlands")]`, and:

```rust
impl DictationPatch {
    /// Aucun réglage n'est demandé.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Le changement touche l'adresse ou la variable : ce qui décide où part le secret.
    pub fn touches_api_target(&self) -> bool {
        self.api_url.is_some() || self.api_key_env.is_some()
    }
}

/// L'évènement d'une action de la boîte et de son contexte ; `None` si mal formé.
pub fn voice_event(name: &str, context: &Map<String, Value>) -> Option<VoiceEvent> {
    let text = |key: &str| context.get(key).and_then(Value::as_str).map(str::to_owned);
    let patch = |p: DictationPatch| Some(VoiceEvent::Dictation(p));
    match name {
        "model" => patch(DictationPatch { model: Some(text("id")?), ..Default::default() }),
        "language" => {
            let code = text("code")?;
            let language = if code.is_empty() { DictationLanguage::Auto } else { DictationLanguage::Code(code) };
            patch(DictationPatch { language: Some(language), ..Default::default() })
        }
        "silence" => match text("value")?.as_str() {
            "off" => patch(DictationPatch { auto_stop: Some(false), ..Default::default() }),
            v => {
                let secs = v.parse::<f32>().ok().filter(|s| s.is_finite() && *s > 0.0)?;
                patch(DictationPatch { auto_stop: Some(true), silence: Some(secs), ..Default::default() })
            }
        },
        "api" => {
            let value = text("value")?;
            let mut p = DictationPatch::default();
            match text("field")?.as_str() {
                "url" => p.api_url = Some(value),
                "model" => p.api_model = Some(value),
                "key_env" => p.api_key_env = Some(value),
                _ => return None,
            }
            patch(p)
        }
        "download" => Some(VoiceEvent::DownloadDictation { model: text("model")? }),
        "cancel_download" => Some(VoiceEvent::CancelDictationDownload),
        "select" => Some(VoiceEvent::SelectAgent { id: text("id")? }),
        "download_agent_model" => Some(VoiceEvent::DownloadAgentModel),
        "cancel_agent_model_download" => Some(VoiceEvent::CancelAgentModelDownload),
        _ => None,
    }
}
```

- [ ] **Step 6: Write `voice_panel.rs`.** Module doc `//! La boîte « Dictée et agent » de l'hôte, traduite à chaque construction.`. `VoicePanel { components: Vec<Component>, data: Value }` with `build`, `surface` (`surface(HOST_VOICE, Place::Thread, None, …)`), `same_layout` (`self.components == other.components`), `data_messages` (`vec![ServerMessage::set_data_model(HOST_VOICE, self.data.clone())]`). `build` starts `data` as `json!({ "choice": active, "progress": { "dictation": "", "agent": "" }, "notice": { "busy": "", "pending": "" } })`, runs `dictation` (if any) then `agents`, pushes `close` (`Component::button("close", l.tr("agent-surface-close"), false, Action::named("dismiss")).icon(ButtonIcon::Close)`), and puts the root first: `let refs: Vec<&str> = b.children.iter().map(String::as_str).collect(); Component::column("root", &refs)`.

  A private `Builder { children: Vec<String>, components: Vec<Component> }` with `push(c)` (records `c.id`) — the same small shape as `deck_packs_panel`'s; a shared builder in `agent::forms` is a later cleanup (not in scope). Rules:
  - **dictation(d, l, data)** fills `data["model"|"language"|"silence"|"api"]` and pushes:
    - `d-title` (`voice-dictation-title`, H4); `d-intro` (`voice-dictation-intro-apple` if a model has `ModelKind::Apple`, else `voice-dictation-intro`);
    - `d-models`: `ChoiceChips { options, value: Dynamic::path("/model"), action: Some(Action::event("model", [("id", Dynamic::path("/model"))])) }`; options = available models plus the chosen one even if unavailable; `label = model_label(m, &d.api.model, l)`; `hint` = note (`voice-note-<id>`), size (`size_text`, when `size > 0`), state (`voice-api-ready` / `voice-api-key-missing {var}` for `Api`, `voice-installed` / `voice-not-installed` for `Local`, none for `Apple`), joined by ` · `;
    - `d-unavailable` (`voice-unavailable {models}`, short names joined by `, `) when a model other than the chosen one is unavailable;
    - when `d.model == "api"`: `TextField`s from `[("url", "d-api-url", "voice-api-url"), ("model", "d-api-model", "voice-api-model"), ("key_env", "d-api-key-env", "voice-api-key-env")]` — `ComponentKind::TextField { label: Dynamic::Literal(l.tr(key)), value: Dynamic::path(format!("/api/{field}")), action: Some(Action::event("api", [("field", lit(json!(field))), ("value", Dynamic::path(format!("/api/{field}")))])) }` — then `d-api-note`;
    - `d-language`: `Select { label: Dynamic::Literal(l.tr("voice-language")), value: Dynamic::path("/language"), options, action: Some(Action::event("language", [("code", Dynamic::path("/language"))])) }`; options: `""` first (`voice-language-auto-resolved {language}` when the chosen model is Apple and `detected` is in `DICTATION_LANGUAGES`, else `voice-language-auto`), then `DICTATION_LANGUAGES`, then the current code if unlisted (label = the code; `AppVoice` accepts it back, Task 3);
    - `d-silence`: `Select { label: voice-silence, value: Dynamic::path("/silence"), options, action: silence {value: /silence} }`; options `off` (`voice-silence-off`), then `[1.0, 1.5, 2.0, 3.0, 5.0]` plus the current value when `secs_value(current)` is not one of theirs, sorted with `f32::total_cmp`; ids `secs_value(v)` (`format!("{v}")`), labels `seconds_text(v, l)`;
    - `d-note` (`voice-phrases-note`);
    - `d-progress` = `Component::new("d-progress", ComponentKind::Text { text: Dynamic::path("/progress/dictation"), variant: None })`, always; when `d.download` is set: `data["progress"]["dictation"] = voice-downloading {name: model_label, percent}` (percent `received.saturating_mul(100) / total.max(1)`, `min(100)`, formatted with `NumberFormat`) and `d-cancel` (`voice-cancel`, `cancel_download`); else, when `d.idle`: chosen `Local` model available and not installed → `d-download` (`voice-download-now {size: size_text}`, action `download` {model: `lit(json!(id))`}); chosen `Apple` not installed → `d-download` labelled `voice-download-language`.
  - **agents(a, l, data)** pushes `a-title` (`voice-agent-title`, H4); `a-agents` (`ChoiceChips { options: presets (label = name, hint), value: Dynamic::path("/choice"), action: (!locked).then(select {id: /choice}) }`); the local state — `NeedsDownload { size }`: `a-local-reason` = `agent-local-needs-download {gigabytes: gigabytes_text(size, l)}` and `a-download` (`agent-model-download`, `download_agent_model`); `Running { percent }`: `data["progress"]["agent"] = agent-model-downloading {percent}` and `a-cancel` (`agent-model-cancel`, `cancel_agent_model_download`); `Failed { size, failure }`: `a-local-reason`, `a-error` (`Failed` → `agent-model-download-failed`, `Integrity` → `-integrity`, `Incomplete` → `-incomplete {got, expected}`, `Cancelled` → `-cancelled`), `a-download` —; `a-progress` bound to `/progress/agent`, always; `a-pending` bound to `/notice/pending`, always (`voice-agent-pending {name}` when pending: the preset's name, its id when the preset is gone); `a-busy` bound to `/notice/busy`, always (`voice-agent-busy` when busy); `a-locked` (`voice-agent-locked`) when locked.
  - Helpers:

```rust
/// Le nom court d'un modèle de dictée ; l'API montre son modèle.
pub fn model_label(model: &ModelView, api_model: &str, l: &Localizer) -> String {
    match model.kind {
        ModelKind::Api => l.tr_with("voice-model-api", &[("model", api_model.into())]),
        ModelKind::Apple | ModelKind::Local => l.tr(&format!("voice-model-{}", model.id)),
    }
}

/// Une taille en Mo (entiers) ou en Go (dixièmes), écrite dans la langue.
pub fn size_text(bytes: u64, l: &Localizer) -> String {
    #[expect(clippy::cast_precision_loss, reason = "taille affichée, arrondie")]
    let mb = (bytes as f64 / 1e6).round();
    let numbers = NumberFormat::for_lang(Some(l.locale()));
    if mb >= 1000.0 {
        l.tr_with("voice-size-gb", &[("value", numbers.format((mb / 100.0).round() / 10.0).into())])
    } else {
        l.tr_with("voice-size-mb", &[("value", numbers.format(mb).into())])
    }
}

/// Une durée en secondes, forme la plus courte, écrite dans la langue.
pub fn seconds_text(seconds: f32, l: &Localizer) -> String {
    let plain = secs_value(seconds);
    let value = NumberFormat::for_lang(Some(l.locale())).localize(&plain).unwrap_or(plain);
    l.tr_with("voice-seconds", &[("value", value.into())])
}

/// Des gigaoctets au dixième, écrits dans la langue.
pub fn gigabytes_text(bytes: u64, l: &Localizer) -> String {
    #[expect(clippy::cast_precision_loss, reason = "taille affichée, arrondie")]
    let gb = (bytes as f64 / 1e8).round() / 10.0;
    NumberFormat::for_lang(Some(l.locale())).format(gb)
}

/// La valeur d'une durée dans le modèle de données : `1.5`, `3`, `1.2`.
fn secs_value(v: f32) -> String {
    format!("{v}")
}
```

  (`secs_value` uses f32's shortest form, so 1.2 stays "1.2"; `localize` only swaps separators. `voice-model-<id>` only receives ids `AppVoice` built from `Model::id`.)

- [ ] **Step 7: Re-export** in `lib.rs` (explicit list; update the crate doc's table of contents).

- [ ] **Step 8: Run.** `cargo nextest run -p oku-slides-session voice_`. Expected: PASS.

- [ ] **Step 9: UI contract.** In `forms_tests.rs` `every_form()`, push the boxes of `voice_panel_tests::covering_views()` (declare that test module `#[cfg(test)] #[path = "voice_panel_tests.rs"] pub(crate) mod voice_panel_tests;` so `forms_tests` reaches it). Regenerate and validate: `OKU_UPDATE_FIXTURES=1 cargo nextest run -p oku-slides-session every_form_matches_the_ui_fixture && (cd ui && bun test src/a2ui/catalog.test.tsx)`. Expected: PASS.

- [ ] **Step 10: Spec check.** If a detail of this task departs from the spec (a name, a key, a component id), update the spec in this commit.

- [ ] **Step 11: Docs.** `crates/oku-slides-session/ARCHITECTURE.md`: extend the panels line with `voice_view`/`voice_panel` (`VoicePanel`, `HOST_VOICE`: an app-wide box, text-free view; `voice_fixture` behind `test-support`) (≤ 50 lines).

- [ ] **Step 12: Check and commit.** `cargo nextest run -p oku-slides-session -p oku-slides-i18n && just check` — PASS. Commit `feat(session) : boîte des réglages de la dictée et de l'agent, vue sans texte et évènements (t-1144, t-1147)`.

---

### Task 3: `VoicePort`, `AppVoice`, `PromptGuard` and the box in every tab (t-1148)

The port, its only implementation and their wiring land together, and the old picker commands are routed through the port in the same commit, so there is one writer and no dead code.

**Files:**
- Create: mcp `src/voice.rs`, `src/voice_tests.rs`, `src/hub/settings.rs`, `tests/voice.rs`; agent `src/voice_port.rs`, `src/voice_port_tests.rs`, `src/agent_switch.rs`, `src/agent_switch_tests.rs`, `src/stt_watch.rs`, `src/stt_watch_tests.rs`, `src/voice_refresh.rs`, `src/table.rs`
- Modify: mcp `Cargo.toml` (`[features] test-support = []`; dev-dependency `oku-slides-session = { workspace = true, features = ["test-support"] }`), `src/{lib.rs,hub.rs,hub/actions.rs,hub/error.rs,hub_tests.rs,hub_scopes_tests.rs}`, `tests/{common/mod.rs,guarded.rs}`; local-agent `Cargo.toml` (dev-dependency `oku-slides-mcp = { workspace = true, features = ["test-support"] }`), `tests/common/mod.rs`; agent `Cargo.toml` (same dev-dependency), `src/{lib.rs,stack.rs,tab_entry.rs,tab_entry_tests.rs,tab_agent.rs,choice.rs,choice_tests.rs,command_error.rs,command_error_tests.rs,error.rs,events.rs,events_tests.rs,turns.rs}`, `tests/{common/mod.rs,stack.rs,availability.rs}`; `agent.ftl` × 20; ARCHITECTURE.md and AGENTS.md of mcp and agent
- Read first: mcp `hub/packs.rs`, `hub/actions.rs` (`act`), `hub/error.rs`, `tests/packs.rs`, `tests/guarded.rs`; agent `stack.rs` (`handle`, `tab_closed`, `shutdown`, `emit_picker`), `tab_entry.rs` (`Out`, `emit` lines 24, 35; `TabEntry::start`), `tab_agent.rs` (`AgentHandle::spawn` 6 params, `Ctx`, `TabAgent::run`), `choice.rs` (`select` holds `active` across write and switch), `command_error.rs`, `error.rs:84` (`Unusable::LocalNeedsDownload`), `turns.rs` (two exhaustive `AgentEvent` matches), `bin/fake_agent.rs:187` (logs `mcp/result` with `{session, tool, out}`), `tests/common/mod.rs` (`stack_with_two_agents`: presets `faux`, `bis`; `stack_locked`; `wait_for`; `cmd`; `logged`; `sessions_in`), `tests/availability.rs`

**Interfaces:**

`oku-slides-mcp/src/voice.rs` (re-exported):

```rust
//! La dictée et l'agent de l'app, tels que la boîte et les outils les voient.

use oku_slides_session::{AgentPresetView, ModelView, VoiceEvent, VoiceView};
use snafu::{Backtrace, Snafu};

/// Qui demande un changement de la dictée ou de l'agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Un clic dans la boîte : appliqué tout de suite.
    User,
    /// Un outil de l'agent de cet onglet.
    Agent {
        /// L'onglet de l'agent qui demande.
        tab: String,
        /// L'utilisateur a confirmé ce que `check` demandait de confirmer.
        confirmed: bool,
    },
}

/// Quand un changement accepté prend effet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceApplied {
    /// Tout de suite.
    Now,
    /// Dès qu'aucune réponse ne tourne (changement d'agent).
    WhenIdle,
    /// Rien à changer.
    Unchanged,
}

/// L'API de dictée configurée par l'utilisateur, montrée avant de l'utiliser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiTarget {
    /// Hôte de l'adresse, montré seul.
    pub host: String,
    /// Adresse normalisée.
    pub url: String,
    /// Nom de la variable de la clé.
    pub key_env: String,
    /// Modèle de l'API qui sera utilisé.
    pub model: String,
}

/// Ce qu'une demande vérifiée changerait, pour la confirmer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Checked {
    /// La demande ne changerait rien.
    pub unchanged: bool,
    /// La dictée passerait par l'API : où elle irait.
    pub api: Option<ApiTarget>,
    /// Un modèle à télécharger, avec le modèle de l'API pour son nom.
    pub download: Option<(ModelView, String)>,
    /// L'agent choisi.
    pub agent: Option<AgentPresetView>,
    /// Taille du modèle de l'agent local à télécharger.
    pub agent_model_size: Option<u64>,
}

impl Checked {
    /// La demande attend une confirmation de l'utilisateur.
    pub fn needs_confirmation(&self) -> bool {
        self.api.is_some() || self.download.is_some() || self.agent.is_some() || self.agent_model_size.is_some()
    }
}

/// Pourquoi un changement de la dictée ou de l'agent est refusé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceErrorKind {
    /// L'hôte n'a pas de dictée.
    NoDictation,
    /// La dictée n'est pas au repos.
    Busy,
    /// Modèle de dictée inconnu.
    UnknownModel,
    /// Le modèle ne gère pas la langue choisie.
    Unavailable,
    /// Le modèle est déjà installé.
    Installed,
    /// Le modèle ne se télécharge pas.
    NotDownloadable,
    /// Rien à annuler.
    NothingToCancel,
    /// Silence hors de la plage acceptée.
    BadSilence,
    /// Langue de dictée inconnue.
    BadLanguage,
    /// Adresse d'API refusée.
    BadUrl,
    /// Nom de variable refusé.
    BadKeyEnv,
    /// Modèle d'API refusé.
    BadApiModel,
    /// Adresse et variable de l'API : réservées à l'utilisateur.
    ApiReservedToUser,
    /// L'état a changé depuis la question : il faudrait confirmer à nouveau.
    Changed,
    /// Rien à télécharger pour l'agent choisi.
    NothingToDownload,
    /// Agent absent de la configuration.
    UnknownAgent,
    /// Le choix de l'agent est imposé.
    AgentLocked,
    /// Le choix de l'agent n'a pas pu être enregistré.
    AgentNotSaved,
}

/// Un changement de la dictée ou de l'agent, refusé.
#[derive(Debug, Snafu)]
#[snafu(display("voice change refused: {kind:?}"))]
pub struct VoiceError {
    kind: VoiceErrorKind,
    backtrace: Backtrace,
}

impl VoiceError {
    /// Un refus de sorte `kind`.
    pub fn new(kind: VoiceErrorKind) -> Self {
        VoiceSnafu { kind }.build()
    }

    /// La sorte du refus.
    pub fn kind(&self) -> VoiceErrorKind {
        self.kind
    }
}

/// Implémenter ce trait pour brancher la dictée et l'agent sur la boîte.
///
/// Les trois méthodes peuvent bloquer (disque) : jamais sur le fil async.
pub trait VoicePort: Send + Sync + std::fmt::Debug {
    /// L'état à montrer.
    fn view(&self) -> VoiceView;
    /// Vérifie une demande sans rien changer ; dit ce qu'il faudrait confirmer.
    fn check(&self, event: &VoiceEvent, origin: &Origin) -> Result<Checked, VoiceError>;
    /// Applique une demande, vérifiée à nouveau.
    fn apply(&self, event: VoiceEvent, origin: Origin) -> Result<VoiceApplied, VoiceError>;
}
```

Plus, behind `#[cfg(any(test, feature = "test-support"))]` (`[features] test-support = []` in mcp; never used by production code — every production `SurfaceHub::new` gets an `AppVoice`):

```rust
/// Un port sans dictée ni agent, pour les tests qui n'y touchent pas.
#[derive(Debug, Default)]
pub struct NoVoice;

impl VoicePort for NoVoice {
    fn view(&self) -> VoiceView {
        VoiceView::default()
    }
    fn check(&self, _: &VoiceEvent, _: &Origin) -> Result<Checked, VoiceError> {
        Err(VoiceError::new(VoiceErrorKind::NoDictation))
    }
    fn apply(&self, _: VoiceEvent, _: Origin) -> Result<VoiceApplied, VoiceError> {
        Err(VoiceError::new(VoiceErrorKind::NoDictation))
    }
}
```

`impl Localize for VoiceError`: one key per kind in `agent.ftl` (en / fr given; 18 translations). `UnknownAgent`, `AgentLocked`, `AgentNotSaved` reuse `agent-error-unknown-agent`, `agent-error-agent-locked`, `agent-error-agent-not-saved`.

| Kind | Key | en | fr |
|---|---|---|---|
| NoDictation | `voice-error-no-dictation` | Dictation is not available here | La dictée n'est pas disponible ici |
| Busy | `voice-error-busy` | Dictation is in use: try again once it is idle | La dictée est en cours : réessayez une fois au repos |
| UnknownModel | `voice-error-unknown-model` | Unknown dictation model | Modèle de dictée inconnu |
| Unavailable | `voice-error-unavailable` | This model does not support the chosen language | Ce modèle ne gère pas la langue choisie |
| Installed | `voice-error-installed` | This model is already installed | Ce modèle est déjà installé |
| NotDownloadable | `voice-error-not-downloadable` | This model is not downloaded | Ce modèle ne se télécharge pas |
| NothingToCancel | `voice-error-nothing-to-cancel` | No download is running | Aucun téléchargement n'est en cours |
| BadSilence | `voice-error-bad-silence` | The silence must last 0.5 to 10 seconds | Le silence doit durer de 0,5 à 10 secondes |
| BadLanguage | `voice-error-bad-language` | Unknown dictation language | Langue de dictée inconnue |
| BadUrl | `voice-error-bad-url` | The API address must start with https:// (http:// only for this computer), without user name, parameters or special characters | L'adresse de l'API doit commencer par https:// (http:// seulement pour cet ordinateur), sans nom d'utilisateur, paramètres ni caractères spéciaux |
| BadKeyEnv | `voice-error-bad-key-env` | The key variable must be a name like OPENAI_API_KEY: capitals, digits and _ | La variable de la clé doit être un nom comme OPENAI_API_KEY : majuscules, chiffres et _ |
| BadApiModel | `voice-error-bad-api-model` | The API model name may only use letters, digits and . _ : / - | Le nom du modèle de l'API n'admet que lettres, chiffres et . _ : / - |
| ApiReservedToUser | `voice-error-api-reserved` | Only the user sets the API address and key variable: open the box with show_dictation and ask them | Seul l'utilisateur règle l'adresse et la variable de la clé de l'API : ouvrez la boîte avec show_dictation et demandez-le-lui |
| Changed | `voice-error-changed` | The settings changed since the question: ask again | Les réglages ont changé depuis la question : redemandez |
| NothingToDownload | `voice-error-nothing-to-download` | Nothing to download for this agent | Rien à télécharger pour cet agent |

`oku-slides-agent`:
- `voice_port.rs`: `pub(crate) struct AppVoice` (fields: `stt: Option<Stt>`, `choice: Arc<AgentChoice>`, `switch: AgentSwitch`, `revision: watch::Sender<u64>`, `edit: Mutex<()>`, `rt: Handle`, `me: Weak<AppVoice>`), `AppVoice::new(stt, choice, rt: Handle) -> Arc<AppVoice>` (`Arc::new_cyclic`; starts the watcher **after** construction with `Arc::downgrade`), `impl VoicePort`, inherent `pub(crate) fn revision(&self) -> watch::Receiver<u64>`, `prompt_started(&self, tab: &str) -> PromptGuard`, `tab_closed(&self, tab: &str)`, `shutdown(&self)`, `localizer(&self) -> Arc<Localizer>`, `availability(&self) -> Option<String>`.
- `agent_switch.rs`: `pub(crate) struct PromptGuard { voice: Weak<AppVoice>, id: u64 }` (Drop → `voice.prompt_ended(id)`), `pub(crate) struct AgentSwitch` holding `state: Mutex<SwitchState>` (`running: HashMap<u64, String>` guard id → tab, `next: u64`, `pending: Option<(String, String)>`, `epoch: u64`, `shut: bool`) and the switch rules (below).
- `stt_watch.rs`: `pub(crate) struct Fingerprint` (`#[derive(Debug, PartialEq)]`, `fn of(&Status) -> Self`) and `pub(crate) fn spawn(stt: Stt, voice: Weak<AppVoice>)`.
- `table.rs`: `pub(crate) struct Table` (moved from `stack.rs`, with `refuses`) and `pub(crate) fn links(&self) -> Vec<(String, TabLink)>`.
- `tab_agent.rs`: `pub(crate) struct ActorParts { tab, hub, tools, link, localizer, keeper }`; `AgentHandle::spawn` (public, unchanged signature) builds `ActorParts` and calls `Self::start(parts, None)`; new `pub(crate) fn spawn_counted(parts: ActorParts, voice: Arc<AppVoice>) -> Self` calls `start(parts, Some(voice))`; `Ctx` gains `voice: Option<Arc<AppVoice>>`.
- `tab_entry.rs`: `pub(crate) struct TabPorts { pub settings: Arc<AppSettings>, pub voice: Arc<AppVoice> }`; `TabEntry::start(session, platform, ports, link, rt, out)`; the actor is started with `spawn_counted`.
- `choice.rs`: `pub(crate) enum Refusal { UnknownAgent, Locked }`; `check(&self, id) -> Result<&AgentPreset, Refusal>`; `switch_to(&self, id: &str)` (no-op when `id` is active; else `link.switch(launch_of(preset))` and `active = id`, cheap); `persist(&self) -> Result<(), ConfigWriteError>` (under `saving: Mutex<()>`, writes the **current** `active()` to `agent.selected`); `agents_view(&self) -> AgentsView` (`busy` false, `pending` None; hint = `command` and `args` joined by spaces, `None` for a builtin); `local_panel(&self) -> Option<LocalModelView>`; `localizer(&self) -> Arc<Localizer>`. `select` is deleted.
- `command_error.rs`: delete `AgentLocked`, `AgentNotSaved` (and their `Localize` arms, tests, the `ConfigWriteError` import); keep `UnknownAgent` (used by `availability`).
- `error.rs`: `Unusable::LocalNeedsDownload` uses `oku_slides_session::gigabytes_text` (one GB format in the app).
- `AgentEvent::Voice` (`{"kind": "voice"}`); `AgentCommand::ShowSettings { tab }`.
- Hub: `SurfaceHub::new(session, localizer, voice: Arc<dyn VoicePort>)`; `pub fn show_settings(&self)`; `pub fn refresh_settings_with(&self, panel: &VoicePanel)`; `HubErrorKind::Voice`.

- [ ] **Step 1: Write the failing tests.**

`crates/oku-slides-mcp/src/voice_tests.rs`:

```rust
use oku_slides_i18n::{Localize, Localizer};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case(VoiceErrorKind::Busy, "La dictée est en cours : réessayez une fois au repos")]
#[case(VoiceErrorKind::NothingToCancel, "Aucun téléchargement n'est en cours")]
fn voice_error_localizes_in_french(#[case] kind: VoiceErrorKind, #[case] text: &str) {
    assert_that!(VoiceError::new(kind).localize(&Localizer::new(Some("fr")))).is_equal_to(text.to_owned());
}

#[rstest]
fn voice_error_every_kind_has_a_translation(
    #[values(
        VoiceErrorKind::NoDictation, VoiceErrorKind::Busy, VoiceErrorKind::UnknownModel, VoiceErrorKind::Unavailable,
        VoiceErrorKind::Installed, VoiceErrorKind::NotDownloadable, VoiceErrorKind::NothingToCancel,
        VoiceErrorKind::BadSilence, VoiceErrorKind::BadLanguage, VoiceErrorKind::BadUrl, VoiceErrorKind::BadKeyEnv,
        VoiceErrorKind::BadApiModel, VoiceErrorKind::ApiReservedToUser, VoiceErrorKind::Changed,
        VoiceErrorKind::NothingToDownload, VoiceErrorKind::UnknownAgent, VoiceErrorKind::AgentLocked,
        VoiceErrorKind::AgentNotSaved
    )]
    kind: VoiceErrorKind,
) {
    // `Localizer::tr` rend l'identifiant d'une clé absente.
    let text = VoiceError::new(kind).localize(&Localizer::new(Some("fr")));
    assert_that!(text).does_not_contain("voice-error-");
    assert_that!(text).does_not_contain("agent-error-");
}
```

(Two assertions on one localized string check one behaviour: "has a translation".)

Test ports. In mcp `tests/common/mod.rs`:

```rust
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
}
```

`Harness` gains `pub voice: Arc<FakeVoice>`, built in `start_with` (the private builder every `start*` constructor calls) and passed to `SurfaceHub::new`. Also add:

```rust
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

/// Le texte d'une confirmation et son drapeau `danger` (`forms::confirm`).
pub fn confirmation_of(surface: &Surface) -> (String, bool) {
    let json = serde_json::to_value(surface).unwrap_or_else(|e| panic!("surface : {e}"));
    let components = json["messages"]
        .as_array()
        .and_then(|m| m.iter().find_map(|m| m["updateComponents"]["components"].as_array().cloned()))
        .unwrap_or_default();
    let find = |id: &str| components.iter().find(|c| c["id"] == id).cloned().unwrap_or_default();
    let text = find("message")["text"].as_str().unwrap_or_default().to_owned();
    (text, find("yes")["danger"] == Value::Bool(true))
}

/// Comme [`guarded`], et rend le texte de la confirmation et son drapeau `danger`.
pub async fn guarded_message(h: &Harness, tool: &'static str, args: Value, confirmed: bool) -> (String, bool, CallToolResult) {
    let mut rx = h.hub.subscribe();
    let c = server(h).await;
    let pending = tokio::spawn(async move { call(&c, tool, args).await });
    let surface = next_surface(&mut rx).await;
    let (text, danger) = confirmation_of(&surface);
    answer(h, &surface.id, confirmed);
    (text, danger, pending.await.unwrap_or_else(|e| panic!("tâche de l'outil : {e}")))
}
```

Move `answer` and `guarded` from `tests/guarded.rs` into common (`pub`); `guarded.rs` imports them. `hub_tests.rs`, `hub_scopes_tests.rs`, local-agent and agent `tests/common` use `oku_slides_mcp::NoVoice`.

`crates/oku-slides-mcp/tests/voice.rs` (hub level):

```rust
mod common;

use common::*;
use oku_slides_i18n::Localize;
use oku_slides_mcp::{HubErrorKind, McpEvent, Origin, SurfaceAction, VoiceErrorKind};
use oku_slides_session::{DictationPatch, DownloadView, HOST_VOICE, VoiceEvent, VoicePanel, VoiceView};
use rstest::rstest;
use serde_json::{Map, Value, json};
use speculoos::prelude::*;

fn ctx(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    tokio::task::spawn_blocking(f).await.unwrap_or_else(|e| panic!("tâche bloquante : {e}"))
}

async fn show(h: &Harness) {
    let hub = h.hub.clone();
    blocking(move || hub.show_settings()).await;
}

async fn refresh(h: &Harness) {
    let (hub, panel) = (h.hub.clone(), VoicePanel::build(&h.voice.view(), &h.localizer));
    blocking(move || hub.refresh_settings_with(&panel)).await;
}

async fn act(h: &Harness, name: &str, context: Value) -> Result<(), oku_slides_mcp::HubError> {
    let hub = h.hub.clone();
    let action = SurfaceAction { surface: HOST_VOICE.into(), name: name.into(), context: ctx(context) };
    blocking(move || hub.act(action)).await
}

fn shown(h: &Harness) -> bool {
    h.hub.surfaces().iter().any(|s| s.id == HOST_VOICE)
}

fn is_box(e: &McpEvent) -> bool {
    matches!(e, McpEvent::Surface { surface } if surface.id == HOST_VOICE)
}

fn is_patch(e: &McpEvent) -> bool {
    matches!(e, McpEvent::SurfacePatch { id, .. } if id == HOST_VOICE)
}

/// Le modèle de données d'une boîte ou d'un patch de la boîte.
fn data_of(e: &McpEvent) -> Value {
    let messages = match e {
        McpEvent::Surface { surface } => serde_json::to_value(&surface.messages).unwrap_or_default(),
        McpEvent::SurfacePatch { messages, .. } => serde_json::to_value(messages).unwrap_or_default(),
        _ => Value::Null,
    };
    messages.as_array().and_then(|m| m.iter().find_map(|m| m.get("updateDataModel").cloned())).unwrap_or_default()["value"].clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn show_settings_puts_the_box_live_outside_any_turn() {
    let h = start().await;
    show(&h).await;
    let s = h.hub.surfaces().into_iter().find(|s| s.id == HOST_VOICE).unwrap();
    assert_that!((s.live, s.turn)).is_equal_to((true, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn show_settings_survives_a_new_conversation() {
    let h = start().await;
    show(&h).await;
    h.hub.clear_thread(&oku_slides_session::agent::AgentScope::Slides);
    assert_that!(shown(&h)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_click_applies_as_user() {
    let h = start().await;
    show(&h).await;
    act(&h, "model", json!({"id": "large"})).await.unwrap();
    let patch = DictationPatch { model: Some("large".into()), ..Default::default() };
    assert_that!(*lock(&h.voice.applied)).is_equal_to(vec![(VoiceEvent::Dictation(patch), Origin::User)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_click_with_the_same_layout_sends_a_patch() {
    let h = start().await;
    show(&h).await;
    lock(&h.voice.view).dictation.as_mut().unwrap().language = Some("fr".into());
    let mut rx = h.hub.subscribe();
    act(&h, "language", json!({"code": "fr"})).await.unwrap();
    assert_that!(rx.try_recv()).is_ok().matches(is_patch);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_click_with_a_new_layout_sends_the_box() {
    let h = start().await;
    show(&h).await;
    lock(&h.voice.view).dictation.as_mut().unwrap().model = "api".into();
    let mut rx = h.hub.subscribe();
    act(&h, "model", json!({"id": "api"})).await.unwrap();
    assert_that!(rx.try_recv()).is_ok().matches(is_box);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_dismiss_closes_the_box() {
    let h = start().await;
    show(&h).await;
    act(&h, "dismiss", json!({})).await.unwrap();
    assert_that!(shown(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_after_dismiss_does_not_reopen() {
    let h = start().await;
    show(&h).await;
    act(&h, "dismiss", json!({})).await.unwrap();
    refresh(&h).await;
    assert_that!(shown(&h)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_click_on_a_closed_box_is_unknown() {
    let h = start().await;
    let r = act(&h, "model", json!({"id": "large"})).await;
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(HubErrorKind::UnknownSurface);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_malformed_click_is_incomplete() {
    let h = start().await;
    show(&h).await;
    let r = act(&h, "model", json!({})).await;
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(HubErrorKind::Incomplete);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_malformed_click_keeps_the_box() {
    let h = start().await;
    show(&h).await;
    let _ = act(&h, "model", json!({})).await;
    assert_that!(shown(&h)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_refused_click_is_a_voice_error() {
    let h = start().await;
    show(&h).await;
    *lock(&h.voice.refuse) = Some(VoiceErrorKind::Busy);
    let r = act(&h, "cancel_download", json!({})).await;
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(HubErrorKind::Voice);
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_refused_click_says_why_in_french() {
    let h = start().await;
    show(&h).await;
    *lock(&h.voice.refuse) = Some(VoiceErrorKind::Busy);
    let e = act(&h, "cancel_download", json!({})).await.unwrap_err();
    assert_that!(e.localize(&h.localizer)).is_equal_to("La dictée est en cours : réessayez une fois au repos".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_refused_click_resends_the_whole_box() {
    let h = start().await;
    show(&h).await;
    *lock(&h.voice.refuse) = Some(VoiceErrorKind::Busy);
    let mut rx = h.hub.subscribe();
    let _ = act(&h, "model", json!({"id": "large"})).await;
    assert_that!(rx.try_recv()).is_ok().matches(is_box);
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_unchanged_sends_nothing() {
    let h = start().await;
    show(&h).await;
    let mut rx = h.hub.subscribe();
    refresh(&h).await;
    assert_that!(rx.try_recv()).is_err();
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_with_a_new_layout_sends_the_box() {
    let h = start().await;
    show(&h).await;
    lock(&h.voice.view).dictation.as_mut().unwrap().model = "api".into();
    let mut rx = h.hub.subscribe();
    refresh(&h).await;
    assert_that!(rx.try_recv()).is_ok().matches(is_box);
}

fn downloading(received: u64) -> impl Fn(&mut VoiceView) {
    move |v| v.dictation.as_mut().unwrap().download = Some(DownloadView { model: "large".into(), received, total: 4 })
}

#[rstest]
#[case::busy(|v: &mut VoiceView| v.agents.busy = true, "/notice/busy", "Changer d'agent interrompra le tour en cours.")]
#[case::pending(|v: &mut VoiceView| v.agents.pending = Some("local".into()), "/notice/pending", "Passage à Local dès qu'aucune réponse ne tourne.")]
#[tokio::test(flavor = "multi_thread")]
async fn refresh_with_only_new_notices_sends_their_text_as_a_patch(
    #[case] change: fn(&mut VoiceView),
    #[case] path: &str,
    #[case] text: &str,
) {
    let h = start().await;
    show(&h).await;
    change(&mut lock(&h.voice.view));
    let mut rx = h.hub.subscribe();
    refresh(&h).await;
    let e = rx.try_recv().unwrap();
    assert_that!((is_patch(&e), data_of(&e).pointer(path).cloned())).is_equal_to((true, Some(json!(text))));
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_with_only_new_progress_sends_a_patch() {
    let h = start().await;
    downloading(0)(&mut lock(&h.voice.view));
    show(&h).await;
    downloading(1)(&mut lock(&h.voice.view));
    let mut rx = h.hub.subscribe();
    refresh(&h).await;
    assert_that!(rx.try_recv()).is_ok().matches(is_patch);
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_when_not_shown_sends_nothing() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    refresh(&h).await;
    assert_that!(rx.try_recv()).is_err();
}
```

(`HubErrorKind::UnknownSurface`: use the real kind name of `HubError::unknown_surface`. The negative `try_recv` checks are sound: each hub call has returned. Check the `rstest` + `#[tokio::test]` attribute order against an existing async rstest in mcp tests. The notices test asserts one behaviour — "the change goes as a patch carrying the text" — as a pair.)

`crates/oku-slides-agent/src/agent_switch_tests.rs` and `voice_port_tests.rs` share a test builder; put it in `voice_port_tests.rs` as `pub(crate)` and declare that module `pub(crate)` (`#[cfg(test)] #[path = "voice_port_tests.rs"] pub(crate) mod voice_port_tests;`):

```rust
use std::sync::Arc;
use std::time::{Duration, Instant};

use oku_slides_mcp::{Origin, VoiceApplied, VoiceErrorKind, VoicePort};
use oku_slides_platform::config::AgentPreset;
use oku_slides_session::{DictationLanguage, DictationPatch, VoiceEvent};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

/// Une boîte de test et ce qui la garde en vie.
pub(crate) struct T {
    pub voice: Arc<AppVoice>,
    pub stt: Option<Stt>,
    pub data: std::path::PathBuf,
    _rt: tokio::runtime::Runtime,
    _dir: tempfile::TempDir,
}

impl T {
    /// Le choix d'agent écrit dans `config.toml`.
    pub fn saved_agent(&self) -> Option<String> {
        let toml = std::fs::read_to_string(self.data.join("config.toml")).unwrap_or_default();
        toml.lines().find_map(|l| l.strip_prefix("selected = \"")).map(|v| v.trim_end_matches('"').to_owned())
    }

    /// Attend que `cond` tienne, deux secondes au plus ; échoue sinon.
    pub fn until(&self, what: &str, cond: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !cond(self) {
            assert!(Instant::now() < deadline, "attente dépassée : {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn preset(name: &str) -> AgentPreset {
    AgentPreset { name: name.into(), command: "true".into(), args: vec![], builtin: None }
}

/// Une plateforme de test à deux agents (`faux` choisi, `bis`), retouchée par `tweak`.
pub(crate) fn build(with_stt: bool, tweak: impl FnOnce(&mut oku_slides_session::Settings)) -> T {
    let dir = tempfile::Builder::new().prefix("oku-voice-").tempdir().unwrap();
    let data = dir.path().to_path_buf();
    let mut settings = oku_slides_session::Settings::defaults().unwrap();
    settings.paths.data = Some(data.clone());
    settings.paths.decks = Some(data.join("decks"));
    settings.agent.presets = [("faux".to_owned(), preset("Faux")), ("bis".to_owned(), preset("Bis"))].into_iter().collect();
    settings.agent.selected = "faux".into();
    settings.i18n.locale = Some("fr".into());
    settings.stt.apple = false;
    tweak(&mut settings);
    let platform = oku_slides_session::Platform::with_settings(settings).unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let choice = Arc::new(AgentChoice::new(&platform, crate::Link::new(AgentChoice::launch_of(&preset("Faux"), &platform))));
    let stt = with_stt.then(|| Stt::new(&platform));
    T { voice: AppVoice::new(stt.clone(), choice, rt.handle().clone()), stt, data, _rt: rt, _dir: dir }
}

pub(crate) fn voice_with_stt() -> T {
    build(true, |_| {})
}

fn voice_without_stt() -> T {
    build(false, |_| {})
}

fn voice_locked() -> T {
    build(true, |s| s.agent.locked = true)
}

fn dictation(p: DictationPatch) -> VoiceEvent {
    VoiceEvent::Dictation(p)
}

fn agent_origin(tab: &str, confirmed: bool) -> Origin {
    Origin::Agent { tab: tab.into(), confirmed }
}
```

(`presets` is collected into the map type `AgentSettings.presets` uses — read it in `config.rs`. These are plain `#[test]`s, like `choice_tests.rs`.) `voice_port_tests.rs` tests:

```rust
#[rstest]
#[case(DictationPatch { model: Some("nope".into()), ..Default::default() }, VoiceErrorKind::UnknownModel)]
#[case(DictationPatch { silence: Some(0.2), ..Default::default() }, VoiceErrorKind::BadSilence)]
#[case(DictationPatch { silence: Some(30.0), ..Default::default() }, VoiceErrorKind::BadSilence)]
#[case(DictationPatch { language: Some(DictationLanguage::Code("xx".into())), ..Default::default() }, VoiceErrorKind::BadLanguage)]
#[case(DictationPatch { api_url: Some("http://evil.example/v1".into()), ..Default::default() }, VoiceErrorKind::BadUrl)]
#[case(DictationPatch { api_key_env: Some("sk-abc-123".into()), ..Default::default() }, VoiceErrorKind::BadKeyEnv)]
#[case(DictationPatch { api_model: Some("m\r\nx".into()), ..Default::default() }, VoiceErrorKind::BadApiModel)]
fn apply_invalid_patch_from_the_user_is_refused(#[case] patch: DictationPatch, #[case] kind: VoiceErrorKind) {
    let t = voice_with_stt();
    assert_that!(t.voice.apply(dictation(patch), Origin::User).map_err(|e| e.kind())).is_err_containing(kind);
}

#[rstest]
#[case(DictationPatch { api_url: Some("https://api.groq.com/openai/v1".into()), ..Default::default() })]
#[case(DictationPatch { api_key_env: Some("GROQ_API_KEY".into()), ..Default::default() })]
fn check_api_target_from_the_agent_is_reserved_to_the_user(#[case] patch: DictationPatch) {
    let t = voice_with_stt();
    let r = t.voice.check(&dictation(patch), &agent_origin("t-1", false));
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::ApiReservedToUser);
}

#[rstest]
#[case(DictationPatch { api_url: Some("https://api.groq.com/openai/v1".into()), ..Default::default() })]
#[case(DictationPatch { api_key_env: Some("GROQ_API_KEY".into()), ..Default::default() })]
fn apply_api_target_from_the_agent_is_reserved_to_the_user(#[case] patch: DictationPatch) {
    let t = voice_with_stt();
    let r = t.voice.apply(dictation(patch), agent_origin("t-1", true));
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::ApiReservedToUser);
}

#[test]
fn apply_api_url_from_the_user_is_stored_normalized() {
    let t = voice_with_stt();
    let patch = DictationPatch { api_url: Some("HTTPS://API.groq.com/openai/v1/".into()), ..Default::default() };
    t.voice.apply(dictation(patch), Origin::User).unwrap();
    assert_that!(t.voice.view().dictation.unwrap().api.url).is_equal_to("https://api.groq.com/openai/v1".to_owned());
}

#[test]
fn apply_invalid_patch_changes_nothing() {
    let t = voice_with_stt();
    let before = t.voice.view();
    let _ = t.voice.apply(dictation(DictationPatch { silence: Some(0.2), ..Default::default() }), Origin::User);
    assert_that!(t.voice.view()).is_equal_to(before);
}

#[test]
fn apply_language_patch_configures_stt() {
    let t = voice_with_stt();
    let patch = DictationPatch { language: Some(DictationLanguage::Code("fr".into())), ..Default::default() };
    t.voice.apply(dictation(patch), Origin::User).unwrap();
    assert_that!(t.voice.view().dictation.unwrap().language).is_equal_to(Some("fr".to_owned()));
}

#[test]
fn apply_the_stored_unlisted_language_is_accepted() {
    let t = voice_with_stt();
    let stt = t.stt.as_ref().unwrap();
    let mut config = stt.status().config;
    config.language = Some("ja".into());
    // Une langue d'avant la liste fixe, écrite directement : seul son retour est testé.
    let _applied = stt.configure(config);
    let patch = DictationPatch { language: Some(DictationLanguage::Code("ja".into())), ..Default::default() };
    assert_that!(t.voice.apply(dictation(patch), Origin::User).map_err(|e| e.kind())).is_ok_containing(VoiceApplied::Unchanged);
}

#[test]
fn apply_accepted_patch_bumps_the_revision() {
    let t = voice_with_stt();
    let rx = t.voice.revision();
    let before = *rx.borrow();
    t.voice.apply(dictation(DictationPatch { auto_stop: Some(false), ..Default::default() }), Origin::User).unwrap();
    assert_that!(*rx.borrow()).is_greater_than(before);
}

#[test]
fn apply_refused_patch_keeps_the_revision() {
    let t = voice_with_stt();
    let rx = t.voice.revision();
    let before = *rx.borrow();
    let _ = t.voice.apply(dictation(DictationPatch { silence: Some(0.1), ..Default::default() }), Origin::User);
    assert_that!(*rx.borrow()).is_equal_to(before);
}

#[test]
fn apply_same_value_is_unchanged() {
    let t = voice_with_stt();
    let current = t.voice.view().dictation.unwrap().model;
    let r = t.voice.apply(dictation(DictationPatch { model: Some(current), ..Default::default() }), Origin::User);
    assert_that!(r.map_err(|e| e.kind())).is_ok_containing(VoiceApplied::Unchanged);
}

#[test]
fn check_switch_to_api_from_the_agent_names_the_user_target() {
    let t = voice_with_stt();
    let checked = t.voice.check(&dictation(DictationPatch { model: Some("api".into()), ..Default::default() }), &agent_origin("t-1", false)).unwrap();
    let target = checked.api.unwrap();
    assert_that!((target.host.as_str(), target.key_env.as_str())).is_equal_to(("api.openai.com", "OPENAI_API_KEY"));
}

#[test]
fn check_switch_to_api_names_the_requested_api_model() {
    let t = voice_with_stt();
    let patch = DictationPatch { model: Some("api".into()), api_model: Some("whisper-1".into()), ..Default::default() };
    let checked = t.voice.check(&dictation(patch), &agent_origin("t-1", false)).unwrap();
    assert_that!(checked.api.map(|a| a.model)).is_equal_to(Some("whisper-1".to_owned()));
}

fn store_invalid_url(t: &T) {
    // Une adresse d'avant les règles (t-1137), écrite directement dans la config de la dictée.
    let stt = t.stt.as_ref().unwrap();
    let mut config = stt.status().config;
    config.api.url = "http://192.168.1.5/v1".into();
    let _applied = stt.configure(config);
}

#[test]
fn check_switch_to_api_from_the_agent_with_an_invalid_stored_url_is_refused() {
    let t = voice_with_stt();
    store_invalid_url(&t);
    let r = t.voice.check(&dictation(DictationPatch { model: Some("api".into()), ..Default::default() }), &agent_origin("t-1", false));
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::BadUrl);
}

#[test]
fn apply_switch_to_api_from_the_user_with_an_invalid_stored_url_is_accepted() {
    // L'utilisateur doit pouvoir passer à l'API pour voir les champs et corriger l'adresse.
    let t = voice_with_stt();
    store_invalid_url(&t);
    let r = t.voice.apply(dictation(DictationPatch { model: Some("api".into()), ..Default::default() }), Origin::User);
    assert_that!(r.map_err(|e| e.kind())).is_ok_containing(VoiceApplied::Now);
}

#[test]
fn check_switch_to_an_uninstalled_model_names_its_download() {
    let t = voice_with_stt();
    let checked = t.voice.check(&dictation(DictationPatch { model: Some("large".into()), ..Default::default() }), &Origin::User).unwrap();
    assert_that!(checked.download.map(|(m, _)| m.id)).is_equal_to(Some("large".to_owned()));
}

#[test]
fn check_download_of_an_uninstalled_model_names_it() {
    let t = voice_with_stt();
    let checked = t.voice.check(&VoiceEvent::DownloadDictation { model: "large".into() }, &Origin::User).unwrap();
    assert_that!(checked.download.map(|(m, _)| m.id)).is_equal_to(Some("large".to_owned()));
}

#[test]
fn apply_unconfirmed_agent_switch_to_api_is_changed() {
    let t = voice_with_stt();
    let r = t.voice.apply(dictation(DictationPatch { model: Some("api".into()), ..Default::default() }), agent_origin("t-1", false));
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::Changed);
}

#[test]
fn view_api_key_found_for_a_set_variable() {
    let t = voice_with_stt();
    let patch = DictationPatch { model: Some("api".into()), api_key_env: Some("HOME".into()), ..Default::default() };
    t.voice.apply(dictation(patch), Origin::User).unwrap();
    assert_that!(t.voice.view().dictation.unwrap().api.key_found).is_true();
}

#[rstest]
#[case(VoiceEvent::DownloadDictation { model: "api".into() }, VoiceErrorKind::NotDownloadable)]
#[case(VoiceEvent::DownloadDictation { model: "nope".into() }, VoiceErrorKind::UnknownModel)]
#[case(VoiceEvent::CancelDictationDownload, VoiceErrorKind::NothingToCancel)]
#[case(VoiceEvent::CancelAgentModelDownload, VoiceErrorKind::NothingToCancel)]
#[case(VoiceEvent::SelectAgent { id: "nope".into() }, VoiceErrorKind::UnknownAgent)]
fn check_refuses_before_any_change(#[case] event: VoiceEvent, #[case] kind: VoiceErrorKind) {
    let t = voice_with_stt();
    assert_that!(t.voice.check(&event, &Origin::User).map_err(|e| e.kind())).is_err_containing(kind);
}

#[test]
fn check_agent_when_locked_is_refused() {
    let t = voice_locked();
    let r = t.voice.check(&VoiceEvent::SelectAgent { id: "bis".into() }, &Origin::User);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::AgentLocked);
}

#[rstest]
#[case(oku_slides_stt::State::Idle, false)]
#[case(oku_slides_stt::State::Recording, false)]
#[case(oku_slides_stt::State::Transcribing, false)]
#[case(oku_slides_stt::State::Downloading, true)]
fn cancel_dictation_allowed_only_while_downloading(#[case] state: oku_slides_stt::State, #[case] allowed: bool) {
    assert_that!(cancel_dictation_allowed(state)).is_equal_to(allowed);
}

#[test]
fn view_without_stt_has_no_dictation() {
    let t = voice_without_stt();
    assert_that!(t.voice.view().dictation).is_none();
}

#[test]
fn dictation_event_without_stt_is_no_dictation() {
    let t = voice_without_stt();
    let r = t.voice.check(&VoiceEvent::CancelDictationDownload, &Origin::User);
    assert_that!(r.map_err(|e| e.kind())).is_err_containing(VoiceErrorKind::NoDictation);
}
```

(`cancel_dictation_allowed(state: State) -> bool` is the pure rule in `voice_port.rs`: only `Downloading`; `stt.cancel()` would reset a recording, so it is never called otherwise. "Installed" is covered by the stt `download_of_an_installed_model_returns_false` test and by `Checked` in mcp: the test platform has no model files.)

`agent_switch_tests.rs` (deterministic: `switch_now` is called directly with a captured epoch; no sleeps):

```rust
use oku_slides_mcp::{Origin, VoiceApplied, VoicePort};
use oku_slides_session::VoiceEvent;
use speculoos::prelude::*;

use crate::voice_port::voice_port_tests::{T, voice_with_stt};

fn select(id: &str) -> VoiceEvent {
    VoiceEvent::SelectAgent { id: id.into() }
}

fn agent(tab: &str) -> Origin {
    Origin::Agent { tab: tab.into(), confirmed: true }
}

/// Une requête d'agent en attente pendant que `t-1` tourne ; rend la garde et l'époque.
fn pending_bis(t: &T) -> (crate::agent_switch::PromptGuard, u64) {
    let guard = t.voice.prompt_started("t-1");
    t.voice.apply(select("bis"), agent("t-1")).unwrap();
    (guard, t.voice.switch_epoch())
}

#[test]
fn select_agent_from_agent_while_prompting_waits() {
    let t = voice_with_stt();
    let _guard = t.voice.prompt_started("t-1");
    assert_that!(t.voice.apply(select("bis"), agent("t-1")).map_err(|e| e.kind())).is_ok_containing(VoiceApplied::WhenIdle);
}

#[test]
fn select_agent_from_agent_with_nothing_running_applies_now() {
    let t = voice_with_stt();
    assert_that!(t.voice.apply(select("bis"), agent("t-1")).map_err(|e| e.kind())).is_ok_containing(VoiceApplied::Now);
}

#[test]
fn select_agent_while_prompting_is_shown_as_pending() {
    let t = voice_with_stt();
    let _keep = pending_bis(&t);
    assert_that!(t.voice.view().agents.pending).is_equal_to(Some("bis".to_owned()));
}

#[test]
fn view_busy_while_any_tab_prompts() {
    let t = voice_with_stt();
    let _guard = t.voice.prompt_started("t-2");
    assert_that!(t.voice.view().agents.busy).is_true();
}

#[test]
fn prompt_start_bumps_the_revision() {
    let t = voice_with_stt();
    let rx = t.voice.revision();
    let before = *rx.borrow();
    let _guard = t.voice.prompt_started("t-1");
    assert_that!(*rx.borrow()).is_greater_than(before);
}

#[test]
fn second_prompt_does_not_bump_the_revision() {
    let t = voice_with_stt();
    let _first = t.voice.prompt_started("t-1");
    let rx = t.voice.revision();
    let before = *rx.borrow();
    let _second = t.voice.prompt_started("t-2");
    assert_that!(*rx.borrow()).is_equal_to(before);
}

#[test]
fn pending_agent_stays_pending_while_another_tab_prompts() {
    let t = voice_with_stt();
    let _other = t.voice.prompt_started("t-2");
    let (guard, _) = pending_bis(&t);
    drop(guard);
    assert_that!(t.voice.view().agents.pending).is_equal_to(Some("bis".to_owned()));
}

#[test]
fn pending_agent_applies_when_the_last_prompt_ends() {
    let t = voice_with_stt();
    let (guard, _) = pending_bis(&t);
    drop(guard);
    t.until("bis actif", |t| t.voice.view().agents.active == "bis");
}

#[test]
fn applied_pending_agent_is_saved() {
    let t = voice_with_stt();
    let (guard, _) = pending_bis(&t);
    drop(guard);
    t.until("bis enregistré", |t| t.saved_agent().as_deref() == Some("bis"));
}

#[test]
fn switch_with_an_old_epoch_after_a_user_click_changes_nothing() {
    let t = voice_with_stt();
    let (_guard, epoch) = pending_bis(&t);
    t.voice.apply(select("faux"), Origin::User).unwrap();
    t.voice.switch_now("t-1", "bis", epoch);
    assert_that!(t.voice.view().agents.active).is_equal_to("faux".to_owned());
}

#[test]
fn switch_with_an_old_epoch_saves_nothing() {
    let t = voice_with_stt();
    let (_guard, epoch) = pending_bis(&t);
    t.voice.apply(select("faux"), Origin::User).unwrap();
    t.voice.switch_now("t-1", "bis", epoch);
    assert_that!(t.saved_agent()).is_not_equal_to(Some("bis".to_owned()));
}

#[test]
fn switch_while_a_prompt_runs_keeps_the_choice_pending() {
    let t = voice_with_stt();
    let (_guard, epoch) = pending_bis(&t);
    t.voice.switch_now("t-1", "bis", epoch);
    let agents = t.voice.view().agents;
    assert_that!((agents.active.as_str(), agents.pending)).is_equal_to(("faux", Some("bis".to_owned())));
}

#[test]
fn user_select_drops_a_pending_one() {
    let t = voice_with_stt();
    let _keep = pending_bis(&t);
    t.voice.apply(select("faux"), Origin::User).unwrap();
    assert_that!(t.voice.view().agents.pending).is_none();
}

#[test]
fn user_select_of_the_active_agent_does_not_restart_it() {
    let t = voice_with_stt();
    let launches = t.voice.link_epoch();
    t.voice.apply(select("faux"), Origin::User).unwrap();
    assert_that!(t.voice.link_epoch()).is_equal_to(launches);
}

#[test]
fn agent_request_for_the_active_one_clears_the_pending() {
    let t = voice_with_stt();
    let _keep = pending_bis(&t);
    t.voice.apply(select("faux"), agent("t-2")).unwrap();
    assert_that!(t.voice.view().agents.pending).is_none();
}

#[test]
fn closing_the_requesting_tab_drops_its_pending_agent() {
    let t = voice_with_stt();
    let _keep = pending_bis(&t);
    t.voice.tab_closed("t-1");
    assert_that!(t.voice.view().agents.pending).is_none();
}

#[test]
fn prompt_end_after_tab_closed_releases_its_guard() {
    let t = voice_with_stt();
    let guard = t.voice.prompt_started("t-1");
    t.voice.tab_closed("t-1");
    drop(guard);
    assert_that!(t.voice.view().agents.busy).is_false();
}

#[test]
fn shutdown_drops_the_pending_agent() {
    let t = voice_with_stt();
    let (guard, _) = pending_bis(&t);
    t.voice.shutdown();
    drop(guard);
    assert_that!(t.voice.view().agents.pending).is_none();
}

#[test]
fn switch_after_shutdown_changes_nothing() {
    let t = voice_with_stt();
    let (_guard, epoch) = pending_bis(&t);
    t.voice.shutdown();
    t.voice.switch_now("t-1", "bis", epoch);
    assert_that!(t.voice.view().agents.active).is_equal_to("faux".to_owned());
}
```

(Test-only accessors on `AppVoice`, `#[cfg(test)]`: `switch_epoch() -> u64`, `link_epoch() -> u64` (the `Link`'s launch counter, via a `#[cfg(test)] pub(crate) fn launch_epoch()` on `Link`), and `switch_now(&self, tab, id, epoch)` is the real `pub(crate)` method the deferred path spawns.)

`stt_watch_tests.rs`: `fingerprint_ignores_the_mic_level`, `fingerprint_follows_the_state` (both build a `Status` from `voice_with_stt().stt.status()` and mutate a clone).

`choice_tests.rs`: replace the `crate::picker::LocalPanel::NeedsDownload { reason }` expectation by `Some(LocalModelView::NeedsDownload { size: AGENT_MODEL_SIZE })`; add `check_unknown_is_refused`, `check_locked_is_refused`, `switch_to_the_active_agent_keeps_the_link`, `persist_writes_the_active_agent`, `agents_view_lists_presets_with_the_active_one`, `agents_view_builtin_has_no_hint`.

`tab_entry_tests.rs:89,102`: its direct `SurfaceHub::new` and its `Agent { session, hub, link, localizer, keeper }` literal get the voice: `let voice = AppVoice::new(None, Arc::new(AgentChoice::new(&platform, link.clone())), Handle::current());` passed to both.

Agent `tests/stack.rs` (new, plus the old picker tests ported **now**, since the commands go through the port in this task):

```rust
#[test]
fn show_settings_command_deserializes_with_its_tab() {
    assert_that!(cmd(json!({ "cmd": "show_settings", "tab": "t" })).tab().to_owned()).is_equal_to("t".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn show_settings_relays_the_box_to_its_tab() {
    let (stack, rx) = stack_with_sink(json!({ "turns": [] }), &["t-1"]).await;
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": "t-1" })))).unwrap();
    let (_, ev) = wait_for(&rx, |(ch, v)| *ch == "mcp" && v["event"]["kind"] == "surface" && v["event"]["surface"]["id"] == "host-voice").await;
    assert_that!(ev["tab"].clone()).is_equal_to(json!("t-1"));
}

/// Le choix d'agent d'une boîte relayée, entière ou en données seules.
fn choice_of(v: &serde_json::Value) -> Option<String> {
    let messages = v["event"]["surface"]["messages"].as_array().or_else(|| v["event"]["messages"].as_array())?;
    messages.iter().find_map(|m| m["updateDataModel"]["value"]["choice"].as_str().map(str::to_owned))
}

/// Le clic sur la pastille `id` dans la boîte de `tab` (écrit `config.toml` : hors des fils du runtime).
fn click_agent(stack: &TestStack, tab: &str, id: &str) -> Result<(), String> {
    let click = json!({ "cmd": "surface_action", "tab": tab, "surface": "host-voice", "name": "select", "context": { "id": id } });
    tokio::task::block_in_place(|| stack.handle(cmd(click)))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_click_in_one_tab_refreshes_the_box_of_the_other() {
    let (stack, rx, _, _) = stack_with_two_agents(json!({ "turns": [] }), json!({ "turns": [] }), &["t-1", "t-2"]).await;
    for tab in ["t-1", "t-2"] {
        tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": tab })))).unwrap();
    }
    click_agent(&stack, "t-1", "bis").unwrap();
    // t-2 ne voit ce choix que par la révision : le clic n'a refait que la boîte de t-1.
    wait_for(&rx, |(ch, v)| *ch == "mcp" && v["tab"] == "t-2" && choice_of(v).as_deref() == Some("bis")).await;
}

/// Un tour où l'agent demande `bis`, puis pose une question qui le garde ouvert.
fn switching_script() -> serde_json::Value {
    json!({ "turns": [{ "mcp": [
        { "tool": "select_agent", "args": { "id": "bis" } },
        { "tool": "ask_choice", "args": { "question": "?", "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] } }
    ] }, { "chunks": ["deux"] }] })
}

/// Confirme la fenêtre de l'onglet `tab`.
async fn confirm_modal(stack: &TestStack, rx: &Relayed, tab: &str) {
    let (_, modal) = wait_for(rx, |(ch, v)| *ch == "mcp" && v["tab"] == tab && v["event"]["surface"]["place"] == "modal").await;
    let id = modal["event"]["surface"]["id"].as_str().unwrap().to_owned();
    let yes = json!({ "cmd": "surface_action", "tab": tab, "surface": id, "name": "confirm", "context": { "confirmed": true } });
    tokio::task::block_in_place(|| stack.handle(cmd(yes))).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_switch_asked_during_a_turn_answers_when_idle() {
    let (stack, rx, log, _) = stack_with_two_agents(switching_script(), json!({ "turns": [] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "passe à bis" }))).unwrap();
    confirm_modal(&stack, &rx, "t-1").await;
    common::until(|| {
        common::logged(&log, "mcp/result").iter().any(|c| c["tool"] == "select_agent" && c["out"].to_string().contains("when_idle"))
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_switch_waits_across_a_queued_prompt_then_reaches_bis() {
    let (stack, rx, log, data) = stack_with_two_agents(switching_script(), json!({ "turns": [{ "chunks": ["bis"] }] }), &["t-1"]).await;
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "passe à bis" }))).unwrap();
    confirm_modal(&stack, &rx, "t-1").await;
    wait_for(&rx, |(_, v)| is_tool_surface(v, "t-1", "ask-")).await;
    // Une consigne en file pendant la question : elle part au même agent, le changement attend.
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "encore" }))).unwrap();
    wait_turn_done(&rx, "t-1", 2).await;
    assert_that!(sessions_in(&log, "session/prompt")).has_length(2);
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "et maintenant" }))).unwrap();
    wait_turn_done(&rx, "t-1", 3).await;
    assert_that!(sessions_in(&data.join("log-bis.jsonl"), "session/prompt")).has_length(1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_turn_in_one_tab_shows_busy_in_the_box_of_the_other() {
    let ask = json!({ "turns": [{ "mcp": [{ "tool": "ask_choice", "args": { "question": "?", "options": [{ "id": "a", "label": "A" }, { "id": "b", "label": "B" }] } }] }] });
    let (stack, rx, _, _) = stack_with_two_agents(ask, json!({ "turns": [] }), &["t-1", "t-2"]).await;
    tokio::task::block_in_place(|| stack.handle(cmd(json!({ "cmd": "show_settings", "tab": "t-2" })))).unwrap();
    stack.handle(cmd(json!({ "cmd": "prompt", "tab": "t-1", "text": "?" }))).unwrap();
    wait_for(&rx, |(ch, v)| *ch == "mcp" && v["tab"] == "t-2" && v.to_string().contains("Changer d'agent interrompra le tour en cours.")).await;
}
```

(`common::until(|| bool)` polls up to 10 s and panics after; add it to agent `tests/common` if absent. In the queued-prompt test, answering the question is what ends turn 1: if the fake agent needs it, answer `a` with a `surface_action` before `wait_turn_done` — read how `tests/stack.rs` answers `ask_choice` and copy it.) Port the existing `select_agent_*` tests by changing the `select` helper to the `show_settings` + `surface_action select` pair (the localized refusal texts stay, since `VoiceError` reuses the keys), and read `choice_of` instead of `picker` events. `tests/availability.rs`: `availability` is now seeded by `voice_refresh` once at start (Step 7, no event sent for the seed), so read each test that counts availability events and keep its count; `tune_with_an_unusable_agent_starts_no_session` keeps its count unchanged.

- [ ] **Step 2: Run them to see them fail.** `cargo nextest run -p oku-slides-mcp -p oku-slides-agent`. Expected: compile errors.

- [ ] **Step 3: mcp `voice.rs`** as in Interfaces; `Cargo.toml` features and dev-dependency; re-exports `ApiTarget, Checked, Origin, VoiceApplied, VoiceError, VoiceErrorKind, VoicePort` and `NoVoice` (under the cfg).

- [ ] **Step 4: agent — `choice.rs` and `command_error.rs`.** Replace `select` by `check`, `switch_to`, `persist` (Interfaces): `switch_to` locks `active`, returns when `*active == id`, else `link.switch(Self::launch_of(preset, &self.platform))` and sets `active`; `persist` locks `saving`, reads `active()` and calls `write_value(…, "agent.selected", …)`. `local_panel` returns `Option<LocalModelView>` (`NeedsDownload { size: AGENT_MODEL_SIZE }`, `Running { percent }`, `Failed { size: AGENT_MODEL_SIZE, failure }` mapping `DownloadErrorKind` → `LocalFailure`); `picker()` maps it back to texts (`Unusable::LocalNeedsDownload`'s message and `failure_text`) until Task 5. Delete `CommandInner::{AgentLocked, AgentNotSaved}` and what only they used. `error.rs`: `Unusable::LocalNeedsDownload` formats with `gigabytes_text`.

- [ ] **Step 5: agent — `agent_switch.rs`, `voice_port.rs`, `stt_watch.rs`.** Module docs: `//! Le changement d'agent demandé par l'agent : requêtes en cours, attente, époque.`, `//! La dictée et l'agent de l'app pour la boîte des réglages et les outils.`, `//! La veille de la dictée : un fil qui signale ce qui change la boîte.`.

  `agent_switch.rs` (`AgentSwitch` owned by `AppVoice`; every method takes the `state` lock for memory work only):
  - `prompt_started(tab) -> (u64, bool)`: `next += 1`, `running.insert(id, tab)`; returns the id and whether `running` went from empty to non-empty (the caller bumps only then).
  - `prompt_ended(id) -> (bool, Option<(String, String, u64)>)`: `running.remove(&id)` (a missing id does nothing and returns `(false, None)`); returns whether `running` became empty (bump) and, if it did and not `shut` and `pending` is set, takes the pending choice with the current `epoch`.
  - `tab_closed(tab)`: drops `pending` when its tab is `tab` (guards release themselves).
  - `shutdown()`: `shut = true`, `pending = None`.
  - `request(tab, id, active: &str) -> Requested`: if `id == active` → clear `pending`, `epoch += 1`, `Unchanged`; else if `running` is empty → `epoch += 1`, `Now(epoch)` (the caller runs `switch_now` at once); else `pending = Some((tab, id))`, `epoch += 1`, `WhenIdle`.
  - `user_select()`: `epoch += 1`, `pending = None`.

  `AppVoice`:
  - `prompt_started(tab)` → `PromptGuard { voice: me, id }` and bumps when busy began; `prompt_ended(id)` (from `Drop`) bumps when idle began and, when a switch was taken, `rt.spawn_blocking(move || voice.switch_now(&tab, &id, epoch))`.
  - `switch_now(tab, id, epoch)`: lock `state`; return if `shut`, `epoch` moved, or a guard runs (then put `(tab, id)` back into `pending` if `epoch` did not move); else `choice.switch_to(&id)` **under the lock** (cheap; no prompt can start between the check and `Link::switch`); unlock; `choice.persist()` (refusal logged at `warn!` with `snafu::Report`; the box keeps showing the active agent); bump.
  - `check(event, origin)` / `apply(event, origin)` share validators:
    - `Dictation(patch)`: `stt` present (`NoDictation`); idle (`Busy`); `Origin::Agent` + `touches_api_target()` → `ApiReservedToUser`; `model` known (`UnknownModel`) and available with the patched language (`Unavailable`); `DictationLanguage::Code(c)` in `DICTATION_LANGUAGES` or equal to the stored language (`BadLanguage`); `silence` in `SILENCE_RANGE` (`BadSilence`); `api_url` → `normalized_url` (`BadUrl`); `api_key_env` → `valid_key_env` (`BadKeyEnv`); `api_model` → `valid_api_model` (`BadApiModel`). When the effective model becomes `api` (it was not) **and the origin is the agent**, validate the stored url and variable (`BadUrl`, `BadKeyEnv`) and fill `Checked.api` with `url_host`, the normalized url, the variable and the effective API model (patched, else stored). When the patched model is `Local` or `Apple`, not installed and differs from the current one, fill `Checked.download`. `unchanged` when no field differs.
    - `apply` re-runs `check`; for `Origin::Agent { confirmed: false }` with `needs_confirmation()` → `Changed`; then under `edit` (I/O lock, never with `state`): read `stt.status().config`, write the fields (`api_url` normalized, `Auto` → `None`), `stt.configure(config)` (`false` → `Busy`); `Unchanged` when nothing differed.
    - `DownloadDictation { model }`: known (`UnknownModel`), not `Api` (`NotDownloadable`), available (`Unavailable`), not installed (`Installed`), idle (`Busy`); `check` fills `Checked.download` with its `ModelView` and the API model; `apply`: `stt.download(model, false)`, `false` → `Busy` when `stt.status().state != Idle`, else `Installed`.
    - `CancelDictationDownload`: `stt` present; `NothingToCancel` unless `cancel_dictation_allowed(state)`; `apply` calls `stt.cancel()` only then.
    - `SelectAgent { id }`: `choice.check(&id)` (`Refusal::UnknownAgent` → `UnknownAgent`, `Refusal::Locked` → `AgentLocked`, exhaustive); `Checked.agent` = its `AgentPresetView` unless the request is `unchanged` (active and nothing pending). `apply`:
      - `Origin::User`: `switch.user_select()` and `choice.switch_to(&id)` under `state` (no-op when active); `choice.persist()` (`Err` → `AgentNotSaved`, after logging the source); `Now`, or `Unchanged` when `id` was already active (it still cleared any pending choice).
      - `Origin::Agent`: under `state`, `switch.request(tab, id, &choice.active())`; `Now(epoch)` → `switch_now(tab, id, epoch)` after unlocking, answer `Now`; `WhenIdle`; `Unchanged`.
    - `DownloadAgentModel`: `check` → `Checked.agent_model_size` from `local_panel()` (`NeedsDownload`/`Failed`), else `NothingToDownload`; `apply`: `choice.start_download(…)` with a callback that upgrades `me` and bumps; `false` → `NothingToDownload`.
    - `CancelAgentModelDownload`: `NothingToCancel` unless `local_panel()` is `Running`; `apply`: `choice.cancel_download()`.
    - Every accepted change bumps; a refused one does not.
  - `view()`: dictation from `stt.status()` (`ModelView { id: m.id.id().into(), kind, … }`; `key_found` = the `Api` model's `installed`, `false` when absent); then lock `switch.state`, copy `busy = !running.is_empty()` and `pending`, unlock; then `choice.agents_view()` (disk, outside any lock) with those two filled.
  - `new`: `Arc::new_cyclic` builds the struct; after it returns, when `stt` is `Some`, `stt_watch::spawn(stt.clone(), Arc::downgrade(&voice))`.

  `stt_watch.rs`:

```rust
/// Ce qui change la boîte dans l'état de la dictée.
#[derive(Debug, PartialEq)]
pub(crate) struct Fingerprint {
    state: State,
    config: Config,
    models: Vec<(Model, bool, bool)>,
    percent: Option<u64>,
    detected: Option<String>,
}

impl Fingerprint {
    /// L'empreinte d'un état de la dictée.
    pub(crate) fn of(s: &Status) -> Self {
        Self {
            state: s.state,
            config: s.config.clone(),
            models: s.models.iter().map(|m| (m.id, m.installed, m.available)).collect(),
            percent: s.download.as_ref().map(|d| d.received.saturating_mul(100) / d.total.max(1)),
            detected: s.detected.clone(),
        }
    }
}

/// Lance la veille : toutes les 400 ms, un changement d'empreinte fait avancer la révision.
///
/// Le fil s'arrête quand la boîte disparaît. Une panique est journalisée, pas rattrapée.
pub(crate) fn spawn(stt: Stt, voice: Weak<AppVoice>) {
    let started = std::thread::Builder::new().name("oku-voice-watch".into()).spawn(move || {
        let _exit = ExitLog;
        let mut last = Fingerprint::of(&stt.status());
        loop {
            std::thread::sleep(WATCH);
            let Some(voice) = voice.upgrade() else { return };
            let now = Fingerprint::of(&stt.status());
            if now != last {
                last = now;
                voice.bump();
            }
        }
    });
    if let Err(e) = started {
        tracing::warn!(error = %e, "veille de la dictée non lancée : la boîte suit seulement les réglages");
    }
}

/// Journalise la fin du fil de veille quand elle vient d'une panique.
struct ExitLog;

impl Drop for ExitLog {
    fn drop(&mut self) {
        if std::thread::panicking() {
            tracing::error!("veille de la dictée arrêtée par une panique");
        }
    }
}
```

  (`WATCH = Duration::from_millis(400)`; `Config` and `State` derive `PartialEq` in stt — check; `bump` is `pub(crate)` on `AppVoice`.)

- [ ] **Step 6: mcp, the hub box.**
  - `hub.rs`: field `voice: Arc<dyn VoicePort>` (`/// La dictée et l'agent de l'app : la boîte des réglages.`), `new(session, localizer, voice)`, `Shown::Settings(SettingsShown)` with `#[derive(Debug, Clone)] struct SettingsShown { last: VoicePanel }`, `mod settings;`; `prune`'s match: `Shown::Settings(_)` in the `return false` arm.
  - `actions.rs` `act`, **before** the `shows.remove`:

```rust
        // La boîte des réglages reste montrée pendant le clic : un rafraîchissement n'est jamais perdu.
        if action.surface == HOST_VOICE && action.name != "dismiss" {
            return self.apply_settings(&action.context, &action.name);
        }
```

  - `hub/error.rs`: `HubInner::Voice { #[snafu(source(from(VoiceError, Box::new)))] source: Box<VoiceError>, backtrace: Backtrace }`, kind `HubErrorKind::Voice` (doc), localized by `source.localize(l)`.
  - `hub/settings.rs`:

```rust
//! Boîte des réglages de la dictée et de l'agent : montrée, ses clics appliqués, rafraîchie.
//!
//! Un seul chemin d'envoi, sous le verrou d'état : données seules quand la disposition ne
//! change pas, la boîte entière sinon et après un refus.

use oku_slides_session::{HOST_VOICE, VoicePanel, voice_event};
use serde_json::{Map, Value};
use snafu::IntoError;

use super::error::{HubError, IncompleteSnafu, VoiceSnafu};
use super::{McpEvent, SettingsShown, Shown, SurfaceHub};
use crate::Origin;

impl SurfaceHub {
    /// Montre la boîte des réglages, seule. Peut bloquer : le port lit le disque.
    pub fn show_settings(&self) {
        let panel = VoicePanel::build(&self.voice.view(), &self.localizer.current());
        let surface = panel.surface();
        self.show_alone(surface, Shown::Settings(SettingsShown { last: panel }), |s| matches!(s, Shown::Settings(_)));
    }

    /// Applique un clic de la boîte montrée, puis envoie ce qui a changé.
    pub(super) fn apply_settings(&self, context: &Map<String, Value>, name: &str) -> Result<(), HubError> {
        if !matches!(self.state().shows.get(HOST_VOICE), Some(Shown::Settings(_))) {
            return Err(HubError::unknown_surface(HOST_VOICE.to_owned()));
        }
        let result = match voice_event(name, context) {
            Some(event) => self.voice.apply(event, Origin::User).map(|_| ()).map_err(|e| VoiceSnafu.into_error(e)),
            None => {
                tracing::warn!(action = name, "action de la boîte des réglages mal formée");
                Err(IncompleteSnafu { key: name }.build())
            }
        };
        let panel = VoicePanel::build(&self.voice.view(), &self.localizer.current());
        self.send_settings(&panel, result.is_err());
        result.map_err(Into::into)
    }

    /// Refait la boîte montrée avec `panel`, si elle a changé.
    pub fn refresh_settings_with(&self, panel: &VoicePanel) {
        self.send_settings(panel, false);
    }

    /// Envoie `panel` à la place de la boîte montrée ; `whole` : toute la boîte (après un refus).
    ///
    /// Relit `state` sous son verrou : une boîte fermée entre-temps n'est jamais rouverte.
    fn send_settings(&self, panel: &VoicePanel, whole: bool) {
        let mut state = self.state();
        let Some(Shown::Settings(shown)) = state.shows.get(HOST_VOICE) else { return };
        if !whole && shown.last == *panel {
            return;
        }
        let event = if !whole && shown.last.same_layout(panel) {
            McpEvent::SurfacePatch { id: HOST_VOICE.to_owned(), messages: panel.data_messages() }
        } else {
            McpEvent::Surface { surface: panel.surface() }
        };
        state.shows.insert(HOST_VOICE.to_owned(), Shown::Settings(SettingsShown { last: panel.clone() }));
        self.record_and_send(&mut state, event);
    }
}
```

  (Read the real `HubError::unknown_surface` and the inner-to-public conversion; `IncompleteSnafu { key }` and the hub `VoiceSnafu` selector names; `HubInner → HubError` is the existing `From`. A malformed click is refused and the whole box resent, so the UI drops its stale draft.)

- [ ] **Step 7: agent, wiring.**
  - `table.rs` (`//! Les onglets démarrés et fermés de la pile, sous un seul verrou.`): `Table` and `refuses` moved from `stack.rs`, plus `links()`. `Stack.table: Arc<Mutex<Table>>`.
  - `Stack`: field `voice: Arc<AppVoice>`; `start` builds `choice`, then `voice = AppVoice::new(stt.clone(), choice.clone(), rt.clone())`, then `voice_refresh::spawn(&rt, Arc::downgrade(&voice), Arc::downgrade(&table), out_tx.clone())`; `TabEntry::start(…, TabPorts { settings, voice: voice.clone() }, …)`. `Stack::tab_closed` calls `self.voice.tab_closed(tab)`; `Stack::shutdown` calls `self.voice.shutdown()` first. Fix the `start` doc (`stt` is for the box and the `settings` tool).
  - `AgentCommand::ShowSettings { tab }` (`/// L'interface ouvre la boîte des réglages de la dictée et de l'agent.`), `| Self::ShowSettings { tab }` in `tab()`, and the arm:

```rust
            // Démarre l'onglet (outils, gardien ; pas l'agent ACP) : la boîte va au hub de l'onglet.
            AgentCommand::ShowSettings { tab } => {
                let Some(t) = self.open(&tab) else { return Ok(()) };
                t.hub.show_settings();
            }
```

  - The old commands go through the port (one writer): `SelectAgent { tab, id }` → `self.voice.apply(VoiceEvent::SelectAgent { id }, Origin::User)` (error → `e.localize(&l)` returned), then `emit_picker`; `DownloadModel { tab }` → `self.voice.apply(VoiceEvent::DownloadAgentModel, Origin::User)` — a refusal is logged at `debug!` as today ("rien à télécharger") and the picker is re-emitted; `CancelDownload` → `apply(CancelAgentModelDownload, User)`, refusal logged at `debug!`, picker re-emitted. The per-percent picker re-emission keeps its current path until Task 5.
  - `tab_agent.rs`: `ActorParts`, `start(parts, voice)`, `spawn` (unchanged signature), `spawn_counted`; `Ctx.voice: Option<Arc<AppVoice>>`. In `TabAgent::run`, `let mut busy: Option<PromptGuard> = None;` before the loop and:

```rust
                Command::Prompt(instruction) => {
                    // Une requête ACP tourne : un changement d'agent attend. Gardé pour une consigne en file.
                    let guard = busy.take().or_else(|| self.ctx.voice.as_ref().map(|v| v.prompt_started(self.ctx.tab.tab())));
                    next = self.prompt(&mut rx, instruction).await;
                    if matches!(next, Some(Command::Prompt(_))) {
                        busy = guard;
                    }
                }
```

    (the guard drops at the end of the arm otherwise; `next` is only ever `Some(Command::Prompt(_))`, so `busy` is `None` whenever the loop waits on `rx`).
  - `tab_entry.rs`: `TabPorts`, `start(session, platform, ports, link, rt, out)`; `SurfaceHub::new(…, ports.voice.clone())`; the actor via `AgentHandle::spawn_counted(parts, ports.voice)`.
  - `voice_refresh.rs` (`//! Suit la révision de la dictée et de l'agent pour tous les onglets.`):

```rust
use std::sync::{Mutex, PoisonError, Weak};

use oku_slides_mcp::VoicePort;
use oku_slides_session::{DictationView, VoicePanel};
use tokio::runtime::Handle;
use tokio::sync::mpsc;

use crate::events::AgentEvent;
use crate::tab_entry::{Out, emit};
use crate::table::Table;
use crate::voice_port::AppVoice;

/// Ce que la tâche a déjà dit aux onglets.
#[derive(Clone, Default)]
struct Seen {
    dictation: Option<DictationView>,
    availability: Option<Option<String>>,
}

/// Ce que voit la boîte au départ : rien n'est envoyé pour lui, seul un changement part.
fn seed(voice: &Weak<AppVoice>) -> Seen {
    voice.upgrade().map(|v| Seen { dictation: v.view().dictation, availability: Some(v.availability()) }).unwrap_or_default()
}

/// Suit `voice` : une vue par changement, la boîte de chaque onglet, le micro relu.
///
/// La tâche async ne garde rien de fort ; tout le travail bloquant passe par le pool.
pub(crate) fn spawn(rt: &Handle, voice: Weak<AppVoice>, table: Weak<Mutex<Table>>, out: mpsc::UnboundedSender<Out>) {
    let Some(mut revision) = voice.upgrade().map(|v| v.revision()) else { return };
    rt.spawn(async move {
        let start = voice.clone();
        let mut seen = match tokio::task::spawn_blocking(move || seed(&start)).await {
            Ok(seen) => seen,
            Err(e) => {
                tracing::warn!(error = %e, "état de départ de la boîte non lu : le premier changement part en entier");
                Seen::default()
            }
        };
        while revision.changed().await.is_ok() {
            let (voice, table, out, previous) = (voice.clone(), table.clone(), out.clone(), seen.clone());
            let done = tokio::task::spawn_blocking(move || {
                let (Some(voice), Some(table)) = (voice.upgrade(), table.upgrade()) else { return previous };
                let view = voice.view();
                let panel = VoicePanel::build(&view, &voice.localizer());
                let availability = voice.availability();
                let links = table.lock().unwrap_or_else(PoisonError::into_inner).links();
                for (tab, link) in links {
                    link.hub.refresh_settings_with(&panel);
                    if view.dictation != previous.dictation {
                        emit(&out, "agent", &tab, &AgentEvent::Voice);
                    }
                    if Some(&availability) != previous.availability.as_ref() {
                        emit(&out, "agent", &tab, &AgentEvent::Availability { unusable: availability.clone() });
                    }
                }
                Seen { dictation: view.dictation, availability: Some(availability) }
            })
            .await;
            match done {
                Ok(next) => seen = next,
                Err(e) => tracing::warn!(error = %e, "boîte des réglages non rafraîchie"),
            }
        }
    });
}
```

  - `events.rs`: `AgentEvent::Voice` (+ serde test `{"kind":"voice"}`); `turns.rs`: `| AgentEvent::Voice` next to `AgentEvent::Availability { .. }` at both exhaustive matches.
  - Agent `tests/common/mod.rs:140,147`: `SurfaceHub::new(…, Arc::new(oku_slides_mcp::NoVoice))`; `AgentHandle::spawn` keeps its signature (no voice there). Local-agent `tests/common/mod.rs:104,111`: the same.

- [ ] **Step 8: Docs and rules (same commit).**
  - mcp `ARCHITECTURE.md`: merge the `settings` line into one "ports" line ("`SettingsPort` in `Tools`; `VoicePort` (view, check, apply) in the hub, which builds the box"); append `hub/settings` to the hub line (a third box, one send path, refreshed by the stack, never by `prune`); merge the two extension points into one line ("New settings host: implement `SettingsPort` and `VoicePort`").
  - agent `ARCHITECTURE.md`: rewrite the `stack` line ("the table of tabs" → `table`), the `choice` line (`check`, `switch_to`, `persist`; download progress bumps the voice revision), and the `settings_port, picker` line to add `voice_port` (`AppVoice`), `agent_switch` (`PromptGuard`, deferred switch), `stt_watch`, `voice_refresh`.
  - mcp `AGENTS.md` Invariants: `- Rebuild the settings box only from the VoicePort view through its one send path, never from prune. (spec:2026-10-23-reglages-dictee-a2ui-design)`.
  - agent `AGENTS.md`: reword "Refuse agent and agent.* keys in agent settings writes; only the user chooses the agent." into `- Refuse agent and agent.* keys in settings writes; an agent's agent switch goes through VoicePort, applied when no prompt runs. (spec:2026-10-23-reglages-dictee-a2ui-design, t-786)`; add `- Hold a PromptGuard from AppVoice while a tab prompts the ACP agent, across a queued prompt. (spec:2026-10-23-reglages-dictee-a2ui-design)` and `- Switch the agent only under the AppVoice lock after re-checking; then persist the active agent, never the requested one. (spec:2026-10-23-reglages-dictee-a2ui-design)`.

- [ ] **Step 9: Run and check.** `cargo nextest run -p oku-slides-mcp -p oku-slides-agent -p oku-slides-local-agent -p oku-slides-i18n && just check && wc -l crates/*/ARCHITECTURE.md`. Expected: PASS, every ARCHITECTURE.md ≤ 50.

- [ ] **Step 10: Commit** `feat(agent) : port de la dictée et de l'agent, boîte host-voice dans chaque onglet, changement d'agent quand aucune requête ne tourne (t-1144, t-1148)`.

---

### Task 4: The seven MCP tools; `set_setting` without dictation (t-1149)

**Files:**
- Create: mcp `src/voice_tools.rs`, `tests/regressions/issue_1144_set_setting_dictation.rs`, `tests/regressions/issue_1144_agent_api_target.rs` (declared in `tests/regressions/main.rs`)
- Modify: mcp `src/{lib.rs,hub.rs (pub(crate) fn voice),tools.rs,handler.rs,tool_error.rs,tool_error_tests.rs,settings.rs,guarded_tools.rs}`, `tests/{voice.rs,guarded.rs,common/mod.rs}`; agent `src/{settings_port.rs,settings_port_tests.rs,instructions/agent.md,instructions_tests.rs}`; local-agent `src/{families.rs,families_tests.rs}`; `agent.ftl` × 20; mcp `ARCHITECTURE.md`, `AGENTS.md`
- Read first: `guarded_tools.rs` (`set_setting`, `denied`), `surface_tools.rs::show_packs`, `tools.rs` (`nest_schema`), local-agent `compact.rs` (only a description's first sentence reaches the local agent)

**Interfaces:**
- Consumes: `VoicePort::{view, check, apply}`, `Checked`, `ApiTarget`, `VoiceApplied`, `Origin::Agent { tab, confirmed }`, `SurfaceHub::{show_settings, confirm(String, bool) -> bool}`, session `{model_label, size_text, DictationLanguage, DictationPatch, ModelKind, ModelView, VoiceEvent, HOST_VOICE, agent::bounds::clip}`.
- Produces: tools `show_dictation`, `set_dictation`, `download_dictation_model`, `cancel_dictation_download`, `select_agent`, `download_agent_model`, `cancel_agent_model_download`; results `{"shown": "host-voice"}` or `{"applied": "now" | "when_idle" | "unchanged"}`; `SurfaceHub::voice()` (`pub(crate)`, first used here); `Tools::denied` → `pub(crate)`; `ToolError::EmptyDictationChange { backtrace }` (`agent-error-empty-dictation`), `ToolError::Voice { source: Box<VoiceError>, backtrace }` (localized through its source); `settings::Applied` keeps only `NextLaunch`.
- Keys (`agent.ftl`, en / fr; 18 translations):

| Key | en | fr |
|---|---|---|
| `agent-confirm-dictation-api` | Send what you dictate to { $host } ({ $url }), model { $model }, with the secret of { $var }? | Envoyer ce que vous dictez à { $host } ({ $url }), modèle { $model }, avec le secret de { $var } ? |
| `agent-confirm-dictation-download` | Use { $name } and download it now ({ $size })? | Utiliser { $name } et le télécharger maintenant ({ $size }) ? |
| `agent-confirm-dictation-language` | Use Apple dictation and download its language now? | Utiliser la dictée d'Apple et télécharger sa langue maintenant ? |
| `agent-confirm-agent` | Switch the app agent to { $name } ({ $command }) once no reply is running? | Passer l'agent de l'app à { $name } ({ $command }) dès qu'aucune réponse ne tourne ? |
| `agent-confirm-agent-builtin` | Switch the app agent to { $name } once no reply is running? | Passer l'agent de l'app à { $name } dès qu'aucune réponse ne tourne ? |
| `agent-confirm-agent-model` | Download the local agent model ({ $size })? | Télécharger le modèle de l'agent local ({ $size }) ? |
| `agent-error-empty-dictation` | Nothing to change: give at least one dictation setting. | Rien à changer : donnez au moins un réglage de la dictée. |

- [ ] **Step 1: Write the failing tests** (append to mcp `tests/voice.rs`; imports `Checked`, `ApiTarget`, `VoiceApplied`, `DictationLanguage`, `AgentPresetView`):

```rust
fn api_target() -> ApiTarget {
    ApiTarget { host: "api.openai.com".into(), url: "https://api.openai.com/v1".into(), key_env: "OPENAI_API_KEY".into(), model: "gpt-4o-transcribe".into() }
}

fn large() -> oku_slides_session::ModelView {
    oku_slides_session::voice_fixture::voice_view_fixture().dictation.unwrap().models.into_iter().find(|m| m.id == "large").unwrap()
}

fn agent_tab(confirmed: bool) -> Origin {
    Origin::Agent { tab: "t-1".into(), confirmed }
}

#[tokio::test(flavor = "multi_thread")]
async fn show_dictation_tool_returns_the_box_id_at_once() {
    let h = start().await;
    let r = call_within(&server(&h).await, "show_dictation", json!({})).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "shown": "host-voice" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_plain_fields_apply_at_once() {
    let h = start().await;
    let r = call_within(&server(&h).await, "set_dictation", json!({"language": "auto", "silence": 2.0})).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "applied": "now" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_plain_fields_apply_unconfirmed_as_this_tab_agent() {
    let h = start().await;
    call_within(&server(&h).await, "set_dictation", json!({"language": "auto", "silence": 2.0})).await;
    let patch = DictationPatch { language: Some(DictationLanguage::Auto), silence: Some(2.0), ..Default::default() };
    assert_that!(*lock(&h.voice.applied)).is_equal_to(vec![(VoiceEvent::Dictation(patch), agent_tab(false))]);
}

#[rstest]
#[case("api.openai.com")]
#[case("https://api.openai.com/v1")]
#[case("OPENAI_API_KEY")]
#[case("gpt-4o-transcribe")]
#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_api_confirmation_names_the_target(#[case] part: &str) {
    let h = start().await;
    lock(&h.voice.checked).api = Some(api_target());
    let (text, _, _) = guarded_message(&h, "set_dictation", json!({"model": "api"}), false).await;
    assert_that!(text).contains(part);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_api_confirmation_is_a_danger() {
    let h = start().await;
    lock(&h.voice.checked).api = Some(api_target());
    let (_, danger, _) = guarded_message(&h, "set_dictation", json!({"model": "api"}), false).await;
    assert_that!(danger).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_api_declined_applies_nothing() {
    let h = start().await;
    lock(&h.voice.checked).api = Some(api_target());
    guarded_message(&h, "set_dictation", json!({"model": "api", "silence": 3.0}), false).await;
    assert_that!(*lock(&h.voice.applied)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_api_accepted_applies_confirmed() {
    let h = start().await;
    lock(&h.voice.checked).api = Some(api_target());
    guarded_message(&h, "set_dictation", json!({"model": "api"}), true).await;
    let origins: Vec<Origin> = lock(&h.voice.applied).iter().map(|(_, o)| o.clone()).collect();
    assert_that!(origins).is_equal_to(vec![agent_tab(true)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_an_uninstalled_model_asks_with_its_size() {
    let h = start().await;
    lock(&h.voice.checked).download = Some((large(), "gpt-4o-transcribe".into()));
    let (text, _, _) = guarded_message(&h, "set_dictation", json!({"model": "large"}), false).await;
    assert_that!(text).is_equal_to("Utiliser Large v3 et le télécharger maintenant (3,1 Go) ?".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_to_an_uninstalled_model_accepted_also_downloads_it() {
    let h = start().await;
    lock(&h.voice.checked).download = Some((large(), "gpt-4o-transcribe".into()));
    guarded_message(&h, "set_dictation", json!({"model": "large"}), true).await;
    let events: Vec<VoiceEvent> = lock(&h.voice.applied).iter().map(|(e, _)| e.clone()).collect();
    assert_that!(events).contains(VoiceEvent::DownloadDictation { model: "large".into() });
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_unchanged_asks_nothing() {
    let h = start().await;
    lock(&h.voice.checked).unchanged = true;
    let r = call_within(&server(&h).await, "set_dictation", json!({"model": "turbo"})).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "applied": "unchanged" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_refused_by_the_port_says_why_in_french() {
    let h = start().await;
    *lock(&h.voice.refuse) = Some(VoiceErrorKind::Busy);
    let r = call_within(&server(&h).await, "set_dictation", json!({"model": "large"})).await;
    assert_that!(text_of(&r)).is_equal_to("La dictée est en cours : réessayez une fois au repos".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_empty_is_refused_in_french() {
    let h = start().await;
    let r = call_within(&server(&h).await, "set_dictation", json!({})).await;
    assert_that!(text_of(&r)).is_equal_to("Rien à changer : donnez au moins un réglage de la dictée.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_unknown_field_is_refused_without_echoing_it() {
    let h = start().await;
    let r = call_within(&server(&h).await, "set_dictation", json!({"key": "sk-secret-1144"})).await;
    assert_that!(text_of(&r)).does_not_contain("sk-secret-1144");
}

#[rstest]
#[case("api_url")]
#[case("key_env")]
#[case("\"url\"")]
#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_schema_has_no_api_target_field(#[case] key: &str) {
    let h = start().await;
    let schema = tool_schema(&server(&h).await, "set_dictation").await.to_string();
    assert_that!(schema).does_not_contain(key);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_dictation_description_first_sentence_reserves_the_api_target() {
    let h = start().await;
    let description = tool_description(&server(&h).await, "set_dictation").await;
    let first = description.split(". ").next().unwrap_or_default().to_owned();
    assert_that!(first).contains("only the user sets");
}

#[tokio::test(flavor = "multi_thread")]
async fn download_dictation_model_confirmation_names_model_and_size() {
    let h = start().await;
    lock(&h.voice.checked).download = Some((large(), "gpt-4o-transcribe".into()));
    let (text, _, _) = guarded_message(&h, "download_dictation_model", json!({"model": "large"}), false).await;
    assert_that!(text).is_equal_to("Utiliser Large v3 et le télécharger maintenant (3,1 Go) ?".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn download_dictation_model_refused_opens_no_dialog() {
    let h = start().await;
    *lock(&h.voice.refuse) = Some(VoiceErrorKind::UnknownModel);
    let r = call_within(&server(&h).await, "download_dictation_model", json!({"model": "nope"})).await;
    assert_that!(text_of(&r)).is_equal_to("Modèle de dictée inconnu".to_owned());
}

#[rstest]
#[case("Bis")]
#[case("fake --bis")]
#[tokio::test(flavor = "multi_thread")]
async fn select_agent_confirmation_names_the_agent_and_its_command(#[case] part: &str) {
    let h = start().await;
    lock(&h.voice.checked).agent = Some(AgentPresetView { id: "bis".into(), name: "Bis".into(), hint: Some("fake --bis".into()) });
    let (text, _, _) = guarded_message(&h, "select_agent", json!({"id": "bis"}), false).await;
    assert_that!(text).contains(part);
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_confirmed_reports_the_port_answer() {
    let h = start().await;
    lock(&h.voice.checked).agent = Some(AgentPresetView { id: "bis".into(), name: "Bis".into(), hint: None });
    *lock(&h.voice.answer) = Some(VoiceApplied::WhenIdle);
    let (_, _, r) = guarded_message(&h, "select_agent", json!({"id": "bis"}), true).await;
    assert_that!(json_of(&r)).is_equal_to(json!({ "applied": "when_idle" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_declined_applies_nothing() {
    let h = start().await;
    lock(&h.voice.checked).agent = Some(AgentPresetView { id: "bis".into(), name: "Bis".into(), hint: None });
    guarded_message(&h, "select_agent", json!({"id": "bis"}), false).await;
    assert_that!(*lock(&h.voice.applied)).is_empty();
}

#[tokio::test(flavor = "multi_thread")]
async fn select_agent_unchanged_goes_through_the_port_without_asking() {
    let h = start().await;
    lock(&h.voice.checked).unchanged = true;
    call_within(&server(&h).await, "select_agent", json!({"id": "faux"})).await;
    assert_that!(*lock(&h.voice.applied)).has_length(1);
}

#[rstest]
#[case(true, 1)]
#[case(false, 0)]
#[tokio::test(flavor = "multi_thread")]
async fn download_agent_model_is_confirmed(#[case] yes: bool, #[case] applied: usize) {
    let h = start().await;
    lock(&h.voice.checked).agent_model_size = Some(2_500_000_000);
    guarded_message(&h, "download_agent_model", json!({}), yes).await;
    assert_that!(*lock(&h.voice.applied)).has_length(applied);
}

#[rstest]
#[case("cancel_dictation_download", VoiceEvent::CancelDictationDownload)]
#[case("cancel_agent_model_download", VoiceEvent::CancelAgentModelDownload)]
#[tokio::test(flavor = "multi_thread")]
async fn cancel_tools_apply_at_once(#[case] tool: &str, #[case] event: VoiceEvent) {
    let h = start().await;
    call_within(&server(&h).await, tool, json!({})).await;
    assert_that!(*lock(&h.voice.applied)).is_equal_to(vec![(event, agent_tab(false))]);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_with_nothing_running_says_so_in_french() {
    let h = start().await;
    *lock(&h.voice.refuse) = Some(VoiceErrorKind::NothingToCancel);
    let r = call_within(&server(&h).await, "cancel_dictation_download", json!({})).await;
    assert_that!(text_of(&r)).is_equal_to("Aucun téléchargement n'est en cours".to_owned());
}
```

(`tool_schema(c, name) -> Value` and `tool_description(c, name) -> String` are new common helpers reading `tools/list` through the client: find how `tools_tests.rs` or `server.rs` lists tools and reuse it. The harness tab id is `t-1` — check `start_with` and use its real tab id in `agent_tab`.)

Regressions:

```rust
//! t-1144 : `set_setting` ne touche plus la dictée.

use crate::common::*;
use serde_json::json;
use speculoos::prelude::*;

#[tokio::test(flavor = "multi_thread")]
async fn issue_1144_set_setting_dictation_model_is_refused_in_french() {
    let h = start().await;
    let r = call_within(&server(&h).await, "set_setting", json!({ "key": "dictation.model", "value": "base" })).await;
    assert_that!(text_of(&r)).is_equal_to("Le réglage dictation.model ne peut pas être changé par l'agent.".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_1144_set_setting_dictation_model_writes_nothing() {
    let h = start().await;
    call_within(&server(&h).await, "set_setting", json!({ "key": "dictation.model", "value": "base" })).await;
    assert_that!(*lock(&h.settings.values)).does_not_contain_key("dictation.model".to_owned());
}
```

```rust
//! t-1144 : l'agent ne choisit jamais l'adresse ni la variable de l'API de dictée.

use rstest::rstest;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::*;

#[rstest]
#[case(json!({"api": {"url": "https://evil.example/v1"}}))]
#[case(json!({"api_url": "https://evil.example/v1"}))]
#[case(json!({"url": "https://evil.example/v1"}))]
#[case(json!({"api_key_env": "GITHUB_TOKEN"}))]
#[case(json!({"key_env": "GITHUB_TOKEN"}))]
#[tokio::test(flavor = "multi_thread")]
async fn issue_1144_agent_api_target_is_refused_in_french(#[case] args: Value) {
    let h = start().await;
    let r = call_within(&server(&h).await, "set_dictation", args).await;
    assert_that!(text_of(&r)).is_equal_to("Seul l'utilisateur règle l'adresse et la variable de la clé de l'API : ouvrez la boîte avec show_dictation et demandez-le-lui".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn issue_1144_agent_api_target_applies_nothing() {
    let h = start().await;
    call_within(&server(&h).await, "set_dictation", json!({"model": "api", "key_env": "GITHUB_TOKEN"})).await;
    assert_that!(*lock(&h.voice.applied)).is_empty();
}
```

Also: `settings_port_tests.rs` `write_dictation_key_is_refused` and `write_dictation_key_writes_nothing`; `instructions_tests.rs` cases `"show_dictation"`, `"set_dictation"`, `"select_agent"`, `"download_dictation_model"`; `families_tests.rs` one rstest case per new tool → `Configure`; `tool_error_tests.rs` one case per new variant (key + French literal); `guarded.rs`'s `set_setting_confirmed_writes_through_port` uses `history.limit` and expects `{"applied": "next_launch"}`.

- [ ] **Step 2: Run them to see them fail.** `cargo nextest run -p oku-slides-mcp --test voice --test regressions`. Expected: FAIL (unknown tools; `set_setting` still writes `dictation.model`, caught by `call_within`).

- [ ] **Step 3: Write `voice_tools.rs`.**

```rust
//! Outils de la dictée et de l'agent : la boîte, les réglages, les téléchargements, l'agent.
//!
//! Tout passe par le port du hub, vérifié avant toute question. L'adresse et la variable
//! de l'API ne sont pas des arguments : seul l'utilisateur les règle (spec t-1144).

use oku_slides_i18n::Localizer;
use oku_slides_session::agent::bounds::clip;
use oku_slides_session::{DictationLanguage, DictationPatch, HOST_VOICE, ModelKind, ModelView, VoiceEvent, model_label, size_text};
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use snafu::{IntoError, ResultExt};

use crate::tool_error::{EmptyDictationChangeSnafu, TaskSnafu, ToolError, VoiceSnafu};
use crate::tools::Tools;
use crate::{ApiTarget, Checked, Origin, VoiceApplied, VoiceError, VoiceErrorKind, VoicePort};

/// Clés d'arguments qui viseraient l'adresse ou la variable de l'API.
const API_TARGET_KEYS: [&str; 5] = ["api", "api_url", "url", "api_key_env", "key_env"];

/// Réglages de la dictée à changer ; absent : inchangé.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetDictationArgs {
    /// apple | turbo | large | base | api
    pub model: Option<String>,
    /// Code de langue (fr, en…) ou "auto".
    pub language: Option<String>,
    /// Arrêter l'écoute seule après un silence.
    pub auto_stop: Option<bool>,
    /// Secondes de silence avant l'arrêt seul, de 0,5 à 10.
    pub silence: Option<f32>,
    /// Modèle du service de transcription.
    pub api_model: Option<String>,
}

/// Un modèle de dictée.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ModelArg {
    /// apple | turbo | large | base
    pub model: String,
}

/// Un agent de la configuration.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentArg {
    /// Clé de [agent.presets] (outil settings).
    pub id: String,
}

/// Un refus de sorte `kind`, en erreur d'outil.
fn refusal(kind: VoiceErrorKind) -> ToolError {
    VoiceSnafu.into_error(Box::new(VoiceError::new(kind)))
}

impl Tools {
    /// Ouvre la boîte hors tour ; elle reste jusqu'à ce que l'utilisateur la ferme.
    pub(crate) async fn show_dictation(&self) -> CallToolResult {
        let hub = self.hub.clone();
        let shown = tokio::task::spawn_blocking(move || hub.show_settings()).await.context(TaskSnafu);
        self.reply(shown, |()| self.json_result(&json!({ "shown": HOST_VOICE })))
    }

    /// Change des réglages de la dictée ; passer à l'API ou à un modèle absent est confirmé.
    pub(crate) async fn set_dictation(&self, args: Value) -> CallToolResult {
        // Avant l'analyse : l'agent apprend que ces réglages sont à l'utilisateur, pas « champ inconnu ».
        if args.as_object().is_some_and(|o| API_TARGET_KEYS.iter().any(|k| o.contains_key(*k))) {
            return self.fail_with(refusal(VoiceErrorKind::ApiReservedToUser));
        }
        let a: SetDictationArgs = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let patch = DictationPatch {
            model: a.model,
            language: a.language.map(|l| if l == "auto" { DictationLanguage::Auto } else { DictationLanguage::Code(l) }),
            auto_stop: a.auto_stop,
            silence: a.silence,
            api_url: None,
            api_model: a.api_model,
            api_key_env: None,
        };
        if patch.is_empty() {
            return self.fail_with(EmptyDictationChangeSnafu.build());
        }
        let event = VoiceEvent::Dictation(patch);
        let checked = match self.voice_check(event.clone()).await {
            Ok(c) => c,
            Err(e) => return self.fail_with(e),
        };
        if checked.unchanged {
            return self.json_result(&json!({ "applied": VoiceApplied::Unchanged }));
        }
        let l = self.localizer.current();
        let question = match (&checked.api, &checked.download) {
            (Some(t), _) => Some((api_question(t, &l), true)),
            (None, Some((m, api_model))) => Some((download_question(m, api_model, &l), false)),
            (None, None) => None,
        };
        let confirmed = question.is_some();
        if let Some((message, danger)) = question
            && !self.hub.confirm(message, danger).await
        {
            return self.denied();
        }
        let applied = self.voice_apply(event, confirmed).await;
        match (applied, checked.download) {
            (Ok(_), Some((m, _))) => self.apply_voice(VoiceEvent::DownloadDictation { model: m.id }, true).await,
            (applied, _) => self.reply(applied, |a| self.json_result(&json!({ "applied": a }))),
        }
    }

    /// Télécharge un modèle ou une langue de dictée, après confirmation.
    pub(crate) async fn download_dictation_model(&self, args: Value) -> CallToolResult {
        let a: ModelArg = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let event = VoiceEvent::DownloadDictation { model: a.model };
        let checked = match self.voice_check(event.clone()).await {
            Ok(c) => c,
            Err(e) => return self.fail_with(e),
        };
        let Some((m, api_model)) = &checked.download else {
            // `check` remplit `download` pour tout modèle qu'il accepte de télécharger.
            tracing::error!("modèle de dictée vérifié sans téléchargement");
            return self.fail_with(refusal(VoiceErrorKind::Installed));
        };
        if !self.hub.confirm(download_question(m, api_model, &self.localizer.current()), false).await {
            return self.denied();
        }
        self.apply_voice(event, true).await
    }

    /// Arrête le téléchargement d'un modèle de dictée.
    pub(crate) async fn cancel_dictation_download(&self) -> CallToolResult {
        self.checked_then_apply(VoiceEvent::CancelDictationDownload).await
    }

    /// Change l'agent de l'app après confirmation, dès qu'aucune réponse ne tourne.
    pub(crate) async fn select_agent(&self, args: Value) -> CallToolResult {
        let a: AgentArg = match self.parse(args) {
            Ok(a) => a,
            Err(r) => return r,
        };
        let event = VoiceEvent::SelectAgent { id: a.id };
        let checked = match self.voice_check(event.clone()).await {
            Ok(c) => c,
            Err(e) => return self.fail_with(e),
        };
        if checked.unchanged {
            return self.apply_voice(event, false).await;
        }
        let Some(preset) = &checked.agent else {
            tracing::error!("agent vérifié sans sa fiche");
            return self.fail_with(refusal(VoiceErrorKind::UnknownAgent));
        };
        let l = self.localizer.current();
        let name = clip(&preset.name, 80);
        let message = match &preset.hint {
            Some(command) => l.tr_with("agent-confirm-agent", &[("name", name.into()), ("command", clip(command, 120).into())]),
            None => l.tr_with("agent-confirm-agent-builtin", &[("name", name.into())]),
        };
        if !self.hub.confirm(message, false).await {
            return self.denied();
        }
        self.apply_voice(event, true).await
    }

    /// Télécharge le modèle de l'agent local, après confirmation.
    pub(crate) async fn download_agent_model(&self) -> CallToolResult {
        let checked = match self.voice_check(VoiceEvent::DownloadAgentModel).await {
            Ok(c) => c,
            Err(e) => return self.fail_with(e),
        };
        let Some(size) = checked.agent_model_size else {
            tracing::error!("modèle de l'agent vérifié sans sa taille");
            return self.fail_with(refusal(VoiceErrorKind::NothingToDownload));
        };
        let l = self.localizer.current();
        if !self.hub.confirm(l.tr_with("agent-confirm-agent-model", &[("size", size_text(size, &l).into())]), false).await {
            return self.denied();
        }
        self.apply_voice(VoiceEvent::DownloadAgentModel, true).await
    }

    /// Arrête le téléchargement du modèle de l'agent local.
    pub(crate) async fn cancel_agent_model_download(&self) -> CallToolResult {
        self.checked_then_apply(VoiceEvent::CancelAgentModelDownload).await
    }

    /// Vérifie puis applique sans question.
    async fn checked_then_apply(&self, event: VoiceEvent) -> CallToolResult {
        if let Err(e) = self.voice_check(event.clone()).await {
            return self.fail_with(e);
        }
        self.apply_voice(event, false).await
    }

    /// Ce qu'une demande changerait, vérifié par le port hors du fil async.
    async fn voice_check(&self, event: VoiceEvent) -> Result<Checked, ToolError> {
        let origin = self.origin(false);
        self.on_port(move |v| v.check(&event, &origin)).await
    }

    /// Applique une demande par le port, hors du fil async.
    async fn voice_apply(&self, event: VoiceEvent, confirmed: bool) -> Result<VoiceApplied, ToolError> {
        let origin = self.origin(confirmed);
        self.on_port(move |v| v.apply(event, origin)).await
    }

    /// Applique et rend `{applied}`.
    async fn apply_voice(&self, event: VoiceEvent, confirmed: bool) -> CallToolResult {
        let done = self.voice_apply(event, confirmed).await;
        self.reply(done, |a| self.json_result(&json!({ "applied": a })))
    }

    /// L'agent de cet onglet ; `confirmed` : l'utilisateur a dit oui à la question.
    fn origin(&self, confirmed: bool) -> Origin {
        Origin::Agent { tab: self.hub.tab().to_owned(), confirmed }
    }

    /// Un appel bloquant au port, hors du fil async.
    async fn on_port<T: Send + 'static>(
        &self,
        f: impl FnOnce(&dyn VoicePort) -> Result<T, VoiceError> + Send + 'static,
    ) -> Result<T, ToolError> {
        let voice = self.hub.voice().clone();
        tokio::task::spawn_blocking(move || f(voice.as_ref()))
            .await
            .context(TaskSnafu)?
            .map_err(|e| VoiceSnafu.into_error(Box::new(e)))
    }
}

/// La question d'un passage à l'API : hôte, adresse, modèle et variable de l'utilisateur.
fn api_question(t: &ApiTarget, l: &Localizer) -> String {
    l.tr_with("agent-confirm-dictation-api", &[
        ("host", t.host.as_str().into()),
        ("url", t.url.as_str().into()),
        ("model", t.model.as_str().into()),
        ("var", t.key_env.as_str().into()),
    ])
}

/// La question d'un modèle à télécharger, nommé et pesé.
fn download_question(m: &ModelView, api_model: &str, l: &Localizer) -> String {
    match m.kind {
        ModelKind::Apple => l.tr("agent-confirm-dictation-language"),
        ModelKind::Local | ModelKind::Api => l.tr_with("agent-confirm-dictation-download", &[
            ("name", model_label(m, api_model, l).into()),
            ("size", size_text(m.size, l).into()),
        ]),
    }
}
```

(`ApiTarget` values come from the user's validated settings and the validated API model; the normalized URL is ≤ 512 chars, so they enter the dialog whole. The `confirmed` flag tells the port the user said yes; the port refuses `Changed` if a needed confirmation was not given. `self.hub.tab()` is the hub's tab id — read its real accessor name.)

- [ ] **Step 4: Register and dispatch.** In `tools.rs` `list()` — the first sentence carries the rule; `SetDictationArgs` through `nest_schema`:
  - `show_dictation`: "Show the dictation and app agent settings box in the dock, open until the user closes it. It shows the recognition model, spoken language, auto stop, the dictation API, model downloads and the app agent. Returns {shown}."
  - `set_dictation`: "Change dictation settings, except the API address and key variable, which only the user sets in the box. Fields: model, language (a code or \"auto\"), auto_stop, silence (0.5–10 s), api_model; switching to api or to a model to download asks the user. Returns {applied: now | unchanged}."
  - `download_dictation_model`: "Download a dictation model or Apple language after the user confirms. Returns {applied}."
  - `cancel_dictation_download`: "Stop the dictation model download in progress; refused when none runs."
  - `select_agent`: "Switch the app agent to a preset id (see settings) after the user confirms; it takes effect once no reply runs. Returns {applied: now | when_idle | unchanged}."
  - `download_agent_model`: "Download the built-in local agent's model after the user confirms."
  - `cancel_agent_model_download`: "Stop the local agent model download in progress; refused when none runs."
  Dispatch in `handler.rs`; `set_setting`'s description says `returns {applied: next_launch}`; the `settings` tool description points at `set_dictation` for dictation.

- [ ] **Step 5: Whitelist and port.**
  - `settings.rs`: `const WRITABLE: &[&str] = &["history.limit"];`, `Applied` keeps only `NextLaunch`, the `SettingsPort` doc says dictation goes through `VoicePort`.
  - `guarded_tools.rs`: `denied` → `pub(crate)`; the `set_setting` doc loses its `dictation.*` list; `SettingLocked { key }` gets `clip(&key, 64)` (an agent-supplied key never carries bidi or invisible characters into a host message).
  - `tests/common` `MemorySettings::write` returns `Ok(Applied::NextLaunch)`.
  - Agent `settings_port.rs`: replace the `dictation.` branch of `write` by `if key.starts_with("dictation.") { return Err("dictation is set through set_dictation".to_owned()); }` (host log text), keep reading `dictation.*`, update the module doc.

- [ ] **Step 6: Instructions, families.** `instructions/agent.md`: replace "Other settings: set_setting." with "Dictation and the app agent: show_dictation shows their settings box; set_dictation changes the model, language, auto stop, silence and API model (only the user sets the API address and key variable, in the box); download_dictation_model and cancel_dictation_download; select_agent switches the app agent once no reply runs; download_agent_model and cancel_agent_model_download. Other settings: set_setting." `crates/oku-slides-local-agent/src/families.rs`: the seven tools in the `Configure` arm; its summary becomes "change app settings: language, dictation, the app agent, interface".

- [ ] **Step 7: Errors and keys.** `ToolError` variants (backtrace each; `Voice` boxes its source), keys × 20.

- [ ] **Step 8: Docs and rules.** mcp `ARCHITECTURE.md`: append `voice_tools` to the tool families line ("seven tools, checked before any question; no API address or key variable argument") and note `set_setting` without `dictation.*`. mcp `AGENTS.md`: replace the settled decision "Prefix dictation settings keys with dictation. …" by `- Change dictation only through VoicePort (set_dictation, the box); set_setting keeps no dictation key. (spec:2026-10-23-reglages-dictee-a2ui-design, t-1144)`; Invariants: `- Never let an agent set the dictation API address or key variable; refuse before parsing, with ApiReservedToUser. (spec:2026-10-23-reglages-dictee-a2ui-design)`, `- Check every voice tool request through VoicePort before confirming; a refusal opens no dialog. (spec:2026-10-23-reglages-dictee-a2ui-design)`; extend the `show_themes/show_packs` settled decision to `show_dictation`. Run `just agents-lint`.

- [ ] **Step 9: Run and check.** `cargo nextest run -p oku-slides-mcp -p oku-slides-local-agent -p oku-slides-agent -p oku-slides-i18n && just check && wc -l crates/*/ARCHITECTURE.md`. Expected: PASS.

- [ ] **Step 10: Commit** `feat(mcp)! : outils de la dictée et de l'agent vérifiés avant confirmation, API réservée à l'utilisateur, set_setting sans dictation.* (t-1144, t-1149)`.

---

### Task 5: The UI loses the popup; the old protocol goes; e2e (t-1150)

**Files:**
- Modify (UI): `ui/src/{api.ts,Dictation.tsx,App.tsx,AgentWindow.tsx,useAgentWiring.ts,styles.css,glass.css}`, `ui/src/agent/{AgentPanel.tsx,surfaces.ts,surfaces.test.ts,AgentDock.tsx,useQuickKeys.ts}`, `ui/src/agent/useRelocalizedPacks.ts` → `useRelocalizedBoxes.ts` (+ its test, renamed), tests `ui/src/{Dictation,AgentWindow}.render.test.tsx`, `ui/src/agent/{AgentPanel,AgentThread}.render.test.tsx`, `ui.ftl` × 20
- Create (UI): `ui/src/voiceSeq.ts`, `ui/src/voiceSeq.test.ts`
- Modify (Rust): agent `src/{stack.rs,events.rs,events_tests.rs,turns.rs,choice.rs,choice_tests.rs,model_download.rs,model_download_tests.rs,lib.rs}`, agent `tests/{stack.rs,availability.rs}`; stt `src/lib.rs` (`SttRequest`), `src/lib_tests.rs`, `src/dictation_stream_tests.rs`, `tests/{dictation.rs,apple.rs}`; `src-tauri/src/commands.rs` (doc); `agent.ftl` × 20 (delete `agent-picker-*`); agent `ARCHITECTURE.md`, `ui/AGENTS.md`
- Modify (e2e): `ui/e2e/210-agent-picker.cjs` → `ui/e2e/210-settings-box.cjs` (`git mv`, rewrite), `ui/e2e/lib/surfaceAction.cjs` (new), `justfile` (`e2e` recipe exports `OKU_E2E_DICTATION_KEY=oku-e2e-secret-1144` **before** the `serve &` line)
- Delete: `ui/src/DictationApiFields.tsx`, `ui/src/agent/{AgentPicker.tsx,AgentPicker.render.test.tsx,useAgentPicker.ts,useAgentPicker.render.test.tsx}`, `ui/src/a2ui/fixtures/agentPicker.ts`, agent `src/{picker.rs,picker_tests.rs}`, `src/snapshots/oku_slides_agent__picker__*.snap`

**Interfaces:**
- Consumes: `{ cmd: "show_settings", tab }`; agent event `{ kind: "voice" }`; surface `host-voice` (dock); SSE named events `mcp` and `agent` on `/api/events`.
- Produces: `agent.showSettings(): Promise<void>`; `HOST_VOICE` in `surfaces.ts`; `AgentPanel` prop `onVoice?: () => void` (called on `kind === "voice"` and after each SSE reconnect); `voiceSeq.ts`: `bumpVoiceSeq(): void`, `subscribeVoiceSeq(l: () => void): () => void`, `voiceSeq(): number`, `useVoiceSeq(): number`; `Dictation` prop `onSettings?: () => void` (absent → no ⚙), and `Dictation` calls `useVoiceSeq()` itself; `useRelocalizedBoxes(surfaces, reshow: ReadonlyMap<string, () => Promise<void>>, say, enabled)`.

- [ ] **Step 1: Write the failing tests.**

`voiceSeq.test.ts`:

```ts
import { expect, test } from "bun:test";
import { bumpVoiceSeq, subscribeVoiceSeq, voiceSeq } from "./voiceSeq";

test("bumpVoiceSeq with a subscriber calls it once", () => {
  let calls = 0;
  const off = subscribeVoiceSeq(() => calls++);
  bumpVoiceSeq();
  off();
  expect(calls).toBe(1);
});

test("bumpVoiceSeq once raises voiceSeq by one", () => {
  const before = voiceSeq();
  bumpVoiceSeq();
  expect(voiceSeq()).toBe(before + 1);
});

test("subscribeVoiceSeq after unsubscribe is not called", () => {
  let calls = 0;
  const off = subscribeVoiceSeq(() => calls++);
  off();
  bumpVoiceSeq();
  expect(calls).toBe(0);
});
```

`Dictation.render.test.tsx`:
- `mount(agentBusy = false, vocabulary = [], remote?, opts: { onSettings?: boolean } = {})` — default `onSettings: true`; pass a counting `onSettings` (or `undefined` when false) and drop the five picker props; return `settingsClicks()` too; remove the `PICKER` import.
- `fakeStt` names its status spy and returns `statusCalls: () => statusSpy.mock.calls.length`, `current(): SttStatus` and `defer(): { resolve(s: Partial<SttStatus>): void }` — the next `stt.status()` call returns a promise the test resolves by hand (merged over `current()`). Drop the `setInterval` stub the old popup tests needed.

```tsx
const gear = (host: HTMLElement) => host.querySelector(`[aria-label="${t("ui-dictation-settings")}"]`) as HTMLButtonElement | null;
const cancel = (host: HTMLElement) => host.querySelector(`[aria-label="${t("ui-dictation-cancel")}"]`);

test("Dictation gear click asks the host for the settings box", async () => {
  const m = await mount(false, [], undefined, { onSettings: true });
  await act(async () => gear(m.host)!.click());
  expect(m.settingsClicks()).toBe(1);
});

test("Dictation without onSettings shows no gear", async () => {
  const m = await mount(false, [], undefined, { onSettings: false });
  expect(gear(m.host)).toBeNull();
});

test("Dictation while recording shows cancel instead of gear", async () => {
  const m = await mount();
  await act(async () => (m.host.querySelector(`[aria-label="${t("ui-dictation-start")}"]`) as HTMLButtonElement).click());
  await settle();
  expect([gear(m.host) === null, cancel(m.host) !== null]).toEqual([true, true]);
});

test("Dictation refetches its status on a voice change", async () => {
  await mount();
  const before = engine.statusCalls();
  await act(async () => bumpVoiceSeq());
  await settle();
  expect(engine.statusCalls()).toBe(before + 1);
});

test("Dictation keeps its status without a voice change", async () => {
  const m = await mount();
  const before = engine.statusCalls();
  await m.rerender(false);
  await settle();
  expect(engine.statusCalls()).toBe(before);
});

test("Dictation late older reply does not overwrite a newer one", async () => {
  const m = await mount();
  const first = engine.defer();
  await act(async () => bumpVoiceSeq());
  const second = engine.defer();
  await act(async () => bumpVoiceSeq());
  second.resolve({ config: { ...engine.current().config, language: "fr" } });
  first.resolve({ config: { ...engine.current().config, language: "en" } });
  await settle();
  expect(m.host.querySelector(".mic")?.getAttribute("title") ?? "").toContain("Français");
});
```

(The last test checks one behaviour, "the newer reply wins", through the mic title. "Shows cancel instead of gear" asserts one behaviour — which button stands there — as a pair; it also checks the existing cancel path, which the recording test previously covered under another name.) Delete the popup tests; keep, as idle-caption assertions on `.dictation-line .caption`, "Apple chosen but unavailable" (`ui-dictation-model-unavailable`) and "API without key" (`ui-dictation-api-key-missing`).

Others:
- `surfaces.test.ts`: `isRequestedBox with the voice box is true`; `isHostSettings with the voice box is false`.
- `useRelocalizedBoxes.render.test.tsx` (renamed; tests renamed "useRelocalizedBoxes …", moved to the `Map` form): `useRelocalizedBoxes reshows the voice box on a locale change`, `useRelocalizedBoxes ignores a box absent from reshow`.
- `AgentPanel.render.test.tsx`: delete both picker tests (lines 315-328, including "thread has no agents button nor picker sheet"), the `PICKER` import, the `pickers`/`pickerSurfaces` harness returns and the picker fakes (lines 77-95, 166); add an `onVoice` prop to the harness and `AgentPanel voice event calls onVoice`, `AgentPanel reconnect calls onVoice`; retarget the quick-keys test from `.agent-picker` to a chip inside `[data-surface-id="host-voice"]` (`AgentPanel quick digit inside the voice box answers no question`).
- `AgentThread.render.test.tsx:444-447`: delete the `.agent-picker` test.
- `AgentWindow.render.test.tsx`: the fake host gains `showSettings: async () => void calls.push("showSettings")` and loses the picker fakes; `openSettings` asserts the gear exists before clicking; `AgentWindow gear asks the host for the settings box`; the read-only test asserts there is no gear.

- [ ] **Step 2: Run them to see them fail.** `cd ui && bun test src/voiceSeq.test.ts src/Dictation.render.test.tsx src/AgentWindow.render.test.tsx src/agent`. Expected: FAIL.

- [ ] **Step 3: `api.ts`.** Remove `{ kind: "picker"; surface }`, add `{ kind: "voice" }`; remove the `picker`, `select_agent`, `download_model`, `cancel_download` commands and API methods; add `| { cmd: "show_settings"; tab: string }` and `/** Ouvre la boîte des réglages de la dictée et de l'agent. */ showSettings: () => agentCall({ cmd: "show_settings", tab }),`. Remove `stt.configure`, `stt.download` and their `SttRequest` members.

- [ ] **Step 4: `voiceSeq.ts`.**

```ts
// La révision de la dictée et de l'agent vue de la page, hors de l'état d'App.

import { useSyncExternalStore } from "react";

let value = 0;
const listeners = new Set<() => void>();

/** La dictée ou l'agent ont changé : les abonnés relisent. */
export function bumpVoiceSeq(): void {
  value += 1;
  for (const l of listeners) l();
}

/** Abonne `l` aux changements ; rend de quoi le désabonner. */
export function subscribeVoiceSeq(l: () => void): () => void {
  listeners.add(l);
  return () => void listeners.delete(l);
}

/** La révision courante. */
export function voiceSeq(): number {
  return value;
}

/** La révision courante, relue par React à chaque changement. */
export function useVoiceSeq(): number {
  return useSyncExternalStore(subscribeVoiceSeq, voiceSeq);
}
```

- [ ] **Step 5: `surfaces.ts`, dock, relocalization, quick keys.**
  - `surfaces.ts`: `/** La boîte des réglages de la dictée et de l'agent. Ouverte par le ⚙ ou l'agent. */ export const HOST_VOICE = "host-voice";` and `isRequestedBox` includes it (doc: thèmes, packs, réglages).
  - `AgentDock.tsx`: import `HOST_VOICE`; `aria-live={isHostSettings(s) || s.id === HOST_VOICE ? "off" : undefined}`; update the comments at lines 40 and 44-45.
  - `useRelocalizedBoxes` (`git mv` + generalize): signature from Interfaces; in the locale listener, `for (const s of now.surfaces) now.reshow.get(s.id)?.().catch((e) => now.say(errorText(e), true));` (a `Map`, so no prototype keys). `AgentPanel`: `const reshow = useMemo(() => new Map([[HOST_PACKS, () => agent.showPacks()], [HOST_VOICE, () => agent.showSettings()]]), [agent]);`.
  - `useQuickKeys.ts`: ``if (e.target instanceof Element && e.target.closest(`[data-surface-id="${HOST_VOICE}"]`)) return;`` with `// La boîte des réglages garde ses touches (focus clavier) : rien ne va à la question ouverte.`; doc: "ou dans la boîte des réglages (focus clavier ; un clic de souris ne la focalise pas dans le WebView)".

- [ ] **Step 6: `Dictation.tsx`.**
  - Delete the `settings` state, the popup JSX, `configure`, `modelNote`, `SILENCES`, the `stt.download` call, and the unused imports (`SttConfig`, `SttModel`, `Surface`, `Field`, `Select`, `Button`, `DictationApiFields`, `AgentPicker`); keep `languages`, `NAMED`, `mo`, `secs`; add `import { IconButton } from "./primitives";` and `import { useVoiceSeq } from "./voiceSeq";`.
  - Props: remove the five picker props; add `onSettings?: () => void` (`/** Ouvre la boîte des réglages ; absent, pas de ⚙ (vue en lecture seule). */`); `onSettings` joins `handlers`; drop `setSettings(false)` in `toggle`.
  - Keep the ternary; its else branch becomes `onSettings && <IconButton className="token ghost" label={t("ui-dictation-settings")} icon={GearIcon} disabled={state !== "idle"} onMouseDown={(e) => e.preventDefault()} onClick={() => handlers.current.onSettings?.()} />`.
  - Order replies by request, not by `seq`:

```tsx
  // Les réponses du statut arrivent dans le désordre : seule la plus récente demandée s'applique.
  const asked = useRef(0);
  const send = useCallback(
    (p: Promise<SttStatus>) => {
      const n = ++asked.current;
      return p
        .then((s) => {
          if (n === asked.current) receive(s);
        })
        .catch((e) => handlers.current.onError(String(e instanceof Error ? e.message : e)));
    },
    [receive],
  );
```

    (`receive` keeps its `s.seq !== lastSeq.current` rule for `onText`: `seq` only moves with a new text, and a server restart resets it, which `!==` handles.)
  - Replace the existing mount effect `useEffect(() => { send(stt.status()); }, [send]);` by:

```tsx
  // Au montage, puis quand la dictée ou l'agent ont changé : l'état du micro se relit.
  // Sans effet de bord : relancé par StrictMode, il relit deux fois, sans plus.
  const settingsSeq = useVoiceSeq();
  useEffect(() => {
    send(stt.status());
  }, [settingsSeq, send]);
```

  - Component doc: "L'appareil de dictée : micro, clavier, niveau et texte provisoire." + "Ses réglages sont la boîte de l'hôte (⚙)."

- [ ] **Step 7: Wiring.**
  - `AgentPanel.tsx`: remove `onPicker` and its event branch; add `onVoice?: () => void` (doc), called on `kind === "voice"` and in `onReady` (after each SSE reconnect, events may have been missed).
  - `useAgentWiring.ts`: drop `useAgentPicker` and its return member.
  - `App.tsx`: drop `agentPicker` from the wiring destructuring and `onPicker=` on its `AgentPanel`; `onVoice={bumpVoiceSeq}`; on `Dictation` (lines 504-508) remove the five picker props and pass `onSettings={() => void agent.showSettings().catch((e) => say(errorText(e), true))}` unconditionally (App only mounts Dictation in editable, attached views).
  - `AgentWindow.tsx`: rename `dictation()`'s `picker = true` parameter to `settings = true`; on its `Dictation` (lines 224-228) remove the five picker props and pass `onSettings={settings ? () => void host.agent.showSettings().catch((e) => say(errorText(e), true)) : undefined}` (import `errorText`); its `AgentPanel` gets `onVoice={bumpVoiceSeq}` and loses `onPicker`; remove the `useAgentPicker` import; keep `NOOP`; update the comment at line 213 ("the read-only window has no `AgentPanel`: its mic status refreshes only while dictation runs; it has no ⚙").

- [ ] **Step 8: Rust removals.**
  - `stack.rs`: remove `AgentCommand::{Picker, SelectAgent, DownloadModel, CancelDownload}`, their `tab()` arms and `emit_picker`; fix the `handle` doc that names `select_agent`.
  - `events.rs`: remove `AgentEvent::Picker` (+ test); `turns.rs`: drop `| AgentEvent::Picker { .. }` at both sites.
  - `choice.rs`: delete `picker()`; `choice_tests.rs` keeps its `LocalModelView` expectations. `model_download.rs`: delete `failure_text` and its tests (`model_download_tests.rs` line 19 import, lines 198-210).
  - Delete `picker.rs`, `picker_tests.rs`, the `picker` snapshots and `mod picker`; delete the `agent-picker-*` keys from the 20 `agent.ftl`.
  - stt: remove `SttRequest::{Configure, Download}` and their `handle` arms; delete `lib_tests.rs` `stt_request_partial_configure_fills_defaults`; `dictation_stream_tests.rs:146,157` call `let _started = stt.download(Model::Apple, false);` and `stt.status()`; `tests/dictation.rs:168` and `tests/apple.rs:25` call `let _applied = stt.configure(…)` (with the one-line comment); fix the `commands.rs:182` doc.
  - `tests/stack.rs`: the `select` helper and the ported tests stay as done in Task 3; delete `picker_lists_the_agents_with_the_active_one_chosen`.
  - `tests/availability.rs`: port the two download tests to `show_settings` then `surface_action` `download_agent_model` / `cancel_agent_model_download` on `host-voice`. With nothing to cancel the call is refused: `assert_that!(r).is_err_containing("Aucun téléchargement n'est en cours".to_owned())`; off Apple Silicon `download_agent_model` gives `"Rien à télécharger pour cet agent".to_owned()`, asserted the same way; then wait for the whole `host-voice` surface re-sent after the refusal. On Apple Silicon, read the box's `a-error` component (was the picker's `download-error`), keeping the availability check after it.
  - Check: `rg -n 'AgentCommand::(Picker|SelectAgent|DownloadModel|CancelDownload)|AgentEvent::Picker|picker_surface|LocalPanel|"picker"|\.agent-picker\b|agent-picker-|SttRequest::(Configure|Download)|failure_text' crates src-tauri ui/src ui/e2e --glob '!**/target/**'` — Expected: no match.

- [ ] **Step 9: Delete files, CSS and keys.** Delete the UI files listed above. CSS — delete exactly: in `styles.css` `.dictation-settings` and its compound rules (`> *`, `:hover`, `.btn`, `.settings-row input:not([type])`), `.badge`, `.badge.ok`, `.dictation-agents`; in `glass.css` the `.dictation-settings` rules (~lines 80, 90, 109). Keep `.model`, `.model-name`, `.model-list`, `.settings-title`, `.settings-row`, `.check`, `@keyframes rise` (ImportDialog and others use them). Before each deletion: `rg -n '\b<class>\b' ui/src -g '*.tsx' -g '*.ts'` finds nothing. Keys: delete from the 20 `ui.ftl` the keys moved in Task 2 **and `ui-dictation-silence`**, each only after `rg -n '"<key>"' ui/src -g '!*.test.ts' -g '!*.test.tsx'` finds nothing.

- [ ] **Step 10: e2e.**
  - `justfile` `e2e` recipe: `export OKU_E2E_DICTATION_KEY=oku-e2e-secret-1144` next to the `OKU_SLIDES_PATHS__*` exports (before `serve &`).
  - `ui/e2e/lib/surfaceAction.cjs`: `waitSurfaceAction(page, surface, name)` — a `page.waitForResponse` whose request body parses (try/catch) to `{cmd: "surface_action", surface, name}`, response ok.
  - `git mv ui/e2e/210-agent-picker.cjs ui/e2e/210-settings-box.cjs` and rewrite it (French labels; presets « Faux », chosen, and « Faux bis »). Header: what it covers; it restores the agent, language, model, API address and key variable for later specs. Before `p.goto`, `await p.addInitScript(() => { const Native = window.EventSource; window.__sse = []; window.EventSource = class extends Native { constructor(...a) { super(...a); for (const n of ["mcp", "agent"]) this.addEventListener(n, (e) => window.__sse.push(e.data)); } }; });` (the server sends named events). Locators: `const BOX = '.agent-dock [data-surface-id="host-voice"]'`, `const part = (id) => \`${BOX} [data-component-id="${id}"]\``, `chip = (p, id, name) => p.locator(\`${part(id)} .chip\`, { hasText: new RegExp(\`^\\s*${name}\\s*$\`) })`; `isChosen` reads `aria-checked`; `untilChosen` on `e2e/lib/until.cjs`. Steps:
    1. `const start = await post(p, '/api/stt', { cmd: 'status' })` (config: model, language, api url and key_env); record the label of the `d-models` chip whose `aria-checked` is true.
    2. Click the gear (`.device.dictation button[aria-label="Réglages"]`), wait for `BOX`.
    3. « Faux » chosen; click « Faux bis » (`waitSurfaceAction(p, 'host-voice', 'select')`); `untilChosen('a-agents', 'Faux bis')`.
    4. Prompt and read the second agent's reply (as the old spec did); `BOX` still there after the turn.
    5. `p.locator(\`${part('d-language')} select\`).selectOption({ label: 'English' })`; poll the mic `title` until it contains `English`; then `selectOption({ value: start.config.language ?? '' })` and poll `/api/stt` status until `config.language` equals the start value.
    6. Choose « Local »: on `process.platform === 'darwin' && process.arch === 'arm64'`, `part('a-download')` is visible; elsewhere it has count 0.
    7. Choose « Faux » back.
    8. `try { … } finally { restore }`: click the API model chip; fill `${part('d-api-url')} input` with `http://127.0.0.1:9/v1` (a closed local port: no audio ever leaves the machine) and press Enter; fill `${part('d-api-key-env')} input` with `OKU_E2E_DICTATION_KEY` and press Enter (each with `waitSurfaceAction(…, 'api')`). Positive control: poll `/api/stt` status until `models.find(m => m.id === 'api').installed === true` (fail with "lancer la spec avec just e2e : la variable du test manque" otherwise). Then assert `oku-e2e-secret-1144` appears nowhere: not in `await p.content()`, not in the values of every `input, textarea, select` (`p.evaluate`), not in `await p.evaluate(() => window.__sse.join('\n'))`, not in any `/api/agent` or `/api/stt` response body collected with `p.on('response')` during the step (each `await r.text()` in try/catch), and not in the file `process.env.E2E_AGENT_LOG`. `finally`: fill the address and variable back with `start.config.api.url` / `start.config.api.key_env` through the box, click the recorded model chip, poll `/api/stt` status until model, url and key_env equal the start values.
    9. Click `part('close')`: `BOX` is detached.
    No `page.waitForFunction` with an async predicate.

- [ ] **Step 11: Docs and rules.** agent `ARCHITECTURE.md`: drop `picker` from the module line and the `choice` line, and `failure_text` from the `model_download` line, by rewriting. `ui/AGENTS.md` Invariants: `- Open dictation and agent settings only via agent.showSettings; the UI draws no settings form. (t-1144)`.

- [ ] **Step 12: Run and check.** `cd ui && bun test && bunx tsc -b && cd .. && cargo nextest run -p oku-slides-agent -p oku-slides-stt -p oku-slides-i18n && just check && just e2e 4999`. Expected: PASS.

- [ ] **Step 13: Commit** `feat(ui)! : le ⚙ de la dictée ouvre la boîte host-voice ; popup, panneau des agents et anciennes commandes supprimés, e2e de la boîte (t-1144, t-1150)`.

---

### Task 6: Help, README and the final gate (t-1151)

**Files:**
- Modify: `docs/aide/en.json`, `docs/aide/fr.json`, `README.md` (dictation settings and agent choice, around lines 288-304)
- Read first: `docs/AGENTS.md`

- [ ] **Step 1: Help.** In both help files, dictation section: the ⚙ of the dictation bar opens the settings in the agent area; they stay open until closed and follow every change; the agent can open and change them when asked (switching to the API, a download and an agent switch ask you first; an agent switch waits until no reply runs); only you set the API address (https, or http for this computer) and the key variable (a name like OPENAI_API_KEY), in the box. Rewrite the existing API line. Follow `docs/AGENTS.md` (run `cargo nextest run -p oku-slides-session help_`).

- [ ] **Step 2: README.** Rewrite the lines saying that a dictation setting change waits for confirmation and that the agent cannot change the agent: the settings are a box in the agent area; the agent changes plain dictation settings at once, asks before switching to the API, downloads and agent switches; the API address and key variable stay the user's.

- [ ] **Step 3: Full gate.** `just ci && just e2e 4999`. Expected: PASS.

- [ ] **Step 4: Commit** `docs(stt) : aide et README de la boîte des réglages de la dictée et de l'agent (t-1144, t-1151)`.

---

## Self-review against the three review rounds

| Finding | Where fixed |
|---|---|
| Agent can send any secret to any host | Decision 1; Task 3 port rules; Task 4 pre-parse refusal, schema tests, regressions |
| `ApiReservedToUser` unreachable by the agent | Task 4 `API_TARGET_KEYS` check before parsing; first sentence of the tool description, tested |
| URL validated ≠ requested; long, empty or odd labels; idempotence; loopback aliases; proxy; redirects | Task 1 `normalized_url`, `is_loopback`, `ApiTranscriber` |
| Key copied on every status | Task 1 `key_present` |
| Danger dialog naming the stored API model | Task 3 effective API model in `ApiTarget` |
| Unconfirmed apply after a state change | Task 3 `Origin::Agent { confirmed }`, `Changed` |
| User locked out by an old invalid address | Task 3: stored target checked only for the agent |
| Late `end_turn`, lost ends, closing mid-turn, guard count underflow | Task 3 `PromptGuard` by unique id in `TabAgent::run`; `tab_closed` drops only the pending choice |
| Config naming another agent than the active one; crossed clicks; abandoned switch saved | Task 3 switch under the lock, then `persist` of `active()` under `saving` |
| Clicking the active agent restarts it | Task 3 `switch_to` no-op on the active id |
| Refresh lost during a click; double send paths | Task 3 one `send_settings` path, box kept in `shows` |
| Old picker commands writing behind `AppVoice` | Task 3 routes them through the port |
| `voice` through a 6-parameter public `spawn` | Task 3 `ActorParts` + `spawn_counted` |
| Non-compiling or failing-check items (fixture visibility, `Default` derives, column `&[&str]`, `Out`/`emit`, `turns.rs`, `tab()`, `CommandError` dead variants, `failure_text`, stt tests, `tab_entry_tests`, `choice_tests`, availability events, `ui-dictation-silence`, picker props, AgentPanel tests) | Tasks 2–5 as listed in their steps |
| `voice_port.rs` too wide | Task 3 split: `voice_port`, `agent_switch`, `stt_watch` |
| Duplicated GB formatting; float noise in seconds | Task 2 `gigabytes_text` reused by `error.rs`; `seconds_text` from f32's shortest form |
| Box rebuilt on busy/pending; App re-rendered per percent; missed events after reconnect | Task 2 data-bound lines; Task 3 bumps only on idle↔busy; Task 5 `useVoiceSeq`, `onReady` |
| Status replies applied out of order | Task 5 request counter in `send` |
| Tests that could not observe, flake or pass vacuously | `voiceSeq` exports; deterministic `switch_now` tests; `call_within`; e2e positive control, SSE capture, closed local URL, `finally` restore |
| e2e broken between commits; platform-dependent check | Task 5 rewrites 210 and runs `just e2e` |
| Naming collision `host-settings` | `host-voice` everywhere |
| Trust boundary of the local server | Decision 6 and the spec |
