# Local fallback agent, lots 2, 5, 4 and 6 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended)
> or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Finish the local fallback agent of t-1009: a read-only `diagnose` MCP tool for every agent
(lot 2), the automatic fallback to the local agent when the chosen one is unusable (lot 5), an Apple
Foundation Models engine (lot 4), and the agent's own instructions, fix recipes, evaluation set and
family tuning (lot 6), with the lot 1 review follow-ups.

**Spec:** `docs/superpowers/specs/2026-10-18-agent-local-repli-design.md` (approved 2026-10-18),
sections "Tool coverage", "Preset and offer", "Engines", "Testing" and "Lots". This plan settles the
points the spec leaves open; each is marked **Decision**.

**Tasks:** parent t-1009; lots t-1021 (Tasks 1-5 = t-1448..t-1452), t-1024 (Tasks 6-9 = t-1453..t-1456), t-1023 (Tasks 10-12 = t-1457..t-1459),
t-1025 (Tasks 13-14 = t-1460, t-1461); Task 15 = t-1462 closes. Oku sub-tasks carry the task titles verbatim.

**Tech stack:** Rust 2024, `tracing-subscriber` layer, Swift 5.9 static bridge through `swift-rs`
(FoundationModels, macOS 26+, weak-linked), `agent-client-protocol`, `rust-mcp-sdk` 1.x, React/TS (Bun),
Playwright e2e.

## Global constraints

- Root `AGENTS.md` and the AGENTS.md of each touched crate apply in full. Highlights: no `unwrap` or
  `expect` outside tests, no silent fallback, Snafu errors with a backtrace and opaque public types,
  Fluent for every user-facing text (English first, then all 20 locales), no `std::fs` in production
  code, never hold a `std::sync::Mutex` guard across an `.await`, sibling `x_tests.rs` with speculoos,
  tests named `unit_scenario_result`, `ARCHITECTURE.md` ≤ 50 lines kept current, every `pub` item
  documented, files ≤ 500 lines (cap 1000).
- macOS minimum stays 12.0: everything Apple-model goes behind a runtime check (`@available` in Swift,
  `FoundationModels` weak-linked); the app must still launch on macOS 12.
- `exit_if_worker` then `exit_if_local_agent` stay the first lines of every app `main`.
- Runs outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir.
- Long gates (`just test`, `just e2e`, full nextest) run detached (`nohup`) with logs under
  `.superpowers/sdd/lot-l/`.
- Commits: Conventional Commits in English, `type(scope): description (t-N)`, citing the lot task and
  its sub-tasks; no `Co-Authored-By`. Implementers stage nothing and write their commit message to
  `.superpowers/sdd/lot-l/msg-<lot>.txt`; the controller reviews, stages and commits.

## Lot 2 — `diagnose` (t-1021)

**Decisions.** `diagnose` takes no argument and returns one JSON object. Host data comes through a new
`VoicePort::diagnosis()` method (the port already sees the agent choice and the dictation; `NoVoice`
returns the default), log lines from a ring in `oku-slides-platform::telemetry`, tab errors from the
session. Redaction happens where text is captured *and* once more on the whole reply (home dir → `~`).
Output, field names fixed:

```json
{ "agent": { "selected": "claude", "name": "Claude", "builtin": false,
             "unusable": "…localized reason or null…", "stderr": ["…last ≤ 40 lines…"] },
  "dictation": { "microphone": "granted|denied|restricted|undetermined|unknown",
                 "engine": "apple|turbo|large|base|api", "model": "missing|partial|complete",
                 "language": "fr|auto" } | null,
  "log": [ { "level": "WARN|ERROR", "at": "RFC 3339", "target": "…", "message": "…" } ],
  "render_error": "…one line…" | null, "import_error": "…one line…" | null }
```

### Task 1: Ring of recent warnings and errors in the app log

**Files:** `crates/oku-slides-platform/src/telemetry.rs` (+ new `log_ring.rs`, `log_ring_tests.rs`),
`crates/oku-slides-platform/ARCHITECTURE.md`.

- [ ] Failing tests first: a `Layer` given INFO, WARN and ERROR events keeps only WARN and ERROR; past
  `LOG_RING_CAP` (50) the oldest go; a message holding the home dir comes back with `~`; a line longer
  than `LOG_LINE_MAX` (500 chars) is cut on a char boundary with `…`.
