# crates/oku-slides-reference — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Run a reference witness only through RenderWorker::reference, in a render worker of its own, never in-process. (t-692, t-327)
- Give the witness the app's fonts and the deck's embedded fonts, and render hidden slides. (t-692)

## Pitfalls

- Read a gap on letter-spaced titles, autofit text, charts or SmartArt as a likely witness error before changing the import. (t-692)
- Do not align the import on office2pdf code without checking it against the OOXML or another witness. (t-692)

## Settled decisions

- Measure the hi-fi visual gap against the reference witness, never against fixtures/import/alto-wealth.pdf. (t-692)
