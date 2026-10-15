# Importing a PowerPoint presentation (then PDF)

Task t-692. Today Oku cannot open anything other than its own
decks (`*.oku-slides.pdf`, or JSON in development). This spec adds
the **import of a `.pptx`** into an Oku deck, in two modes:

- **hi-fidelity**: the most faithful copy possible; creates a **new theme,
  a new pack (templates + widgets)** and the deck;
- **retrofit**: projects the content onto an **existing pack and theme**,
  and creates a widget (in a child pack) only if nothing fits.

The pipeline is **as deterministic as possible**: extraction, style
resolution, skeleton and the bulk of the work are pure code; an agent only does
the **fine-tuning** at the end. PDF import is a phase 2 (separate spec)
that reuses the same intermediate representation.

Builds on `2026-10-01-packs-portables-design.md` (embedded packs,
`parent`), `2026-10-05-themes-packs-editeur-design.md` (themes),
`2026-10-02-creation-widgets-design.md` (Typst widgets, probe),
`2026-09-28-champs-editables-design.md` (`{slide, owner, field}` fields) and
`2026-10-08-widget-tableau-design.md` (tables).

## Decisions (made on 2026-10-09)

1. **PPTX first, PDF next.** v1 covers PPTX end to end in
   both modes. The intermediate representation (`SourceDeck`) is
   designed right now to accommodate a PDF reader.
2. **Hi-fi: editable first, as faithful as possible.** Texts always remain
   editable fields (`plain`/`rich`); shapes → `shape`,
   tables → `table`, charts → `chart` with their data. Only
   what cannot be translated is frozen as an image (or SVG), and the report lists it.
3. **One common core, two entry points.** An `oku-slides-import` crate
   without UI. The **CLI** does the pure deterministic part
   (scriptable, testable); the **app** does the full import with the agent.
4. **Retrofit: content + relative geometry, target theme's style.** We
   keep the structure (titles, body, lists, tables, charts, images)
   and the semantic emphasis (bold, italic). The relative layout is used to
   choose the template and the grid arrangement. Colors are
   projected onto the target tokens or dropped; no token
   override is injected.
5. **Retrofit: pattern without a widget → dedicated widget.** A deterministic fallback
   always exists and never blocks. When that fallback loses
   information (relations, data, order — not just style), the agent
   **creates a dedicated widget**, without asking, in a **child pack** of the target
   pack (the original pack is never modified), **one widget per pattern**.
6. **Hi-fi: one slide widget per PPTX layout used.** Grouping
   similar slides into element widgets is left to the agent during
   fine-tuning (it proposes, the user confirms).
7. **Fine-tuning: bounded autonomous loop, then final review.** The agent
   processes the slides from worst to best up to a threshold or an
   iteration budget, then presents a summary. Everything can be undone.
8. **Reference rendering by LibreOffice, optional.** `soffice --headless
   --convert-to pdf`, rasterized by `hayro` (already in the lockfile via
   Typst 0.15). Without LibreOffice: no visual diff, the import continues.
   Superseded by decision 1 of the lot 4 plan (office2pdf witness).
9. **Approach: intermediate representation + two projectors** (rather
   than two direct converters or an agent that does everything). The
   hard part — style cascade and geometry — is isolated in the
   reader; the projectors are pure functions tested with snapshots.

Decisions from the lot 2 plan (2026-10-10):

- **Templates in the core.** `Template` and `PackManifest` are in
  `oku-slides-core` (`pack.rs`); the session re-exports them. The import
  crate reads a pack's templates without depending on the session.
- **No child pack in lot 2.** The retrofit emits the `WidgetRequest`s
  (in the plan and the report); the child pack is created in lot 5, with the
  first widget created by the agent. `ImportPlan` has no `pack` field
  in lot 2.
- **Fonts in lot 3.** The retrofit takes the target theme's style:
  no font is looked up or substituted. `fonts.rs` (system
  lookup, substitution) arrives with hi-fi.
