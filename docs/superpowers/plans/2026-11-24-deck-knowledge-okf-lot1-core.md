# Deck knowledge base — lot 1: core model and replica — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A deck carries an OKF knowledge bundle (`Deck.knowledge`) that core ops edit, the
Loro replica stores and merges per character (format 8), and core scans for cited image
assets; the ops stay hidden from every caller until lot 3 (approval).

**Architecture:** `Deck.knowledge` is a map `file id → { path, text }`. Four new `Op`
variants edit it through `apply_all`, with strict path, frontmatter and size validation in a
new `knowledge` module. A pure `knowledge::normalize` repairs what a peer may send (bad path,
collision) and reports the rest (frontmatter, over cap) on replica read; `canonicalize` runs
its structural part, `knowledge::repair`. The replica keeps
the bundle in a **root container** `knowledge` (beside `deck` and `meta`), reconciled by the
existing `Writer::sync_map` diff in the same commit as the deck.

**Tech Stack:** Rust, Loro 1.17.1 (pinned), snafu, Fluent (`oku-slides-i18n`), `time`,
`saphyr-parser` 0.4 (YAML events, new), `pulldown-cmark` 0.13 (CommonMark, new), speculoos,
rstest, proptest.

**Spec:** `docs/superpowers/specs/2026-11-24-deck-knowledge-okf-design.md` (lots: 1 core +
replica, this plan; 2 MCP tools + index; 3 approval; 4 documents + extraction; 5 tab;
6 export/CLI).

## Plan decisions (builder calls; the spec is updated with this plan)

- **Map, not list.** `Deck.knowledge: BTreeMap<String, KnowledgeFile>` keyed by file id
  serializes to a JSON object, so the replica's `Nested(Ctx)` reconciliation applies.
- **Format 8 with one read exception.** `REPLICA_FORMAT` 7 → 8, `SOURCES_VERSION` 13 → 14,
  `DECK_VERSION` stays 9. Import and merge accept formats 7 and 8; a format-7 doc is upgraded
  to 8 by a silent (`oku-silent`) commit at load and reads with an empty knowledge. Sources
  manifests 13 and 14 load; 14 is written. This is the one exception to core's "no backward
  compatibility" rule, recorded in `crates/oku-slides-core/AGENTS.md`.
- **Ops hidden until lot 3.** `ops_schema()` leaves the four knowledge variants out and the
  session refuses them from every caller (`ensure_caller_ops`), so no agent, MCP client,
  pack or theme tab can write knowledge before approval exists. Core tests apply them
  directly. Lot 3 removes both gates.
- **Hard caps in core.** Core owns the ceilings (file count, bytes per file, total bytes,
  frontmatter bytes, YAML depth), enforced on ops and on read. Lot 2's `[knowledge]` config
  may only lower them.
- **YAML through events.** `saphyr-parser` events let the check refuse anchors, aliases and
  tags, stop after the first document and bound depth; a loaded-value API cannot.
- **Cited knowledge assets are a separate scan.** `asset_refs` is unchanged (it runs twice
  per op in `embed_op_assets`); `knowledge_asset_refs(deck)` serves lots 4 and 6. Assets are
  add-only, so a knowledge image is never pruned.
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

## Global Constraints

- Errors: snafu, `backtrace` in each variant, Fluent-localized; new keys in all 20 locales,
  same order, translated (never English copies); tests assert on the key or variant plus one
  non-English locale.
- No `thiserror` (direct), `chrono`, `serde_yaml`; timestamps with `time`, RFC 3339.
- Unit tests in sibling `x_tests.rs` via `#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`
  declared at the end of each new file; speculoos assertions; regression tests none here.
- Replica rules (`crates/oku-slides-core/src/replica/AGENTS.md`): never translate Ops into
  Loro mutations, always diff the `Deck`; an unchanged apply is write-free; containers are
  created up front (t-188); iterative, bounded reads; the view equals the reread.
- Bound constants live only in core (`knowledge/limits.rs`):
  `MAX_FILES = 1024`, `MAX_FILE_BYTES = 1 << 20`, `MAX_TOTAL_BYTES = 16 << 20`,
  `MAX_FRONTMATTER_BYTES = 16 << 10`, `MAX_YAML_DEPTH = 16`, `PATH_MAX_BYTES = 255`,
  `PATH_MAX_SEGMENTS = 8`.
