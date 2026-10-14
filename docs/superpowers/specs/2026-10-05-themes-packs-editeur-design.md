# Themes as versioned packs and token editor — design

Date: 2026-10-05 · Task: t-554

## Goal

The `papier` and `pro` themes are no longer hard-coded: they are built-in
themes of a **versioned theme library**, twin of the template pack
library (t-328). A deck references a theme at a version and
embeds it. The templates perspective gains a **Theme** tab that edits all
the design tokens: colors, fonts, sizes, weights, leading,
spacing, radii, strokes, page, chart palette, components,
UI colors, CJK fallback.

Two phases in a single plan: 1. themes as packs (no visible change
outside the theme menu); 2. editor.

Out of scope: import/export of a theme alone, fonts embedded in a
theme (they stay in the deck's `fonts/`), cleanup of versions, the
"newer version of the theme" banner (the relation is computed, the display
will come with a dedicated task if needed).

## Decisions

- **A theme is a separate file.** Template packs stay
  independent of the theme ("Un ensemble ne dépend pas du thème", a set does not depend on the theme).
- **Same mechanics as packs.** UUID v4 id, content-addressed versions
  (`sha256:<hex>`), one current version per theme, version parent
  (genealogy only, no effect on rendering), built-ins read-only
  and with a constant id. The cache is authoritative at render: a deck renders with the
  version it embeds.
- **Generic `Library`** over a kind (`LibraryKind`): templates and themes
  share storage, versions, `current`, `relation`, built-ins (G11).
- **A theme version is a complete snapshot.** No inheritance between
  themes: the merge stays `default-tokens ← theme ← deck.tokens`. A copy
  copies all tokens; `parent` only keeps the genealogy.
- **The JSON is the only source of truth for tokens.** `default-tokens` and
  the built-in themes become JSON files of the Typst package, read by
  Typst (`json(...)`) and embedded by Rust (`include_str!`).
- **Saving a built-in theme creates a copy** (new id, name "<nom>
  (copie)" ("<name> (copy)"), parent = the built-in version). A user theme gains
  a version, which becomes current. The deck set aside adopts the saved
  version.
- **Token panel on the right** of the stage, fixed "Thème" (Theme) tab after
  "Modèles" (Templates).
- **No backward compatibility**: `DECK_VERSION` 3 → 4, `SOURCES_VERSION`
  5 → 6; old decks and zips are no longer read.

## Model (core)

```rust
pub struct ThemeRef { pub id: Uuid, pub version: PackVersion }
pub struct Deck { …, pub theme: Option<ThemeRef>, … }   // was Option<String>
```

- `PackVersion` and `PackFiles` serve both kinds. A theme version
  is a `PackFiles` whose manifest is `theme.json` and which has no
  widget file: same canonical content, same fingerprint.
- `theme: None`: the default theme (`papier`, constant id), at its current
  version, embedded and referenced on load (`adopt_default_theme`,
  like `adopt_default_pack`).
- In the Loro replica, `theme` is an atom (last writer wins), like
  `templates`.
- **Ops.** `SetTheme { theme: Option<ThemeRef> }`. `SetToken { path, value }`
  sets `value`, `null` included ("clearing" a key that accepts it); new
  `UnsetToken { path }` removes the key (back to the lower layer). The
  MCP schema and the agent follow.

### `theme.json`

```json
{ "id": "<uuid>", "name": "Papier", "description": "", "parent": null,
  "created": "2026-10-05T00:00:00Z", "tokens": { "color": { … }, … } }
```

`ThemeManifest { id, name, description, parent, created, tokens: Map }`
(session), next to `PackManifest`.

## File (sources zip, version 6)

```
themes/<hex>/theme.json
```

- `SourceBundle.themes: BTreeMap<PackVersion, PackFiles>`, append only,
  unioned by `merge`, kept if referenced by the deck or its history.
- On read, a version whose fingerprint does not match is the only one
  dropped (`Issue::ThemeVersionMismatch`); a missing deck version gives
  `Issue::ThemeVersionMissing`, and the deck renders with the default theme.
  An invalid `themes/` path (folder other than 64 lowercase hex,
  subfolder, file other than `theme.json`) rejects the whole zip.

## Library (session)

```text
<data>/themes/<uuid>/current
<data>/themes/<uuid>/versions/<hex>/theme.json
```

- `Library<K: LibraryKind>`: `K::DIR` (`templates` / `themes`),
  `K::MANIFEST_FILE` (`pack.json` / `theme.json`), `K::Manifest` (trait
  `VersionManifest`: `id`, `name`, `parent`, `created`, `from_files`),
  `K::builtins()`. Aliases: `PackLibrary`, `ThemeLibrary`.
- Built-ins: `PAPIER_THEME` (default) and `PRO_THEME`, constant UUIDs.
- `ThemeInfo { id, version, name, builtin, default }` for the list.

## Rendering

- `typst/oku-slides/src/default-tokens.json`; `tokens.typ`:
  `#let default-tokens = json("default-tokens.json")`.
- `typst/oku-slides/themes/papier.json`, `pro.json` (the complete `theme.json`);
  `lib.typ`: `#let papier = json("themes/papier.json").tokens` (the Typst
  package's `theme: papier` usage unchanged). `papier.typ`, `pro.typ`,
  `THEMES` go away.
- `Renderer::set_theme(Option<serde_json::Map>)` (and the worker's
  `Request::SetTheme`): the world serves `/theme.json`, the main source passes
  `theme: json("/theme.json")` or `none`.
- Chart font: read from the resolved tokens (`font.body`, first
  family), in a second pass like the palette; `charts.rs::body_font`
  goes away.

## Session

- `Session::deck_theme()`: the embedded version; `embed_theme_from_library`
  like `embed_from_library`. On every version change, the renderer
  receives `set_theme`.
- Snapshot: `theme: Option<ThemeRef>`, `themes: Vec<ThemeInfo>`.
- `set_theme { id }` request: embeds the current version of `id` and applies
  `SetTheme` (undoable).

## Editor (phase 2)

### Template mode

- On entry, the working deck has `theme: None` and `tokens` = **all the
  tokens of the edited theme** (the theme of the deck set aside). The deck's own
  overrides no longer apply: the templates are seen with the pure
  theme, as in any deck that uses it.
- `Mode::Templates` keeps `theme_edit: ThemeEdit { base: ThemeRef, name }`.
- Each setting is a `SetToken` / `UnsetToken` on the working deck:
  render, thumbnails, app appearance and undo/redo follow automatically.
- Choosing another theme in the tab: `ThemeEdit.base` changes and the tokens
  of the working deck are replaced in one undoable step (`SetTokens {
  tokens }`, new op that replaces the whole `deck.tokens`).
- On save (`save_templates`), if the tokens or the name differ from
  the base version: new theme version (a copy for a built-in),
  embedded, current, and the deck set aside switches to that `ThemeRef`. Otherwise, if
  `base` differs from the deck's theme, the deck adopts `base`.
- Closing without saving discards the draft, as for templates.

### Token schema

`crates/oku-slides-session/src/theme_schema.json`: for each path, a
control and its constraints.

| Control | Paths | Value |
|---|---|---|
| `color` | `color.*`, `app*.*` colors, color components | `#rrggbb[aa]` or name of a `color` token |
| `pt` | `size`, `space`, `radius`, `stroke`, `page` (except `width`/`height`), dimension components | bounded number, or name of a token of the referenced category |
| `em` | `leading.*` | number 0 to 2 |
| `weight` | `weight.*`, weight components | 100 to 900 in steps of 100, or name of a `weight` token |
| `fonts` | `font.*` | ordered list of families |
| `choice` | `script-style.*`, `app.scheme`, `layout`, `case`… | one of the listed values |
| `bool` | `bar`, `rule`… | boolean |
| `text` | templates (`footer.left`…) | string |
| `palette` | `chart.palette` | list of colors or of `color` token names |
| `number` | `opacity`, `ratio`, `tracking`, `elevation` | bounded number |

Each entry has `nullable` ("clearing" allowed) and a display category.
`page.width` and `page.height` are not editable (1920 × 1080 grid).
A theme's own colors (`brand-ink`…) are editable like `color.*`
and can be added or removed (name `[a-z][a-z0-9-]*`).

**Validation** on the session side at each `SetToken`/`UnsetToken` in template
mode: value conforming to the control, reference to an existing token of the
right category, removal refused if other tokens refer to it (the message
names them). Outside template mode, color validation also applies to the
deck's overrides (rule t-194).

**Coverage**: a test checks that the schema and `default-tokens.json` have
exactly the same paths (except the free colors of `color.*`).

### `host-theme` surface

Built in Rust (`theme_panel.rs`, next to `panel.rs`), rendered by
`HostSettings` with the existing A2UI controls (`Color`, `Stepper`,
`Slider`, `Select`, `Text`, `List`), extended as needed. One section per
category (Colors, Typography, Spacing, Shapes, Page, Charts,
Components — one subgroup per component —, Light UI, Dark
UI, CJK scripts). Each row shows its state:

| State | Meaning | Action |
|---|---|---|
| Default | absent from the draft, comes from `default-tokens` | modify → the key enters |
| Set | in the draft, equal to the base version | "Retirer du thème" (Remove from theme; `UnsetToken`) |
| Modified | different from the base version | ↺ → value of the base version |

### UI

- `TemplateTabs`: fixed "Thème" tab after "Modèles".
- Active tab: template pile on the left, stage of the chosen template in the center,
  `ThemePanel` panel on the right (header: theme selector, name, filter
  field; then the `host-theme` surface in collapsible sections).
- The theme selector of the ⋯ menu disappears from template mode.
- Labels: Fluent, `ui-theme-*` (UI) and `theme-token-*` (labels of
  the surface, session side) in the 20 languages.

## Errors

Snafu, translated messages: `UnknownTheme`, `ThemeVersionMissing`,
`ThemeIdMismatch`, `ThemeJson`, `InvalidTokenValue { path, reason }`,
`TokenInUse { path, users }`, `NoThemesDir`. An unreadable version of the
library is ignored with a `warn`; the built-ins always stay
available.

## Tests

- Core: serialized `ThemeRef`, canonical content of a theme, round trip of the
  `themes/` zip, invalid paths, `ThemeVersionMismatch`, `SetToken null`,
  `UnsetToken`, `SetTokens`, Loro replica (`theme` atom, `null` tokens).
- Session: generic `Library` (existing pack tests + theme twins),
  adoption of the default theme, `set_theme`, save in template mode
  (copy of a built-in, version of a user theme, nothing if unchanged,
  discard on close), schema (coverage, validation, `TokenInUse`),
  `host-theme` surface (default/set/modified states).
- Render: golden snapshots of `papier` and `pro` **unchanged** after the switch
  to JSON; regression: chart font read from `font.body`.
- UI: render of `ThemePanel` and of the tab; e2e (isolated data): modify
  Papier's `color.accent`, see the stage change, save, "Papier
  (copie)" exists and the deck uses it.
- i18n: `i18n-keys.test.ts` (en/fr), 18 other languages completed.
