# Deck knowledge base — lot 5: the knowledge tab — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The user reads a deck's knowledge base in a read-only `knowledge` tab linked to its
deck tab. The tab shows a tree, file sub-tabs, a frontmatter header and the rendered body. It
follows the agent's writes live, and the user approves awaiting files from it. The agent opens
it with `open_knowledge`.

**Architecture:** A knowledge tab is a workspace tab with no session of its own: a new
`TabDocument::Knowledge { deck_tab }` held in a new `Slot::View`. Like the deck-pack tab, it is
saved in `workspace.json` and restored only when its deck tab is. It closes with its deck tab.
Its `TabInfo` mirrors the owner's title. The owner deck tab's `TabInfo` carries a `knowledge`
revision, so every knowledge change reaches the windows through the existing workspace
broadcast.

The UI renders `KnowledgeTab` in place of `<App>` for such a tab. Every request goes to the
owner deck tab's session (`tabApi(deck_tab)`): `knowledge_tree`, `knowledge_page`,
`knowledge_asset`, and lot 3's review requests. `knowledge_asset` is the guarded asset route.
It answers only a raster image cited by an approved file, sniffed from its bytes. The bytes go
over the same request channel as everything else (`guard::check` in serve, IPC in the app).
This is a recorded deviation from the spec's wording: no new HTTP route.

A new `KnowledgeMarkdown` renders the body. It shares a tree walk with the agent's `Markdown`,
moved to a neutral file, with mandatory hooks for links, images and headings. It renders no raw
HTML, and every href goes through one classifier with a scheme allowlist.

**Tech Stack:** Rust, snafu, `saphyr-parser`, `base64` 0.22 (in `[workspace.dependencies]`;
Task 2 adds `base64.workspace = true` to `crates/oku-slides-session/Cargo.toml`, no new crate
in `Cargo.lock`), Fluent (20 locales). UI: React 19/TS, `@lezer/markdown` (already in
`ui/package.json`), Bun tests, happy-dom render tests, Playwright e2e. No new dependency.

**Parent task:** t-1728 stays open until lot 6. Sub-tasks: Task 1 t-1783, Task 2 t-1784,
Task 3 t-1785, Task 4 t-1786, Task 5 t-1787, Task 6 t-1788, Task 7 t-1789, Task 8 t-1761,
Task 9 t-1790.

**Spec:** `docs/superpowers/specs/2026-11-24-deck-knowledge-okf-design.md`, sections
« Knowledge tab » and « Plan decisions » (lots 1–4). Model plan:
`docs/superpowers/plans/2026-11-25-deck-knowledge-okf-lot4-documents.md`.

## Plan decisions (builder calls; Task 8 copies them into the spec)

### Tab model

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

### Live updates

- `TabInfo.knowledge: Option<KnowledgeRevision { generation: u64, approvals: usize }>`.
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
  re-reads on the next revision.

### Reading requests (deck session, user-only like lot 3's review)

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
  - `id` and `asset` must both be `sha256:` followed by 64 lowercase hex digits.
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

### Header

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

### Rendering

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
  - It strips ASCII whitespace and control characters anywhere, then lowercases the part
    before the first `:`. Lezer does not decode entities or escapes in link destinations:
    `resolveLink` decodes CommonMark escapes, numeric references and a fixed set of named
    references itself first.
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
- Every attribute value (title, aria-label) comes from Fluent, never from file text.

### Opening

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

### Deferred

- Receipt entries for knowledge writes, and documents shown in sent turns (lot 3 and lot 4
  deferrals): a new sub-task under t-1728, created in Task 9, named "Knowledge: receipts for
  writes, documents in sent turns".
- Lot 6: export and CLI.

## Global Constraints

- Every user-facing string goes through Fluent, in `en` and the 19 other locales (ar bn de es
  fr hi it ja ko nl pl pt ru th tr uk vi zh-Hans zh-Hant). `fr` keys are in the same order as
  `en`. UI keys are used literally (`i18n-keys.test.ts`).
- Localized errors are tested on their key or variant plus one non-English locale.
- Every new `Request` variant is mirrored in `ui/src/api.ts`, and every `WorkspaceRequest`
  variant in `ui/src/workspaceApi.ts`, with zod where the file already uses it.
- Imported assets are untrusted bytes: sniff them, never trust an extension, and never put
  them in the DOM except as `blob:` `<img>`.
- Never hold a std `Mutex` guard across an await. A session is only `try_lock`ed under the
  workspace lock.
- Rust tests go in sibling `_tests.rs` files, with speculoos and rstest cases. Test names
  follow `<unit>_<scenario>_<expected>`. Use no thiserror and no chrono.
- TS tests are side by side, with sentence names (`<unit> <scenario> <expected>`). New comments
  are in English. New types live in `ui/src/knowledge/`, with named re-exports only.
- UI: modals use `useModalFrame`, menus use `ContextMenu`, buttons use `primitives.tsx`, ids
  come from `randomId`, and effects must survive a StrictMode remount. Read
  `docs/agents/ui-e2e.md` before any render test or e2e.
