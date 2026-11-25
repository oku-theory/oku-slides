# Deck knowledge base — lot 3: approval and agent writes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The deck agent writes the knowledge base (its writes approved at once), and the
user approves received knowledge from the deck tab, files cited by id; the hiding of the
knowledge ops goes.

**Architecture:** `KnowledgeApprovals` gains a strict read-merge-write path.
`hide_knowledge_ops` and `KnowledgeOpsHidden` go in one commit; the session follows each
file id through a caller batch (minting create ids, refusing edits of unapproved text) and
approves the texts the batch authored before anything is installed. Four deck-only MCP tools
wrap the ops. Three requests (`knowledge_review`, `knowledge_text`, `approve_knowledge`) and a
`Snapshot.knowledge` summary, cached on a knowledge-only generation, feed a deck-tab notice
and an approval dialog where only a shown, whole text can be approved.

**Tech Stack:** Rust, snafu, Fluent (20 locales), sha2 via `oku_slides_core::asset_id`, Loro
1.17.1, rust-mcp-sdk, React/TS (Bun), speculoos, rstest.

**Parent task:** t-1728 (stays open until lot 6). Sub-tasks: Task 1 t-1731, Task 2 t-1743,
Task 3 t-1744, Task 4 t-1745, Task 5 t-1746, Task 6 t-1747, Task 7 t-1748, Task 8 t-1749.

**Spec:** `docs/superpowers/specs/2026-11-24-deck-knowledge-okf-design.md` (« Received
knowledge », « Settled after the builder check », lot 1 and lot 2 plan decisions). Lot 2 plan:
`docs/superpowers/plans/2026-11-25-deck-knowledge-okf-lot2-mcp-index.md`.

## Plan decisions (builder calls; Task 3 copies them into the spec)

- **t-1731 lands first.** Opened writes make every apply pay for the bundle. `Sources` compares
  the next bundle with its own (`BTreeMap ==`: early exit on a difference, one memcmp of the
  bundle when equal, a few ms at 16 MiB, accepted); unchanged, it moves the bundle out of the
  deck for the replica call (no serialization, no Loro sync) and back. Render requests carry
  no bundle. The `apply_ops_inner` deck clone stays (an `Arc` bundle is deferred).
- **Knowledge generation.** `Sources::knowledge_generation()` takes a fresh value from a
  process-wide counter at creation and whenever its bundle changes (apply, undo, redo,
  restore, merge): the snapshot summary's cache key, never moved by a slide edit.
- **Approvals file: strict read-merge-write.** Checks still use the set read once per
  process. `approve` holds the set's lock, reads the file strictly (missing = empty; any other
  read or parse error refuses, file untouched), writes memory ∪ file ∪ new atomically, and
  only then updates memory: no reader ever sees an unsaved approval. Nothing new → no write.
  Two processes writing at the same instant can still lose one's hashes (fail closed: those
  texts await again), as for guide approvals.
- **Approve on write, by id.** Every caller apply (agent tools, `apply`, a UI apply: the agent
  or the user wrote it) follows each file id through the batch: an id is *tainted* when its
  text before the batch is not approved; a whole-text write clears it; move and delete keep
  it; an edit of a tainted id is refused (`KnowledgeEditAwaiting`), before core runs, so an
  edit's `old` never probes an unread file. The ids written or edited are *authored*; right
  after `apply_all_moving` and before assets, widgets or commit, the hash of each authored
  file's text in the next deck is approved. A failed approvals write refuses the apply,
  nothing changed. A commit failing later leaves approved texts our own caller wrote: no
  injection path. Never on open, merge, receive, undo, redo or restore. Approval is per text,
  for every deck (spec): an agent write of a text equal to a received file's approves both.
  The spec said "an agent tool or extraction"; the user's own writes join it (amendment).
- **Over limit: only deleting brings it back.** Core refuses `knowledge_write` and
  `knowledge_edit` while the bundle is past any limit (new reason `OverLimit`); move and
  delete pass. The notice, the dialog and `knowledge.md` say exactly that.
- **Ids minted with `oku_slides_core::knowledge_file_id()`** for a write without id on a path
  free at that point of the batch.
- **Knowledge ops fit deck tabs only** (`WrongDocument` in pack, theme and help tabs; history
  is read-only); previews refuse them with `KnowledgeOpsInPreview`. Preview tools' schemas now
  list the knowledge ops too (one schema); the refusal says why.
