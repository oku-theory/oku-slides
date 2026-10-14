# Voice agent in the workspace (tabs, windows, markdown)

Complement to `2026-09-26-agent-vocal-design.md` (t-224, under t-89). The
agent branch assumes a single document (`SharedSession =
Arc<Mutex<Session>>`); `main` has since gained windows and tabs, one
session per tab (`Workspace`, t-142, t-158), and an editing text in
markdown read by the UI alone (t-180). This document says how the agent
plugs into it; the rest of the original spec still holds.

## Decisions (chosen on 2026-09-27)

1. **One ACP subprocess for the app, one agent per tab.** Each tab has
   its ACP session (`session/new`, created at its first prompt) and its turns;
   the turns of two tabs run at the same time on the same connection.
   "Nouvelle conversation" (New conversation) only touches the current tab.
2. **One MCP tool server per tab.** Started with the tab's ACP session
   (free port, its own token), bound to that tab: its tools only
   see that document. It stops when the tab closes.
3. **One turn per tab.** In a tab, a prompt during a turn
   cancels it then starts again (as today); the other tabs are
   not touched.
4. **Each tab's thread is kept by the host** (200 thread entries,
   merged chunks; current state replayed) and replayed by `resync`: a tab's UI is remounted
   on every tab change (`<App key={tab}>`) and a tab can
   change window (another webview).
5. **Rewording goes through the UI parser.** An element's `rewrite`
   action carries `paragraphs` (read by `textToParagraphs`,
   `deck.ts`); the server no longer has a text reader
   (`text_to_paragraphs` goes away). A field (`@title`…) keeps `text`.

## Session: access to a tab

`Workspace::with_session(ws, tab, f: impl FnOnce(&mut Session) -> R) ->
Result<R>` (new, public): clones the slot's `Arc` under the workspace
lock, releases it, locks the slot, wakes it up
(`Slot::ready`), calls `f`. Same lock order as `request`; a
closed tab gives `UnknownTab`. It is the agent's only door into a
session.

## MCP (`oku-slides-mcp`)

- `SharedSession` becomes `TabSession`: a tab and an access to its
  session. Trait `Sessions` ("Implement this trait to give the agent a
  tab's session"): `fn with(&self, tab: &str, f: &mut dyn FnMut(&mut
  Session)) -> Result<(), SessionError>`; the hosts implement it on
  `Mutex<Workspace>` (via `with_session`), the tests on a single session.
- `SurfaceHub`, `Tools` and `ToolServer` stay **per tab** and keep
  their logic; only their access to the session changes (`TabSession`).

## Agent (`oku-slides-agent`)

- `Stack`: the shared ACP connection (subprocess started at the first
  prompt of any tab, restarted after a stop) and a table
  `tab → TabAgent { hub, tool server, ACP session, thread, actor }`,
  created at the tab's first prompt.
- Connection: notifications and permission requests are routed by
  `SessionId` to the `TabAgent`; `cwd` of `session/new`: folder of the
  tab's document (otherwise the fallback). A stop of the subprocess ends
  the turns of all tabs (error in each affected thread) and forgets
  their ACP sessions.
- Turn: `begin_turn`, `agent_context`, `end_turn` via `Sessions::with` on
  the tab. Tab closed during a turn: the turn is cancelled.
- `AgentCommand`: `prompt`, `cancel`, `new_conversation`, `surface_action`,
  `resync` carry `tab`. Events relayed in an envelope
  `{ tab, event }` (`mcp` and `agent`); `resync{tab}` sends back
  `Thread{entries}` then `Surfaces{list}`.
- Lifecycle: `tab_closed(tab)` (turn cancelled, ACP session forgotten,
  server stopped, questions released; a late command for that
  tab is then ignored, since ids never come back);
  `after_change(tab)` (pruning of surfaces). Another document opens in
  a new tab: no "document changed" on the host side. The agent's
  `new_deck` tool replaces its tab's deck (the conversation
  continues, the sticky notes are pruned). `open` (t-242) opens the file
  in a new tab of the same window, or activates the one that has it
  (`Sessions::open_tab`, `Workspace::open_beside`, never a replaced
  tab): the new tab has its own conversation, the agent's tab
  keeps its deck. `save {path}` refuses a file open in another
  tab (`Sessions::open_elsewhere`).
- Bounded startup: the agent must be ready (handshake done) within
  `READY_WITHIN` (120 s, `npx` may download); beyond that it is killed and
  the prompt receives `agent-stopped`.

## Hosts

- Tauri: `AppState` keeps `stack: Stack`; the `request` command
  calls `after_change(tab)`; `workspace` and `open_files` call
  `tab_closed` for each tab gone from the view; `agent` command
  (off the main thread).
  Events `oku-slides:mcp` / `oku-slides:agent` emitted to all
  windows (envelope with the tab), like `workspace`.
- `oku-slides serve`: the `main` server is a synchronous loop without
  streams; it gains the branch's: `/api/events` (SSE, dedicated threads) and
  `/api/agent`, next to `/api`, `/api/workspace`, `same_origin` and
  `MAX_BODY` from `main`, with the branch's anti-CSRF guard.

## UI

- `agent.*` (`api.ts`) takes the tab; `tabApi(tab)` provides one bound to it.
- `App` (one tab): filters envelopes on its tab, `resync` on
  mount: thread and surfaces reloaded from the host.
- Dictation ⌘⇧D: prompt for the active tab; `agentBusy` follows the turn of
  that tab.
- Rewording: `sendAction` adds `paragraphs = textToParagraphs(text)`
  when the action is `rewrite` on an element (not `original`).

## Tests

- Session: `with_session` (sleeping tab woken up, closed tab →
  `UnknownTab`, workspace lock released during `f`).
- MCP: existing tests on `TabSession`; rewording with paragraphs
  (bold kept); regression t-222.
- Agent (fake agent): two tabs, two ACP sessions on one connection,
  simultaneous turns, routed notifications and permissions; `tab_closed`
  during a turn; subprocess stop during two turns; thread replayed
  by `resync`.
- UI: filtering by tab, `sendAction` rewrite → paragraphs.
- End to end: the `agent.cjs` scenario in one tab, then a second
  tab with an empty thread whose prompt runs while the first one waits
  for an answer.
