# oku-slides-import

PowerPoint (`.pptx`) import: read into a format-neutral, resolved intermediate representation (`source::SourceDeck`,
every styled value with its provenance), projected into a plan of Oku ops. Spec: `specs/2026-10-09-import-pptx-design`.

## API

- `read_pptx(bytes, &ReadLimits) -> Result<SourceDeck, ImportError>`.
- `retrofit::project(&SourceDeck, &RetrofitTarget) -> ImportPlan` : content on an existing pack and theme; the
  session pins `base` first. Element kinds come only from `oku_slides_core::kinds::base`.
- `hifi::project(&SourceDeck, &HifiTarget) -> ImportPlan` : the source on a generated theme and pack requiring `base`;
  ops start with `AddPack base` (`HifiTarget.base`); `hifi::sought_fonts`, fonts it looks for; `hifi::source_rect`, the page.
- `ImportPlan` (`plan`) : ops, assets, `WidgetRequest`s, report, `origins` (item → element); in hi-fi the
  generated theme, pack, schemas, fonts, `targets` (item → elements or slide param), `groups` (`hifi::groups`);
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
- `retrofit/` : decor omission (`decor`, shared with `coverage`), template choice (`template`), headings, clusters,
  placement (`slide`, `zone`, `MAX_SLIDE_ITEMS`), text, tables, charts (`dataset_row_budget`), `color`, card
  stacks (`group`, SmartArt). A text-only first cover keeps a title template, other texts in notes.
- `hifi/` : page fit (`page`), theme tokens and `ThemeManifest` (`theme`, `color`), one slide
  widget per layout (`widget`, `pack`, bounded by `PackLimits`), master and layout decor as
  Typst (`typst/`), shape outlines and arrowheads (`geometry/`, with the embedded ECMA-376
  `presetShapeDefinitions.xml`), slide items as elements (`slide`, `element`, `table`).
- `pattern` : repeated-item signatures (retrofit requests, hi-fi `groups`); `fonts/` : embedded, catalog, substitute, `Missing`.
- `retrofit/measure`: a flow's height from theme tokens (`TextTokens`); too tall, it goes to the grid if
  that fits, else a smaller theme size, else reports `Overflow` (t-816; never split). `place` lays out.

## Extension points

- New source format (PDF): a `pptx/` sibling emitting the same `SourceDeck` in OOXML terms (`source` docs).
- New projector: a sibling of `retrofit/` and `hifi/` returning an `ImportPlan` (+ session `ImportTarget`).
- New loss: a `LossKind` variant and its `import-loss-*` Fluent key.

## Dependencies
`oku-slides-core`, `oku-slides-chart`, `oku-slides-i18n`, `zip`, `roxmltree`, `ttf-parser`, `uuid`, `serde`,
`serde_json`, `snafu`. Render checks: `oku-slides-render/tests/import_*.rs`.
