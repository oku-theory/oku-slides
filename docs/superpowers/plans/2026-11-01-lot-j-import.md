# Lot J — hi-fi chart fidelity and lot E follow-ups (t-1384, t-1385)

Goal: imported charts draw the source's gridlines, bar width and value scale (t-1384), and the seven
deferred lot E review items are closed (t-1385). One squashed commit on `main`.

Parents: steps 1 to 4 are sub-tasks of t-1384, steps 5 to 10 of t-1385 (step 10 covers both).

## Steps

1. **Chart options gain gridlines, bar gap and tick interval.** `oku-slides-chart`: `Axis.splitLine`
   (`{ show }`, ECharts name; value axis default shown), `Axis.interval` (tick step, > 0), bar
   `Series.barCategoryGap` (`Length`, ECharts name: gap between categories, `"40%"` of the band).
   `ChartModel` carries them; `ChartsRsRenderer` hides the grid (transparent stroke), turns
   `interval` into exact bounds and split count (bounded), and narrows the bars of each category
   around its centre by rewriting the bar `<rect>`s charts-rs 1.1 emits (it has no gap option).
   Validation: `interval` finite > 0, gap percent in [0, 100). Schema snapshot and agent docs follow.
2. **The PPTX reader reads gridlines, gap width and scaling.** `source::Chart` gains `gridlines`
   (the value axis has `c:majorGridlines`), `gap_width` (`c:gapWidth`, default 150, clamped 0..500)
   and `scale` (`c:scaling` `c:min`/`c:max`, `c:majorUnit`, finite, min < max). `retrofit::chart`
   writes `splitLine.show`, `min`/`max`/`interval` and `barCategoryGap = g / (n + g)` (n bar slots).
3. **`c:delete` follows the app version (t-1385 item 3).** The reader reads `docProps/app.xml`; in a
   file written by PowerPoint 2007 (`AppVersion` 12.x) an absent or bare `c:delete` means shown,
   as oox's `bMSO2007Doc`; elsewhere unchanged (t-954).
4. **Measure kitchen p2 and drop `WitnessLimit::Chart`.** Re-measure under office2pdf and
   LibreOffice; when the chart region scores under the threshold, remove the variant, its Fluent
   key and the measured comments (`import_reference.rs`, `config.default.toml`).
   Outcome: gridlines and bar widths now match, but the chart region still scores 33.6 under
   LibreOffice (46.1 before) and 46.1 under office2pdf, which draws the deleted axes; the gap left
   is PowerPoint's automatic value scale. `WitnessLimit::Chart` stays; follow-up t-1493.
5. **Table rows overflow from slack first (item 1).** `table.typ`: a row taller than its weight
   takes its overflow from rows with slack before the table shrinks with fit-h. New base version.
6. **Direct Typst render test for column widths and row heights (item 2).** 1:2 ratio drawn; a wrong
   count draws unweighted.
7. **`packs-add` reworded in 20 locales (item 4)**: the button also lists deck-only packs.
8. **OS pick queue: 'dismiss all' and a full-queue notice (items 5, 7).** A request forgets every
   pick of the window up to the newest one the page showed (`through`), so a pick arriving
   meanwhile is still offered; a dropped `.pptx` raises a localized notice instead of a log only. The
   notice counts the dropped files and names the first three, cleaned by `bounds::label`, then
   "and N others" (user decision, 2026-11-01).
9. **`just test-soffice` cannot lose a renamed test (item 6).** Before running, the gate finds
   every test ignored with the `real LibreOffice` marker, refuses one outside the two packages it
   runs or an `ignore` that names LibreOffice without the marker, and fails unless the name filter
   selects each marked test (by name and by count).
10. **Review round and `just ci`.** Full review roster, fixes, `just ci` and `just test-soffice` green.