- **Files cited by id** in every request and view of this lot; paths and titles are labels,
  passed through `agent::bounds::label` (`label_within 80` for titles) wherever they reach a
  message or a view.
- **The user approves only a text the dialog showed whole.** `knowledge_text` returns the
  text and its hash; Approve appears only once the text is shown and sends that hash; the host
  recomputes each file's hash and refuses (`KnowledgeChanged`) on a mismatch. A file past
  `MAX_FILE_BYTES` is shown cut and cannot be approved (`KnowledgeTooLargeToApprove`): delete
  it. No "Approve all". Hidden characters (controls, format, private-use, unassigned) are shown
  as visible `⟨U+XXXX⟩` escapes with a warning.
- **The review requests are user-only**: refused to the agent window, not deck changes, not
  persisted; refused outside `Mode::Deck` (`WrongDocument`).
- **Deferred with reason:** provenance stamping waits for lot 4's extraction (one
  implementation); receipt entries for knowledge writes wait for lot 5's tab (the receipt
  lists elements, a knowledge entry links to the tab); the approvals file is not capped (72
  bytes per agent write).
- **No e2e this lot**: no UI writes knowledge; render and session tests cover the flow.

## Global Constraints

- Deck tabs only (`Mode::Deck` for requests and summary; `Kinds::DECK` for tools).
- Approval key: `oku_slides_core::asset_id(text.as_bytes())` (`sha256:<hex>`); approvals are a
  user click or a caller write, never an agent tool that approves by itself.
- Errors: snafu, Fluent key in all 20 locales, tested on key plus a non-English locale; CJK
  classifiers without a space (`{ $count }个`); counts use `[one]`/`*[other]` selectors as
  `en/ui.ftl:18-19`. Paths reach messages only through `bounds::label`; no host path ever
  reaches the agent.
- Never hold the Session mutex across an await; all new session code is synchronous. MCP
  errors through `Tools::fail_with`; MCP tested as a real server on 127.0.0.1 port 0.
- New `impl Session` code in its own module; `lib.rs` gains match arms, `mod` lines and one
  cache field only. Each new `Request` variant: `request.rs` (variant, arm, persist list),
  `request_class.rs` (each exhaustive match), `agent_window_allow.rs`, `ui/src/api.ts`.
- UI: modals on `useModalFrame`; every string through `t()`; received text in text nodes and
  `<pre dir="auto">` only, never Markdown or HTML; buttons from `primitives.tsx`; render tests
  per `docs/agents/ui-e2e.md` (`bun test .render.test. Knowledge`).
- Tests named `<unit>_<scenario>_<expected>`, one behavior each; Rust unit tests in sibling
  `_tests.rs` with speculoos.
- `ARCHITECTURE.md` files at 50 lines (core, core replica, mcp, render-worker) are edited by
  rewording existing lines, never by appending; a rule change updates the nearest `AGENTS.md`
  in the same commit, single line ending with `(plan:2026-11-25-deck-knowledge-okf-lot3-approval, t-N)`.
- App runs and evals use fresh `OKU_SLIDES_PATHS__DATA` / `OKU_SLIDES_PATHS__DECKS` temp dirs.
- Commits cite their sub-task; message files under `.superpowers/sdd/knowledge-okf-lot3/`;
  subagents stage only (one path per `git add`); the controller commits each task after its
  SDD spec and quality reviews, trailer `Reviewed-by: sdd task review (spec, quality)`.

## Review Focus

1. Laundering: an edit of an unapproved file, directly, after a move, or after a delete then
   write of another id at the same path: refused, nothing approved (Task 3).
2. A file changed by a merge between the dialog's text fetch and the click: refused with
   `KnowledgeChanged`, nothing approved (Task 5).
3. A corrupt approvals file: approve refuses and leaves it byte-identical (Task 2).
4. A bundle past a limit after a merge: write and edit refused, delete passes, the summary
   names the limit and the notice says delete only (Tasks 3, 5, 6).
5. An unwritable approvals file during a batch that also changes widgets: the apply is
   refused, deck and widget registry unchanged (Task 3).

---

### Task 1: Knowledge: avoid per-apply work on an unchanged bundle (t-1731)

**Files:**
- Modify: `crates/oku-slides-core/src/replica/mod.rs` (`reconcile_doc` l. 477-504,
  `apply_moving` l. 428, `apply_silent` l. 461), `src/sources.rs` (`apply_moving`,
  `apply_silent`, `reread`, `restore`, `merge`, constructors), `src/model.rs` (`Deck`)
