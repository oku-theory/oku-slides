# crates/oku-slides-mcp — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Limit agent set_setting to its whitelist of keys; never let it reach the paths section. (spec:2026-09-26-agent-vocal-design)
- Never let agent open replace a tab, and refuse agent save to a file open in another tab. (spec:2026-09-27-agent-onglets-design)
- Keep Connection: close on the MCP 401 response so unauthenticated clients never hold a connection slot. (plan:2026-10-09-petits-tickets-t678)
- Route every MCP tool error through Tools::fail_with; never return hardcoded English text. (plan:2026-10-09-tickets-debloques-t598)
- Pass only safe arguments such as surface id or action name to localized errors; keep full error strings in tracing. (plan:2026-10-09-tickets-debloques-t598)
- File each thread surface in the turn read when its tool call arrives, never in the turn open when it shows. (spec:2026-10-03-agent-a2ui-design)
- Answer a typed or dictated reply only to the question id the UI names; the hub never guesses the target. (plan:2026-10-04-agent-thread-followups-2)
- Log each MCP tool call at debug level with its name and duration only, never its arguments. (spec:2026-09-26-agent-vocal-design, plan:2026-09-26-agent-vocal)
- Bind the MCP tool server to 127.0.0.1 only. (plan:2026-09-26-agent-vocal)
- Update the hub list of open surfaces only inside emit, under the state lock, so it never diverges from events. (plan:2026-09-26-agent-vocal)
- In SurfaceHub, lock the session before state, never the reverse, and never hold state across an await. (plan:2026-09-26-agent-vocal)
- Refuse render and report calls outside their open turn so late calls never show in any turn. (spec:2026-10-03-agent-a2ui-design, plan:2026-10-03-agent-a2ui)
- Once a new turn begins, swap the previous receipt for its plain version and refuse its actions. (plan:2026-10-03-agent-a2ui)
- Keep a frozen question at the same id and position in the open list so thread order survives a resync. (plan:2026-10-03-agent-a2ui)
- Decide whether to freeze or close a surface inside show_static under its lock, so nothing runs in between. (plan:2026-10-03-agent-a2ui)
- Never send deleteSurface in a surface_patch; close surfaces with close_surface. (plan:2026-10-03-agent-a2ui)
- Leave thread persistence to the tab keeper; the hub files turns by scope and sends only the viewed thread. (plan:2026-10-08-sessions-par-perspective)
- Never hold the shared Session mutex across an await in MCP tools; lock only for one Session::handle call. (spec:2026-09-26-agent-vocal-design)
- Reject MCP tool paths that are not absolute, normalized and free of .. segments. (spec:2026-09-26-agent-vocal-design)
- Do not give ask_* surfaces a timeout; Escape, a new turn or conversation end must return cancelled. (spec:2026-09-26-agent-vocal-design)
- Scope SurfaceHub, Tools and ToolServer to one tab; their tools must only see that tab's document. (spec:2026-09-27-agent-onglets-design)
- Answer ask or permission requests on a closed tab with Cancelled; the SurfaceHub closes when the tab entry drops. (t-249)
- On resync emit the full open surface list, and have the UI replace its surfaces rather than merge them. (t-182)
- Make the set_setting tool refuse agent settings. (t-786)
- Refuse save with a path to a file that is already open in another tab. (t-255)
- Push every resync and follow to all views of a tab, not only to the view that asked. (t-670)

## Pitfalls

- Nest a tool schema's $defs under its property; rust-mcp-sdk 1.x keeps only properties and required at the root. (plan:2026-09-26-agent-vocal)

## Testing

- Test the MCP server as a real server on 127.0.0.1 port 0 with a rust-mcp-sdk client in the test. (spec:2026-09-26-agent-vocal-design)

## Settled decisions

- Bound the MCP tool server with the existing [serve] header_timeout_ms and max_connections, never a new config key. (plan:2026-10-09-petits-tickets-t678)
- Translate hub and MCP errors in Rust via Fluent; the UI only displays the text it receives. (plan:2026-10-09-tickets-debloques-t598, t-463)
- Stay on rust-mcp-sdk 1.x until agents embed MCP 2026-07-28 clients; the 2.0 server speaks only that version. (plan:2026-09-26-agent-vocal)
- Prefix dictation settings keys with dictation. so they are not confused with the stt section of config.toml. (plan:2026-09-26-agent-vocal)
- Confirm non-undoable MCP tools through SurfaceHub::confirm before executing; add no host-side permission trait. (spec:2026-09-26-agent-vocal-design)
- Emit widget_touched only in templates mode. (plan:2026-10-03-widgets-agent-a2ui)
- Return invalid render or report input to the agent as a readable tool-result error, and display nothing. (spec:2026-10-03-agent-a2ui-design)
- Do not expose rename_widget over MCP; the agent writes a new widget and then removes the old one. (spec:2026-10-02-creation-widgets-design)
