# crates/oku-slides-extract — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Keep extraction pure: no filesystem, clock, network or process; the session runs it only in its worker. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1752)
- Pass every extracted string through the markdown module (escape, sanitize_markdown or a fence) before it reaches the output; never write document text raw. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1752)
- Keep only http, https and mailto links, encoded through encode_link_url; any other link becomes its text. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1752)
- State every cut both in the Markdown (cut_line) and in Extracted.cuts; never drop content silently. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1752)
- Read zips only through oku_slides_import::bounded and XML only through its bounded parser; never open another zip reader. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1752)
- Bound spreadsheet repeats by the row and column caps and one text budget shared by all sheets of a document before storing a cell, counting the rest saturating. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1752)
- Skip hidden and veryHidden sheets and say so in the output; never read their cells. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1752)
- Check an image against max_images, its header's pixels and max_image_bytes before keeping it, and list each one left out by reason. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1753)
- Walk document XML with explicit stacks capped at depth 64, never recursion, and state the cut. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1753)
- Run every extraction under catch_unwind on its own 8 MiB-stack thread, a panic refused as Damaged; never set panic = "abort" in a profile. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1754)
- Read PDF text through `pdf_content`, never lopdf's `extract_text`, which skips form `XObject`s; state every form, inline image or drawn name not read. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1754)
- Return worker faults (thread spawn) as `ExtractError`, never as an `ExtractRefusal` of the document. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1754)
- Size a PDF image from its dictionary with checked_mul against the pixel and part limits before decoding, and decode every PDF stream only through lopdf's limited decompression. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1754)

## Testing

- Build test packages on the fly from tests/common or hand-written fixtures under fixtures/knowledge/, never committed binaries. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1752)
- Give each bomb test (zip, repeat, far corner) an output size bound and a behavioral assert (refusal, cut, read budget), never a wall-clock bound: the nextest timeout catches a hang. (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1752, t-1760)