- Modify: `crates/oku-slides-render-worker/src/slot.rs:147` (`Box::new(deck.without_knowledge())`)
- Modify: `crates/oku-slides-core/src/replica/AGENTS.md` (one rule), `replica/ARCHITECTURE.md`
  (reword the reconcile line)
- Test: `crates/oku-slides-core/src/replica/knowledge_tests.rs`, `src/sources_tests.rs`,
  `crates/oku-slides-render-worker/src/slot_tests.rs` (or the crate's existing slot test file)

**Interfaces:**
- Produces (core):
  - `pub enum KnowledgeSync { Changed, Unchanged }` (replica): `DeckReplica::apply_moving(&mut
    self, deck, label, merge, moves, knowledge: KnowledgeSync)` and `apply_silent(&mut self,
    deck, knowledge: KnowledgeSync)`; `Unchanged` skips the `KNOWLEDGE` `sync_map`. `create`,
    `restore` and tests always sync.
  - `Sources::apply_moving`/`apply_silent`: `Unchanged` when `deck.knowledge ==
    self.deck.knowledge`, then `std::mem::take` the bundle for the replica call and put it back.
  - `Sources::knowledge_generation(&self) -> u64` (process-wide `AtomicU64`, fresh at creation
    and on each change).
  - `Deck::without_knowledge(&self) -> Deck`: an exhaustive destructure with no `..` (a new
    `Deck` field fails to compile), cloning every field but the bundle.
- Rule (replica `AGENTS.md`): "Sync the knowledge root only when the bundle changed; the
  caller says which, never a default (plan:2026-11-25-deck-knowledge-okf-lot3-approval, t-1731)".

- [ ] **Step 1: Failing tests**:
  `reconcile_unchanged_knowledge_keeps_doc_text` (apply with `Unchanged` and a deck whose
  bundle differs: the replica re-read keeps the old text, proving the skip);
  `sources_slide_edit_keeps_bundle_and_generation` (1 MiB bundle, title edit: re-read equal,
  generation unchanged); `sources_knowledge_change_syncs_and_bumps_generation`;
  `sources_undo_redo_merge_restore_bump_generation_on_knowledge_change` (rstest cases);
  `deck_without_knowledge_keeps_everything_else`; `slot_render_request_has_no_knowledge`;
  `render_same_pages_with_or_without_knowledge` (page hashes equal).
- [ ] **Step 2:** `RUSTC_WRAPPER= cargo nextest run -p oku-slides-core -p oku-slides-render-worker -E 'test(/knowledge|generation/)'` → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** PASS; `just check`.
- [ ] **Step 5:** Commit `perf(core,render-worker): skip the unchanged knowledge bundle on apply and render (t-1731)`.

### Task 2: Knowledge lot 3: approvals store write path (t-1743)

**Files:**
- Modify: `crates/oku-slides-session/src/approvals_file.rs` (+ tests: `read_set_strict`, doc),
  `src/knowledge_approvals.rs` + `_tests.rs` (doc: strict approve, the same-instant race),
  `src/library_packs.rs:97,142`, `src/knowledge_fixture.rs` and every test caller of `load`,
  `src/error.rs` (variant, kind lists l. 446-451 and 701-704, kind arm), `session.ftl` ×20

**Interfaces:**
- Produces:
  - `approvals_file::read_set_strict(path: &Path, storage: &Storage) -> Result<BTreeSet<String>, ReadError>`
    (missing = empty; unreadable or not a JSON array of strings = error)
  - `KnowledgeApprovals::load(path: Option<PathBuf>, storage: Storage) -> Self` (keeps path and
    storage, as `GuideApprovals`)
  - `KnowledgeApprovals::approve(&self, hashes: &[String]) -> Result<()>`: set lock held
    throughout; strict read; nothing new in memory ∪ file → `Ok` without writing; else write
    the sorted union with `Storage::write_atomic` in a private `write` helper, then replace
    memory with the union. Without a path: memory only.
  - `KnowledgeApprovals::count(&self) -> usize` (only grows)
  - `SessionError::KnowledgeApprovalsWrite` (no path field; the path goes to `tracing`), key
    `session-knowledge-approvals-write` = "Your knowledge approvals could not be saved."

- [ ] **Step 1: Failing tests** (each asserts the file bytes through `std::fs` in a tempdir,
  not a second `load`, which shares the process set): `approve_writes_sorted_array`;
  `approve_keeps_hashes_added_by_hand_after_load` (Review Focus 3, and they are approved
  after); `approve_corrupt_file_refuses_and_leaves_it` (Review Focus 3); `approve_unwritable_path_refuses_and_leaves_memory`
  (`is_approved` false); `approve_known_hashes_writes_nothing` (mtime/bytes unchanged);
  `approve_empty_and_duplicate_input`; `approve_two_threads_keep_both`;
  `approve_without_path_in_memory`; `count_grows`; `read_set_strict_missing_is_empty`;
  Fluent key on `en` and `fr`.
- [ ] **Step 2:** `RUSTC_WRAPPER= cargo nextest run -p oku-slides-session -E 'test(/approvals/)'` → FAIL.
- [ ] **Step 3:** Implement. **Step 4:** PASS; `just check`.
- [ ] **Step 5:** Commit `feat(session): write knowledge approvals (t-1743)`.

### Task 3: Knowledge lot 3: open knowledge writes, approve on write (t-1744)

**Files:**
- Modify (core): `src/ops/mod.rs` (remove `hide_knowledge_ops`, `is_knowledge_variant`),
  `src/lib.rs:93`, `src/ops/schema_tests.rs:28`, `tests/tables.rs` if it names them;
  `src/ops/knowledge.rs` (+ tests: write and edit refused while the bundle is past a limit),
  `src/error/reason.rs` (`KnowledgeReason::OverLimit`, key `knowledge-reason-over-limit`),
  `core.ftl` ×20 ("The knowledge base is past its limits: only deleting files brings it back
  under."), `crates/oku-slides-core/ARCHITECTURE.md:21` (drop "hidden from `ops_schema`")
- Modify (mcp): `src/tools.rs:248` (`tool()` no longer hides) and the `apply` description
  (append: "knowledge_write, knowledge_edit, knowledge_move and knowledge_delete change the
  deck's knowledge base; prefer the knowledge_* tools.")
- Modify (session): `src/document_kind.rs` + tests (remove `ensure_no_knowledge_ops`; `fits`:
  knowledge ops `true` for `DocumentKind::Deck` only, a separate `Help` arm);
  `src/agent/variants.rs:149,215` + tests (`KnowledgeOpsInPreview`); `src/error.rs`
  (remove `KnowledgeOpsHidden` at its four sites; add `KnowledgeOpsInPreview`,
  `KnowledgeEditAwaiting { path }`); `session.ftl` ×20 (remove `session-knowledge-ops-hidden`;
  add `session-knowledge-ops-in-preview` = "A preview shows slide changes only: write
  knowledge files with the knowledge tools.", `session-knowledge-edit-awaiting` = "The
  knowledge file { $path } awaits the user's approval: replace it whole or leave it.")
- Create: `src/knowledge_write.rs` + `_tests.rs`; `lib.rs` `mod` line
- Modify: `src/deck_widgets.rs` (`apply_ops_inner` l. 355-414; `apply_silent_ops` l. 328)
- Modify: `ui/src/slideTypes.ts:206` (drop the comment)
- Modify: session `AGENTS.md` (replace the lot 2 hiding rule and amend the knowledge index
  rule at l. 43: files cited by id in views, the agent block keeps paths), session
  `ARCHITECTURE.md` (reword l. 9 and 24), the spec: lines 86, 209, 219 and a new section
  « Plan decisions (lot 3 builder) » condensing this plan's decisions
- New session rule: "Only deck tabs take caller knowledge ops; previews refuse them; the
  session follows ids through a batch, refuses an edit of unapproved text and approves what
  the batch authored before installing anything, never on open, merge, receive, undo or redo
  (plan:2026-11-25-deck-knowledge-okf-lot3-approval, t-1744)"

**Interfaces:**
- Consumes: `KnowledgeApprovals::approve` (Task 2), `Session::knowledge_approved` (lot 2),
  `oku_slides_core::knowledge_file_id`.
- Produces (`knowledge_write.rs`):
  - `Session::prepare_knowledge_ops(&self, ops: &mut [Op]) -> Result<BTreeSet<String>>`:
    simulates paths → ids from the current deck, taint per id as in Plan decisions; mints ids;
    returns the authored ids. Validates each path with `KnowledgePath::parse` before the walk
    (an invalid path is left to core's refusal, never tracked).
  - `Session::approve_knowledge_writes(&self, next: &Deck, authored: &BTreeSet<String>) -> Result<()>`:
    approves `asset_id` of each authored id's text present in `next` (one `approve` call).
- `apply_ops_inner`: when any op is knowledge, `prepare_knowledge_ops` first, then
  `apply_all_moving`, then `approve_knowledge_writes`, then `embed_op_assets` and the rest.
  `apply_silent_ops` refuses knowledge ops with `WrongDocument`.

- [ ] **Step 1: Failing tests** (`knowledge_write_tests.rs`, data dir in a tempdir):
  `write_without_id_mints_hex_id_and_approves` (32 hex, distinct ids for two writes in one
  batch, readable through `knowledge_file`); `replace_of_awaiting_file_approves_new_text`;
  `edit_of_awaiting_file_refused_before_core` (nonmatching `old` still gives
  `KnowledgeEditAwaiting`, key + `fr`, deck unchanged); `move_then_edit_of_awaiting_refused`
  (Review Focus 1); `delete_then_write_same_path_then_edit_allowed`;
  `write_then_edit_awaiting_in_one_batch_allowed`; `move_keeps_awaiting_file_awaiting`;
  `move_of_approved_file_stays_approved`; `failed_batch_approves_nothing` (a later op
  refused by core); `undo_redo_of_agent_write_never_approves` (received text awaits after
  undo; a redo approves nothing new); `merge_never_approves`;
  `unwritable_approvals_refuse_apply_with_widgets_unchanged` (Review Focus 5);
  `over_limit_refuses_write_and_edit_allows_delete_and_move` (Review Focus 4, core and
  session); `pack_theme_help_tabs_refuse_knowledge_ops`; `history_refuses_knowledge_ops`;
  `apply_silent_ops_refuses_knowledge_ops`; `apply_by_agent_path_approves` (through
  `AgentWrites`); previews expect `KnowledgeOpsInPreview` (mixed batch too); core
  `ops_schema_lists_knowledge_ops`; mcp `apply_schema_lists_knowledge_ops`.
- [ ] **Step 2:** `RUSTC_WRAPPER= cargo nextest run -p oku-slides-core -p oku-slides-session -p oku-slides-mcp -E 'test(/knowledge|document_kind|variants|schema/)'` → FAIL.
- [ ] **Step 3:** Implement; `rg -n "hide_knowledge_ops|KnowledgeOpsHidden|knowledge-ops-hidden|ensure_no_knowledge_ops"`
  outside `docs/superpowers` returns nothing.
- [ ] **Step 4:** PASS; `just check`.
- [ ] **Step 5:** Commit `feat(session): open knowledge writes, approved as written (t-1744)`.

### Task 4: Knowledge lot 3: MCP knowledge write tools (t-1745)

**Files:**
- Modify: `crates/oku-slides-mcp/src/knowledge_tools.rs` (+ `_tests.rs`), `tools.rs` (four
  entries after `knowledge_search`, `Kinds::DECK`), `handler.rs`, `tools_tests.rs:203`
  (alphabetical list), `ARCHITECTURE.md` (reword the knowledge line)
- Modify: `crates/oku-slides-agent/src/instructions/knowledge.md` (not `deck.md`) +
  `instructions_tests.rs:63`
- Modify: `crates/oku-slides-local-agent/src/families.rs` (four tools → `&[Knowledge]`; doc
  comment l. 13 "read only" → "the knowledge base"; summary waits for Task 7)
- Test: `crates/oku-slides-mcp/tests/knowledge.rs`

**Interfaces:**
- Consumes: `Session::apply_by_agent` with one op (Task 3 mints and approves).
- Args: `KnowledgeWriteArgs { path, text }`, `KnowledgeEditArgs { path, old, new }`,
  `KnowledgeMoveArgs { from, to }`, `KnowledgeDeleteArgs { path }`; undo label as `apply`'s.
- Output: `{"path": "<the file's path after the op>"}` (`delete`: the deleted path).
- Descriptions:
  - `knowledge_write`: "Create or replace a knowledge base file with its whole text, frontmatter included (a concept file needs a type). path: bundle-absolute, such as /notes/pricing.md. Undoable."
  - `knowledge_edit`: "Replace one exact, unique passage of a knowledge base file (old must appear once) with new. Refused for a file awaiting the user's approval. Undoable."
  - `knowledge_move`: "Move or rename a knowledge base file (from, to: bundle-absolute paths). Undoable."
  - `knowledge_delete`: "Delete a knowledge base file. Undoable."
- `knowledge.md`: fold into the existing "A file awaiting approval cannot be read…" sentence:
  "…cannot be read or edited: replace it whole or leave it." Then add: "Keep the base current
  with knowledge_write and knowledge_edit: one concept per file with a `type` and a `title` in
  its frontmatter, `sources` for where a fact comes from, an `index.md` per directory listing
  its files, and a `log.md` entry for each research session. Your writes are approved at
  once. A base past its limits takes only deletes and moves: tell the user and propose files
  to delete."

- [ ] **Step 1: Failing tests** (real server, data dir seeded): `deck_tab_lists_write_tools`,
  `help_pack_theme_tabs_list_none`; `write_then_read_returns_approved_text`; edit, move,
  delete outputs; `edit_of_awaiting_file_error_in_fr`; `turn_writes_undo_in_one_step`
  (`undo_turn`); `knowledge_instructions_name_write_tools` and help still has no "knowledge".
- [ ] **Step 2:** `RUSTC_WRAPPER= cargo nextest run -p oku-slides-mcp -p oku-slides-agent -p oku-slides-local-agent -E 'test(/knowledge|instructions|families/)'` → FAIL.
- [ ] **Step 3:** Implement. **Step 4:** PASS; `just check`.
- [ ] **Step 5:** Commit `feat(mcp): knowledge write tools for the deck agent (t-1745)`.

### Task 5: Knowledge lot 3: user approval requests and snapshot summary (t-1746)

**Files:**
- Create: `crates/oku-slides-session/src/knowledge_review.rs` + `_tests.rs` (requests,
  `KnowledgeSummaryCache`)
- Modify: `src/request.rs` (variants, arms l. 568, the no-persist list l. 516),
  `src/request_class.rs` + tests (each exhaustive match: none changes the deck; all refused
  in theme tabs), `src/agent_window_allow.rs:114` (all three refused), `src/lib.rs` (arms,
  `mod`, the cache field), `src/views.rs` + `views_tests.rs`, `src/knowledge_view.rs`
  (`cut` → `pub(crate)`), `src/error.rs`, `session.ftl` ×20, session `ARCHITECTURE.md`
- Modify: `crates/oku-slides-core/src/knowledge/normalize.rs` (+ re-exports in
  `knowledge/mod.rs`, `lib.rs`), `src/error/reason.rs` (`KnowledgeLimit::slug`)
- Modify: `ui/src/api.ts` (types next to the `Snapshot` types, `Request` union members, three
  calls next to `approveGuides`), new `ui/src/knowledgeTypes.ts`, every typed `Snapshot`
  fixture (`knowledge: null`)

**Interfaces:**
- Produces (core): `pub fn knowledge_over_limits(files: &BTreeMap<String, KnowledgeFile>) -> Vec<KnowledgeLimit>`
  (the scan `over_limits` uses); `KnowledgeLimit::slug(self) -> &'static str`
  (`too-many-files`, `file-too-large`, `total-too-large`).
- Produces (session), deck mode only:
  - `Request::KnowledgeReview` → `KnowledgeReview { files: Vec<AwaitingKnowledge>, over_limit: Vec<&'static str> }`;
    `AwaitingKnowledge { id, path /* label */, title: Option<String> /* label_within 80 */, bytes }`,
    awaiting files only, sorted by path, at most 200 rows plus `more: usize`
  - `Request::KnowledgeText { id }` → `KnowledgeText { id, path, hash, text, truncated: usize }`:
    read from the deck (not `knowledge_file`), `cut` at `MAX_FILE_BYTES`, `hash` of the whole
    text
  - `Request::ApproveKnowledge { files: Vec<KnowledgeApproval { id, hash }> }` → Snapshot:
    at most `MAX_FILES` entries, a repeated id refused (`KnowledgeApprovalInvalid`), each id
    must exist (`KnowledgeIdNotFound`), its recomputed hash must equal (`KnowledgeChanged
    { path }`), its text ≤ `MAX_FILE_BYTES` (`KnowledgeTooLargeToApprove { path }`); all or
    nothing; already approved is a no-op; empty list is a no-op
  - `Snapshot.knowledge: Option<KnowledgeSummary { awaiting: usize, over_limit: Vec<&'static str> }>`
    (`Serialize`); `None` outside `Mode::Deck` or with nothing awaiting and no limit passed.
    `KnowledgeSummaryCache` (a `Mutex<Option<(u64, usize, Option<KnowledgeSummary>)>>`, lock not
    held while hashing) keyed on `(knowledge_generation, approvals.count())`; a
    `#[cfg(test)]` computation counter.
  - Errors (keys `session-knowledge-*`): `KnowledgeIdNotFound { id }` ("No knowledge file has
    the id { $id }."), `KnowledgeChanged { path }` ("The knowledge file { $path } changed since
    it was shown: review it again."), `KnowledgeTooLargeToApprove { path }` ("The knowledge
    file { $path } is too large to approve: delete it."), `KnowledgeApprovalInvalid` ("This
    approval names a file twice or too many files.")
- Produces (UI): `knowledgeReview(): Promise<KnowledgeReview>`, `knowledgeText(id): Promise<KnowledgeText>`,
  `approveKnowledge(files: KnowledgeApproval[]): Promise<Snapshot>`; `KnowledgeLimitSlug =
  "too-many-files" | "file-too-large" | "total-too-large"`; `Snapshot.knowledge: KnowledgeSummary | null`.

- [ ] **Step 1: Failing tests** (through the `Request` match arm): review lists only awaiting
  files by path with label-cleaned path and bidi-stripped title; text whole, ASCII cut
  (`truncated = 10`) and a multi-byte char straddling the cut; text of an approved file;
  unknown id; approve two ids (index and `knowledge_file` agree, summary `None`); approve
  empty, same id twice, one valid plus one stale (nothing approved), already approved, a
  forged hash, a file over `MAX_FILE_BYTES`; merge between text fetch and approve →
  `KnowledgeChanged` key + `fr` (Review Focus 2); a move after approval keeps it; summary
  names `total-too-large` at limit + 1 and nothing at the limit, a bundle cut by the read
  ceiling; cache: a slide edit between two snapshots computes once, a knowledge write, undo or
  merge recomputes, an approve recomputes; pack, theme, help and history snapshots `None`;
  the three requests refused in a pack tab, in a theme tab and in the agent window;
  `request_class` cases; snapshot JSON has `knowledge`.
- [ ] **Step 2:** `RUSTC_WRAPPER= cargo nextest run -p oku-slides-core -p oku-slides-session -E 'test(/knowledge|request_class|agent_window|views/)'` → FAIL.
- [ ] **Step 3:** Implement. **Step 4:** PASS; `just check` (tsc sees the api types).
- [ ] **Step 5:** Commit `feat(session): approve received knowledge from the deck tab (t-1746)`.

### Task 6: Knowledge lot 3: UI notice and approval dialog (t-1747)

**Files:**
- Create: `ui/src/KnowledgeNotice.tsx`, `ui/src/KnowledgeDialog.tsx`,
  `ui/src/knowledgeText.ts` (`revealHidden`) + `knowledgeText.test.ts`,
  `ui/src/KnowledgeNotice.render.test.tsx`, `ui/src/KnowledgeDialog.render.test.tsx`
- Modify: `ui/src/App.tsx` (minimal: render the notice above the deck tab's slide view, the
  dialog when open, `knowledgeOpen` joins `dialogOpen`), `crates/oku-slides-i18n/src/locales/*/ui.ftl` ×20,
  `ui/ARCHITECTURE.md` if it lists dialogs

**Interfaces:**
- Consumes: Task 5 api and `Snapshot.knowledge`.
- `revealHidden(text: string): { text: string; hidden: number }`: every char matching
  `/[\p{Cf}\p{Co}\p{Cn}]|[^\P{Cc}\n\t]/gu` becomes `⟨U+XXXX⟩`.
- Notice (persistent banner, deck tab, hidden when `read_only` and while a received deck
  awaits review or the agent modal is open): `ui-knowledge-awaiting` = "{ $count ->
  [one] This deck holds one knowledge file you have not approved *[other] This deck holds {
  $count } knowledge files you have not approved }: the agent cannot read them." (only when
  awaiting > 0) with `ui-knowledge-review` = "Review…"; `ui-knowledge-over-limit` = "The
  knowledge base is past its limits ({ $limits }). Only deleting files brings it back under:
  ask the agent to delete some." (`$limits` = `Intl.ListFormat` of a literal slug →
  `ui-knowledge-limit-<slug>` table; an unknown slug throws). Review opens the dialog even
  when only over-limit is set.
- Dialog (`useModalFrame`, as `ReceivedDialog`): title `ui-knowledge-dialog-title` =
  "Received knowledge"; over-limit sentence on top when set; one row per file: path in
  `<bdi>`, title isolated, size `Math.ceil` KB (`ui-knowledge-size`), Show/Hide
  (`ui-knowledge-show`/`ui-knowledge-hide`, `aria-expanded`, one row open at a time) with the
  `<pre dir="auto">` of `revealHidden(text)` and `ui-knowledge-hidden-chars` = "{ $count }
  hidden characters are shown as ⟨U+…⟩." when any; Approve (`ui-knowledge-approve`) only once
  shown and not truncated, sending `{id, hash}` from the shown `KnowledgeText`; a truncated
  text says `ui-knowledge-too-large` = "This file is too large to approve: delete it."; a
  `more` count line `ui-knowledge-more`; Close (initial focus, Escape). One `busy` flag
  disables every button; replies to a stale request are ignored; after an approval (success or
  refusal, through `run`) the list is fetched once; an empty list with no over-limit closes.

- [ ] **Step 1: Failing tests** (read `docs/agents/ui-e2e.md` first): `revealHidden` on a tag
  char, a zero-width char, RLO, `\n` kept; notice: count with `[one]`/`[other]`, over-limit
  alone without the awaiting sentence, `fr` and `ja` limit lists, nothing for `null`;
  dialog: Approve absent before Show, Show renders `<script>` and `**bold**` literally,
  Approve sends the shown hash once, a second click while busy sends nothing, a truncated row
  has no Approve, a refusal shows the localized message and fetches the list exactly once
  more (2 calls), a stale reply is ignored, empty list closes, Escape closes.
- [ ] **Step 2:** `cd ui && bun test .render.test. Knowledge && bun test src/knowledgeText` → FAIL.
- [ ] **Step 3:** Implement. **Step 4:** PASS; `bun run test`; `just check`.
- [ ] **Step 5:** Commit `feat(ui): approve received knowledge from the deck tab (t-1747)`.

### Task 7: Knowledge lot 3: local agent write tools and eval run (t-1748)

**Files:**
- Modify: `crates/oku-slides-local-agent/src/families.rs:59` (Knowledge summary: "read,
  search or write the deck's knowledge base: notes, sources, references") +
  `families_tests.rs`, `eval/requests.json` (+2: « Note dans la base de connaissance que le
  client veut un ton sobre » → `knowledge_write`; "Save to the knowledge base that the budget
  is 40k" → `knowledge_write`), `eval_tests.rs:43` (`>= 36`, and a data test that both new
  requests expect `knowledge_write`), `docs/superpowers/plans/2026-11-01-lot-l-local-agent-eval.md`
  (new dated English section: run count, engine, seed)

- [ ] **Step 1:** Update; `RUSTC_WRAPPER= cargo nextest run -p oku-slides-local-agent` PASS.
- [ ] **Step 2: Eval run** (sandbox off, fresh temp dirs), as lot 2 Task 6. Pass: on the 34
  earlier requests route and first tool each at least lot 2's last recorded run minus 1 (the
  engine drift is t-1742's); the 2 new ones ≥ 1/2 each. On a miss, add "save or note facts"
  to the Knowledge summary only (≤ 3 runs); still missing: record it and go on. No local
  engine: record it and report the task blocked.
- [ ] **Step 3:** Record the runs. **Step 4:** Commit `test(local-agent): knowledge write tools in the eval (t-1748)`.

### Task 8: Knowledge lot 3: gate, review, land (t-1749)

- [ ] **Step 1:** `just ci` (timeouts on a loaded machine: rerun with `cargo nextest run
  --no-fail-fast`, each failure confirmed alone).
- [ ] **Step 2:** `/review` on `origin/main..HEAD`; fix every finding, minor included; commit
  fixes with `Reviewed-by:`.
- [ ] **Step 3:** `git fetch`, merge `origin/main` again if it moved (`bun install` in `ui/`
  if its deps changed), then land per the root `AGENTS.md` workflow; close the sub-tasks,
  t-1728 stays open.

## Deferred (backlog)

- Provenance stamping (`generated`, `x-oku-slides-agent`) with lot 4's extraction.
- Receipt entries for knowledge writes, with lot 5's tab.
- An `Arc` bundle (no clone per apply), the per-hash cache, after profiling;
  `knowledge_read` response size; t-1742.
- The knowledge tab's per-file Approve and awaiting badges (lot 5).
