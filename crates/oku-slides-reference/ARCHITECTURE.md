# oku-slides-reference

Reference rendering of an imported presentation: a witness independent of Oku renders the
`.pptx` to PDF, whose pages are rasterized and compared with the Oku rendering (hi-fi visual
gap, `oku_slides_import::metrics`, t-692 lot 4).

## API

- `ReferenceRenderer` (trait): `witness()` (name and version written to the report),
  `render(pptx, fonts, slides: Option<SlideSpan>) -> ReferencePdf`. `Office2Pdf`: `WITNESS`
  its name, pinned with `=0.8.1` (a test checks it); it drops embedded fonts (see Isolation).
- `Soffice` (t-885): LibreOffice, `soffice --headless --convert-to pdf` without a shell, in a
  temp dir per render (isolated profile `-env:UserInstallation`, fonts in its `user/fonts`),
  `ExportHiddenSlides` and `PageRange`; `--version` read once (`LibreOffice 26.8.1.1`). One
  deadline per render; past it, its process group is killed (`process.rs`, `group.rs`).
  `find_soffice`: the first existing candidate, as its real path; on macOS a script is refused (`Render`).
- `ReferencePdf`: `page_count`, `raster(i, width) -> Raster` (hayro, on white, ≤ `MAX_RASTER_SIDE`).
- `Raster`: opaque 8-bit RGBA; `crop`, `from_png`, `to_png` (the worker pipe carries PNGs).
  Feature `test-support`: blank PDF, fake `soffice`. Real-LibreOffice tests: `OKU_SLIDES_TEST_SOFFICE=1`.
- `ReferenceError` opaque, `kind() -> ReferenceErrorKind` (`Render`, `Timeout`, `Pdf`,
  `NoPage`, `Size`, `Png`): never fatal for the import, whose report says the reference is missing.

## Isolation

The witness reads an untrusted file: it runs in its own render worker, bounded in time and memory
and, on macOS, sandboxed to write only in its own dir (t-912). 20 slides at a time. office2pdf writes an embedded font under its unchecked
`typeface` (t-898): the `.pptx` is rewritten without `ppt/fonts/` or any `.fntdata`, then reread
with office2pdf's own zip 0.6, refused if it still sees one. LibreOffice runs as a child of that
worker, outside its memory bound: only its own timeout bounds it, shorter than the worker's.

## Known limits (`office2pdf`; LibreOffice shrinks `normAutofit` text, t-692 spike)

- It shrinks `normAutofit` text without `fontScale` (PowerPoint keeps 100 %: the alto p. 2 title,
  every footer), draws letter-spaced titles 2–5 px wider than Oku and LibreOffice, charts roughly
  and SmartArt text too small: such regions often rank first.
- It shares Typst's shaping and line breaking with Oku: a PowerPoint line-break gap (t-863)
  does not show.
- The fonts given come first; the system fonts stay its fallback (outside WASM).
- An import behavior copied from office2pdf (Apache-2.0, credited in the file) is no longer
  measured by it: check it against another witness or against the OOXML.
- Its DOCX and XLSX readers (~48 crates) sit unused in worker and app; never give it a `.docx` (t-898).

## Extension points

- New witness (PowerPoint): a `ReferenceRenderer` and a worker `ReferenceWitness` variant.

## Dependencies

`office2pdf` (without `embedded-fonts`), `hayro`, `png`, `snafu`, `oku-slides-platform`
(`Storage`), `tempfile`, `libc` (unix: `killpg`).
