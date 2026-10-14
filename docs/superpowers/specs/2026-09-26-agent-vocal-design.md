# Voice agent: ACP, MCP and A2UI surfaces — design

Date: 2026-09-26 · Task: t-89

## Goal

The user talks to an agent: ⌘⇧D dictates an instruction (local Whisper),
sent over ACP to an agent that acts on Oku Slides only through MCP tools
that Oku Slides injects into it. These tools cover every change
to the deck, the settings and the application's actions. When a decision
belongs to the user, the agent answers with an A2UI surface: choice,
confirmation, rewrite, parameters, design variants.

Out of scope: stdio MCP relay for agents without HTTP; raw A2UI
written by the agent; conversation saved in the document; speech
synthesis of the answers; `undo` / `redo` for the agent.

## Decisions

| # | Decision |
| --- | --- |
| 1 | A single mic, one draft with two outputs (revised on 2026-09-27; first two modes, ⌘D text and ⌘⇧D agent, but stopping on silence sent partial instructions). ⌘D writes at the cursor while editing a text, otherwise into the draft; ⌘⇧D always writes into the draft. The draft is **inserted** as is (local, offline) or **sent to the agent**. |
| 2 | Any ACP agent, declared in `[agent]` (`command`, `args`); `claude-code-acp` by default. |
| 3 | The agent acts only through Oku Slides' MCP tools: the ACP client announces neither `fs` nor `terminal`. |
| 4 | ACP over stdio (Oku Slides launches the agent); MCP over streamable HTTP, in the Oku Slides process, on `127.0.0.1`, token per launch. An agent without `mcpCapabilities.http` is rejected with a clear error. |
| 5 | What can be undone applies directly; one agent turn = one ⌘Z entry. What cannot be undone first asks the user for confirmation: the MCP server asks for it itself before running the tool; the agent's `session/request_permission` calls show in the same window. |
| 6 | Two families of surface tools: blocking `ask_*`, persistent `show_*`. A surface action that matches a known `Op` applies without going through the agent. |
| 7 | High-level tools: the agent describes the intent, the session composes the A2UI. Five forms: choice, confirmation, rewrite, parameters, variants. |
| 8 | One ACP session per document, not saved; "Nouvelle conversation" (New conversation) button. Automatic context attached to each prompt. |
| 9 | Hybrid interface: choices and rewrites in a card on the element, variants in a drawer below the stage, parameters and thread in the right panel. |
| 10 | Two new Rust crates: `oku-slides-mcp` (`rust-mcp-sdk` + `rust-mcp-axum`) and `oku-slides-agent` (`agent-client-protocol`). `oku-slides-session` stays synchronous. |
| 11 | A single `apply(ops)` tool, whose schema is that of `Request::Schema`. `set_setting` limited to an allowlist. |

## Architecture

```text
UI (React) ──invoke / HTTP──▶ Host (src-tauri | oku-slides serve)
   ▲  events (Tauri emit | SSE)          │
   │                                     ├─▶ oku-slides-agent ──stdio ACP──▶ agent (subprocess)
   │                                     │                                     │
   │                                     └─▶ oku-slides-mcp ◀──HTTP MCP────────┘
   │                                               │
   └──────────── surfaces, changes ◀──────── oku-slides-session (synchronous)
```

### `oku-slides-mcp` (new)

- `rust-mcp-axum` server (`create_axum_server`) on `127.0.0.1`, port
  chosen by the system, random 256-bit token per launch required in
  `Authorization: Bearer`. Without a token or with a wrong token: 401, nothing
  is executed.
- Holds an `Arc<Mutex<Session>>` shared with the host. A tool takes the
  lock for the duration of one `Session::handle`; never across an `.await`.
- After each change, notifies the host (`tokio::sync::broadcast` channel
  of `Changed`, `Surface`, `Focus` events).
- `ask_*`: creates a `oneshot`, pushes the surface, waits for the answer without
  holding the lock; the host's `surface_action` completes the `oneshot`.
- Non-undoable tools: the server itself asks for confirmation before
  executing, through the `PermissionGate` trait provided by the host; the host
  implements it with the confirmation form (same `oneshot` as
  `ask_confirm`). `session/request_permission` goes from the agent to the client and
  cannot be used here.
- Shutdown through the host's `CancellationToken`.

### `oku-slides-agent` (new)

- Lazy launch on the first ⌘⇧D: `[agent] command` / `args`.
  `initialize` with `clientCapabilities.fs` and `terminal` absent; checks
  `agentCapabilities.mcpCapabilities.http`.
- `session/new`: `cwd` = the document's folder (otherwise `paths.data`),
  `mcpServers = [{ type: "http", name: "oku-slides", url, headers:
  [{ name: "Authorization", value: "Bearer <jeton>" }] }]`. The token is
  never in the URL nor in the logs.
- One session per document: `New`, `Open` and "Nouvelle conversation"
  open a fresh one; the process stays alive.
