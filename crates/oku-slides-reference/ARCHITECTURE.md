# oku-slides-reference

Reference rendering of an imported presentation: a witness independent of Oku renders the `.pptx`
to PDF, rasterized and compared with the Oku rendering (hi-fi gap, `oku_slides_import::metrics`).

## API

- `ReferenceRenderer` (trait): `witness()` (name and version written to the report),
  `render(pptx, fonts, slides: Option<SlideSpan>) -> ReferencePdf`. `Office2Pdf`: `WITNESS`
  its name, pinned with `=0.8.1` (a test checks it); it drops embedded fonts (see Isolation).
- `Soffice` (t-885): LibreOffice, `soffice --headless --convert-to pdf` without a shell, in a
  temp dir per render (isolated profile `-env:UserInstallation`, fonts in its `user/fonts`),
  `ExportHiddenSlides` and `PageRange`; `--version` read once (`LibreOffice 26.8.1.1`). One deadline
  per render; past it, its process group is killed (`process.rs`, `group.rs`). `find_soffice`: the
  first existing candidate, as its real path; on macOS a script is refused (`Render`).
- `ReferencePdf`: `page_count`, `raster(i, width) -> Raster` (hayro, on white, ≤ `MAX_RASTER_SIDE`);
  `Raster`: opaque 8-bit RGBA, `crop`, `scaled`, `from_png`, `to_png` (PNGs on the worker pipe).
  Feature `test-support`: blank PDF, fake `soffice`. Real-LibreOffice tests: `OKU_SLIDES_TEST_SOFFICE=1`.
- `ReferenceError` opaque, `kind()` (`Render`, `Timeout`, `Pdf`, `NoPage`, `Size`, `Png`): never
  fatal for the import, whose report says the reference is missing.

## Isolation

The witness reads an untrusted file in its own render worker (`Reference`, `ReferencePng`), 20 slides
at a time, bounded in time and memory and, on macOS, sandboxed to write only in its dir (t-912).
office2pdf writes an embedded font under its unchecked `typeface` (t-898): the `.pptx` is rewritten
without `ppt/fonts/` or any `.fntdata`, reread with office2pdf's zip 0.6, refused if it still sees one.
LibreOffice, a child of that worker outside its memory bound, has its own timeout, below the worker's.

## Known limits (`office2pdf`; LibreOffice shrinks `normAutofit` text, t-692 spike)

- It shrinks `normAutofit` text without `fontScale` (PowerPoint keeps 100 %) when it is one
  unbulleted paragraph in a box at most two lines tall or anchored middle or bottom; it draws
  letter-spaced text 2–5 px wider, charts roughly, SmartArt text too small: `metrics::WitnessLimit`
  marks such regions `suspect` (office2pdf only), ranked last and never flagging a slide.
- It shares Typst's shaping and line breaking with Oku (a PowerPoint line-break gap, t-863, does
  not show); the fonts given come first, the system fonts stay its fallback (outside WASM).
- An import behavior copied from it (Apache-2.0, credited) is no longer measured by it: check it
  against LibreOffice or the OOXML. Its DOCX/XLSX readers sit unused; never give it a `.docx` (t-898).

## Calibration (2026-10-16, office2pdf, worst non-suspect region; `score_threshold = 7`)

Faithful: alto ≤ 3.4 (p. 8 bullets 2.81 since t-863), libreoffice and kitchen ≤ 5.8. Flagged: alto
p. 6 (9.38, intro line), p. 7 (15.71, callout), both p. 1 tables (≥ 20), kitchen p. 2 (8.7).
LibreOffice is not calibrated: the same threshold applies, with no suspect region.

## Extension points and dependencies

New witness (PowerPoint): a `ReferenceRenderer` and a worker `ReferenceWitness` variant. Uses `office2pdf`
(no `embedded-fonts`), `zip-office2pdf`, `zip`, `hayro`, `png`, `snafu`, `oku-slides-platform`, `tempfile`, `libc`.
