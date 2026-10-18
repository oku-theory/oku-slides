# Local fallback agent — design

Tasks: t-1009 (this spec), decision t-1006; lots t-1020 to t-1025; remote dictation t-1026. Date: 2026-10-18.
Status: design approved by the user on 2026-10-18; nothing is implemented.
Related: `2026-10-18-pilotage-sans-mains-design.md` (hands-free control, lots 1 to 8, t-988 and
t-996 to t-1002).

## Problem

Since t-988 the main menu only holds Present, Open and Help: everything else goes through the
voice agent. The user decided (t-1006) that the menu entries do not come back when no agent is
usable. Instead, the app offers a simple agent that runs on the machine. Today an ACP agent is
"unusable" when its program is not on `PATH` (`npx` without Node) or its last launch failed
(`Stack::availability`, `Link::failure`): a crash, no MCP over HTTP, offline, or not logged in.
In those cases the app currently can only be edited with the keyboard and mouse, and cannot be
configured or repaired at all.

## Goals

- When the chosen ACP agent is unusable, a local agent takes over without the user leaving voice
  control. It works offline, needs no account and sends nothing off the machine.
- The local agent covers the same ground as any agent: **editing**, **configuring the app** and
  **troubleshooting it**. Troubleshooting covers why the main agent fails, dictation and the
  microphone, recent app errors, and render or import failures.
- It is an ordinary agent preset, also selectable on purpose (privacy, offline work).
- It follows every rule other agents follow: tools only through the tab's MCP server, guarded
  calls confirmed by the user, receipts through `report`.

Non-goals: matching a frontier model's quality on open-ended writing; a cloud fallback; running
on Windows or Linux in this spec (the GGUF engine is portable, the offer and packaging are macOS
first); remote dictation engines (separate task, see "Out of scope").

## Approaches considered

- **Built-in ACP subprocess (chosen).** A new crate implements the agent side of ACP. The app
  re-executes itself with a `--local-agent` argument, as it does with `--render-worker`, and `stack` and
  `link` see an ordinary stdio agent that receives the tab's MCP server over HTTP. A crash or an
  out-of-memory abort in the engine stays out of the app process, which matters for a 2.5 GB
  model.
- **In-process agent behind `Link`.** It saves a process, but it breaks the crate invariant
  "the agent acts only through `oku-slides-mcp` tools, as a subprocess", and a llama.cpp abort
  would take the app down.
- **No model, only a command grammar.** It is predictable and instant, but it cannot
  troubleshoot or handle open-ended configuration, which the user requires.

## Design

### Engines

An `Engine` trait (`Engine::complete`: a chat completion with tool calls that returns a whole
reply, not a stream, and takes a `CancellationToken` that real engines check per token) with two implementations,
mirroring the dictation split between Apple's engine and downloaded Whisper models:

- **Apple** — Foundation Models (`LanguageModelSession` with `Tool` calling), on macOS 26+ with
  Apple Intelligence enabled. Nothing to download. It is reached through a new `OkuModel` target
  in a Swift package next to `OkuSpeech`, with the same static C ABI and the same build
  requirement (Xcode with the macOS 26 SDK; the app still runs from macOS 12). Its context is
  about 4,096 tokens, which drives the routing below.
- **Gguf** — llama.cpp through the `llama-cpp-2` crate, with Metal, on Apple silicon. The model
  is **Qwen3-4B-Instruct (2507), Q4_K_M, about 2.5 GB**, with a 16K-token context. It is
  downloaded on demand from Hugging Face into `models/agent/` under the data dir, with the
  same progress, cancel and `.complete` marker as Whisper models (`oku-slides-stt` `models.rs`).
  The repository revision and the file's SHA-256 are pinned in code, and a mismatch deletes the
  file: downloaded bytes are untrusted.
- Setting `local_agent.engine = "auto" | "apple" | "gguf"` (default `auto`): `auto` takes Apple
  when it reports itself available, else Gguf. A forced engine that is unavailable reports why;
  nothing falls back silently. On an Intel Mac Gguf is reported unsupported (no Metal), and the
  local agent is unavailable unless Apple is.
- A third implementation, `fake` (scripted replies), exists for tests and e2e only, selected by
  `OKU_SLIDES_LOCAL_AGENT__ENGINE=fake` in test builds.
  **Lot 1 deviation:** the engine is chosen by the `--engine <name> --script <file> --log <file>`
  arguments of the `--local-agent` process (`fake`, debug builds only), not by that variable.
  Lots 3 and 5 choose the engine through the `local_agent.engine` setting read by the child
  process, and tests inject `fake` the same way.

