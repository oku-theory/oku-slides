# Typst widgets with named parameters — design

Date: 2026-09-27 · Task: t-270

## Goal

Slide components (card, text, image, shape, chart, group) and
the slides themselves become **widgets declared in Typst**, each
with a schema of named, typed parameters. The deck model moves from a
closed Rust enum (`ElementBody`, `SlideKind`) to a generic form
`{ id, kind, params }`, validated against that schema.

This is the foundation of field editing: a card exposes `kicker`,
`title` and `body`, three Typst parameters, which in-place editing will be
able to target separately.

Program in three sub-projects, each with its spec:

1. **Typst widgets and generic model** (this spec, t-270).
2. In-place editable fields per widget (t-271): one editor per field, at its
   exact position, placeholders for empty fields, Tab between fields.
3. Inline near-rendering in the editor (t-272): markdown marks hidden
   away from the cursor, style faithful to the field.

Out of scope here: the per-field editor and the near-rendering (sub-projects 2
and 3), widgets embedded in a deck, reading v1 decks.

## Decisions

- **Schema declared in Typst, read by introspection.** A single source of
  truth: the `.typ` file that renders the widget also declares its
  parameters. Rejected: a JSON manifest alongside (two files that
  diverge) and deduction from the function signature (Typst
  exposes neither the names nor the types of the parameters).
- **Everything becomes a widget**, slides included. `image`, `shape`, `chart` and
  `group` too; the `ElementBody` enum goes away.
- **Widgets provided by the library and by template packs.** A
  deck carries no Typst code; it references a pack by its name.
- **A slide is a root widget.** Its element containers are
  `slot` parameters.
- **Explicit JSON**: `{"id", "kind", "params": {…}}`.
- **No backward compatibility.** `DECK_VERSION` moves to 2; a v1 deck
  (including its Loro sources) is not read. Fixtures and templates are
  rewritten.
- **Vocabulary.** A *field* (`field`) is a text region editable
  in place, bound to a parameter; a *slot* is a parameter that contains
  elements. "Zone" keeps its current meaning: the body zone where
  free elements are placed (`<oku-slides-zone>`, `ZoneBox`). The model's
  `Zone { Blocks, Elements }` enum goes away in favor of slots.

## Typst side

### Pack

The `typst/oku-slides` library provides the base pack. A template
pack becomes a folder (built in or in the user's data
folder):

```
templates/grilles/
  pack.json        # id, name, description, template slides (formerly grilles.json)
  widgets.typ      # optional: the pack's widgets, added to the base
```

### Declaration

`src/widget.typ` exports `widget`, `field` and `p`:

```typst
#let card = widget("card",
  label: "Carte", category: "element",
  params: (
    kicker:    p.plain(label: "Surtitre"),
    title:     p.plain(label: "Titre"),
    body:      p.rich(label: "Corps"),
    highlight: p.flag(label: "Mise en avant", default: false),
  ),
  (args, ctx) => { … field(ctx, "kicker", args.kicker, style: …) … })
```

`p.*` descriptors, all serializable as a dictionary
`(type: …, label: …, default: …, bind: …)`:

| Descriptor | Value |
|---|---|
| `p.plain()` | single-line text, without formatting |
| `p.rich()` | paragraphs of runs (`Paragraph`) |
| `p.flag()` | boolean |
| `p.number(min:, max:)` | number |
| `p.choice(..values)` | one value from a list |
| `p.asset()` | asset identifier `sha256:…` |
| `p.slot(mode: "flow" \| "free", accepts: (..kinds) \| none)` | list of elements (`none`: any element widget) |
| `p.grid()` | grid `{columns, rows, gap}` |
| `p.host("chart")` | type provided and validated by the Rust host |

`bind: "<key>"` (on `plain`) binds the parameter to `deck.params.<key>`: the
value is read and written there. The cover uses it for `title` and
`subtitle`.

`widget(name, label:, category:, params:, render)` returns
`(name, schema, render)`. In `schema`, the parameters form an ordered
array `((name: …, type: …, …), …)`: the declaration order is the editing
order (Tab) and the inspector's. `category` is `"slide"` or `"element"`. A
pack's registry is the `registry` array exported by its `widgets.typ`;
a name already present in the base **replaces** the base widget (see
"Override").

