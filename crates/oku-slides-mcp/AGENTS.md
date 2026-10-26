# crates/oku-slides-mcp — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Limit agent set_setting to its whitelist; refuse agent set_dictation on API address, variables, query or headers. (spec:2026-09-26-agent-vocal-design, t-1144, t-1213)
- Never let agent open replace a tab, and refuse any save to a file already open in another tab. (spec:2026-09-27-agent-onglets-design, t-255)
- Keep Connection: close on the MCP 401 response so unauthenticated clients never hold a connection slot. (plan:2026-10-09-petits-tickets-t678)
- Route every MCP tool error through Tools::fail_with; never return hardcoded English text. (plan:2026-10-09-tickets-debloques-t598)
- Pass only safe arguments such as surface id or action name to localized errors; keep full error strings in tracing. (plan:2026-10-09-tickets-debloques-t598)
- File each thread surface in the turn read when its tool call arrives, never in the turn open when it shows. (spec:2026-10-03-agent-a2ui-design)
- Answer a typed or dictated reply only to the question id the UI names; the hub never guesses the target. (plan:2026-10-04-agent-thread-followups-2)
- Log each MCP tool call at debug level with its name and duration only, never its arguments. (spec:2026-09-26-agent-vocal-design, plan:2026-09-26-agent-vocal)
- Bind the MCP tool server to 127.0.0.1 only. (plan:2026-09-26-agent-vocal)
- Update the hub list of open surfaces only inside emit, under the state lock, so it never diverges from events. (plan:2026-09-26-agent-vocal)
- In SurfaceHub, take the state lock only after the session's or a Sessions call's own locks, never the reverse, and never hold it across an await. (plan:2026-09-26-agent-vocal, plan:2026-10-25-onglets-pack-theme-lot3-pack-tabs)
- Refuse render and report calls outside their open turn so late calls never show in any turn. (spec:2026-10-03-agent-a2ui-design, plan:2026-10-03-agent-a2ui)
- Once a new turn begins, swap the previous receipt for its plain version and refuse its actions. (plan:2026-10-03-agent-a2ui)
- Keep a frozen question at the same id and position in the open list so thread order survives a resync. (plan:2026-10-03-agent-a2ui)
- Decide whether to freeze or close a surface inside show_static under its lock, so nothing runs in between. (plan:2026-10-03-agent-a2ui)
- Never send deleteSurface in a surface_patch; close surfaces with close_surface. (plan:2026-10-03-agent-a2ui)
- Leave thread persistence to the tab keeper; the hub keeps one thread per tab and sends only shown surfaces. (spec:2026-10-24-onglets-pack-theme-design, t-1154)
- Never hold the shared Session mutex across an await in MCP tools; lock only for one Session::handle call. (spec:2026-09-26-agent-vocal-design)
- Reject MCP tool paths that are not absolute, normalized and free of .. segments. (spec:2026-09-26-agent-vocal-design)
- Do not give ask_* surfaces a timeout; Escape, a new turn or conversation end must return cancelled. (spec:2026-09-26-agent-vocal-design)
- Scope SurfaceHub, Tools and ToolServer to one tab; their tools must only see that tab's document, and only edit_pack may open or focus a pack tab, through Sessions::open_pack. (spec:2026-09-27-agent-onglets-design, plan:2026-10-25-onglets-pack-theme-lot3-pack-tabs)
- Answer ask or permission requests on a closed tab with Cancelled; the SurfaceHub closes when the tab entry drops. (t-249)
- On resync emit the full open surface list, and have the UI replace its surfaces rather than merge them. (t-182)
- Make the set_setting tool refuse agent settings. (t-786)
- Never let an agent set the dictation API address or key variable; refuse before parsing, with ApiReservedToUser. (spec:2026-10-23-reglages-dictee-a2ui-design)
- Check every voice tool request through VoicePort before confirming; a refusal opens no dialog. (spec:2026-10-23-reglages-dictee-a2ui-design)
- Rebuild the settings box only from the VoicePort view through its one send path, never from prune. (spec:2026-10-23-reglages-dictee-a2ui-design)
- Call prune after every surface action that changes the deck, so the theme picker and packs panel re-read it. (t-1133)
- Let edit_pack open at most one new pack tab per agent turn through SurfaceHub::reserve_pack_tab, and none or no focus change outside a turn; refuse that turn's widget writes in the requesting tab once it opened one; never show the agent another tab's title, hub errors included. (plan:2026-10-25-onglets-pack-theme-lot3-pack-tabs)
- Push every resync and follow to all views of a tab, not only to the view that asked. (t-670)
- Emit page events (Present, OpenHistory, Interface) only after ensure_shown, and open history through the tab so its pending edit and save go first. (t-988, t-991, t-992)

## Pitfalls

- Nest a tool schema's $defs under its property; rust-mcp-sdk 1.x keeps only properties and required at the root. (plan:2026-09-26-agent-vocal)

## Testing

- Test the MCP server as a real server on 127.0.0.1 port 0 with a rust-mcp-sdk client in the test. (spec:2026-09-26-agent-vocal-design)

## Settled decisions

- Bound the MCP tool server with the existing [serve] header_timeout_ms and max_connections, never a new config key. (plan:2026-10-09-petits-tickets-t678)
- Translate hub and MCP errors in Rust via Fluent; the UI only displays the text it receives. (plan:2026-10-09-tickets-debloques-t598, t-463)
- Word a session error for the agent in say_to_agent (tool replies), never in the session; hub cards keep say_session. (t-1171)
- Stay on rust-mcp-sdk 1.x until agents embed MCP 2026-07-28 clients; the 2.0 server speaks only that version. (plan:2026-09-26-agent-vocal)
- Change dictation only through VoicePort (set_dictation, the box); set_setting keeps no dictation key. (spec:2026-10-23-reglages-dictee-a2ui-design, t-1144)
- Confirm non-undoable MCP tools through SurfaceHub::confirm before executing; add no host-side permission trait. (spec:2026-09-26-agent-vocal-design)
- Confirm every edit_theme, a built-in theme included (saved as a variant extending it); a change to this deck only goes through set_token ops on deck.tokens. (t-692, t-1100)
- Emit widget_touched only in pack mode. (plan:2026-10-03-widgets-agent-a2ui, t-1065)
- Return invalid render or report input to the agent as a readable tool-result error, and display nothing. (spec:2026-10-03-agent-a2ui-design)
- Change the interface language through set_interface, never set_setting, so every window follows at once. (t-988)
- Open the theme picker and packs panel only through show_themes/show_packs/show_dictation and their SurfaceHub methods, outside any turn, show_themes refusing an unknown slide; ThemeChoice stays host-only in render. (t-1089, t-1114)
- Do not expose rename_widget over MCP; the agent writes a new widget and then removes the old one. (spec:2026-10-02-creation-widgets-design)
