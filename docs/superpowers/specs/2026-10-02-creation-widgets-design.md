# Widget creation by the agent and Typst editor — design

Date: 2026-10-02 · Task: t-329

## Goal

The user has the agent create or modify widgets ("make me a
KPI widget"), or writes them themselves in a Typst editor. A widget is a
Typst file of a template pack; it travels with the deck (t-328) and
runs in the bounded render process (t-327).

"Portable widgets" program: 0. bounded compilation (t-327, delivered);
1. portable packs (t-328, delivered); 2. **widget creation** (this
spec).

Out of scope: Typst autocompletion and LSP, renaming a widget already
used in decks (it is renamed in the edited version; the decks
keep theirs), sharing widgets between packs.

## Decisions

- **One file per widget** in a pack version, plus an optional shared
  file `_commun`. The app generates the `registry`.
- **Available in template mode and in deck mode.** Modifying a widget
  modifies the user's pack, wherever it happens: the version
  produced becomes its current version.
- **One version per sitting** (t-328 rule): successive widget
  modifications replace the same version; the sitting ends when leaving
  template mode, or, in deck mode, at the first operation that is not
  a widget modification.
- **Dedicated preview** `preview_widget`: the widget alone on a temporary
  slide, without adding anything to the pack.
- **Isolated errors**: each widget compiles separately; a broken widget
  is left out of the registry with its diagnostics, the others stay.
- **Format break**: `SOURCES_VERSION` moves to 5 (`widgets/<nom>.typ`
  replaces `widgets.typ`); no backward compatibility.

## Pack format (core)

```rust
pub struct PackFiles { pub pack_json: Vec<u8>, pub widgets: BTreeMap<WidgetFile, String> }
pub struct WidgetFile(String);   // "[a-z][a-z0-9-]*" or "_commun"
```

- The file name is the widget's name; `_commun` is not a widget.
- **Canonical content**: canonical `pack.json` (t-328), byte `0`, then
  for each file in name order: name, `0`, source, `0`. An
  empty file counts as absent. `PackVersion` is the sha-256 of this content.
- Sources zip (v5): `packs/<hex>/pack.json`,
  `packs/<hex>/widgets/<nom>.typ`. Library:
  `<uuid>/versions/<hex>/widgets/<nom>.typ`. An invalid file name
  under `packs/<hex>/widgets/` rejects the zip (`BadName`), like an invalid
  `packs/` path.
- Limits (`[render]`): `max_widget_kib = 256` per file,
  `max_widgets = 64` per version; the files count toward
  `max_sources_mib`.

## Assembly and probe (render, render-worker)

- Each `widgets/<nom>.typ` exports the `widget` binding, result of
  `widget("<nom>", …)`. It may import `"_commun.typ"` and the `oku-slides`
  library. The Typst world serves `/packs/<hex>/widgets/<nom>.typ`.
- The host generates an assembly module that imports each widget and forms
  `registry`; `_commun` does not go into it.
- **Per-file probe**, in the render process: each widget is
  compiled alone; the result is, per file, its schema or its
  diagnostics (file, line, column, message). The pack's registry
  contains only the valid widgets. A widget whose declared name differs
  from the file name is rejected with a diagnostic. The probe cache is
  keyed by (name, file hash, hash of `_commun`).
- A timeout or memory overrun during a file's probe gives
  a diagnostic for that file. During editing, the pack is not
  set aside (it is being fixed); the setting aside of t-327 remains that of the deck
  render.

## Session

- `WidgetStatus { name, schema: Option<WidgetSchema>, diagnostics }`,
  `widget_status() -> Vec<WidgetStatus>` for the current pack.
- Operations (template mode and deck mode):
  - `set_widget(name, source)`: creates or replaces a file; returns the
    status of that widget and that of the widgets the modification turned
    from valid to broken (a modified `_commun`).
  - `remove_widget(name)`: refused if templates of the pack use it
    (list of templates in the error).
  - `widget_source(name)`.
- **In template mode**: the operations modify the edited pack; the sitting's
  version follows the t-328 rule (first modification: new
  version, `parent` = current at the start of the sitting, made current;
  subsequent ones: `replace_version`).
- **In deck mode**: the first modification creates a version of the deck's
  pack (`parent` = embedded version), adds it to the library, makes it
  current, adds it to the bundle and switches the deck onto it through a
  `SetTemplates`; the following ones in the same sitting replace that version
  (`replace_version`, new `SetTemplates`), merged into a single undo
  step (key `templates-adopt:<id>:<parent>`). Undo brings the
  deck back to its previous version; the library keeps the created version
  (as for a save in template mode).
- `preview_widget(name, params)`: temporary slide in a dedicated
  slot of the render process, the deck's theme and tokens; missing
  parameters: the schema's `default`, otherwise a localized example (`plain`,
  `rich`) or the type's neutral value. Returns PNG (agent) or SVG (UI) and the
  diagnostics. An exceeded timeout gives a diagnostic.