- **Content coverage in lot 2.** `coverage.rs` measures, on the resulting
  deck, the recall of runs, the presence of images and the equality of
  table and chart data; lot 4 keeps the reference
  and the visual gap (`metrics.rs`).

Decisions from the lot 3 plan (2026-10-11):

1. **Extended base** (core and Typst). The `shape` widget gains `shape:
   "path"` and the host parameter `path` (`{ d }`, 0..1 coordinates stretched
   to the frame, like an image); numeric `Style.radius`; `Style` gains
   `valign` (`top`/`middle`/`bottom`), `inset` (4 margins in pt) and
   `tracking` (pt), read by the `text` widget. A shape that carries text
   gives two elements (the shape, then the text) on the same frame.
   Rotation, per-run font, underline and strikethrough of elements: reported
   losses, without extension.
2. **Page.** The generated theme keeps 1920 × 1080; a source with another
   ratio is scaled uniformly (`min(1920 / w, 1080 / h)`) and
   centered, reported (`PageFitted`). Page margins of the generated theme are zero:
   the body area is the page, frames are in page pt.
3. **Fonts** (`fonts/`, pure): embedded in the PPTX → catalog (fonts
   of the app, then of the system, enumerated by the CLI with `fontdb`) →
   metric-compatible substitute (Calibri → Carlito, Cambria →
   Caladea, Arial/Helvetica → Liberation Sans, Times New Roman → Liberation
   Serif, Courier New → Liberation Mono; shipped in `assets/fonts`) →
   body font, reported. `fsType` "restricted" never embedded.
4. **Colors.** `Origin::Scheme` without a modifier → token
   reference (`lt1 → bg`, `dk1 → ink`, `dk2 → ink-muted`, `lt2 → surface`, `accent1 →
   accent`, `accent2..6 → accent-2..accent-6`, `hlink → link`, `folHlink →
   link-visited`); with modifiers or `Explicit` → resolved `#rrggbb`
   (`#rrggbbaa` with alpha).
5. **Presets.** An interpreter of the ECMA-376 guides evaluates the embedded
   `presetShapeDefinitions.xml` (all presets, `adj` included);
   `rect`, `roundRect`, `ellipse`, `line` stay native.
6. **The generated pack and theme live in the deck only** (embedded); the
   library is not touched.
7. **Probe.** The projector declares the schemas of the generated widgets (core
   types) in addition to the Typst; the CLI probes the pack through the render
   process and rejects any mismatch (a generator bug).
8. **Pack.** `_commun.typ` (decor of each master), one slide widget per
   layout used (`[a-z][a-z0-9-]*`), placeholders → `field(ctx, …)` at
   their position (`rich`; `plain` for date, footer, number), `free` slot
   `extras`; one template per layout, the first source slide that uses it
   as the sample.
9. **Effects.** `LossKind::Effects` (t-817) is emitted by hi-fi for
   ignored shadows, reflections and 3D.
10. **Bounds** (plan review). The generated pack respects the
    probe's limits (`[render]` in the config: 64 files, 256 KiB per file): at
    most `max_files − 1` layout widgets, for the most
    used layouts; the other slides take `blank`, background and decor as
    elements (`LayoutDropped`); a file that is too large loses its last
    decor items (`DecorTruncated`). The CLI reruns the probe as long as the
    budget leaves unprobed files and distinguishes a reached bound
    (`cli-import-probe-limit`) from a schema mismatch. Embedded fonts:
    32 MiB per file, 64 MiB in total. Body font missing: first
    shipped and loaded fallback font (Liberation Sans, Carlito, Inter),
    otherwise the report says so; the message names the chosen fallback.
11. **Line spacing** (plan review). The `text` widget renders `leading` (a
    `set` enclosed in an `if` did not apply). First baseline
    following the CSS model (font centered in the line pitch), shared by
    elements, placeholders and decor; the spacing between paragraphs and list
    items of an element goes through `Style.spacing` (pt).
