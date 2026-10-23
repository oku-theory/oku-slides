# Typed packs, lot 5 — the packs UI: "Deck packs" panel, add grid from packs, editable `extends` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The deck's packs become visible and editable in one place.
- A "Deck packs" panel lists the deck's layout packs, widget packs and theme. From it you can add a pack from the library, remove one (a refusal says why) and move a pack to another version. It replaces the pack banner.
- The add grid lists the element widgets of every embedded widget pack, grouped by pack, and seeds each from the pack's examples.
- The theme editor edits the draft's `extends` list and shows the namespace of its base.

**Architecture:**
- **Panel**
  - The session builds a host A2UI surface `host-packs` (place `thread`, live) from a view.
  - The MCP hub shows it, as it shows the theme picker (`Shown::Packs`). It refreshes it after changes and routes its events to session methods.
  - It opens from a toolbar button (`AgentCommand::ShowPacks`) and from the MCP tool `show_packs`.
  - The UI already starts each tab's agent through `resync` when the agent panel mounts (`ui/src/agent/useAgentScope.ts:66-70` → `Stack::open`), so the hub exists without any extra start.
- **Refusals.** One core function decides whether a pack can be removed. It does not clone the deck, and both `Op::RemovePack` and the panel call it. The panel shows the refusal in place of the Remove button.
- **Version notices.** The relation of each pin to the library is shown per pack. A toolbar dot lights when the deck is older than the library. The banner and its requests are deleted.
- **Add grid**
  - The snapshot lists the add widgets grouped by pack, each with its seed. The seed is the pack's examples (localized for built-in packs), plus the manifest's new `styles`.
  - Which slides accept elements is read from the layouts' free slots.
- **Theme editor**
  - `ThemeEdit` gains its base namespace and its own `extends` list. Editing the list is one undo step, through the base-switch rank mechanism.
  - `host-theme` shows one row per parent (up, down and remove icons) plus an "Add a parent" select.
- **Formats.** Deck, replica, sources and worker formats are unchanged. The resume state moves from 2 to 3, and the A2UI catalog from v3 to v4. `base` and `essentiel` get new shipped versions.

**Tech Stack:** Rust (Snafu, serde, speculoos, rstest, insta), A2UI catalog (Rust `oku-slides-a2ui` + TS zod `ui/src/a2ui/catalog.tsx`), Fluent (20 locales), React/TS (Bun, happy-dom render tests), Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-10-18-packs-layouts-widgets-themes-design.md` (§2, §6, §9 item 5). Previous plans, for what exists and their deviations:
- `docs/superpowers/plans/2026-10-18-packs-lot1-widget-packs.md`
- `docs/superpowers/plans/2026-10-18-packs-lot2-base.md`
- `docs/superpowers/plans/2026-10-19-packs-lot3-essentiel.md`
- `docs/superpowers/plans/2026-10-22-packs-lot4-themes.md`

**Task:** parent t-1008; sub-tasks t-1110 … t-1117 (Task 1 … Task 8).

## Global Constraints

- **What lot 3 already delivered for slides.** `LayoutPicker.tsx` lists the layouts of every pinned layout pack (`layouts_view`, `pack_mode_previews.rs:50`), seeded from `examples`. This lot keeps it as it is; its examples become localized (below). The spec's "adding a slide lists layouts instead of templates" is done.
- **Removal check (core).** `oku_slides_core::remove_pack_refusal(deck: &Deck, kind: PackKind, ns: &Namespace) -> Option<OpError>`, in `ops/packs.rs`, re-exported.
  - It returns `BaseRequired`, `UnknownNamespace`, `PackRequired { by }` or `PackInUse { users }`, in the order `packs::remove` checks them today.
  - `packs::remove` calls it, so the panel and the op cannot disagree.
- **Panel surface**
  - Id `host-packs`, place `thread`, `live: true`.
  - The session builds it with `deck_packs_panel(view, l)`, where `l` is the hub's `LiveLocalizer::current()`. The view (`DeckPacksView`) holds no localized text: refusals are kept as `OpError`, relations as `Relation`.
  - Only the hub shows it (`Shown::Packs(PacksShown { last: Surface })`), alone, like `Shown::Themes`.
  - `refresh_packs` runs from `prune`. It returns at once when `host-packs` is not shown. It closes the panel outside deck mode, and otherwise rebuilds it, emitting only if the result differs from `last`.
  - It uses only existing catalog components: `Card`, `Column`, `Row`, `Text`, `Badge`, `Button`, `Select`.
  - UI dock: `host-packs` is exempt from `isHostSettings` and from the folding behind a pending question, like `HOST_THEME_PICKER` (`ui/src/agent/surfaces.ts:41-43`, `AgentDock.tsx:41`).
- **Panel sections, in order: Layouts, Widgets, Theme**
  - **Each pinned pack** is one card with:
    - its name, a version badge (the first 7 hex digits of the version), and its namespace as a caption;
    - for a library pack whose pin is not the library's current version, a relation line (`packs-relation-older`, `-newer`, `-parallel`);
    - "Use my version" (`packs-use-mine`, event `use_mine`), shown when the relation is set;
    - "Make theirs my version" (`packs-make-current`, event `make_current`), shown when the library holds the deck's version and the pack is not built in;
    - "Remove" (`packs-remove`, event `remove_pack`), or, when `remove_pack_refusal` returns an error, that error localized as a caption instead. The text comes from `OpError`'s own `Localize`, never from the `transaction-refused` wrapper.
  - **Deck-only packs** (scope `deck`: `deck`, `deck-layouts`) have no relation line and no version buttons.
  - **Each pack section** ends with an "Add from the library" select (`packs-add`, event `add_pack`).
    - It lists the library packs of that kind whose namespace the deck pins under neither kind, filtered by R15 (a built-in namespace belongs to the built-in pack).
    - It is absent when that list is empty.
  - **The Theme section** shows:
    - the theme's name, and its `extends` names as a caption;
    - the number of deck overrides (`packs-theme-overrides`, plural);
    - a select of the library themes (`packs-theme-pick`, event `theme`);
    - "Use my version" (event `theme_update`), shown when the pinned theme version is older than the library's current one.
- **Event contexts.** Every session method re-checks what the panel offers. A forged context is refused with a typed error, never trusted.

  | Event | Context keys | Session call | Session checks |
  |---|---|---|---|
  | `add_pack` | `kind` (`layouts`/`widgets`), `namespace` | `add_library_pack(kind, &ns)` | deck mode; namespace pinned under neither kind; R15 owner in the library; library scope |
  | `remove_pack` | `kind`, `namespace` | `remove_deck_pack(kind, &ns)` | deck mode; the op itself (`remove_pack_refusal`) |
  | `use_mine` | `kind`, `namespace` | `pack_use_mine(kind, &ns)` | deck mode; pinned; library pack; current version differs |
  | `make_current` | `kind`, `namespace` | `pack_make_current(kind, &ns)` | deck mode; the version comes from the deck's pin only; not deck-only; not built in; the library holds that version |
  | `theme` | `theme` (namespace) | `pick_theme(&ns)` | existing checks (deck mode, known theme) |
  | `theme_update` | none | `update_deck_theme()` | deck mode; the theme pin is older than the current version |
  | `dismiss` | none | closes the surface (the existing close button) | — |

  - The deck changes are one undo step each, labelled `packs-undo-add`, `packs-undo-remove`, `packs-undo-use-mine` and `packs-undo-theme-update`.
  - An `AddPack` goes through `Session::apply`, which prepends the missing requirements (`closure_ops`), all in the same step.
  - `pack_make_current` changes only the library: no undo step.
  - A malformed context makes `packs_event` return `None`. The hub logs it with `warn` and returns `Incomplete`, as `apply_theme` does.
- **Toolbar**
  - A "Deck packs" button (`ui-toolbar-packs`) in `ToolsTray`. It is shown in deck mode, but not in help (`read_only`) or history. It sends `{cmd: "show_packs", tab}`.
  - It has a dot, with a tooltip suffix `ui-toolbar-packs-update`, when `Snapshot.pack_updates > 0`.
  - `pack_updates` counts the pins (packs and theme) whose relation to the library is `Ancestor`, i.e. the deck is older. `Descendant` and `Sibling` show a line in the panel but no dot (user decision).
  - It is cached on the session: recomputed on open, after any applied op that touches pins (`AddPack`, `RemovePack`, `SetTheme`), after undo, redo and merge, and after `pack_make_current` and `save_pack`. It is never computed per snapshot.
- **Add grid**
  - The snapshot field `add_widgets` becomes `add_packs: Vec<AddPackView>`, with:
    - `AddPackView { namespace: String, name: String, widgets: Vec<AddWidget> }`;
    - `AddWidget { name: String, label: String, description: Option<String>, seed: ElementSeed }`, owned, no lifetime, no `Copy`.
  - Packs come in pin order (by namespace), and widgets in registry order.
  - In pack mode, `add_packs` has one section, the edited widget pack, seeded from its working manifest, as `pack_widget_names` does today.
  - `label` is `kind_name(name, schema label, l)`. It is always a string: `session-widget-kind-<ns>-<name>` for built-ins, else the schema label, else the kind.
  - `description` is the schema description, else `session-widget-description-<ns>-<name>` for built-in namespaces only. The seven `base` keys replace `ui-add-kind-description`.
  - `ElementSeed { params: Map<String, Value>, style: Option<Style> }`, with `#[serde(skip_serializing_if = "Option::is_none")]` on `style`. TS: `style?: Style`.
    - `params` are the pack's `examples[name]`, localized, minus the values the schema rejects (`trial_fits`).
    - `style` is the manifest's `styles[name]`, dropped if empty.
    - Seeds are cached per (pack version, locale) in the session (`seed_cache`), so a snapshot does no manifest parsing for packs it has seen.
  - The UI builds `{id, kind, params: seed.params, style?}` and adds it with `add_element { defaults: true }`, as today. A new `base/text` from the grid opens in edit mode, as today (`useSlideOps.ts:49-50`).
  - Paste keeps its own path: `addElement(kind, seed, { text, label })`. Pasted text replaces the seed's `body`, under the label `ui-undo-paste`.
  - Deleted:
    - TS: `ADD_NAMES`, `AddName`, `packOnlyWidgets`, `newElement`'s per-name switch, `newPackWidget`, `starterChart` (`chart/chartDeck.ts:25`), `starterTable` (`table/tableDeck.ts:46`), the native `Icon` previews (`Icon.text`, `Icon.card`, `Icon.shape` if unread elsewhere) and `WidgetChoice.pick`'s `native` arm;
    - Fluent: `ui-add-kind`, `ui-add-kind-description`, `ui-new-text`, `ui-new-card-title`, `ui-new-card-detail`, `ui-undo-add`.
    - The undo label of every grid add is `ui-undo-add-widget { $name }` (fr « Ajouter « Graphique » »).
  - The grid also shows `base/image` (it renders its placeholder until an asset is set in the target panel) and `base/stack` (empty), because the spec lists every element widget.
- **Which slides accept elements**
  - `Snapshot.free_slots: BTreeMap<String, Option<Vec<String>>>` maps each pinned layout kind to its first `p.slot(mode: "free")` param's `accepts`, or `None`. A layout without a free slot is absent. `add_element` already targets that first free slot.
  - TS:
    - `acceptsElements(slideKind, freeSlots)` = `slideKind in freeSlots`;
    - `canAdd(slideKind, kind, freeSlots)` = `acceptsElements` and (`accepts` is `null` or contains `kind`).
  - The "no text on `essentiel/blank`" exception is deleted (user decision).