### Fields

`field(ctx, name, content, style:)` generalizes `tracked()`: it emits
`#metadata((owner, field, page, x, y, w, h, ax, aw, style))<oku-slides-field>`
with the style probe (`text-probe`), then the content. `owner` is the id of
the element or of the slide, read from `ctx.id`. A separate label, rather than
`<oku-slides-box>`, keeps element boxes intact: the render
stores them in `Layout::fields` (`FieldBox`). Each widget wraps its
`plain` and `rich` parameters in `field()`. Sub-project 2 consumes these
boxes; this sub-project only emits and tests them.

### Generic rendering

`data.typ` no longer tests the `kind`. For each object:

1. `registry.at(kind)`; absent → the current "unknown element" box.
2. Fills `params` with the schema's `default`s; resolves the `bind`s.
3. Converts each `rich` into content (`_paragraphs`, interpolation of
   `vars`), each `plain` through `interp`.
4. Renders each `slot` recursively into an array of entries. `flow` slot:
   `(kind, id, body)`, the arrangement (stack, columns, grid) is up to the
   widget. `free` slot: `(kind, id, frame, cell, show)`, where `show(frame)`
   renders the child at the size of a resolved frame; the widget places them with
   `free-area(t, children, grid:)` (`src/zone.typ`), which resolves the
   grid cells in the actual size of the zone.
5. Calls `render(args, ctx)`; `ctx` carries the theme, the vars, the id.
6. Wraps the element in its `<oku-slides-box>` box and its `e-<id>` label
   as today.

The current slides (`title-slide`, `content-slide`, `chapter-slide`,
`toc-slide`, `end-slide`, `blank-slide`) become widgets of
category `slide`. The `content` layout (`default`, `columns`, `split`,
`center`, `quote`) becomes a `choice` parameter. Their elements live
in `blocks: p.slot(mode: "flow")` and `elements: p.slot(mode: "free")`.

The current elements become:

| Widget | Parameters |
|---|---|
| `text` | `body: rich` |
| `card` | `kicker: plain`, `title: plain`, `body: rich`, `highlight: flag` |
| `image` | `asset: asset`, `fit: choice("cover", "contain", "stretch")` (default `cover`) |
| `shape` | `shape: choice("rect", "ellipse", "line")` |
| `chart` | `option: host("chart")` |
| `stack` | `layout: choice("rows", "columns", "grid")`, `count: number`, `gap: choice(tokens)`, `children: slot(flow)` |
| `canvas` | `children: slot(free)` |

`image` and `chart` keep the host's image loader (`image-loader`,
`world.rs`): their `render` receives it through `ctx`.

The current `group` splits into `stack` (formerly the `rows`, `columns`,
`grid` layouts) and `canvas` (formerly the `free` layout): a slot's mode is set by the
schema, not by the value of another parameter. In a `free` slot, a
child without `frame` or `cell` is a validation issue; in a
`flow` slot, a `frame` or a `cell` is too.

### Registry probe

The host compiles a minimal document per pack:

```typst
#import "widgets.typ": registry
#metadata(registry.map(w => (name: w.name, schema: w.schema)))<oku-widgets>
```

## Registry and Rust model

### Registry (`oku-slides-core`, with no dependency on Typst)

```rust
pub struct WidgetRegistry { widgets: BTreeMap<String, WidgetSchema> }
pub struct WidgetSchema  { name: String, label: Option<String>, category: Category,
                           params: Vec<ParamSchema> }   // declaration order
pub enum   Category      { Slide, Element }
pub struct ParamSchema   { name: String, label: Option<String>, ty: ParamType,
                           default: Option<Value>, bind: Option<String> }
pub enum   ParamType     { Plain, Rich, Flag, Number { min: Option<f64>, max: Option<f64> },
                           Choice { values: Vec<String> }, Asset,
                           Slot { mode: SlotMode, accepts: Option<Vec<String>> }, Grid,
                           Host { host: HostType } }
pub enum   SlotMode      { Flow, Free }
pub enum   HostType      { Chart }
```

