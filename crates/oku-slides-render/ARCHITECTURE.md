# oku-slides-render

Deck rendering with embedded Typst: the deck is exposed as JSON to the `typst/oku-slides`
library (no deck text is Typst code); one compiled document gives SVG, PNG and PDF.

## Key types

- `Renderer` (`lib.rs`): keeps a live Typst world between renders (incremental compilation).
  `render`/`set_deck` compile the deck; `set_widgets` loads the core (slide) widgets plus a
  `PackSet` (one assembly per pack, widgets as `ns/name`, never overriding a core widget);
  `with_builtins` adds the built-in `base` pack; `set_language`: language of fallback frames.
  `set_theme`: theme tokens resolved by the session (`/theme.json`, `render-deck(…, theme:)`),
  never `deck.theme`; default tokens without one. Charts are redrawn when the compiled palette changes.
- `Compiled` (`lib.rs`): `svg`/`png`/`pdf`, `page_hash` (cache key), `layout()` (read once).
- `Layout` (`lib.rs`): what rendering says about the document, by page: `SlidePage`,
  `ElementBox` (real box), `FieldBox` (editable field), `ItemBox` (list item, `list-item`),
  `ZoneBox`/`GridBox` (free zone and its grid), `TextStyle` (measured typography). `field()`
  emits one box per field, even empty; `fields-stack` sticks an empty field's marker to the
  next item, taking no room (`FieldBox.empty`, `side`).
- `PackWidgets`: a namespace and `widgets/<name>.typ` files (`_commun` shared) under `/packs/<id>/`,
  plus a generated `/packs/<id>/widgets.typ` naming them `ns/name`; `PackSet`: a deck's packs by
  namespace, unique namespaces and ids (`DuplicatePack`). `probe_file` (`widget_probe.rs`): one file alone, no other pack reachable, gives a
  `FileProbe` (schema or diagnostics; declared name = file name); `assemble`: base + one pack's
  valid files. `Registries` memoizes by `PackSet::fingerprint` and `probe_key`; `core_registry`
  (slides), `builtin_probe` / `builtin_registry` (core + `base`, memoized; `assemble` keeps a probe's packs).
- `base_pack.rs`: render owns the `base` pack sources (`typst/oku-slides/packs/base`, embedded),
  the session lists them; `is_trusted_pack`: namespace `base` and a version of `versions.txt`.
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
