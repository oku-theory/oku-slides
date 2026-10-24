# Packs and themes as tabs, the workshop perspective removed

Task t-1152. Supersedes the workshop half of `2026-10-04-atelier-widget-scene-planche-design.md`
and all of `2026-10-08-sessions-par-perspective-design.md` (per-scope agent sessions).

Today a tab edits a deck, and entering the workshop (`Mode::Pack`) sets that deck aside
(`stash`) and shows a second bar of sub-tabs: "Layouts" (the scene, layout packs only),
"Thème", then one sub-tab per open widget. The agent keeps one conversation per
perspective (`AgentScope::{Slides, Atelier, Widget}`).

After this change there is no workshop. **A pack and a theme are documents opened in
their own window tab, like a deck**, each with a distinctive tab border. A pack tab is
symmetric with a deck tab: its items (widgets or layouts) fill the pile the way slides
do, the chosen item is shown on the stage with its tools.

## Decisions (made on 2026-10-24)

1. **Tab granularity: one tab per pack.** Its widgets or layouts are listed in the pile,
   symmetric with the slides of a deck; no sub-tabs.
2. **Themes get a third kind of tab**, with its own border.
3. **A pack tab is standalone**: a library document, not tied to the deck it was opened
   from. Decks receive its versions through the "Packs du deck" panel (`pack_updates`).
   A deck-only pack (import) opens in a tab too, and its saves go back into its deck.
4. **A version is frozen when a deck adopts it.** Until then it is a draft, replaced in
   place by each autosave; after, the next save opens a child version.
5. **One agent conversation per tab.** `AgentScope` disappears.
6. **Approach: one document kind per session**, fixed at creation. `Mode::Pack` loses
   its `stash`; `EditPack` and `ClosePack` disappear. Rejected: three session types
   behind a trait (large split of `Session` for a mostly aesthetic gain); a pack stored
   as a Loro deck (a second representation of params, schemas and examples to keep in
   sync, at odds with the no-text-loss round-trip invariant).

## Tab model and resume (`oku-slides-session`)

`TabInfo` (`views.rs`) replaces `help: Option<String>` and `editing_pack: bool` with:

```rust
/// What a tab edits; its kind decides the pile, the tools and the border.
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TabDocument {
    Deck { path: Option<PathBuf>, deck_uid: Option<String> },
    Help { locale: String },
    Pack { pack: Uuid },
    DeckPack { deck_tab: String, namespace: Namespace },
    Theme { theme: Uuid },
}
```

- A document opens once: opening a pack or theme already open focuses its tab (the rule
  `path` already follows for decks).
- `tabs/t-<n>/` holds, for a pack or theme tab, its working document (today's `pack/`)
  and `state.json`, which also keeps the draft version and the preview theme.
- `DeckPack` is the only linked tab: it names its owner deck tab. Closing that deck
  flushes the pack tab's autosave, then closes it.
- `TabView` exposes `document.kind` to the UI. The title is the deck file name or title,
  the pack name, or the theme name; `dirty` keeps its meaning.
- `workspace.json` layout version 3, `state.json` version 4. A tab saved in pack mode by
  an older version reopens on its deck (the stash becomes the document); the pack draft
  is dropped and a Fluent notice says so. No shim.

## Sessions by document kind

```rust
pub(crate) enum Mode {
    Deck,
    Help { locale: &'static str },
    History { stash: Box<Document>, history: HistoryMode }, // the only stash left
    Pack { edit: PackMode, scratch: PackScratch },
    Theme { theme: ThemeEdit, switches: Vec<ThemeSwitch> },
}
```

- `Session::open_pack(id)`, `Session::open_deck_pack(files, namespace)`,
  `Session::open_theme(id)` create a session of that kind; it never changes kind.
  Closing the tab closes the document.
- **Pack tab preview context.** The working deck is built from the pack, its `requires`
  and a **preview theme** chosen in the tab (default: the default theme of new decks),
  kept in `state.json`. Changing it does not dirty the pack. No user assets: an `asset`
  parameter stays absent, as `widget_preview.rs` already allows.
- **Theme tab.** Its pile holds **specimen slides**, one per layout of the built-in
  `essentiel` pack, filled with the existing localized examples (`layout_examples`,
  `example_text`) and rendered with the draft. `theme_edit`, `theme_draft` and
  `theme_extends` are reused as is, without the pack's working deck.
- The theme draft leaves `Mode::Pack`: editing a widget pack no longer edits the deck's
  theme.
- A request that makes no sense for the session's kind (a slide op on a theme, `SavePack`
  on a deck) returns `WrongDocument { kind }`, a Snafu variant with a backtrace and a
  Fluent message. No silent fallback.
