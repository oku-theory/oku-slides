# Deck knowledge base (Open Knowledge Format)

Task t-1721. Each deck carries a knowledge base: a virtual tree of Markdown files in the
[Open Knowledge Format](https://github.com/open-knowledge-format/open-knowledge-format) (OKF,
v0.3). The deck's agent builds it from the user's prompts, documents they hand it and its own
research, and reads it back as context while it builds the slides. A read-only tab shows it:
the tree on the left, one sub-tab per open file, Markdown rendered with its images.

## Intent

The user wants the context behind a deck (market figures, a client's brief, a report's key
points, research notes) to live **in the deck**, travel with it (inside the PDF, like the rest
of the sources) and stay usable by the agent across conversations, instead of being re-pasted
into the chat or lost when the conversation ends. The knowledge is working context for the
agent; it is never shown in the slides themselves.

## Goals

- A per-deck OKF bundle, stored in the deck's Loro replica: undoable, merged, in history,
  saved in the PDF's sources.
- The deck agent writes and reads it through MCP tools; one agent turn's slide and knowledge
  edits are one undo step (`undo_turn` reverts both).
- The user hands documents to the agent in the **deck chat** (PDF, DOCX, PPTX, XLSX/XLS/ODS,
  Markdown, plain text), as they paste images today; the agent imports them as extracted Markdown with images.
- The agent receives the bundle's index every turn it changes and reads files on demand.
- A read-only knowledge tab, linked to its deck tab.
- Knowledge received from someone else's deck reaches the agent only once the user approved it.
- Exporting the sources (`oku-slides open … --out`) writes a conformant OKF bundle.

## Non-goals

- Editing knowledge by hand in the tab (the agent is the only writer; the user asks it).
- Showing knowledge in slides or presenter notes.
- Keeping the original binary documents (only their extracted Markdown and images).
- OCR of scanned PDFs; legacy binary `.ppt` (refused, asking for a `.pptx`); HTML; Excel charts.
- A knowledge base shared across decks, or importing an existing OKF folder from disk.
- Web research by the local fallback agent: research relies on the selected ACP agent's own
  tools (Claude Code, Codex…); the local agent works from prompts and documents only.
- Running OKF Attested Computations (fields are kept and shown, never executed).

## Anti-goals (a failure even if it works)

- The agent reading received knowledge the user never approved (prompt injection path).
- Text lost on a save/open or merge round trip.
- Knowledge making decks heavy or slow to open: it is bounded and charged to the sources budget.
- A silent fallback: an unreadable file, a failed extraction or a truncated index is reported,
  never hidden.

## Decisions (user, 2026-11-24)

1. **Purpose: agent context for building slides**, not audience content.
2. **The agent produces it** from prompts, documents and research.
3. **Documents are kept as extracted Markdown plus their images** (option C), under
   `references/`; the original binary is dropped after extraction. Rejected: distilled notes
   only, originals discarded (A: the agent can never re-read a source); originals kept in the
   deck (B: heavy, and the agent only reads text anyway).
4. **The tab is read-only**: rendered Markdown and images.
5. **Documents and research requests go through the deck chat**; the knowledge tab has no
   agent of its own. Rejected: a dedicated agent in the knowledge tab (cuts the knowledge off
   from the conversation building the slides).
6. **Stored in the deck's Loro replica (approach 1).** Rejected: a second Loro document for
   knowledge (two undo stacks and merges to coordinate; `undo_turn` across both); plain files
   in the add-only sources zip (no undo, whole-file merge, dead versions kept).

## Design

### Model (`oku-slides-core`)

- `Deck.knowledge`: a map `id → KnowledgeFile { path, text }`; directories are virtual,
  derived from paths. `text` is the whole file, frontmatter included, so the stored form is
  exactly the OKF file. *(my call: simplest form that round-trips byte for byte.)*
