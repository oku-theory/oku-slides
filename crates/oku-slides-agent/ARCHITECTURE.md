# oku-slides-agent

The voice agent host: one ACP agent subprocess (stdio) shared by the app, one ACP session
per tab and scope, each with its own MCP tools (local HTTP, bearer token). Dictated or typed
instructions are relayed with the UI context; agent updates become A2UI turn surfaces.

## Modules

- `stack` : `Stack`, the table of tabs and the entry point; `handle` takes an `AgentCommand`
  (`Prompt`, `Tune`, `Cancel`, `NewConversation`, `SurfaceAction`, `Resync`, `Follow`, `SetDraft`,
  `Picker`, `Availability`, `SelectAgent`), `tune` starts an import's fine-tuning turn for the host
  (never deserialized; refused, the tab told why, while `availability` gives a reason: program not
  on `PATH`, else the link's recent failure), `forward` relays `{tab, event}`, `shutdown` kills it.
- `tab_entry` : one tab; starts its tools then its agent on the runtime, queues commands until
  then, and holds the draft (`TabLink`). Closing it closes its `SurfaceHub`.
- `tab_agent` : `AgentHandle`, the per-tab actor (`prompt`, `cancel`, `new_conversation`) with its
  ACP session and `ToolsAccess`; `reopen` resumes a known session (`Reopened`).
- `link` : `Link`, the shared ACP connection: subprocess, handshake, notifications and permission
  requests routed by launch epoch and `SessionId`; `Link::switch` changes agent; `failure` is the
  last launch failure of this epoch (`Unusable`, never `NotFound`), cleared by a success or a
  switch, stale after `FAILURE_TTL` (30 s); `agent_fingerprint` identifies the agent command.
- `keeper` : `Keeper`, the per-tab queue that alone reads and writes the session's
  conversations (`Keep`); the actor only holds live ACP state.
- `events` : `AgentEvent`, `AgentState`, `PlanItem`; `from_update` maps an ACP update to an
  event; every event of a turn carries its `turn` number.
- `turns` : `Turns`, reducer from events to A2UI turn surfaces (`TurnOut`), emitted via the hub.
- `turn`, `permission` (ACP option to `PermissionChoice`), `config` (`Launch`, `READY_WITHIN`,
  from the active agent preset), `settings_port` (`AppSettings` for `set_setting`).
- `picker` : the pure agent-picker surface (`picker_surface`, `PICKER_ID`).
- `instructions` : `instructions/*.md` (English only): agent and widgets on a session's first
  turn, import (bounded fine-tuning loop) only on a `Tune` turn, before the context.
- `error` (`AgentError`), `command_error` (`CommandError`, refused UI commands): localized.
- `bin/fake_agent` : scripted minimal ACP agent (`--script`, `--log`), for tests only.

## Flow

Idle, no agent runs. A tab's first `Prompt` launches the agent (killed if not ready within
`READY_WITHIN`), then opens or resumes the tab's session for its scope (`AgentScope`). The MCP hub
numbers turns (`SurfaceHub::begin_turn`); the actor stamps what it emits, and `link` stamps ACP
updates with the hub's open turn, so the reducer survives `broadcast` lag. A stopped agent fails
every tab's turn; the next prompt relaunches it. Dropping the last `Link` stops the agent.

The agent has no `fs` or `terminal` capability: it acts only through `oku-slides-mcp` tools, and
an agent without MCP over HTTP is refused before any session.

## Dependencies

`agent-client-protocol`, `oku-slides-mcp` (tool server, hub), `oku-slides-session` (tab sessions,
conversations, `KEPT_TURNS`), `oku-slides-a2ui`, `oku-slides-platform` (settings, presets),
`oku-slides-stt`, `oku-slides-i18n`.