- **Deck-only pack.** Its autosave produces the pack files; the workspace forwards them
  to the owner deck with `Request::ReceiveDeckPack`, one named undo step in the deck per
  autosave burst (merged by key, as `ADOPT_MERGE` does today). Locks: `Workspace`, then
  the pack slot, then the deck slot, in sequence, never nested; slow work outside.

## Versions and adoption (library)

- **`adopted` marker** in `versions/<hex>/`, next to `received`. Written whenever a deck
  embeds that version: an update accepted in "Packs du deck", a pack added from the add
  grid, a pin by the agent. Built-in and received versions count as frozen.
- **`Library::save_draft(id, draft, files) -> PackVersion`** is the only way a tab saves.
  Under a per-entry lock shared by every session of the process (through `PackLibrary`):
  draft not frozen → `replace_version`, then `set_current`; frozen → a new version whose
  parent is `draft`, then `set_current`.
- **`Library::adopt(id, v)`** takes the same lock and writes the marker; it is
  idempotent. An autosave after an adoption therefore always opens a new version; two
  decks adopting at once freeze the same version.
- The tab's draft (`PackMode::session_version`) is kept in `state.json`: after a restart
  the tab resumes the same draft unless it was frozen meanwhile.
- Undo acts on the tab's document; the next save follows the same rule. Content equal to
  an existing version is not duplicated (content addressing).
- Decks keep `refresh_pack_updates` (`Older` drift): the badge follows the current
  draft. The library index `bump` already notifies open tabs.
- Themes follow the same rule (`ThemeLibrary` is the same `Library<K>`): accepting a
  theme update in a deck freezes that theme version.
- "Save as copy" stays (agent tool `save_pack` with `as_new`, and the same for themes).

## Interface (`ui`)

**Tab bar (`TabBar.tsx`).** Each tab carries `data-kind`. Pack: a 3 px top border in
`--tab-pack` and a brick icon before the title (replaces the `tab-badge` "Pack" text).
Theme: `--tab-theme` and a palette icon. Deck and help unchanged. Both variables live in
`styles.css` with light and dark values; the active tab keeps its border with a light
tint. Drag, context menu and moving between windows work unchanged.

**Pack tab**, on the same skeleton as a deck tab:

- Pile (`PileColumn` / `SlidePile`): one `layout-<name>` slide per layout or one
  `widget-<name>` thumbnail per widget, as `pack_mode*` already produce. Pile gestures:
  add ("+ Widget" / "+ Layout"), rename, duplicate, delete, reorder, with the usual
  context menu and keys.
- Stage: the chosen item. Right: its tools, `WidgetPane` / `WidgetTools` (parameters,
  example values) and the Typst editor (`CodeSwitch`), as in today's widget sub-tab.
- Header: the pack name (`TitleField`) and the preview theme selector (`ThemeMenu`).

**Theme tab.** Pile of read-only specimen slides (selectable, not editable); stage shows
the chosen one; `ThemePanel` on the right (base, name, `extends`, tokens).

**Entry points** (each opens the tab, or focuses it if open):

1. "Packs du deck": an "Ouvrir" button on each pack and on the deck's theme; a deck-only
   pack opens a `DeckPack` tab.
2. A widget element selected on a slide: "Éditer le widget" (today's gesture at
   `App.tsx:428`) opens its pack's tab with that widget selected.
3. App menu: "Nouveau pack de widgets", "Nouveau pack de layouts", "Nouveau thème". The
   tab bar's `+` still creates a deck.

The kind of a tab is known before `App` mounts and never changes: each kind mounts its
own panels around the shared pile / stage / right-panel skeleton, instead of branching on
`snap.editing` throughout.

**Removed:** `PackTabs`, `widgetTabs`, `usePackMode`, `THEME_TAB`, the `packMode` branch
of `AppMenu`, `editing_pack`, the pack path of `close.ts`, most of `workshop.css` (what
remains joins the pile and stage styles).

## Agent

**Removed:** `AgentScope` (`agent/scope.rs`), `ScopeMove`, `take_scope_moves`,
`rename_scope` and its replay in `undo.rs`; the `scope` field of
`AgentCommand::{Prompt, NewConversation, Resync}` and of
`AgentEvent::{Status, UserText, Conversation}`; in the hub, `turn_scopes`, `viewing` and
the shown/hidden filter by scope (`hub_scopes.rs`): a surface is shown when it is live or
belongs to a turn of the thread; in the UI, `useAgentScope`, `agent/scope.ts` and the
cross-scope block ("L'agent répond dans les slides…"); the cleanup of orphan scopes at
resume. ⌘⇧D and Escape cancel the tab's turn.

