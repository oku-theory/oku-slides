# Typed packs: layouts, widgets and themes over a widget-free core — design

Task: t-1008. Date: 2026-10-18.
Status: approved in conversation 2026-10-18, pending review of this written spec.
The user's answers: several packs per deck; pack widgets named by a declared namespace
(`base/card`); no migration of existing decks; charts, tables and stack leave the core too;
three typed pack kinds (layouts, widgets, theme); a theme `extends` zero or more themes and the
deck keeps its own token overrides (`deck.tokens`, which exists today); component tokens are declared by the pack that owns the
component; the core keeps no widget at all; `canvas`, templates (gabarits) and the Grilles pack
are deleted; `citation` moves into `base` (revised by the lot 2 plan: no `citation` widget exists, it is a template).

## Problem

Today the Typst library `typst/oku-slides` hard-codes fourteen widgets (`text`, `card`, `image`,
`shape`, `chart`, `table`, `stack`, `canvas` and the slide widgets `title`, `toc`, `chapter`,
`content`, `end`, `blank`). A deck points at one pack (`deck.templates: Option<PackRef>`) that
mixes slide templates and widgets, plus one theme (`deck.theme`) whose tokens also hold the
component tokens of the built-in widgets (`component.card`, `component.table`…).

This blocks three things:

- A widget cannot be shipped, versioned or replaced like a pack widget once it is built in;
  `card` and `shape` are the ones the import and the UI lean on most.
- A deck cannot combine packs: a deck on Grilles has no Essentiel widgets and vice versa, and an
  imported deck lives on a deck-only copy of one pack.
- A future marketplace needs packs that coexist without name clashes, depend on each other, and
  can be styled by any theme.

## Goals

- Everything visible is a pack: the core is an engine, not a widget library.
- Three typed pack kinds, each versioned, namespaced and embedded in the deck.
- A deck combines N layout packs, N widget packs and one theme, plus its own token overrides.
- Pack names never clash; a broken pack disables only itself (and what requires it).
- Rendering of existing content is pixel-identical after the move (the golden test).

Non-goals: the marketplace itself (discovery, publishing, signing, trust); migrating existing
decks; version ranges in `requires` (a deck pins exact versions, as today).

## 1. The core

The library `typst/oku-slides` keeps only the engine: `widget()`, the param types `p.*`
(`plain`, `rich`, `number`, `flag`, `choice`, `asset`, `list`, `slot`, `grid`, `host`), `field`,
`fields-stack`, `list-item`, zones and grids (`free-area`, `zone-marker`), token resolution
(`color-of`, `size-of`…), the slide frame helpers used by layouts, and the fallback frame.
It registers **no widget** and owns no bare name. `canvas` is deleted (no production use).

## 2. Pack kinds

All three share a trunk: `id` (UUID), `namespace`, content-addressed versions (sha-256),
`parent` (version lineage), `created`, `scope` (`library` or `deck`).

| Kind | Manifest | Files | Links |
|---|---|---|---|
| Layouts | `layouts.json` | `layouts/<name>.typ` (widgets of category `slide`), `_commun` | `requires`: widget namespaces |
| Widgets | `widgets.json` | `widgets/<name>.typ` (element widgets), `_commun`, `examples` | `requires`: widget namespaces |
| Theme | `theme.json` | none | `extends`: ordered theme namespaces |

- `namespace` matches `[a-z][a-z0-9-]*(\.[a-z][a-z0-9-]*)?` (the dotted form is reserved for
  `publisher.name` later), is unique in the library across all three kinds, and is inherited by
  every new version, fork and deck-only child of a pack.
- Layout and widget packs declare the defaults of their **component tokens** in their manifest,
  under `tokens`, as values or references to global tokens (`color.surface`, `space.md`). They
  are exposed to Typst and to the theme editor as `component.<namespace>.<name>`.
- A Typst file of a pack never `import`s a file of another pack; the probe refuses it
  (`CrossPackImport`). A layout reaches another pack's widget through the registry in `ctx`.