- Paths: bundle-absolute `/a/b.md`; segments `[a-z0-9._-]+`, not starting or ending with
  `.`, not a Windows reserved name (`con prn aux nul com1-9 lpt1-9`, compared on the part
  before the first `.`); no empty segment; ≤ 8 segments and ≤ 255 bytes; ops write `.md` only.
- Frontmatter (strict on write): a `---` line, YAML, a `---` line (`\n` or `\r\n`, one leading
  BOM tolerated); a mapping with unique keys; no anchor, alias or tag; first document only;
  ≤ 16 KiB and depth ≤ 16. Concept files need a non-empty string `type`; `generated` if
  present is a mapping whose `at` parses as RFC 3339 with an offset (`time` accepts lowercase
  `t`/`z`); unknown keys allowed. `/index.md` may carry only `okf_version`, `title`,
  `description`, `license`; another `index.md` and every `log.md` carry none.
- Text is stored and read byte for byte (no trim, no newline change).
- Each touched `ARCHITECTURE.md` stays ≤ 50 lines and is committed with its code; a rule change
  updates the nearest `AGENTS.md` in the same commit.
- Commits cite t-1721 (or its sub-task); message file under `.superpowers/sdd/knowledge-okf/`,
  `git commit -F`, trailer `Reviewed-by:` after review; no `Co-Authored-By`.
- Fresh temp `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` for any app or e2e run.

## Review Focus

1. **Two peers create a file at the same path concurrently.** Both survive; the higher id is
   renamed `{stem}-2.md` with an `Issue`; the result is the same from either merge direction
   and on reread, and the next apply persists it write-free afterwards. Test in Task 4.
2. **A received replica holds a hostile path** (`/../x.md`, `/.git/config`, 10 000 bytes,
   `/con.md`). Read never yields it: `/recovered/<id>.md` with an `Issue` (id 32 lowercase hex),
   else dropped. Lot 6 writes these paths to disk. Test in Task 4.
3. **CRLF, BOM and empty text.** `---\r\n` frontmatter parses the same; a leading BOM is
   tolerated; the stored text is byte-identical to the input, an empty `/log.md` included,
   through the replica. Tests in Tasks 2 and 4.
4. **A YAML bomb or alias** in a peer's frontmatter is refused on write and reported on read
   without expanding. Test in Task 2.
5. **An image cited only inside a fenced code block, the frontmatter or an `<img>` tag** is not
   a cited asset; one in a file whose frontmatter is invalid still is. Test in Task 5.

---

### Task 1: Model, paths and limits

**Files:**
- Modify: `crates/oku-slides-core/src/model.rs` (`Deck`), `model_tests.rs`
- Create: `crates/oku-slides-core/src/knowledge/{mod.rs, path.rs, path_tests.rs, limits.rs}`
- Modify: `crates/oku-slides-core/src/error/reason.rs` (`PathReason`)
- Modify: `crates/oku-slides-core/src/lib.rs` (module, named re-exports)
- Modify (compile sweep, `knowledge: BTreeMap::new()` or `..`): `canonical_tests.rs:28,436`,
  `oku-slides-session/src/{theme_tab.rs:80, deck_theme_edit.rs:86, pack_mode.rs:121, lib.rs:481}`
- Modify: `crates/oku-slides-render/src/world.rs:245` (strip `knowledge` from `/deck.json`)

**Interfaces:**
- Produces:
  - `pub struct KnowledgeFile { pub path: String, pub text: String }` (Debug, Clone,
    PartialEq, Eq, Serialize, Deserialize, JsonSchema).
  - `Deck.knowledge: BTreeMap<String, KnowledgeFile>`,
    `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`.
  - `pub struct KnowledgePath(String)`: `parse(&str) -> Result<Self, PathReason>`,
    `parse_writable(&str)` (adds `.md`, else `NotMarkdown`; `NotMarkdown` comes only from
    here), `as_str()`, `file_name()`, `stem()`, `dir()`, `is_index()`, `is_root_index()`,
    `is_log()`, `with_suffix(n: u32) -> Result<Self, PathReason>` (`/a/b.md` → `/a/b-2.md`,
    `TooLong` past 255 bytes).
  - `#[non_exhaustive] pub enum PathReason { Empty, NotAbsolute, BadSegment, DotSegment,
    Reserved, TooDeep, TooLong, NotMarkdown }` in `error/reason.rs`, derives `Debug, Clone,
    Copy, PartialEq, Eq`, `Display` in English, Fluent `knowledge-path-*`.
  - `pub fn knowledge_file_id() -> String` (`Uuid::new_v4().simple()`), `pub fn
    is_file_id(&str) -> bool` (32 lowercase hex).
  - `knowledge::limits` constants (Global Constraints).
  - Re-exports from `lib.rs`: `KnowledgeFile, KnowledgePath, PathReason, knowledge_file_id`.
  - Decisions on Deck literals: theme and pack specimens (`theme_tab.rs`, `pack_mode.rs`)
    get an empty knowledge; `deck_theme_edit.rs` and `session/lib.rs` keep the deck's.

