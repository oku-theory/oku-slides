# Lot E: PPTX import backlog batch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Parent task:** t-1362. **Sub-tasks:**

| Task | Sub-task | Ticket |
|---|---|---|
| 1 | t-1367 | t-1295 |
| 2 | t-1368 | t-954 |
| 3 | t-1369 | t-955 |
| 4 | t-1370 | t-957 |
| 5 | t-1371 | t-961 |
| 6 | t-1372 | t-937 |
| 7 | t-1373 | t-960 |
| 8 | t-1374 | — |

**Goal:** close the seven import tickets of lot E in one squash commit on `main`.

**User decision** (2026-10-31, binding): **t-960:** mark the real-LibreOffice tests
`#[ignore = "…"]` and run them with a `just test-soffice` recipe (`--run-ignored ignored-only`), like
`just test-apple` in `oku-slides-stt`; the env var goes, the t-896 rule is rewritten.

## Task 1: t-1295 reproduce images lost on reopen

Commit 6dd6588a fixed grey fallback frames after a render worker restart and added
`issue_1295_imported_images_survive_reopening` (resume in the same process, real render
subprocess). Reproduce the ticket's own scenario end to end: `oku-slides serve` with fresh data,
import `kitchen` and `alto-wealth` in hi-fi through `/api/workspace`, read each slide's SVG, kill the
server (SIGTERM), restart it on the same data and a free port, read again; isolated
`OKU_SLIDES_PATHS__DATA`/`__DECKS`. Check `<data>/resume/tabs/t-N/deck/assets/`.
- Images still drawn after the restart: close t-1295 with the evidence (commands and counts).
- Otherwise: failing regression test first, then keep the assets in the deck.

## Task 2: t-954 hi-fi chart without axes or legend

- `source::Chart` gains `legend: bool` (a `c:legend` under `c:chart`) and `axes: Axes { x, y }`.
  Axes are read by role among the axes the first chart type names (`c:axId`, at most 8): a
  category or date axis is x, a value axis y, swapped for horizontal bars; a scatter's two value
  axes go by `c:axPos`, else by order. An axis is shown only with `c:delete val="0"`: an absent
  `c:delete` and an absent axis mean hidden, as LibreOffice and python-pptx read them.
- `oku-slides-chart` `Axis` gains `show: Option<bool>` (ECharts `xAxis.show`); `ChartModel.axes_show`
  carries it; the charts-rs renderer sets `x_axis_hidden` / `y_axis_hidden` (bottom / left).
- `retrofit::chart::chart` writes `show: false` on a hidden axis and always writes `legend.show`.
- Tests: `chart_tests` for reading, `retrofit/chart_tests` for the option, chart convert and render
  tests, regression `issue_0954_chart_deleted_axes_and_no_legend.rs` (kitchen, libreoffice.pptx).
- Review whether `WitnessLimit::Chart` still hides a real gap (ticket); note the answer.
  Answer (2026-10-31): yes. With axes and legend fixed, kitchen p2 still scores 46.1 against both
  witnesses: Oku draws gridlines the source lacks, bars wider than `gapWidth` 150 % and another value
  scale. The limit stays; the gap is a follow-up ticket.

## Task 3: t-955 hi-fi table geometry

Cause: the hi-fi import draws a table with the base `table` widget, which sizes columns on their
content and rows on their text, with the theme's padding, then scales the whole table down to fit
its frame: narrower columns, lower rows and a smaller font than PowerPoint.
- `TableOption` gains `column-widths` (relative weights, one per column) and `row-heights`
  (minimum heights as weights of the frame height, one per row). Validated in `validate_table`
  (`valid_weights`: length equal to the column or row count, each finite, > 0 and at most
  `MAX_TABLE_WEIGHT` = 1e6, so a sum stays finite) with a new `chart-table-geometry` Fluent key in
  20 locales; rejected on patch and data ops like any invalid option.
- `table_data` keeps them aligned for every `DataOp` (insert: the mean weight, remove: dropped,
  replace: kept only along a dimension whose count is unchanged). A patch with a list of the wrong
  length is rejected (`chart-table-geometry`). Deviation (review): `parse_table` drops a list of the
  wrong length, like `align` is truncated, because the rows are merged by Loro and the weights are
  last-writer-wins: two concurrent row inserts would otherwise leave the table uneditable.
