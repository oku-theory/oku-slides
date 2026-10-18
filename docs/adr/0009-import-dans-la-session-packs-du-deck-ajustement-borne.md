# ADR-0009: Import pipeline in the session, deck-only packs, bounded agent tuning

- **Status:** accepted
- **Date:** 2026-10-17
- **Tasks:** t-692, t-884
- **Spec:** [`2026-10-09-import-pptx-design.md`](../superpowers/specs/2026-10-09-import-pptx-design.md)
  (decisions of the lot 5 plan)
- **Partially supersedes:**
  [ADR-0008](0008-import-representation-intermediaire-et-projecteurs.md) ("rendering-based checks stay
  in the CLI")

## Context

Lot 5 brings the PowerPoint import into the desktop app and lets the agent fine-tune a hi-fi import
against the reference witness. Until then the orchestration (read, project, apply, render, witness,
score) lived in the CLI, and the hi-fi pack and theme were ordinary library versions. Copying the
CLI code into the app would have made two pipelines; writing the generated pack and theme to the
library would fill it with one-off versions; an agent left to judge its own progress could loop on
a slide, chase witness errors or burn its turn on theme edits.

## Decision

- **One pipeline, in `oku_slides_session::import`.** Reading (with a size bound), projection, the
  checked plan applied in one history step, rendering through the render worker, overflow notes,
  the witness and the import state live in the session, which may depend on `oku-slides-import`
  and `oku-slides-reference` (core still never does). The CLI keeps its options, output and report
  file; the app's workspace builds the tab from the same code (`WorkspaceRequest::ImportTab`).
- **Deck-only versions: `PackScope::Deck`.** The hi-fi pack and theme, and the retrofit child pack
  created at the first widget write, carry `scope: "deck"` in their manifest. The library refuses
  them (`DeckOnly`); edits embed new deck-only versions. `PackScope::Library`, the default, is never
  serialized, so every existing pack and theme keeps its content hash.
- **A tuning loop bounded by the tools.** `import_report` and `import_diff` hold the budget, not the
  instructions: tries counted on the scored raster's fingerprint (never Typst's page hash), gain
  measured on a slide's total excess over the threshold, `stalled` after tries without gain, a
  deck-wide budget for theme and pack widget changes, regions the witness is known to draw wrong
  (`suspect`) never flagging a slide. The witness runs once per import, outside the session lock;
  the state lives in the tab's memory only. The whole agent turn is one undo group above the
  import's own step.

## Consequences

- The session crate grows by the import module and two dependencies; the CLI is a thin caller and
  a fix to the pipeline reaches both entry points.
- A deck-only pack cannot be edited in the Workshop (t-902) and a deck-only theme cannot be picked
  again once replaced (t-903); both are limits, not design goals. Both were addressed later: the
  Workshop edits the embedded version (t-902), and the theme menu offers the deck's own themes (t-903).
- The budget rules are session code with unit tests, so the instructions in `import.md` only explain
  them; an agent ignoring them is refused by the tools.
- Calibration (threshold, suspect regions) rests on three fixtures (t-909); the app offers only
  office2pdf as witness (t-905).