### Loading (`oku-slides-render`)

`load_registry(pack) -> Result<WidgetRegistry, RegistryError>` compiles the
probe with the render's `World`, reads `<oku-widgets>`, merges with the base
(override rule), deserializes. The cache (`Registries`, in render)
stores registries by fingerprint (sha-256 of the library and of
`widgets.typ`); the fingerprint is not in `WidgetRegistry`.

**Override.** A pack may redefine a base widget if each base parameter
exists with a compatible type and `bind`; additions are
allowed. Compatible means: any value valid for the base parameter
stays valid for the pack's — a pack `number` covers
the range of the base `number` (equal or wider bounds), a `slot`
keeps the same mode and its `accepts` covers the base's (superset, or
`none` for "accept everything"), a pack `choice` covers the base's `values`
(superset), `bind` is identical, the other types are identical.
Otherwise the pack does not load (`IncompatibleOverride`); a different
`category` does not load either (`IncompatibleCategory`). A deck
thus stays valid whatever the pack.

### Model

```rust
pub struct Slide   { id: String, kind: String, params: Params,
                     name: Option<String>, description: Option<String>, notes: String }
pub struct Element { id: String, kind: String, frame: Option<Frame>, cell: Option<Cell>,
                     style: Style, params: Params }
pub struct Params(BTreeMap<String, ParamValue>);
pub enum   ParamValue { Text(String), Rich(Vec<Paragraph>), Slot(Vec<Element>), Value(Value) }
pub struct DeckParams { title, subtitle, author, date, organization, lang, extra }  // formerly the deck's `Params`
```

JSON:

```json
{ "id": "c1", "kind": "card",
  "params": { "kicker": "Pilier 1", "title": "Fiabilité",
              "body": [ { "list": "bullet", "runs": [ { "t": "Diviser par deux les incidents" } ] } ] } }
```

- **Deserialization without a registry, by the shape of the JSON.** Ops arrive
  deserialized by serde at the Tauri (`Request::Apply`) and MCP
  (`apply`) boundaries, where no registry can be injected. `ParamValue` is
  therefore deduced from the shape: string → `Text`; non-empty array of objects all
  carrying `id` and `kind` as strings → `Slot`; array where each object has
  only the keys of a paragraph (`runs`, `list`) and each run only those
  of a `Run` (`t`, `bold`, `italic`, `color`, `size`, `link`,
  `highlight`), empty included → `Rich`; the rest, including an array
  mixing objects with partial `id`/`kind` or other keys → `Value`
  (never an error: nothing is lost). The schema applies afterwards: `validate`,
  the ops (`&WidgetRegistry`) and `reconcile` normalize according to the declared
  type (a `[]` declared `slot` becomes an empty `Slot`; a `host("chart")`
  is checked by `oku_slides_chart::normalize`).
- **Nothing is lost.** A parameter absent from the schema, or the whole `params`
  of an unknown `kind`, is kept as `ParamValue::Value` and comes back out
  identical. `validate` reports it; the render ignores it or shows
  the error box.
- Going away: `ElementBody`, `SlideKind`, `SlideLayout`, `GroupLayout`,
  `Zone`, `ShapeKind` (becomes a `choice`), the `title`,
  `subtitle`, `kicker`, `layout`, `text`, `blocks`, `elements`, `grid` fields of
  `Slide`.
- Staying: `Style`, `Frame`, `Cell`, `Grid`, `Paragraph`, `Run` —
  placement and presentation common to all widgets. `Deck.params` keeps
  its fields.
- `DECK_VERSION = 2`.

## Loro replica and ops

### Replica

`replica/schema.rs` stops being a static table:

- `Ctx::Slide` and `Ctx::Element` now only cover the common attributes
  (`id`, `kind`, `frame`, `cell`, `style`, `name`, `description`, `notes`).
  Their `params` key is a `LoroMap` of context `Ctx::Params`.
