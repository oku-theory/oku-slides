# oku-slides-local-agent

The built-in fallback agent: an ACP agent that every Oku Slides binary becomes when started
with `--local-agent` (see `LOCAL_AGENT_FLAG` in `oku-slides-platform`). It needs no account and
no network, and acts only through the MCP tools of the tab (no `fs`, no `terminal`).

## Modules

- `entry` : `exit_if_local_agent`, first in `main` after `exit_if_worker`; loads `Platform`, then
  `build_engine` asks `local_engine` (`local_agent.engine`) for an `Arc<dyn Engine>` (Apple, Gguf, or debug-only
  Fake via `fake_script` / `fake_log`) before the runtime exists. Unavailable engine: `Unavailable`
  (reason); `fake` without script: `NoScript`; bad script: `Script` / `ScriptFormat`. Runtime, then engine, dropped before exit.
- `engine` : `Engine` (`complete(Completion, cancel) -> Reply`), `Step`, `Message` (+ `Call`), `Reply`
  (`Text`, `Call`, `Invalid`), `EngineError`. `gguf` : llama.cpp (Metal) on one thread, Apple silicon. `apple` :
  Foundation Models, guided replies on `spawn_blocking`; to fit 4,096 tokens an Act is 2-3 replies (`apple_steps`:
  pick a tool by name, pick 1-3 tags of its biggest union when its schema passes `FILL_BYTES`, then its call);
  `apple_schema` (Route and one-tool call schemas in `GenerationSchema` form), `apple_prompt` (history as text), `apple_reply`.
- `fake` : debug-only `FakeEngine` replaying a JSON script (route text, replies per turn, optional `hang`), logging each call as a JSON line. The only `std::fs` user.
- `prompt` : `read_prompt` (last text block = request, `<oku-slides-context>` block, interface
  locale code) and `INSTRUCTIONS` (the agent's own short English prompt, `instructions/local.md`).
- `acp` : `serve`, the ACP server on stdin/stdout: `initialize` (MCP over HTTP only, `load_session`),
  `session/new` (keeps the tab's MCP URL and `Authorization`), `session/load` (same process and MCP URL only,
  else `resource_not_found`), `session/prompt` (cancels the previous turn; a cancel wins over the first MCP connect;
  runs `run_turn`, keeps a one-line summary), `session/cancel`; drops every session when the pipe closes. `AcpUpdates`
  sends `ToolCall` / `ToolCallUpdate` / `AgentMessageChunk`, call ids prefixed by turn (`t2-call-1`).
- `sessions` : `Sessions`, one `SessionState` per tab MCP URL (`local-<nonce>-1`…, nonce random per process; a new or
  loaded session drops the URL's others): MCP access, lazy `McpTools`, cancel token renewed by `begin_turn`, last 3 summaries.
- `turn` : `run_turn` (route → act ≤ `MAX_CALLS` steps → `report`), `ToolPort`, `Updates`, `TurnEnd`, `FailKind`,
  `route_completion` / `act_completion`. Unknown, rejected or `Invalid` calls get one repair (`error: …`); a transport
  failure fails the turn (`Engine`); a failure ends with a `report` titled in the prompt's locale, one `local-agent-retry` fact. Cancel: no report.
- `mcp` : `McpTools`, the `ToolPort` over one `rust-mcp-sdk` client per session. `tool_error` : `ToolError`: `Rejected` (repairable), `Transport`, `Schema`.
- `families` : `Family` (edit, present, files, packs, configure, troubleshoot, plus the always-exposed `AskReport`),
  `families_of(tool)` and `Family::summary` (tuned on the eval set), `offered_families` (tools shared by every kind and
  spanning several families are ignored; `Configure` always), `parse_route`, `route_grammar` (GBNF over the offered keys).
- `chatml` : `render`, the hand-written Qwen3 ChatML prompt with its `<tools>` block (llama.cpp skips the GGUF template). `reply` : `parse`, model text into a `Reply`.
- `compact` : `ToolSpec`, `compact` (first sentence, not at `e.g.`/`i.e.`, plus required-argument hints like
  `report`'s `facts: 1-4 short lines`; schema without descriptions), `exposed` (routed families + `AskReport`, each tool once).
- `error` : `LocalAgentError`, opaque, logs only. `bin/oku-slides-local-agent-dev` : test launcher. `eval/requests.json` +
  `eval_tests`: 30 requests; the ignored test runs Route and the first Act on the configured engine.

## Flow

Host launches `<app exe> --local-agent` → ACP handshake → one session per tab with its MCP server
→ prompt → engine routes to 1-2 offered families (no route step when one or none) → acts with only their compact tools → `report`.

## Dependencies

`oku-slides-platform` (flag, telemetry), `oku-slides-i18n` (failure titles), `oku-slides-apple-model`, `rust-mcp-sdk`, `agent-client-protocol`, `tokio`, `snafu`, `serde`, `serde_json`, `async-trait`, `tokio-util`, `llama-cpp-2` (Apple silicon);
`oku-slides-mcp` and `oku-slides-session` (the shared tool set per `DocumentKind`).
Stdout is the ACP pipe: logs go to stderr.