- [ ] **Step 1: Failing tests.** `model_tests.rs`: `deck_without_knowledge_serializes_without_key`
  (fixture round-trips, no `knowledge` key), `deck_knowledge_round_trips`. `path_tests.rs`
  (rstest): `/a/b.md` ok; `/` → `Empty`; `a.md` → `NotAbsolute`; `/A.md`, `/a//b.md`, `/a/`,
  `/a./b.md` → `BadSegment`; `/../x.md`, `/.x.md` → `DotSegment`; `/con.md`, `/a/lpt1.txt.md` →
  `Reserved`; 9 segments → `TooDeep`; 256 bytes → `TooLong`; `/a.png` by `parse_writable` →
  `NotMarkdown`, by `parse` ok; `with_suffix(2)` on `/a/b.md` → `/a/b-2.md`; on a 254-byte
  path → `TooLong`; `is_file_id` on 32 hex ok, uppercase or 31 chars false. Proptest
  `parse_never_panics` over arbitrary strings.
  `oku-slides-render`: `deck_json_has_no_knowledge` (a deck with one file renders a
  `/deck.json` with no `knowledge` key).
- [ ] **Step 2:** `cargo nextest run -p oku-slides-core -E 'test(/knowledge|model_tests/)'` →
  FAIL (compile).
- [ ] **Step 3:** Implement. `parse` refuses, never normalizes case or slashes.
- [ ] **Step 4:** Same command → PASS; `cargo check --workspace --tests` clean;
  `cargo nextest run -p oku-slides-core -p oku-slides-render` green (existing snapshots
  unchanged).
- [ ] **Step 5:** Commit `feat(core): deck knowledge model and paths (t-1721)`.

### Task 2: Frontmatter validation

**Files:**
- Create: `crates/oku-slides-core/src/knowledge/{frontmatter.rs, frontmatter_tests.rs}`
- Modify: `crates/oku-slides-core/src/error/reason.rs` (`FrontmatterReason`)
- Modify: workspace `Cargo.toml` (`saphyr-parser = "0.4"`), core `Cargo.toml`
  (`.workspace = true`), `deny.toml` (ban `serde_yaml`, wrappers `typst-library`, `hayagriva`),
  `Cargo.lock`

**Interfaces:**
- Consumes: `KnowledgePath`, `limits` (Task 1).
- Produces:
  - `pub fn split(text: &str) -> Result<(&str, &str), FrontmatterReason>` → (YAML, body);
    `Missing` when the text does not open with `---`, `Unterminated` when no closing line.
  - `pub struct Frontmatter { pub kind: Option<String>, pub title: Option<String>,
    pub description: Option<String>, pub keys: Vec<String> }` and `pub fn
    frontmatter(text: &str) -> Result<Frontmatter, FrontmatterReason>` (lot 2 index uses it).
  - `pub fn check_text(path: &KnowledgePath, text: &str) -> Result<(), FrontmatterReason>`.
  - `#[non_exhaustive] pub enum FrontmatterReason { Missing, Unterminated, TooLarge, NotYaml,
    NotMapping, DuplicateKey { key: String }, Alias, Tag, TooDeep, TypeMissing,
    BadTimestamp { key: String }, NotAllowed { key: String } }` (derives `Debug, Clone,
    PartialEq, Eq`; Fluent `knowledge-frontmatter-*` with `$key`). `NotAllowed` covers a key
    outside the root `index.md` set and any frontmatter on another `index.md` or a `log.md`
    (`key` = first key).

- [ ] **Step 1: Dependency audit.** Run the `common:team-dep-auditor` agent on
  `saphyr-parser` 0.4 and `pulldown-cmark` 0.13 (`default-features = false`); record the verdict
  and the transitive `thiserror` (already in `Cargo.lock`) in the commit message.