- Every app or e2e run sets `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh
  temp dir.
- Each touched `ARCHITECTURE.md` stays at 50 lines or fewer (session and mcp are at 50, core at
  49: compress) and is updated in the commit that changes its code. `ui/ARCHITECTURE.md` is
  updated in Task 6. Files stay under 1000 lines (target 500).
- The app crate is tested with `just test` (it excludes `oku-slides-app`, so check the host
  compiles with `cargo check -p oku-slides-app --tests`).
- Implementers stage only. The controller commits with a message file in
  `.superpowers/sdd/knowledge-okf-lot5/` and `git commit -F`. Messages follow Conventional
  Commits with the task ref, with the trailer `Reviewed-by: sdd task review (spec, quality)`,
  and no `Co-Authored-By`.
- Snapshots are accepted with `INSTA_UPDATE=always` followed by a `git diff` check, never with
  the interactive `cargo insta review`.

## Review Focus

1. **Owner closed while the knowledge tab is open, or the knowledge tab moved to another window
   first.** Closing the deck tab closes the knowledge tab wherever it is, and requests in
   flight resolve to `UnknownTab`. Tested in Task 3 (`close_owner_in_other_window_closes_
   knowledge`, `open_knowledge_owner_closed_meanwhile_unknown_tab`).
2. **Hostile links and images in imported text.** Cases: `javascript:`, `JaVaScRiPt:`,
   `java\tscript:`, `&#106;avascript:`, `<javascript:…>`, a reference definition to
   `javascript:`, `//evil.com/x.md`, `\\host`, `%2e%2e/`, a NUL, `/a/../../x`, raw `<a href>`
   HTML, `![x](https://tracker)`, an SVG asset, and a bare URL. None makes a live link or a
   fetch. Tested in Task 5.
3. **A file deleted or moved while its sub-tab is open, or a page reply arriving after the file
   is gone.** The sub-tab closes with one toast, and no error follows from the late reply. A
   move keeps the sub-tab. Tested in Task 6.
4. **Restore from a hostile or stale `workspace.json`.** Cases: the owner failed to wake, the
   owner is not a deck, there are two knowledge tabs for one owner, or the owner is itself a
   knowledge tab. Each is dropped, with no `state.json` read. Tested in Task 3.
5. **A huge bundle, a file at `MAX_FILE_BYTES`, an 8 MiB+ asset, deep directories, an agent
   writing many files in a row.** The tree stays usable, the body is cut with
   `ui-knowledge-truncated`, the asset shows its alt text, and re-reads coalesce. Tested in
   Tasks 2 and 6.

---

### Task 1: Core knowledge header (t-1783)

**Files:**
- Modify: `crates/oku-slides-core/src/knowledge/frontmatter.rs` (`Node::Seq(Vec<Node>)`,
  `Open::Seq(Vec<Node>)`)
- Create: `crates/oku-slides-core/src/knowledge/header.rs`, `header_tests.rs`
- Modify: `crates/oku-slides-core/src/knowledge/mod.rs`, `crates/oku-slides-core/src/lib.rs`
  (re-export), core `ARCHITECTURE.md`

**Interfaces:**
- Produces:
  - `pub struct KnowledgeHeader { pub kind: Option<String>, pub title: Option<String>, pub
    description: Option<String>, pub status: Option<String>, pub stale: bool, pub tags:
    Vec<String>, pub generated_by: Option<String>, pub generated_at: Option<String>, pub
    verified: Option<String>, pub sources: Vec<KnowledgeSource> }`
  - `pub struct KnowledgeSource { pub resource: Option<String>, pub title: Option<String>, pub
    url: Option<String> }`
  - `pub const MAX_HEADER_ITEMS: usize = 20;`
  - `pub fn knowledge_header(text: &str) -> Result<KnowledgeHeader, FrontmatterReason>`
  - `pub fn body(text: &str) -> &str`: the text after a terminated frontmatter block, or the
    whole text when there is none or it is unterminated. It never parses YAML; Task 2 decides
    from `knowledge_header`'s result whether to use it.

- [ ] **Step 1: Write the failing tests** in `header_tests.rs`:
  - `knowledge_header_full_block_reads_every_field`: a block with type, title, status, `tags:
    [a, b]`, `generated: {by: x, at: 2026-11-27T10:00:00Z}`, `verified: {tier: gold}`,
    `stale: true` and two sources (one map, one scalar) gives exactly those values.
  - `knowledge_header_lone_scalar_tags_gives_one_tag`.
  - `knowledge_header_nested_tag_item_skipped`: `tags: [a, {b: c}, [d]]` gives `["a"]`.
  - `knowledge_header_empty_tags_empty`: `tags: []`.
  - `knowledge_header_non_bool_stale_false`: `stale: "yes"`.
  - `knowledge_header_items_capped_at_twenty`: 25 tags give 20.
  - `knowledge_header_source_without_fields_dropped`: `sources: [{x: 1}]` gives none.
  - `knowledge_header_alias_refused`: `FrontmatterReason::Alias`, as `frontmatter`.
  - `knowledge_header_bom_and_crlf_read`.
  - `body_without_frontmatter_whole_text`.
  - `body_after_fence_rest`.
  - `body_unterminated_fence_whole_text`.
  - `body_later_rule_kept`: text with a `---` line inside the body keeps it.
  - Every existing `frontmatter_tests` keeps passing (the `Seq` change must not alter them).
- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-core header` and expect it to FAIL
  (unresolved names).
- [ ] **Step 3: Implement.** Collect the items of `Open::Seq` into `Node::Seq`, stopping past
  `MAX_HEADER_ITEMS`. Read fields with `Node::text`. `body` reuses `split`.
- [ ] **Step 4: Run** `cargo nextest run -p oku-slides-core` and `just check`, and expect
  PASS.
- [ ] **Step 5: Stage** the files. The controller commits `feat(core): read the knowledge
  header for display (t-1783)`.

### Task 2: Session reading requests and the live revision (t-1784)

**Files:**
- Create: `crates/oku-slides-session/src/knowledge_read.rs`, `knowledge_read_tests.rs`
- Modify:
  - `crates/oku-slides-session/Cargo.toml`: `base64.workspace = true`.
  - `request.rs`: variants `KnowledgeTree`, `KnowledgePage { id }`,
    `KnowledgeAsset { id, asset }`, plus dispatch.
  - `request_class.rs`: `reviews_deck_knowledge` covers the three, and `changes_deck` lists
    them as `false`.
  - `agent_window_allow.rs` (beside the lot 3 variants at lines 120-122) and
    `crates/oku-slides-cli/src/serve.rs` (the agent-window allowlist near line 223): refuse
    the three.
  - `knowledge_review.rs`: make `ensure_knowledge_review` `pub(crate)`.
  - `error.rs`: `KnowledgeAssetUnavailable { asset }`.
  - `views.rs`: `TabInfo.knowledge`, `KnowledgeRevision`. Set `knowledge: None` in
    `pack_tab_info` and `theme_tab_info`.
  - The deck `TabInfo` builder (`tab_info`), and `shown_same`.
  - `workspace_session.rs`: a change of `knowledge` alone broadcasts without `save_layout`.
  - `workspace_resume.rs`: `SavedTabInfo` excludes `knowledge`.
  - `crates/oku-slides-i18n/src/locales/*/session.ftl`: `session-knowledge-asset-unavailable`.
  - `ui/src/api.ts`: zod types and `TabApi.knowledgeTree/knowledgePage/knowledgeAsset`, and
    `TabInfo.knowledge`.
  - `ui/src/knowledge/types.ts`: the TS types, re-exported by name.
  - Session `ARCHITECTURE.md` and `AGENTS.md` (the user-only reading rule).

**Interfaces:**
- Consumes: `knowledge_header`, `body` (Task 1); `ensure_knowledge_review`, `knowledge_scan`,
  `knowledge_files`, `knowledge_text`, `cited_images`, `media::sniff`, `label`,
  `label_within`, `cut`.
- Produces (serde `Serialize`, snake_case):
  - `KnowledgeTree { files: Vec<KnowledgeNode> }`
  - `KnowledgeNode { id: String, path: String, title: Option<String>, kind: Option<String>,
    awaiting: bool }`
  - `KnowledgePage { id: String, path: String, awaiting: bool, header:
    Option<KnowledgeHeaderView>, body: String, truncated_bytes: usize, images:
    Vec<KnowledgeImage> }`
  - `KnowledgeImage { path: String, asset: String }`
  - `KnowledgeHeaderView`: the fields of `KnowledgeHeader`, cleaned.
  - `KnowledgeAsset { mime: &'static str, data: String }`
  - `KnowledgeRevision { generation: u64, approvals: usize }`, with
    `#[derive(Serialize, PartialEq, Eq, Clone, Copy, Debug)]`.
  - `const MAX_KNOWLEDGE_ASSET_BYTES: usize = 8 * 1024 * 1024;`
- Produces in TS:
  - `api.knowledgeTree(): Promise<KnowledgeTree>`
  - `api.knowledgePage(id: string): Promise<KnowledgePage>`
  - `api.knowledgeAsset(id: string, asset: string): Promise<KnowledgeAsset>`
  - `TabInfo.knowledge?: { generation: number; approvals: number } | null`

- [ ] **Step 1: Write the failing tests** in `knowledge_read_tests.rs`:
  - `knowledge_tree_lists_every_file_with_awaiting_flag`: an awaiting node has no title or
    kind.
  - `knowledge_tree_reuses_scan_cache`: two calls at one generation leave `computations()` at
    one.
  - `knowledge_page_splits_header_and_body`.
  - `knowledge_page_unparsable_frontmatter_whole_text_as_body`.
  - `knowledge_page_awaiting_whole_text_no_header_no_images`.
  - `knowledge_page_past_max_bytes_truncated_counted`.
  - `knowledge_page_lists_cited_images`.
  - `knowledge_page_unknown_id_refused`: `KnowledgeIdNotFound`.
  - `knowledge_asset_cited_png_answers_base64`.
  - Refusal cases, as one rstest, each `KnowledgeAssetUnavailable`:
    - uncited;
    - awaiting file;
    - an asset cited only by an awaiting file B, asked through an approved file A;
    - an approved file edited since, so now awaiting;
    - SVG;
    - missing;
    - past `MAX_KNOWLEDGE_ASSET_BYTES`;
    - a malformed `id` or `asset`.
  - `knowledge_asset_unavailable_localized_fr`.
  - `knowledge_requests_in_history_refused`.
  - `knowledge_requests_help_tab_refused`.
  - `tab_info_knowledge_changes_on_write_and_on_approval`.
  - `tab_info_knowledge_none_outside_deck`.
  - `tab_info_knowledge_in_history_reads_live_deck`.
  - `tab_info_knowledge_dormant_none_and_not_woken`.
  - `workspace_view_serializes_knowledge_saved_layout_omits_it`.
  - `knowledge_revision_change_alone_does_not_save_layout`.
  - In `request_class_tests`: `knowledge_reading_requests_change_no_deck`.
  - In `agent_window_allow_tests`: `knowledge_reading_requests_refused_to_agent_window` (three
    cases).
  - In `serve_tests`: the three are refused to the agent window.
  - In `ui/src/api.test.ts`: the zod parse of a page and a tree.
- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-session knowledge_read` and expect it
  to FAIL.
- [ ] **Step 3a: Implement the reads** in `knowledge_read.rs`: `Session::knowledge_tree()`,
  `knowledge_page(id)` and `knowledge_asset(id, asset)`. Encode with
  `base64::engine::general_purpose::STANDARD`, after every check.
- [ ] **Step 3b: Implement the revision** in `views.rs`, `tab_info`, `shown_same`, `note` and
  `SavedTabInfo`.
- [ ] **Step 3c: Implement the classifiers** (`request_class`, `agent_window_allow`, serve) and
  the TS mirror.
- [ ] **Step 3d: Add the Fluent key** in all 20 `session.ftl`.
- [ ] **Step 4: Run** `just test` (in the background on a cold build) and `just check`, and
  expect PASS.
- [ ] **Step 5: Stage.** The controller commits `feat(session): knowledge tree, page and asset
  requests (t-1784)`.

### Task 3: Workspace knowledge tab (t-1785)

**Files:**
- Create: `crates/oku-slides-session/src/workspace_knowledge.rs`,
  `workspace_knowledge_tests.rs`
- Modify:
  - **Kinds.** `tab_document.rs`: `Knowledge { deck_tab }`, with `kind()` →
    `DocumentKind::Knowledge` and `deck_tab()` covering it. `document_kind.rs`: `Knowledge`,
    its docs, and its arms in `ensure_fits` and `ensure_caller_ops`, which refuse it.
  - **The kind's consumers**, every exhaustive match:
    - `crates/oku-slides-mcp/src/tool_kinds.rs` (`Kinds`: no tool for knowledge);
    - `crates/oku-slides-agent/src/instructions.rs` (lines 41 and 53) and `usage_board.rs`
      (line 128);
    - `workspace_usage.rs:35`;
    - `crates/oku-slides-local-agent/src/families.rs:146`;
    - `ui/src/tabKind.ts` `documentKind()`, and the zod `TabDocument` in `ui/src/api.ts`.
  - **Fluent.** `session.ftl` in all 20 locales: a `[knowledge]` branch of
    `session-wrong-document`.
  - **Slot.** `workspace_slot.rs`: `Slot::View`, `Tab::view(info)`. `ready()` returns
    `WrongDocument { kind: Knowledge }`. Settle, wake, set_aside, `close_all`, compaction,
    usage, UI-state saves and the same-deck check skip it.
  - **Request.** `workspace_request.rs`: `OpenKnowledge { tab }`. Classify it in
    `src-tauri/src/commands.rs:48-65`, `request_class`, `agent_window_allow.rs` (refused to
    the agent window) and serve's allowlist.
  - **Close.** `workspace_deck_packs_close.rs`: the owner's close also closes
    `knowledge_tab_of(owner)`. `linked_tabs` is unchanged.
  - **Restore.** `workspace_resume.rs`: knowledge entries are handled before `restore_tab`.
  - **Title.** `workspace_session.rs`: `note` mirrors the owner title onto its knowledge tab.
  - **Agent port.** `agent/workspace_port.rs`: `open_knowledge`, defaulting to `Unavailable`.
    Update the test fakes in `crates/oku-slides-mcp/src/*_tests.rs` and
    `crates/oku-slides-agent` that implement `AgentWorkspace`, if any break.
  - `workspace_agent.rs`: the agent form.
  - `ui/src/workspaceApi.ts`: the `TabDocument` union, `tabPath` (null for knowledge),
    `openKnowledge`, `sessionTab`.
  - Session `ARCHITECTURE.md` and `AGENTS.md`.

**Interfaces:**
- Produces:
  - `Workspace::open_knowledge(ws: &Mutex<Self>, deck_tab: &str) -> Result<(String, bool)>`:
    the knowledge tab's id and whether it is new. It creates the tab, or focuses the existing
    one, placed `TabPlace::After(deck_tab)`. It takes only the workspace lock.
  - `Workspace::knowledge_tab_of(&self, owner: &str) -> Option<String>`.
  - The agent form: `Workspace::agent_open_knowledge(ws, tab, gate) -> Result<TabOpen,
    WorkspaceRefusal>`, following `agent_activate`'s gate pattern. It is shown only if `tab`
    is shown.
  - `AgentWorkspace::open_knowledge(&self, own: &str, gate: &mut dyn FnMut(TabGate) -> bool)
    -> Result<TabOpen, WorkspaceRefusal>`.
  - `WorkspaceRequest::OpenKnowledge { tab }` (`cmd: "open_knowledge"`).
- Produces in TS:
  - `workspace.openKnowledge(tab: string): Promise<WorkspaceView>`
  - `sessionTab(view: WorkspaceView, tab: string): string`: the owner for a knowledge tab,
    else `tab`.

- [ ] **Step 1: Write the failing tests** in `workspace_knowledge_tests.rs`:
  - `open_knowledge_new_placed_after_deck_tab`.
  - `open_knowledge_twice_focuses_same_tab`.
  - `open_knowledge_from_help_tab_refused_wrong_document`.
  - `open_knowledge_owner_closed_meanwhile_unknown_tab`.
  - `knowledge_tab_session_request_refused_wrong_document`.
  - `wrong_document_knowledge_localized_fr`.
  - `knowledge_tab_title_follows_owner_rename`.
  - `close_owner_closes_knowledge`.
  - `close_owner_in_other_window_closes_knowledge`.
  - `close_knowledge_alone_keeps_owner_and_never_prompts`.
  - `settle_wake_set_aside_close_all_skip_view_slot`.
  - `knowledge_tab_never_dirty_and_skipped_by_compaction`.
  - `knowledge_tab_round_trips_workspace_json`.
  - Restore cases, as one rstest:
    - `restore_knowledge_with_deck_owner`.
    - `restore_knowledge_owner_not_deck_dropped`.
    - `restore_knowledge_owner_failed_dropped_no_state_read`.
    - `restore_second_knowledge_same_owner_dropped`.
    - `restore_knowledge_owner_is_knowledge_dropped`.
  - `saved_knowledge_tab_has_no_legacy_keys` (the downgrade check).
  - `agent_open_knowledge_gate_refused_opens_nothing`.
  - `agent_open_knowledge_unshown_requester_unshown`.
  - `tab_list_knowledge_kind`.
  - In `agent_window_allow_tests`: `open_knowledge_refused_to_agent_window`.
  - In `src-tauri/src/commands_tests.rs`: an `open_knowledge` case.
  - In `ui/src/workspaceApi.test.ts`: `sessionTab` and `tabPath` for a knowledge tab.
- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-session workspace_knowledge` and expect
  it to FAIL.
- [ ] **Step 3a: Implement the kind and its consumers.** Add the `TabDocument` variant and the
  `DocumentKind`, then fix every match the compiler names. Add the Fluent branch.
- [ ] **Step 3b: Implement the `Slot::View` skips.**
- [ ] **Step 3c: Implement open and close.** `workspace_knowledge.rs` holds `open_knowledge`,
  `knowledge_tab_of` and the close hook.
- [ ] **Step 3d: Implement restore and the title mirror.**
- [ ] **Step 3e: Implement the agent form and the port default.**
- [ ] **Step 3f: Implement the request and its classifiers, and the TS side.**
- [ ] **Step 4: Run** `just test` (background), `cargo check -p oku-slides-app --tests` and
  `just check`, and expect PASS.
- [ ] **Step 5: Stage.** The controller commits `feat(session): knowledge tab linked to its
  deck tab (t-1785)`.

### Task 4: `open_knowledge` agent tool (t-1786)

**Files:**
- Modify:
  - `crates/oku-slides-mcp/src/hub_turns.rs`: `TabTarget::Knowledge`, Display `knowledge`.
  - `tools.rs`: the description, `Kinds::DECK`.
  - `handler.rs`: dispatch.
  - `knowledge_tools.rs`: `open_knowledge`.
  - `crates/oku-slides-local-agent/src/families.rs`: the Knowledge family.
  - `src-tauri/src/app_workspace.rs`: `open_knowledge`, calling Task 3 and then `follow`
    (`src-tauri/src/state.rs:99`).
  - The serve-side `AgentWorkspace` in `crates/oku-slides-cli`, if it implements the port.
  - The snapshot files: `tools_tests.rs`, `tool_kinds_tests.rs` and the families snapshot.
    Their diff adds only `open_knowledge`.
  - `crates/oku-slides-agent/src/instructions/knowledge.md`: one sentence, "Show the user the
    base with open_knowledge when they ask to see it."
  - The MCP `ARCHITECTURE.md`.

**Interfaces:**
- Consumes: `AgentWorkspace::open_knowledge` (Task 3), `TabVerdict`.
- Produces: the tool `open_knowledge`, no args, replying `{ok, tab, shown}`.

- [ ] **Step 1: Write the failing tests**:
  - `open_knowledge_deck_tab_opens_and_claims_move`.
  - `open_knowledge_second_move_refused_tab_gate`.
  - `open_knowledge_not_listed_for_help_pack_theme`.
  - `open_knowledge_no_turn_refused`.
  - `open_knowledge_unavailable_workspace_refused`.
  - A local-agent families test listing `open_knowledge` under Knowledge.
- [ ] **Step 2: Run** `cargo nextest run -p oku-slides-mcp open_knowledge` and expect it to
  FAIL.
- [ ] **Step 3: Implement** after the pattern of `help` (`workspace_tools.rs:252`), without
  the paging.
- [ ] **Step 4: Run** the MCP, local-agent and agent suites with `INSTA_UPDATE=always`, check
  `git diff` on the snapshots, then run `just check`. Expect PASS.
- [ ] **Step 5: Stage.** The controller commits `feat(mcp): open_knowledge shows the
  knowledge tab (t-1786)`.

### Task 5: Knowledge Markdown renderer (t-1787)

**Files:**
- Create:
  - `ui/src/markdownRender.tsx`: the shared walk, moved from `ui/src/agent/Markdown.tsx`.
  - `ui/src/knowledge/KnowledgeMarkdown.tsx`.
  - `ui/src/knowledge/knowledgeLinks.ts`, `knowledgeLinks.test.ts`.
  - `ui/src/knowledge/KnowledgeMarkdown.render.test.tsx`.
- Modify: `ui/src/agent/Markdown.tsx` (uses the shared walk; `rel="noopener noreferrer"`),
  `ui/src/styles.css`.

**Interfaces:**
- Produces:
  - `renderMarkdown(text: string, parser: MarkdownParser, hooks: MarkdownHooks): ReactNode`,
    with `MarkdownHooks { link(href: string, label: ReactNode, key: string): ReactNode;
    url(href: string, key: string): ReactNode; image(src: string, alt: string, key: string):
    ReactNode; heading(level: 1|2|3|4|5|6, children: ReactNode, key: string): ReactNode }`.
  - `resolveLink(href: string, from: string, paths: ReadonlySet<string>): { kind: "external";
    url: string } | { kind: "bundle"; path: string } | { kind: "broken"; path: string } |
    { kind: "inert" }`.
  - `resolveBundlePath(target: string, from: string): string | null`, shared by links and
    images.
  - `KnowledgeMarkdown({ text, from, paths: ReadonlySet<string>, onOpen(path: string),
    image(path: string): string | null })`. `image` receives the resolved bundle path and
    returns a `blob:` URL, or `null` for alt text.

- [ ] **Step 1: Write the failing tests**:
  - `resolveLink` cases, one rstest-style `test.each`:
    - `https://x` and `mailto:a@b` are external.
    - `JaVaScRiPt:`, ` javascript:`, `java\tscript:`, `&#106;avascript:` (as lezer hands it),
      `data:`, `file:` and `vbscript:` are inert.
    - `//evil.com/x.md`, `\\host\x`, `%2e%2e/x.md`, `a%00b.md` and `/a/../../x` are inert.
    - `./b.md` from `/a/x.md` is `/a/b.md`; `../c.md` is `/c.md`; `/a/b.md#h` is `/a/b.md`.
    - `a%20b.md` decodes to `/a/a b.md`.
    - A path absent from `paths` is `broken`.
  - Render tests:
    - `knowledge markdown gfm table rendered`.
    - `knowledge markdown task list stays text`.
    - `knowledge markdown raw html literal text`.
    - `knowledge markdown javascript link plain text`.
    - `knowledge markdown angle bracket javascript link plain text`.
    - `knowledge markdown reference link resolved like inline`.
    - `knowledge markdown bare url stays text`.
    - `knowledge markdown external link opens in new window`: `target="_blank"`, `rel`.
    - `knowledge markdown bundle link calls on open`.
    - `knowledge markdown missing bundle link broken`.
    - `knowledge markdown remote image alt only`.
    - `knowledge markdown asset image blob src`.
    - `knowledge markdown headings shifted and clamped at h6`.
  - The agent `Markdown` render tests still pass unchanged, except the `rel` value.
- [ ] **Step 2: Run** `cd ui && bun test knowledgeLinks` and the render test, and expect FAIL.
- [ ] **Step 3a: Move the walk** to `markdownRender.tsx` with mandatory hooks, and port the
  agent `Markdown` to it.
- [ ] **Step 3b: Implement** `knowledgeLinks.ts` and `KnowledgeMarkdown.tsx`.
- [ ] **Step 4: Run** `bun test` (and the render tests) and `just check`, and expect PASS.
- [ ] **Step 5: Stage.** The controller commits `feat(ui): knowledge Markdown renderer with
  sanitized links (t-1787)`.

### Task 6: Knowledge tab view (t-1788)

**Files:**
- Create:
  - `ui/src/knowledge/KnowledgeTab.tsx`: the layout, live revision, sub-tabs, toast and
    `KnowledgeDialog`.
  - `ui/src/knowledge/KnowledgeTreeView.tsx`: the tree.
  - `ui/src/knowledge/KnowledgeHeaderView.tsx`.
  - `ui/src/knowledge/knowledgeTree.ts`, `knowledgeTree.test.ts`: building the tree and the
    sort.
  - `ui/src/knowledge/useKnowledgeAssets.ts`, `useKnowledgeAssets.test.ts`.
  - `ui/src/stripKeyNav.ts`, `stripKeyNav.test.ts`: extracted from `WidgetTab.tsx:292`.
  - Render tests.
- Modify:
  - `ui/src/WidgetTab.tsx`: use `stripKeyNav`.
  - `ui/src/styles.css`.
  - `crates/oku-slides-i18n/src/locales/*/ui.ftl`, for the `ui-knowledge-*` keys:
    `-tab-title`, `-tree-label`, `-empty`, `-awaiting`, `-review`, `-file-gone`,
    `-link-broken`, `-truncated`, `-in-history`, `-header-type`, `-header-status`,
    `-header-tags`, `-header-generated`, `-header-verified`, `-header-stale`,
    `-header-sources`, `-close-file`, `-open`.
  - `ui/ARCHITECTURE.md`; `ui/AGENTS.md` if a rule is new.

**Interfaces:**
- Consumes: Task 2's TabApi methods and types; Task 5's `KnowledgeMarkdown` and
  `resolveBundlePath`; lot 3's `KnowledgeDialog` (`{ api, run, onError, onClose }`);
  `Latest`; `revealHidden`.
- Produces: `KnowledgeTab({ api: TabApi, revision: { generation: number; approvals: number } |
  null, say: (message: string) => void, run: Run })`, where `api` is `tabApi(deck_tab)`.
- Produces: `buildTree(files: KnowledgeNode[], locale: string): TreeDir`, with these rules:
  - Directories come from paths.
  - `index.md` is the directory's own page, shown as the directory row.
  - Directories come before files.
  - Files are sorted by title, then path, using `localeCompare` with the UI locale. A file with
    no title sorts by its file name.
- Produces: `stripKeyNav(event, index, count, rtl): number | null`.

- [ ] **Step 1: Write the failing tests**:
  - `buildTree`: `index as dir page`, `sorted by title then path`, `untitled uses name`,
    `deep paths nested`.
  - `stripKeyNav`: arrows, Home, End, and RTL.
  - `useKnowledgeAssets`:
    - `asset image survives strict remount`;
    - `revokes its urls on cleanup`;
    - `late reply after unmount dropped`;
    - `decode failure marked failed`;
    - `cleared on approvals change`.
  - Render tests:
    - `knowledge tab opens root index first`.
    - `knowledge tab tree click opens subtab`.
    - `knowledge tab second click focuses subtab`.
    - `knowledge tab header shows fields`.
    - `knowledge tab header url is text`: a `javascript:` source url makes no link.
    - `knowledge tab header proto key harmless`: a `__proto__` tag shows as text.
    - `knowledge tab awaiting badge and pre text`.
    - `knowledge tab awaiting path bidi stripped`.
    - `knowledge tab review opens dialog`.
    - `knowledge tab new revision rereads tree and active page only`.
    - `knowledge tab null revision rereads nothing`.
    - `knowledge tab rereads coalesce`: five revisions during one read give two reads.
    - `knowledge tab deleted file closes subtab one toast`: the late page reply is ignored.
    - `knowledge tab moved file keeps subtab`.
    - `knowledge tab in history shows notice no toast`.
    - `knowledge tab truncated notice`.
    - `knowledge tab strip arrow keys rtl`.
- [ ] **Step 2: Run** the new tests, and expect FAIL.
- [ ] **Step 3a: Extract `stripKeyNav`** and port `WidgetTab.tsx` to it.
- [ ] **Step 3b: Implement** the tree, header, asset hook and tab.
- [ ] **Step 3c: Add the keys** to all 20 `ui.ftl` (the `fr` order matches `en`).
- [ ] **Step 4: Run** `bun test`, the render tests, `i18n-keys.test.ts` and `just check`, and
  expect PASS.
- [ ] **Step 5: Stage.** The controller commits `feat(ui): knowledge tab view (t-1788)`.

### Task 7: Workspace wiring in the UI (t-1789)

**Files:**
- Modify:
  - `ui/src/tabKind.ts`: `"knowledge"`.
  - `ui/src/Workspace.tsx`. For a knowledge tab, it renders `KnowledgeTab` with
    `tabApi(deck_tab)`, the owner's `info.knowledge`, and the `say` of a `useToast` it owns
    (rendering its `node`). It handles `tabShortcut` `knowledge` on the active deck tab.
  - `ui/src/tabs.ts`: ⌘⇧K; `othersToClose` treats a knowledge tab as linked to its owner.
  - `ui/src/closeFlow.ts`: a knowledge tab closes with no snapshot read and no prompt; closing
    its owner closes it with one prompt (the owner's).
  - `ui/src/TabBar.tsx`: the title through `ui-knowledge-tab-title`, the icon, and the
    context-menu entry on deck tabs.
  - `ui/src/AppMenu.tsx`: the entry.
  - `ui/src/agent/target.ts`: `agentTarget` maps through `sessionTab`.
  - The `useVoiceContext` call: the owner tab.
  - `src-tauri/capabilities/default.json` and `agent.json`: `opener:allow-open-url` scoped to
    `http://*`, `https://*` and `mailto:*`.
  - The help content if it lists shortcuts (`docs/AGENTS.md` says where).
  - The 20 `ui.ftl`.
