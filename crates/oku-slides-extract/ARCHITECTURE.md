# oku-slides-extract

Turns an imported knowledge document into sanitized, bounded Markdown (plan `…-okf-lot4-documents`).
## API

- `extract(bytes, DocumentType, &ExtractLimits, &ReadLimits, token) -> Result<Result<Extracted,
  ExtractRefusal>, ExtractError>`: on an 8 MiB-stack thread (`ExtractError::Spawn` if it cannot start),
  a panic caught as `Damaged`. Markdown, text, XLSX, ODS, DOCX, PPTX, PDF.
- `Extracted`: `title` (raw, capped at `MAX_TITLE_CHARS`; the caller runs `label_within`), `markdown`
  (at most `max_text_bytes`), `images` (`ExtractedImage`: mime among `RASTERS`, bytes; image `i` cited as
  `![alt](oku-image-<token>-<i>)`), `cuts` (one `Cut` per kind). Contract types (`DocumentType`,
  `ExtractLimits`, `Cut`, `ExtractRefusal`) live in `oku_slides_core::knowledge`.

## Modules

- `markdown`: the one sanitizer. `clean` (CRLF, controls, `Cf`, bidi, tags), `escape_inline` (public), `escape_text`,
  `line_start` (specials; indentation dropped, block marks escaped at line starts), `escape_cell`, `link_url`, `table`,
  `fence_for`, `cut_line` (cut and `Bound`), `skipped_line`, `fit` (cut at a line end, fence closed, cut
  stated), `sanitize_markdown` (raw HTML, comments, definitions removed; fenced if it never settles).
- `text`: Markdown (frontmatter dropped, then sanitized) and plain text (fenced when a line could open
  a fence or frontmatter, escaped otherwise); invalid UTF-8 is a `TypeMismatch`.
- `archive`: zips through `oku_slides_import::bounded::BoundedZip` and its bounded XML parser; errors
  mapped to refusals (CFB: `Encrypted` or `Legacy`; not a zip: `TypeMismatch`; over a limit: `TooLarge`;
  else `Damaged`); package relationships, external hyperlinks, the main part.
- `sheet`: `Row`, `Grid` keep cells under the caps, count the rest; `Book` shares one budget. `xlsx`:
  officeDocument workbook, sheets in order, hidden ones skipped, shared strings (no phonetic runs),
  cached formula values, dates as serials. `ods`: `mimetype` first and exact, encryption refused,
  hidden tables from their style, repeats through `sheet`, typed values before display text.
- `doc`: shared by DOCX, PPTX, PDF. `Inline` (runs, capped; line starts escaped once assembled), `Doc`
  (one text budget, then only counted, images unread). `images`: count, header pixels, then bytes checked
  (equal bytes kept once); the rest listed by reason; an image whose placeholder `fit` cuts is listed.
- `docx`: `w:body` walked with explicit stacks, cut past depth 64; heading styles, `numPr` lists, tables,
  links, images, text boxes (after their paragraph), alternate content's choice. Not read, stated: hidden
  text, deletions, comments, headers and footers, foot- and endnotes.
- `pptx`: `read_pptx` decompresses pictures before `max_images` is checked, bounded by `ReadLimits`;
  `## Slide N — title`, items in drawing order, `### Notes`. Each warning matched: nesting a depth cut,
  the rest stated; past its warning cap, slide pictures are read again to list the refused ones.
- `pdf`: lopdf (object streams within `max_part_bytes`); encrypted refused; page tree walked once, a loop
  `Damaged`; 2000 pages, text budget or `max_total_bytes`, then a stated cut; `## Page N` (no text said; none:
  `TooLarge`, `Damaged`, `Unsupported` or `NoTextLayer`); DCT (RGB, gray, ICC N 1/3), 8-bit Flate; masks, CMYK stated.
- `pdf_content`: a page's operators and text within `max_part_bytes / 8` and the total left (`ToUnicode`
  checked as it grows), forms by `Do` on a stack (depth 8, once per page); `Cache`: fonts by id (maps charged), a form
  with own resources read once per document; inline images, unread forms, broken names stated.

## Data flow, dependencies
Bytes under `max_input_bytes` are read by the format module into a body `finish` fits to
`max_text_bytes`; no files, clock or network (the session runs it in a bounded worker). A new format:
a core `DocumentType`, its module, `extract` arm, fixture; a cut: `CutKind`, `cut_line`. Depends on
`oku-slides-core` (`media` sniffing), `oku-slides-import` (bounded zip, XML, `ReadLimits`, `read_pptx`),
`lopdf`, `png`, `pulldown-cmark`, `roxmltree`, `snafu`; tests: `zip`, `insta`, `fixtures/knowledge/`.
