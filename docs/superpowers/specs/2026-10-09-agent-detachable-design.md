# Agent area detachable into a separate window

Task t-654. Today the agent lives in the right-hand column (`notes`) of
each workspace window: its thread, the dock of live A2UI
surfaces, the draft and the dictation device. This spec makes it possible to
**detach it into a window of its own**, designed for a second screen: the
main window gives its full width back to the slide.

Builds on `2026-09-26-onglets-fenetres-design.md` (workspace
windows) and `2026-10-08-sessions-par-perspective-design.md` (one session
per scope, state in the host).

## Decisions (made on 2026-10-09)

1. **Use: second screen.** A normal window, neither floating nor "always
   on top" (out of scope).
2. **A single agent window for the whole application**, with label `agent`.
   It follows **the active tab of the last workspace window
   brought to the foreground** (`WorkspaceView.focused` → `active`).
3. **What moves:** the thread (`AgentThread`), its dock, the draft
   (`DictationDraft`) and the dictation device (`Dictation`). **What
   stays** on the main window's stage: anchored surfaces
   (`sheet`), the variants drawer (`drawer`) and modals (`modal`) —
   they are about the slide.
4. **Persisted.** The detached state is kept with the workspace
   layout; size, position and screen through `tauri-plugin-window-state`
   (label `agent`). Closing the agent window reattaches it.
5. **The draft survives** detaching, reattaching and switching
   tabs: it is held by the host, per tab, in memory (not across
   restarts).
6. **The host stays the single source of truth** (approach A): the agent window
   subscribes to the agent events itself, and reads the scope and the
   draft from the host. The only channel between windows carries what exists
   only in the main window: where to write a text.
7. **Tauri only.** In browser mode (dev server), no detach button or
   menu.

Rejected: a "mirror" window driven by the main window (a second
protocol, doubled state); an agent window that recomputes the scope (it
depends on UI state specific to the main window).

## Architecture

### Host (Rust)

- **Workspace** (`oku-slides-session`): `agent_detached: bool`
  in the layout, exposed in `WorkspaceView`, saved and read back by
  `workspace_resume`. Methods `detach_agent()` / `attach_agent()`.
  Matching `workspace` commands; each one rebroadcasts `workspace`.
- **Focus**: `Workspace::focus` only receives the labels of workspace
  windows; `commands::focused` explicitly ignores the `agent` label
  (today rejected by accident, as "focus ignoré" (focus ignored)). Giving
  focus to the agent window never changes the target.
- **Windows** (`src-tauri/src/windows.rs`): `sync` also reconciles the
  `agent` window — open if `agent_detached`, closed otherwise. The decision
  (open / close / nothing) is a pure function, tested without Tauri. Default
  size 420×800, minimum 320×400; geometry restored by
  window-state. Its close request (`CloseRequested`) calls
  `attach_agent` instead of touching the tabs. It closes with the
  last workspace window, without changing `agent_detached`.
- **Capability**: `src-tauri/capabilities/default.json` covers
  `["w-*", "agent"]` (same commands: `agent`, `workspace`, `stt`, `draft`,
  and `core:event` for `emitTo`).
- **Displayed scope**: the hub already holds it (`HubState.viewing`, set by
  `resync`). New subcommand `agent` → `scope { tab }` that reads it, and
  new event `AgentEvent::Scope { scope }` emitted by `resync` when
  it changes. The main window alone decides the scope
  (`useAgentScope`); the agent window follows it.
- **Draft**: `draft: Option<String>` in memory in the tab's
  session. Command `draft`: `get { tab }`, `set { tab, text | null }`;
  `set` broadcasts `oku-slides:agent` → `{ kind: "draft", text }`. The last
  write wins.
- **Menu**: in the "Fenêtre" (Window) menu, an "Agent dans une fenêtre
  séparée" (Agent in a separate window) checkbox (`CheckMenuItem`), kept up to
  date from `agent_detached`. Labels via Fluent.

### UI (React)

- **Root**: `main.tsx` mounts `AgentWindow` if the window label is
  `agent`, otherwise `Workspace`.
- **`agentTarget(view)`** (pure): the target tab, `null` if none.
- **`AgentWindow`**: listens to `workspace`, derives the target tab from it; on each
  tab change: `subscribeHost(tab, …)`, `agent.scope(tab)`,
  `draft.get(tab)`, then `agent-link:hello`. Mounts `AgentThread` (surfaces
  routed by `routeSurfaces`, only those of the thread), `DictationDraft` and
  `Dictation`. "Rattacher" (Reattach) button. Read-only or help tab: empty
  state ("Pas d'agent pour cet onglet" (No agent for this tab)), the dictation
  device stays.
- **`App`, agent detached**: the `notes` column no longer mounts the thread, the
  draft or the dictation; it shrinks to 0 outside the "Thème" (Theme) tab
  (`ThemePanel` stays). The `sheet` / `drawer` / `modal` surfaces are still
  rendered here. `App` publishes its writing context (below) and applies
  the `insert` messages it receives.