12. **Tables** (plan review). The style is read from the cells; the
    generated theme sets `component.table` (header, bands, nullable `body-bg`,
    size, rule) from the first table; each table
    sets `header-row` and `striped`; the rest (column widths,
    per-cell colors, borders) gives a `TableStyle` loss; a header or a body with
    heterogeneous style (color, bold, italic, font) is one too.

Clarifications made along the way:

- The app's fonts (substitutes included) are not embedded: they
  are everywhere the deck is opened; only the faces from the PPTX or the system
  go into the deck's `fonts/`.
- The `numero` parameter defaults to `{n}` (the page number).
- A `decor` parameter (`flag`, default true) exists when a slide of the
  layout hides the decor (`showMasterSp="0"`).
- A slide's own background (other than its layout's) is the first
  element of `extras`, over the whole page.
- `ThemeManifest` moves down into the core (`oku_slides_core::pack`), re-exported
  by the session, built by the import.
- CLI: `--no-system-fonts`; pack and theme ids derived from the SHA-256 of the
  `.pptx` (UUID v8, two halves): two imports of the same file give the
  same pack.
- `component.list.marker` takes the master's body bullet;
  `marker-color` is nullable (`null`: the text color).

Decisions from the lot 4 plan (2026-10-14):

1. **Witness.** No embeddable PPTX renderer is a faithful reference, and the
   user ruled out a local LibreOffice install: the reference comes from the
   `office2pdf` crate (pure Rust on Typst 0.15, Apache-2.0), behind a
   `ReferenceRenderer` trait in a new crate `oku-slides-reference` (PDF
   rasterized by `hayro`), pinned at `=0.8.1` (`WITNESS` names it in the
   report). It gets the app's fonts and the deck's embedded fonts first (the
   `.pptx` it reads has its `ppt/fonts/` parts removed: office2pdf would write
   them to the system temp dir), and renders hidden slides. LibreOffice and
   PowerPoint witnesses are backlog work, as other implementations of the trait.