- `table.typ`: column weights become fractions of the width; row weights are normalized to the
  height and soft (each row is the max of its measured content and its weight), ignored when the
  height is infinite; length guards in Typst too. The shrink-to-fit still applies. Append a new
  base version to `typst/oku-slides/packs/base/versions.txt`; the golden render without the new
  keys is unchanged.
- Hi-fi: the table option carries weights only when all are > 0, sliced to the 20 columns / 100 rows
  kept; `TableLook` writes `padding-x`/`padding-y` from cell insets when uniform (`l == r`,
  `t == b`), clamped to the token's 400 pt maximum; other margins are irregular, and `matches`
  compares padding (a token). Unequal widths and stretched rows are no longer irregular.
- Docs: MCP `tools.rs` table description, `ops/mod.rs` option doc, chart ARCHITECTURE.md.
- Regression `issue_0955_hifi_table_keeps_its_geometry.rs`; rewrite the table-flag tests of
  `crates/oku-slides-cli/tests/import_reference.rs` and recalibrate its bounds and the measured
  table in `config.default.toml` after live office2pdf runs on alto, kitchen and libreoffice
  (LibreOffice through `just test-soffice`).

## Task 4: t-957 witness reason through Fluent

`ImportState::choose_witness` writes a French hard-coded detail and `e.to_string()`.
- New `ReferenceCause::Refused` (Fluent `import-reference-unavailable`, `[refused]` branch, 20
  locales): LibreOffice found but not usable (a script on macOS).
- `oku-slides-reference`: the script refusal is its own `ReferenceErrorKind::Script`, display in
  English; `soffice.rs` error texts in English.
- `witness_missing` takes the searched paths (logged as the detail, the localized cause says the
  rest); `witness_refused` takes the `ReferenceError`: cause `Refused` for a script, else `Failed`,
  its display logged as the detail.

## Task 5: t-961 queue of OS import picks

`Workspace.import_pick` becomes a queue `import_picks` (at most 32; a path already queued is not
added twice; it keeps its first window); each pick keeps its window label and a fresh id.
`WorkspaceView.import_picks` lists every window's picks (deviation: the view is shared, each page
filters by its label); `ImportPicked` removes that id. `forget_window` moves a closed window's picks
to the end of the queue for the focused window with new ids (to the next window when none is
focused), so the queue stays ordered by id.
Renamed across the session, `src-tauri/src/windows.rs` and the UI fixtures.
UI (`useImportPick`): take the pick with the lowest `id > handled`; do not offer while a dialog is open
or an import runs (`free`), and offer again once it frees, at most once per pick; the
`useFileActions` caller follows. Tests on both sides and regression
`issue_0961_several_pptx_from_the_os.rs` in the session crate.

## Task 6: t-937 deck-only pack in the packs panel

A deck-only pack removed from the deck stays in its sources. The packs panel's add choices list it
(deviation: the newest leaf of each deck-only pack id not pinned, a version no embedded version of
any scope names as `parent`, which cannot loop; then `created`, then version), with its label, one
per namespace. A separate `pin_deck_only` (found by `deck_only_choice`) pins it without touching the library (the library
`add_library_pack` keeps its DeckOnly guard). Deviation (security review): a namespace the library
offers is listed once, as the library's, and a builtin namespace claimed by another id is never
offered (R15), so a received deck cannot pass for a trusted pack. The A2UI fixture needs no
regeneration (its deck has no removed deck-only pack); tests prove the library is unwritten;
regression `issue_0937_deck_only_pack_offered_again.rs`.

## Task 7: t-960 real-LibreOffice tests ignored, not passed

`#[ignore = "real LibreOffice, it opens in the Dock: just test-soffice"]` on the real-soffice tests
of `oku-slides-reference` and `oku-slides-cli`, which fail without LibreOffice; `just test-soffice`;
rules and ARCHITECTURE updated.

## Task 8: review and gates

Multi-angle review (`/review`), fixes, `just ci`, `git fetch` and merge `main` again, one squash
commit citing every ticket, push, close the tickets and sub-tasks.