- [ ] **Step 2: Failing tests** (rstest over `check_text`): concept with `type: Note` ok;
  no frontmatter → `Missing`; no close → `Unterminated`; 17 KiB YAML → `TooLarge`;
  `: :` → `NotYaml`; `- a` → `NotMapping`; `type: a` twice → `DuplicateKey{key:"type"}`;
  `a: &x 1` and `b: *x` → `Alias`; `type: !!str x` → `Tag`; 17 nested mappings → `TooDeep`;
  `type: ""` → `TypeMissing`; `generated: {at: 2026-11-24T10:00:00}` →
  `BadTimestamp{key:"generated.at"}`; `generated: 3` → `BadTimestamp{key:"generated"}`; `Z`
  and `z` ok; `x-foo: 1` ok; a second YAML document after `---` inside the block is not read;
  `/index.md` with `okf_version`, `title` ok, with `type` → `NotAllowed{key:"type"}`;
  `/a/index.md` with any → `NotAllowed`; `/log.md` empty ok; CRLF and BOM variants of the ok
  case ok (Review Focus 3); a billion-laughs alias chain → `Alias` without expansion (Review
  Focus 4). `split` tests for `Missing` vs `Unterminated`; `frontmatter` returns `kind`,
  `title`, `keys` in order. Proptests `split_never_panics`, `check_text_never_panics`.
  Localization: `FrontmatterReason::DuplicateKey` in `fr` names the key.
- [ ] **Step 3:** `cargo nextest run -p oku-slides-core -E 'test(/frontmatter/)'` → FAIL.
- [ ] **Step 4:** Implement over `saphyr_parser::Parser` events; reject on the first
  `Alias`, non-zero anchor or explicit tag; stop at the first `DocumentEnd`; depth counter on
  mapping and sequence starts. Add the 20-locale keys.
- [ ] **Step 5:** Same command → PASS; `just check` and `just audit` green;
  `git diff Cargo.lock` shows only the two crates and their listed deps.
- [ ] **Step 6:** Commit `feat(core): OKF frontmatter validation (t-1721)`.

### Task 3: Knowledge ops, errors and the hiding gates

**Files:**
- Modify: `crates/oku-slides-core/src/ops/mod.rs` (variants, dispatch, `ops_schema`)
- Create: `crates/oku-slides-core/src/ops/{knowledge.rs, knowledge_tests.rs}`
- Modify: `crates/oku-slides-core/src/error/{mod.rs, reason.rs}`
- Modify: `crates/oku-slides-i18n/src/locales/*/core.ftl`, `*/session.ftl` (20 locales)
- Modify (exhaustive matches): `oku-slides-session/src/{agent/turn_touches.rs:274,
  pack_mode_preview.rs:~99, document_kind.rs:98, theme_edit.rs, theme_extends.rs,
  theme_guides.rs}`, `oku-slides-core/tests/ops/main.rs`, `oku-slides-session/tests/agent.rs`
  if they match exhaustively
- Modify: `oku-slides-session/src/error.rs` (`KnowledgeOpsHidden`)
- Modify: `ui/src/slideTypes.ts:178` (Op union; t-355 mirror test)

**Interfaces:**
- Consumes: Tasks 1–2.
- Produces:
  - `Op::KnowledgeWrite { path: String, text: String, id: Option<String> }` (create, or
    replace keeping the id), `Op::KnowledgeEdit { path, old: String, new: String }`,
    `Op::KnowledgeMove { from: String, to: String }`, `Op::KnowledgeDelete { path }`; serde
    tags `knowledge_write`… as the other variants. Creation needs `id` (valid, unused);
    replacing needs `id` absent or equal. Apply is deterministic (it never mints an id).
  - `pub fn Op::is_knowledge(&self) -> bool`.
  - `#[non_exhaustive] pub enum KnowledgeReason { Path(PathReason),
    Frontmatter(FrontmatterReason), NotFound, Exists, IdMissing, IdInvalid, IdTaken,
    IdMismatch, EditEmptyOld, EditNoMatch, EditAmbiguous, TooManyFiles, FileTooLarge,
    TotalTooLarge }` in `reason.rs`.
  - `OpInner::Knowledge { path: String, reason: KnowledgeReason, backtrace }`, display
    `knowledge file {path}: {reason}`; `OpErrorKind::Knowledge`; arms in `kind()`, `subject()`
    (`path`) and `Localize` (`op-knowledge`, `$path`, `$reason` from `knowledge-reason-*`);
    `OpError::knowledge_reason() -> Option<&KnowledgeReason>`.
  - `ops_schema()` omits the four variants (filter by tag) until lot 3.
  - Session: `ensure_caller_ops` refuses any `is_knowledge` op first, for every kind,
    with `KnowledgeOpsHidden` (Fluent `session-knowledge-ops-hidden`).
  - Re-exports: `KnowledgeReason`.