- **`useDraft(tab)`** replaces the draft's `useState`, in `App`
  (attached) as in `AgentWindow`: reads `draft.get`, follows the `draft`
  event, writes through `draft.set` — typing debounced (~300 ms); dictation,
  Send, Discard immediately. The `dictations` /
  `typing` counters (focus, scrolling) stay local.
- **⧉ button** in the `AgentThread` header, next to "Nouvelle
  conversation" (New conversation): `detach_agent`. Hidden outside Tauri.

The agent wiring in `App.tsx` is to be extracted at the same time (see
t-462): `App` must not carry two branches (attached / detached) everywhere.

## Channel between windows: `agent-link`

Targeted Tauri events; all carry `tab`, and a message whose tab no longer
matches is ignored.

| Event | Direction | Content | When |
|---|---|---|---|
| `agent-link:context` | main → `agent` | `{ tab, editing, vocabulary, readOnly }` | on each change, and in response to `hello` |
| `agent-link:hello` | `agent` → all | `{ tab }` | opening, change of target tab |
| `agent-link:insert` | `agent` → `focused` | `{ tab, id, text }` | dictation to the cursor |
| `agent-link:insert-ack` | main → `agent` | `{ tab, id, ok }` | after `insert` |
| `agent-link:shortcut` | main → `agent` | `{ tab, key: "dictate" \| "draft" }` | ⌘D / ⌘K pressed in the main window |

- `editing`: a text edit is in progress; `vocabulary`: deck and slide
  titles, for dictation.
- Only the main window that displays the target tab answers `hello`.

## Flows

- **Dictation.** The agent window routes with `dictationRoute(mode,
  context.editing)`: to the cursor → `insert`; otherwise into the draft
  (`draft.set`).
- **Insert.** `insert` → the main window calls `insertText`, which writes at
  the cursor only while a text edit is in progress, and answers `insert-ack`.
  The draft has no Insert action since t-953: it only talks to the agent.
  With no `ack` within 1 s, or `ok: false` (tab changed, edit ended), the agent window puts the
  text back into the draft with a "Texte gardé dans le brouillon" (Text kept in
  the draft) toast.
- **Send.** `agent.prompt(scope, text)` from the agent window.
- **Shortcuts.** ⌘D in the main window → `shortcut: dictate`: the
  agent window starts or stops listening **without taking focus**
  (the edit keeps its cursor). ⌘K → `shortcut: draft`: the agent window
  comes to the front and opens the draft for typing. In the agent window, ⌘D and
  ⌘K act in place.
- **Focus lost during editing.** Clicking the agent window's microphone
  makes the main window lose focus: `editing` and the cursor position must
  survive there, so that `insert` writes in the right place.
- **Detach / reattach during a turn.** The turn lives in the host: the
  new view does `resync` and resumes the stream. The draft is read back from
  the host.

## Edge cases

| Situation | Behavior |
|---|---|
| The foreground window changes, or its active tab | `AgentWindow` changes target (resubscription, scope, draft, `hello`). |
| Last workspace window closed | The agent window closes with the app; `agent_detached` stays. |
| Target tab read-only / help | Empty state; dictation visible, its text receives the session's rejection via `insert`. |
| Restored screen missing | window-state moves the window back onto a visible screen (checked). |
| No target tab (view not received yet) | Loading state, no command sent. |

## Tests

**Rust**
- `detach_agent` / `attach_agent`; `agent_detached` saved then read back by resume.
- Focusing a label outside the workspace windows does not change `focused`.
- Reconciliation decision for the `agent` window (open / close / nothing).
- `resync` emits `Scope` when the scope changes, not otherwise; `scope { tab }` returns it.
- `draft`: `set` then `get`; broadcast; isolation between tabs; `null` clears.

**TypeScript (`bun test`)**
- `agentTarget`: foreground window, active tab, empty view.
- `routeSurfaces` on the agent window side: only the thread's surfaces.
- `useDraft`: typing debounce, immediate sends, following the event.
- `agent-link` channel as pure modules: dictation routing, `ack`, fallback
  into the draft on rejection or timeout, message from another tab ignored.

**e2e** (free port, isolated data)
- Detach → write in the draft → Send → reattach: the draft is kept.
- Restart: the agent window is restored detached.
- Editing in progress, microphone clicked in the agent window: the text arrives at the cursor.

## Plan clarifications (2026-10-09, revised after the team review and t-638)

The plan (`docs/superpowers/plans/2026-10-09-agent-detachable.md`) details the
mechanics. These points replace anything above that contradicts them.

1. **Viewed scope.** The hub holds it (`HubState.viewing`) and broadcasts it as
   `McpEvent::Viewing { scope }`:
   - `resync` emits it before the list;
   - `rename_scope` emits it when the viewed scope is renamed;
   - the lagging relay sends it again with the list.

   The command `agent` → `follow { tab }` replays the state and the draft, then
   sends the keeper the `Keep::Replay` order. The keeper reads the scope
   **when it processes the order**, so that a `resync` sent just before
   always goes first. There is neither `AgentEvent::Scope` nor an
   `agent.scope` request.