## Agent (MCP)

Tools, available in both modes:

| Tool | Role |
|---|---|
| `widget_sources` | the pack's files, with their status |
| `widget_source(name)` | source of a file |
| `set_widget(name, source)` | creates or replaces; returns the status and the broken widgets |
| `remove_widget(name)` | deletes (refused if a template uses it) |
| `preview_widget(name, params?)` | PNG image of the widget alone |

The agent's prompt gains a "Create a widget" section: shape of a
file (imports, `widget("<nom>", params: …, (args, ctx) => …)`,
`field(ctx, …)` for the editable fields, `fields-stack`), `p.*`
descriptors, theme tokens (`color-of`, `size-of`) rather than hard-coded
values, `_commun` for shared code, and the loop write → status → preview →
fix. An example widget is provided in the `typst/oku-slides` README.
Inserting the widget into the deck remains an ordinary `add_element`.

## Typst editor (UI)

- **Widgets** panel, accessible in template mode and in deck mode: list
  of files (`_commun` first, labeled "Commun") with a status
  badge (valid, errors); actions New, Rename (in the edited
  version), Delete.
- CodeMirror editor (already present) for the chosen file, minimal Typst
  highlighting (comments, strings, keywords, `#`, `p.*`), diagnostics as
  underlines and in the gutter at their line and column.
- Preview on the right, updated 400 ms after the last keystroke, with its
  diagnostics.
- Typing goes through `set_widget`; the agent and the editor use the
  same operations and see each other.

## Errors

- core: invalid file name → `BadName` when reading the zip;
  `WidgetFile` rejects an invalid name.
- session: `InvalidWidgetName { name }`, `WidgetTooLarge { name, max }`,
  `TooManyWidgets { max }`, `WidgetInUse { name, templates }`,
  `UnknownWidget { name }` (Snafu, backtrace, lowercase).
- Fluent, 20 languages: these errors, the preview examples, the
  panel UI.

## Repository

- t-328 tests and fixtures that use `widgets.typ` rewritten as
  `widgets/<nom>.typ`. The built-in packs have no widgets.
- `typst/oku-slides` README: shape of a widget file, `_commun`,
  example.

## Tests

In build order, in TDD:

1. **Core**: multi-file canonical content (name order, empty
   file = absent), frozen sha-256 vector; v5 zip: `widgets/<nom>.typ` read
   and checked, invalid name rejected; `WidgetFile` accepts `kpi`, `_commun`,
   rejects `Kpi`, `../x`, `_autre`.
2. **Render**: assembled registry equal to that of an equivalent
   hand-written `widgets.typ`; broken widget left out, the others kept;
   `_commun` imported, absent from the registry; declared name ≠ file name
   rejected; probe cache: modifying a widget recompiles only that one,
   modifying `_commun` recompiles all widgets (t-390: the hash of
   `_commun` always goes into the key, the import is no longer detected from the
   text).
3. **Session**: template mode: `set_widget` ×2 then save → one
   version with the file; deck mode: `set_widget` ×2 → one current
   version, the deck switches to it, a single undo step, undo returns the
   previous version; `remove_widget` refused if a template uses it;
   `preview_widget` fills in the defaults; widget that loops in the preview →
   diagnostic, pack not set aside; broken `_commun` → `set_widget` returns the
   broken widgets.
