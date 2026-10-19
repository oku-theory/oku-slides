# oku-slides-local-agent

The built-in fallback agent: an ACP agent that every Oku Slides binary becomes when started
with `--local-agent` (see `LOCAL_AGENT_FLAG` in `oku-slides-platform`). It needs no account and
no network, and acts only through the MCP tools of the tab (no `fs`, no `terminal`).

## Modules

- `entry` : `exit_if_local_agent`, first in `main` after `exit_if_worker`; loads `Platform`, then
  `build_engine` asks `local_engine` (`local_agent.engine`) for an `Arc<dyn Engine>` (Gguf, or debug-only
  Fake via `fake_script` / `fake_log`) before the runtime exists. Unavailable engine: `Unavailable`
  (reason); `fake` without script: `NoScript`; bad script: `Script` / `ScriptFormat`. Runtime, then engine, dropped before exit.
- `engine` : `Engine` (`complete(Completion, cancel) -> Reply`), `Step`, `Message` (+ `Call`), `Reply`
  (`Text`, `Call`, `Invalid`), `EngineError`. `gguf` : llama.cpp (Metal) on one thread, Apple silicon.
- `fake` : debug-only `FakeEngine` replaying a JSON script (route text, then replies per turn,
  optional `hang`), logging each call as a JSON line. The only `std::fs` user (test tooling).
- `prompt` : `read_prompt` (last text block = request, `<oku-slides-context>` block, interface
  locale code) and `INSTRUCTIONS` (the agent's own short English prompt, `instructions/local.md`).
- `acp` : `serve`, the ACP server on stdin/stdout: `initialize` (advertises MCP over HTTP only),
  `session/new` (keeps the tab's MCP URL and `Authorization`), `session/prompt` (starts a turn,
  cancelling the previous one, runs `run_turn`, keeps a summary), `session/cancel`. `AcpUpdates`
  sends `ToolCall` / `ToolCallUpdate` / `AgentMessageChunk`, call ids prefixed by turn (`t2-call-1`).
- `sessions` : `Sessions`, one `SessionState` per tab (`local-1`…): MCP access, lazy `McpTools`,
  cancel token renewed by `begin_turn`, last 3 turn summaries.
- `turn` : `run_turn` (route → act ≤ `MAX_CALLS` steps → `report`), `ToolPort`, `Updates`,
  `TurnEnd`, `FailKind`. Unknown, rejected or `Invalid` calls get one repair (`error: …`); a
  transport failure fails the turn (`Engine`), never repaired; a failure ends with a `report` titled
  in the prompt's locale (`local-agent-*` keys), one `local-agent-retry` fact. Cancel: no report.
- `mcp` : `McpTools`, the `ToolPort` over one `rust-mcp-sdk` client per session.
- `tool_error` : `ToolError`: `Rejected` (repairable), `Transport`, `Schema`.
- `families` : `Family` (edit, present, files, templates, configure, troubleshoot, plus the
  always-exposed `AskReport`), `families_of(tool)` (the static table; a test checks that every MCP
  tool has a family), `parse_route` (first two family keys in the reply), `route_grammar` (GBNF
  for the route step).
- `chatml` : `render`, the hand-written Qwen3 ChatML prompt with its `<tools>` block (llama.cpp
  skips the GGUF template). `reply` : `parse`, model text into `Text`, `Call` or `Invalid`.
- `compact` : `ToolSpec`, `compact` (first sentence, schema without descriptions), `exposed`
  (compact tools of the routed families + `AskReport`).
- `error` : `LocalAgentError`, opaque, logs only. `bin/oku-slides-local-agent-dev` : test launcher.

## Flow

Host launches `<app exe> --local-agent` → ACP handshake → one session per tab with its MCP server
→ prompt → engine routes to 1-2 families → acts with only their compact tools → `report`.

## Dependencies

`oku-slides-platform` (flag, telemetry), `oku-slides-i18n` (failure titles), `rust-mcp-sdk`, `agent-client-protocol`, `tokio`, `snafu`, `serde`, `serde_json`, `async-trait`, `tokio-util`, `llama-cpp-2` (Apple silicon);
`oku-slides-mcp` in tests only (tool names).
Stdout is the ACP pipe: logs go to stderr.