2. **Draft.** The stack holds it in memory, in the tab's view
   (`TabLink`). It is written through `agent` → `set_draft { tab, text, origin }`,
   which broadcasts `AgentEvent::Draft { text, origin }`.
   - Each view ignores the echo of its own sends (`origin`) and sends them
     in order, one at a time.
   - The draft is capped at 256 KiB: beyond that, the command is rejected.
   - `follow` and `set_draft` never start a tab. On a tab with no
     started agent, `follow` replays "nothing" (idle, no draft, `slides`,
     no surface), and `set_draft` is rejected with a localized error.
   - A draft's `origin` is capped at 64 bytes.
   - Each send from a view carries its number in the origin (`v-xxx:n`).
     As long as the echo of the last one has not come back, a draft coming
     from elsewhere is older than the view's last write, and the view ignores it.
   - A replay (null `origin`) while the host does not have the view's last
     write (missing echo, or send rejected for lack of a started tab): the
     view sends its local draft back to it. This way a ⌘K or a dictation made
     before the first `resync` survives the tab's startup.
3. **UI.** `AgentPanel` (t-638) has two scope modes:
   - **driver**: the scope comes from the view; this is the main window's
     mode;
   - **follower**: the scope comes from `viewing` after `follow`; this is the
     agent window's mode.

   Once the agent is detached:
   - the main window keeps a driver panel without the thread (sheets, drawer,
     confirmation window, quick keys);
   - the agent window mounts a follower panel with the thread. Its quick
     keys only target the dock, and it routes the sheets according to the one
     the stage shows (`staged`).

   As long as the host has not announced the scope, sending a prompt stays
   disabled, as with no agent. The agent window shows its loading state
   only before the first workspace view.

   Once the agent is detached, the main window's quick keys no longer
   target the dock.

   On macOS, the agent window sets the same title bar
   (`.titlebar`) as the other windows: without it, it could not be
   moved.

   On a read-only tab, the agent window shows an empty state,
   but dictation stays visible.
4. **Writing context.** `agent-link:context` carries
   `{ editing, ui, staged, keysOff }`. The agent window reads
   the vocabulary, read-only status, thumbnails and deck appearance from the
   tab's snapshot itself. The context also carries `look`,
   the fingerprint of this appearance: when it changes, the agent window
   reads the snapshot again. The scheme (light, dark) applies to the app: the host
   broadcasts the choice (`scheme`) to all windows.
5. **Transport.** A single Tauri event, `agent-link`, broadcast to all
   windows (the sender included).
   - Each page listens only once and keeps the messages addressed
     to it (`to`, `tab`).
   - Each message is validated by a zod schema.
   - Outside Tauri, the channel is a `BroadcastChannel` and the agent window
     is the page `/?window=agent` (e2e).
   - `hello` is sent only once listening is in place.
   - `insert` waits 3 s for an acknowledgement. Without an answer, it sends
     `insert-cancel`, which the main window honors, and the text returns
     to the draft with a toast.
   - Known limit: if the main window has already written a text whose
     acknowledgement arrives after the deadline, that text also ends up in the
     draft. Writing a text would have to take more than 3 s.
   - Message fields are bounded: 64 characters for an
     identifier, 256 for a label, the draft size for a text.
6. **Shortcuts.** The main window relays ⌘D, ⌘⇧D and ⌘K **raw**
   (`auto`, `draft`, `type`). The agent window applies them with its
   own state: ⌘⇧D stops the turn only if no listening is in
   progress. ⌘K brings the agent window to the front; ⌘D does not take focus.
   Known limit: Escape in the main window does not stop listening in
   the agent window; ⌘D does.
7. **Focus.**
   - A change of foreground window broadcasts the `workspace` view, and the
     agent window follows it.
   - The `agent` window never changes `focused`.
   - It is not a tab drop target.
   - The text editor no longer ends the edit when the document loses
     focus (`document.hasFocus()` false), to another window or another
     app. This change also applies with no detached agent.
8. **Closing.** The system close of the agent window goes through its
   page (`close-requested`), which flushes pending typing then reattaches
   the agent. The "Rattacher" (Reattach) button does the same.
9. **Menu.** Only macOS has an app menu: the checkbox exists only there.
   The checkbox reflects the workspace, and a click toggles the state read from
   the workspace. Reattaching through the menu goes through the agent
   window's page (`close-requested`), like the system close; without
   this window (failed opening), the host reattaches directly.
10. **Tests.** The e2e runs in a browser, without Tauri. It covers:
    detaching, sending from the agent window, relayed ⌘K, inserting into
    the selected box and draft kept on reattach.

    The rest is covered at unit level, then checked by hand in the app
    in the last task: restart, focus loss during editing, tracking
    window focus, microphone in a second webview, menu.

    Spoken dictation is replaced by keyboard input: the e2e has no
    fake speech recognition engine.

## Out of scope

"Always on top" window; several agent windows; detaching in browser
mode; draft kept across restarts; detaching the stage surfaces.