- **Localized examples (built-in packs only)**
  - In a manifest whose `id` is a built-in pack id (`BuiltinPack::ALL`), a JSON string `"ftl:<key>"` is replaced when `<key>` starts with `session-example-` and the key exists (`Localizer::try_tr`).
  - A marker whose key is missing stays verbatim, with a `warn` log. An i18n test asserts that every marker in the shipped manifests has its key in all 20 locales.
  - A manifest that is not built in keeps every string verbatim.
  - This lives in one function, `localized_examples(manifest_id: Uuid, values: &Map<String, Value>, l: &Localizer) -> Map<String, Value>` (`crates/oku-slides-session/src/example_text.rs`), used by:
    - `example_values` (`widget_examples.rs:63`);
    - `example_slide` and `layout_slides` (`pack_mode.rs:64-77`);
    - `realign_layouts` (`pack_mode.rs:94`);
    - `preview_slide` (`pack_mode_preview.rs:34`);
    - `preview_inputs` (`widget_preview.rs:323`);
    - the seeds.
  - Saving keeps markers where nothing changed:
    - `write_examples` rewrites only the edited widget's entry.
    - Saving a built-in pack as a copy (new id and namespace, `pack_mode_save.rs:85-98`) resolves every marker into plain text in the current locale, because the copy is no longer built in.
  - `base/widgets.json` gains `examples`:

    | Widget | Values |
    |---|---|
    | `text` | `body` = `[{"runs": [{"t": "ftl:session-example-base-text-body"}]}]` (en "New text", fr "Nouveau texte") |
    | `card` | `title` = `ftl:session-example-base-card-title` ("Card title" / "Titre de la carte"); `body` = one paragraph `ftl:session-example-base-card-body` ("Detail" / "Détail") |
    | `chart` | `option` = `{"xAxis": {"type": "category", "data": ["1","2","3"]}, "series": [{"type": "bar", "data": [3,5,2]}]}` |
    | `table` | `data` = `{"dataset": {"source": [[h1,h2,h3], ["",0,0], ["",0,0]]}}`, `hN` = `ftl:session-example-base-table-column-N` ("Column N" / "Colonne N") |

    The English and French texts are today's `ui-new-*` and `ui-table-column` values; the other 18 locales move with them.
  - `base/widgets.json` also gains `"styles": {"shape": {"fill": "surface", "radius": "md"}}`.
  - `essentiel/layouts.json`: every example string becomes `ftl:session-example-essentiel-<layout>-<param>[-<n>]`. The fr values are today's French strings verbatim; en and the other locales are translations.
  - New shipped versions:
    - `base` and `essentiel` each set `parent` to their previous shipped version, so the previous version's relation to the new one is `Ancestor`.
    - Each appends one line to its `versions.txt` (computed as `builtin_packs.rs:76` reads it).
    - `builtin_packs_tests.rs:219` expects 3 versions.
    - No golden page changes: the golden deck reads no examples.