- [ ] **Step 1: Failing tests** (`knowledge_tests.rs`): write creates (path, text
  byte-identical); write on existing path replaces, id kept; create without id → `IdMissing`,
  `ABC` → `IdInvalid`, an id in use → `IdTaken`; replace with another id → `IdMismatch`; bad
  path, bad frontmatter refused, deck unchanged; edit replaces the single occurrence; empty
  `old` → `EditEmptyOld`; absent → `EditNoMatch`; `aa` in `aaa` (overlapping) and in
  `aaaa` → `EditAmbiguous` (Review Focus 4 of the spec); `\r\n` in text matched only
  by `\r\n` in `old` (documented: edits are byte-exact); edit result re-checked (removing
  `type` refused); move keeps id and text and re-checks the text at the target
  (`/a.md` → `/index.md` refused `NotAllowed`); move onto existing → `Exists`; move/delete of
  unknown → `NotFound`; 1025th file → `TooManyFiles`; 1 MiB + 1 → `FileTooLarge`; total past
  16 MiB → `TotalTooLarge`; a failing op in a batch leaves `apply_all` with no change;
  `localize` in `fr` for `EditAmbiguous` contains the French text and the path, test on the
  key. `ops_schema_omits_knowledge_ops`. Session: `caller_knowledge_op_is_refused_in_every_kind`
  (deck, theme, pack, help). Existing `every_op_variant_is_mirrored_in_the_ui_op_type` passes.
- [ ] **Step 2:** `cargo nextest run -p oku-slides-core -E 'test(/knowledge/)'` → FAIL.
- [ ] **Step 3:** Implement in `ops/knowledge.rs` (lookup by path scans the map; at most
  1024 files). `str::matches(old).count()` then `replacen(old, new, 1)`. Exhaustive-match
  arms: `turn_touches` and `pack_mode_preview` touch no slide (`continue` / empty); theme
  matches treat them as non-token ops. Add the TS Op variants. Translations in 20 locales.
- [ ] **Step 4:** PASS; `cargo check --workspace --tests`; `cargo nextest run -p
  oku-slides-core -p oku-slides-session -p oku-slides-i18n`; `cd ui && bun run tsc` green.
- [ ] **Step 5:** Commit `feat(core): knowledge ops, hidden from callers until approval (t-1721)`.

### Task 4: Normalize, replica storage and format 8

**Files:**
- Create: `crates/oku-slides-core/src/knowledge/{normalize.rs, normalize_tests.rs}`
- Modify: `crates/oku-slides-core/src/canonical.rs` (calls `normalize`; issues discarded
  there, reported on read)
- Modify: `crates/oku-slides-core/src/replica/schema.rs` (`Ctx::Knowledge`,
  `Ctx::KnowledgeFile`; `kind`, `texts`, `maps`)
- Modify: `crates/oku-slides-core/src/replica/mod.rs` (`KNOWLEDGE`, `REPLICA_FORMAT = 8`,
  `check_format`, upgrade, `read_deck`, `reconcile_doc`)
- Modify: `crates/oku-slides-core/src/replica/read.rs` (verbatim text, `deck(root, knowledge)`)
- Modify: `crates/oku-slides-core/src/sources.rs:51` (`SOURCES_VERSION = 14`),
  `sources_zip.rs:233-234` (accept 13 and 14)
- Modify: `crates/oku-slides-core/src/validate.rs` (`Issue` constructors), `core.ftl` ×20
- Modify: `replica/read_tests.rs:107` helper; `replica/{ARCHITECTURE.md, AGENTS.md}`,
  `crates/oku-slides-core/{ARCHITECTURE.md, AGENTS.md}`