- **Writing**: a parameter's type comes from its type declared by the
  registry (`declared_kind`) when there is one (`Plain` → `Prose`, `Rich` →
  `LoroMap { rich: LoroText with marks }`, `Slot` → `Collection(Element)`,
  everything else, including `Choice` and `Asset`, → `Atom`: although they
  are written like text on the `ParamValue::Text` side, they are written and
  merged as a whole, never per character — merging two `choice` values
  per character would yield an invalid one). Without a declared type
  (parameter unknown to the widget), the type is read from the shape of the JSON
  value (`shape_kind`), same classification as `ParamValue`: string → `Prose`,
  array of `{id, kind}` objects → `Collection(Element)`, array of
  paragraphs → `Rich`, the rest → `Atom`.
  `reconcile(deck, &registry)`
  creates in advance (t-188 workaround) all the `plain`, `rich` and
  `slot` parameters declared by the widget's schema.
- **Reading without a registry**: `LoroText` → text, `LoroMap` with the single key
  `rich` → paragraphs, `LoroMovableList` → slot, value → `Value`. An
  empty text reads back as absent. History, merging and the preview of a
  past version therefore do not depend on the installed pack. `MAX_SLOT_DEPTH`
  (16) also bounds reading: a slot nested deeper by a hostile
  peer is cut and reported (`issue-slot-too-deep`) rather than
  crashing (`MAX_DEPTH` of the container walk, 64, set by the stack).
  The canonical form of a deck (`canonicalize`, in `canonical.rs`, called
  by `SourceBundle::new` and by `apply_all` before writing into the replica)
  sorts the keys of JSON objects and removes empty slots (empty = absent).
  Reading the replica (`replica/read.rs`) does not call `canonicalize` again:
  it sorts its own keys independently, identically.
- `SOURCES_VERSION` moves to 3: older sources are not read.
- `SetKind` concurrent with a parameter edit: the parameter survives,
  unknown to the new widget, kept and reported.
- `SourceBundle::merge` and `SourceBundle::restore` (`sources.rs`) read back the
  rebuilt deck then pass the view to `validate`, with the registry and the
  bundle's asset ids: the `Vec<Issue>` they return is thus the problems of the
  read path (unreadable subtrees) followed by those reported by
  `validate`, without duplicates (`merge` also returns its
  replica's `MergeReport`, separately). `DeckReplica` itself (`replica/mod.rs`) does not
  validate: its `merge` returns a `MergeReport` without `Issue`, and its `restore`
  returns nothing.

### Ops

| Today | Tomorrow |
|---|---|
| `SetText`, `SetSlideField`, `SetParam` (cover) | `SetParam { target, param, value }`, validated against the schema; a `bind` parameter writes into `deck.params` |
| `SetParam { key, value }` (deck) | `SetDeckParam { key, value }` |
| `AddElement { slide, zone, parent, index, element }` | `AddElement { slot: SlotRef { owner: Target, param }, index, element }` |
| `SetGrid` | `SetParam` on the `grid` parameter of the slide widget |
| `SetChartOption`, `SetChartData` | kept, addressed by `(target, param)` |
| — | `SetKind { target, kind }`: changes widget, keeps same-named parameters |
| `ReplaceElement`, `RemoveElement`, `SetStyle`, `ReplaceStyle`, `SetFrame`, `SetCell`, `AddSlide`, `RemoveSlide`, `MoveSlide`, `SetNotes`, `SetTheme`, `SetTemplates`, `SetToken`, `SetVar` | unchanged (`slide`/`element` addressing kept) |

`Target`: `Slide(id)` or `Element { slide, element }`. `locate` becomes a
generic walk over the slots.

`Op::apply(self, &mut Deck, &WidgetRegistry) -> Result<(), OpError>`: same
shape as before plus the registry (the return was already `()`, no value).
`apply_all(&mut Deck, Vec<Op>, &WidgetRegistry) -> Result<(), TransactionError>`.

### Validation

`validate(deck, registry)` returns `Issue`s: unknown `kind`, unknown
parameter, incompatible type, `choice` out of list, `slot` that rejects a
`kind`, placement inconsistent with the slot's mode (see "Generic
rendering"), widget of category `element` in place of a slide (and
vice versa), `bind` to an unknown `deck.params` key. An inconsistent
deck opens; the app shows the problems. The ops, for their part, reject
a badly typed value.