- Replica: a `knowledge` `LoroMap` created up front, `id → LoroMap { path (LWW), text
  (LoroText, Prose) }`, so concurrent edits to one file merge per character (per line past
  64 Ki chars; a diff past its time budget replaces the whole text) and a move is a path
  change. Removal removes the file's map entry: a delete wins over a concurrent edit, and
  undo restores it. Two files ending on the same path after a merge are repaired on read
  (the lowest id keeps the path, the others take `{stem}-{n}.md`, an `Issue`); the repair
  persists on the next apply. `meta.format` goes to 8 and
  `SOURCES_VERSION` with it; a format-7 deck reads with an empty knowledge and is upgraded to 8
  by a silent load-time commit (the one exception to core's no-backward-compatibility rule).
- Ops: `knowledge_write { path, text, id? }` (create or replace; a create carries its id,
  minted by the session, so applying an op is deterministic), `knowledge_edit { path, old,
  new }` (one exact, unique replacement, overlapping matches counted: large files are not
  resent whole), `knowledge_move { from,
  to }`, `knowledge_delete { path }`. Applied to `Deck`, never to the replica (existing rule).
  Hidden from every caller (left out of the op schema, refused by the session) until lot 3's
  approval lands. Core holds hard caps (files, bytes per file, total bytes, frontmatter size,
  YAML depth); `[knowledge]` config may only lower them.
- Paths: normalized, bundle-absolute (`/a/b.md`), segments `[a-z0-9._-]`, no `..`, no dot-files,
  depth and length bounded; only `.md` files are written by ops (images are assets, below).
  `index.md` and `log.md` are allowed with their OKF rules (no frontmatter except the root
  `index.md`'s `okf_version`, `title`, `description`, `license`).
- Validation on write (refused with a Fluent error, never repaired): frontmatter parses as a
  YAML mapping; a concept file has a non-empty `type` (stricter than OKF v0.3, which tolerates
  type-less files: we are a producer); timestamps carry an offset. Unknown keys are kept.
  Our producer keys use the `x-oku-slides-*` prefix.
- Provenance stamped by the host on every agent write: `generated: { by:
  oku-slides/<version>, at: <now UTC> }`; the agent's own model name goes in
  `x-oku-slides-agent` when known. *(my call)*
- Images: stored in the deck's existing content-addressed assets (`asset_id`), shown in the
  bundle as `/references/assets/<hex>.<ext>`; `knowledge_asset_refs` lists those Markdown
  image links. Assets are add-only: a knowledge image is never dropped, even with its last
  citing file.
- Limits in `[knowledge]` (`config.default.toml`): files per deck, bytes per file, total text
  bytes; images count against `[sources] max_sources_mib` like other assets. A write past a
  limit is refused with the limit named.

### Documents in the deck chat (`oku-slides-session`, `oku-slides-agent`)

- `attachments` grows from images to documents: PDF, DOCX, PPTX, XLSX/XLS/ODS, `.md`, `.txt`, sniffed by content,
  stored per tab in memory as images are, bounded by new `[paste] max_document_mib` and
  `max_draft_documents`. Dropping or pasting a file in the deck chat attaches it; the prompt
  lists each by id, name, type and size (not its content).
- `knowledge_import { attachment, path? }` (MCP) extracts it **in the render worker** (untrusted
  bytes, sandboxed, time and memory bounded): a new `ExtractDocument` request returning
  Markdown and images. PDF: text in reading order per page, page breaks as headings, embedded
  images; DOCX: headings, lists, tables, emphasis, links, `media/` images; PPTX (user,
  2026-11-24), through `oku-slides-import`'s reader: one `## Slide N — <title>` section per
  slide with its text, tables, speaker notes and images; spreadsheets (user, 2026-11-24): one
  section per sheet, a Markdown table of cached values (not formulas), capped in rows and
  columns (`[knowledge]`), the cut stated in the file (`… N rows not imported`); `.md`/`.txt`:
  as is (`.md` gets frontmatter added if missing). A PDF with no text layer is refused
  (`knowledge-no-text-layer`), never imported empty.
- Result: `/references/<slug>.md` with `type: Reference`, `title`, `sources: [{ resource:
  <file name>, title }]`, `generated`, then the body; its images go to assets. The agent then
  writes its own concept files linking to it. Extraction libraries are the builder's choice
  (dependency audit, no `std::fs` outside Storage, pure Rust preferred).

### Agent (`oku-slides-mcp`, `oku-slides-session/agent`, `oku-slides-agent`)

- Deck tabs only, a `Knowledge` tool family: `knowledge_list` (paths, `type`, `title`,
  `description`, `status`, staleness; optional directory), `knowledge_read { path }` (text; for
  each embedded image an image block when the agent takes images and within `[paste]
  prompt_*` limits, else listed), `knowledge_search { query }` (plain substring, capped hits
  with path and line), `knowledge_write`, `knowledge_edit`, `knowledge_move`,
  `knowledge_delete`, `knowledge_import`, `open_knowledge` (shows the tab; counts as the turn's
  one tab move). Writes are undoable, inside the turn's undo group, listed in the receipt like
  other edits, never confirmed (they are undoable). Each tool is in the local agent's
  `families_of`.
- Index block: `<oku-slides-knowledge>` with path, `type`, `title`, `description`, `status` per
  approved file, sent on the first turn and again when its digest changes (as
  `<oku-slides-guides>`), framed and sanitized as untrusted text, capped (`[knowledge]
  index_entries`, bytes); past the cap it says how many entries it left out and to use
  `knowledge_list`. An empty base sends a one-line block saying the base is empty, so the agent
  knows the feature exists.
- Instructions (`instructions/deck.md`, English): when to consult and update the base, OKF
  conventions (one concept per file, `type`, `sources` with footnotes, `index.md` per
  directory, `log.md` entry per research session), import documents before quoting them.

### Received knowledge (prompt injection)

- A file reaches the agent (index, read, search) only if its content hash is approved. Writes
  made by this app's agent or extraction approve their result at once; a file arriving by
  opening someone else's deck or by a merge, or changed by one, awaits approval.
  Approvals live in the data folder (`knowledge-approvals.json`, per hash), like guide
  approvals. *(my call: per file, so a merge re-asks only for what changed.)*
- Awaiting files appear in the agent's index by path only, marked awaiting, and the read tools
  refuse them with a localized reason. The knowledge tab shows them with a badge and an
  "Approve" action for one file or all; the deck tab shows a notice when the deck opens with
  unapproved knowledge. Approval is a user click from the interface, never an agent tool.

### Knowledge tab (`ui`, `src-tauri`, workspace)

- A new tab kind `knowledge`, linked to its deck tab (opened from the deck tab's menu, a
  shortcut, or `open_knowledge`); one per deck tab; it closes with its deck tab and restores
  with it. Its title is the deck's with a knowledge marker.
- Left: the tree (directories from paths, `index.md` shown as the directory's page, files
  sorted by title then path, awaiting files badged). Right: a strip of open file sub-tabs
  (reusing the pack item strip), the file's header from its frontmatter (type, status, tags,
  generated by/at, verified tier, stale, sources), then the rendered body.
- Rendering: a CommonMark + GFM tables reader, raw HTML not rendered, links sanitized
  (`encodeLink` rules); a bundle link opens that file in a new sub-tab, an external link goes
  through the app's existing external-link path, a broken link is shown as broken. Images load
  from the deck's assets through the host's guarded asset route (imported assets are
  untrusted bytes).
- Live: the tab follows the deck session's knowledge revision, so it updates while the agent
  writes; an open file deleted by the agent closes its sub-tab with a toast.
- All strings in Fluent (English reference plus the 19 locales).

### Export and CLI

- `oku-slides open deck.pdf --out dir/` also writes `dir/knowledge/`, the bundle as files with
  images under `references/assets/`, plus a root `index.md` with `okf_version: "0.3"` if the
  agent wrote none. `oku-slides check` reports knowledge issues (repairs on read, broken links
  as warnings).

### Settled after the builder check (my calls)

- **Approval key: the content hash alone.** A hash is approved when an agent tool or an
  extraction writes that text, recorded by the session on that write path (MCP ops), never on
  open, merge or receive. Replaying an approved text is the same text the user's own agent
  produced, so it adds no injection path. Undo and redo land on texts approved or not as they
  were: undoing back to a received, unapproved version leaves it awaiting.
- **Writes keep the replica's diff rule.** `knowledge_write` and `knowledge_edit` change
  `Deck`; the replica's `LoroText` update by diff gives character-level edits, as for slide
  prose. No op bypasses it.
- **Format compatibility as for every bump.** A format-8 deck is refused by a format-7 app
  (existing check on import and merge); nothing is ever dropped on save.
- **Reading is lenient, writing strict.** A received or merged file that fails write
  validation (no frontmatter, no `type`, bad timestamp) loads as is, with an `Issue`, as OKF
  consumers must not reject; only agent writes are refused. Extraction adds frontmatter.

### Plan decisions (lot 1 builder, recorded from the plan)

- **Bounded text diff for all prose.** `sync_text` diffs per character up to 64 Ki chars,
  per line past that, each diff under a 100 ms budget, and replaces the whole text past it:
  this holds for every prose field, slides included, not only knowledge. A step down loses
  merge granularity, never content.
- **Hiding covers every path.** Until lot 3 the knowledge ops are left out of the op schema,
  of the MCP tool schemas and of the preview paths, and the session refuses them from every
  caller.
- **Over-limit bundles are kept (R10).** A read or merge past a ceiling keeps every readable
  file and reports one `knowledge_over_limit` issue per ceiling passed (first file, count);
  only ops refuse to grow the bundle. Only entries that do not read as a file (bad id, not a
  map) are dropped, with an issue. `canonicalize` runs the structural repair only (ids,
  paths, collisions); the text checks and ceilings are reported on read.
- **Over-limit recovery is by delete only.** A bundle past `MAX_TOTAL_BYTES` after a merge
  refuses every write and edit that leaves it past the ceiling, even one that shrinks it, so
  in practice only deletes bring it back under. Lot 3's UI must say so.
- **Overlapping edit matches are ambiguous.** `knowledge_edit` counts overlapping
  occurrences: `aa` in `aaa` is `EditAmbiguous`.
- **Paths are not stable identifiers across merges.** A collision repair renames a file, so
  lot 3 cites files by id, never by path.
- **Cited images are found with a CommonMark parser** (same rules as the tab: images in code
  blocks are not cited), awaiting files included, so an image never disappears from under a
  file. Knowledge images share `[sources] max_sources_mib`; a write or import past it is
  refused naming that budget, before any change.
- **Document attachments:** a new host command generalizes `paste_image` to
  `attach` (images and documents); the type is sniffed in process from magic bytes only
  (`media::sniff`, as for images today); all parsing happens in the worker.
- **Images in `knowledge_read` results:** MCP image content, `prompt_image_mib` per image and
  `prompt_total_mib` per call; the local agent gets them listed, not sent.
- **The knowledge tab** follows the deck-only pack tab's precedent (`deck_pack_link`);
  `open_knowledge` uses the turn's one tab move, refused with the existing tab-gate reason.
- **Timestamps** with the `time` crate (already in core), RFC 3339 in UTC.

### Plan decisions (lot 2 builder, recorded from the plan)

- **Read tools before approval writes.** Lot 2 ships `knowledge_list`, `knowledge_read`,
  `knowledge_search` and the index, gated by `knowledge-approvals.json` read only (a JSON
  array of `sha256:<hex>` content hashes); lot 3 adds approving. Until then every file
  awaits and reads refuse it.
- **Status and staleness** come from the frontmatter: `status` when it is text, `stale` only
  for the plain scalar `true`.
- **Images in `knowledge_read`** are always attached when they fit the `[paste] prompt_*`
  limits (the server cannot know its caller's image support) and always listed in the text.
- **A file kept past `MAX_FILE_BYTES`** is returned cut at that size, the cut stated.
- **Read ceiling for forged replicas:** materializing stops at 4× each ceiling, the rest
  dropped and reported; R10 holds for any bundle ops and merges of valid decks can build.
- **`[knowledge]`:** `index_entries` (200, ceiling 1024), `index_kib` (16, ceiling 64),
  `search_hits` (50, ceiling 200); a higher value uses the ceiling, with a warning.
- The local agent routes the three tools through a `knowledge` family.

## Testing

- Core: op validation (paths, frontmatter, limits), replica round trip, merge of concurrent
  edits to one file, path collision repair, format-7 read, cited image assets kept (assets are add-only).
- Session/MCP: turn undo reverts slides and knowledge together; unapproved files withheld from
  index/read/search; approval on local write; index digest re-send; caps.
- Worker: extraction fixtures (text PDF, scanned PDF refused, DOCX with table and image, PPTX with
  notes and image, XLSX past the row cap, `.ppt` refused),
  under `fixtures/` (LFS for PDFs), timeout and memory bounds.
- UI: tree, sub-tabs, frontmatter header, link and image rendering, awaiting badge (render
  tests); one e2e: attach a PDF in the deck chat with the fake agent importing it, open the
  knowledge tab, read the extracted file.
- Localized errors tested on their keys plus a non-English locale.

## Left to the builder

- Extraction crates for PDF, DOCX and spreadsheets (e.g. `calamine`), and the YAML parser (no deprecated `serde_yaml`),
  through the dependency audit.
- Whether the knowledge tab holds its own `Slot` or is a view over its deck tab's session.
- Exact limit values, sub-tab persistence across restarts, tree sort details.
- Splitting the work into lots (suggested: core + replica; MCP tools + index; documents and
  extraction; approval; tab; export).