- Test: `replica/knowledge_tests.rs` (new), existing merge and convergence proptests

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces:
  - `pub(crate) fn knowledge::repair(files: BTreeMap<String, KnowledgeFile>) ->
    (BTreeMap<String, KnowledgeFile>, Vec<Issue>)`, pure, structural only (texts are not
    read; `canonicalize` runs it, skipped for an empty bundle), in this order:
    1. drop an entry whose id fails `FileId::parse` (`Issue::knowledge_unreadable(id)`);
    2. an invalid path becomes `/recovered/<id>.md` (`Issue::knowledge_path_invalid(id,
       reason)`);
    3. pass one: each valid, non-recovered path is claimed by its lowest id; pass two: every
       loser (and recovered path that collides) takes the first free `with_suffix(n)`,
       n from 2, bounded by the file count; a `TooLong` suffix falls back to
       `/recovered/<id>.md`, then suffixed (`Issue::knowledge_path_collision(path)`);
       `/index-2.md` and `/log-2.md` become concept files; a file that finds no place
       (unreachable) is reported `knowledge_unreadable`.
  - `pub fn knowledge::normalize(files)` (same signature), pure, used on read: `repair`, then
    4. `check_text` failure → kept unchanged with `Issue::knowledge_invalid(path, reason)`;
    5. last, keep every file past `MAX_FILES`, `MAX_FILE_BYTES` or `MAX_TOTAL_BYTES`
       (counted in id order), with one `Issue::knowledge_over_limit(limit: KnowledgeLimit,
       first id, count)` per ceiling passed (R10).
  - `Issue` constructors take typed reasons; keys `issue-knowledge-*`.
  - `Ctx::Knowledge` (every key → `Nested(KnowledgeFile)`), `Ctx::KnowledgeFile` (`path` →
    `Atom`, `text` → `Prose`, other keys → `Atom`, dropped on read); `texts()` lists
    `KnowledgeFile => &["text"]`; `sync_map` looks entries up by id key.
  - Read of `Ctx::KnowledgeFile.text` is verbatim: an empty `LoroText` reads `""`, nothing
    trimmed (other prose keeps blank-as-absent).
  - `reconcile_doc` syncs `doc.get_map(KNOWLEDGE)` from `obj.remove("knowledge")` in the
    same commit as `deck`; the `deck` map never has a `knowledge` key.
  - `fn read_deck(doc)` reads both roots (`read::deck(&root, &knowledge)`), so restore,
    `deck_at`, merge trial and undo all see knowledge. A `knowledge` root of the wrong
    container type reads empty with `Issue::knowledge_unreadable("knowledge")`.
  - `check_format` accepts 7 and 8, refuses others; `import_snapshot` on a format-7 doc sets
    `meta.format = 8` in a silent commit after `read_deck` succeeds.

- [ ] **Step 1: Failing tests.** `normalize_tests.rs`: each step above, one case each;
  two files claiming `/a.md` → lowest id keeps it, other `/a-2.md`, and with a third already
  at `/a-2.md` → `/a-3.md`; collision on a 253-byte path → recovered; id `XYZ` dropped;
  over-cap files kept with one issue per ceiling; idempotence (`normalize(normalize(x)) == normalize(x)`,
  no issue the second time; proptest). Replica (`knowledge_tests.rs`): round trip with two
  files byte-identical, an empty `/log.md` included (Review Focus 3); unchanged apply → no
  new change (`oplog_vv` equal); deck map has no `knowledge` key; a format-7 snapshot (built
  by writing `meta.format = 7` silently) opens with empty knowledge and saves as 8; format 9
  refused `UnsupportedReplicaFormat`; sources 13 loads, 14 written, 15 refused; save/load
  round trip of sources with knowledge. Concurrency matrix (distinct peer ids, both merge
  directions, results equal): same file, edits at two positions → both kept; delete vs edit;
  move vs edit; write vs edit; three peers creating `/a.md` (Review Focus 1); after merge
  the first apply persists the repair and the second is write-free. Hostile raw entries
  written into the root map: `/../x.md`, `/con.md`, 10 000-byte path → `/recovered/<id>.md`
  with an issue; bad id dropped (Review Focus 2); raw text without frontmatter kept with
  `knowledge_invalid`; root `knowledge` written as a list → empty plus issue. `undo` after a
  knowledge write restores the text; `deck_at` and `restore` see knowledge. Issue localized
  in `de`, asserted on key. Extend the convergence proptest with knowledge ops at
  `with_cases(48)`.
