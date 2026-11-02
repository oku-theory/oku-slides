# Agent working directory and received packs validation — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** give each tab's agent an empty app-owned working directory (t-1413), and install the
packs and themes a deck brings only after the user validates them in one dialog that shows a
capability manifest (t-1414), with a regression test that the render stays offline.

**Architecture:** Part 1 moves the ACP `cwd` from the deck folder to `tabs/<tab>/work/` in the
tab's resume directory (Storage), and the subprocess's cwd to `<data>/agent-process/`. Part 2
splits `receive_packs` into a pure listing (`received_to_review`) and an install on request
(`install_received`), extends t-1267's guide dialog into `ReceivedDialog`, and adds a pure
`Capabilities::of` in core.

**Tech stack:** Rust (oku-slides-core, -session, -agent, -platform, -render), React/TS (`ui`,
Bun), Fluent (20 locales), Playwright e2e.

**Parents:** Task 1 — parent: t-1413; Tasks 2 to 6 — parent: t-1414 (Task 6 also closes t-1413).

**Spec:** `docs/superpowers/specs/2026-11-01-agent-sandbox-and-received-packs-design.md`
(approved by the user on 2026-11-01: every recommendation kept).

## Global constraints

- No `std::fs` in production code: Storage only (`create_dir_all`, `delete_dir`, `list_files`).
- No `unwrap`/`expect` outside tests; no silent fallback: a missing work dir fails the turn.
- Snafu errors with backtraces, Fluent-localized; test errors on key/variant plus `fr`.
- Unit tests in sibling `x_tests.rs` with speculoos; names `unit_scenario_result`; rstest tables.
- Regressions in `tests/regressions/issue_NNNN_topic.rs`, declared in that `main.rs`.
- Every pub item documented, first sentence under 15 words; files under 500 lines (cap 1000).
- Fresh temp data dirs for every run outside `just e2e` (`OKU_SLIDES_PATHS__DATA`, `__DECKS`).
- Commits `type(scope): description (t-1413)` or `(t-1414)`, English, no Co-Authored-By.
- Fluent keys added in `en` (reference) and translated in every locale that has the file.
- Typst work only in the render worker; the session never parses Typst.

---

### Task 1: Agent working directory (t-1413)

**Files:**
- Modify: `crates/oku-slides-session/src/agent/context.rs` (`document_dir` → `agent_work_dir`)
- Modify: `crates/oku-slides-session/src/agent/conversation.rs` (`forget_conversation` empties it)
- Modify: `crates/oku-slides-agent/src/tab_agent.rs` (`setup` fails the turn on error)
- Modify: `crates/oku-slides-agent/src/link.rs` (spawn `current_dir`), `config.rs`, `choice.rs`
  (drop `Launch.fallback_cwd`, add the process dir)
- Modify: `crates/oku-slides-agent/tests/common/mod.rs`, `crates/oku-slides-local-agent/tests/common/mod.rs`,
  `crates/oku-slides-agent/tests/launch.rs` (harness: `session.resume(dir/tabs/<name>)`)