4. **MCP**: the five tools in both modes; `set_widget` response
   with diagnostics and broken widgets.
5. **UI**: list and states; diagnostics at the right line; preview after the
   typing delay (bun).
6. **Regression**: a widget that does not compile does not prevent the other
   widgets of the same pack from rendering in the deck.

## Implementation decisions

- `rename_widget` exists in the session (`widgets_edit.rs`) but is not
  exposed to MCP: the agent writes a new widget then removes the old one.
- In template mode, a widget modification immediately saves the
  sitting's version (`adopt_session_version`); no separate "save".
- A broken widget of the embedded pack is left out and reported by the
  `session-widgets-broken` notice; the valid widgets are still rendered.
- An element widget is previewed on a blank slide, in a
  trial slot that never blames the pack.
- Deck mode (final review): the sitting's version is embedded and the
  deck rendered (`apply`) before any write to the library. A failure,
  including a widget body that fails at render, changes neither the
  library, nor the sitting, nor the deck; `set_widget` then returns the
  render diagnostics in the status (`WidgetChange`), not an error. A
  library write that fails after the render is logged and
  sets the `session-widgets-not-saved` notice (t-390): the deck already embeds
  the version, the library does not have it.
- Template mode (final review): "diagnostics + success" choice. The version
  is saved, then the templates document rendered; if that render fails,
  the version stays and its diagnostics are returned with the statuses, like a
  faulty template. `adopt_session_version` no longer renders: the caller renders.
  `save_templates` follows the same choice (t-390): once the version is adopted, a
  render of the templates document that then fails is not an error;
  the save succeeds and a notice (`set_notice`) gives the cause.
- No-op guard: a write whose widget files are identical to
  those of the current version saves nothing (both modes).
- Bundle growth: the replaced sitting version leaves the sources
  of the document (and, in template mode, those of the deck set aside) **only
  if the write joined the undo step that had introduced it**:
  `SourceBundle::undo_steps` (stacked steps, never decremented) is
  unchanged by the apply. Otherwise (new step, for example after the
  group of an agent turn), undo comes back to it: it stays in the
  sources (append only) and in the library — in deck mode, the new
  version is added there without `replace_version`; in template mode (library
  written first), the old one is added back. History that would show
  a merged step falls back on the "pack manquant" (missing pack) notice. Chosen over
  "do not embed before the end of the sitting", which would leave the deck
  referencing a version absent from its sources.
- The deck-mode widget sitting also ends at `begin_turn` and
  `end_turn`: it never overlaps an agent turn's step. In
  template mode, a turn started in template mode groups the templates
  document, not the deck set aside: its saves merge into it by
  key. Fix (t-390): a turn started in deck mode groups the deck,
  which is set aside if the turn enters template mode; `end_turn` closes that
  group. A save after the turn then makes a new step: the
  non-merged path is reachable (the turn's version stays in the
  sources and is added back to the library; undo comes back to it).
- NUL byte: a widget source that contains one is rejected, on
  read (`PackFiles::from_bytes` → `None`: zip, library, resume
  drop it) and on write (`set_widget` → `WidgetHasNul`, Fluent
  `session-widget-nul`). The canonical `pack.json` (compact JSON) already escapes
  NUL: nothing to reject on that side.
- Pack probe (final review): total `timeout_ms` budget per call of
  `probe_pack`. Once exhausted, the remaining uncached files are not
  executed (localized `probe-*` diagnostic, in the language of the worker's
  slots, like the other limit diagnostics) and are not
  kept: they are probed at the next call (`PackProbe::unprobed` names
  them; a failure preparing the link also files the file there). As long as some remain for the installed pack of the open document,
  each deck render resolves its widgets again, one budget per render,
  (t-390). A file that is merely unprobed is not called broken: the
  `session-widgets-broken` notice omits it, and no other notice reports it (it
  is probed at the next render, the notice would be stale at once). The resume
  that judges it broken sets the notice, reduced to the files unprobed until then.
  A file whose probe exceeds the timeout is
  kept broken: it no longer counts among the unprobed, no endless
  loop. `RenderWorker::probe` (probe
  of the whole pack, without isolation) is removed; its tests go through
  `probe_pack` or `set_widgets`.
