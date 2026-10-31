# A2UI playbook: a dev gallery and an agent reference

Task t-1312.

Today nothing shows the A2UI catalog `urn:oku-slides:a2ui:catalog:v4` as a whole: its 25
components are seen only where the app happens to use them, and the agent learns the nine
it may `render` from a hand-written line in the `render` tool description
(`crates/oku-slides-mcp/src/tools.rs`). After this change **one Rust source of examples**
feeds a dev-only gallery page that renders every component in its meaningful variants, the
`render` description (generated from the catalog's JSON schema) and a new `a2ui_examples`
MCP tool that returns props and examples on demand.

## Decisions (made on 2026-10-30)

1. **Scope: A2UI only.** Typst slide widgets (`packs/base/widgets`) stay out.
2. **Both audiences from one source**: a visual gallery for developers and a reference for
   the agent.
3. **The source is Rust** (`oku-slides-a2ui`), as typed `Component` values. Rejected: TS
   examples next to `catalog.tsx` (Rust would read untyped data, no compile-time coverage of
   `ComponentKind`); Markdown with JSON blocks (a third format, checked only by tests).
4. **A custom dev page, not Storybook.** Storybook indexes stories by statically reading
   named exports, so stories generated from a JSON fixture need its unstable custom indexer,
   and its test tooling is Vitest while the repo runs `bun test`. A later move stays cheap:
   an indexer over `playbook.json` needs no change to the Rust source.
5. **The gallery is dev-only**: a second Vite HTML entry, not in the app build, no e2e
   screenshot (the render test covers it).
6. **The agent gets both** a short generated list in the `render` description and the
   `a2ui_examples` tool for full props and examples.
7. **Signatures come from `JsonSchema`** derived on `ComponentKind` and its prop types, not
   hand-written; **prop descriptions are exposed** in `a2ui_examples`, cleaned of rustdoc links.
8. **Each example stores its group** (Layout, Atom, Field, HostOnly), computed by one
   exhaustive `match` on `ComponentKind`; HostOnly must agree with `Component::agent_allowed`.
   (Revised while planning: no Composite group; every composite is either a field,
   `ListTable` and `RichTextField`, or host-only.)
9. **`Receipt` shows its no-thumbnail state** in the gallery: the page has no deck, so its page
   hash does not resolve. `SlideThumb` carries its SVG as a prop: its example holds a tiny
   literal SVG. No mock thumbnail cache.
10. **"Agent" means "`render` accepts it", not `agent_allowed`.** (Found while planning:
    `render` also refuses the fields, which go through `show_params`, and the primary, danger
    and icon button styles and the danger tone.) Each example states `agent` by hand, and a
    session test checks `agent == render accepts the example` for every example. Only
    components with an agent example reach the agent, and only the props and enum values
    those examples use, so the agent never reads a prop or value `render` refuses.

## Rust source (`oku-slides-a2ui`)

New module `src/playbook.rs` (tests in `src/playbook_tests.rs`), data only:

```rust
/// One playbook example: a component in one variant, as a small surface.
pub struct Example {
    pub component: &'static str, // v0.9 name, "Button"
    pub variant: &'static str,   // "default", "danger", "with-icon", "bound"…
    pub note: &'static str,      // one English sentence: when to use it
    pub group: Group,            // Layout | Atom | Field | HostOnly, from the root's kind
    pub agent: bool,             // render accepts it (checked by a session test)
    pub components: Vec<Component>, // the tree, one id "root"
    pub data: serde_json::Value, // data model, {} for literal surfaces
}
pub fn examples() -> Vec<Example>;
```

- **Coverage**: a private `kind_info(&ComponentKind) -> (&'static str, Group)` matches every
  variant with no `_` arm, so a new variant does not compile until the playbook names and
  groups it; a test checks that the components of `examples()` are exactly the schema's 25. The `ARCHITECTURE.md` extension point becomes "a variant, its examples,
  and `catalog.tsx`".
- **Variants** are those that change the rendering or the events, not a combinatorial
  matrix: tones, `TextVariant`, primary / danger / icon buttons, `{path}`-bound fields,
  a `Notice` with a button, a nested `ListTable`, a `Disclosure` with a child, small realistic
  `ChartData` / `TableData` / `Timeline` / `Receipt` data.
- **Content strings are English literals.** They are fixture data, not interface strings.
- `variant` is unique within a component; `group == HostOnly` iff the root fails
  `agent_allowed`; every example's tree is valid (one root, ids unique, children exist).
- The crate gains `snafu` (workspace dependency) for `PlaybookError`, an opaque error with a
  backtrace for a schema the generator cannot read. It is a programming fault: the MCP tool
  logs it and tells the agent the existing generic `agent-tool-failed` message.

### Two serializations

- **UI fixture**: each example as `{component, variant, note, group, agent, messages}`,
  where `messages` are the v0.9 `createSurface` + `updateComponents` + `updateDataModel` of a
  surface `playbook-<component>-<variant>`, the shape of a `Surface` in `agent-surfaces.json`.
  Written to `ui/src/a2ui/fixtures/playbook.json` by a test that compares it with the file and
  rewrites it under `OKU_UPDATE_FIXTURES=1` (the `every_form_matches_the_ui_fixture` pattern).
- **Agent output**: the agent examples grouped per component (below), each surface in the
  exact input shape of `render`: `[{id, component, …}]`.

### Schema, signatures and descriptions

- Derive `schemars::JsonSchema` on `Component`, `ComponentKind` and every type its variants
  reach (`Tone`, `TextVariant`, `ButtonIcon`, `Dynamic<T>`, `Action`, `Event`, `ListColumn`,
  `SubList` and `SubListRaw` (its `try_from` source), `SubColumn`, `ColumnKind`,
  `SeriesSwatch`, `TimelineStep`, `StepStatus`, `ToolCallDetail`, `PromptImage`,
  `DiffSegment`, `DiffKind`, `SlideRef`…).
- The schema is generated once with `SchemaSettings` carrying a `Transform` that rewrites
  every `description`: `` [`X`](path) `` and `` [`X`] `` become `` `X` ``, and only the first
  paragraph is kept. No doc comment is rewritten by hand.
- `ComponentKind` is internally tagged (`tag = "component"`): its schema is a `oneOf` whose
  branches carry `"component": {"const": "<Name>"}`. The generator reads names, `required`,
  `properties`, string enums reached through `$ref`, and array item names from it.
- `pub fn agent_reference() -> Result<Vec<ComponentReference>, PlaybookError>`: per component
  with an agent example, in catalog order, `props: [{name, required, type, values?,
  description}]` and `examples: [{variant, note, surface}]`. Props are the required ones
  plus the optional ones some agent example sets; `values` (a string enum's values) keeps
  only those agent examples use. `Dynamic<T>` reads as `T` (agent surfaces are literal); an
  array of a named type reads `"[DiffSegment]"`; any JSON reads `"any"`.
- `pub fn render_components(reference: &[ComponentReference]) -> String`: that reference as
  `Name{prop, prop?: a|b|c}` (optional props get `?`, enums list their values),
  comma-separated.
- The generator fails (and its tests with it) when an `agent_allowed` variant has no
  `oneOf` branch or a `$ref` does not resolve. Errors are Snafu with a backtrace.
- The `ARCHITECTURE.md` line saying `ChoiceOption` is the only type with a JSON schema is
  replaced: the catalog has a schema, its descriptions reach the agent only through the
  cleaning transform. `ChoiceOption`'s own docs stay free of rustdoc links: it is also
  embedded in MCP input schemas generated outside the transform.
- Prop doc comments found in French on these types are translated in the same commit.

## Gallery page (`ui/`)

- `ui/playbook.html` and `ui/src/playbook/main.tsx`: Vite serves any root HTML file in dev, and
  `vite build` bundles only `index.html` (no `rollupOptions.input`), so the page never ships.
  `main.tsx` loads i18n and the scheme as `src/main.tsx` does, then mounts `<Playbook>`.
  Opened with `just web` at `http://localhost:5173/playbook.html`.
- `ui/src/playbook/Playbook.tsx`: four sections (Layout, Atoms, Fields, Host only). Each
  example is a card: component and variant, note, an "agent" badge, the live surface through
  `SurfaceView`, and a collapsible JSON view of its components.
- Header: a light / dark switch (`applyDefaultScheme`), a filter on component names, and an action
  log: `onAction` appends each event a click or an edit sends. `onError` shows the error in
  that example's card.
- Page strings are English literals (dev-only, not shipped). `i18n-keys.test.ts` only reads
  `"ui-…"` literals, so it needs no exception, but the `t()` rule of `ui/AGENTS.md` gets one,
  in the same commit. Controls come from `ui/src/primitives.tsx` (`Button`, `Field`), as
  that file's rule requires; each example sits in its own error boundary, and the page is
  its own scroll container (`body` does not scroll in the app's stylesheet).

## Agent side

- **`render` description**: the hand-written `Components: …` list is replaced by
  `render_components()`; the rules stay hand-written (literal values only, the `suggest`
  action, 40 components, 6 levels, 64 KiB, `report` for conclusions), plus "Call
  `a2ui_examples` for a component's props and full examples." The description is built at
  tool listing like `import_diff_description()`.
- **`a2ui_examples` tool**: input `{ components?: [String] }` (absent: every agent component);
  output `[{component, props, examples}]` from `agent_reference`. Its own `a2ui_tools`
  module, `Kinds::ALL`, an entry in the local agent's `families_of`.
- **Errors**, three `ToolError` variants with Fluent keys in all 20 locales (the i18n rule),
  and a fourth, `Playbook`, for a generator fault (the generic `agent-tool-failed`):
  - an unknown component name: the message lists the valid names;
  - a catalog component `render` refuses: the message names the tool that shows it
    (fields: `show_params`; `Receipt`, `SlideThumb`: `report`; `ThemeChoice`: `show_themes`),
    or says it is reserved to the host (`Prompt`, `Timeline`, `Notice`, `ChartData`,
    `TableData`).
- `Render.components`' doc says catalog v3: corrected to v4.

## Tests

- `oku-slides-a2ui`: variant names unique per component; group matches `agent_allowed`; every
  tree valid; every Layout and Atom component has an agent example; the UI fixture is
  current; `render_components` equals its exact expected string, prop types are read right
  (`string`, `[DiffSegment]`, `Action`), and an `insta` snapshot holds the full
  `agent_reference()`; no exposed description is empty or still contains `](` or
  `crate::`; each generator fault (missing branch, undocumented prop, unresolved or cyclic
  `$ref`, unknown shape) is reported as its own variant, not skipped.
- `oku-slides-session` (next to `agent/render.rs`): for every example, `render` accepts it iff
  `agent` is true, so what `a2ui_examples` shows, `render` accepts.
- `oku-slides-mcp`: `a2ui_examples` with no input, an empty list, a list, a mixed list, an
  unknown name, too many names, field and host-only names, and `ThemeChoice` in a pack tab;
  error tests on the variant and on the `fr` text differing from `en`, not on English text;
  the `render` description contains `render_components()`. The local agent's compacted
  descriptions keep the component list.
- `ui`: `src/playbook/playbook.schemas.render.test.tsx` validates every fixture entry with the
  catalog's zod schemas (the `catalog.schemas.render.test.tsx` pattern);
  `src/playbook/Playbook.render.test.tsx` mounts the page and checks that the four groups and
  every example render with no `onError`.

## Docs

`oku-slides-a2ui/ARCHITECTURE.md` (playbook, schema, extension point; it stays at most 50
lines), `oku-slides-mcp/ARCHITECTURE.md` (the tool, on its existing tool-families line: the
file is at 50 lines), an invariant in `crates/oku-slides-a2ui/AGENTS.md` (every new variant gets
examples, `agent` true only when `render` accepts them), and a Commands line in the root
`AGENTS.md` (`just web`, then `/playbook.html`). `ui/` has no `ARCHITECTURE.md`; none is created. A rule changed or added by
the work goes to the nearest `AGENTS.md` in the same commit.

## Out of scope

Typst widgets; Storybook; e2e screenshots; using the catalog schema as `render`'s MCP input
schema (still `Vec<serde_json::Value>`); a mock thumbnail for `SlideThumb` / `Receipt`.

## Planning revisions (2026-10-31)

Found while planning and reviewing the plan (`docs/superpowers/plans/2026-10-30-playbook-a2ui.md`);
they refine the decisions above, none reverses one.

- `agent_reference()` is computed once and cached:
  `Result<&'static [ComponentReference], &'static PlaybookError>`. `render`'s description
  and `a2ui_examples` share it.
- The generator reads `any` only for a schema that accepts any JSON; any other shape it
  cannot read, and a `$ref` chain deeper than 8, are `PlaybookError`s.
- `ComponentKind::is_field()` names the eight data-bound inputs once: `render` refuses them
  through it and the playbook's tests check `Group::Field` against it. Refusals in
  `a2ui_examples` match on the root's `ComponentKind`, never on a name.
- `ThemeChoice` points to `show_themes` only in a tab that lists it (deck and help);
  elsewhere it is reported as host only.
- `a2ui_examples` refuses unknown argument keys and more than 25 names, and echoes a refused
  name only as ASCII letters and digits, at most 32.
- The `render` and `a2ui_examples` descriptions put what the agent needs in their first
  sentence: the local agent's `compact()` keeps only that one.
- The UI fixture entry also carries its `surface` id. The Timeline example uses typed
  literal `TimelineStep`s. `SubListRaw` derives the schema (schemars reads a `try_from`
  source), with `SubList`'s field docs.
- Enum values stay cut to those the agent examples use (decision 10): `Card` and `Badge`
  show `tone?: accent`, not `neutral`.
