# oku-slides-agent

The voice agent host: one ACP agent subprocess (stdio) shared by the app, one ACP session per
tab, each with its own MCP tools (local HTTP, bearer token). Instructions are relayed
with the UI context; agent updates become A2UI turn surfaces.

## Modules

- `stack` : `Stack`, the entry point over `table` (tabs started and closed), started with `Voices` (dictation, read-aloud `Speaker`); `handle` takes an `AgentCommand`; a
  typed `Prompt` that is a shortcut word (`shortcut`: `is_spoken_theme_shortcut`, interface then dictation
  language, mirrored by the UI) shows the hub's theme picker instead of a turn in deck mode. `tune` starts an
  import's fine-tuning turn for the host (never deserialized; refused, the tab told why, only while neither
  the selected nor the local agent can run, `AppVoice::tune_refusal`), `forward` relays `{tab, event}`, `shutdown` kills it.
- `choice` : `AgentChoice` (an `Arc`): active agent and its `Fallback` (`choice_fallback`: `fall_back`/`back_to_main`, the link runs the first built-in preset, `agent.selected` untouched; the selected agent's failure kept 30 s, doubled by each failed retry up to 10 min, reset by `main_served` or a choice; `diagnosis` tells its reason and stderr tail kept at the switch),
  `main_reason`/`availability` (no program on `Platform.program_path`; built-in: no app binary, else `local_engine` states; else the link's recent failure, or the one kept at the fallback), `local_check`, `check`, `switch_to` (the selected agent chosen again retries it; the running fallback preset is kept), `persist`,
  `check_model`/`choose_model` (per preset), `agents_view`, `local_panel`, `start_download` (`model_download`).
- `tab_entry` : one tab; `Agent::run` reads its kind, builds that kind's tools then starts its agent, queueing
  commands until then; holds the draft (`TabLink`: text, pasted images, `session`; `prefill_draft`). Closing it closes its `SurfaceHub`.
- `drafts_port` : `Starter` starts a tab's entry (`Stack::open`), prefills a tab's empty draft (never its agent) and pages it. `tab_port` : `StackTabs`, the `TabPort` (weak `Stack`): `Stack::prefill`, `Stack::page`, and `Stack::tune` for `import` with `tune`.
- `tab_agent` : `AgentHandle`, the per-tab actor (ACP session, `ToolsAccess`); `reopen` resumes; a turn's end hands its traces to the hub before `TurnEnded`; `fallback_turn`: routes before `connect` (notice line, `AgentEvent::Fallback`, guarded model download with progress, one re-route after a launch failure). `keeper` : `Keeper`, the per-tab queue that alone reads and writes the conversation (`Keep`); `lookup` by agent fingerprint: the session another agent took over waits aside, in memory.
- `voice_router` : a thread (weak `Stack`) routes each `Stt::utterances` item (dropped if begun while the speaker spoke, `Speaker::hears`) with `route_voice`: stop, wake phrase, focus gate (`AgentWorkspace`: the listening tab, focused or left), shortcut (`Lexicon::theme_shortcut`, the picker shown at once; not presenting), open question or drawer (`answer_voice`, a prompt only on `NotAQuestion`), presentation (a nav word to the page at once, else dropped), else a `Voice`/`Wake` prompt (`TurnOrigin`); logs seq only. `after_turn` : the actor drains a finished turn's `AfterTurn` after its reply, after its receipt when it recorded any (`drain_after_reply`, a new conversation joining `Pending`; a stop or failure drops them, an own close's decision too); a thread (weak `Stack`, `catch_unwind`) runs quit (unsaved tabs checked again) and install through `AppPort`, and sends an own close's decision only while its tab is shown and its dirty revisions unchanged (`close_own`), a refusal as a notice; `Stack::take_close_decision` redeems one for the page's window. `page_reports` : `PageReports`, each tab's `Context`/`Presenting` ordered by (page, seq), only from its window (`handle_from`, `ReportFrom::of`, the holder check of the app and serve; `stack_reports`, which copies presenting to the tab's hub). `agent_command` : `AgentCommand`.
- `paste` : `paste_image` (base64, `[paste] max_draft_images`, `Session::paste_image`), the draft's images, `resolve_images` (ids of a prompt).
- `link` : `Link`, the shared ACP connection: subprocess, handshake, notifications and permission requests routed
  by epoch and `SessionId`; `Link::switch` changes agent, its preset id and wanted model; `failure`: last launch failure of this epoch (`Unusable`),
  cleared by success or switch, stale after 30 s. `stderr_tail`: the child's stderr piped in 4 KiB chunks to tokio's stderr, its last 40 lines (`\n` or `\r`, first 2,000 bytes) kept per launch (`Link::stderr_tail`, for `diagnose`). `wire`: its stdio, logged on `ACP_TARGET` (`OKU_SLIDES_DEBUG_ACP`).
  `session_model`: `ModelBoard`, sessions' `model` options; the actor asks the chosen one before a prompt if offered; `usage_board`: `UsageBoard`, each session's cumulative cost (replays and unknown sessions ignored, a reopened session's first one only the base), `record_turn` files the turn's `UsageRecord` in its deck (`TabSession::record_usage`), `tells` guards session-state updates.
- `events` : `AgentEvent`, `AgentState`, `PlanItem`; `from_update` maps ACP updates (with `turn`); `turns` : `Turns`, reducer to A2UI turn surfaces (`TurnOut`).
- `turn` (puts the session's guides block, when `agent_guides` returns one, before the context block; returns the reply's `Usage`; `prompt_images`: "Pasted images" block, image blocks if `SessionCaps.image` and under
  `[paste] prompt_*` limits), `permission` (ACP option to `PermissionChoice`), `config` (`Launch`, its `path`
  the child's `PATH` and lookup, `READY_WITHIN`, `for_preset`: a built-in preset relaunches this binary with `--local-agent`; `Launch.env`: tests only), `settings_port` (`AppSettings`).
- `voice_port` : `AppVoice`, the one writer of dictation and agent choice (`VoicePort`; `diagnosis`: `AgentChoice::diagnosis`, the dictation's microphone, engine, files, language); `fallback`: `turn_route` (`route_read` off the lock, then `route_apply` under it: `TurnRoute` by the pure `decide`, `Main` if another agent was chosen since, the link switched only while the tab prompts alone); `agent_switch`
  (`PromptGuard` per prompting tab, deferred switch by epoch); `stt_watch` (400 ms `Stt` fingerprint);
  `voice_refresh` (one panel per revision for every tab's box, `Voice` and `Availability` on change, `Fallback` cleared once it ends); `voice_port_hands_free` (`apply_patch`: listening follows the switch; the agent only turns it off; the first on turns the read-aloud on); `voice_port_speech` (`SpeechView`, `set_speech` checks, `Stt::configure_speech`); `reader` : a thread (`catch_unwind`) that says, for the focused tab only, the `SpokenText` its relay hands it (`heard_of`), muted while presenting unless `in_presentation` and a `Wake` turn; a new turn cuts, a question's end moves its arming (`spoken_until`).
- `instructions` : `instructions/*.md` (English): a shared core and one paragraph per tab kind (`for_kind`), then widgets (not in a theme tab), on a first turn; import on `Tune`.
- `error` (`AgentError`, `Unusable`, `Reason`), `command_error` (`CommandError`): localized; `bin/fake_agent`: scripted ACP agent, tests only (`by_prompt`, `by_prefix` with `${ARG}`).

## Flow

Idle, no agent runs. A tab's first `Prompt` launches the agent (killed if not ready within
`READY_WITHIN`), then opens or resumes the tab's session (one thread per tab). The MCP hub
numbers turns (`SurfaceHub::begin_turn`); the actor stamps what it emits, and `link` stamps ACP
updates with the hub's open turn, so the reducer survives `broadcast` lag. A stopped agent fails
every tab's turn; the next prompt relaunches it. Dropping the last `Link` stops the agent. The process runs in
`<data>/agent-process` (`Launch.process_dir`, emptied by the `Link`'s storage at each launch), each session in its conversation's `work/<uuid>` (`Session::agent_work_dir`, the uuid saved with the conversation;
none: the turn fails; `AgentWorkDir::prepare` strips agent config names out of the lock before every new, resumed or loaded session; the keeper removes it for a new conversation), never the deck's folder. No `fs` or `terminal` capability (only `oku-slides-mcp` tools); one without MCP over HTTP is refused.

## Dependencies

`agent-client-protocol`, `oku-slides-mcp` (tool server, hub), `oku-slides-session` (tab sessions, conversations,
`KEPT_TURNS`, the box), `oku-slides-platform` (settings, presets, model download), `oku-slides-stt`, `oku-slides-i18n`.
