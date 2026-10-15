# oku-slides-render

Deck rendering with embedded Typst: the deck is exposed as JSON to the `typst/oku-slides`
library (no deck text is Typst code); one compiled document gives SVG, PNG and PDF.

## Key types

- `Renderer` (`lib.rs`): keeps a live Typst world between renders (incremental compilation).
  `render`/`set_deck` compile the deck; `set_widgets` loads the base widgets overridden by a
  `PackWidgets`; `set_language` sets the language of fallback frames. `set_theme` sets the
  theme tokens resolved by the session (served as `/theme.json`, passed to
  `render-deck(…, theme:)`): rendering reads neither `deck.theme` nor a theme library; without
  a theme, the default tokens. Charts are redrawn before Typst when the compiled theme's
  palette differs from the one they were drawn with (a second pass).
- `Compiled` (`lib.rs`): a compiled document: `svg`/`png`/`pdf`, `page_hash` (cache key),
  `layout()` (read once from the document, `OnceCell`).
- `Layout` (`lib.rs`): what rendering says about the document, by page: `SlidePage`,
  `ElementBox` (real box), `FieldBox` (editable field), `ItemBox` (list item, `list-item`),
  `ZoneBox`/`GridBox` (free zone and its grid), `TextStyle` (measured typography). `field()`
  emits one box per field, even empty; `fields-stack` sticks an empty field's marker to the
  next item, taking no room (`FieldBox.empty`, `side`).
- `PackWidgets`: `widgets/<name>.typ` files (shared `_commun`) served under `/packs/<id>/`, with
  a generated assembly module `/packs/<id>/widgets.typ`. `probe_file` (`widget_probe.rs`): one
  file alone gives a `FileProbe` (schema, or diagnostics with line and column; declared name =
  file name); `assemble`: the registry of the valid ones only. `Registries` (`registry.rs`)
  memoizes by pack and by `probe_key`; `base_registry` gives the base without a pack.
- Bounded errors (`error.rs`): `RenderErrorKind` has `Timeout`, `OutOfMemory`, `WorkerLost`,
  `Frame`, `Stale`, `PackDisabled`, `Protocol` besides Typst failures; `is_bounded` says which
  ones (not `Frame`, `Stale`, `Protocol`) make the session fall back to the base widgets.
- `wire` (`wire.rs`): `WireError`, the serializable form (no backtrace) of
  `RenderError`/`RegistryError` for the worker pipe; `Layout` is serializable too;
  `Fonts::files` exposes font bytes for fingerprints and replay.
- `OkuSlidesWorld` (`world.rs`, private): virtual file system (embedded library with its JSON
  data `src/default-tokens.json` and `themes/*.json`; deck, theme, assets, pack) and fonts
  (`Fonts`, `fonts.rs`), no system font.
- `charts` (private): an element with `host("chart")` becomes an SVG drawn by
  `oku-slides-chart`, served as `/charts/<fingerprint>.svg` (options, size, font, palette,
  labels); drawn at its box size (measured at compilation), never scaled.
- `tables` (private): guard pass before Typst (canonical form, or `null` if invalid); Typst
  draws the table, no Rust pass. `diagnostic`: localized Typst messages; `palette`: colors.

## Extension points

- A new page output format: a method on `Compiled`. New document metadata: a Typst label
  (`oku-slides-*`) in the library, a `Layout` field and an entry in `Layout::layout()`.

## Dependencies

`oku-slides-core`, `oku-slides-i18n`, `oku-slides-chart`, `typst` and its family (`typst-pdf`,
`typst-svg`, `typst-render`, `typst-layout`, `typst-assets`); dev: `oku-slides-pdf`, `speculoos`, `insta`.
