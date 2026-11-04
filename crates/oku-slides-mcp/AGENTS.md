# crates/oku-slides-mcp — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Limit agent set_setting to its whitelist, never agent settings; refuse agent set_dictation on API address, variables, query or headers. (spec:2026-09-26-agent-vocal-design, t-1144, t-1213, t-786)
- Never let agent open replace a tab, and refuse any save to a file already open in another tab. (spec:2026-09-27-agent-onglets-design, t-255)
- Keep Connection: close on the MCP 401 response so unauthenticated clients never hold a connection slot. (plan:2026-10-09-petits-tickets-t678)
- Route every MCP tool error through Tools::fail_with; never return hardcoded English text. (plan:2026-10-09-tickets-debloques-t598)
- Pass only safe arguments such as surface id or action name to localized errors; keep full error strings in tracing. (plan:2026-10-09-tickets-debloques-t598)
- File each thread surface in the turn read when its tool call arrives, never in the turn open when it shows. (spec:2026-10-03-agent-a2ui-design)
- Answer a typed reply only to the question id the UI names, a host-routed utterance to the hub's own target (modal, the sheet on the page's reported slide, dock question, then drawer). Answer a guarded confirmation or a permission only from a host-routed microphone utterance, through the lexicon: the verb for danger, the option kind for permissions; never from prompt text or agent-written labels. (plan:2026-10-04-agent-thread-followups-2, plan:2026-11-01-lot-m-hands-free, t-997)
- Log each MCP tool call at debug level with its name and duration only, never its arguments. (spec:2026-09-26-agent-vocal-design, plan:2026-09-26-agent-vocal)
- Bind the MCP tool server to 127.0.0.1 only. (plan:2026-09-26-agent-vocal)
- Update the hub list of open surfaces only inside emit, under the state lock, so it never diverges from events. (plan:2026-09-26-agent-vocal)
- In SurfaceHub, take the state lock only after the session's or a Sessions call's own locks, never the reverse, and never hold it across an await. (plan:2026-09-26-agent-vocal, plan:2026-10-25-onglets-pack-theme-lot3-pack-tabs)
- Refuse render and report calls outside their open turn so late calls never show in any turn. (spec:2026-10-03-agent-a2ui-design, plan:2026-10-03-agent-a2ui)
- Once a new turn begins, swap the previous receipt for its plain version and refuse its actions. (plan:2026-10-03-agent-a2ui)
- Keep a frozen question at the same id and position in the open list so thread order survives a resync. (plan:2026-10-03-agent-a2ui)
- Decide whether to freeze or close a surface inside show_static under its lock, so nothing runs in between; freeze sets Place::Thread, so a frozen inspector offer shows and saves in its turn. (plan:2026-10-03-agent-a2ui, spec:2026-11-02-hub-inspector-traces-design, t-1507)
- Never send deleteSurface in a surface_patch; close surfaces with close_surface. (plan:2026-10-03-agent-a2ui)
- Leave thread persistence to the tab keeper; the hub keeps one thread per tab and sends only shown surfaces. (spec:2026-10-24-onglets-pack-theme-design, t-1154)
- Never hold the shared Session mutex across an await in MCP tools; lock only for one Session::handle call, or one undo or redo run of at most 20 steps. (spec:2026-09-26-agent-vocal-design, t-999)
- Reject MCP tool paths that are not absolute, normalized and free of .. segments. (spec:2026-09-26-agent-vocal-design)
- Do not give ask_* surfaces a timeout; Escape, a new turn or conversation end must return cancelled. (spec:2026-09-26-agent-vocal-design)
- Scope SurfaceHub, Tools and ToolServer to one tab; their tools must only see that tab's document, and only open_pack and open_theme may open or focus a pack or theme tab, through Sessions::open_pack / Sessions::open_theme; tabs, tab and window alone reach the requester's window, through AgentWorkspace, and close only through a decision the page redeems. (spec:2026-09-27-agent-onglets-design, plan:2026-10-25-onglets-pack-theme-lot3-pack-tabs, t-1191, plan:2026-11-01-lot-m-hands-free)
- Answer ask or permission requests on a closed tab with Cancelled; the SurfaceHub closes when the tab entry drops. (t-249)
- On resync emit the full open surface list, and have the UI replace its surfaces rather than merge them. (t-182)
- Never let an agent set the dictation API address or key variable; refuse before parsing, with ApiReservedToUser. set_dictation may turn hands-free off, never on. Read aloud only host-built SpokenText: a guard's template and verb, a permission's template, an error's host template only, never its message, title or option labels. (spec:2026-10-23-reglages-dictee-a2ui-design, plan:2026-11-01-lot-m-hands-free, t-1002)
- Check every voice tool request through VoicePort before confirming; a refusal opens no dialog. (spec:2026-10-23-reglages-dictee-a2ui-design)
- Rebuild the settings box only from the VoicePort view through its one send path, never from prune. (spec:2026-10-23-reglages-dictee-a2ui-design)
- Call prune after every surface action that changes the deck, so the theme picker and packs panel re-read it. (t-1133)
- Let open_pack, open_theme, import, help and tab (activate, reorder, move) make at most one tab move (new or focused) per agent turn through SurfaceHub::reserve_tab, and give the claim back when no tab moved (Here, Refused, denied, error); none outside a turn; import claims it before its confirmation and before any AgentWorkspace lock, whose gate never calls the hub; refuse that turn's widget writes in the requesting tab once it moved; never show the agent another tab's title, hub errors included, except the tabs listing of its window, each title through bounds::label. Their request is agent-written data: only a moved tab's draft shows it, only the user sends it, and no result echoes it. (plan:2026-10-25-onglets-pack-theme-lot3-pack-tabs, t-1191, t-1277, plan:2026-11-01-lot-m-hands-free)
- Push every resync and follow to all views of a tab, not only to the view that asked. (t-670)
- List and serve only the tools of the tab's document kind, read once when its agent starts; refuse any other with UnknownTool. (spec:2026-10-24-onglets-pack-theme-design, t-1191)
- Emit page events (Present, OpenHistory, Interface, Close) only after ensure_shown, an own close checked shown again once the turn ended, a present start only if Session::can_present allows (the page reads Snapshot.can_present, never a copy of the rule), the other present commands only while the hub presents (its state taken only from the page's Presenting reports, ordered by the host, never from a present call), and open history through the tab so its pending edit and save go first. (t-988, t-991, t-992, t-1007, t-998, plan:2026-11-01-lot-m-hands-free)

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
- Confirm non-undoable MCP tools through SurfaceHub::confirm before executing, naming a ConfirmVerb for a danger one; add no host-side permission trait. A pack or theme tab saves through its autosave; the agent's save_pack and save_theme only save confirmed copies. (spec:2026-09-26-agent-vocal-design, t-1191, t-997)
- Confirm every edit_theme, a built-in theme included (saved as a variant extending it); a change to this deck only goes through set_token ops on deck.tokens. (t-692, t-1100)
- Emit widget_touched only in pack mode. (plan:2026-10-03-widgets-agent-a2ui, t-1065)
- Return invalid render or report input to the agent as a readable tool-result error, and display nothing. (spec:2026-10-03-agent-a2ui-design)
- Change the interface language through set_interface, never set_setting, so every window follows at once. (t-988)
- Open the theme picker, packs panel and settings box only through show_themes/show_packs/show_dictation and their SurfaceHub methods, show_themes refusing an unknown slide; a tool call files the box in agent_turn (a chip), a command or spoken shortcut in no turn, and every refresh keeps it; ThemeChoice stays host-only in render. (t-1089, t-1114, t-1506)
- Do not expose rename_widget over MCP; the agent writes a new widget and then removes the old one. (spec:2026-10-02-creation-widgets-design)
