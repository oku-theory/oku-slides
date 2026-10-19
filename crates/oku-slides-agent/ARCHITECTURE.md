# oku-slides-agent

The voice agent host: one ACP agent subprocess (stdio) shared by the app, one ACP session per
tab and scope, each with its own MCP tools (local HTTP, bearer token). Instructions are relayed
with the UI context; agent updates become A2UI turn surfaces.

## Modules

- `stack` : `Stack`, the table of tabs and the entry point; `handle` takes an `AgentCommand`; a
  `Prompt` that is a shortcut word (`shortcut`: `matches_shortcut` on a `ui.ftl` key, mirrored by
  the UI) shows the hub's theme picker instead of a turn in deck mode. `tune` starts an
  import's fine-tuning turn for the host (never deserialized; refused, the tab told why, while
  `availability` gives a reason), `forward` relays `{tab, event}`, `shutdown` kills it.
- `choice` : `AgentChoice` (an `Arc`): active agent, `availability` (no program on `PATH`; built-in:
  no app binary, else `local_engine` states; else the link's recent failure), `picker`, `select`,
  `local_panel`, `start_download` (picker and availability re-emitted at each percent and the end).
- `model_download` : `ModelDownload`, the cancellable local model download thread; `failure_text`.
- `tab_entry` : one tab; starts its tools then its agent on the runtime, queues commands until
  then, and holds the draft (`TabLink`). Closing it closes its `SurfaceHub`.
- `tab_agent` : `AgentHandle`, the per-tab actor (ACP session, `ToolsAccess`); `reopen` resumes.
- `link` : `Link`, the shared ACP connection: subprocess, handshake, notifications and permission
  requests routed by launch epoch and `SessionId`; `Link::switch` changes agent; `failure` is the
  last launch failure of this epoch (`Unusable`, never `NotFound`), cleared by a success or a
  switch, stale after `FAILURE_TTL` (30 s); `agent_fingerprint` identifies the agent command.
- `keeper` : `Keeper`, the per-tab queue that alone reads and writes conversations (`Keep`).
- `events` : `AgentEvent`, `AgentState`, `PlanItem`; `from_update` maps ACP updates (with `turn`).
- `turns` : `Turns`, reducer from events to A2UI turn surfaces (`TurnOut`), emitted by the hub.
- `turn`, `permission` (ACP option to `PermissionChoice`), `config` (`Launch`, `READY_WITHIN`,
  `Launch::for_preset`: a built-in preset relaunches this binary with `--local-agent`; `Launch.env`
  adds variables, tests only), `settings_port` (`AppSettings`), `picker` (`picker_surface`; its
  `LocalPanel`: reason and Download, progress and Cancel, or the failure).
- `instructions` : `instructions/*.md` (English): agent, widgets on a first turn; import on `Tune`.
- `error` (`AgentError`, `Unusable`), `command_error` (`CommandError`): localized.
  `bin/fake_agent`: scripted ACP agent (`--script`, `--log`), tests only.

## Flow

Idle, no agent runs. A tab's first `Prompt` launches the agent (killed if not ready within
`READY_WITHIN`), then opens or resumes the tab's session for its scope (`AgentScope`). The MCP hub
numbers turns (`SurfaceHub::begin_turn`); the actor stamps what it emits, and `link` stamps ACP
updates with the hub's open turn, so the reducer survives `broadcast` lag. A stopped agent fails
every tab's turn; the next prompt relaunches it. Dropping the last `Link` stops the agent.
The agent has no `fs` or `terminal` capability: it acts only through `oku-slides-mcp` tools;
an agent without MCP over HTTP is refused before any session.

## Dependencies

`agent-client-protocol`, `oku-slides-mcp` (tool server, hub), `oku-slides-session` (tab sessions,
conversations, `KEPT_TURNS`), `oku-slides-a2ui`, `oku-slides-platform` (settings, presets, model
download), `oku-slides-stt`, `oku-slides-i18n`.
