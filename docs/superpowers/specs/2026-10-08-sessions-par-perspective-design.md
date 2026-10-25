# Agent sessions per perspective, restored on reopening

> Superseded by docs/superpowers/specs/2026-10-24-onglets-pack-theme-design.md (t-1152): one conversation per tab, no scopes.

Task t-598. Complements `2026-09-27-agent-onglets-design.md` (one ACP session
and one thread per tab): within a tab, the agent now has **one session
per perspective** — the slides, a pack's workshop, each widget of a
pack. Switching from one perspective to another recalls the linked session (its thread
and the agent's memory); closing then reopening the application restores them.

Revised after the team review of the plan (2026-10-08): architecture
(ACP sessions outside the hub, one keeper per tab for writes),
scope identity (pack + name), hardening of the read-back.

## Decisions (made on 2026-10-08)

1. **Three kinds of scopes.** `Slides` (deck mode, and the history),
   `Atelier { pack }` (templates mode, "Modèles" (Templates) and "Thème" (Theme) tabs: the
   theme affects all slides, same conversation), `Widget { pack, name }`
   (templates mode, a widget's sub-tab). `pack` is the id of the edited pack
   (`TemplateMode::set`): two packs edited in the same tab each have
   their own conversations.
2. **Separate thread and memory.** Each scope has its ACP session and its thread.
   The agent of one scope does not see what was said in another.
3. **A widget session is linked to the widget, not to its sub-tab.**
   Closing the sub-tab keeps it; reopening it recalls its thread. Renaming the
   widget moves it. **Deleting the widget does not erase it right away**:
   undoing the deletion finds it again. Conversations whose widget (or
   pack) no longer exists are cleaned up at the next resume. Undoing a
   rename does not bring the conversation back under the old name (known limit).
4. **One turn at a time per tab** (the undo groups of `Session`
   are unique). A turn continues when the perspective changes, and ends
   in its thread. Meanwhile, in another scope, the interface blocks
   sending an instruction ("L'agent répond dans les slides…" (The agent is answering in the slides…)); ⌘⇧D
   cancels it wherever it is (Escape cancels only the turn of the viewed scope), and
   answering a live question of that turn remains possible. In the turn's scope, a prompt cancels then restarts, as
   today.
5. **Restoration after restart.** Threads are read back when the tab's
   agent starts, without launching the ACP process. A scope's ACP session
   is resumed only at its first prompt: `session/resume` if
   the agent announces it (`session_capabilities.resume`), otherwise `session/load`
   (`agent_capabilities.load_session`) ignoring its replay. On failure, or with neither
   one nor the other: new session, the thread stays, a card says so.
6. **"Nouvelle conversation" (New conversation) touches only the current scope**, in
   memory and on disk. Closing the document's tab forgets all its
   scopes (its resume folder goes with it).

## Scope (`oku-slides-session`, `agent::scope`)

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentScope { #[default] Slides, Atelier { pack: Uuid }, Widget { pack: Uuid, name: String } }
```

JSON: `{"kind":"slides"}`, `{"kind":"atelier","pack":"<uuid>"}`,
`{"kind":"widget","pack":"<uuid>","name":"frise"}`. The interface computes it
and sends it; the host does not infer it from the mode, so that a single source
decides what the user is looking at.

File name (`file_name`): `slides.json`, `atelier.<uuid>.json`,
`widget.<uuid>.<32 hex de sha256(nom)>.json` — fixed length, safe whatever
the name.

## Interface

- `agentScope({ templates, history, pack, widget })` (pure, `ui/src/agent/scope.ts`):
  outside templates mode, or history → `slides`; templates mode without an active
  widget → `atelier`; otherwise `widget`.
- `useAgentScope` (`ui/src/agent/useAgentScope.ts`) holds the scope, the agent's
  state (with the scope of its turn) and the thread recall: when the
  scope changes (snapshot loaded, subscription ready), the interface clears its
  surfaces then requests `resync(scope)`; the list that comes back is counted
  as a resynchronization (it is not read aloud).
  `onReady` resynchronizes only on reconnections. `Resync` starts the
  tools and the tab's keeper (not the ACP agent): a reopened tab
  thus finds its threads again as soon as it is shown.
- `agentApi(tab)`: `prompt`, `newConversation` and `resync` take the
  scope; `AgentCommand::{Prompt, NewConversation, Resync}` carry it
  (`#[serde(default)]`: when absent, it is `Slides`).
- `AgentEvent::Status` carries the turn's scope (`UserText` and
  `Conversation` too, for the turns reducer). While a turn runs
  in another scope: the thread shows the agent idle, the draft
  shows the note and refuses sending (except an answer to a question), a
  clicked follow-up is refused with the same note.

## Hub (`oku-slides-mcp`) — without I/O

The hub remains **a list of surfaces per tab** and does not touch the
session. Turn numbers remain unique within the tab; each turn
records its scope:

```rust
// HubState : turn_scopes: HashMap<u32, AgentScope>, viewing: AgentScope
```

- A surface is **shown** if it is live (dock, question, stage,
  drawer, modal), outside a turn, or if its turn is in the viewed scope (a turn
  without a known scope is not shown). The hub
  keeps everything, but sends only what is shown; a surface that goes from
  shown to hidden (frozen question of another scope) leaves as
  `CloseSurface`. The late relay sends back `visible_surfaces()`.
- `begin_turn(scope)` records the turn's scope and closes the oldest turns
  of the scope beyond `KEPT_TURNS` (read-back turns included).
- `resync(scope)` sets `viewing` and emits the shown list.
- `clear_thread(scope)` releases only the questions of the cleared turns.
- `restore(conversations)` puts back the read-back threads, **sanitized**: thread
  surfaces only, without actions, frozen; turn and identifier numbers
  bounded; a turn already taken (by another scope, or by an open turn)
  is ignored and logged; last `KEPT_TURNS` turns. No turn
  opens before the read-back.
- `thread_snapshot(scope)`: frozen surfaces of the scope, without actions, of the
  last **closed** turns (never the open turn), within a byte budget
  (`THREAD_BYTES`, under the 1 MiB read limit); nothing fits in
  the budget → `None` (nothing is written, the saved history stays).
- `rename_scope(from, to)`, `turn_scope(turn)`.

`KEPT_TURNS` (50) is defined only once, in `oku-slides-session::agent`.

## Tab keeper (`oku-slides-agent`, `keeper.rs`)

One task per tab, **the only path to conversation writes**:
it receives queued orders and applies them one by one, outside the async thread.

- On startup: reads `Session::conversations()` back and hands them to
  `hub.restore`. The actor waits for this read-back before its first turn.
- Before each order: applies to the hub the renames recorded by the session
  (`take_scope_moves` → `rename_scope`), and rewrites the scope of the
  order itself according to them (a session open during a rename follows the widget).
- `Thread(turn)` (end of turn, from the relay): the scope is read at the
  moment of writing (`turn_scope`), the thread's image taken then written
  (`keep_thread`). Turn cleared in the meantime: nothing.
- `Session { scope, id, agent }`, `Forget(scope)` (from the actor).
- `Lookup(scope, réponse)` (from the actor): the known session of the
  scope and its agent's fingerprint, read **in queue order** —
  hence after a forget or a rename already requested.
- `Show(scope)` (from `Resync`): `hub.resync(scope)`. `Stack::handle` therefore never
  takes the session.
- Closing the tab or the app: the queue is drained (at most 2 s for
  the whole app) before shutdown.

Queue order settles the races: a thread write that precedes a
forget is erased by it; a rename is applied before any write.

## Actor (`oku-slides-agent`, `tab_agent.rs`)

The actor keeps only the live state: `live: HashMap<String, u64>` (session
id → agent launch). It asks the keeper for a scope's session
(`Lookup`): no per-scope cache, hence nothing to keep up to date on
a rename or a forget.

- `prompt(scope, text, ui)`: live session of the current launch → it;
  otherwise a known session (read back, or from an agent stopped since) → resume;
  none → `session/new`. `fresh` (first-turn instructions) applies to
  a new session only.
- `new_conversation(scope)`: `Forget(scope)` to the keeper, which returns the forgotten
  id (its route is forgotten), then `hub.clear_thread(scope)`.
- After a finished connection, the session stays known: it will be resumed.
- `Starting` is emitted as long as the session is not live in the
  current launch.
- A prompt from another scope during a turn keeps the current behavior
  (cancel then instruction): it is the interface that blocks.

### Resuming a session

1. If the recorded agent fingerprint differs from `Link::agent_id()`
   (sha256 of the command and its arguments: never the command in
   clear on disk), no resume (step 4).
2. `session/resume` if `session_capabilities.resume`, otherwise `session/load`
   if `load_session`; `cwd` = `document_dir()` and the tab's MCP server
   (current URL and token).
3. The route is set **only after** the response: the `session/update` of the
   `session/load` replay arrive before it and are ignored like those
   of an unknown session.
4. Refusal by the agent, or no capability: `session/new`, and a card in
   the turn: "L'agent ne se souvient plus de cette conversation" (The agent no longer remembers this conversation)
   (`agent-memory-lost`, 20 languages). A finished connection remains an error
   of the turn.

## Persistence (`oku-slides-session`)

`agent/<file_name>` in the tab's resume folder, one file per
scope, versioned, read back by `read_json` (1 MiB at most):

```rust
pub struct SavedConversation { pub scope: AgentScope, pub agent: Option<String>, pub session: Option<String>, pub thread: Vec<Surface> }
// file: { "version": 1, ...SavedConversation }
```

- A single writer: the session; a single caller: the keeper.
  `keep_thread`, `keep_agent_session`, `forget_conversation`. The `Job` of the
  `resume_writer` carries the conversations to write or erase; merging
  two deposits keeps them all (the most recent wins per scope).
- `rename_widget` moves the conversation and records `ScopeMove`. Deleting a
  widget touches nothing (decision 3).
- Read-back: on the tab's resume; an unreadable file, or one whose
  scope does not match its name, is ignored (logged). Then
  **cautious** cleanup: a `Widget` conversation is erased only if
  files of its pack were found (edited version, deck's pack,
  current library version) and none of them contains the widget. A
  pack that cannot be found (deck from another machine, failed read) causes
  nothing to be erased; `Atelier` conversations are not cleaned up.
- Writing a scope: refused (logged) if its widget is known
  to no file of the pack or if its name is invalid (`parse_name`) — a
  scope invented by the interface does not create a file.
- `ServerMessage` (A2UI) and its messages, `Surface` and `Place` gain
  `Deserialize`.

## Errors

- Unreadable file: ignored and logged (`warn`), the scope starts empty.
- Failing write: logged (`warn`), the turn is not blocked.

## Tests

- `scope.test.ts`: `agentScope` (deck, history, workshop, widget),
  `sameScope`, `busyElsewhere`.
- Hub: surfaces shown per scope; surface shown then hidden →
  `CloseSurface`; `clear_thread(scope)` keeps the questions of the other
  scopes; `restore` sanitizes and bounds; eviction per scope;
  `thread_snapshot` (without actions, without questions, byte budget);
  `rename_scope`.
- Session: round trip across a restart; long widget name;
  corrupted or misnamed file ignored; rename; cleanup on resume
  (deleted widget, vanished pack); deletion then undo keeps the
  conversation; merging of deposits (pure function).
- Keeper: rename applied before the write; forget after a queued
  write.
- Actor: one session per scope; resume at the first prompt only;
  `resume` preferred to `load`; `load` replay absent from events and
  surfaces; refusal → new session + card + instructions; other agent →
  no resume; resume after the agent stopped; status carrying the
  scope; instruction from another scope queued during a turn.
- Stack: end-to-end restart (session resumed from a folder → thread
  shown on `resync`, first prompt as `session/resume` with the read-back id).
- e2e: prompt in the slides, open the workshop (empty thread), prompt through the
  draft, go back to the slides (only the first thread).

## Out of scope

- Listing or choosing old conversations (`session/list`).
- One scope per slide.
- `Origin` check of the development server (earlier).
