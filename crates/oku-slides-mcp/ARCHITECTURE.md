# oku-slides-mcp

Serves the agent's MCP tools for one tab (HTTP on `127.0.0.1`, random bearer token per launch)
and holds the tab's A2UI surfaces (`SurfaceHub`) shown in the agent thread and dock.

## Modules

- `server` : `ToolServer`, the HTTP server (system-chosen port, token compared by SHA-256 in
  `token_matches`, bounded by `[serve]` limits and `SHUTDOWN_GRACE`).
- `handler` : the `ServerHandler`; `Tools::call` dispatches a call to its tool.
- `tools` : `Tools`, the tool list (`list`) and input schemas (`nest_schema` adapts JSON Schema
  to MCP 1.x); helpers `parse`, `with_session`, `json_result`, `reply`.
- Tool families: `read_tools` (outline, theme_tokens (themes go by namespace), slide, render, `packs`, `layouts`, widgets,
  settings), `edit_tools` (undoable edits, `edit_pack`: a pack tab per turn, a deck-only pack's tab linked to its deck, `add_pack`/`remove_pack`, `set_theme_extends`), `widget_tools` (no `pack`: `deck`, so
  `deck-layouts` is named; writes refused in a turn that opened a pack tab), `guarded_tools` (confirmed: `save`, `save_copy`, `open`, `move`, `merge`, `new`,
  `save_pack`, `delete_pack`, `edit_theme`, always confirmed, `set_setting`…), `app_tools` (`history`; opening it,
  `present` and `set_interface` emit an event the shown tab applies), `surface_tools` (`report`, `render`, `ask_*`,
  `show_*`; `show_themes`/`show_packs` open the hub's own boxes), `import_tools` (`import_report`, `import_diff`: import
  tuning; `ReferenceJob` in a blocking task), `voice_tools` (seven tools, checked before asking, no API target argument).
- `tool_error` : `ToolError`, `say_to_agent`; `Tools::fail_with` logs it whole and tells the agent a localized message in
  the current interface language (`LiveLocalizer`, also read by the hub). `error` : `McpError`, `say_session`; `paths` (agent-given paths).
- Ports: `settings` (`SettingsPort` in `Tools`, `Applied`, whitelist without `dictation.*`); `voice` (`VoicePort`: view, check, apply, in the hub, which builds the box).
- `hub` : `SurfaceHub`, open surfaces and events to the UI (`McpEvent`); `emit` alone changes
  the open list; `resync` resends it whole. `hub/asks` (questions, confirmations, permissions),
  `hub/actions` (`SurfaceAction`, `act`, `prune`); the hub builds two boxes itself: `hub/themes`
  (`show_themes`, cached cards) and `hub/packs` (`show_packs`, re-sent by `prune` only when changed); `hub/error`;
  `hub/settings` a third box (`show_settings`, one send path, refreshed by the stack, never by `prune`).
- `hub_turns` : thread turns (`begin_turn`, `close_turn`, `show_static`, `STATIC_PER_TURN`, `reserve_pack_tab`: one new pack tab per turn),
  freezing answered questions, eviction past `KEPT_TURNS` in the tab's thread.
- `hub_thread` : the tab's one thread (`in_thread`), shown surfaces, `restore` and `thread_snapshot`; the agent's tab keeper persists it.

## Turn of a tool call

A thread surface goes in the turn open **when its call arrived**, not when it shows: `Tools::call`
reads `SurfaceHub::turn` first and passes it on (`show_static`, `show_report`, `ask_then`,
`show_params`). A late call from a closed turn is refused. The server stamps calls itself
because the agent does not know turn numbers; ACP events carry theirs (`oku-slides-agent`).

## Extension points

- New settings host: implement `SettingsPort` and `VoicePort`.
- New tool: an entry in `tools::list` (`tool_names`), an arm in `Tools::call`, a family in the
  local agent's `families_of`; its errors are `ToolError` variants with a Fluent key.

## Dependencies

`oku-slides-session` (the tab's `TabSession`, agent surfaces), `oku-slides-core`,
`oku-slides-a2ui`, `oku-slides-chart`, `oku-slides-i18n`, `oku-slides-platform` (listener),
`rust-mcp-sdk` and `rust-mcp-axum` 1.x (protocol `2025-11-25`); a 2.x migration stays local to
`server.rs`, `handler.rs` and `tools.rs`.
