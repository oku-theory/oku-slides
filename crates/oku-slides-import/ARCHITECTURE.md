# oku-slides-import

PowerPoint (`.pptx`) import: read into a format-neutral, resolved intermediate representation
(`source::SourceDeck`, every styled value with its provenance), then project it into a plan of
Oku ops. Spec: `docs/superpowers/specs/2026-10-09-import-pptx-design.md`.

## API

- `read_pptx(bytes, &ReadLimits) -> Result<SourceDeck, ImportError>`.
- `retrofit::project(&SourceDeck, &RetrofitTarget) -> ImportPlan` : the content laid out on an
  existing pack and theme (templates, registry, `ColorTokens`, `PageTokens`).
- `hifi::project(&SourceDeck, &HifiTarget) -> ImportPlan` : the source reproduced on a generated
  theme and pack; `hifi::sought_fonts` lists the families it will look for.
- `ImportPlan` (`plan`) : ops, assets, `WidgetRequest`s, report; in hi-fi also the generated
  theme and pack (`PackFiles`), declared schemas and fonts; `report_json` is the `--report` form.
- `report` : `ImportReport`, `Loss`/`LossKind`, warnings, `Coverage`; `coverage::coverage`
  measures what of the source survives in the resulting deck.
- `fonts` : `FontCatalog`, `FontOrigin`, `FontPlan`, `SUBSTITUTES`, `FALLBACKS`.

## Layers

- `source/` depends on nothing; `error`, `limits`, `collect` (`Sink`) are neutral.
- `pptx/` is the only reader: `package` (bounded zip), `xml` (bounded nodes), `context`, `master`,
  `slide`, `placeholder`, `cascade` (the single text-style resolution), `shapes`, `frame`,
  `table`, `chart`, `smartart`, `fonts` + `eot`. Nesting is bounded by `GroupCtx`.
- Projectors `retrofit/` and `hifi/` use `source/`, `plan`, `report`, `coverage`, `fonts`,
  never `pptx/`. They are pure: ids and dates come from the target.
- `retrofit/` : decor omission (`decor`, shared with `coverage`), template choice (`template`),
  headings, clusters, flow or free placement (`slide`, `zone`), text, tables, charts, `color`.
- `hifi/` : page fit (`page`), theme tokens and `ThemeManifest` (`theme`, `color`), one slide
  widget per layout (`widget`, `pack`, bounded by `PackLimits`), master and layout decor as
  Typst (`typst/`), shape outlines (`geometry/`, with the embedded ECMA-376
  `presetShapeDefinitions.xml`), slide items as elements (`slide`, `element`, `table`).
- `pattern` : retrofit's signature that groups repeated item groups into one widget request.
- `fonts/` : embedded font, then app or system catalog, then metric substitute, then `Missing`.

## Extension points

- New source format: a sibling of `pptx/` that produces the same `SourceDeck`.
- New projector: a sibling of `retrofit/` and `hifi/` returning an `ImportPlan`; the CLI
  `ImportMode` gains a variant.
- New loss: a `LossKind` variant and its `import-loss-*` Fluent key.

## Dependencies

`oku-slides-core`, `oku-slides-chart`, `oku-slides-i18n`, `zip`, `roxmltree`, `ttf-parser`,
`uuid`, `serde`, `serde_json`, `snafu`.
Hi-fi render snapshots: `oku-slides-render/tests/import_*.rs`.