## Consumers

### Session

- Holds the active pack's `Arc<WidgetRegistry>`; reloads it on
  `SetTemplates`. Pack in error → diagnostic shown, fallback to the base
  pack.
- `views.rs`: `Target` carries `widget` and the list of the schema's parameters
  (name, type, label). The ids `@title`, `@subtitle`, `@kicker`, `@text`
  go away: a slide's fields are `FieldBox`es whose `owner` is
  the slide's id, like those of an element.
- New `widgets()` view: the registry (name, label, category,
  parameters).

### MCP and agent

- `widgets` tool: the registry as JSON Schema, one schema per widget with
  its typed `params`. The agent's prompt refers to it before any element
  creation.
- The ops' argument schemas describe `params` as an object; the
  check happens at apply time, and the localized error names the
  parameter and the expected type.

### UI

- `api.ts`: generic `Element`, `Slide`, `WidgetSchema` type.
- `deck.ts`: `textOps` produces `set_param`.
- `slideTools.ts`, `a2ui/panelActions.ts`: widget lists read from the
  registry.
- The current in-place editing keeps working: on a card it
  edits `body`, on a text `body`, on a slide the existing `plain`
  fields. Per-field positioning arrives with t-271.

### Repository

- `fixtures/*.json`, `templates/*` (in folder format), CLI, MCP,
  session, agent tests and the `typst/oku-slides` README rewritten in v2.
- Regressions whose subject goes away (e.g. `issue_0164`): adapted to the
  new model if the risk remains (weight of an element), otherwise
  removed.
- i18n: Fluent keys of the new errors in the 20 languages.

## Errors

- `RegistryError` (render): `BadPackId { id }`, `RegistryCompile { diagnostics }`,
  `Missing`, `Schema { source: WidgetError }` (declaration defect or
  incompatible override, detailed by `WidgetErrorKind`), `Setup`.
- `OpError` extended: `UnknownWidget`, `UnknownParam`,
  `ParamType { param, expected }`, `SlotRejects { slot, kind }`,
  `ChoiceOutOfRange`, `WrongCategory`.
- Snafu with backtrace, lowercase messages, localized via Fluent.

## Outside plan 2

- Localized labels of widgets and parameters (Fluent keys):
  no screen consumes them before t-271.

## Plans

Two plans, a single branch:

1. **Plan 1, additive**: `widget`, `field`, `p`; all widgets declared;
   the old `data.typ` dispatcher calls their renders; `WidgetRegistry`
   (core), `load_registry` and `Registries` (render), pack injected into the
   Typst world. The workspace stays green at every task.
2. **Plan 2, v2 switch**: model, Loro, ops, validation, generic rendering,
   then session, MCP, agent, UI, packs in folder format, fixtures, i18n.
   Only the crate in progress is green at every task; the workspace is at the
   end. The golden test stays in place: its decks are converted to v2 and the
   render stays identical (bounded antialiasing difference tolerated, as in
   plan 1).

## Tests

In build order, in TDD:

1. **Typst**: the base pack probe produces the expected JSON (`insta`
   snapshot); `field()` emits `(owner: "c1", field: "title")` with box and style; the generic
   render of `fixtures/roadmap` is identical to the current render
   (comparison of the rendered pages).
2. **Registry**: `p.*` descriptors; cache by fingerprint; override
   (`rstest`: addition accepted, renaming rejected, type change rejected).
3. **Model**: guided deserialization; unknown parameters and `kind`
   kept identical in a round trip.
4. **Ops**: `SetParam` for each type; `bind` to `deck.params`;
   `SetKind` keeps same-named parameters; `AddElement` rejected by a slot.
5. **Loro**: round trip `Deck → Loro → Deck`; merges title ‖ body of
   the same card, `SetKind` ‖ parameter edit; `convergence.rs` ported
   to v2.
6. **Session, MCP, UI**: `widgets` tool; `set_param` from in-place
   editing; registry reload on `SetTemplates`.
