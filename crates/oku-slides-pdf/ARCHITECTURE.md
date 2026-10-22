# oku-slides-pdf

The Oku Slides sources blob (a zip holding the deck's Loro replica), stored
in a PDF in its page-piece dictionary (`/PieceInfo/OkuSlides/Private`,
ISO 32000 §14.5). PDF readers do not list it among attachments.

## API

- `embed_sources(pdf, zip)`: writes or replaces the blob (`/Format`,
  `/Version 2`, `/SHA256`, `/LastModified`).
- `extract_sources(pdf) -> Option<Sources>`: reads the blob. `None` when it
  is absent **or** older than version 2 (a `/Version 1` envelope,
  `deck.json`, is ignored as if it did not exist; no conversion).
  `PdfErrorKind::UnsupportedVersion` for a version newer than this binary
  knows.
- `strip_sources(pdf)`: removes the blob ("clean" export of a PDF without
  history).
- `Sources::modified_elsewhere()`: the PDF on disk differs from the hash
  known at opening (triggers the merge on save, in the session).

## Zip format (version 12)

```
manifest.json          { format: "oku-slides-sources", version: 12 }
deck.loro              full snapshot of the replica (oku-slides-core::DeckReplica)
assets/<hex>           as is
fonts/<name>           as is
widgets/<hex>/widgets.json  widget pack version (hex = sha-256 of the canonical content)
widgets/<hex>/widgets/<name>.typ  one per widget file (_commun included)
layouts/<hex>/layouts.json  layout pack version
layouts/<hex>/layouts/<name>.typ  one per layout file (_commun included)
themes/<hex>/theme.json  theme version
```

This crate does not interpret the zip: `oku-slides-core::SourceBundle`
builds and reads it. Here it is only a byte stream to put into the PDF or
take out of it.

## Extension points

- A new blob format: bump `/Version`, keep the old read branch (`Ok(None)`
  or `UnsupportedVersion` depending on the direction), never migrate in
  place.

## Dependencies

`lopdf` (low-level PDF structure), `oku-slides-i18n` (Fluent messages),
`sha2`/`hex` (integrity), `time` (timestamps).
