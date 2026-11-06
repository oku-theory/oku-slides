# oku-slides-mcp

Serves the agent's MCP tools for one tab (HTTP on `127.0.0.1`, random bearer token per launch)
and holds the tab's A2UI surfaces (`SurfaceHub`): thread, dock, popovers and inspector offers.

## Modules

- `server` : `ToolServer`, the HTTP server (system-chosen port, token compared by SHA-256 in `token_matches`, bounded by `[serve]` limits and `SHUTDOWN_GRACE`).
- `handler` : the `ServerHandler`; `Tools::call` dispatches a call to its tool.
- `tools` : `Tools`, `list_for(kind)`: the tab's kind, read once when its agent starts; an unlisted tool is
  `UnknownTool` (`tool_kinds`: `Kinds`; a theme tab lists no `widgets`, `packs` or `ask_rewrite`); input schemas (`nest_schema`: MCP 1.x); `parse`, `with_session`, `reply`.
- Tool families: `read_tools` (outline, theme_tokens (themes go by namespace), slide, render, `packs`, `layouts`, widgets,
  settings), `edit_tools` (undoable edits, `add_pack`/`remove_pack`, `set_theme_extends`, `focus`: shows a slide, never selects; no slide deselects), `open_tools` (`open_pack`, `open_theme`: one tab move per turn, a deck-only pack's tab linked to its deck), `tab_tools` (pack tab: `select_item`, `set_preview_theme`; theme tab: `rename_theme`, `save_theme`; both: `set_guide`, an agent write that is never echoed back and that the turn's receipt lists, `note_guide_write`), `widget_tools` (pack tab: `duplicate_widget`, `reorder_widgets`; no `pack`: `deck`, so
  `deck-layouts` is named; writes refused in a turn that moved to another tab), `guarded_tools` (confirmed: `save`, `save_copy`, `open`, `move`, `merge`, `new`,
  `save_pack` (a copy), `delete_pack`, `edit_theme`, always confirmed, `set_setting`…), `app_tools` (`history`, its list paged by `history_page`; opening it,
  `present` (`PresentCommand`: start, or a step while the hub presents, `set_presenting` from page reports) and `set_interface` emit an event the shown tab applies), `a2ui_tools` (`a2ui_examples`: the playbook's agent reference, cached), `surface_tools` (`report`, `render`, `ask_*`,
  `show_*`; `show_themes`/`show_packs` open the hub's own boxes, filed in `agent_turn`), `import_tools` (`import`: a `.pptx` in a new tab through `AgentWorkspace`, confirmed, the turn's one tab move, `tune` through `TabPort`; `import_report`, `import_diff`: import
  tuning; `ReferenceJob` in a blocking task), `voice_tools` (eight tools, checked before asking, no API target argument; `set_speech` unconfirmed), `diagnose_tools` (`diagnose`, every kind: `VoicePort::diagnosis`, the log ring (the tab's lines and those of no tab), the tab's last errors; capped, secrets and home redacted once more, escapes and controls stripped; `ToolServer` registers its token as a secret), `undo_tools` (`undo`, `redo`: up to 20 steps, stopping at the first refusal; `undo_turn`, never the call's own turn; none confirmed, none in help tabs), `usage_tools` (`usage`, `show_usage`: the linked deck's ledger as JSON capped at 1000 rows, a thread box capped at 100; not in help tabs), `host_tools` (`update` through `AppPort`, family Configure: an install is confirmed, then recorded as an `AfterTurn`; `pack_update`, family Packs: the packs panel's own event, then `prune`), `workspace_tools` (`tabs`: the window's tabs, titles labelled; `tab`, `window`: through `AgentWorkspace`, a turn's one move, unused arguments refused; a close, only from the shown tab, is confirmed, stored as a decision and sent, or for its own tab or window `AfterTurn::CloseOwn`; `help`: the help tab at its query's first hit, `HELP_HITS` titles labelled, a reused tab's pages sent it (`TabPort::page`), no move claimed from the help tab; `new_conversation`, `quit`: recorded `AfterTurn`s, a quit confirmed after `AppPort::can_quit` and refused while tabs are unsaved).
- `tool_error` : `ToolError`, `say_to_agent`; `Tools::fail_with` logs it whole and tells the agent a localized message in
  the current interface language (`LiveLocalizer`, also read by the hub). `error` : `McpError`, `say_session`; `paths` (agent-given paths); `lexicon` (`Lexicon`, `ConfirmAsk`: a `ConfirmVerb` and its button label, or a plain `ConfirmAction` read aloud; spoken command words matched whole, forms from i18n `voice.ftl`, the theme shortcut's from `ui-shortcut-theme`; `lexicon_token`: tokens, segmentation).
- Ports: `app` (`AppPort` in `Tools`, from session); `settings` (`SettingsPort` in `Tools`, `Applied`, whitelist without `dictation.*`); `voice` (`VoicePort`: view, check, apply, `diagnosis` (`diagnosis.rs`), in the hub, which builds the box);
  `draft` (`open_*`'s `request`, cleaned, at most `MAX_REQUEST_BYTES`, to a moved tab's draft); `tab_port` (`TabPort` in `Tools`: prefill, page, tune; `Prefill`, `TuneRefusal`; `HostPorts` groups the four host ports `Tools::new` takes).
- `hub` : `SurfaceHub`, open surfaces and events to the UI (`McpEvent`); `emit` alone changes
  the open list; `resync` resends it whole; `McpEvent::Surface.spoken` (host-only, `surface_aloud`: receipts, `ask_aloud` questions, a guard's template and verb or action, a permission's template) feeds the read-aloud. `hub/asks` (questions, confirmations, permissions, each with its kind and arming; a typed reply never answers a guard; a turn's received-guide note shows in each, `set_permission_note`), `hub/voice_answer` (`answer_voice`: a routed utterance, `ARMING`, moved to a question's speech end by `spoken_until`, `AnswerOutcome`; decides under the lock, acts after),
  `hub/actions` (`SurfaceAction`, `act`, `prune`); the hub builds two boxes itself: `hub/themes`
  (`show_themes`, cached cards) and `hub/packs` (`show_packs`, re-sent by `prune` only when changed); `hub/error`;
  `hub/settings` a third box (`show_settings`, one send path, refreshed by the stack, never by `prune`); `hub/close` (`CloseDecision`s: stored pending, sent live as `McpEvent::Close`, sent ones evicted first past `KEPT`, never replayed, taken once by their window within `CLOSE_DECISION_TTL`).
- `hub_turns` : thread turns (`begin_turn`, `close_turn`, `show_static`, `note_turn_traces` for the fallback receipt, `STATIC_PER_TURN`, `reserve_tab`: one pack, theme, import or tab move per turn; `tab_gate`: its verdict; `after_turn`: what runs once the turn ended, one quit, install or own close at most, a new conversation too; `after_turn_notice`; `turn_ended`: waits for a turn's end and receipt), freezing answered questions, eviction past `KEPT_TURNS`; it keeps each turn's `TurnOrigin` (`begin_turn_from`). `heard` : `HeardUtterance`, a closed utterance for an answer.
- `hub_thread` : the tab's one thread (`in_thread`), shown surfaces, `restore` and `thread_snapshot`; the agent's tab keeper persists it.

## Turn of a tool call

A thread surface goes in the turn open **when its call arrived**, not when it shows: `Tools::call`
reads `SurfaceHub::turn` first and passes it on (`show_static`, `show_report`, `ask_then`,
`show_params`). A late call from a closed turn is refused. The server stamps calls itself
because the agent does not know turn numbers; ACP events carry theirs (`oku-slides-agent`).

## Extension points

- Ports live with who implements them. Host ports (`AppPort`, `AgentWorkspace`, `Sessions`) are in
  `oku-slides-session`, since `src-tauri` does not depend on mcp; stack ports (`SettingsPort`,
  `VoicePort`, `TabPort`) are here, implemented by `oku-slides-agent`.
- New tool: an entry `(Tool, Kinds)` in `list_all`, an arm in `Tools::call`, a family in the local agent's `families_of`; its errors are `ToolError` variants with a Fluent key.

## Dependencies

`oku-slides-session` (the tab's `TabSession`, agent surfaces), `oku-slides-core`, `oku-slides-a2ui`,
`oku-slides-chart`, `oku-slides-i18n`, `oku-slides-platform` (listener),
`rust-mcp-sdk` and `rust-mcp-axum` 1.x (protocol `2025-11-25`); a 2.x migration stays local to
`server.rs`, `handler.rs` and `tools.rs`.
