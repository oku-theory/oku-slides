# crates/oku-slides-reference — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Run a reference witness only through RenderWorker::reference, in a render worker of its own, never in-process. (t-692, t-327)
- Give the witness the app's fonts and the deck's embedded fonts, and render hidden slides. (t-692)
- Run soffice without a shell on an isolated temp profile, and kill its process group past its timeout. (t-885)
- Keep the soffice timeout below its render worker's, so soffice dies before the worker could orphan it. (t-885)
- Strip every font part from the .pptx and recheck it with zip-office2pdf before office2pdf; never give it a .docx. (t-898)

## Pitfalls

- Read a gap on letter-spaced titles, autofit text, charts or SmartArt as a likely witness error before changing the import. (t-692)
- Do not align the import on office2pdf code without checking it against the OOXML or another witness. (t-692)
- Re-audit without_fonts against office2pdf's embedded_fonts.rs and keep zip-office2pdf equal to its zip on every bump. (t-898)

## Settled decisions

- Measure the hi-fi visual gap against the reference witness, never against fixtures/import/alto-wealth.pdf. (t-692)
- Keep office2pdf as the auto witness; LibreOffice runs only on --reference soffice and is optional. (t-885)

## Testing

- Test the soffice witness with a fake script from test_support; run real LibreOffice tests only with OKU_SLIDES_TEST_SOFFICE=1 and soffice installed, as it opens in the Dock. (t-885, t-896)
