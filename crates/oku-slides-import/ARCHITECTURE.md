# oku-slides-import

PowerPoint (`.pptx`) import: read into a format-neutral, resolved intermediate representation (`source::SourceDeck`,
every styled value with its provenance), projected into a plan of Oku ops. Spec: `specs/2026-10-09-import-pptx-design`.

## API

- `read_pptx(bytes, &ReadLimits) -> Result<SourceDeck, ImportError>`.
- `retrofit::project(&SourceDeck, &RetrofitTarget) -> Result<…>` : content on the `profiles` of the chosen layout
  packs (in order; none: `NoProfiles`) and the tokens of a theme, after the session pins `base` and `essentiel`; kinds from `kinds`.
- `hifi::project(&SourceDeck, &HifiTarget) -> ImportPlan` : on the pinned theme (`HifiTarget.theme`, Neutre) with generated tokens and a deck-only layout pack
  `deck-layouts`; ops: `AddPack` `base`, `essentiel` (fallback `essentiel/blank`), `SetTheme` (the session fills the
  closure), `SetTokens` (the generated tokens), `AddPack` `deck-layouts`; `sought_fonts`, `source_rect`.
- `ImportPlan` (`plan`) : ops, assets, `WidgetRequest`s, report, `origins` (item → element); in hi-fi the
  generated pack, schemas, fonts, `targets` (item → elements or slide param), `groups` (`hifi::groups`);
  `report_json` is the `--report` form; `check_budget` (fatal `TooLarge`) gives the `CheckedPlan` to apply.
- `report` : `ImportReport`, `Loss`/`LossKind`, warnings, `Coverage`, `ReferenceReport`, slide
  `score`; `coverage::coverage` measures what of the source survives in the deck.
- `metrics` : a hi-fi slide's gap to the reference, regions ranked, `suspect` ones last (pure);
  `fonts` : `FontCatalog`, `FontOrigin`, `FontPlan`, `SUBSTITUTES`, `FALLBACKS`.

## Layers

- `source/` depends on nothing (`lang`: scripts); `error`, `limits`, `collect`, `oklab`, `formula` are neutral.
- `pptx/` is the only reader: `package` (bounded zip), `xml` (bounded nodes), `context`, `master`,
  `slide`, `placeholder`, `cascade` (the single text-style resolution), `bounds` (schema ranges,
  clamp warnings), `shapes`, `frame`, `table` (+ built-in `table_styles`), `chart`, `smartart` (model
  as `source::SmartArt`, drawing as a group), `fonts` + `eot`. Nesting bounded by `GroupCtx`. Assets typed
  by their bytes (`media::sniff`), rasters refused on a bad header (`media::dimensions`), SVGs sanitized.
- Projectors `retrofit/` and `hifi/` use `source/`, `plan`, `report`, `coverage`, `fonts`,
  never `pptx/`. They are pure: ids and dates come from the target (ADR-0008).
- `retrofit/` : decor omission (`decor`, shared with `coverage`), profile choice (`profile`), headings, clusters,
  placement (`slide`, `zone`, `MAX_SLIDE_ITEMS`), text, tables, charts (`dataset_row_budget`), `color`, card
  stacks (`group`, SmartArt). A text-only first cover keeps a title profile, other texts in notes.
- `hifi/` : page fit (`page`), generated tokens: `list` global, `component.base.table` (`theme`, `color`), one slide
  widget per layout in `deck-layouts` (`widget`, `pack`, bounded by `PackLimits`), master and layout decor as
  Typst (`typst/`), shape outlines and arrowheads (`geometry/`, with the embedded ECMA-376
  `presetShapeDefinitions.xml`), slide items as elements (`slide`, `element`, `table`).
- `pattern` : repeated-item signatures (retrofit requests, hi-fi `groups`); `fonts/` : embedded, catalog, substitute, `Missing`.
- `retrofit/measure`: a flow's height from theme tokens (`TextTokens`); too tall: grid if it fits, else a smaller size, else
  `Overflow` (t-816). `retrofit/zone` reads the built-in packs' manifests for the component defaults.

## Extension points
- New source format (PDF): a `pptx/` sibling emitting the same `SourceDeck` in OOXML terms (`source` docs).
- New projector: a sibling of `retrofit/` and `hifi/` returning an `ImportPlan` (+ session `ImportTarget`).
- New loss: a `LossKind` variant and its `import-loss-*` Fluent key.

## Dependencies
`oku-slides-core`, `oku-slides-chart`, `oku-slides-i18n`, `zip`, `roxmltree`, `ttf-parser`, `uuid`, `serde`,
`serde_json`, `snafu` (not render). Render checks: `oku-slides-render/tests/import_*.rs`.