2. **Isolation.** The witness reads an untrusted file without bounds of its
   own: it runs in a render worker of its own, not the one holding the Oku
   render (`Reference`, `ReferencePng`, `ReleaseReference` requests;
   `RenderWorker::reference` → `RemoteReference`), under `[import]
   reference_timeout_ms` (30 s) and `memory_mib`, 20 slides per request
   (office2pdf's `slide_range`): memory does not grow with the deck. Slides pair with the
   witness pages by their source number. A failing or mismatched reference, or
   a slide that cannot be scored (`ReferenceCause`: `failed`, `timeout`,
   `memory`, `pages`), is reported and logged, never an error, and leaves no
   slide with a score.
3. **Metric** (`oku-slides-import/src/metrics.rs`, pure, calibrated on alto).
   Slide score: mean OKLab ΔE ×100 over the source page at 960 px after a
   σ = 2 px Gaussian blur (at σ = 1, alto titles aligned within 1 px ranked
   above the real gaps). Region score: each pixel belongs to the topmost
   item covering it (at most 256 regions, the topmost); the region is
   realigned by the integer shift (at most 3 px per axis) that minimizes its
   mean gap, the shift is reported; its score is the mean gap of its worst
   16 px tile (tiles overlap by half); below 100 own pixels or with a negative
   frame, no score. The report keeps the 3 worst regions.
4. **Threshold** (plan review). The slide mean does not separate a faithful
   slide from a damaged one (the broken table of `libreoffice.pptx` p. 1
   scores 0.6): a slide is flagged when its **worst region** exceeds
   `[import] score_threshold = 10.0`. Measured on alto: a title aligned
   within 1 px scores 7.7–8.3 (the noise floor), the real gaps 8.7–22.5, the
   witness's own errors up to 46; the flag is a coarse alert, the regions
   point at the item. At 10 the alto p. 8 bullet-indent gap (worst region
   8.72) is not flagged, though the regions still designate it; recalibrating
   the threshold belongs with the autofit-aware ranking (backlog t-884).
   Tests run the witness live (pinned, no versioned
   reference PDF) with per-fixture bounds on the slide mean (measurement +
   25 %) and designation checks: alto p. 8 bullets and p. 7 callout among
   the 3 worst regions, the `libreoffice.pptx` table first.
5. **Retrofit.** No visual gap (another theme); `--reference` is refused there.
   In both modes, a non-empty text field rendered past its page edge becomes an
   `Overflow` loss on its slide (t-816 stays open for the fix).
6. **Known witness limits** (plan review, real render). It shrinks
   `normAutofit` text without `fontScale` (the alto p. 2 title and every
   footer), draws letter-spaced titles 2–5 px wider than Oku (which matches
   HarfBuzz within 1 px) and LibreOffice, draws charts roughly and SmartArt
   text too small: those regions often rank first. It shares Typst's line
   breaking with Oku (a PowerPoint line-break gap, t-863, is invisible to
   it), and office2pdf always keeps the system fonts as its fallback. A
   behavior copied from office2pdf into the import is no longer measured by
   it. An autofit-aware ranking is backlog work before lot 5.
7. **Cost.** office2pdf brings its DOCX and XLSX readers into the render
   worker, hence the desktop app (about 48 crates); no feature flag in lot 4,
   since lot 5 needs the witness in the app.

Defaults chosen without asking:

- **Page size.** Hi-fi: 1920 × 1080, the source scaled and
  centered (decision 2 of lot 3). Retrofit: the target theme's page, the
  projection onto the grid absorbs the ratio difference.
- **Fonts** (hi-fi, lot 3). Lookup order: fonts embedded in the PPTX →
  app and system fonts → metric-compatible substitute (Carlito ↔ Calibri,
  Caladea ↔ Cambria, Liberation ↔ Arial/Times/Courier), reported in the report.
  The chosen font is embedded in the deck's `fonts/`, except the app's own.

## Architecture

```
.pptx ──▶ pptx::read ──▶ SourceDeck ──┬─▶ hifi::project ─────────┐
                                     └─▶ retrofit::project ─────┤
                         (target pack, registry, theme) ────────┘
                                                                 ▼
                                                            ImportPlan
                                     ┌───────────────────────────┤
                                     ▼                           ▼
                        CLI: new session, apply,       App: new session, apply,
                        save + report.json             then fine-tuning agent
```

Crate `crates/oku-slides-import`:

| Module | Role |
|---|---|
| `source/` | `SourceDeck` and its types (neutral format) |
| `pptx/` | OOXML reader: rels, presentation, theme, master, layout, slide, chart, table |
| `collect.rs` | neutral outputs of any read: deduplicated assets, bounded warnings, deferred fatal error, string sanitization |
| `pptx/cascade.rs` | text style resolution (a single function) |
| `pptx/context.rs` | context of a part (relationships, colors, linked parts) |
| `pptx/placeholder.rs` | placeholder linking and inheritance |
| `pptx/frame.rs` | graphic frames: table, chart, SmartArt, OLE |
| `pptx/units.rs` | EMU → pt, angles, percentages |
| `pptx/eot.rs` | EOT embedded fonts (`.fntdata`) |
| `pptx/geometry.rs` | reading geometries (`prstGeom`, `custGeom`); the conversion to a path is in `hifi/geometry/` |
| `pptx/fonts.rs` | inventory of the families used, embedded fonts (lot 1) |
| `fonts/` | font resolution: catalog (app, system), substitutes, faces to embed (lot 3) |
| `hifi/mod.rs` | hi-fi projector: page, fonts, theme, pack, slides, ops, coverage (lot 3) |
| `hifi/page.rs`, `hifi/color.rs` | source page onto 1920 × 1080; color as a token or as hexadecimal |
| `hifi/theme.rs` | tokens and `ThemeManifest` of the generated theme |
| `hifi/geometry/` | ECMA-376 presets and `custGeom` as paths normalized to the frame (guides, pen) |
| `hifi/typst/` | Typst generator of the decor (shapes, paths, text, images) and of the placeholders |
| `hifi/widget.rs`, `hifi/pack.rs` | slide widget of a layout; generated pack, declared schemas, `pack.json` |
| `hifi/slide.rs`, `hifi/element*.rs` | a slide: placeholders as parameters, the rest as `extras` elements |
| `retrofit/` | retrofit projector: template scoring, grid, colors, fallbacks |
| `pattern.rs` | pattern signature (shared by retrofit / hi-fi grouping) |
| `plan.rs` | `ImportPlan`, `WidgetRequest` |
| `coverage.rs` | content coverage, measured on the resulting deck (lot 2) |
| `metrics.rs` | visual gap and worst regions (hi-fi, lot 4; images rendered by the CLI) |
| `report.rs` | `ImportReport` |

Dependencies: `zip` (existing), `roxmltree` (new, read-only DOM). The
reference witness and the rasterization live in `oku-slides-reference`
(`office2pdf`, `hayro`), run by the render worker (lot 4). The crate
depends neither on the session nor on the UI: it takes as input the widget
registry, the pack's templates (`oku_slides_core::Template`) and the target
theme's tokens, and returns a plan.

## Intermediate representation: `SourceDeck`

**Resolved** (no inheritance left to compute), but each styled value
keeps **its provenance**. The authoritative definition is the code:
`crates/oku-slides-import/src/source/mod.rs` (and its submodules), with the
"Review decisions" of the lot 1 plan
(`docs/superpowers/plans/2026-10-09-import-pptx-lot1.md`), which take precedence over
any earlier sketch: one theme per master, `Origin::Scheme { name, slot,
mods }`, `PlaceholderLink`, typed per-level styles, typed warnings.

- `frame` is in **source pt**; scaling to 1920×1080 is the
  projector's job.
- Masters and layouts keep their decor items (non-placeholders) and their
  placeholders (role, `idx`, frame): hi-fi needs them to generate the
  widgets, retrofit for the layout's `type`.

## PPTX reader

- Order: `[Content_Types].xml` → `presentation.xml` (+ rels) → `theme` →
  masters → layouts → slides → linked parts (images, charts, diagrams,
  notes).
- **EMU → pt**: 12,700 EMU per pt.
- **Text style cascade**, a single function:
  `master txStyles → layout placeholder → slide placeholder → lstStyle →
  pPr → rPr`, per paragraph level. Table-tested.
- **Placeholders**: a slide placeholder inherits frame and style from the
  layout placeholder with the same `type`/`idx`, which itself inherits from the master's.
- **SmartArt**: imported from its already drawn version
  (`ppt/diagrams/drawingN.xml`) as a `Group` of editable shapes.
- **`mc:AlternateContent`** (decision 11 of the lot 1 plan): the branch is
  chosen from `Requires` before being read (`Choice` if each prefix
  designates a known namespace, otherwise `Fallback`); only the equation
  (`oMath`) becomes `Opaque { Math, fallback }`.
- **Charts**: `ppt/charts/chartN.xml`, values read from
  `c:numCache`/`c:strCache` (the embedded workbook is not opened).
- **Tables**: `a:tbl`, merges (`gridSpan`, `rowSpan`, `hMerge`,
  `vMerge`), resolved table style (`tableStyles.xml`).
- **Outside v1**: animations, transitions; video/audio → `Opaque` with
  the poster image; effects (shadow, reflection, 3D) ignored and reported.

## `ImportPlan`

```rust
ImportPlan {
    theme: Option<PackFiles>,      // hi-fi: generated theme (`theme.json`)
    pack: Option<PackFiles>,       // hi-fi: generated pack (lot 3); retrofit: none in lot 2, child pack in lot 5
    schemas: Vec<WidgetSchema>,    // hi-fi: declared schemas, which the probe must find again
    assets, fonts,                 // fonts: files of the deck's `fonts/`
    ops: Vec<Op>,                  // existing `apply` ops
    widget_requests: Vec<WidgetRequest>,
    report: ImportReport,
}
WidgetRequest { signature, slides: Vec<SlideId>, sample: Vec<Item>, fallback_applied }
```

The caller opens a new deck, installs theme/pack/assets/fonts, then
applies `ops` in **a single transaction**: the import goes through the same
validations as the agent and is undone in one go.

## Hi-fi projector

Delivered in lot 3 (decisions of the lot 3 plan above).

**Generated theme** (`parent`: none; that of the first slide's master):

| Source | Tokens |
|---|---|
| `clrScheme` | `color`: `lt1→bg`, `dk1→ink`, `dk2→ink-muted`, `lt2→surface`, `accent1→accent`, `accent2..6→accent-2..accent-6`, `hlink→link`, `folHlink→link-visited`; `on-accent` (`bg` or `ink` depending on contrast), `rule`; `chart.palette` = `accent1..6` |
| `fontScheme`, fonts | `font.display = major`, `font.body = minor`, through the chosen family (`FontPlan::used`); default kept if absent |
| placeholders of the master and layouts, otherwise `txStyles` | `size.title`, `size.subtitle`, `size.body`, `size.small` (× scale); `weight.body` 400, `weight.strong` 700 |
| first-level bullet of the body | `component.list.marker`, `marker-color` (its color or `null`) |
| first table of the deck (resolved cells) | `component.table`: `header-bg`, `header-color`, `header-weight`, `body-bg` (nullable, added to the base), `stripe-bg`, `size`, `rule-color`, `rule-width`; each table sets `header-row`/`striped` read from its cells, `TableStyle` loss for what the widget does not render |
| `sldSz` | `page`: 1920 × 1080, zero margins; scale and centering carried by the frames |

**Generated pack**:

- `_commun.typ`: `champ` and the decor of each master.
- **One slide widget per layout used** (`l<rang>-<slug>`): background, decor
  of the master (if the layout shows it) and of the layout; each text
  placeholder → `field(ctx, …)` at its exact position (`titre`, `sous-titre`,
  `corps`… as `rich`; `date`, `pied`, `numero` as `plain`, `numero` at
  `{n}`); nothing for image, chart, table, media placeholders;
  `grid`; a `free` slot named `extras` over the whole page; `decor` (`flag`)
  if a slide hides the decor.
- One template per layout in `pack.json`, with the first source slide
  that uses it as the sample.

**Typst generator** (deterministic): a decor `Item` → `place(dx, dy,
rect | ellipse | line | curve | image | text)` in page pt. `rect`,
`roundRect`, `ellipse`, `line` native; any other preset (and `custGeom`)
converted to `curve` by the ECMA-376 geometry formulas
(`presetShapeDefinitions.xml`, interpreted). Rotation and mirroring drawn;
table, chart, opaque without fallback → `Opaque` loss. All source text
goes through an escaped string literal.

**Deck**: each slide → `add_slide` with `kind = <widget du layout>`
(`blank` without a readable layout), text placeholders left in their box →
`params`, other items → elements (`text`, native `shape` or `path`,
`image`, `table`, `chart`) in `extras`, frames scaled and
centered; own background first. `Scheme` color without a modifier →
**token reference**; otherwise hexadecimal. Hidden slide → `HiddenSlide`
loss.

## Retrofit projector

Inputs: `SourceDeck`, target pack (templates), `WidgetRegistry` (probe),
target theme.

1. **Template choice by deterministic score.** For each template:
   required fields that can be filled, slots compatible with the content
   types, match of the source layout `type` (`title`, `secHead`,
   `obj`, `twoObj`, `titleOnly`, `blank`…). Tie → pack order.
   Before the choice (lot 2): repeated decor (header, footer, logo in the
   top and bottom bands of the page, on at least half of the slides) and
   the date, footer, number placeholders are omitted, mentioned once in the report;
   without a title placeholder, title, kicker and subtitle are inferred from
   the size and position of the texts, and a cover from a title
   of at least 40 pt; a filled shape that carries texts becomes a
   card (title, kicker, body), and neighboring texts whose cells
   overlap merge. A row of cards makes columns.
2. **Layout.** A "title + one body" slide or a row of
   columns goes into `flow` blocks (the template's `columns`/`split` layout; a
   row between full-width texts becomes a stack in columns).
   Otherwise the bounding box of the contents is fitted into the body area
   (uniform scale, centered): frames on a free template, cells of the
   template's grid if they do not overlap, otherwise frames.
3. **Colors.** `Scheme(accentN)` → fixed table to the target tokens;
   the deck's saturated colors without a close token, ranked by usage,
   take `accent` then the theme's other `accent-*`; `Explicit` →
   nearest token in **OKLab ΔE** under a threshold, otherwise dropped.
   A text color is emitted only if readable (WCAG contrast ≥ 3:1 on
   its background, page or card; on a dark background, a light text token of the
   theme). A dark card becomes a raised card. Bold and italic
   kept; text in caps is written out in uppercase.
4. **Patterns without a widget.** The fallback (e.g. a `stack` of `card` for a
   SmartArt) is applied right away: the deck is **complete and
   renderable**. If the fallback loses information (omitted shapes, regular
   pattern of at least three boxes, SmartArt), a `WidgetRequest` is
   emitted, grouped by **pattern signature** (multiset of item types +
   normalized relative layout, `pattern.rs`). The agent handles it during
   fine-tuning (decision 5), in a child pack created in lot 5.

**Known limit of lot 2 (t-816).** In flow, content taller than its area
overflows the page without the import noticing: a target theme with a larger
body (Papier, slides 6 and 7 of alto), or, with the default theme, a
chart or an image that fills a column in `columns` flow (kitchen,
slide 2, on Essentiel and Pro: "Gauche" and "Droite" are cut off below the
chart). Content coverage measures the deck model, not the rendering:
it does not see it; only the reference rendering (lot 4) will measure it.

## Reference rendering and metrics

- **Reference** (`oku-slides-reference`, lot 4 decisions 1 and 2): the
  `office2pdf` witness in the render worker, PDF rasterized by `hayro`.
  Absent or failing → reported, no error.
- **Hi-fi — visual gap**: Oku rendering cropped to the source page and
  reference at the same resolution (960 px wide), mean OKLab ΔE after a
  light blur; plus a **per-region score** from the items' `frame`, to point
  at the faulty element (lot 4 decision 3).
- **Retrofit — content coverage**: text recall (each source run
  found), presence of images, equality of table and chart
  data. Target 100%; every gap is listed.

## Import report

`ImportReport` (JSON): per slide, score (gap or coverage), fallbacks
applied, `Opaque`, ignored effects; globally, substituted fonts
(`fonts`: one decision per family, lot 3), reference (witness, threshold,
cause when missing; `above_threshold` on each slide beyond it, lot 4),
`widget_requests`. Messages through Fluent
(`oku-slides-i18n`).

## Agent fine-tuning (app)

- **New MCP tools** (`oku-slides-mcp`): `import_report` (the
  current report) and `import_diff(slide)` (reference PNG, Oku PNG,
  per-region scores).
- **Bounded autonomous loop**: slides from worst to best;
  corrections through `apply` / `set_widget`; stop under the threshold or after N
  iterations per slide (threshold and N in config).
- **Retrofit**: each `WidgetRequest` → widget created in the child pack
  (`set_widget`, `preview_widget`), then fallback replaced on all the
  slides of the pattern, without asking.
- **Hi-fi**: the agent proposes the groupings ("N slides share this
  pattern → element widget?") through `ask_confirm`.
- **End**: summary through `report` (changed, still above the threshold,
  frozen). Everything goes through the session, hence undoable.
- The **fine-tuning instructions** are a versioned text shipped with
  the app, not hard-coded.

## Entry points

- **CLI** (`oku-slides-cli`), purely deterministic:
  `oku-slides import deck.pptx --mode hifi|retrofit [--pack <id|chemin|nom>]
  [--theme <id|nom>] [--reference auto|none] -o out.oku-slides.pdf
  [--report r.json] [--no-system-fonts]`. `--mode` defaults to `retrofit`;
  `--pack` is required there. `--mode hifi` is delivered in lot 3:
  `--pack` and `--theme` are rejected there (the pack and the theme are generated,
  embedded in the deck only); the presentation's fonts are
  looked up among the app's (`--fonts`) and, unless `--no-system-fonts`,
  the system's. `--reference auto|none` (lot 4): hi-fi only, `auto` by
  default; a second output line gives the mean gap and the slides whose worst
  region is above the threshold, or why the reference is missing.
- **App**: "Ouvrir" (Open) accepts `.pptx`; a dialog asks for the mode, and in
  retrofit the target pack and theme. The app creates a new deck from the
  plan, then launches the agent with the fine-tuning instructions.

## Errors

Opaque Snafu `ImportError`, **only for the fatal cases**: invalid zip,
missing `presentation.xml`, encrypted file (or 97-2003 `.ppt`), read
limits exceeded (`ReadLimits`: protection against zip bombs;
exceeding `max_xml_nodes` is `TooLarge`, fatal, never an omitted part), target
pack not found. A `TooLarge` raised while reading any part (slide, notes,
chart, SmartArt, fonts, table styles) is propagated as a fatal
error (`PartCtx::fail` / `soft`), never degraded into an omitted part with
`UnreadablePart`. Conversely, an XML part nested more than 256
levels deep is not a `ReadLimits` limit: it is rejected as
unreadable (`Unreadable`) and handled like any unreadable part — omitted with
`UnreadablePart` (linked item: `Opaque` + `UnreadableItem`), fatal only
for a structural part (`[Content_Types].xml`, root relationships,
`presentation.xml`) or for lack of a usable master. An unreadable
item never makes the import fail: it becomes `Opaque`, with an
entry in the report.

## Tests

- **Fixtures**: hand-written unzipped OOXML
  (`fixtures/import/<nom>/`, zipped on the fly by the tests — readable and
  reviewed as diffs), plus `libreoffice.pptx`, the same deck rewritten by
  LibreOffice (verbose, realistic). *(Clarified in the lot 1 plan:
  `python-pptx` is not available and hand-written XML files
  read better.)*
- **Real deck**: `fixtures/import/alto-wealth.pptx` (generated by PptxGenJS)
  and `alto-wealth.pdf`, an HTML render without embedded fonts (text ~5 %
  narrower), visual target of lot 6; lot 4 measures against the witness.
- **`insta` snapshots** of `SourceDeck` and `ImportPlan` per fixture.
- **Style cascade** table-tested (`rstest`).
- **Hi-fi gold**: the witness runs live in CLI tests (no LibreOffice, no
  versioned reference PDF); visual gap under a per-fixture bound, worst
  regions pointing at known gaps (lot 4 decision 4).
- **Retrofit**: 100% content coverage on the fixtures.
- **CLI**: end-to-end integration test (pptx → reopenable
  `.oku-slides.pdf`).

## Lots

One plan and its subtasks per lot, in this order:

1. *(delivered, t-692)* `SourceDeck` + PPTX reader (style cascade included) — plan:
   `docs/superpowers/plans/2026-10-09-import-pptx-lot1.md`.
2. *(delivered, t-692)* Retrofit projector + CLI + content coverage —
   validates the representation end to end without a generator or reference;
   plan: `docs/superpowers/plans/2026-10-10-import-pptx-lot2.md`.
3. *(delivered, t-692)* Hi-fi projector: theme, Typst generator, pack,
   fonts (`fonts/`), extended base (paths, text anchoring); plan:
   `docs/superpowers/plans/2026-10-11-import-pptx-lot3.md`.
4. *(delivered, t-692)* Reference rendering (`office2pdf` witness in the
   render worker) + visual gap + overflow losses; plan:
   `docs/superpowers/plans/2026-10-14-import-pptx-lot4.md`.
5. App + agent fine-tuning (MCP tools, instructions, child pack of the
   widgets created for the `WidgetRequest`s).
6. *(Phase 2, separate spec)* PDF reader → `SourceDeck` through
   `hayro-interpret`, with inference of semantic hints (roles,
   master from repeated elements, paragraphs).