- A turn: opens the undo group "Agent : <start of the instruction>",
  sends `session/prompt` with two text blocks (context, then instruction),
  relays `session/update` (`agent_message_chunk`, `tool_call`,
  `tool_call_update`, `plan`) to the thread, presents the agent's
  `session/request_permission` in the confirmation window
  (options provided by the agent), closes the group on
  `stopReason`. Escape or ⌘⇧D during a turn: `session/cancel`.
- Crash (stdout closed): localized error, group closed with what has
  been applied; the next ⌘⇧D relaunches the agent.
- First prompt of each session: role instructions (Fluent) — edit
  an Oku Slides presentation through the tools, prefer `ask_*` when the
  decision belongs to the user, answer in their language.

### `oku-slides-session` (extended)

- `History` (in `oku-slides-core`): open group. `begin_group(label)`,
  `end_group()`; the `commit`s of an open group merge into a single
  transaction; an empty group creates nothing. The group is serialized with
  the history (compatible with state resume, spec
  `2026-09-26-resume-state-design.md`).
- New module `forms.rs`: composes the five forms into A2UI messages.
- Variants: applies each variant's `ops` on a clone of the deck,
  compiles, renders the slide's SVG, discards the clone. An invalid variant
  makes the whole call fail.
- Word-by-word differences of the rewrite computed with `similar`.
- Automatic context: `agent_context()` returns the text block; actions
  done without the agent (variant picked, parameter set) accumulate in
  `since_last_turn` and are cleared after sending.

### `oku-slides-render` (extended)

- `render_slide` as PNG with `typst-render`, from the already
  compiled document.

### `oku-slides-a2ui` (extended)

- `CATALOG_ID` = `urn:oku-slides:a2ui:catalog:v2` (replaces
  `urn:echo:a2ui:catalog:panel:v1`, without alias).
- New components: `Button`, `ChoiceChips`, `Slider`, `ColorField`,
  `Select`, `SlideThumb` (SVG rendered by the session), `DiffText` (segments
  `keep | del | ins`). Described in zod in `ui/src/a2ui`.

### Hosts

- New host commands (outside `Session::handle`, which does not know about
  the agent): `agent_prompt { text }`, `agent_cancel`,
  `agent_new_conversation`, `surface_action { surface, action, context }`.
- `src-tauri`: `AppState` gains the agent and the MCP server; events
  go out through `emit` (`session-changed`, `agent-update`, `surface`).
- `oku-slides serve`: same wiring; events over SSE (`/api/events`).

### Settings

```toml
[agent]
command = "claude-code-acp"
args = []
```

`OKU_SLIDES_AGENT__COMMAND` and `OKU_SLIDES_AGENT__ARGS` override them.

## MCP tools

### Read (free)

| Tool | Role |
| --- | --- |
| `deck_outline` | Slides (id, kind, title, number of elements), theme, template set |
| `slide(id)` | Detailed slide, like `Request::Slide` |
| `render_slide(id)` | PNG image of the slide |
| `templates` | Sets and templates |
| `settings` | App settings (read) |

### Undoable changes (direct, in the turn's group)

| Tool | Role |
| --- | --- |
| `apply(ops)` | All `Op`s; input schema = `Request::Schema` |
| `chart_kind`, `chart_palette` | Existing chart requests |
| `edit_templates(set)`, `close_templates` | Templates mode (the deck is set aside) |
| `focus(slide, element?)` | Shows a slide or an element in the interface |

### Non-undoable (confirmation first, asked by the MCP server)

| Tool | The confirmation shows |
| --- | --- |
| `save(path?)`, `open(path)`, `new` | The full path; the unsaved changes |
| `save_templates(as_new)`, `delete_template_set(set)` | The set's name |
| `set_setting(key, value)` | Key, old and new value |

Paths: absolute, normalized, without `..`; rejected otherwise. Allowlist of
`set_setting`: `i18n.locale`, `stt.model`, `stt.language`, `stt.vad.*`,
`history.limit`. `[paths]` is out of reach.

### Surfaces

| Tool | Arguments | Where | Return |
| --- | --- | --- | --- |
| `ask_choice` | `question`, `options[{id, label, hint?}]`, `anchor?{slide, element}` | Card on the element, otherwise in the thread | `{choice}` · `{answer_text}` · `{cancelled}` |
| `ask_confirm` | `message`, `danger?` | Modal window | `{confirmed}` · `{cancelled}` |
| `ask_rewrite` | `slide`, `element`, `proposals[]` (2 to 3) | Card on the element, differences highlighted | `{kept: "original" \| index, text}`; Oku Slides applies the chosen `SetText` |
| `show_params` | `title`, `fields[{label, kind, value, op_template}]` | Sticky note in the right panel | Nothing; each change applies the `Op` |
| `show_variants` | `slide`, `variants[{label, ops}]` (2 to 4) | Drawer below the stage, rendered thumbnails | Nothing; the choice applies the `ops` |

- `kind`: `stepper | slider | color | select | text`.
- `op_template`: an `Op` with `"$value"` in place of the value; validated
  at call time (filled with the initial value, checked against the schema).
