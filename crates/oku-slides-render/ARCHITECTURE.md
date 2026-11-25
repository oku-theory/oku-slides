# oku-slides-render

Deck rendering with embedded Typst: the deck is exposed as JSON to the `typst/oku-slides`
library (no deck text is Typst code); one compiled document gives SVG, PNG and PDF.

## Key types

- `Renderer` (`lib.rs`): keeps a live Typst world between renders (incremental compilation).
  `render`/`set_deck` compile; `set_widgets` loads a `PackSet` (an assembly per pack, `ns/name`;
  the core has none: `new` draws "unknown slide"); `with_builtins`: `base` + `essentiel`.
  `set_language`: fallback frames' language. `set_theme`: the caller's layer (`/theme.json`:
  packs' `component.<ns>` defaults, then the theme chain), never `deck.theme`; packs fail
  without it. Charts are redrawn when the compiled palette changes.
- `Compiled` (`lib.rs`): `svg`/`png`/`pdf`, `page_hash` (cache key), `layout()` (read once).
- `Layout` (`lib.rs`): by page, `SlidePage`, `ElementBox` (real box), `FieldBox` (editable
  field), `ItemBox` (`list-item`), `ZoneBox`/`GridBox` (free zone, grid), `TextStyle` (measured).
  `field()` emits a box per field, even empty; `fields-stack` sticks an empty field's marker to
  the next item, taking no room (`FieldBox.empty`, `side`).
- `PackWidgets` (`from_files`: id = version hash, the only trusted build): namespace, `PackKind`, `<dir>/<name>.typ` files (`widgets` or `layouts`,
  `_commun` shared) under `/packs/<id>/` and a generated `/packs/<id>/<dir>.typ` naming them
  `ns/name`; `PackSet`: packs by namespace, unique namespaces and ids (`DuplicatePack`).
  `probe_file` (`widget_probe.rs`): one file alone, by kind, gives a `FileProbe` (schema, read as a
  `WidgetSchema`, or diagnostics); `assemble_set`: a fixed probe's packs + valid files
  (category contrary to the kind dropped). `Registries` memoizes by fingerprint and `probe_key` (empty set: empty
  registry); `base_probe` (`base`), `builtin_registry` (`base` + `essentiel`) are memoized.
- `builtin_packs.rs`: `BuiltinPack` (`Base`, `Essentiel`) embeds `typst/oku-slides/packs/*`;
  `is_trusted_pack`: built-in namespace, shipped version (`versions.txt`); `is_never_blamed`: trusted
  or any `base`; `builtin_component_defaults`: each manifest's token defaults.
- Bounded errors (`error.rs`): `RenderErrorKind` has `Timeout`, `OutOfMemory`, `WorkerLost`,
  `Frame`, `Stale`, `PackDisabled`, `Protocol`; `is_bounded` (not `Frame`, `Stale`, `Protocol`): the session disables the pack.
- `wire` (`wire.rs`): `WireError`, serializable `RenderError`/`RegistryError` (no backtrace) for
  the worker pipe; `Layout` too; `Fonts::files` exposes font bytes for fingerprints and replay.
- `OkuSlidesWorld` (`world.rs`, private): virtual file system (embedded library with its JSON
  data; deck, theme, assets, packs), fonts (`Fonts`), no system font; `load-image(…, flow:)`.
- `views` (private): `resolve_views`, run by `set_deck`, clones the deck, `data` emptied, chart and
  table params given the dataset `plan_views` cut (plain values). `charts` (private): a `host("chart")`
  becomes an SVG drawn by `oku-slides-chart`, `/charts/<fingerprint>.svg` (option, resolved dataset,
  size, font, palette, labels), at its box size (measured at compilation), never scaled.
- `tables` (private): guard pass before Typst (canonical form, or `null` if invalid); Typst
  draws the table, no Rust pass. `diagnostic`: localized Typst messages; `palette`: colors.

## Extension points

- A new page output format: a method on `Compiled`. New document metadata: a Typst label
  (`oku-slides-*`), a `Layout` field and an entry in `Layout::layout()`.

## Dependencies

`oku-slides-core`, `oku-slides-i18n`, `oku-slides-chart`, `typst` and its family (`typst-pdf`,
`typst-svg`, `typst-render`, `typst-layout`, `typst-assets`); dev: `oku-slides-data`, `oku-slides-pdf`, `speculoos`, `insta`.