- Templates (pre-filled slides) no longer exist: `Template`, `PackManifest.templates` and the
  template metadata of `Slide` are deleted. A layout's defaults and `examples` replace them.
  (Revised by the lot 3 plan: a layout manifest also declares `profiles`, named presets of one of
  its layouts with fit hints, that the retrofit import scores across every target layout pack; a
  pack's kind fixes its widgets' category, a file of the other category is dropped at the probe;
  `deck-layouts` is reserved for the deck's own layout pack, as `deck` is for its widget pack.)

### Built-in packs

| Namespace | Kind | Content | Requires / extends |
|---|---|---|---|
| `base` | widgets | `text`, `image`, `card`, `shape`, `chart`, `table`, `stack` | — |
| `essentiel` | layouts | `title`, `content`, `chapter`, `toc`, `end`, `blank` (new id in lot 3) | requires `base` |
| `pro`, `papier` | themes | today's global tokens, without `component.*` | — |

Grilles is deleted. (Revised by the lot 2 plan: no `citation` widget exists — `citation` is an Essentiel
template — so `base` holds seven widgets.) The component tokens
now in `default-tokens.json` and the themes move to the pack owning the component (`card`,
`table`… to `base`; `title`… to `essentiel`), with values that render identically.

## 3. The deck

`Deck` loses `templates` and gains (revised by the lot 1 plan: `widgets` is a map
`namespace → PackPin { id, version, requires }`, and `layouts` follows the same shape in lot 3):

- `layouts: Vec<PackRef>` and `widgets: Vec<PackRef>`, sorted by namespace in canonical form,
  at most one version per namespace (revised by the lot 3 plan: `layouts` is a map of `PackPin`s
  like `widgets`; `requires` of either kind names widget namespaces; a namespace pinned under one
  kind is refused under the other with `PackKindConflict`);
- `theme: ThemeRef` and every ancestor theme reachable through `extends`, embedded.

`deck.tokens` (the deck's own token overrides, edited by `SetToken`, `UnsetToken`, `SetTokens`)
is unchanged and applied last.

**Closures are materialized.** Adding a pack embeds the current library version of every
namespace it requires that the deck lacks, recursively. Setting a theme embeds its whole
`extends` graph. Rendering and validation read flat lists, never resolve anything transitively,
and a deck renders identically whatever the library holds (the embedded version is authoritative,
as today).

**Names.** `Element.kind` and `Slide.kind` are always `namespace/name` (`base/card`,
`essentiel/content`). The pack assembly derives the prefix from the verified manifest; the
`widget("card", …)` declaration in the file stays short, so a pack cannot claim another pack's
namespace. Validation reports `unknown-widget` for a kind whose namespace is not in the deck or
whose name is not in that pack.

**Ops.** `Op::SetTemplates` is replaced by `Op::AddPack { kind, pack }` (adds or replaces the
version of that namespace, plus missing requirements) and `Op::RemovePack { kind, namespace }`,
refused with `PackRequired { by }` when another embedded pack requires it and with
`PackInUse { elements }` while an element or slide uses one of its widgets. (Revised by the lot 3
plan: `AddPack` and `RemovePack` carry `kind: widgets | layouts`.) `Op::SetTheme`
embeds the `extends` graph. All go through the replica like today's ops, with one undo step
each.

**Format.** `SOURCES_VERSION` is bumped. An older deck is refused on read with a localized
`SourcesVersionTooOld` error. No migration code is written (MVP rule, precedent: t-532).
The sources zip stores `layouts/<hex>/`, `widgets/<hex>/` and `themes/<hex>/`, add-only and
verified on read as today.

## 4. Theme tokens

A theme's `extends` is an ordered list. Effective tokens are applied in this order:

1. global defaults (`default-tokens.json`, the core);
2. component token defaults of every embedded layout and widget pack;
3. the `extends` graph, linearized: depth-first post-order, each theme applied **once** at its
   first position, later parents winning over earlier ones;
4. the deck's theme;
5. `deck.tokens`.

Merging is deep by token path: an object merges key by key, a scalar or a list (`font.body`)
replaces. A cycle is refused with `ThemeCycle { chain }`. Example: `marque` extends
`[papier, sombre]`, both extend `neutre` → defaults, packs, `neutre`, `papier`, `sombre`,
`marque`, `deck.tokens`.

The theme editor's schema (`theme_schema.json`) becomes the global section plus one section per
embedded pack, built from that pack's declared component tokens.

## 5. Rendering

- `PackWidgets` becomes `PackSet`: the deck's embedded layout and widget packs, each served under
  `/packs/<hex>/` with a generated assembly module; the registry is a dictionary
  `namespace/name → widget`. `Registries` memoizes per pack version, so changing one pack never
  re-probes the others. `probe_file` is unchanged apart from the cross-pack import check.
- Zone sizing no longer reads names (`widgets.typ:385`): a widget declares
  `grow: false | true | "fit"` in its schema; `essentiel/content` reads it from each block.
- Charts and tables keep their Rust passes, keyed on the host types `host("chart")` and
  `host("table")`, never on a widget name. Any pack widget may declare a host param.

### Per-pack fallback

A pack that fails to probe, is marked faulty by the worker (`PackDisabled`) or is missing from
the sources is disabled **alone**. Its elements and the slides of its layouts render as the
fallback frame (widget name and reason); one notice names the pack. Packs requiring a disabled
pack are disabled too, with a cascading notice. The old "fall back to the base widgets" path is
removed: there is no base registry any more.

## 6. Consumers

- **Import** builds decks on `essentiel` + `base` (revised by the lot 3 plan: the retrofit targets
  one or more layout packs, default `essentiel`, and scores their `profiles`; it has no child pack;
  the hi-fi import puts its generated slide widgets in a deck-only `deck-layouts` layout pack). Every widget name it emits comes from one
  constants module (`base/card`, `base/shape`, `base/stack`, `base/table`, `base/chart`,
  `essentiel/content`…); `place.rs` and `measure.rs` stop matching bare names. The generated
  theme becomes `deck.tokens` on a built-in theme instead of a deck-only theme, so deck-only
  themes (`PackScope::Deck` on `theme.json`) disappear. The
  agent's deck-only child pack stays a deck-only widget pack inheriting its parent's namespace.
- **Agent (MCP)**: the widget and layout catalogue is grouped by pack with qualified kinds; new
  tools `add_pack` / `remove_pack` map to the ops; the widget-writing tools target a layout or
  widget pack namespace; the receipt (`receipt.rs`) and prompts use qualified names.
- **Pack mode** (lot 3 plan): templates mode becomes pack mode; a layout pack's working deck has
  one slide per layout whose params are saved as that layout's example; requests, MCP tools and
  UI move from "templates" to "pack(s)" (`edit_pack`, `save_pack`, `close_pack`, `delete_pack`,
  `packs`, `layouts`).
- **UI**:
  - `ADD_KINDS` and the hard-coded `newElement.ts` cases go: the add grid lists the widgets of
    every embedded widget pack, grouped by pack, seeded from `examples` or schema defaults;
  - adding a slide lists layouts instead of templates;
  - the pack banner (A2UI surface) becomes a "Deck packs" panel with three sections (Layouts,
    Widgets, Theme): version, add from the library, remove (refusals give their reason),
    new-version notice;
  - `TargetPanel.tsx` and `chartDeck.ts` pick panels by the host type of a param, not by kind;
  - the theme editor shows `extends` and the deck overrides.
- **CLI**: `render`, `check`, `import` read and write the new format; `--pack` flags take a
  namespace.
- **Docs**: `docs/aide` (en, fr), `typst/oku-slides/README.md`, every touched
  `ARCHITECTURE.md` and the `AGENTS.md` rules that name `frise`, `card`, the base registry or
  `PackScope::Deck` themes.

## 7. Errors

Snafu, a backtrace in every variant, opaque public types, Fluent messages (en reference, fr):
`NamespaceInvalid`, `NamespaceTaken`, `UnknownNamespace`, `PackRequired { by }`,
`PackInUse { elements }`, `ThemeCycle { chain }`, `CrossPackImport { file, target }`,
`SourcesVersionTooOld { found }`. Tests assert on keys or variants plus one non-English locale.

## 8. Testing

- **Golden render**: the golden deck is rewritten with qualified kinds on `essentiel` + `base`
  and must stay pixel-identical. Any diff is a bug, never a re-record.
- **Registries**: `fixtures/widgets/base-registry.json` and its snapshot are replaced by one
  snapshot per built-in pack (revised by the lot 2 plan: one combined core + `base` fixture until
  lot 3; revised by the lot 3 plan: one fixture `fixtures/widgets/builtin-registry.json` holding
  the probe of `base` + `essentiel`, and one render snapshot per built-in pack; the probe has no
  `base` key any more since the core registers nothing).
- **Core** (rstest, speculoos): `requires` closure, `AddPack` replacing a version, `RemovePack`
  refusals, namespace validation, `extends` linearization (single, multiple, diamond, cycle,
  last parent wins, list replaces), canonical order of `layouts` / `widgets`.
- **Render**: cross-pack import refused, `grow` read from the schema, a pack widget with
  `host("chart")` and `host("table")` rendered through the Rust passes.
- **Session**: per-pack fallback and its cascade, as regression tests under `tests/regressions`.
- **Import**: snapshots regenerated; the review checks that only kinds and the theme (now
  `deck.tokens`) changed.
- **UI / e2e**: add and remove a pack, add a pack widget, add a slide from a layout, a theme
  with `extends` applied.

## 9. Delivery

One spec, five plans, each merged green on its own. Revised 2026-10-18 with the lot 1 plan
(`docs/superpowers/plans/2026-10-18-packs-lot1-widget-packs.md`, see its deviations):

1. Widget packs: namespaces, `requires`, `deck.widgets` as a map of `PackPin`s by namespace,
   `AddPack` / `RemovePack`, sources 11, qualified registry, multi-pack rendering and worker
   protocol 4, per-pack fallback, explicit write target with a `deck` pack for the deck's own
   widgets. Core widgets keep bare names.
2. Pack `base`: the seven element widgets out of the core, `grow` and `typography` in the schema,
   host-typed consumers (`Snapshot.host_params`), import constants, `canvas` deleted, deck version 7,
   built-in pack versions trusted by the per-pack fallback, one combined core + `base` registry
   fixture until lot 3 (see `docs/superpowers/plans/2026-10-18-packs-lot2-base.md`, deviations).
3. Layout packs and pack `essentiel`: `deck.layouts`, the six layouts out of the core,
   templates and Grilles deleted (see `docs/superpowers/plans/2026-10-19-packs-lot3-essentiel.md`,
   deviations: kinded pack ops, category fixed by pack kind, layout `profiles` for the retrofit,
   pack mode, renamed vocabulary, deck 8, replica 6, sources 12, worker protocol 5).
4. Themes: `extends`, linearization, component tokens declared by packs, composed editor
   schema, import writing `deck.tokens`.
5. UI: the "Deck packs" panel, add grid and slide add from packs, theme editor.

Plans 2 and 3 depend on 1; 4 on 1; 5 on all. Each lot updates every hard-coded kind in the UI
and the agent that it changes, so the app works after every lot.