- `ask_*` does not expire. Escape, a new turn or the end of the conversation
  return `{cancelled: true}`. A ⌘⇧D while a card is waiting returns
  `{answer_text}` to that tool instead of opening a turn.
- `show_*` outlives the end of the turn: a sticky note until it is closed or its
  slide is deleted; a new `show_variants` replaces the drawer.
- A direct action outside a turn is its own ⌘Z entry, named after
  the surface ("Variante : image à gauche" (Variant: image on the left)).

## Automatic context

First block of each `session/prompt`, short and stable:

```text
<oku-slides-context>
mode: deck | templates(set=…)
locale: fr · theme: papier · template_set: essentiel
slide: s-4 (3/12) kind=content title="Nos objectifs"
selection: element=e-2 field=title editing=false
since_last_turn:
  - variante retenue : « image à gauche » (s-4)
  - paramètre : grille colonnes 12 → 8 (s-4)
</oku-slides-context>
```

The whole deck is never sent; the agent reads it through the tools.

## Interface

- **Mic**: ⌘D text (dark), ⌘⇧D agent (accent color, label
  "Agent").
- **Thread** (right panel): active agent and "Nouvelle conversation" at
  the top; spoken instruction; plan as checkboxes; one line per tool call; agent
  text streamed. `aria-live="polite"`. React state, not saved.
- **Card** (`ask_choice`, `ask_rewrite`): anchored below the element,
  mirrored in Arabic; `1`–`3` choose, `↵` confirm, `E` touch up, Escape
  abandon.
- **Drawer** (`show_variants`): below the stage; click or `1`–`4`; Escape
  closes.
- **Sticky note** (`show_params`): in the right panel; ✕ closes.
- **Modal window** (`ask_confirm`, permissions): full path and
  consequence; options provided by the agent.
- Cards and windows: `role="dialog"`, focus trapped then returned to the element.
- All strings through Fluent, 20 languages.

## Errors

`AgentError` and `McpError`: opaque Snafu, backtrace per variant,
`Localize` like `SessionError`. No `.unwrap()` nor `.expect()`.

| Case | User (Fluent) | Agent |
| --- | --- | --- |
| Command not found | "Agent introuvable : `<command>`. Réglage `[agent] command`." (Agent not found) | — |
| No `mcpCapabilities.http` | "Cet agent ne sait pas se connecter aux outils d'Oku Slides." (This agent cannot connect to Oku Slides' tools) | — |
| Agent stopped mid-turn | "L'agent s'est arrêté." (The agent stopped); group closed | — |
| MCP port unavailable | "Les outils de l'agent n'ont pas pu démarrer." (The agent's tools could not start) | — |
| Invalid `Op`, unknown id, invalid template | — | `isError: true`, stable English message with the faulty path |
| Permission denied | "✕ Refusé" (Denied) in the thread | `isError: true`, "denied by user" |
| Key outside the allowlist, path rejected | — | `isError: true` |
| Missing or wrong token | — | HTTP 401 |

Logs: `info` agent launch and stop; `debug` name and duration of
each tool (without arguments); `trace` the ACP stream.

## Tests

TDD; `_tests.rs` next to the code; speculoos; `insta` for snapshots.

- **`oku-slides-core`**: group → one entry; `undo` of the group restores
  everything; empty group → nothing; serialization of the group.
- **`oku-slides-session`**: five forms → A2UI (snapshots); variants
  on a clone without touching the deck; invalid variant → call fails;
  difference segments; context block and `since_last_turn`.
- **`oku-slides-render`**: PNG of a slide (dimensions, not empty).
- **`oku-slides-mcp`**: real server on `127.0.0.1:0` and `rust-mcp-sdk`
  client in the test; list of tools and schemas (snapshot);
  401 without a token; `apply` in the group; permission granted or denied
  (simulated gate); `ask_choice` blocks while `Session` stays
  lockable; key outside the allowlist rejected; path with `..` rejected.
- **`oku-slides-agent`**: fake ACP agent (`tests/bin/fake_agent.rs`,
  `agent-client-protocol`, JSON scenario); `initialize` without `fs` nor
  `terminal`; `mcpServers` with URL and header; context (snapshot);
  cancel; crash → error and group closed; agent without HTTP rejected.
- **UI (bun test)**: v2 catalog components; card / drawer /
  sticky note routing; mic mode; shortcuts.
- **End to end (`just e2e`)**: `OKU_SLIDES_AGENT__COMMAND` = fake agent,
  `OKU_SLIDES_STT__FAKE_MIC`; dictation → rewrite → choice → variant →
  ⌘Z undoes the turn.
- **Outside continuous integration**: `just agent-smoke` with the real
  `claude-code-acp` on a sample deck.

## Check first

- `rust-mcp-axum`: port `0` accepted and actual port readable back; token
  check through a minimal `AuthProvider` or an axum layer.
- `claude-code-acp` does announce `mcpCapabilities.http`.
- `typst-render` compatible with the workspace's Typst version.
