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
- The user hands documents to the agent in the **deck chat** (PDF, DOCX, PPTX, XLSX/ODS,
  Markdown, plain text; XLS is refused like `.ppt`), as they paste images today; the agent imports them as extracted Markdown with images.
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
  Lot 2 hid them from every caller; lot 3 opens them to deck tabs only, each write approved
  as written (Plan decisions, lot 3). Core holds hard caps (files, bytes per file, total bytes, frontmatter size,
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

- `attachments` grows from images to documents: PDF, DOCX, PPTX, XLSX/ODS, `.md`, `.txt` (XLS refused as legacy), sniffed by content,
  stored per tab in memory as images are, bounded by new `[paste] max_document_mib` and
  `max_draft_documents`. Pasting a file, picking it with the draft's file button or, in a
  browser, dropping it in the deck chat attaches it (the app's webview takes OS file drops,
  t-819); the prompt lists each by id, name, type and size (not its content). A sent turn shows
  only its images until lot 5's receipts.
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
- **Hiding covered every path (lifted by lot 3).** Until lot 3 the knowledge ops were left
  out of the op schema, of the MCP tool schemas and of the preview paths, and the session
  refused them from every caller. Lot 3 lists them in every schema; previews still refuse
  them (`KnowledgeOpsInPreview`).
- **Over-limit bundles are kept (R10).** A read or merge past a ceiling keeps every readable
  file and reports one `knowledge_over_limit` issue per ceiling passed (first file, count);
  only ops refuse to grow the bundle. Only entries that do not read as a file (bad id, not a
  map) are dropped, with an issue. `canonicalize` runs the structural repair only (ids,
  paths, collisions); the text checks and ceilings are reported on read.
- **Over-limit recovery is by delete only.** Since lot 3, core refuses every
  `knowledge_write` and `knowledge_edit` while the bundle is past any limit (files, bytes per
  file, total bytes), with `OverLimit`; moves and deletes pass, and only deletes bring it back
  under. Lot 3's UI says so.
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

### Plan decisions (lot 3 builder, recorded from the plan)

- **t-1731 lands first.** `Sources` skips an unchanged bundle on apply (no serialization, no
  Loro sync); render requests carry no bundle.
- **Knowledge generation:** a process-wide counter, taken anew whenever a bundle changes
  (apply, undo, redo, restore, merge); the snapshot summary's cache key.
- **Approvals file: strict read-merge-write.** `approve` holds the set's lock, reads the file
  strictly, writes memory ∪ file ∪ new atomically, then updates memory; nothing new, no
  write. A concurrent writer may lose hashes: fail closed, those texts await again.
- **Approve on write, by id.** Every caller apply (agent tools, `apply`, a UI apply) follows
  each file id through the batch. An id is tainted when its text before the batch is not
  approved; a whole-text write clears the taint, delete keeps it, and an edit or a move of a
  tainted id is refused (`KnowledgeChangeAwaiting`) before core runs (final review: a move
  would carry an awaiting text to a path the user trusts). The ids written or edited are
  authored: their texts in the next deck are approved in `commit`, after the render and
  before the replica write, so a refused render approves nothing (final review). A failed
  approvals write refuses the apply (`KnowledgeApprovalsWrite`, or `KnowledgeApprovalsDamaged`
  when the file cannot be read or parsed); a later commit failure leaves only approved texts
  the caller wrote. Never on open, merge, receive, undo, redo or restore. Approval is per
  text, for every deck, keyed by hash only. The user's own writes join the agent's (amendment
  of "an agent tool or extraction").
- **Over limit: only deleting brings it back** (`OverLimit`, above).
- **Ids are minted with `knowledge_file_id()`** for a write without id on a path free at that
  point of the batch.
- **Knowledge ops fit deck tabs only:** `WrongDocument` in pack, theme and help tabs; history
  is read-only; previews refuse them with `KnowledgeOpsInPreview`, their schemas listing the
  knowledge ops too.
- **Files are cited by id** in this lot's requests and views; paths and titles are labels,
  through `agent::bounds::label` (`label_within 80` for titles).
- **The user approves only a text the dialog showed whole:** the host recomputes the hash and
  refuses a mismatch (`KnowledgeChanged`); a file past `MAX_FILE_BYTES` cannot be approved
  (`KnowledgeTooLargeToApprove`); no "Approve all"; hidden characters shown as `⟨U+XXXX⟩`.
- **Review requests are user-only:** refused to the agent window and outside `Mode::Deck`.
- **Deferred:** provenance stamping (done in lot 4, see below), receipt entries for knowledge
  writes (lot 5's tab); the approvals file is not capped. Until provenance stamping lands
  (lot 4), approve-on-write can launder text injected through a slide or the conversation:
  the agent writes it and that write approves it.
- **No e2e this lot:** no UI writes knowledge yet; render and session tests cover the flow.

### Plan decisions (lot 4 builder, recorded from the plan)

- **Provenance stamp:** approvals keyed by text hash let an agent write approve a received
  file with equal text, so the host stamps every file an *agent* batch authors: concept files
  get `generated: {by, at, id}` (16 hex from getrandom) and `x-oku-slides-agent: "<label>"`,
  `index.md`/`log.md` a last `<!-- oku-slides generated <at> <id> -->` line. The pass runs
  after core's apply and before the commit, through core again (one undo step); a flow-style
  root mapping or `...` is refused (`FlowMapping`). User writes stay unstamped. The label is
  the preset (and model), cleaned by `label_within(_, 80)`, set per turn.
- **Attachments:** detection by magic bytes only, in process (no zip opened there): PDF, zip
  subtype by extension (`.docx`, `.pptx`, `.xlsx`, `.ods`), OLE2 is `Legacy`
  (`knowledge-legacy-format`), UTF-8 without NUL is Markdown or Text; a known extension whose
  magic disagrees is `TypeMismatch`. `[paste] max_document_mib` 32 (ceiling 47),
  `max_draft_documents` 4, `max_kept_mib` 200; documents in deck tabs only; `AttachmentRef`
  is never persisted; `embed_op_assets` resolves image attachments only.
- **XLS is refused like `.ppt`:** a BIFF reader would need calamine, rejected by the
  dependency audit.
- **Extraction:** one dedicated child worker per import (`[knowledge] extract_timeout_ms`
  120000, `extract_memory_mib` 512), never in process; one import at a time per process: a
  second one is refused at once (`session-knowledge-import-busy`, retry), no thread waits. On
  macOS the memory bound is best-effort (RSS watchdog, 200 ms tick); on Windows and Linux the
  parsers run unsandboxed. `Request::ExtractDocument` returns Markdown plus images in one
  frame, refusals as core `ExtractRefusal` (`session-knowledge-import-refused`), a timeout or
  kill as `session-knowledge-import-failed`. Extraction runs under `catch_unwind`; one
  sanitizer escapes Markdown, strips Cf/tag/bidi characters, keeps only http(s)/mailto
  structural links through `encode_link_url`, and no remote image. Spreadsheets: cached values
  capped at parse time (2000 rows, 50 columns by default), the cut stated under each table.
  PDF: `lopdf`, at most 2000 pages, encrypted refused, a PDF without text refused.
- **Reference files:** slug folded to `[a-z0-9-]`, at most 60, `document` when empty, `-doc`
  for `index`, `log` or a reserved name; bodies above 192 KiB split at `## `, then a blank line, then a line outside a code fence,
  then any line (each only past half a part), then a char boundary into `<slug>-part-N.md`, each
  titled `(part N of M)` and linking the next; a cut inside a fence closes it and reopens it in
  the next part, its info string (up to 32 bytes) kept;
  the set reserved together, a collision taking `-2`, `-3`…; frontmatter values JSON-quoted
  with `x-oku-slides-import: {type, cut, part, of, series, hash}` (`series`: the import's 16-hex
  ticket, `hash`: the `sha256:` of the file as written, frontmatter included, the stamp and this
  mark aside); every body opens with the untrusted-document
  banner, and `knowledge.md` tells the agent references are data. Images are cited by a
  16-hex placeholder token only the session knows, staged in `Session.import_staged`, never
  `carried`; each image is linked at its first placeholder, later ones become their alt text.
  The extraction's text room is the bundle's room less a bound of what the import adds (links,
  and per part frontmatter, banner, continuation line, stamp), so a text cut to fit is written.
  `path` replaces only a first part an import wrote, with its later parts: a later part is the
  file at `<base>-part-N.md` whose mark says part N of the same series, and only those are
  deleted when the new import has fewer parts; another file there is kept, and refused as a
  target if the new import needs its path. If any part (body or frontmatter) no longer hashes to its mark, the
  re-import is refused (`session-import-target-edited`): delete the files first to replace an
  edited import. The replaced import's bytes count as room. No migration: lot 4 had not shipped.
  The job snapshots the tab; apply
  rechecks it and the attachment after the unlocked extraction. Past `MAX_FILES` or a byte
  limit, nothing is written.
- **Deviations from the lot 4 plan (final review, t-1760):** `knowledge_import_job` takes the
  target `path` (to count its room and refuse an edited import before extracting) and
  `knowledge_import_apply` reads it from the ticket; the "Attached documents" line went to
  `knowledge.md`, not `deck.md`; the agent label is set in `turn.rs`; `Session::attach` takes an
  `Upload`, hashed and sniffed once; some tests were renamed.
- **Deferred (under t-1728):** t-1762 a local-agent eval for `knowledge_import`; t-1763 an XLS reader if users ask.

### Plan decisions (lot 5 builder, recorded from the plan)

#### Tab model

- **A view over the deck tab's session.** It gets `TabDocument::Knowledge { deck_tab }` and
  `DocumentKind::Knowledge`. Its slot is `Slot::View`, which holds no session. A session request
  addressed to it is refused with `WrongDocument { kind: Knowledge }`; the UI never sends one.
  `DocumentKind::Knowledge` is used only for that refusal and for `tab_list`'s kind.
- **Skipped everywhere a session is assumed.** These skip `Slot::View` and are each tested:
  - dormancy: settle, wake, set_aside;
  - `close_all`, compaction, usage;
  - UI-state saves, the same-deck check, the dirty prompts;
  - `state.json`: none is written or read.
- **One per deck tab.** Opening again focuses the existing one. It opens right after its deck
  tab, in the deck tab's window. It may then be moved to another window like any tab.
  `open_knowledge` takes no session lock, only the workspace lock. An owner lookup never
  unwraps: a vanished owner gives `UnknownTab`.
- **Linked tabs.** `linked_tabs` stays deck-pack only, because `pull_linked` unparks a pack,
  and both `detach` and the `close_with_links` flush assume one. A new `knowledge_tab_of(owner)
  -> Option<String>` serves closing:
  - the owner's close path closes it, in any window;
  - `close_all` and window closes treat it as linked.
- **Closing alone never prompts.** It is never dirty and holds no work. The UI `closeFlow`
  reads no snapshot for it.
- **Restore.** A knowledge entry is handled before `restore_tab`, which needs `state.json`.
  - It is restored only when its `deck_tab` was restored as a deck.
  - It is dropped with a `tracing::warn!` when the owner is missing, is not a deck, or already
    has a knowledge tab. The `deck_tab` string is untrusted and only ever looked up.
  - A downgrade to an older binary reads it as an unknown tab document and drops it. This
    holds because `SavedTabInfo` uses a tagged `TabDocument`, which the previous release does
    not read. The previous release reads `path`, `deck_uid` and `help`, and for a knowledge tab
    they are all absent. Task 3 verifies this with a test on the saved JSON.
- **Title.** Its `TabInfo.title` is the owner's title, kept current in `Workspace::note`. The UI
  shows it through Fluent `ui-knowledge-tab-title` (`{ $name } — Knowledge`), with a book icon.
  The agent's `tab_list` shows the raw title and the kind `knowledge`.
- **Agent.** A knowledge tab has no agent. The agent window and voice context follow its deck
  tab (`sessionTab(view, tab)`). There is no chat panel in the knowledge tab.
- **Sub-tabs are not persisted.** On restore, the tab opens on the root `/index.md` if it is
  approved, otherwise on nothing.
- **Moves keep ids.** `knowledge_move` keeps a file's id (core test
  `knowledge_move_against_edit_keeps_both`), so an open sub-tab follows a move.

#### Live updates

- `TabInfo.knowledge: Option<KnowledgeRevision { generation: u64, approvals: usize,
  in_history: bool }>`.
  - It uses `#[serde(default, skip_deserializing)]`, like `dirty_revision`. It is sent to the
    UI and excluded from `SavedTabInfo`.
  - It is set on awake deck tabs only. It is read from the live deck (the stash in History or
    Pack mode), and set to `None` in Help or Theme mode and on dormant tabs.
  - `shown_same` compares it.
  - A change of `knowledge` alone broadcasts but does not call `save_layout`.
  - The approvals count is library-wide, so another deck's approval can move it. This is
    accepted: it only costs one re-read.
- **UI side.**
  - `null` means unknown: the tab keeps what it shows and re-reads nothing.
  - On a new `generation` or `approvals` value (the effect depends on those two numbers), it
    re-reads the tree and the active sub-tab. The other sub-tabs are marked stale and re-read
    when shown.
  - Re-reads coalesce: one in flight plus at most one pending.
  - Stale replies are dropped with one `Latest` per file and one for the tree.
- **File gone.** When an open sub-tab's file id is no longer in the tree, the sub-tab closes
  with the toast `ui-knowledge-file-gone` naming its path. A ref dedupes the toast per id, never
  inside a setState updater. A late page reply for it is ignored.
- **History.** While the owner browses history, the reads are refused (`InHistory`). The tab
  shows the notice `ui-knowledge-in-history` in place of the pane, never an error toast, and
  reads nothing. `KnowledgeTab` takes a required `deckInHistory` prop: when it goes from true
  to false (the deck leaves history), the tab re-reads the tree and the shown file.

#### Reading requests (deck session, user-only like lot 3's review)

- `knowledge_tree` → `KnowledgeTree { files: Vec<KnowledgeNode> }`.
  - Each node is `{ id, path, title, kind, awaiting }`. The path is sent whole (core allows
    only `[a-z0-9._-]` segments); the UI cuts a long one with a CSS ellipsis. The title is
    cleaned with `label_within 80`.
  - An awaiting node carries its path only, with `title` and `kind` set to `None`, like lot 3's
    `knowledge_entry`.
  - The awaiting set comes from the cached `knowledge_scan`; nothing is hashed per call. It
    builds on `knowledge_files()`, the same source as `all_knowledge_entries`.
  - All files are listed, since the core caps them at `MAX_FILES`.
- `knowledge_page { id }` → `KnowledgePage { id, path, awaiting, header, body,
  truncated_bytes, images }`.
  - **Approved file, header parsed:** `header: Some(view)`, and `body` is the text after the
    frontmatter.
  - **Approved file, header not parsed:** `header: None`, and `body` is the whole text.
  - **Awaiting file:** `header: None`, `body` is the whole raw text, and `images` is empty.
    The approval hash covers the whole text, so the user reviews all of it.
  - `body` is cut at `MAX_FILE_BYTES` with `cut`; `truncated_bytes` counts what was cut.
  - `images: Vec<KnowledgeImage { path, asset }>` is core `cited_images(path, text)`: the
    resolved bundle path and the asset id of each cited image. The UI never re-parses
    citations.
  - It builds on lot 3's `knowledge_text`, so the same id lookup and error apply.
- `knowledge_asset { id, asset }` → `KnowledgeAsset { mime, data }`, with `data` in standard
  base64.
  - `id` is the knowledge file id, 32 hex characters. `asset` is `sha256:` followed by 64
    lowercase hex characters; that is the only place `sha256:` applies.
  - Approval and citation are checked on every request against the current text.
  - It is refused with `KnowledgeAssetUnavailable { asset }` when:
    - the file is unknown or awaiting approval;
    - the file does not cite `asset`;
    - the deck holds no such asset;
    - the asset is larger than `MAX_KNOWLEDGE_ASSET_BYTES` (8 MiB);
    - the sniffed MIME is not `image/png`, `image/jpeg`, `image/gif` or `image/webp`. SVG and
      every other type are refused, and the UI shows the alt text instead.
  - Encoding happens after every check.
- All three are refused outside a deck tab and in history (`ensure_knowledge_review`, made
  `pub(crate)`). They are refused to the agent window like lot 3's review requests
  (`reviews_deck_knowledge`, `agent_window_allow.rs`, serve's allowlist).

#### Header

- The core gets `knowledge_header(text) -> Result<KnowledgeHeader, FrontmatterReason>`, on the
  same strict parser. To keep items, `Node::Seq` now holds them. Collection stops past 20 items
  for `tags` and `sources`.
- Fields:
  - `kind`, `title`, `description`, `status`, `stale`: as in `Frontmatter`.
  - `tags`: the scalar items of `tags`, or a lone scalar `tags`.
  - `generated_by`, `generated_at`: from the `generated` mapping.
  - `verified`: a scalar `verified`, otherwise `verified.tier` when it is a scalar.
  - `sources`: one `KnowledgeSource { resource, title, url }` per item. A scalar item becomes
    `resource`. A map item with no scalar `resource`, `title` or `url` is dropped.
- The session builds `KnowledgeHeaderView` from it through one `From`-style function that
  cleans every string with `label_within 200`.
- The UI shows header strings, `url` included, as text only. It never makes a link from them.

#### Rendering

- `KnowledgeMarkdown` uses the parser `commonmark.configure([Table, Strikethrough])`, with no
  task lists and no GFM autolinks.
  - Headings are real `h1`–`h6`, shifted by one so the body never uses `h1` (the file title is
    the pane's `h1`), and clamped at `h6`.
  - Raw HTML (blocks and inline) is shown as literal text.
  - Fenced and indented code goes in `pre > code`, with no highlighting.
  - CommonMark `<…>` autolinks and reference-style links (`[a][r]` with a `LinkReference`
    definition) go through the same `resolveLink` as inline links. A reference definition
    itself renders nothing.
- The shared walk moves from `ui/src/agent/Markdown.tsx` to `ui/src/markdownRender.tsx`. It
  exports `renderMarkdown(tree, text, hooks)`, where the hooks `link`, `image`, `heading` and
  `url` are all mandatory. The walk never emits an `<a>` itself. The agent's `Markdown` passes
  hooks that keep its output byte-identical, and its render tests pass unchanged.
- **Links.** `resolveLink(href, from, paths)` classifies the raw destination once. The value it
  classifies is the value it emits.
  - Lezer does not decode entities or escapes in link destinations, so `resolveLink`
    (`ui/src/knowledge/knowledgeLinks.ts`) decodes CommonMark escapes, numeric references
    and a fixed set of named references itself, before it classifies the link. It finds the
    scheme with ASCII whitespace and control characters removed, and lowercases the part before
    the first `:`; the target itself only drops tabs and newlines and is trimmed.
  - A `:` before the first `/`, `?` or `#` means a scheme. `http`, `https` and `mailto` are
    external; any other scheme is inert.
  - A target starting with `//` or `\` is inert.
  - Any other target is a bundle path. It is resolved against the file's directory, or from
    the root if it starts with `/`, with `?…` and `#…` dropped. It is percent-decoded once, and
    a `%2e`/`%2f` that decodes to `.` or `/` makes it inert. A `..` above the root, a NUL, or a
    `\` makes it inert. The result is `{kind:"bundle", path}` when `paths` holds it, otherwise
    `{kind:"broken", path}`.
- **External links** render as `<a href target="_blank" rel="noopener noreferrer">`. This is
  the app's existing external-link path: `tauri-plugin-opener`'s click hook opens a `_blank`
  http(s)/mailto link in the browser. It needs `opener:allow-open-url`, which no capability
  grants today, so the agent's links are dead too. Task 7 adds it to `default.json` and
  `agent.json`, scoped to `http://*`, `https://*` and `mailto:*`. In serve mode the browser
  opens `_blank` itself. The agent's `Markdown` moves to `rel="noopener noreferrer"` as well.
- **Bundle links** render as a button that opens that file in a new sub-tab, or focuses it.
  **Broken links** render as `span.md-link-broken` with the title
  `ui-knowledge-link-broken`. **Inert links** render as their label in plain text.
- **Images.** The src is resolved with the same bundle-path rules. If `page.images` holds the
  result, it loads through `knowledge_asset` into a `blob:` URL. Anything else, remote images
  included, shows its alt text in `span.knowledge-image-alt`. Nothing remote is ever fetched.
- **The asset cache.** `useKnowledgeAssets(api, page)` keeps `Map<asset, string | "failed">`.
  - Each effect run builds its own map and revokes the URLs it created on cleanup.
  - It drops replies after unmount or cleanup, and catches decode failures as `"failed"`.
  - It is keyed by the shown file's id and its image list, and gets no page while the deck
    browses history, so a refused image loads once history is left. At most 4 fetches run at
    once, and the results land in one render.
- **Awaiting files** are shown as preformatted text, with hidden characters revealed (lot 3's
  `revealHidden`). They are never rendered as Markdown and load no images. A badge
  `ui-knowledge-awaiting-badge` and a "Review…" button open lot 3's `KnowledgeDialog`, which the
  `KnowledgeTab` owns and renders. There is no approve-all (lot 3 ruling). An awaiting node in
  the tree shows its path with bidi and Cf characters removed.
- Every attribute value (title, aria-label) comes from Fluent, never from file text, except
  an external link's title, which shows its destination.

#### Opening

- From the UI:
  - The deck's main menu (`AppMenu`) gets an entry, `ui-knowledge-open`.
  - The tab context menu of a deck tab gets the same entry.
  - The shortcut is ⌘⇧K (Ctrl+Shift+K elsewhere), handled in `tabShortcut` as
    `{ kind: "knowledge" }`. On a non-deck tab it does nothing.
  - All of these send `WorkspaceRequest::OpenKnowledge { tab }`.
- From the agent, `open_knowledge` (no arguments, deck tabs only, `Kinds::DECK`, Knowledge
  family, in `knowledge_tools.rs`) works like `help`:
  - It is the turn's one tab move (`TabTarget::Knowledge`), refused with the existing tab-gate
    reason.
  - It returns `{ok, tab, shown}`.
  - It is shown only if the deck tab is the one shown in its window.
- The sub-tab strip reuses the pack item strip's markup and the shared keyboard helper
  `stripKeyNav`, which is extracted from `WidgetTab.tsx`'s local `onTabKey` and shared by both.
  It is not copied.

#### Deferred

- Receipt entries for knowledge writes, and documents shown in sent turns (lot 3 and lot 4
  deferrals): a new sub-task under t-1728, created in Task 9, named "Knowledge: receipts for
  writes, documents in sent turns".
- Lot 6: export and CLI.

#### Corrections after the task reviews

These replace what the plan said; the text above already reads this way.

- **Entities in links.** Lezer does not decode entities or escapes in link destinations.
  `resolveLink` (`ui/src/knowledge/knowledgeLinks.ts`) decodes CommonMark escapes, numeric
  references and a fixed set of named references itself before it classifies a link. Only a
  literal `http`, `https` or `mailto` scheme becomes an href.
- **Ids.** The knowledge file `id` is 32 hex characters. The asset id is `sha256:` followed by
  64 lowercase hex characters, and that is the only place `sha256:` applies.
- **Opener.** The opener permission is scoped to `http`, `https` and `mailto` in the main
  windows and to `https` and `mailto` in the agent window; `build.rs` requires each file's
  exact scope, once.
- **e2e (t-1761).** `ui/e2e/460-knowledge-tab.cjs` attaches a PDF built in memory
  (`ui/e2e/lib/pdf.cjs`, no binary fixture; the extraction test
  `pdf_hand_written_page_extracts_its_text` writes the same bytes), has the fake agent run
  `knowledge_import` then `open_knowledge`, reads the file in the tab, and proves that closing
  the deck tab closes its knowledge tab without a question.

#### Decisions after the final review

- **Sub-tabs survive a tab switch.** A tab switch remounts the view, so each knowledge tab's
  open and shown files live in a module store (`ui/src/knowledge/subTabs.ts`) under its tab id.
  The workspace prunes the entries of tabs no longer in the view.
- **History comes from the tab list.** `KnowledgeRevision` carries `in_history`, set from the
  owner's mode, and the view takes `deckInHistory` from it. No snapshot is read: a snapshot
  hands out the session's one-shot notice, which belongs to the deck tab. A refusal that lands
  before the flag reaches the view is told once.
- **Last tab close.** Only linked deck-pack tabs that would empty the workspace keep a close
  (`Closed::LastWindow`). A lone deck tab whose only companion is its knowledge tab closes both,
  as a plain last tab does: a view holds no work.
- **Nesting and render failures.** The shared Markdown walk renders nodes past depth 64 as their
  source text, and the knowledge body and the agent's Markdown sit in an error boundary that
  shows the raw text under `ui-markdown-failed`. A file whose frontmatter does not parse shows its raw text under
  `ui-knowledge-header-invalid` (`KnowledgePage.header_invalid`).
- **Keyboard close.** Delete or Backspace on the sub-tab strip closes the shown file and moves
  the focus to the sub-tab shown next.
- **Link targets.** `resolveLink` strips whitespace and controls only to find the scheme; the
  target keeps its inner spaces, drops tabs and newlines, and is trimmed, then `encodeLink`
  encodes it.
- **Agent window web links.** The agent window's opener refuses `http:`, so its Markdown shows
  an `http:` link as a broken link (`span.md-link-broken`, title `ui-markdown-link-blocked`)
  rather than a dead click; the main windows still open it.

## Testing

- Core: op validation (paths, frontmatter, limits), replica round trip, merge of concurrent
  edits to one file, path collision repair, format-7 read, cited image assets kept (assets are add-only).
- Session/MCP: turn undo reverts slides and knowledge together; unapproved files withheld from
  index/read/search; approval on local write; index digest re-send; caps.
- Worker: extraction fixtures (text PDF, scanned PDF refused, DOCX with table and image, PPTX with
  notes and image, XLSX past the row cap, `.ppt` refused),
  under `fixtures/` (LFS for PDFs), timeout and memory bounds.
- UI: tree, sub-tabs, frontmatter header, link and image rendering, awaiting badge (render
  tests); one e2e (lot 5, t-1761): attach a PDF in the deck chat with the fake agent importing it, open the
  knowledge tab, read the extracted file.
- Localized errors tested on their keys plus a non-English locale.

## Left to the builder

- Extraction crates for PDF, DOCX and spreadsheets (e.g. `calamine`), and the YAML parser (no deprecated `serde_yaml`),
  through the dependency audit.
- Whether the knowledge tab holds its own `Slot` or is a view over its deck tab's session.
- Exact limit values, sub-tab persistence across restarts, tree sort details.
- Splitting the work into lots (suggested: core + replica; MCP tools + index; documents and
  extraction; approval; tab; export).