The plan audits `llama-cpp-2` (build on macOS 12, licence, binary size) before adding it to
`workspace.dependencies`.

### Process and ACP

- New crate `oku-slides-local-agent`: the ACP agent loop (initialize, new / load session,
  prompt, cancel), the MCP client towards the URL and bearer token given in `mcp_servers`, the
  engines, and the routing. It never reads files or runs commands. It holds no deck state: each
  turn it reads what it needs through tools (`deck_outline`, `slide`, `settings`, `diagnose`).
- `exit_if_local_agent()` sits next to `exit_if_worker()` as one of the first lines of `main` in
  every app binary (the app and the CLI for `oku-slides serve`). When the process has the
  `--local-agent` argument it runs the agent on stdin/stdout and exits.
- The host launches `current_exe()` with that argument, so no extra binary ships and the
  preset needs no path. It is an argument, not an env var: the config crate reads every
  `OKU_SLIDES_<SECTION>` variable, so `OKU_SLIDES_LOCAL_AGENT` would collide with the
  `[local_agent]` section (`local_agent.engine`). It also matches `--render-worker`.
- Per-turn context: the system instructions (`instructions/local.md`, English, short), the UI
  context the host already sends, and a summary of the last 3 turns of the session. The full
  thread is not replayed, so the small context is not exhausted.
- Cancel: an ACP cancel stops generation at the next token and drops pending tool calls.

### Fitting the tools into a small context: routing per turn

About 40 MCP tools exist today and lots 1 to 6 of the hands-free spec add more. Their schemas do
not fit in Apple's window and would crowd Gguf's. Each turn has two steps:

1. **Route.** The model sees the request, the UI context and one line per **tool family**, and
   answers with one or two family names (grammar-constrained on Gguf, a `@Generable` enum on
   Apple). Families: *edit* (apply, focus, chart and table tools, show_*), *present and
   navigate*, *files and history* (open, new, save, move, merge, history, import), *templates and
   themes* (edit_templates, widget tools, edit_theme, theme_tokens), *configure* (settings,
   set_setting, set_interface, agent choice, model download), *troubleshoot* (diagnose, render,
   import_report, import_diff, settings). *ask and report* (ask_*, report) is always included.
2. **Act.** Only those families' tools are exposed, with compacted schemas (descriptions trimmed
   to their first sentence, enums and required fields kept). The tool loop is capped at 8 calls,
   then the turn ends with a `report`.

The family table is static in the crate. A test fails when a tool from `oku-slides-mcp`
`tools::list` belongs to no family, so a new tool cannot be forgotten. The compacted schemas are
derived from `tools::list` at startup, never copied.

### Tool coverage

- **Editing, presenting, files, templates:** every existing MCP tool, through its family.
- **Configuring:** `set_setting` (its whitelist unchanged), `set_interface` (lot 1, t-988), and
  `select_agent` and `download_model` from hands-free lot 6 (t-1000). Lots 1 and 6 are
  dependencies of the full coverage, not of the first local-agent lots.
- **Troubleshooting:** a new read-only MCP tool **`diagnose`**, offered to every agent, not only
  the local one. It returns:
  - the selected ACP agent, its availability reason and the tail of its launch stderr (last 40
    lines);
  - dictation status: microphone permission, engine, model state (missing, partial, complete),
    language;
  - the last 50 warnings and errors from the app log, with paths under the home directory shown
    as `~` and nothing above the WARN level;
  - the last render error and the last import error of the tab.

  `instructions/local.md` holds short fix recipes for each availability reason: Node or `npx`
  missing, login needed, broken preset. Fixes that need a terminal are explained to the user,
  never run.

### Preset and offer

- `[agent.presets.local] name = "Local"` with `builtin = "local"`. `AgentPreset` gains
  `builtin: Option<Builtin>`; a preset has either `command` and `args` or `builtin`, checked at
  settings load. The picker lists it like the others.
- The `[agent.presets.local]` entry lands in `config.default.toml` with the first real engine
  (lot 3, t-1022): a preset whose only engine is the debug `fake` would be a half feature.
