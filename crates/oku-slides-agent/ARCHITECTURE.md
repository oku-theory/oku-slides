# oku-slides-agent

The voice agent host: one ACP agent subprocess (stdio) shared by the app, one ACP session per
tab and scope, each with its own MCP tools (local HTTP, bearer token). Instructions are relayed
with the UI context; agent updates become A2UI turn surfaces.

## Modules

- `stack` : `Stack`, the entry point over `table` (tabs started and closed); `handle` takes an `AgentCommand`; a
  `Prompt` that is a shortcut word (`shortcut`: `matches_shortcut` on a `ui.ftl` key, mirrored by
  the UI) shows the hub's theme picker instead of a turn in deck mode. `tune` starts an
  import's fine-tuning turn for the host (never deserialized; refused, the tab told why, while
  `availability` gives a reason), `forward` relays `{tab, event}`, `shutdown` kills it.
- `choice` : `AgentChoice` (an `Arc`): active agent, `availability` (no program on `PATH`; built-in:
  no app binary, else `local_engine` states; else the link's recent failure), `check`, `switch_to`,
  `persist`, `agents_view`, `local_panel`, `start_download` (each percent bumps the voice revision).
- `model_download` : `ModelDownload`, the cancellable local model download thread.
- `tab_entry` : one tab; starts its tools then its agent on the runtime, queues commands until
  then, and holds the draft (`TabLink`: text and pasted images, `session`). Closing it closes its `SurfaceHub`.
- `tab_agent` : `AgentHandle`, the per-tab actor (ACP session, `ToolsAccess`); `reopen` resumes.
- `paste` : `paste_image` (base64, `[paste] max_draft_images`, `Session::paste_image`), the draft's images, `resolve_images` (ids of a prompt).
- `link` : `Link`, the shared ACP connection: subprocess, handshake, notifications and permission
  requests routed by launch epoch and `SessionId`; `Link::switch` changes agent; `failure`: last
  launch failure of this epoch (`Unusable`), cleared by success or switch, stale after 30 s.
- `keeper` : `Keeper`, the per-tab queue that alone reads and writes conversations (`Keep`).
- `events` : `AgentEvent`, `AgentState`, `PlanItem`; `from_update` maps ACP updates (with `turn`).
- `turns` : `Turns`, reducer from events to A2UI turn surfaces (`TurnOut`), emitted by the hub.
- `turn` (`prompt_images`: "Pasted images" block, image blocks if `SessionCaps.image` and under
  `[paste] prompt_*` limits), `permission` (ACP option to `PermissionChoice`), `config` (`Launch`,
  `READY_WITHIN`, `for_preset`: a built-in preset relaunches this binary with `--local-agent`; `Launch.env`:
  tests only), `settings_port` (`AppSettings`).
- `voice_port` : `AppVoice`, the one writer of dictation and agent choice (`VoicePort`); `agent_switch`
  (`PromptGuard` per prompting tab, deferred switch by epoch); `stt_watch` (400 ms `Stt` fingerprint);
  `voice_refresh` (one panel per revision for every tab's box, `Voice` and `Availability` on change).
- `instructions` : `instructions/*.md` (English): agent, widgets on a first turn; import on `Tune`.
- `error` (`AgentError`, `Unusable`), `command_error` (`CommandError`): localized; `bin/fake_agent`: scripted ACP agent (`--script`, `--log`), tests only.

## Flow

Idle, no agent runs. A tab's first `Prompt` launches the agent (killed if not ready within
`READY_WITHIN`), then opens or resumes the tab's session for its scope (`AgentScope`). The MCP hub
numbers turns (`SurfaceHub::begin_turn`); the actor stamps what it emits, and `link` stamps ACP
updates with the hub's open turn, so the reducer survives `broadcast` lag. A stopped agent fails
every tab's turn; the next prompt relaunches it. Dropping the last `Link` stops the agent. The agent has
no `fs` or `terminal` capability (only `oku-slides-mcp` tools); one without MCP over HTTP is refused.

## Dependencies

`agent-client-protocol`, `oku-slides-mcp` (tool server, hub), `oku-slides-session` (tab sessions, conversations,
`KEPT_TURNS`, the box), `oku-slides-a2ui`, `oku-slides-platform` (settings, presets, model download), `oku-slides-stt`, `oku-slides-i18n`.