- Modify: `crates/oku-slides-session/tests/agent.rs` (drop `document_dir_of_unsaved_deck_is_none`)
- Modify: `crates/oku-slides-agent/src/error.rs` + Fluent `agent-error-work-dir` (all locales)
- Create: `crates/oku-slides-agent/tests/regressions/issue_1413_cwd_outside_deck.rs` (+ `main.rs` entry)
- Create: preset rule test in `crates/oku-slides-platform/src/config_tests.rs` (or the presets' test file)
- Modify: `docs/aide/en.json`, `docs/aide/fr.json` (agent card `s-agent`)
- Modify: `crates/oku-slides-agent/ARCHITECTURE.md`, `crates/oku-slides-agent/AGENTS.md`
- Modify (found during the task): `crates/oku-slides-cli/src/serve.rs` and `workspace.rs`: the dev
  server built its workspace without a resume folder, so its tabs had no work dir and every
  turn would fail. `Workspace::new_in(fonts, platform, file, dir)` writes the tabs' state under
  `<data>/serve`, emptied at launch and never read back (tab `t-1` still opens the file).

**Interfaces:**
- Produces: `Session::agent_work_dir(&self) -> crate::Result<PathBuf>` — `<resume dir>/work`,
  created with `Storage::create_dir_all`; error variant `NoWorkDir` (session without resume)
  or the Storage error. `pub const AGENT_WORK_DIR: &str = "work"` in `resume_agent.rs`.
- Produces: `Launch.process_dir: PathBuf` (replaces `fallback_cwd`), `<data>/agent-process`,
  created through `Storage` in `Link::launch` before `spawn`, passed as `current_dir`.

- [ ] Step 1: failing session tests in `agent/context_tests.rs` (or a new
  `agent/work_dir_tests.rs`): `agent_work_dir_with_resume_is_tab_work_folder` (dir ends with
  `work`, exists, empty); `agent_work_dir_twice_is_same_folder`;
  `agent_work_dir_without_resume_is_no_work_dir_error`;
  `forget_conversation_empties_work_dir` (write a file there with `std::fs` in the test, forget,
  folder exists and is empty); `closed_tab_removes_work_dir` (workspace close; tab dir gone).
- [ ] Step 2: run `cargo nextest run -p oku-slides-session work_dir` (background, cold build): FAIL.
- [ ] Step 3: implement `agent_work_dir` (read the dir from `self.resume`'s `Writer`; add a
  `pub(crate) fn dir(&self) -> &Path` on `Writer` if needed); `forget_conversation` deletes and
  recreates `work` through Storage (logged on failure, never fatal for the forget).
- [ ] Step 4: tests PASS.
- [ ] Step 5: failing agent tests (fake agent log): `session_new_cwd_is_tab_work_dir` for a
  saved deck (fixture `roadmap.json`) and `session_new_cwd_of_unsaved_deck_is_tab_work_dir`;
  `session_resume_cwd_is_tab_work_dir`; `agent_process_cwd_is_agent_process_dir` (fake agent
  logs `std::env::current_dir()` at startup: add that line to `bin/fake_agent.rs`'s log).
- [ ] Step 6: implement: `setup` returns `Result<(PathBuf, McpServer), AgentError>`; `open` and
  `reopen` map it to the turn failure (`request_failed` with a localized `WorkDir` error).
  Remove `fallback_cwd` everywhere; harnesses call `session.resume(dir.join("tabs").join(name))`
  (assert it returns true).
- [ ] Step 7: regression `issue_1413_cwd_outside_deck.rs`: copy `roadmap.json` into a temp deck
  folder next to `CLAUDE.md` and `.claude/settings.json`, open it, prompt once, read the logged
  `session/new` cwd: not the deck folder, and `read_dir` of it holds neither file.
- [ ] Step 8: preset rule test, rstest over flags `--dangerously-skip-permissions`, `--yolo`,
  `--full-auto`, `--ask-for-approval`, `never`, `bypassPermissions`, `acceptEdits`:
  `default_presets_never_skip_permission_requests` — no arg of any preset in
  `Settings::defaults()` contains one.
- [ ] Step 9: help paragraph in `s-agent` (en, fr): private empty folder, deck only through
  Oku Slides' tools, files next to the deck not seen (paste images), the agent's own
  permission settings still apply.
- [ ] Step 10: `ARCHITECTURE.md` (agent: cwd line), `crates/oku-slides-agent/AGENTS.md` rule:
  "Give the ACP session the tab's empty work dir, never the deck folder or the data dir.
  (spec:2026-11-01-agent-sandbox-and-received-packs-design)". Run the agent and session
  suites, then commit `fix(agent,session): agent works in an empty folder of its tab (t-1413)`.

### Task 2: Capability manifest in core (t-1414)

**Files:**
- Create: `crates/oku-slides-core/src/capabilities.rs`, `capabilities_tests.rs`
- Modify: `crates/oku-slides-core/src/lib.rs` (module, explicit re-export), `ARCHITECTURE.md`

**Interfaces:**
- Produces:

```rust
/// What a pack or theme version holds, read from its files alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Capabilities {
    pub kind: CapabilityKind,          // Widgets | Layouts | Theme, serde "widgets"/"layouts"/"theme"
    pub name: String,                  // raw; the session cleans it with bounds::label
    pub namespace: String,
    pub description: String,
    pub items: Vec<String>,            // widget or layout names, `_commun` excluded, in `order` then name
    pub requires: Vec<String>,         // packs: requires; themes: extends
    pub tokens: Vec<String>,           // packs: component token paths; themes: token group names
    pub fonts: Vec<String>,            // font families named in tokens (sorted, deduplicated)
    pub profiles: usize,               // layout profiles (0 for others)
    pub code_bytes: usize,             // total bytes of Typst sources (0 for themes)
}
impl Capabilities {
    /// Reads the capabilities of version `files`; fails on an unreadable manifest.
    pub fn of(files: &PackFiles) -> Result<Self, PackManifestError>;
}
```
  Theme manifests read through `ThemeManifest::from_files` (map its `serde_json::Error` into a
  `PackManifestError` variant or a new `CapabilitiesError` Snafu type — implementer picks the
  existing one if it fits, else adds `CapabilitiesError` with backtrace).
  Fonts: every string value under a token whose path segment is `font` / `family` (theme
  `font.*` group, pack component tokens of type font); the implementer reads `ComponentToken`
  and the theme schema to find the exact shape and documents it.

- [ ] Step 1: failing rstest `capabilities_of_version_lists_what_it_holds` with three cases
  built in the test (a widgets pack with `_commun` + 2 widgets, a layouts pack with a profile,
  a theme extending `papier` with `color` and `font` groups), asserting every field.
- [ ] Step 2: `capabilities_of_unreadable_manifest_is_error`.
- [ ] Step 3: implement; tests PASS; commit `feat(core): capability manifest of a pack or theme version (t-1414)`.

### Task 3: Received versions reviewed, installed on validation (t-1414)

**Files:**
- Modify: `crates/oku-slides-session/src/pack_receive.rs` (`receive_packs` → `install_received`)
- Rename+modify: `guide_install.rs` → `received_review.rs` (+ tests file), `GuidesReview` →
  `ReceivedReview`
- Modify: `deck_file.rs`, `merge.rs` (stop installing at open and merge), `views.rs`, `lib.rs`
  (`guides_to_approve` → `received_to_review`), `request.rs`, `request_class.rs`
  (`AcceptReceived`, `LaterGuides` → `LaterReceived`), `deck_packs_panel.rs`
  (`add_to_library` action on a not-installed library-scope version), pack tab opening path
  from Packs du deck (gate), `ARCHITECTURE.md`, `AGENTS.md`
- Create: `crates/oku-slides-session/tests/regressions/issue_1414_silent_install.rs`

**Interfaces:**
- Consumes: `Capabilities::of` (Task 2); `Library::receive_version` (packs and themes, generic).
- Produces:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ReceivedReview {
    pub id: Uuid,
    pub version: PackVersion,
    pub capabilities: Capabilities,      // names cleaned through bounds::label
    pub relation: ReceivedRelation,      // "new_pack" | "new_version" | "held" (serde snake_case)
    pub install: bool,                   // true: AcceptReceived adds it to the library
    pub blocker: Option<String>,         // localized reason it stays in the deck; then install is false
    pub guides: Guides,                  // as the agent reads them; empty when none await approval
}
pub enum Request { …, AcceptReceived { versions: Vec<VersionRef> }, LaterReceived { versions: Vec<VersionRef> } }
pub(crate) fn received_to_review(&self) -> Vec<ReceivedReview>;   // deck tab only
pub(crate) fn install_received(&mut self, versions: &[VersionRef]) -> Result<()>;
```
  Listed: every held version (`held_versions`, packs then themes) that is a library-scope
  version the matching library lacks (not built-in, not shipped), or has received guides not
  approved — minus `received_later` (renamed `guides_later`). A blocker is computed by a dry
  check: `Library::check_receive(id, files) -> Result<()>` (new, the `ensure!`s of `add` before
  any write; `add` calls it). `AcceptReceived` installs every listed `install: true` version
  (packs: `self.library.receive_version`, themes: `self.themes.receive_version`, a theme's
  `extends` parents first), approves the guides of every listed version, then
  `refresh_pack_updates` and `theme_list` refresh. A failed install is an error of the request
  (the dialog stays).
  CLI and headless paths: nothing calls `install_received` but the request.

- [ ] Step 1: rename the module and types (mechanical, tests still green), commit
  `refactor(session): received review generalizes the guides install list (t-1414)`.
- [ ] Step 2: failing tests in `received_review_tests.rs`:
  `open_deck_with_new_library_pack_leaves_library_unchanged`,
  `received_to_review_lists_new_pack_with_capabilities`,
  `accept_received_installs_pack_marked_received_and_approves_guides`,
  `later_received_empties_list_for_tab_and_deck_still_renders`,
  `reopened_tab_lists_later_version_again`,
  `received_to_review_with_taken_namespace_has_blocker_and_no_install`,
  `accept_received_skips_blocked_version_installs_others`,
  `merge_bringing_version_lists_it`,
  `accept_received_installs_theme_and_its_parents`,
  rstest `received_to_review_never_lists_install_of(builtin | local | deck_only)`,
  `received_to_review_outside_deck_tab_is_empty`.
- [ ] Step 3: implement; PASS.
- [ ] Step 4: Packs du deck: a held library-scope version absent from the library gets an
  "Add to my library" action (`add_to_library`, A2UI event) that sends `AcceptReceived` for
  it; test `packs_panel_add_to_library_installs_version`. If the panel's catalog changes,
  change `ui/src/a2ui/catalog.tsx` in the same commit (invariant).
- [ ] Step 5: pack tab gate: opening a pack tab on a not-installed received version is refused
  with a localized error `PackNotInstalled` (the panel shows "Add to my library" first);
  test `open_pack_tab_on_not_installed_received_version_is_refused`.
- [ ] Step 6: regression `issue_1414_silent_install.rs` (open → library lacks the pack).
- [ ] Step 7: Fluent keys (en + all locales): `received-blocker-namespace-taken`,
  `received-blocker-namespace-reserved`, `received-blocker-namespace-changed`,
  `error-pack-not-installed`, panel label; rules: session `AGENTS.md` "A received version
  enters the library only through install_received after the user's validation.
  (spec:2026-11-01-agent-sandbox-and-received-packs-design)"; ARCHITECTURE update. Commit
  `feat(session): install received packs and themes only on validation (t-1414)`.
- Deviations (found during the task): the error key is `session-pack-not-installed` (session
  errors all use `session-*`); two more blockers, `received-blocker-deck-only` and
  `received-blocker-unreadable` (a version whose content claims another kind than where the deck
  holds it); the panel's `add_to_library` installs without approving guides; a deck-mode widget
  write on a received pack the library does not know is refused (`PackNotInstalled`), since it
  would otherwise write the pack into the library; the AGENTS.md rule is merged into the existing
  received-version bullet (the file was at its line cap).

### Task 4: ReceivedDialog in the UI (t-1414)

**Files:**
- Rename+modify: `ui/src/GuidesInstallDialog.tsx` → `ui/src/ReceivedDialog.tsx`, its
  `.render.test.tsx`; `ui/src/App.tsx`; `ui/src/api.ts` (`ReceivedReview`, `received_to_review`,
  `acceptReceived`, `laterReceived`); CSS class `received-dialog` (keep `confirm-dialog`)
- Modify: Fluent `ui.ftl` (all locales): `ui-received-title`, `ui-received-intro`,
  `ui-received-offline` ("Packs and themes cannot read your files or the network."),
  `ui-received-add`, `ui-received-approve`, `ui-received-later`, relation and field labels;
  remove the replaced `ui-guides-install-*` keys
- Modify: `ui/e2e/330-guides.cjs` (renamed field, button labels); create
  `ui/e2e/NNN-received-packs.cjs` (next free number)

**Interfaces:** consumes `ReceivedReview` and `Request::AcceptReceived/LaterReceived`
(serde `cmd: "accept_received" | "later_received"`).

- [ ] Step 1: failing render tests: lists each version's name, kind, item names, requires,
  tokens, fonts, code size, guides as plain text (`<pre>`, never HTML); primary label is
  "Add to my library" when any `install` is true, "Approve guides" otherwise; a blocker line
  shows and that version has no install mark; the offline line shows; "Later" has focus.
- [ ] Step 2: implement; `bun test` PASS; `tsc` clean.
- [ ] Step 3: e2e: deck written elsewhere bringing a new pack → dialog → « Plus tard » →
  library (`/api` packs list) lacks it, deck renders → Packs du deck « Ajouter à ma
  bibliothèque » → library has it. Update `330-guides.cjs`. Run `just e2e 4999`.
- [ ] Step 4: commit `feat(ui): one dialog validates what a received deck brings (t-1414)`.
- Deviations (found during the task): the kind keys move to `ui-received-kind-*` so the dialog's
  keys form one block; `330-guides.cjs` ends its reopen step by adding the pack from the dialog,
  since the reopened deck still lists it for install; the e2e helper becomes `writeDeckBringingPack`.

### Task 5: Offline render regression and dependency check (t-1414)

**Files:**
- Create: `crates/oku-slides-render/tests/regressions/issue_1414_render_stays_offline.rs`
  (+ `main.rs` entry)
- Modify: `justfile` (`check` gains `render-offline-deps`), `crates/oku-slides-render/AGENTS.md`

- [ ] Step 1: rstest over widget sources: `#import "@preview/cetz:0.3.4": *`,
  `#read("/etc/hosts")`, `#image("/System/Library/CoreServices/SystemVersion.plist")`,
  `#read("../../../../etc/passwd")`, `#plugin("/tmp/x.wasm")`, `#read("https://example.com")`;
  each render (through the crate's test helper for a widget in a deck) is an error whose
  diagnostic says not found (file or package), within 10 s.
  `render_widget_reaching_outside_fails_not_found`.
- [ ] Step 2: it passes at once (the world is already closed): a pinning test; check it fails
  if `world.rs::file` served the path (temporarily, locally, not committed).
- [ ] Step 3: `just render-offline-deps`: `cargo tree -p oku-slides-render-worker -e normal
  --prefix none` must not list `ureq`, `reqwest`, `hyper`, `typst-kit`; wired into `check`.
- [ ] Step 4: render `AGENTS.md` rule: "Keep the Typst world closed: serve only in-memory
  files, never a package or a path on disk. (t-1414)". Commit
  `test(render): the render stays offline (t-1414)`.

### Task 6: Review, gate, merge

- [ ] `just check` (agents-lint included), `just test` (background), `just e2e 4999`.
- [x] Review round (`common:review` team) on the branch diff; fix findings; record the round.
  Round 1 (2026-11-02, 15 reviewers): 2 critical, 11 major, ~20 minor, all fixed; the
  deviations it surfaced are in the spec; deferred items filed as t-1498 to t-1501.
- [ ] `git fetch`, merge `origin/main`, `bun install` in `ui/` if deps moved, `just ci`.
- [ ] Squash to `main` citing `(t-1413, t-1414)`, push, `oku cli task done` both tickets and
  every sub-task.