- `Stack::availability` for the built-in preset reports the engine state: ready, needs a download
  (with its size), or unsupported (reason: macOS too old for Apple and Intel for Gguf, or Apple
  Intelligence off). These reasons are new localized `Unusable` variants.
- **Offer when the selected agent is unusable** (decided by the user: auto-switch, ask only for
  a download). At a prompt, or at an import's tuning turn:
  - local agent ready: the turn runs on it at once. The selected preset is **not** overwritten.
    The thread header shows "Local agent" and the reason the main agent failed (an A2UI line in
    the turn, so read-aloud of lot 8 reads it).
  - download needed: a guarded confirmation states the model name and size (answerable by voice
    once hands-free lot 3 lands). On yes, download progress shows in the thread, and the turn runs
    when it completes; on no, the turn fails with the main agent's reason, as today.
  - unsupported: the turn fails with both reasons (main agent and local agent).
- On each new turn while running on the local agent, the host re-checks the selected agent's
  availability and goes back to it as soon as it is usable. The local conversation stays in the
  thread history; the main agent starts its own session.
- `useAgentAvailability` and `AgentPanel` show "Local agent (fallback)" while a turn runs on it.
  There is no new menu entry.

### Guarantees and errors

- The local agent never confirms its own guarded calls: the hub rules are unchanged (and lot 3's
  voice lexicon is matched by the host, not by any agent).
- An invalid tool call (unknown tool, schema mismatch) is sent back once with the MCP error for
  a repair. A second failure ends the turn with a localized failure and a `report`.
- Engine errors are Snafu with a backtrace and Fluent keys (`local-agent-*`), surfaced like
  `AgentError` today. An engine abort ends the subprocess; `link` reports it like any agent
  crash, and the next prompt relaunches it.
- The model is loaded once per subprocess and kept between turns. The subprocess exits with the
  last tab, like other agents.

### Testing

- Unit tests with the `fake` engine: routing picks families, the tool loop cap, repair retry,
  cancel. Each test has one behaviour, in sibling `_tests.rs` files.
- Family completeness test against `tools::list`.
- An ACP round-trip test: `Stack` with the built-in preset and the fake engine against a real
  `ToolServer`, ending in a `report`. **Lot 1 deviation:** the round trip builds its `Launch` by
  hand (arguments above); the `Stack`-level test with the built-in preset belongs to lot 5.
- `Stack::availability` for each engine state, keyed on the variant, plus a non-English locale.
- `diagnose` tests: redaction of home paths, the WARN floor and the caps.
- e2e (`ui/e2e`): the selected agent's program is missing, the turn runs on the local agent
  (fake engine), and the header shows the fallback.
- A manual evaluation set (`docs/` next to the plan, about 30 French and English requests across
  edit, configure and troubleshoot) measures tool-choice accuracy per engine before each release
  that changes the model or the instructions. It does not run in CI.

## Lots

| Lot | Task | Content | Depends on |
|---|---|---|---|
| 1 | t-1020 | `builtin` preset, `exit_if_local_agent`, ACP agent loop, MCP client, routing and tool loop on the `fake` engine | — |
| 2 | t-1021 | `diagnose` MCP tool for every agent | — |
| 3 | t-1022 | Gguf engine (llama.cpp, Metal), pinned model download, availability states | 1 |
| 4 | t-1023 | Apple Foundation Models engine through the Swift bridge, `local_agent.engine` | 1 |
| 5 | t-1024 | Offer: auto-switch at a prompt or tuning turn, download confirmation, return to the main agent, UI fallback badge | 1, 3 or 4 |
| 6 | t-1025 | `instructions/local.md` with fix recipes, the manual evaluation set, tuning of the family table | 2, 3, 4 |

## Out of scope

- **Remote dictation** (asked by the user on 2026-10-18): connecting dictation to a remote
  transcription service such as OpenAI `gpt-4o-transcribe` or the Whisper API (t-1026). It belongs to
  `oku-slides-stt`, not to the agent, and gets its own task and spec. It conflicts with the
  hands-free principle "continuous listening stays on-device (local STT engines only)". That
  spec must decide whether a remote engine is allowed in push-to-talk only, or whether the
  principle changes.

## Open questions

- Whether the local agent should also take over when the main agent is usable but slow or
  rate-limited. Not now: only unusable triggers the fallback.
- A smaller Gguf model (about 1.7B) for 8 GB Macs, if the evaluation set shows Qwen3-4B swaps
  there.