- [ ] `LogRing` (`Mutex<VecDeque<LogLine>>`, poison-safe), `LogLine { level, at, target, message }`
  (`Serialize`), message = the event's `message` field followed by `key=value` fields; `redact_home(text,
  home)` replaces the home path (from `std::env::home_dir`, read once) with `~`.
- [ ] `init()` adds the layer under the same `EnvFilter` (so `async_openai=off` still drops secrets) with
  its own `LevelFilter::WARN`; `pub fn recent_problems() -> Vec<LogLine>` reads the global ring.

### Task 2: Tail of the agent's launch stderr

**Files:** `crates/oku-slides-agent/src/link.rs` (+ new `stderr_tail.rs`, `stderr_tail_tests.rs`).

- [ ] Failing tests first: lines fed to `StderrTail` keep the last 40; a new launch clears it; a line
  over 500 chars is cut; every line is still forwarded (writer injected in tests).
- [ ] `launch` pipes the child's stderr; a task reads it line by line (lossy UTF-8), writes each line to
  the app's stderr unchanged (today's `inherit` behaviour) and pushes it into the link's tail, cleared at
  each launch. `Link::stderr_tail() -> Vec<String>`.

### Task 3: Microphone permission and dictation state for diagnosis

**Files:** `crates/oku-slides-stt/swift/Sources/OkuSpeech/OkuSpeech.swift`, `crates/oku-slides-stt/src/apple.rs`,
`crates/oku-slides-stt/src/lib.rs`; `crates/oku-slides-mcp/src/voice.rs` (+ new `diagnosis.rs`),
`crates/oku-slides-agent/src/voice_port.rs`, `crates/oku-slides-agent/src/choice.rs`.

- [ ] Swift `oku_speech_microphone() -> Int32` (`AVCaptureDevice.authorizationStatus(for: .audio)`:
  0 undetermined, 1 restricted, 2 denied, 3 granted), same code table in Rust; `Stt::microphone()` →
  `Microphone` enum, `Unknown` off macOS. Never requests the permission.
- [ ] `oku-slides-mcp::diagnosis`: `Diagnosis { agent: AgentDiagnosis, dictation: Option<DictationDiagnosis> }`
  (`Serialize`, `Default`), `VoicePort::diagnosis(&self) -> Diagnosis` (may block: disk).
- [ ] `AppVoice::diagnosis`: selected preset id and name, `builtin`, `choice.availability()`, `link.stderr_tail()`;
  dictation from `Stt`: microphone, chosen model id, `missing|partial|complete` (complete = installed,
  partial = a download of it runs or its `.part` file exists, missing otherwise), language code or `auto`.
- [ ] Tests: `AppVoice::diagnosis` with a missing agent program names it unusable; with no dictation it is `null`.

### Task 4: Last render error and last import error of the tab

**Files:** `crates/oku-slides-session/src/` (the session's render and import paths), its ARCHITECTURE.md.

- [ ] Failing tests first: a deck whose render fails records its error; a later successful render clears
  it; a failed import records its error.
- [ ] The session keeps `last_render_error` and `last_import_error` (`Option<String>`, one line
  through `oku_slides_platform::errors::one_line`), set at the one place each failure is surfaced to the
  UI, read with `TabSession` accessors. No new persistence.

### Task 5: MCP tool `diagnose`

**Files:** `crates/oku-slides-mcp/src/` (new `diagnose_tools.rs`, `diagnose_tools_tests.rs`, `tools.rs`
list, `handler.rs` dispatch), `crates/oku-slides-agent/src/instructions/*.md` (one line in the shared
core: call `diagnose` when something fails), `crates/oku-slides-mcp/ARCHITECTURE.md`.

- [ ] Failing tests first (real server on 127.0.0.1:0 with an `rust-mcp-sdk` client, as the crate
  requires): `diagnose` is listed for every document kind; its reply has the five keys; a home path in a
  log line or stderr line comes back as `~`; `log` never holds INFO; caps 40 and 50 hold.
- [ ] Read-only, all kinds, no confirmation; reads the port off the async thread (`spawn_blocking`).
  Its schema has no property. The local agent's family table already maps it to `troubleshoot`.

## Lot 5 — automatic fallback (t-1024)

**Decisions.**
- The fallback preset is the first preset with `builtin = "local"`. No fallback when the selected agent
  is itself built-in, or when no built-in preset exists.
- `AgentChoice` gains a `Fallback { reason: Unusable-or-text, main_failure: Option<(Unusable, Instant)> }`
  while the link runs the local preset. `active()` stays the *selected* preset and is never written:
  `agent.selected` is not touched. The main agent's availability while in fallback uses its own failure
  remembered at the switch plus the static checks, so it is retried once that failure is stale. The wait
  starts at the link's `FAILURE_TTL` (30 s) and doubles after each failed retry, capped at 10 min; it resets
  when the main agent serves a turn or the user chooses an agent (review round, t-1462: a steady fallback
  otherwise relaunched both agents after every pause over 30 s).
- While standing in, the agent choice keeps the main agent's stderr tail and its session aside: `diagnose`
  reports the main agent, and back on it the tab resumes the main agent's session (in memory only).
- The decision `AppVoice::turn_route(tab) -> TurnRoute` runs at the start of each prompt, before
  `connect`, under the `AgentSwitch` lock: `Main`, `Local { reason }`, `NeedsDownload { reason, size }`,
  `Unusable { main, local }`. It switches the link (either way) only when this tab's guard is the only
  running prompt; otherwise it keeps the current route. Without `AppVoice` (tests), `Main`.
- When `connect` fails with an `Unusable` error on the main agent, the failure is remembered, the route is
  decided again once, and the turn retries on the local agent if it is ready.
- Notice: an A2UI line (`show_static` in the turn): "Local agent — <main agent's reason>", localized,
  and `AgentEvent::Fallback { reason: Option<String> }` to the tab (`None` once back on the main agent).
- Download: a guarded confirmation (`SurfaceHub::confirm`) naming the model and its size; yes → the
  existing `ModelDownload` starts, a progress line in the turn moves by 1 % steps, the turn runs when it
  completes (or fails with the download's localized error); no → the turn fails with the main reason.
- `Stack::tune` refuses only when neither the main agent nor the local agent can run (download included).

### Task 6: Fallback decision in the agent choice

**Files:** `crates/oku-slides-agent/src/choice.rs`, `voice_port.rs`, `agent_switch.rs`, new `fallback.rs`
(+ tests), `error.rs` (Fluent keys in `oku-slides-i18n`).

- [ ] Failing tests first, one per route: main usable → `Main`; main missing + local ready → `Local` and
  the link runs the built-in preset while `active()` and `config.toml` keep the selected one; local needs
  a download → `NeedsDownload`; local unsupported → `Unusable` with both reasons; back to `Main` as soon
  as the main is usable again; no switch while another tab prompts; selected agent built-in → no fallback.

### Task 7: Fallback turn: notice, download confirmation, retry, tuning

**Files:** `crates/oku-slides-agent/src/tab_agent.rs`, `stack.rs`, `events.rs`, `turns.rs`,
`crates/oku-slides-session` (notice and progress surfaces, if built there like other agent cards),
Fluent keys.

- [ ] Failing tests first (stack tests with the fake agent and the built-in preset with the fake engine
  `OKU_SLIDES_LOCAL_AGENT__ENGINE=fake`): a prompt with a missing main program runs on the local agent and
  ends in its `report`, the turn shows the notice and the tab gets `Fallback`; a launch failure discovered
  at `connect` retries on the local agent; the download confirmation answered no fails the turn with the
  main reason; a tune turn is accepted while the local agent is ready.

### Task 8: Fallback badge in the agent panel and e2e

**Files:** `ui/src/agent/useAgentAvailability.ts`, `AgentPanel.tsx`, their tests, the UI's message
catalog, new `ui/e2e/340-local-fallback.cjs` (+ `e2e/lib` helpers if needed).

- [ ] `useAgentAvailability` tracks `fallback: string | null`; `AgentPanel` shows "Local agent (fallback)"
  with the reason while it is set. Render tests first.
- [ ] e2e: a server whose selected preset's program is missing and whose built-in preset runs the fake
  engine; a prompt runs on the local agent and the panel shows the fallback. The spec starts its own
  server (fresh data dir) if the shared one cannot be configured so without breaking later specs.

### Task 9: Lot 1 review follow-ups

**Files:** `crates/oku-slides-local-agent/src/{acp,sessions}.rs` and tests, `crates/oku-slides-local-agent/tests/`
(`acp_wire.rs`, `builtin_preset.rs`), `crates/oku-slides-agent/tests/builtin_preset.rs`, `crates/oku-slides-cli/tests/`.

- [ ] A prompt on a built-in preset without the app binary fails with the no-program reason (test; the
  turn's outcome is `agent-local-no-program`, `AgentInner::NoProgram`, since 0f73f0e9).
- [ ] The first MCP connect of a session races the turn's cancellation (`select!` on the token).
- [ ] Sessions are dropped when the ACP connection closes and when `session/new` replaces a tab's
  previous session for the same MCP URL, so `McpTools::drop` runs (test: N new conversations keep one client).
- [ ] `session/load`: advertised (`load_session`), restores the session's MCP access and keeps its
  summaries for the life of the process; an unknown id is refused with `resource_not_found`, which the
  host handles as `Refused`.
- [ ] A `Stack`-level test with the built-in preset (`[program, --local-agent]`, fake engine) ending in a
  `report`, in `oku-slides-local-agent/tests` (the only crate with the dev launcher binary; `program` is a
  shell script giving the child its fake engine and temp dirs); the CLI `local_agent` test reads with a timeout.

## Lot 4 — Apple Foundation Models engine (t-1023)

**Decisions.**
- New leaf crate `crates/oku-slides-apple-model`: the Swift package `swift/` (target and product
  `OkuModel`, laid out like `OkuSpeech`), its `build.rs`, and the only `unsafe` of the feature in
  `bridge.rs`. Both `oku-slides-platform` (availability, for `local_engine`) and
  `oku-slides-local-agent` (the engine) depend on it, so one crate links the bridge.
- `FoundationModels` is weak-linked (`-weak_framework`); every Swift entry is guarded by
  `#available(macOS 26.0, *)`. Verify with `otool -l` that the app binary loads it with
  `LC_LOAD_WEAK_DYLIB`.
- The bridge is minimal; prompt, schema and reply logic stay in tested Rust. C ABI:
  `oku_model_availability() -> i32` (0 ready, 1 device not eligible, 2 Apple Intelligence off,
  3 model not ready, 4 macOS too old, 5 unknown) and
  `oku_model_respond(instructions, prompt, schema_json, temperature, max_tokens, ctx, cb, free) -> handle`,
  `oku_model_cancel(handle)`, `oku_model_release(handle)`; `cb(ctx, kind, text)` with kind 0 reply
  (the generated JSON), 1 error (code + English message: context overflow, guardrail, other).
- Each completion is one fresh `LanguageModelSession(instructions:)` and one guided `respond(to:schema:)`
  whose schema is JSON decoded into `GenerationSchema` (its `Codable` form is JSON Schema with `$defs`,
  `title`, `x-order`, `additionalProperties: false`; probed on macOS 27). Route: an object with
  `families`, an array of 1-2 offered keys. Act (revised in Task 14, the context being 4,096 tokens):
  a pick `{tool: <enum of exposed names>}`, then, only when the tool's schema passes 6,000 bytes, a pick
  of 1-3 tags of its biggest tagged union (`apply`'s `Op`), then the call
  `{tool: <its name>, arguments: <its compact schema, union narrowed>}` alone, with no `{text}` branch.
- Setting `local_agent.engine = "auto" | "apple" | "gguf"` (+ `fake` in debug). `auto` takes Apple when
  ready, else Gguf. A forced unavailable engine gives its reason. New `Unavailable` / `Unusable` variants:
  `AppleIntelligenceOff`, `AppleModelNotReady`, `AppleUnsupported` (device or macOS); under `auto` on a
  machine without Gguf, the Apple reason is shown when the user can fix it (off, not ready).

### Task 10: Swift bridge crate `oku-slides-apple-model`

**Files:** new crate (Cargo.toml with `swift-rs` build-dep on macOS only, `build.rs`, `swift/Package.swift`,
`swift/Sources/OkuModel/OkuModel.swift`, `src/lib.rs`, `src/bridge.rs`, `src/bridge_tests.rs`,
`AGENTS.md`, `ARCHITECTURE.md`), root `Cargo.toml` workspace member and dependency, README crate map.

- [ ] Safe API: `availability() -> AppleModel` (enum above; `MacOsTooOld` off macOS), and
  `respond(&Request, &CancelToken) -> Result<String, AppleModelError>` (blocking, its own channel; Snafu,
  backtrace, kinds `ContextOverflow`, `Refused`, `Failed`, `Cancelled`).
- [ ] Tests: code tables both ways; an `#[ignore]` manual test runs one guided reply on this Mac.

### Task 11: Apple engine in `local_engine` and availability

**Files:** `crates/oku-slides-platform/src/{config.rs,local_agent.rs,config.default.toml}` (+ tests),
`crates/oku-slides-agent/src/{choice.rs,error.rs}`, Fluent keys in all 20 locales.

- [ ] Failing table tests first for `resolve(engine, target, apple, installed)`: every engine × target ×
  Apple state × model state; Unusable variants localized (key + one non-English locale).
- [ ] `ReadyEngine::Apple`; `LocalEngine::Apple`; `local_panel` shows no download when Apple runs.

### Task 12: Apple engine in the local agent

**Files:** `crates/oku-slides-local-agent/src/` new `apple.rs` (engine), `apple_prompt.rs`,
`apple_schema.rs`, `apple_reply.rs` (+ tests), `entry.rs`, ARCHITECTURE.md, AGENTS.md.

- [ ] Failing tests first: `apple_schema::route` and `::act` (titles, `x-order`, `additionalProperties`,
  `$defs` hoisted with unique names, unsupported keywords dropped, enums kept); `apple_prompt::render`
  (history of user, assistant, call and tool-result messages as text, interface locale kept);
  `apple_reply::parse` (call, text, invalid JSON → `Invalid`).
- [ ] `AppleEngine::complete` runs `respond` in `spawn_blocking`, cancelled through the token; context
  overflow is an `EngineError` (the turn ends with its failure report). `build_engine` handles `Apple`.
- [ ] `#[ignore]` manual test: a Route and an Act completion on the real model.

## Lot 6 — instructions, recipes, evaluation (t-1025)

### Task 13: Compact schemas and turn summaries, review follow-ups

**Files:** `crates/oku-slides-local-agent/src/{compact.rs,turn.rs,sessions.rs}` and tests.

- [ ] Failing tests first: the first sentence ends at `.\n` and at a final period, never inside `e.g. `
  or `i.e. `; `report`'s compact description keeps its `facts: 1-4 short lines` hint (compact keeps the
  `name: hint` clauses of required arguments); `exposed` with a
  tool of several families lists it once; a multi-line request gives a one-line turn summary; a `$defs`
  or `definitions` entry named `description` is kept.

### Task 14: `local.md` with fix recipes, the evaluation set, family tuning

**Files:** `crates/oku-slides-local-agent/src/instructions/local.md`, new
`crates/oku-slides-local-agent/eval/requests.json` and `src/eval_tests.rs` (`#[ignore]`), new
`docs/superpowers/plans/2026-11-01-lot-l-local-agent-eval.md`, `families.rs` (+ tests).

- [ ] `local.md` (English, short): read before acting, `diagnose` first for anything that fails, and one
  recipe per availability reason — Node or `npx` missing, login needed, broken preset, model to download,
  Apple Intelligence off — explaining terminal steps to the user, never running them.
- [ ] `requests.json`: 30 requests, French and English, across edit, present, files, configure,
  troubleshoot and packs, each with its expected families and accepted first tools. The ignored test runs
  Route and the first Act step on the engine `local_agent.engine` picks, with a deck tab's tools, and
  prints accuracy per request.
- [ ] Fit the Apple context (**Decision**, measured with `SystemLanguageModel.tokenCount`): one Act reply
  with the `edit` family's schemas is 13,914 tokens and `apply` alone 4,797, over the 4,096 window. The
  Apple Act becomes `apple_steps`: pick a tool by name (~1,060 tokens), pick 1-3 tags of the tool's biggest
  tagged union when its call schema passes 6,000 bytes (214), then the call with that schema alone,
  narrowed and pruned (`apply` ≤ 1,841). Gguf keeps the one-step Act. Numbers in the eval doc.
- [ ] Run it with the Apple engine here (Gguf if its model is present), record results and date in the
  eval doc, and tune `families_of` / `Family::summary` where it misroutes; table tests follow the change.
  2026-11-01, Apple: route 26/30 and first tool 21/30, after tuning 28/30 and 28/30.

### Task 15: Review round and `just ci`

- [ ] Independent review of the whole branch (code reviewer + security reviewer), fixes applied.
- [ ] Review round A fixes (t-1462): local session ids carry a random per-process nonce and a
  `session/load` from another MCP URL is refused (`resource_not_found`); a process-wide secret registry
  (`telemetry::register_secret`: dictation key and secret headers, each tab's MCP token) masks
  `captured` text and the `diagnose` reply; `diagnose` keeps the calling tab's log lines and those of no
  tab, strips control characters and escapes, and calls its text data; `async_openai=off` follows any
  `RUST_LOG`; an interior NUL reaches Apple's bridge as U+FFFD; the API model's `model` value is documented;
  `oku-slides-local-agent/tests/weak_link.rs` checks with `otool -l` that FoundationModels loads as
  `LC_LOAD_WEAK_DYLIB` with `minos 12.0` (a `-weak_framework` link-arg in apple-model's `build.rs` never
  reaches the binaries, so none is set).
- [ ] `just ci` green; `otool -l` check of the weak FoundationModels link.
- [ ] Merge `main` again, compile check, squash to `main` as one commit citing t-1021, t-1023, t-1024,
  t-1025 and t-1009, push, close the sub-tasks, the lots and t-1009.