- Test: `tabs.test.ts`, `target.test.ts`, the `closeFlow` tests, and the `TabBar` / `AppMenu`
  render tests.

**Interfaces:**
- Consumes: `workspace.openKnowledge`, `sessionTab` (Task 3), `KnowledgeTab` (Task 6).

- [ ] **Step 1: Write the failing tests**:
  - `tab shortcut cmd shift k knowledge`.
  - `tab shortcut cmd k not knowledge` (⌘K stays the agent's typing shortcut).
  - `agent target knowledge tab maps owner`.
  - `close deck with knowledge closes both one prompt`.
  - `close knowledge alone no snapshot read`.
  - `others to close keeps knowledge with its owner`.
  - `tab bar knowledge title marked`.
  - `app menu open knowledge calls workspace`.
  - In `src-tauri/src/commands_tests.rs` or a capabilities test: the opener permission is
    scoped to exactly those three schemes.
- [ ] **Step 2: Run** them, and expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `bun test`, the render tests, `cargo check -p oku-slides-app --tests`
  and `just check`, and expect PASS.
- [ ] **Step 5: Stage.** The controller commits `feat(ui): open the knowledge tab from the
  menu, ⌘⇧K and the tab bar (t-1789)`.

### Task 8: e2e attach, import, read (t-1761)

**Files:**
- Create:
  - `ui/e2e/460-knowledge-tab.cjs`.
  - `ui/e2e/lib/pdf.cjs`: builds a valid one-page text PDF in memory, with correct xref
    offsets, a Helvetica `Tj` and the text "Knowledge e2e marker". No binary fixture is added.
  - A Rust test in `crates/oku-slides-extract` that the same PDF bytes (written as a test
    helper in Rust, same layout) extract to text holding the marker.
- Modify:
  - `fixtures/agent/e2e.json`: a `by_prefix` turn keyed `import knowledge ` calling
    `knowledge_import { "attachment": "${ARG}" }`, then `open_knowledge`. The spec types the
    prompt `import knowledge sha256:<hex>`, with the hex computed from the PDF bytes as
    `450-attach-document.cjs` does. No fake-agent change is needed.
  - The spec: copy this plan's « Plan decisions » into the spec's Plan decisions, as "Lot 5".
  - `docs/agents/ui-e2e.md` if a helper is new.

- [ ] **Step 1: Write the spec.** The UI runs in the French locale, as the other specs do, so
  select by role, test id or class, never by English text.
  1. Open a deck.
  2. Attach the PDF through `.draft-pick` with `setInputFiles({ name: "notes.pdf",
     mimeType: "application/pdf", buffer })`.
  3. Type and send the prompt.
  4. Wait for the fake agent log to show `knowledge_import`, then `open_knowledge`, in that
     order (`logOffset`).
  5. Expect a tab with the knowledge marker class.
  6. Expect the tree to hold `notes.md` under `references`. Click it.
  7. Expect the header's type `Reference`, and the body to contain "Knowledge e2e marker".
  8. Close the deck tab, and expect the knowledge tab gone, with no dialog.
  9. Expect no page error during the whole spec.
- [ ] **Step 2: Run** `just e2e 4999` with fresh `OKU_SLIDES_PATHS__*`, and expect the new spec
  and every other spec to PASS.
- [ ] **Step 3: Stage.** The controller commits `test(e2e): attach a PDF, import it, read it in
  the knowledge tab (t-1761)`.

### Task 9: Gate, review, land (t-1790)

- [ ] **Step 1: Run `just ci`.** If tests time out on a loaded machine, rerun with
  `cargo nextest run --no-fail-fast`, then run each failure alone.
- [ ] **Step 2: Review.** Run `/review origin/main..HEAD` and fix every finding, minor ones
  included. The controller commits the fixes with the same `Reviewed-by:` trailer.
- [ ] **Step 3: Land, per AGENTS.md.**
  1. `git fetch`; merge `origin/main` if it moved, and run `bun install` in `ui/` if its deps
     changed; rerun `just ci` after a merge.
  2. On main (`~/Projects/slider`): `git merge --squash knowledge-tab-559d0f`, then commit
     with `-F` and the trailer. Check that `git diff --quiet knowledge-tab-559d0f HEAD`
     passes.
  3. `git fetch` again; if main moved, start over at step 1. Push.
  4. Remove the worktree and the branch.
- [ ] **Step 4: Tasks.** Create the deferred sub-task "Knowledge: receipts for writes,
  documents in sent turns" under t-1728. Close t-1783 to t-1790 and t-1761 with `oku cli task done`.
  t-1728 stays open.