- [ ] **Step 2:** `cargo nextest run -p oku-slides-core -E 'test(/knowledge|replica|sources/)'`
  → FAIL.
- [ ] **Step 3:** Implement. Docs in the same commit: replica `ARCHITECTURE.md` Format line
  → 8 and a `knowledge` root container line (≤ 50 lines, trim elsewhere); replica `AGENTS.md`
  rule "Keep the knowledge bundle in the root container `knowledge`, never under `deck`, and
  read its text verbatim. (plan:2026-11-24-deck-knowledge-okf-lot1-core)"; core `AGENTS.md:50`
  gains "except format 7, read as 8 with an empty knowledge (spec:2026-11-24-deck-knowledge-
  okf-design)"; core `ARCHITECTURE.md` gets the `knowledge/` module and the two parsers
  (≤ 50 lines).
- [ ] **Step 4:** PASS, including the Loro probe tests (`cargo nextest run -p oku-slides-core
  -E 'test(/probe/)'`); `just check` (agents-lint) green.
- [ ] **Step 5:** Commit `feat(core): knowledge in the replica, format 8 (t-1721)`.

### Task 5: Cited image assets

**Files:**
- Create: `crates/oku-slides-core/src/knowledge/{assets.rs, assets_tests.rs}`
- Modify: `crates/oku-slides-core/src/validate.rs` (`knowledge_asset_refs`), `lib.rs`
- Modify: workspace and core `Cargo.toml` (`pulldown-cmark = { version = "0.13",
  default-features = false }`), `Cargo.lock`

**Interfaces:**
- Consumes: `KnowledgePath` (Task 1), `split` (Task 2).
- Produces:
  - `pub const ASSETS_DIR: &str = "/references/assets/"`.
  - `pub fn cited_assets(file: &KnowledgeFile) -> BTreeSet<String>`, infallible: CommonMark
    images (`Tag::Image`) in the body (whole text when `split` fails); the URL is resolved
    against the file's directory, `..` lexical and refused past the root; refused: a scheme,
    `//host`, a query, a fragment, any `%`; kept: `/references/assets/<64 lowercase
    hex>.<ext>` → `sha256:<hex>`. HTML `<img>` is not counted (documented).
  - `pub fn knowledge_asset_refs(deck: &Deck) -> BTreeSet<String>` (union over files).
  - Re-exports: `ASSETS_DIR`, `cited_assets`, `knowledge_asset_refs`.

- [ ] **Step 1: Failing tests:** absolute and relative (`assets/<hex>.png` from
  `/references/a.md`, `../references/assets/…` from `/notes/a.md`) → `{sha256:<hex>}`;
  `/../../references/assets/…` from `/a.md` → empty; `https://x/references/assets/…`,
  `//h/…`, `?q`, `#f`, `%2e%2e` → empty; uppercase hex, 63 hex → empty; fenced block, a link,
  an `<img>` tag, an image in the frontmatter → empty (Review Focus 5); invalid frontmatter
  still scanned. Sources: an asset cited only by knowledge survives save/load and survives
  deleting its last citing file (add-only). Proptest `cited_assets_never_panics`.
- [ ] **Step 2:** `cargo nextest run -p oku-slides-core -E 'test(/assets/)'` → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** PASS; `just check` and `just audit` green; core `ARCHITECTURE.md` line on
  cited knowledge assets (≤ 50 lines).
- [ ] **Step 5:** Commit `feat(core): knowledge images are cited assets (t-1721)`.

### Task 6: Spec sync and gate

**Files:**
- Modify: `docs/superpowers/specs/2026-11-24-deck-knowledge-okf-design.md` only if
  implementation moved a decision this plan records.

- [ ] **Step 1:** `just ci` green (check, test, audit).
- [ ] **Step 2:** `/review` on `main..HEAD`; fix CRITICAL and MAJOR in the task commits'
  files; commit fixes with `Reviewed-by:`.
- [ ] **Step 3:** Land per the root `AGENTS.md` workflow.

## Deferred (backlog)

- Content hash per file for lot 3 approval diffs.
- `Arc` for knowledge clones if profiling shows deck clones grow.
- Check `document.json` does not store knowledge twice (lot 6).
- `comrak` vs `pulldown-cmark` for lot 5 rendering.
- MCP read tools only after lot 3.