**Kept, simplified.** The tab keeper (`keeper.rs`) stays the only path to conversation
writes (its queue order still settles a thread write against "Nouvelle conversation"); it
loses `Lookup(scope)` and renames, `Session { id, agent }` and `Forget` take no scope.
Resume uses one file, `tabs/t-<n>/agent/conversation.json`, with the same
`session/resume`, else `session/load`, else new session plus a card. Existing
`atelier.*.json` and `widget.*.json` files are set aside as `*.failed-…`.

**What the agent sees.** The context (`agent/context`) states the document kind, its
name, the selected item (as the current slide of a deck) and the preview theme. The MCP
tools are **filtered by kind when the tab's tool server starts** (the kind never changes,
so no `tools/list_changed`):

- deck: today's tools, without the workshop ones;
- pack: `widget_tools.rs` (create, edit, rename a widget, its code, parameters and
  examples), the layout tools, `save_pack` as a copy, reads and surfaces;
- theme: `edit_theme_extends`, token tools, save as a copy, reads and surfaces.

A deck's agent gets `open_pack` / `open_theme`, routed by the workspace: it opens the tab;
editing happens there, in that tab's conversation.

## Testing

Test first for every lot (RED, GREEN, REFACTOR), names `unit_scenario_result`, rstest
for tables, speculoos assertions, localized errors checked on the key plus a non-English
locale.

- Session: `open_pack_session_lists_items_as_slides`,
  `wrong_document_request_returns_typed_error` (rstest per kind),
  `theme_session_renders_specimens_with_draft`.
- Library: `save_draft_unadopted_replaces_in_place`,
  `save_draft_adopted_opens_child_version`,
  `adopt_then_save_concurrent_never_rewrites_adopted` (two threads),
  `adopt_is_idempotent`; the same for themes.
- Workspace: `open_pack_twice_focuses_existing_tab`,
  `deck_pack_tab_closes_with_owner_deck`,
  `deck_pack_save_reaches_owner_as_one_undo_step`,
  `resume_v2_pack_mode_tab_reopens_on_deck`.
- Agent and MCP: `tool_list_filtered_by_document_kind`; one thread per tab restored after
  restart (the `tests/stack.rs` scenarios, without scope).
- UI: `TabBar` renders `data-kind`; pack item pile; theme tab (`*.render.test.tsx`).
- e2e: `070-agent-scopes` deleted; `160-widgets`, `170-code-layout`, `180-widget-values`,
  `200-theme-editor`, `230-raccourci-theme`, `250-packs` rewritten on tabs (open from
  "Packs du deck", edit a widget, back to the deck, accept the update, the version is
  frozen).
- Regressions: `issue_0613_annuler_un_renommage_ramene_la_conversation` deleted (no
  scopes); `issue_0902_deck_only_workshop` rewritten on a `DeckPack` tab;
  `issue_1093_pack_save_conflict_writes_no_version` and `issue_1099_parent_seul_embarque`
  adapted to `save_draft`; `issue_0611` and `issue_0612` kept without scope.

## Documentation

In the same commits as the code: `ARCHITECTURE.md` of `oku-slides-session`,
`oku-slides-agent` and `oku-slides-mcp`; `ui/AGENTS.md`; the nearest `AGENTS.md` gets the rules
"a version is frozen by its adoption; only `save_draft` saves a draft" and loses any scope
rule. `2026-10-08-sessions-par-perspective-design.md` is marked superseded. The help
(`docs/aide`) describes pack and theme tabs instead of the workshop.

## Lots

Each lot ships alone with green tests.

1. **Library**: `adopted` marker, `save_draft`, `adopt`, wired into today's pack mode.
2. **Agent**: `AgentScope` removed, one thread per tab (the workshop, still there, shares
   the deck's thread).
3. **Session and workspace** (breaking): `TabDocument`, `open_pack`, `open_theme`,
   `DeckPack`; `Mode::Pack` without stash, `Mode::Theme`; `EditPack` / `ClosePack`
   removed; resume version 3.
4. **UI**: borders, item pile, theme tab, entry points; `PackTabs` and its helpers
   removed; e2e rewritten.
5. **MCP tools**: filtering by kind, `open_pack` / `open_theme`.

## Out of scope

- Splitting `Session` into one type per kind (a possible later refactor).
- A deck chosen as preview context for a pack (its assets, its theme): the preview theme
  selector covers the need for now.
- Editing a theme on a deck's real slides.
