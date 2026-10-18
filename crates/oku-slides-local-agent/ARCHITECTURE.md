# oku-slides-local-agent

The built-in fallback agent: an ACP agent that every Oku Slides binary becomes when started
with `--local-agent` (see `LOCAL_AGENT_FLAG` in `oku-slides-platform`). It needs no account and
no network, and acts only through the MCP tools of the tab (no `fs`, no `terminal`).

## Modules

- `entry` : `exit_if_local_agent`, called first in `main` after `exit_if_worker`; `parse_args`
  reads `--engine`, `--script`, `--log`; `EngineChoice` (`Fake`, compiled in debug builds only);
  `build_engine` turns `Args` into an `Arc<dyn Engine>`. Without a known engine (or a script
  for `fake`) the launch fails with `NoEngine`; an unreadable or invalid script fails with
  `Script` / `ScriptFormat`, keeping its cause (the host shows a failed launch).
- `engine` : the `Engine` trait (`complete(Completion, cancel) -> Reply`), `Step` (`Route`, `Act`),
  `Message`, `Reply` (text or tool call), `EngineError` (`is_cancelled`). Gguf/Apple implement it.
- `fake` : debug-only `FakeEngine` replaying a JSON script (route text, then replies per turn,
  optional `hang`), logging each call as a JSON line. The only `std::fs` user (test tooling).
- `prompt` : `read_prompt` (last text block = request, `<oku-slides-context>` block, interface
  locale code) and `INSTRUCTIONS` (the agent's own short English prompt, `instructions/local.md`).
- `acp` : `serve`, the ACP server on stdin/stdout: `initialize` (advertises MCP over HTTP only),
  `session/new` (keeps the tab's MCP URL and `Authorization`), `session/prompt` (starts a turn,
  cancelling the previous one, runs `run_turn`, keeps a summary), `session/cancel`. `AcpUpdates`
  sends `ToolCall` / `ToolCallUpdate` / `AgentMessageChunk`, call ids prefixed by turn (`t2-call-1`).
- `sessions` : `Sessions`, one `SessionState` per tab (`local-1`, `local-2`…): MCP access, lazy
  `McpTools`, cancel token renewed by `begin_turn`, last 3 turn summaries.
- `turn` : `run_turn` (route → act ≤ `MAX_CALLS` steps → `report`), `ToolPort`, `Updates`,
  `TurnEnd`, `FailKind`. Unknown or failed calls get one repair (`error: …`); a failure ends with a
  `report` titled in the prompt's locale (`local-agent-*` keys) with one `local-agent-retry` fact. Cancel (even during a failing step) ends without a report.
- `mcp` : `McpTools`, the `ToolPort` over one `rust-mcp-sdk` client per session; a result with
  `is_error` is an `Err`.
- `families` : `Family` (edit, present, files, templates, configure, troubleshoot, plus the
  always-exposed `AskReport`), `families_of(tool)` (the static table; a test checks that every MCP
  tool has a family), `parse_route` (first two family keys in the model's reply).
- `compact` : `ToolSpec`, `compact` (first sentence, schema without descriptions), `exposed`
  (compact tools of the routed families plus `AskReport`).
- `error` : `LocalAgentError`, opaque; messages go to the logs only.
- `bin/oku-slides-local-agent-dev` : test launcher that only runs `exit_if_local_agent`.

## Flow

Host launches `<app exe> --local-agent` → ACP handshake → one session per tab with its MCP
server → prompt → engine routes to 1-2 families → engine acts with only their compact tools →
`report` (the model's, or a localized failure report).

## Dependencies

`oku-slides-platform` (flag, telemetry), `oku-slides-i18n` (failure titles), `rust-mcp-sdk`, `agent-client-protocol`, `tokio`, `snafu`, `serde`, `serde_json`, `async-trait`, `tokio-util`;
`oku-slides-mcp` in tests only (tool names).
Stdout is the ACP pipe: logs go to stderr.