- **Theme editor**
  - `ThemeEdit { base: ThemePin, namespace: Namespace, name: String, extends: Vec<Namespace> }`.
    - `namespace` is the base's namespace, read when the edit is created (`initial_theme_edit`, `set_edited_theme`, from the manifest they already load).
    - `extends` starts as `[namespace]` for a child of a built-in, and as `base.extends` otherwise. `ThemeEdit::starting_extends(&self, base: &ThemeManifest) -> Vec<Namespace>` computes that start.
  - Rendering the draft (`draft_chain`):
    - For a user theme (not a child), it uses `closure_from(&namespace, (ThemePin { extends: edit.extends.clone(), ..base }, files))` and `chain_tokens(…, with_root = false)`. A parent whose closure reaches the draft's own namespace is a `ThemeCycle`, detected by the existing `closure_with`.
    - For a child, the draft has no namespace yet. A new `closure_of_parents(parents: &[Namespace], find) -> Result<Closure>` in `theme_closure.rs` roots the closure at a synthetic pin under the reserved namespace `deck` (no theme can hold it), linearizes, then drops the root. Cycle detection and order stay those of `closure_with` and `linearize`.
    - An empty list is allowed: the draft renders over the defaults and the packs' component tokens.
  - Saving (`saved_manifest`) writes `extends: edit.extends.clone()` in both branches. Its "nothing changed" early return also requires `edit.extends == starting_extends`. `theme_dirty` uses the same comparison.
  - `Session::set_edited_theme_extends(Vec<Namespace>) -> Result<()>`. In order, cheap checks first:
    1. pack mode (`NotEditingPack`);
    2. length ≤ the number of library themes, and no duplicate (`ThemeExtendsDuplicate { theme }`, new);
    3. each namespace is a library theme (`UnknownTheme`);
    4. for a non-child, not the draft's own namespace, and no cycle through the closure (`ThemeCycle { chain }`).

    An accepted change is one undo step, labelled `session-undo-theme-extends`. It sets the edit before `apply`, writes a new rank in `DRAFT_SWITCH_VAR` with no token change, restores the edit if `apply` fails, and pushes `ThemeSwitch { from, to }`. Undo and redo bring the list back through `realign_theme_edit`.
  - `Session::edit_theme_extends(ExtendsEdit) -> Result<()>`, where `ExtendsEdit` is `Add { theme }`, `Remove { theme }`, `Up { theme }` or `Down { theme }`. It applies the delta to the session's current list, then calls `set_edited_theme_extends`, so two quick clicks never overwrite each other. Moving the first parent up, or the last down, is refused (`ThemeExtendsMove`, new).
  - `ThemeEditView { base, namespace: String, name, child, extends: Vec<ThemeParentView>, dirty, revision }`, with `ThemeParentView { namespace: String, name: String }`. A parent missing from every source shows its namespace as its name, with a `warn` log (today's behaviour, `theme_draft.rs:79-86`).
  - `host-theme` header (`theme_panel.rs:276-293`): the `extends` caption is replaced by:
    - a title `theme-extends-title`;
    - one `Row` per parent: its name, then icon buttons `up`, `down` and `remove` (labels `theme-extends-up`, `-down`, `-remove`, event `extends`, context `edit`: `up`/`down`/`remove`, `theme`: its namespace). The first row has no `up` and the last no `down`.
    - a `Select` `theme-extends-add` (value path `/extends_add`, event `extends`, context `edit`: `add`, `theme`: path `/extends_add`), listing the library themes that are not in the list and are not the draft's own namespace.
  - Refusals reach the user as the toast `run()` already shows (the localized error; `session-theme-cycle` names the chain).
  - On resume, `ThemeEdit` goes through the checks of `set_edited_theme_extends`. A list that fails them (the library changed) makes the pack-mode resume fail and be set aside with the existing notice, never silently repaired.
- **A2UI catalog**
  - `ButtonIcon` gains `Up`, `Down` and `Remove` in Rust. The zod enum (`catalog.tsx:79`) becomes `["close", "up", "down", "remove"]`, with no `.catch`. `catalog.tsx:343` renders `Icon[props.icon]`, so `ui/src/icons.tsx` gains entries named exactly `up`, `down` and `remove`.
  - `CATALOG_ID` becomes `urn:oku-slides:a2ui:catalog:v4` in Rust and in TS, in the same commit (Task 4). No v3 is kept. Every v3 literal changes with it:
    - `ui/src/a2ui/fixtures/agentPicker.ts:12`;
    - `ui/src/a2ui/fixtures/agent-surfaces.json`;
    - the insta snapshots under `crates/oku-slides-agent/src/snapshots/` and `crates/oku-slides-session/src/agent/snapshots/` (review with `cargo insta review`; only the catalog id may change);
    - `crates/oku-slides-a2ui/ARCHITECTURE.md:4`.
  - `Place::Banner` is deleted in Task 4, Rust and TS together:
    - `crates/oku-slides-session/src/agent/forms.rs:34-35`;
    - `ui/src/api.ts:682`;
    - `ui/src/agent/surfaces.ts:104`;
    - `ui/src/agent/surfaces.test.ts:78-79`;
    - the `banner` fixture `agent-surfaces.json:1535-1600`.
- **MCP**
  - New tools:
    - `add_pack {kind, namespace}` (deck mode) → `add_library_pack`;
    - `remove_pack {kind, namespace}` (deck mode) → `remove_deck_pack`;
    - `show_packs {}` (deck mode) → `hub.show_packs()`;
    - `set_theme_extends {extends: [namespace…]}` (pack mode) → `set_edited_theme_extends`.
  - They live in `edit_tools.rs`, next to the other undoable edits. They run through `with_session`/`spawn_blocking`, report errors through `Tools::fail_with` with the localized text (the `OpError`'s own text for refusals), and call `prune_surfaces` after a change, as `edit_tools.rs:82-92` does.
  - `use_mine` and `make_current` are not agent tools.
  - `deck_outline.edited_theme` gains `namespace`, and `extends` becomes `[{namespace, name}]` (`read_tools.rs:56-63`). The tool description changes to match (`tools.rs:254`).
  - The `apply` tool keeps documenting the `add_pack` and `remove_pack` ops.
  - `crates/oku-slides-agent/src/instructions/agent.md` names `show_packs`, `add_pack`, `remove_pack` and `set_theme_extends`. `crates/oku-slides-local-agent/src/families.rs` routes them.
- **Requests (`crates/oku-slides-session/src/request.rs`)**
  - New: `SetEditedThemeExtends { extends: Vec<Namespace> }` and `EditThemeExtends { edit: ExtendsEdit }`.
    - Both go in `edits()` (`:366`), in `changes_deck()` (`:289`) and in the "apply writes its own resume state" exclusion next to `SetEditedTheme`.
  - Deleted: `PackUseMine`, `PackMakeCurrent`, `PackNoticeClose`, at every match site (`:218-222,309,335-336,366,416-417,426,453-454,558-560`).
  - `Interface` (language) and `SavePack` are added to `changes_deck()` if they are not there, so `prune` refreshes the panel after them.
  - The UI mirror (`ui/src/api.ts`) follows in Task 6. Between Tasks 2–5 and Task 6 the TS side lags, which is allowed only because the branch is squash-merged.
- **Format bumps**
  - Resume state `VERSION` 2 → 3 (`crates/oku-slides-session/src/resume.rs:36`; user decision). An older state is set aside with the existing notice (no migration, precedent t-532).
  - `DECK_VERSION` 9, `REPLICA_FORMAT` 7, `SOURCES_VERSION` 13 and worker `PROTOCOL_VERSION` 5 are unchanged.
- **Kind and namespace literals**
  - `ui/src/builtinKinds.ts` keeps `BASE` and `baseName`: `stageFields.ts:42` and `useClipboard.ts:95` still read them, and paste creates `BASE.text`. It deletes `ESSENTIEL` and `ESSENTIEL_NAMESPACE` if `slideTools.ts` and `packOps.ts` no longer read them.
  - No other TS file names a `base/*` kind outside tests. The Rust rules of lots 2–4 still hold.
- **Errors and code rules**
  - No `unwrap`/`expect` outside tests, and no silent fallback.
  - Snafu errors with `backtrace: Backtrace` in every variant: `ThemeExtendsDuplicate`, `ThemeExtendsMove`, `PackNotMakeCurrent`, import `GeneratedRegistry`.
  - Every new, renamed or deleted Fluent key changes in all 20 locales (`crates/oku-slides-i18n/src/locales/*/`), at the same position, translated.
  - Hub and MCP texts use `LiveLocalizer::current()` at each use.
  - Doc first sentences stay under 15 words; code comments are in French.
- **Tests**
  - Rust unit tests go in a sibling `x_tests.rs`, use speculoos, are named `unit_scenario_result`, and use rstest for tables.
  - Regressions go in `tests/regressions/issue_NNNN_topic.rs`, declared in that crate's `tests/regressions/main.rs` (NNNN = the ticket or sub-task number). A regression is first seen failing for its stated reason.
  - A localized error is tested on its kind or variant, **and** by asserting that its fr text differs from its en text (the localizer falls back to English, so "does not contain the key" proves nothing).
  - Session helpers to reuse:
    - `test_support.rs`: `session()`, `ns()`, `manifest_ns()`, `files_of()`, `add_version()`, `library_of()`, `adopt_deck_only_pack()`, `pin_builtin_theme()`;
    - `lib_tests_support.rs`: `session_with_fixture()`, `session_in()`;
    - `theme_edit_tests.rs`: `session_with_theme()`, `add_user_theme()`, `set_accent()`, `theme_edit()`, `rendered_token()`;
    - `pack_notice_tests.rs`: `opened_on_newer()` and `files()`, which move to `deck_packs_tests.rs`.
  - MCP tests are integration tests in `crates/oku-slides-mcp/tests/`, with `common::start()`, `h.hub.act(action(id, name, ctx))`, `h.hub.subscribe()`, `next_surface(&mut rx)` and `h.hub.surfaces()` (see `tests/themes.rs:39-60`).
  - UI render tests are `*.render.test.tsx`; other UI tests are `*.test.ts(x)`, importing from `bun:test`. e2e specs are `ui/e2e/NNN-name.cjs`, they read French labels, and helpers go in `ui/e2e/lib/`.
- **Docs**
  - Every pub item and field is documented. Source files stay under 500 lines when possible, never above 1000.
  - New `impl Session` code goes in its own module, not in `lib.rs`.
  - Each `ARCHITECTURE.md` and `AGENTS.md` change lands in the task that changes the code it describes, ≤ 50 lines; replace lines, never append past the cap.
- **Commits**
  - Format `type(scope) : description (t-N)`, message in a scratchpad file passed with `git commit -F` (a file name without "plan"), **no `Co-Authored-By` line**.
  - SDD protocol:
    - `git config --worktree oku.review deferred` during execution, removed at the end;
    - implementers only stage and write `<workspace>/task-N-commit.txt`;
    - the controller commits, with the trailer `Review-waived: revue SDD par tâche et revue finale (oku.review=deferred, t-1008)`.
  - The squash commit carries a `BREAKING CHANGE:` footer naming:
    - resume state version 3 (an older state is set aside);
    - A2UI catalog v4 (`ButtonIcon` up/down/remove) and `Place::Banner` removed;
    - the pack banner removed: the requests `pack_use_mine`, `pack_make_current`, `pack_notice_close` and `Snapshot.pack_notice`, replaced by the `host-packs` panel, `Snapshot.pack_updates` and `{cmd: "show_packs"}` on the agent channel;
    - `Snapshot.add_widgets` replaced by `add_packs` (`AddWidget.label` always a string, plus `seed`), and `free_slots` added;
    - the requests `set_edited_theme_extends` and `edit_theme_extends`;
    - `ThemeEditView.namespace`, and `extends` as `{namespace, name}`;
    - the MCP tools `add_pack`, `remove_pack`, `show_packs`, `set_theme_extends`, and the `deck_outline.edited_theme` shape;
    - the manifest key `styles` and the `ftl:` markers of built-in packs;
    - new shipped versions of `base` and `essentiel` (older decks show the update dot);
    - behaviour changes: text can be added to `essentiel/blank`; the grid shows `base/image` and `base/stack`; layout examples follow the UI language;
    - import: `hifi::project` returns a `Result`.
- Runs outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir.

## Deviations from the spec (approved with this plan; spec §2, §6 and §9 updated in the same commit)

1. **The panel is a host A2UI surface shown by the hub** (user decision, after weighing a session-request delivery).
   - It is `host-packs`, place `thread`, built by the session from a text-free view and localized by the hub.
   - It opens from a toolbar button (`AgentCommand::ShowPacks`) and from `show_packs`, follows the detached agent window, and is visible to the agent.
   - Known limit: a library change made in another tab shows here at this tab's next change.
2. **The banner is deleted; version notices live in the panel** (user decision).
   - Every library pin shows its relation. A dot lights only when the deck is older than the library.
   - `receive_packs` stays, moved to `pack_receive.rs`. Everything else in `pack_notice.rs` goes.
3. **One core check decides removals**: `remove_pack_refusal`, used by the op and by the panel. `PackKindConflict` concerns adding; the session refuses it when a forged context asks for it.
4. **Seeds travel in the snapshot; built-in examples are localized** (user decision on localization).
   - Each `AddWidget` carries its seed, so adding needs no round trip and cannot race with a slide change.
   - `essentiel`'s French example strings move to markers too.
   - Markers are resolved only for built-in pack ids, and are written out as plain text in a copy.
5. **A widget pack manifest gains `styles`** (`{<widget>: <element style>}`, read leniently like `examples`, kept by pack-mode saves, renamed and removed with its widget). Only `base/shape` uses it.
6. **Slides that accept elements are read from the layouts' free slots**, replacing constants. The "no text on a blank slide" exception goes (user decision).
7. **`extends` is edited on the draft** (t-1103; the user chose icons, catalog v4 and the resume bump).
   - `ThemeEdit` holds its base namespace and its own list. Edits are deltas, applied by the session.
   - A child renders over its list through a synthetic root. Lot 4 rendered it over its built-in, which is the list `[base]`.
8. **MCP gains four tools** (user decision): `add_pack`, `remove_pack`, `show_packs` and `set_theme_extends`.
9. **Folded tickets** (user decision: all inherited tickets except t-1053):

   | Ticket | Task |
   |---|---|
   | t-1103 | 4 |
   | t-1107 | 4 (view), 5 (MCP), 6 (menu) |
   | t-1104, t-1105, t-1092, t-1057 point 2 | 1 |
   | t-1106 | 1 (ops tests split), 3 (deck-mode validation, label fallback, `theme_edit_tests.rs` split) |
   | t-1091, t-1093, t-1109 | 3 |
   | t-1108 | 7 |

   t-1053 stays open.
10. **Intermediate commits.**
    - Tasks 1–4 go bottom-up. Each task's gate is `cargo check --workspace --all-targets` plus the tests of the crates it touches.
    - The TS side catches up in Task 6, except for the catalog and `Place` changes, which land in Task 4 in both languages.
    - `just ci` and `just e2e` run in Task 8. The branch is squash-merged.

---

### Task 1: Core, render and import — removal check, `styles`, trusted packs by content, carried fixes

Sub-task: t-1110.

**Files:**
- `crates/oku-slides-core/src/`:
  - `ops/packs.rs`: `pub fn remove_pack_refusal(deck: &Deck, kind: PackKind, ns: &Namespace) -> Option<OpError>` (doc: « Pourquoi le pack `ns` ne peut pas quitter le deck ; `None` s'il le peut. »). `remove` calls it. Re-export from `lib.rs`.
  - `pack.rs:391-402`: next to `examples`, in the same multi-line attribute style:
    ```rust
    /// Style de départ des widgets ajoutés depuis la grille, par widget.
    #[serde(
        default,
        deserialize_with = "crate::pack_examples::lenient_styles",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub styles: BTreeMap<WidgetFile, Style>,
    ```
    A manifest without `styles` re-serializes byte for byte as before, so existing versions keep their fingerprint.
  - `pack_examples.rs`: `lenient_styles`. An entry that does not parse as a `Style` is dropped with `diagnose(widget, …)`, as a bad example is.
  - `error/token.rs` (t-1104): `impl Localize for PackManifestError`. `Json` → new key `pack-manifest-json { $reason }` in `core.ftl`; `ManifestTokens` delegates to `TokenEntryError::localize` (`pack-token-entry-invalid`).
  - `sources.rs:169-172` (t-1104): an invalid `tokens` entry of a pinned pack reports `issue-pack-tokens-invalid { $pack, $reason }` (new key) instead of `issue-pack-pin-mismatch`.
  - `tokens_tests.rs` (t-1105): rstest `merge_two_peers_is_commutative` over disjoint paths, a shared path (documented order) and a prefix overlap (`a` vs `a.b`).
  - Every `PackManifest { … }` literal gains `styles` (`PackManifest` has no `Default`):
    - production: `crates/oku-slides-session/src/pack_mode_save.rs:85-98` (add `styles` to the kept tuple at `:85-86`) and `crates/oku-slides-import/src/hifi/pack.rs:239`;
    - tests: `crates/oku-slides-session/src/test_support.rs:71,106`, `library_namespace_tests.rs:126`, session regressions `issue_1015*`, `issue_1068_legacy_essentiel_copy`, `issue_0329*`, `crates/oku-slides-core/src/pack_tests.rs:283`, `crates/oku-slides-cli/tests/cli.rs:673`, `pack_broken.rs:18`, `regressions/issue_0355*:27`.

    Find any others with `rg -n 'PackManifest \{' crates`.
  - `crates/oku-slides-session/src/widgets_edit.rs:294,361,388-389`: removing or renaming a widget does the same to its `styles` entry as to its `examples` entry.
- Split `crates/oku-slides-core/tests/ops.rs` (989 lines, t-1106) by concept into `tests/ops/{main,elements,slides,packs,tokens}.rs`. Themes stay in the existing `tests/theme_ops.rs`. Every test is moved, none is changed.
- `crates/oku-slides-render/src/registry.rs:35-60` (t-1057 point 2): `PackWidgets::from_files(namespace: Namespace, files: &PackFiles) -> Result<Self, RegistryError>`, whose id is `files.version().hex()` and cannot be chosen. Use it at the trust-relevant constructions:
  - `crates/oku-slides-render/src/builtin_packs.rs:112`;
  - `crates/oku-slides-session/src/deck_widgets.rs:147`;
  - `crates/oku-slides-session/src/probed_widgets.rs:211`.

  `is_trusted_pack` (`builtin_packs.rs:145`) keeps its cheap comparison; its doc says the id is content-derived by `from_files`. `PackWidgets::new` stays for trials and tests, and its doc says such packs are never trusted unless built by `from_files`.
- `crates/oku-slides-import/src/hifi/mod.rs:90,107-110` (t-1092):
  - `project` returns `Result<ImportPlan, ImportError>`, with a new variant `GeneratedRegistry { source: WidgetError, backtrace }`. Its key is `import-generated-registry` in `document.ftl`.
  - The `debug_assert!` and the fallback are deleted.
  - Update the caller `crates/oku-slides-session/src/import/hifi.rs:90` and the import tests that call `project`.
- i18n: the new keys in all 20 locales.

**Interfaces:**
- Produces:
  - `remove_pack_refusal`, for Task 3;
  - `PackManifest.styles: BTreeMap<WidgetFile, Style>`, for Task 2;
  - `PackWidgets::from_files`.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-core/src/ops/packs_tests.rs (create, declared in packs.rs)
#[rstest]
#[case::base("base", "BaseRequired")]
#[case::in_use("acme", "PackInUse")]
#[case::required("dep", "PackRequired")]
fn remove_pack_refusal_names_the_reason(#[case] pack: &str, #[case] expected: &str) {
    let deck = deck_with_acme_in_use_and_dep_required_by_acme();
    let err = remove_pack_refusal(&deck, PackKind::Widgets, &ns(pack)).expect("refus attendu");
    assert_that!(format!("{:?}", err.kind())).contains(expected);
}

#[test]
fn remove_pack_refusal_unused_pack_is_none() {
    let deck = deck_with_unused("solo");
    assert_that!(remove_pack_refusal(&deck, PackKind::Widgets, &ns("solo"))).is_none();
}

// crates/oku-slides-core/src/pack_tests.rs
#[test]
fn manifest_without_styles_keeps_its_version() {
    let before = PackFiles::from_parts(manifest_json_without_styles(), widgets()).unwrap();
    let reread = PackFiles::from_parts(before.manifest_bytes().to_vec(), widgets()).unwrap();
    assert_that!(reread.version()).is_equal_to(before.version());
}

#[test]
fn manifest_styles_bad_entry_is_dropped_others_kept() {
    let files = files_with_manifest(r#"{…,"styles":{"shape":{"fill":"surface","radius":"md"},"bad":42}}"#);
    let m = PackManifest::from_files(&files).unwrap();
    assert_that!(m.styles.keys().map(WidgetFile::as_str).collect::<Vec<_>>()).is_equal_to(vec!["shape"]);
}
```

Build `PackFiles` the way the neighbouring tests of `pack_tests.rs` do (`rg -n 'fn files|from_parts|PackFiles::' crates/oku-slides-core/src/pack_tests.rs`), and replace `from_parts`/`manifest_bytes` in the sketch with those constructors.

Regressions:
- `crates/oku-slides-core/tests/regressions/issue_1104_manifest_errors_are_localized.rs`:
  - a `PackFiles` whose `widgets.json` is `{` gives `kind() == PackManifestErrorKind::Json`, and its fr text differs from its en text;
  - a pinned pack with an invalid `tokens` entry gives `issue-pack-tokens-invalid`.
- `crates/oku-slides-render/tests/regressions/issue_1057_trust_follows_content.rs`: `PackWidgets::from_files` on the shipped `base` files with one widget source changed is not trusted, while the shipped files are.
- `crates/oku-slides-import/tests/regressions/issue_1092_generated_registry_error.rs`: `hifi::project` with a `target.registry` holding an unqualified widget name (which makes `probe_name` fail) returns `ImportErrorKind::GeneratedRegistry`. Today it panics on the `debug_assert!`.

Create each crate's `tests/regressions/main.rs` if absent.

- [ ] **Step 2: Run them to see them fail.** Run `cargo nextest run -p oku-slides-core -p oku-slides-render -p oku-slides-import`. Expected: FAIL (missing items; the hi-fi regression panics).

- [ ] **Step 3: Implement** per the file list. Split `tests/ops.rs` as a pure move: `git diff --stat` shows only deletions there and additions in `tests/ops/`.

- [ ] **Step 4: Run the gate.** Run `cargo check --workspace --all-targets`, `cargo nextest run -p oku-slides-core -p oku-slides-render -p oku-slides-import -p oku-slides-i18n` and `cargo clippy --workspace --all-targets -- -D warnings`. Expected: PASS. The golden snapshots are unchanged.

- [ ] **Step 5: Stage and write `task-1-commit.txt`**: `fix(packs) : refus de retrait en un seul contrôle, styles dans les manifestes, confiance par empreinte, erreurs de manifeste localisées, découpe des tests d'ops (t-1110, t-1104, t-1105, t-1106, t-1092, t-1057)`.

---

### Task 2: Session — localized examples, seeds, add grid grouped by pack, free slots

Sub-task: t-1111.

**Files:**
- Create `crates/oku-slides-session/src/example_text.rs` (+ `example_text_tests.rs`):
  ```rust
  //! Textes d'exemple traduits des packs intégrés : marqueurs `ftl:` de leurs exemples.

  /// Préfixe d'un texte d'exemple traduit.
  const FTL: &str = "ftl:";
  /// Seules ces clés se traduisent : un pack ne lit aucun autre texte de l'app.
  const KEY_PREFIX: &str = "session-example-";

  /// Les exemples `values` du pack `manifest_id`, traduits s'il est intégré.
  pub(crate) fn localized_examples(manifest_id: Uuid, values: &Map<String, Value>, l: &Localizer) -> Map<String, Value>

  /// Les exemples `values` aux marqueurs écrits en clair, pour une copie qui n'est plus intégrée.
  pub(crate) fn resolved_examples(values: &Map<String, Value>, l: &Localizer) -> Map<String, Value>
  ```
  - The built-in check is `oku_slides_render::BuiltinPack::ALL` ids (`rg -n 'pub fn id' crates/oku-slides-render/src/builtin_packs.rs`).
  - The walk is recursive over objects and arrays. The input always comes from `serde_json` (depth limit 128).
- Create `crates/oku-slides-session/src/add_grid.rs` (+ `add_grid_tests.rs`), holding everything the add grid needs, out of `lib.rs`:
  ```rust
  /// Le contenu de départ d'un élément ajouté depuis la grille.
  #[derive(Debug, Clone, PartialEq, Serialize)]
  pub struct ElementSeed {
      /// Les valeurs d'exemple du pack, traduites, que le schéma admet.
      pub params: Map<String, Value>,
      /// Le style de départ du manifeste (`styles`), s'il en a un.
      #[serde(skip_serializing_if = "Option::is_none")]
      pub style: Option<Style>,
  }
  /// Un widget d'élément à ajouter : kind, nom montré, description et départ.
  #[derive(Debug, Clone, PartialEq, Serialize)]
  pub struct AddWidget { pub name: String, pub label: String, pub description: Option<String>, pub seed: ElementSeed }
  /// Les widgets à ajouter d'un pack de widgets épinglé.
  #[derive(Debug, Clone, PartialEq, Serialize)]
  pub struct AddPackView { pub namespace: String, pub name: String, pub widgets: Vec<AddWidget> }
  impl Session {
      /// Les packs de la grille d'ajout, dans l'ordre des épinglages.
      pub(crate) fn add_packs(&self) -> Vec<AddPackView>
      /// Le premier slot libre de chaque layout épinglé, et ce qu'il accepte.
      pub(crate) fn free_slots(&self) -> BTreeMap<String, Option<Vec<String>>>
      /// Le départ du widget d'élément `kind` (exemples traduits, style).
      pub(crate) fn seed_element(&self, kind: &str) -> Result<ElementSeed>
  }
  ```
  - Every field gets a doc comment.
  - `seed_cache: HashMap<(PackVersion, String /* locale */), BTreeMap<WidgetFile, ElementSeed>>` lives on `Session` (`lib.rs` field) and is cleared with `pack_cache`.
  - `kind_description(kind, schema_description, l)` goes in `param_fields.rs`, next to `kind_name`. It tries the Fluent key only for built-in namespaces.
- Modify:
  - `crates/oku-slides-session/src/lib.rs:505-512`: delete `add_widgets`. The snapshot calls `add_packs()` and `free_slots()`.
  - `views.rs:105-125`: `Snapshot.add_widgets` → `add_packs: Vec<AddPackView>`, plus `free_slots`. Delete the old `AddWidget<'a>`. Re-export `AddPackView`, `AddWidget` and `ElementSeed` from `lib.rs` (they are part of `Snapshot`).
  - `widget_examples.rs:63-67`, `pack_mode.rs:64-94`, `pack_mode_preview.rs:34`, `widget_preview.rs:323`: read examples through `localized_examples`. `example_slide` and `layout_slides` take `&Localizer`; update every caller (`rg -n 'example_slide|layout_slides|realign_layouts|preview_slide' crates/oku-slides-session/src`).
  - `pack_mode_save.rs:85-98`: a copy (new id) gets `resolved_examples` for every entry.
- Modify `typst/oku-slides/packs/base/widgets.json` (`examples`, `styles`, `parent`) and `typst/oku-slides/packs/essentiel/layouts.json` (markers, `parent`). Append both new versions to their `versions.txt`.
  - `crates/oku-slides-render/src/builtin_packs_tests.rs:219`: 3 versions.
  - Fix the session tests that snapshot `add_widgets` (`crates/oku-slides-session/tests/widgets.rs`, `rg -n 'add_widgets' crates`).
- i18n `session.ftl` (20 locales):
  - `session-example-base-*` and `session-example-essentiel-*`, per the Global Constraints;
  - `session-widget-description-base-{text,card,image,shape,chart,table,stack}`, from today's `ui-add-kind-description` (image and stack: "An image from a file", "Elements stacked in rows, columns or a grid", translated).
  - Add `crates/oku-slides-i18n/tests/example_markers.rs`: every `ftl:` marker in the two shipped manifests has its key in all 20 locales.

**Interfaces:**
- Consumes: `PackManifest.styles` (Task 1).
- Produces:
  - `Snapshot.add_packs: Vec<AddPackView>` (`AddWidget { name, label, description, seed }`) and `Snapshot.free_slots`;
  - `localized_examples(Uuid, &Map<String, Value>, &Localizer)`;
  - `Session::seed_element`.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-session/src/example_text_tests.rs
#[rstest]
#[case("en", "New text")]
#[case("fr", "Nouveau texte")]
fn localized_examples_builtin_marker_is_translated(#[case] locale: &str, #[case] expected: &str) {
    let values = json!({"body": [{"runs": [{"t": "ftl:session-example-base-text-body"}]}]});
    let out = localized_examples(BASE_ID, values.as_object().unwrap(), &Localizer::new(Some(locale)));
    assert_that!(out["body"][0]["runs"][0]["t"].as_str()).is_equal_to(Some(expected));
}

#[test]
fn localized_examples_pack_not_builtin_keeps_marker() {
    let values = json!({"title": "ftl:session-example-base-card-title"});
    let out = localized_examples(Uuid::new_v4(), values.as_object().unwrap(), &Localizer::new(Some("en")));
    assert_that!(out["title"].as_str()).is_equal_to(Some("ftl:session-example-base-card-title"));
}

#[test]
fn localized_examples_key_outside_prefix_is_kept() {
    let values = json!({"title": "ftl:ui-toolbar-open"});
    let out = localized_examples(BASE_ID, values.as_object().unwrap(), &Localizer::new(Some("en")));
    assert_that!(out["title"].as_str()).is_equal_to(Some("ftl:ui-toolbar-open"));
}

#[test]
fn localized_examples_missing_key_is_kept() {
    let values = json!({"title": "ftl:session-example-nothing-here"});
    let out = localized_examples(BASE_ID, values.as_object().unwrap(), &Localizer::new(Some("en")));
    assert_that!(out["title"].as_str()).is_equal_to(Some("ftl:session-example-nothing-here"));
}

// crates/oku-slides-session/src/add_grid_tests.rs
#[test]
fn seed_element_shape_has_the_manifest_style() {
    let s = session();
    let seed = s.seed_element(kinds::base::SHAPE).unwrap();
    assert_that!(serde_json::to_value(seed.style).unwrap()).is_equal_to(json!({"fill": "surface", "radius": "md"}));
}

#[test]
fn seed_element_card_title_is_french_in_french() {
    let s = session_in_locale("fr"); // `session()` with `Localizer::new(Some("fr"))`, as session_in does
    let seed = s.seed_element(kinds::base::CARD).unwrap();
    assert_that!(seed.params["title"].as_str()).is_equal_to(Some("Titre de la carte"));
}

#[test]
fn seed_element_layout_kind_is_unknown_widget() {
    let s = session();
    let err = s.seed_element(kinds::essentiel::CONTENT).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWidget);
}

#[test]
fn seed_element_drops_a_value_the_schema_rejects() {
    let s = session_with_widget_pack_example("acme", "badge", json!({"label": 42})); // `label` is p.plain()
    assert_that!(s.seed_element("acme/badge").unwrap().params.contains_key("label")).is_false();
}

#[test]
fn add_packs_lists_base_with_seven_widgets_labelled() {
    let s = session();
    let base = s.add_packs().into_iter().find(|p| p.namespace == "base").unwrap();
    assert_that!(base.widgets).has_length(7);
    assert_that!(base.widgets.iter().find(|w| w.name == kinds::base::CARD).unwrap().label.as_str()).is_equal_to("Card");
}

#[test]
fn free_slots_content_and_blank_accept_everything() {
    let slots = session().free_slots();
    assert_that!(slots.get(kinds::essentiel::CONTENT)).is_equal_to(Some(&None));
    assert_that!(slots.get(kinds::essentiel::BLANK)).is_equal_to(Some(&None));
    assert_that!(slots.contains_key(kinds::essentiel::TITLE)).is_false();
}

#[test]
fn saving_essentiel_as_copy_writes_plain_text_examples() {
    let mut s = session();
    let copy = save_builtin_layouts_as_copy(&mut s, "Ma copie"); // edit_pack(essentiel) then save_pack { as_new: true }
    assert_that!(serde_json::to_string(&copy.examples).unwrap()).does_not_contain("ftl:");
}
```

- `session_with_widget_pack_example`: add a library widget pack with one `badge` widget (`label: p.plain()`) and that example, using `manifest_ns`, `files_of` and `add_version` from `test_support.rs`, then pin it.
- `save_builtin_layouts_as_copy`: drive `edit_pack` and `save_pack` the way `pack_mode_save_tests.rs` does (`rg -n 'as_new' crates/oku-slides-session/src/*_tests.rs`).
- Regression `crates/oku-slides-session/tests/regressions/issue_1111_layout_examples_follow_ui_language.rs`: the layout picker's `essentiel/end` example text is "Thank you" in en and "Merci" in fr.
- In `crates/oku-slides-session/src/deck_packs_tests.rs` (Task 3 creates the file; add this test there): `pack_updates_old_builtin_base_is_one` — a deck pinning the previous shipped `base` counts 1, which proves the `parent` link.

- [ ] **Step 2: Run them to see them fail.** Run `cargo nextest run -p oku-slides-session -p oku-slides-i18n`. Expected: FAIL.

- [ ] **Step 3: Implement** per the file list.

- [ ] **Step 4: Run the gate.** Run `cargo check --workspace --all-targets`, `cargo nextest run -p oku-slides-session -p oku-slides-render -p oku-slides-i18n` and clippy. Expected: PASS. Golden unchanged: `git diff main -- crates/oku-slides-render/tests/snapshots/golden__reference_pages.snap` is empty.

- [ ] **Step 5: Stage and write `task-2-commit.txt`**: `feat(session) : exemples traduits des packs intégrés, grille par pack avec départ, slots libres, nouvelles versions de base et essentiel (t-1111)`.

---

### Task 3: Session — the `host-packs` panel and pack actions, banner removed, carried session fixes

Sub-task: t-1112.

**Files:**
- Create `crates/oku-slides-session/src/deck_packs.rs` (+ `deck_packs_tests.rs`): the view and the actions. Every pub field is documented.
  ```rust
  /// Les packs du deck et ce qu'on peut en faire, sans texte traduit.
  #[derive(Debug, Clone, PartialEq)]
  pub struct DeckPacksView { pub layouts: Vec<PinnedPackView>, pub widgets: Vec<PinnedPackView>,
      pub addable: BTreeMap<PackKind, Vec<(Namespace, String)>>, pub theme: ThemeSectionView }
  /// Un pack épinglé : identité, relation à la bibliothèque, refus de retrait.
  #[derive(Debug, Clone, PartialEq)]
  pub struct PinnedPackView { pub kind: PackKind, pub namespace: Namespace, pub name: String, pub version: String,
      pub relation: Option<Relation>, pub builtin: bool, pub deck_only: bool, pub library_has_pinned: bool,
      pub remove_refusal: Option<OpError> }
  /// Le thème du deck dans le panneau.
  #[derive(Debug, Clone, PartialEq)]
  pub struct ThemeSectionView { pub active: Namespace, pub name: String, pub extends: Vec<String>,
      pub overrides: usize, pub choices: Vec<(Namespace, String)>, pub outdated: bool }
  impl Session {
      /// La vue du panneau des packs ; refusée hors du mode deck.
      pub fn deck_packs(&self) -> Result<DeckPacksView>
      /// Ajoute au deck la version courante d'un pack de la bibliothèque.
      pub fn add_library_pack(&mut self, kind: PackKind, ns: &Namespace) -> Result<()>
      /// Retire un pack du deck, en une étape d'annulation.
      pub fn remove_deck_pack(&mut self, kind: PackKind, ns: &Namespace) -> Result<()>
      /// Le deck passe à ma version courante d'un pack.
      pub fn pack_use_mine(&mut self, kind: PackKind, ns: &Namespace) -> Result<()>
      /// La version du deck devient ma version courante ; le deck ne change pas.
      pub fn pack_make_current(&mut self, kind: PackKind, ns: &Namespace) -> Result<()>
      /// Le deck passe à ma version courante de son thème.
      pub fn update_deck_theme(&mut self) -> Result<()>
      /// Les épinglages plus anciens que la bibliothèque (cache).
      pub(crate) fn pack_updates(&self) -> u32
      /// Recalcule le nombre d'épinglages en retard.
      pub(crate) fn refresh_pack_updates(&mut self)
  }
  ```
  - **One relation function** (`fn pin_relation(&self, kind, ns, pin) -> Option<Relation>`, moved from `pack_notice.rs:138-155` minus the closed-notice check) serves `PinnedPackView.relation` and `pack_updates`. The theme uses its counterpart on the theme library, in the same module.
  - **One R15 helper**: `PackLibrary::owned_current(kind: PackKind, ns: &Namespace) -> Option<StoredVersion<PackManifest>>` in `library_packs.rs`, factored from `may_own` and `by_namespace` (`:196`). It is used by `by_namespace`, `addable`, `add_library_pack`, and by `current_pin` (t-1091, which then also resolves layout packs).
  - Every action calls `ensure_deck_mode()` first.
  - `refresh_pack_updates` is called at the points listed in the Global Constraints:
    - `deck_file.rs:120-122` (open);
    - after `apply` when any op is `AddPack`, `RemovePack` or `SetTheme`;
    - after undo, redo and merge (`rg -n 'fn undo|fn redo|fn merge' crates/oku-slides-session/src`);
    - after `pack_make_current` and `save_pack`.
- Create `crates/oku-slides-session/src/deck_packs_panel.rs` (+ tests):
  ```rust
  /// Surface du panneau des packs du deck.
  pub const HOST_PACKS: &str = "host-packs";
  /// La surface du panneau, traduite par `l` (celui de l'hôte, à chaque usage).
  pub fn deck_packs_panel(view: &DeckPacksView, l: &Localizer) -> Surface
  /// L'action du panneau que décrivent un évènement et son contexte ; `None` si mal formée.
  pub fn packs_event(name: &str, context: &Map<String, Value>) -> Option<PacksEvent>
  /// Une action du panneau des packs.
  #[derive(Debug, Clone, PartialEq)]
  pub enum PacksEvent {
      /// Ajouter la version courante d'un pack de la bibliothèque.
      AddPack { kind: PackKind, namespace: Namespace },
      /// Retirer un pack du deck.
      RemovePack { kind: PackKind, namespace: Namespace },
      /// Passer le deck à ma version.
      UseMine { kind: PackKind, namespace: Namespace },
      /// Faire de la version du deck ma version courante.
      MakeCurrent { kind: PackKind, namespace: Namespace },
      /// Choisir le thème du deck.
      Theme { theme: Namespace },
      /// Passer le deck à ma version de son thème.
      ThemeUpdate,
  }
  impl Session {
      /// Applique une action du panneau.
      pub fn packs_action(&mut self, event: PacksEvent) -> Result<()>
  }
  ```
  - Component ids: `root`, `title`, `close`, `layouts`, `widgets`, `theme`. Each pack gets `<kind>-<namespace>` (card), `-name`, `-version`, `-ns`, `-relation`, `-use-mine`, `-make-current`, `-remove` and `-refusal`. The selects are `<kind>-add` and `theme-pick`.
  - A refusal caption is `view.remove_refusal.localize(l)`.
  - `surface(HOST_PACKS, Place::Thread, None, components, json!({}))`, then `live` set as for `theme_picker`.
- `pack_notice.rs` → rename to `pack_receive.rs`, keeping only:
  - `is_deck_only`, `THEMES_SKIPPED_SEEN`;
  - `notice_skipped_themes`, `carried_themes_notice`, `receive_packs`;
  - `deck_pack_is_deck_only`, `embeds_deck_only_pack`.

  Rewrite its module doc. Move `opened_on_newer`, `files` and the action tests from `pack_notice_tests.rs` into `deck_packs_tests.rs`, adapted to the per-pack methods. Keep the receive tests in `pack_receive_tests.rs`.
- `lib.rs:279-283,421-423,697`: drop `pack_notice` and `pack_notices_closed`; add the field `pack_updates: u32`; the snapshot reads it.
- `deck_file.rs:108,122`: drop the banner lines, keep `receive_packs()`, then call `refresh_pack_updates()`.
- `views.rs:94`: `pack_notice` → `pack_updates: u32`.
- `request.rs`: delete the three requests at every match site (Global Constraints).
- `help_tests.rs:194-198`: the `pack_make_current` read-only case is deleted. A test that `deck_packs()` refuses in help mode replaces it.
- Re-export from `lib.rs` only what the hub needs: `HOST_PACKS`, `deck_packs_panel`, `packs_event`, `PacksEvent`, `DeckPacksView`.
- `crates/oku-slides-session/ARCHITECTURE.md`: replace the `pack_notice` lines (stay ≤ 50).
- Carried fixes:
  - **t-1091**: `deck_closure.rs:25` `current_pin` uses `owned_current`. Regression `tests/regressions/issue_1091_requires_ignores_legacy_builtin_copy.rs`: a library holding a legacy user copy under `base` (as `issue_1068_legacy_essentiel_copy` builds one), and a pack requiring `base`, resolves to the built-in pin.
  - **t-1093**: `pack_mode_save.rs:199,212`: run `pin_all` on clones of the decks and sources before any library write, then write. Regression `issue_1093_pack_save_conflict_writes_no_version.rs`: the stashed deck pins one namespace under both kinds; `save_pack` fails with `NamespaceTaken`, and the library's version count for that pack is unchanged. `legacy_copy_gets_no_new_version` asserts `SessionErrorKind::NamespaceTaken`.
  - **t-1109**: `resume_tests.rs` `write_snapshot_unsafe_names_writes_nothing_outside_dir` creates its own `tempfile::tempdir()` as the parent, puts the resume dir inside it, and checks that parent, never `$TMPDIR/x`.
  - **t-1106**:
    - `deck_theme_edit.rs:113` `edit_lower` validates against the deck's embedded theme pins (`deck.themes`, read through `embedded_or_library` with the deck's sources first). Regression `issue_1106_deck_theme_edit_uses_embedded_closure.rs`: the deck pins a theme version absent from the library, and the deck-mode edit validates against it.
    - `theme_composed_tests.rs`: `token_label_pack_not_builtin_falls_back_to_label_then_path`.
    - Split `theme_edit_tests.rs` (811 lines) into `theme_edit_tests.rs`, `theme_edit_save_tests.rs` and `theme_edit_switch_tests.rs`.
- i18n `session.ftl` (20 locales):
  - new: `packs-title`, `packs-section-layouts`, `packs-section-widgets`, `packs-section-theme`, `packs-relation-older`, `packs-relation-newer`, `packs-relation-parallel`, `packs-use-mine`, `packs-make-current`, `packs-remove`, `packs-add`, `packs-theme-overrides`, `packs-theme-pick`, `packs-undo-add`, `packs-undo-remove`, `packs-undo-theme-update`, `session-pack-not-make-current`;
  - renamed: `pack-notice-use-mine` → `packs-undo-use-mine`;
  - deleted: `pack-notice-older`, `-newer`, `-parallel`, `-make-current`, `-close`.
- `error.rs`: `PackNotMakeCurrent { pack, backtrace }` (built-in, deck-only, or version absent from the library).

**Interfaces:**
- Consumes:
  - `remove_pack_refusal` (Task 1);
  - Task 2 snapshot.
- Produces, for Task 5:
  - `Session::{deck_packs, packs_action, add_library_pack, remove_deck_pack, update_deck_theme}`;
  - `deck_packs_panel`, `packs_event`, `PacksEvent`, `HOST_PACKS`;
  - `Snapshot.pack_updates`.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-session/src/deck_packs_tests.rs
#[rstest]
#[case::base("base", OpErrorKind::BaseRequired)]
#[case::essentiel_in_use("essentiel", OpErrorKind::PackInUse)]
fn deck_packs_builtin_refusal_has_its_reason(#[case] pack: &str, #[case] expected: OpErrorKind) {
    let s = session_with_fixture(); // a deck with base/* elements on essentiel/* slides
    let view = s.deck_packs().unwrap();
    let all = view.layouts.iter().chain(&view.widgets);
    let pinned = all.into_iter().find(|p| p.namespace.as_str() == pack).unwrap();
    assert_that!(pinned.remove_refusal.as_ref().map(OpError::kind)).is_equal_to(Some(expected));
}

#[test]
fn deck_packs_refusal_text_is_french_in_french() {
    let s = session_with_fixture();
    let view = s.deck_packs().unwrap();
    let surface_fr = deck_packs_panel(&view, &Localizer::new(Some("fr")));
    let surface_en = deck_packs_panel(&view, &Localizer::new(Some("en")));
    assert_that!(text_of(&surface_fr, "widgets-base-refusal")).is_not_equal_to(text_of(&surface_en, "widgets-base-refusal"));
    assert_that!(text_of(&surface_fr, "widgets-base-refusal")).does_not_contain("0");
}

#[test]
fn add_library_pack_with_requirement_is_one_undo_step() {
    let mut s = session_with_library_widget_packs(&[("dep", &[]), ("acme", &["dep"])]);
    s.add_library_pack(PackKind::Widgets, &ns("acme")).unwrap();
    assert_that!(s.deck().widgets.contains_key(&ns("dep"))).is_true();
    s.undo().unwrap();
    assert_that!((s.deck().widgets.contains_key(&ns("acme")), s.deck().widgets.contains_key(&ns("dep")))).is_equal_to((false, false));
}

#[test]
fn add_library_pack_already_pinned_namespace_is_refused() {
    let mut s = session();
    let err = s.add_library_pack(PackKind::Layouts, &ns("base")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::Op); // PackKindConflict from the core
}

#[test]
fn remove_deck_pack_refused_leaves_deck_and_history() {
    let mut s = session_with_fixture();
    let before = (s.deck().clone(), s.undo_label());
    assert_that!(s.remove_deck_pack(PackKind::Widgets, &ns("base"))).is_err();
    assert_that!((s.deck().clone(), s.undo_label())).is_equal_to(before);
}

#[test]
fn pack_make_current_deck_only_pack_is_refused() {
    let mut s = session();
    let pin = adopt_deck_only_pack(&mut s);
    let ns = s.deck().widgets.iter().find(|(_, p)| **p == pin).map(|(n, _)| n.clone()).unwrap();
    assert_that!(s.pack_make_current(PackKind::Widgets, &ns).unwrap_err().kind()).is_equal_to(SessionErrorKind::PackNotMakeCurrent);
}

#[test]
fn pack_make_current_changes_library_not_deck() {
    let (_a, _b, mut s, theirs, _mine) = opened_on_newer();
    let deck = s.deck().clone();
    s.pack_make_current(PackKind::Widgets, &namespace_of_pack(&theirs)).unwrap();
    assert_that!(s.deck().clone()).is_equal_to(deck);
    assert_that!(s.undo_label()).is_none();
}

#[test]
fn deck_packs_deck_only_pack_has_no_relation() {
    let mut s = session();
    adopt_deck_only_pack(&mut s);
    let view = s.deck_packs().unwrap();
    let own = view.widgets.iter().find(|p| p.deck_only).unwrap();
    assert_that!(own.relation).is_none();
}

#[test]
fn deck_packs_outside_deck_mode_is_refused() {
    let mut s = session();
    s.edit_pack(None).unwrap();
    assert_that!(s.deck_packs().unwrap_err().kind()).is_equal_to(SessionErrorKind::NotDeckMode);
}

#[test]
fn pack_updates_newer_library_counts_one_and_use_mine_clears_it() {
    let (_a, _b, mut s, theirs, _mine) = opened_older_than_library(); // opened_on_newer with the roles swapped
    assert_that!(s.pack_updates()).is_equal_to(1);
    s.pack_use_mine(PackKind::Widgets, &namespace_of_pack(&theirs)).unwrap();
    assert_that!(s.pack_updates()).is_equal_to(0);
}

// crates/oku-slides-session/src/deck_packs_panel_tests.rs
#[rstest]
#[case("add_pack", json!({"kind": "widgets", "namespace": "acme"}), Some(PacksEvent::AddPack { kind: PackKind::Widgets, namespace: ns("acme") }))]
#[case("remove_pack", json!({"kind": "layouts", "namespace": "acme"}), Some(PacksEvent::RemovePack { kind: PackKind::Layouts, namespace: ns("acme") }))]
#[case("use_mine", json!({"kind": "widgets", "namespace": "acme"}), Some(PacksEvent::UseMine { kind: PackKind::Widgets, namespace: ns("acme") }))]
#[case("make_current", json!({"kind": "widgets", "namespace": "acme"}), Some(PacksEvent::MakeCurrent { kind: PackKind::Widgets, namespace: ns("acme") }))]
#[case("theme", json!({"theme": "pro"}), Some(PacksEvent::Theme { theme: ns("pro") }))]
#[case("theme_update", json!({}), Some(PacksEvent::ThemeUpdate))]
#[case("add_pack", json!({"kind": "themes", "namespace": "acme"}), None)]
#[case("add_pack", json!({"kind": "widgets"}), None)]
#[case("add_pack", json!({"kind": "widgets", "namespace": "Not Valid"}), None)]
#[case("other", json!({}), None)]
fn packs_event_parses_or_rejects(#[case] name: &str, #[case] ctx: Value, #[case] expected: Option<PacksEvent>) {
    assert_that!(packs_event(name, ctx.as_object().unwrap())).is_equal_to(expected);
}

#[test]
fn deck_packs_panel_refused_pack_has_caption_not_button() {
    let view = session_with_fixture().deck_packs().unwrap();
    let ids = component_ids(&deck_packs_panel(&view, &Localizer::new(Some("en"))));
    assert_that!(ids.contains(&"widgets-base-remove".to_owned())).is_false();
    assert_that!(ids.contains(&"widgets-base-refusal".to_owned())).is_true();
}
```

Helpers to write in `deck_packs_tests.rs`:
- `session_with_library_widget_packs(&[(ns, requires)])`, built on `manifest_ns`, `files_of` and `add_version`;
- `opened_older_than_library()`;
- `namespace_of_pack(&PackFiles)`;
- `text_of(&Surface, id)` and `component_ids(&Surface)`: read `surface.messages` as the theme picker tests do (`rg -n 'fn components|fn component' crates/oku-slides-session/src/*_tests.rs`).

Add `NotDeckMode` to the session errors if `ensure_deck_mode` uses another kind; use whatever kind it returns (`rg -n 'fn ensure_deck_mode' -A5 crates/oku-slides-session/src`).

- [ ] **Step 2: Run them to see them fail.** Run `cargo nextest run -p oku-slides-session`. Expected: FAIL.

- [ ] **Step 3: Implement** per the file list. Then `rg -n 'pack_notice|PackNotice|PackUseMine|PackMakeCurrent|PackNoticeClose|"pack_make_current"|"pack_use_mine"|"pack_notice_close"' crates` returns nothing.

- [ ] **Step 4: Run the gate.** Run `cargo check --workspace --all-targets`, `cargo nextest run -p oku-slides-session -p oku-slides-i18n` and clippy. Expected: PASS.

- [ ] **Step 5: Stage and write `task-3-commit.txt`**: `feat(session)! : panneau host-packs (ajout, retrait, versions), bandeau de pack supprimé, correctifs de session (t-1112, t-1091, t-1093, t-1106, t-1109)`.

---

### Task 4: Session and A2UI — editable `extends`, base namespace, catalog v4, `Place::Banner` gone

Sub-task: t-1113.

**Files:**
- `crates/oku-slides-a2ui/src/message.rs`:
  - `ButtonIcon` gains `Up` (« Une flèche vers le haut : monter. »), `Down` and `Remove`;
  - `CATALOG_ID` = `urn:oku-slides:a2ui:catalog:v4`;
  - `message_tests.rs`: `button_icon_up_serializes_lowercase`.
- `crates/oku-slides-session/src/agent/forms.rs:34-35`: delete `Place::Banner`.
- TS in the same commit:
  - `ui/src/a2ui/catalog.tsx:79`: `icon: z.enum(["close", "up", "down", "remove"])`; the TS `CATALOG_ID` → v4;
  - `ui/src/icons.tsx`: entries `up`, `down` and `remove`, hand-drawn `stroke(...)` SVGs like the others;
  - `ui/src/a2ui/catalog.test.tsx`: a v4 surface with `icon: "remove"` validates, and `icon: "spin"` is rejected;
  - `ui/src/a2ui/fixtures/agentPicker.ts:12` and `agent-surfaces.json` (catalog id; delete the `banner` fixture at `:1535-1600`);
  - `ui/src/api.ts:682` (`Place` without `banner`), `ui/src/agent/surfaces.ts:104`, `ui/src/agent/surfaces.test.ts:78-79`.
- Insta snapshots: the v3 id in `crates/oku-slides-agent/src/snapshots/*` and `crates/oku-slides-session/src/agent/snapshots/*`. `cargo insta review` accepts only catalog-id diffs. `crates/oku-slides-a2ui/ARCHITECTURE.md:4`.
- `crates/oku-slides-session/src/`:
  - `theme_edit.rs`:
    - `ThemeEdit { base, namespace, name, extends }` and `ThemeEdit::of(base, namespace, name, extends)`;
    - `starting_extends(&self, base: &ThemeManifest) -> Vec<Namespace>`;
    - `initial_theme_edit` and `set_edited_theme` fill `namespace` and `extends`;
    - `ThemeEditView { base, namespace, name, child, extends: Vec<ThemeParentView>, dirty, revision }` and `ThemeParentView { namespace, name }`, all documented;
    - `theme_dirty` also compares `extends` with `starting_extends`.
  - New `theme_extends.rs` (+ `theme_extends_tests.rs`): `ExtendsEdit` (documented variants, `Deserialize` with `#[serde(tag = "edit", rename_all = "lowercase")]`), `set_edited_theme_extends` and `edit_theme_extends`, per the Global Constraints.
  - `theme_closure.rs`: `closure_of_parents(parents, find)`.
  - `theme_draft.rs`:
    - `draft_chain` per the Global Constraints;
    - `draft_extends_names` → `draft_parents(edit) -> Vec<ThemeParentView>`; update its caller `agent/context.rs:140`;
    - `saved_manifest` writes `extends`, and its early return compares `extends`.
  - `theme_panel.rs:276-293` `theme_header`: the rows and the select. `pub const THEME_EXTENDS_EVENT: &str = "extends"`.
  - `theme_panel_tests.rs:353`: the old caption assertion becomes the row assertions.
  - `request.rs`: `SetEditedThemeExtends { extends: Vec<Namespace> }` and `EditThemeExtends { edit: ExtendsEdit }`, at every match site (Global Constraints).
  - `error.rs`: `ThemeExtendsDuplicate { theme, backtrace }` and `ThemeExtendsMove { theme, backtrace }`, with Fluent keys `session-theme-extends-duplicate` and `session-theme-extends-move`.
  - `resume.rs:36`: `VERSION = 3`. On resume, the restored `ThemeEdit` is validated with the extends checks (Global Constraints).
  - `resume_tests.rs`: the "older version is refused" case uses 2.
- i18n `session.ftl` (20 locales):
  - new: `theme-extends-title`, `theme-extends-up`, `theme-extends-down`, `theme-extends-remove`, `theme-extends-add`, `session-undo-theme-extends`, `session-theme-extends-duplicate`, `session-theme-extends-move`;
  - deleted: `theme-extends`.
- `ui/e2e/200-theme-editor.cjs:49`: the `[data-component-id="extends"]` caption check becomes a check on the first parent row's name ("Papier"). This keeps the e2e suite consistent at this commit; the new steps come in Task 7.

**Interfaces:**
- Produces:
  - `Session::{set_edited_theme_extends, edit_theme_extends}`, `ExtendsEdit`, the two requests;
  - `ThemeEditView.namespace`, `ThemeParentView`;
  - `ButtonIcon::{Up, Down, Remove}`, catalog v4.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-session/src/theme_extends_tests.rs
// Papier and Pro set color.accent to #3f6b5e and #00bef4: the last parent wins.
#[test]
fn set_edited_theme_extends_reorder_then_undo_redo_follow() {
    let mut s = pack_mode_on_user_theme_extending(&["papier", "pro"]);
    assert_that!(draft_accent(&s)).is_equal_to("#00bef4".to_owned());
    s.set_edited_theme_extends(vec![ns("pro"), ns("papier")]).unwrap();
    assert_that!((edit_extends(&s), draft_accent(&s))).is_equal_to((vec!["pro".into(), "papier".into()], "#3f6b5e".to_owned()));
    s.undo().unwrap();
    assert_that!((edit_extends(&s), draft_accent(&s))).is_equal_to((vec!["papier".into(), "pro".into()], "#00bef4".to_owned()));
    s.redo().unwrap();
    assert_that!(edit_extends(&s)).is_equal_to(vec!["pro".to_owned(), "papier".to_owned()]);
}

#[test]
fn set_edited_theme_extends_two_edits_two_undos_return_to_start() {
    let mut s = pack_mode_on_builtin("papier");
    s.edit_theme_extends(ExtendsEdit::Add { theme: ns("pro") }).unwrap();
    s.edit_theme_extends(ExtendsEdit::Up { theme: ns("pro") }).unwrap();
    s.undo().unwrap();
    s.undo().unwrap();
    assert_that!(edit_extends(&s)).is_equal_to(vec!["papier".to_owned()]);
}

#[test]
fn set_edited_theme_extends_after_token_edit_undo_keeps_the_token() {
    let mut s = pack_mode_on_builtin("papier");
    set_accent(&mut s, "#ff0000");
    s.edit_theme_extends(ExtendsEdit::Add { theme: ns("pro") }).unwrap();
    s.undo().unwrap();
    assert_that!(edited_accent(&s)).is_equal_to(Some("#ff0000".to_owned()));
}

#[test]
fn set_edited_theme_extends_own_namespace_is_a_cycle() {
    let mut s = pack_mode_on_user_theme("marque");
    assert_that!(s.set_edited_theme_extends(vec![ns("marque")]).unwrap_err().kind()).is_equal_to(SessionErrorKind::ThemeCycle);
}

#[test]
fn set_edited_theme_extends_parent_reaching_draft_is_a_cycle() {
    let mut s = pack_mode_on_user_theme("marque");
    add_library_theme(&s, "sombre", &["marque"]);
    assert_that!(s.set_edited_theme_extends(vec![ns("sombre")]).unwrap_err().kind()).is_equal_to(SessionErrorKind::ThemeCycle);
}

#[test]
fn set_edited_theme_extends_child_listing_its_base_and_pro_is_accepted() {
    let mut s = pack_mode_on_builtin("papier");
    assert_that!(s.set_edited_theme_extends(vec![ns("papier"), ns("pro")])).is_ok();
}

#[rstest]
#[case::duplicate(vec!["papier", "papier"], SessionErrorKind::ThemeExtendsDuplicate)]
#[case::unknown(vec!["nowhere"], SessionErrorKind::UnknownTheme)]
fn set_edited_theme_extends_bad_list_is_refused(#[case] list: Vec<&str>, #[case] expected: SessionErrorKind) {
    let mut s = pack_mode_on_builtin("papier");
    let err = s.set_edited_theme_extends(list.into_iter().map(ns).collect()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(expected);
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_not_equal_to(err.localize(&Localizer::new(Some("en"))));
}

#[test]
fn set_edited_theme_extends_outside_pack_mode_is_refused() {
    let mut s = session();
    assert_that!(s.set_edited_theme_extends(vec![]).unwrap_err().kind()).is_equal_to(SessionErrorKind::NotEditingPack);
}

#[test]
fn set_edited_theme_extends_empty_list_is_accepted() {
    let mut s = pack_mode_on_user_theme_extending(&["papier"]);
    assert_that!(s.set_edited_theme_extends(vec![])).is_ok();
}

#[test]
fn edit_theme_extends_first_up_is_refused() {
    let mut s = pack_mode_on_builtin("papier");
    let err = s.edit_theme_extends(ExtendsEdit::Up { theme: ns("papier") }).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::ThemeExtendsMove);
}

#[test]
fn saved_extends_only_change_writes_a_version() {
    let mut s = pack_mode_on_user_theme_extending(&["papier", "pro"]);
    s.set_edited_theme_extends(vec![ns("pro"), ns("papier")]).unwrap();
    let saved = save_and_read_theme(&mut s, "marque");
    assert_that!(saved.extends).is_equal_to(vec![ns("pro"), ns("papier")]);
}

#[test]
fn saved_child_keeps_edited_extends() {
    let mut s = pack_mode_on_builtin("papier");
    s.set_edited_theme_extends(vec![ns("papier"), ns("pro")]).unwrap();
    set_accent(&mut s, "#ff0000");
    let saved = save_and_read_new_theme(&mut s);
    assert_that!(saved.extends).is_equal_to(vec![ns("papier"), ns("pro")]);
}

#[test]
fn editing_view_exposes_base_namespace() {
    let s = pack_mode_on_builtin("pro");
    assert_that!(s.editing_view().unwrap().theme.namespace.as_str()).is_equal_to("pro");
}

// crates/oku-slides-session/src/resume_tests.rs
#[test]
fn resume_pack_mode_keeps_edited_extends() { /* write state in pack mode after an extends edit, reopen, same list */ }

#[test]
fn resume_version_2_state_is_set_aside() { /* a state.json with "version":2 is refused, with notice-tab-set-aside */ }
```

Helpers in `theme_extends_tests.rs`, built on the `theme_edit_tests.rs` helpers (`session_with_theme`, `add_user_theme`, `set_accent`, `theme_edit`, `rendered_token`):
- `pack_mode_on_builtin(ns)`;
- `pack_mode_on_user_theme(ns)`, `pack_mode_on_user_theme_extending(&[…])`;
- `add_library_theme(&s, ns, extends)`;
- `edit_extends(&s) -> Vec<String>`;
- `draft_accent(&s) -> String` (the rendered `color.accent` under the draft), `edited_accent(&s)`;
- `save_and_read_theme(&mut s, ns)`, `save_and_read_new_theme(&mut s)`.

The accent values come from `typst/oku-slides/themes/{papier,pro}.json`; read them there and fix the literals if they differ.

`theme_panel_tests.rs`:
- `host_theme_first_parent_has_no_up_button`;
- `host_theme_add_select_excludes_listed_and_own` (the options of `theme-extends-add`).

- [ ] **Step 2: Run them to see them fail.** Run `cargo nextest run -p oku-slides-session -p oku-slides-a2ui` and `cd ui && bun test src/a2ui src/agent`. Expected: FAIL.

- [ ] **Step 3: Implement** per the file list.

- [ ] **Step 4: Run the gate.** Run `cargo check --workspace --all-targets`, `cargo nextest run -p oku-slides-session -p oku-slides-a2ui -p oku-slides-agent -p oku-slides-i18n`, clippy, and `cd ui && bun test src/a2ui src/agent`. Expected: PASS. `rg -n 'catalog:v3' crates ui/src` returns nothing.

- [ ] **Step 5: Stage and write `task-4-commit.txt`**: `feat(theme)! : liste extends éditable dans l'éditeur, espace de noms de la base, catalogue A2UI v4, Place::Banner supprimé (t-1113, t-1103, t-1107)`.

---

### Task 5: MCP hub, agent, local agent and CLI — `show_packs`, pack tools, `set_theme_extends` — workspace green

Sub-task: t-1114.

**Files:**
- `crates/oku-slides-mcp/src/`:
  - `hub.rs:91`: `Shown::Packs(PacksShown)`, with `struct PacksShown { last: Surface }` (documented).
  - New `hub/packs.rs` (`mod packs;` in `hub.rs`):
    - `pub fn show_packs(&self) -> Result<bool, HubError>`: `false` outside deck mode; it builds with `self.localizer.current()` and shows it alone;
    - `pub(super) fn apply_packs(&self, context: &Map<String, Value>, name: &str) -> Result<PacksShown, HubInner>`: it parses with `packs_event` (logging `warn` and returning `Incomplete` on `None`), calls `Session::packs_action`, emits `McpEvent::Changed`, then the rebuilt surface, and returns the new `PacksShown`;
    - `pub(super) fn refresh_packs(&self)`: it returns unless `Shown::Packs` is present, closes outside deck mode, and otherwise rebuilds and emits only if the surface differs from `last`.
  - `hub/actions.rs:54-66` `act`: an arm `(Shown::Packs(_), name) if name != "dismiss" => self.apply_packs(&action.context, name).map(|p| After::Replace(Shown::Packs(p)))`, placed before `(_, "dismiss")`. In `prune` (`:146-160`), call `self.refresh_packs()` next to `refresh_themes()`, and add `Shown::Packs(_)` to the `return false` arm at `:160`.
  - `edit_tools.rs`: `add_pack`, `remove_pack` and `set_theme_extends`, through `with_session`, errors through `Tools::fail_with`, `prune_surfaces` after a change. `surface_tools.rs`: `show_packs`.
  - `tools.rs` and `handler.rs`: register the four tools, with descriptions in the style of `show_themes` (`tools.rs:291`). Update the `deck_outline` description (`tools.rs:254`).
  - `read_tools.rs:56-63`: `edited_theme` with `namespace` and `extends: [{namespace, name}]`.
  - `crates/oku-slides-mcp/tests/edit.rs:186`: the exact `edited_theme` JSON.
  - `ARCHITECTURE.md`: `hub/packs`, and correct the "only box the hub builds itself" line. `AGENTS.md` if a rule names the tools.
- `crates/oku-slides-session/src/request.rs`: `Interface` and `SavePack` in `changes_deck()` if absent (Global Constraints), so `Stack::after_change` runs `prune`.
- `crates/oku-slides-agent/src/stack.rs`:
  - `AgentCommand::ShowPacks { tab }` (documented: « L'interface ouvre le panneau des packs du deck : la surface va à cet onglet. ») and its `tab()` arm;
  - `handle` opens the tab (`self.open`) and returns `Err(String)` with the localized hub error when `show_packs` fails.
  - The forwarders (`src-tauri/src/commands.rs:197`, `crates/oku-slides-cli/src/serve.rs:180`) deserialize `AgentCommand` as a whole. Check that they need no change, and add a test case in `stack` tests that `{"cmd":"show_packs","tab":"t"}` deserializes.
- `crates/oku-slides-agent/src/instructions/agent.md`: add « Show the deck's packs with show_packs; add or remove a pack with add_pack / remove_pack (a refusal says why); in pack mode, set_theme_extends sets the edited theme's parents. »
- `crates/oku-slides-local-agent/src/families.rs:58`: route `show_packs` with `show_themes`, `add_pack`/`remove_pack` with `apply`, and `set_theme_extends` with the theme tools. Add a routing test.

**Interfaces:**
- Consumes: Tasks 3–4.
- Produces: `{cmd: "show_packs", tab}`, for Task 6.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/oku-slides-mcp/tests/packs.rs (new integration test, like tests/themes.rs)
fn show(h: &common::Harness) -> bool {
    tokio::task::block_in_place(|| h.hub.show_packs()).unwrap_or_else(|e| panic!("panneau des packs : {e}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn show_packs_puts_the_panel_live_outside_any_turn() {
    let h = start().await;
    let mut rx = h.hub.subscribe();
    assert_that!(show(&h)).is_true();
    let surface = next_surface(&mut rx).await;
    assert_that!((surface.id.as_str(), surface.live, surface.turn)).is_equal_to((HOST_PACKS, true, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn packs_add_event_pins_and_the_last_panel_lists_it() {
    let h = start_with_library_widget_pack("acme").await; // common::start, then a library pack added through the session
    let mut rx = h.hub.subscribe();
    show(&h);
    let _ = next_surface(&mut rx).await;
    tokio::task::block_in_place(|| h.hub.act(action(HOST_PACKS, "add_pack", json!({"kind": "widgets", "namespace": "acme"})))).unwrap();
    let last = last_surface(&mut rx, HOST_PACKS).await;
    assert_that!(component_ids(&last)).contains("widgets-acme".to_owned());
}

#[tokio::test(flavor = "multi_thread")]
async fn packs_malformed_context_is_incomplete_and_keeps_the_panel() {
    let h = start().await;
    show(&h);
    let r = tokio::task::block_in_place(|| h.hub.act(action(HOST_PACKS, "add_pack", json!({"kind": "themes"}))));
    assert_that!(r).is_err();
    assert_that!(h.hub.surfaces().iter().any(|s| s.id == HOST_PACKS)).is_true();
}

#[tokio::test(flavor = "multi_thread")]
async fn packs_panel_closes_when_pack_mode_starts() {
    let h = start().await;
    show(&h);
    h.session(|s| s.edit_pack(None)).unwrap();
    tokio::task::block_in_place(|| h.hub.prune());
    assert_that!(h.hub.surfaces().iter().any(|s| s.id == HOST_PACKS)).is_false();
}

#[tokio::test(flavor = "multi_thread")]
async fn show_packs_outside_deck_mode_shows_nothing() { /* edit_pack first; show() is false */ }

// crates/oku-slides-mcp/tests/edit.rs
#[tokio::test(flavor = "multi_thread")]
async fn remove_pack_in_use_returns_the_localized_reason() {
    let h = start().await; // the fixture deck uses base widgets
    let out = h.call("remove_pack", json!({"kind": "widgets", "namespace": "base"})).await;
    assert_that!(out.is_error).is_equal_to(Some(true));
    assert_that!(text(&out)).does_not_contain("0"); // no "Operation 0 refused" wrapper
}
```

Write the helpers in `tests/common/mod.rs`, after reading what `Harness` offers (`h.session`, `h.call`, `text`):
- `start_with_library_widget_pack`;
- `last_surface(rx, id)`: drain the channel and keep the last surface with that id;
- `component_ids`.

Also add tests for `set_theme_extends` outside pack mode (error) and `deck_outline_edited_theme_has_namespace`.

- [ ] **Step 2: Run them to see them fail.** Run `cargo nextest run -p oku-slides-mcp -p oku-slides-agent -p oku-slides-local-agent`. Expected: FAIL.

- [ ] **Step 3: Implement** per the file list.

- [ ] **Step 4: Run the workspace gate.** Run `cargo nextest run --workspace --exclude oku-slides-app` and `cargo clippy --workspace --all-targets -- -D warnings`. Expected: PASS.

- [ ] **Step 5: Stage and write `task-5-commit.txt`**: `feat(agent) : panneau des packs par le hub, outils show_packs, add_pack, remove_pack, set_theme_extends, espace de noms du thème édité dans deck_outline (t-1114, t-1107)`.

---

### Task 6: UI — packs button, add grid from packs, free slots, theme editor `extends`, banner gone

Sub-task: t-1115.

**Files:**
- `ui/src/api.ts` (866 lines): move the pack and theme view types into a new `ui/src/packTypes.ts`, imported explicitly where used. In `api.ts`:
  - `Snapshot`: drop `pack_notice` and `add_widgets`; add `pack_updates: number`, `add_packs: AddPackView[]` and `free_slots: Record<string, string[] | null>`.
  - `AddWidget { name; label: string; description: string | null; seed: ElementSeed }`, `ElementSeed { params: Record<string, unknown>; style?: Style }`, `AddPackView`.
  - Requests: drop `pack_use_mine`, `pack_make_current` and `pack_notice_close` (`:609-613`, `packNotice` at `:850`). Add `set_edited_theme_extends { extends }` and `edit_theme_extends { edit, theme }`, with `setEditedThemeExtends(extends)` and `editThemeExtends(edit, theme)`.
  - `ThemeEditView`: `namespace: string`, `extends: { namespace: string; name: string }[]`.
  - `AgentCommand`: `{ cmd: "show_packs"; tab: string }`, plus `agentApi(tab).showPacks()`.
- `ui/src/agent/surfaces.ts:41-43`: `export const HOST_PACKS = "host-packs"`; `isHostSettings` excludes it. `AgentDock.tsx:41`: it is not folded behind a question. Add a test in `surfaces.test.ts`.
- `ui/src/ToolsTray.tsx`:
  - the "Packs du deck" button (`ui-toolbar-packs`), shown when `!snap.editing && !snap.read_only && !snap.history`, with class `has-update` and `title` suffix `ui-toolbar-packs-update` when `snap.pack_updates > 0`, calling `agentApi(tab).showPacks()`, errors to the toast;
  - `ToolsTray.tsx:71`: `disabled={!acceptsElements(view?.slide.kind, snap.free_slots)}`.
- Add grid:
  - `widgetChoices.ts`:
    - `WidgetChoice { key: string /* kind */; name; label; description; widget: AddWidget }`;
    - `widgetSections(packs: AddPackView[]): { namespace; name; choices: WidgetChoice[] }[]`;
    - `filterChoices` keeps working on the flat list.
  - `WidgetPicker.tsx`: one `<section>` per pack, with an `<h3>` pack name; every card shows its preview through `usePreviewUrls`; the native `Icon` branch goes.
  - `newElement.ts`:
    ```ts
    /** Élément neuf du widget `kind` (`id` fourni), parti de son départ `seed`. */
    export function newElement(kind: string, id: string, seed: ElementSeed): Element {
      return { id, kind, params: structuredClone(seed.params), ...(seed.style ? { style: structuredClone(seed.style) } : {}) };
    }
    ```
  - `useSlideOps.ts:46-58`: `addElement(widget: AddWidget, override?: { text: Paragraph[]; label: string })`:
    - it is synchronous, using `widget.seed`;
    - the id prefix is the first letter of the name part after `/`, as today;
    - the undo label is `override?.label ?? t("ui-undo-add-widget", { name: widget.label })`;
    - a `BASE.text` element without override opens in edit mode;
    - `addPackWidget` merges into it.
  - `useClipboard.ts:59,95,116,120`: paste uses `acceptsElements` plus `canAdd(slide, BASE.text, free_slots)`, and calls `addElement` with the `base/text` entry of `snap.add_packs` and an override. If `base/text` is absent from the grid, paste of text is disabled.
  - `App.tsx:213-214,521`: pick → `addElement(c.widget)`; `canPick={(c) => canAdd(view?.slide.kind, c.key, snap.free_slots)}`.
  - `slideTools.ts`: delete `ADD_NAMES`, `AddName` and `packOnlyWidgets`; add `acceptsElements` and the new `canAdd`; fix the `canRemoveSlide` comment ("jeu de modèles" → "pack de layouts").
  - `usePackWidgets.ts`: memoize on `add_packs` content; drop the `lang` dependency and its comment (the session relocalizes `add_packs` on a language change; check that `Interface` triggers a fresh snapshot).
  - Delete `starterChart` (`chart/chartDeck.ts:25`) and `starterTable` (`table/tableDeck.ts:46`) and their tests; `Icon.text`, `Icon.card` and `Icon.shape` if unread (`rg -n 'Icon\.(text|card|shape)' ui/src`); the stale comment `icons.tsx:17`.
  - `builtinKinds.ts`: keep `BASE` and `baseName`; delete `ESSENTIEL` and `ESSENTIEL_NAMESPACE` if unread after this task (`packOps.ts` reads `ESSENTIEL_NAMESPACE`: keep it if so).
- Delete:
  - `PackNotice.tsx`, `PackNotice.render.test.tsx`, `packNoticeCommand.ts`, `packNoticeCommand.test.ts`;
  - the `App.tsx:341-342` block;
  - the `.pack-notice` CSS (`styles.css:865,877`, `workshop.css:140`) and the `notice` area of the `.table` grid template.
- Theme editor:
  - `ThemePanel.tsx:52`: the menu value is `theme.namespace` (t-1107).
  - `useThemeSettings.ts`: before the `name !== "token"` early return, handle `extends`. `context.edit` must be one of `add`, `remove`, `up`, `down`, and `context.theme` a non-empty string; anything else is ignored with `console.error`. Call `api.editThemeExtends(edit, theme)`, under the same refusal and relaunch contract as `token`.
- i18n `ui.ftl` (20 locales):
  - deleted: `ui-add-kind`, `ui-add-kind-description`, `ui-new-text`, `ui-new-card-title`, `ui-new-card-detail`, `ui-undo-add`, plus every TS read of them;
  - added: `ui-toolbar-packs`, `ui-toolbar-packs-update`, written as string literals so `i18n-keys.test.ts` finds them.
  - `i18n-keys.test.ts` checks en/fr only. Also run `rg -n '^ui-(add-kind|new-|undo-add )' crates/oku-slides-i18n/src/locales` (empty).
- Tests:
  - `slideTools.test.ts`: the `canAdd`/`acceptsElements` table below.
  - `newElement.test.ts`: below.
  - `widgetChoices.test.ts`: rewrite the native-order and `packOnlyWidgets` tests as `widgetSections with two packs keeps pack and widget order`.
  - `WidgetPicker.render.test.tsx`: rewrite the native-name tests (`:17-35,116`) on `mount({ packs })`; add `WidgetPicker with two packs shows two pack headings`.
  - `ThemePanel.render.test.tsx`: `ThemePanel with a base absent from the library uses its namespace as menu value`.
  - `useThemeSettings.render.test.tsx`: `useThemeSettings extends add event sends the edit`; `useThemeSettings extends event with a bad edit sends nothing`.
  - `ToolsTray.render.test.tsx` (create): `ToolsTray with pending updates shows the dot`; `ToolsTray in help mode hides the packs button`.
  - `usePackWidgets` test: same content gives the same array identity.
- `ui/ARCHITECTURE.md` / `ui/AGENTS.md`: the add grid, the packs button, the banner gone.

**Interfaces:**
- Consumes: Tasks 2–5.
- Produces: none.

- [ ] **Step 1: Write the failing tests**

```ts
// ui/src/slideTools.test.ts
import { expect, test } from "bun:test";
import { acceptsElements, canAdd } from "./slideTools";

const slots = { "essentiel/content": null, "essentiel/blank": null, "acme/split": ["base/text"], "acme/none": [] };
test.each([
  ["essentiel/content", "base/card", true],
  ["essentiel/blank", "base/text", true],
  ["essentiel/title", "base/card", false],
  ["acme/split", "base/text", true],
  ["acme/split", "base/card", false],
  ["acme/none", "base/text", false],
])("canAdd on %s with %s gives %s", (slide, kind, expected) => {
  expect(canAdd(slide, kind, slots)).toBe(expected);
});
test("acceptsElements on a layout without free slot gives false", () => {
  expect(acceptsElements("essentiel/title", slots)).toBe(false);
});

// ui/src/newElement.test.ts
import { expect, test } from "bun:test";
import { newElement } from "./newElement";

test("newElement with a seed style copies the style", () => {
  const el = newElement("base/shape", "s-1", { params: {}, style: { fill: "surface", radius: "md" } });
  expect(el.style).toEqual({ fill: "surface", radius: "md" });
});
test("newElement without a seed style has no style key", () => {
  expect(Object.hasOwn(newElement("base/text", "t-1", { params: { body: [] } }), "style")).toBe(false);
});
test("newElement with a __proto__ param keeps it as data", () => {
  const params = JSON.parse('{"__proto__": {"polluted": true}}');
  const el = newElement("acme/x", "x-1", { params });
  expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  expect(Object.hasOwn(el.params ?? {}, "__proto__")).toBe(true);
});
```

```tsx
// ui/src/WidgetPicker.render.test.tsx (with the file's existing `mount` helper, extended to take `packs`)
test("WidgetPicker with two packs shows two pack headings", () => {
  const root = mount({ packs: [pack("base", "Base", ["base/text", "base/card"]), pack("acme", "Acme", ["acme/badge"])] });
  expect([...root.querySelectorAll("section h3")].map((h) => h.textContent)).toEqual(["Base", "Acme"]);
});
```

`pack(namespace, name, kinds)` builds an `AddPackView` with seeds `{ params: {} }`.

- [ ] **Step 2: Run them to see them fail.** Run `cd ui && bun test`. Expected: FAIL.

- [ ] **Step 3: Implement** per the file list. Then `rg -n '"base/(text|card|shape|chart|table|image|stack)"' ui/src --glob '!*.test.*'` returns only `builtinKinds.ts`.

- [ ] **Step 4: Run the gate.** Run `just test` and `cd ui && bun x tsc --noEmit`. Expected: PASS.

- [ ] **Step 5: Stage and write `task-6-commit.txt`**: `feat(ui)! : bouton Packs du deck, grille d'ajout par pack amorcée par la session, slots libres, édition de extends, bandeau supprimé (t-1115, t-1107)`.

---

### Task 7: e2e — packs panel, grid, `extends`, and the flaky specs

Sub-task: t-1116.

**Files:**
- Create `ui/e2e/lib/packs.cjs`:
  - `createLibraryWidgetPack(p, tab, name)`: through the tab's HTTP API (`ui/e2e/lib/http.cjs`), it sends `edit_pack` on the built-in `base` pack id, `save_pack { as_new: true }` with the new name, then `close_pack`. It returns the new pack's id and namespace (from the `packs("widgets")` listing).
  - `deleteLibraryPack(p, tab, id)`: `delete_pack`.

  Read `http.cjs` and `atelier.cjs` for the request helpers and the tab id.
- Create `ui/e2e/250-packs.cjs`, after 240. Each step waits with `until` from `ui/e2e/lib/until.cjs`.
  1. The toolbar button « Packs du deck » opens the panel in the dock, with the three sections « Layouts », « Widgets », « Thème ».
  2. `base` shows its refusal caption (the fr text of `op-base-required`) and no « Retirer » button. `essentiel` shows the fr text of `op-pack-in-use`.
  3. Inside `try { … } finally { deleteLibraryPack }`:
     - after `createLibraryWidgetPack(…, "Acme")`, adding it from the Widgets select pins it;
     - it appears in the panel and as its own section in the add grid;
     - ⌘Z unpins it.
  4. A `base/card` added from the grid shows « Titre de la carte ».
  5. Choosing Pro in the panel's theme select changes the stage, and ⌘Z brings Papier back.
- Modify `ui/e2e/200-theme-editor.cjs`. In pack mode, on the « Thème » tab, the parent rows have French aria labels (« Monter », « Descendre », « Retirer »), and:
  - add « Pro » as a parent of the Papier variant;
  - move it up;
  - remove it;
  - ⌘Z brings it back;
  - the theme being edited is not offered by « Ajouter un parent ».
- Modify `ui/e2e/020-toolbar.cjs:30-38`:
  - the card list is now the « Base » section: Texte, Carte, Image, Forme, Graphique, Tableau, Pile, in registry order (read the order from the session's `add_packs` once, then assert it);
  - the undo label becomes « Annuler : Ajouter « Graphique » ».
- Fix every other spec that names the old native cards: `rg -n "'Carte'|'Forme'|'Graphique'|'Tableau'|'Texte'|Ajouter un graphique|Ajouter une carte" ui/e2e/*.cjs`.
- **t-1108**:
  - `220-import.cjs`: wrap the `edit_pack` / `host_theme` / `close_pack` round trip in `try … finally`.
  - `170-code-layout.cjs`:
    1. Record a failure first: run `just e2e 4999` up to three times, or until 170 fails. Keep the failing log in the commit-message file, or note that it did not reproduce in three runs.
    2. The likely race is a null `boundingBox()` at `:63` and `:83` while the divider remounts. Replace those single reads with `until(p, async () => (await divider.boundingBox()) !== null, …)` before using the box.

**Interfaces:** none.

- [ ] **Step 1: Write the new spec, the helper and the changed steps.**
- [ ] **Step 2: Run** `just e2e 4999`. Expected: PASS, 250 included. `just e2e` sets its own data dirs.
- [ ] **Step 3: Stage and write `task-7-commit.txt`**: `test(e2e) : panneau des packs, grille par pack, extends ; 170 stabilisé, 220 en finally (t-1116, t-1108)`.

---

### Task 8: Docs, rules, consolidation and full gate

Sub-task: t-1117.

**Files:**
- `docs/aide/{en,fr}.json`:
  - the add grid (one section per pack, starting content);
  - a new help slide « Les packs du deck » / "Deck packs", placed after the help slide on themes (`rg -n '"s-' docs/aide/fr.json` for the ids; new id `s-packs`);
  - the theme editor's parent list.
  - Keep the strings `crates/oku-slides-session/src/help_tests.rs` quotes, or update that test.
- `typst/oku-slides/README.md`: the `styles` manifest key, and `ftl:` markers being reserved to built-in packs.
- The spec, if Step 1 finds a gap.
- Every `AGENTS.md` and `ARCHITECTURE.md` that names the pack banner, `pack_notice`, `ADD_KINDS`, `ADD_NAMES`, native cards or `Place::Banner`. Find them with `rg -n -i 'pack banner|bandeau|pack_notice|ADD_KINDS|ADD_NAMES|native card|Place::Banner' --glob '**/AGENTS.md' --glob '**/ARCHITECTURE.md'`.
- Oku:
  - the t-1008 body: lot 5 done, the refactor complete;
  - sub-task statuses.

**Interfaces:** none.

- [ ] **Step 1: Spec update check.** The spec changes listed under "Deviations" are committed with this plan. Verify with `git log -p -- docs/superpowers/specs/2026-10-18-packs-layouts-widgets-themes-design.md`.
- [ ] **Step 2: Consolidation pass (G10).** Each of these returns nothing:
  - `rg -n 'pack_notice|PackNotice|packNotice|Place::Banner|"banner"|pack_use_mine|pack_make_current|pack_notice_close' crates ui/src` (besides `pack_use_mine`/`pack_make_current` as the new per-pack session methods);
  - `rg -n 'ADD_NAMES|AddName|packOnlyWidgets|newPackWidget|starterChart|starterTable|ui-add-kind|ui-new-(text|card)|draft_extends_names' crates ui/src`;
  - `rg -n 'catalog:v3' crates ui/src`.

  Then check by hand:
  - one relation function (`deck_packs.rs`), one removal check (`remove_pack_refusal`), one R15 helper (`owned_current`), one example localization (`example_text.rs`);
  - `just agents-lint`.
- [ ] **Step 3: Full gate.**
  - Run `just ci` and `just e2e 4999`. All green.
  - The golden snapshots are unchanged: `git diff main -- crates/oku-slides-render/tests/snapshots/golden__reference_pages.snap crates/oku-slides-render/tests/snapshots/golden__reference_boxes.snap` is empty.
- [ ] **Step 4: Commit**: `docs(packs) : aide, README et règles du lot 5, consolidation (t-1117)`.
- [ ] **Step 5: Finish.**
  1. Run `git config --worktree --unset oku.review`.
  2. Merge `main` into the branch, then rerun the gate if `main` moved.
  3. Squash into `main` with the `BREAKING CHANGE:` footer of the Global Constraints.
  4. Push. If the push fails on the SSH key, ask the user to unlock 1Password.
  5. Remove the worktree and the branch.
  6. Mark t-1117 done.
  7. Close t-1103, t-1104, t-1105, t-1106, t-1107, t-1108, t-1109, t-1091, t-1092 and t-1093.
  8. Comment on t-1057 (point 2 done; points 1 and 3 unchanged) and on t-1053 (still open, reason unchanged).
