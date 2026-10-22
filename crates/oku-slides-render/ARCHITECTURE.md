# oku-slides-render

Deck rendering with embedded Typst: the deck is exposed as JSON to the `typst/oku-slides`
library (no deck text is Typst code); one compiled document gives SVG, PNG and PDF.

## Key types

- `Renderer` (`lib.rs`): keeps a live Typst world between renders (incremental compilation).
  `render`/`set_deck` compile; `set_widgets` loads a `PackSet` (an assembly per pack, `ns/name`;
  the core has none: `new` draws "unknown slide"); `with_builtins`: `base` + `essentiel`.
  `set_language`: fallback frames' language. `set_theme`: tokens resolved by the session
  (`/theme.json`, `render-deck(…, theme:)`), never `deck.theme`; defaults without one. Charts
  are redrawn when the compiled palette changes.
- `Compiled` (`lib.rs`): `svg`/`png`/`pdf`, `page_hash` (cache key), `layout()` (read once).
- `Layout` (`lib.rs`): by page, `SlidePage`, `ElementBox` (real box), `FieldBox` (editable
  field), `ItemBox` (`list-item`), `ZoneBox`/`GridBox` (free zone, grid), `TextStyle` (measured).
  `field()` emits a box per field, even empty; `fields-stack` sticks an empty field's marker to
  the next item, taking no room (`FieldBox.empty`, `side`).
- `PackWidgets`: namespace, `PackKind`, `<dir>/<name>.typ` files (`widgets` or `layouts`,
  `_commun` shared) under `/packs/<id>/` and a generated `/packs/<id>/<dir>.typ` naming them
  `ns/name`; `PackSet`: packs by namespace, unique namespaces and ids (`DuplicatePack`).
  `probe_file` (`widget_probe.rs`): one file alone, by kind, gives a `FileProbe` (schema or
  diagnostics); `assemble_set`: a fixed probe's packs + valid files (category contrary to the
  kind dropped). `Registries` memoizes by fingerprint and `probe_key` (empty set: empty
  registry); `base_probe` (`base`), `builtin_registry` (`base` + `essentiel`) are memoized.
- `builtin_packs.rs`: `BuiltinPack` (`Base`, `Essentiel`) embeds `typst/oku-slides/packs/*`;
  `is_trusted_pack`: a built-in namespace and a version of that pack's `versions.txt`.
- Bounded errors (`error.rs`): `RenderErrorKind` has `Timeout`, `OutOfMemory`, `WorkerLost`,
  `Frame`, `Stale`, `PackDisabled`, `Protocol` besides Typst failures; `is_bounded` says which
  ones (not `Frame`, `Stale`, `Protocol`) make the session disable the faulty pack.
- `wire` (`wire.rs`): `WireError`, serializable `RenderError`/`RegistryError` (no backtrace) for
  the worker pipe; `Layout` too; `Fonts::files` exposes font bytes for fingerprints and replay.
- `OkuSlidesWorld` (`world.rs`, private): virtual file system (embedded library with its JSON
  data; deck, theme, assets, packs) and fonts (`Fonts`), no system font; `load-image(…, flow:)`
  sizes a flow image at its proportions when its widget asks.
- `charts` (private): an element with `host("chart")` becomes an SVG drawn by
  `oku-slides-chart`, served as `/charts/<fingerprint>.svg` (options, size, font, palette,
  labels); drawn at its box size (measured at compilation), never scaled.
- `tables` (private): guard pass before Typst (canonical form, or `null` if invalid); Typst
  draws the table, no Rust pass. `diagnostic`: localized Typst messages; `palette`: colors.

## Extension points

- A new page output format: a method on `Compiled`. New document metadata: a Typst label
  (`oku-slides-*`), a `Layout` field and an entry in `Layout::layout()`.

## Dependencies

`oku-slides-core`, `oku-slides-i18n`, `oku-slides-chart`, `typst` and its family (`typst-pdf`,
`typst-svg`, `typst-render`, `typst-layout`, `typst-assets`); dev: `oku-slides-pdf`, `speculoos`, `insta`.
