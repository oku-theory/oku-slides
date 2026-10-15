# oku-slides-reference

Reference rendering of an imported presentation: a witness independent of Oku renders the
`.pptx` to PDF, whose pages are rasterized and compared with the Oku rendering (hi-fi visual
gap, `oku_slides_import::metrics`, t-692 lot 4).

## API

- `ReferenceRenderer` (trait): `witness()` (name and version written to the report),
  `render(pptx, fonts, slides: Option<SlideSpan>) -> ReferencePdf`. `Office2Pdf` is the only
  witness shipped; `WITNESS` its name, pinned with the `=0.8.1` requirement (a test checks it).
  It strips `ppt/fonts/` from the `.pptx` (office2pdf would write them to the temp dir).
- `ReferencePdf`: `page_count`, `raster(i, width) -> Raster` through
  `hayro`, on white, sides at most `MAX_RASTER_SIDE`.
- `Raster`: opaque 8-bit RGBA; `crop`, `from_png`, `to_png` (the worker pipe carries PNGs).
- `ReferenceError` opaque, `kind() -> ReferenceErrorKind` (`Render`, `Pdf`, `NoPage`, `Size`,
  `Png`): never fatal for the import, whose report says the reference is missing.

## Isolation

The witness reads an untrusted file without bounds of its own: it runs in the render worker
(`oku-slides-render-worker`, `Reference` and `ReferencePng` requests), bounded in time and memory;
the CLI gives it a worker of its own and asks 20 slides at a time.

## Known limits (`office2pdf` witness)

- It shrinks `normAutofit` text without `fontScale` (PowerPoint keeps 100 %: the alto p. 2 title,
  every footer), draws letter-spaced titles 2–5 px wider than Oku and LibreOffice, charts roughly
  and SmartArt text too small: such regions often rank first.
- It shares Typst's shaping and line breaking with Oku: a PowerPoint line-break gap (t-863)
  does not show.
- The fonts given come first; the system fonts stay its fallback (outside WASM).
- An import behavior copied from office2pdf (Apache-2.0, credited in the file) is no longer
  measured by it: check it against another witness or against the OOXML.
- It also pulls the crate's DOCX and XLSX readers (`docx-rs`, `umya-spreadsheet`), never called,
  into the render worker and so the desktop app (~48 crates; lot 5 needs the witness there).

## Extension points

- New witness (LibreOffice, PowerPoint): an implementation of `ReferenceRenderer`.

## Dependencies

`office2pdf` (without `embedded-fonts`), `hayro`, `png`, `snafu`.
