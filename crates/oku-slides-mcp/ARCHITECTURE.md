# oku-slides-mcp

Serves the agent's MCP tools for one tab (HTTP on `127.0.0.1`, random bearer token per launch)
and holds the tab's A2UI surfaces (`SurfaceHub`) shown in the agent thread and dock.

## Modules

- `server` : `ToolServer`, the HTTP server (system-chosen port, token compared by SHA-256 in
  `token_matches`, bounded by `[serve]` limits and `SHUTDOWN_GRACE`).
- `handler` : the `ServerHandler`; `Tools::call` dispatches a call to its tool.
- `tools` : `Tools`, the tool list (`list`) and input schemas (`nest_schema` adapts JSON Schema
  to MCP 1.x); helpers `parse`, `with_session`, `json_result`, `reply`.
- Tool families: `read_tools` (outline, theme_tokens, slide, render, templates, widgets, settings),
  `edit_tools` (undoable deck edits), `widget_tools` (sources, preview, trials),
  `guarded_tools` (user-confirmed: `save`, `save_copy`, `open`, `move`, `merge`, `new`, `edit_theme`, `set_setting`…),
  `app_tools` (`history`; opening it, `present` and `set_interface` emit an event the shown tab applies),
  `surface_tools` (`report`, `render`, `ask_*`, `show_*`), `import_tools` (`import_report`,
  `import_diff`: the session's import tuning; its `ReferenceJob` runs in a blocking task, unlocked).
- `tool_error` : `ToolError`; `Tools::fail_with` logs it whole and tells the agent a localized
  message in the current interface language (`LiveLocalizer`, also read by the hub at each
  text). `error` : `McpError`, `say_session`. `paths` : validation of agent-given paths.
- `settings` : `SettingsPort` (implemented by the host), `Applied`, the agent's key whitelist.
- `hub` : `SurfaceHub`, open surfaces and events to the UI (`McpEvent`); `emit` alone changes
  the open list; `resync` resends it whole. `hub/asks` (pending questions, confirmations,
  permissions), `hub/actions` (`SurfaceAction`, `act`, `prune`), `hub/error` (`HubError`,
  `HubErrorKind`).
- `hub_turns` : thread turns (`begin_turn`, `close_turn`, `show_static`, `STATIC_PER_TURN`),
  freezing answered questions, eviction past `KEPT_TURNS` per scope.
- `hub_scopes` : turns filed by scope; only the viewed thread is sent. The hub never touches
  the session; the agent's tab keeper persists the thread.

## Turn of a tool call

A thread surface goes in the turn open **when its call arrived**, not when it shows: `Tools::call`
reads `SurfaceHub::turn` first and passes it on (`show_static`, `show_report`, `ask_then`,
`show_params`). A late call from a closed turn is refused. The server stamps calls itself
because the agent does not know turn numbers; ACP events carry theirs (`oku-slides-agent`).

## Extension points

- New settings host: implement `SettingsPort`.
- New tool: an entry in `tools::list` (`tool_names`), an arm in `Tools::call`, a family in the
  local agent's `families_of`; its errors are `ToolError` variants with a Fluent key.

## Dependencies

`oku-slides-session` (the tab's `TabSession`, agent surfaces), `oku-slides-core`,
`oku-slides-a2ui`, `oku-slides-chart`, `oku-slides-i18n`, `oku-slides-platform` (listener),
`rust-mcp-sdk` and `rust-mcp-axum` 1.x (protocol `2025-11-25`); a 2.x migration stays local to
`server.rs`, `handler.rs` and `tools.rs`.
