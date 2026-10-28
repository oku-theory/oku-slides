# Packs and themes as tabs — lot 5: deck-only pack tabs — Implementation Plan (v4)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A deck-only pack (an import's, a deck's own `deck` / `deck-layouts` pack) opens in its own window tab linked to its owner deck tab, whose saves reach that deck as one undo step, which is closed with its deck; the in-deck workshop, `Request::EditPack` and `Request::ClosePack` are removed.

**Architecture:** A tab says it edits a deck-only pack of a given deck tab (`TabDocument::DeckPack { deck_tab, pack }`). Its session is an ordinary pack tab (`Mode::Pack`, a preview deck copied from its owner, as lot 3's library pack tabs) except that it has no lease and writes nothing to the library: each save leaves the pack's files in an **outbox**. A public call on the tab (`Workspace::request` / `with_session`) reads a due send after the call, releases the tab, hands it to the owner's session (`receive_deck_pack`: validated against the owner's own pins, ordered by a process-wide sequence number, applied as one undo step merged per editing session), then acknowledges it to the tab, both sides persisting their state; the caller waits for that, as the user decided. Every session acquisition on a deck tab first hands it the links the workspace layout names for it; the owner keeps each link's state by tab id and, while linked, refuses every change of the version it pins for that pack (deck-mode writes, raw ops, removal, merges, history restore, undo and redo steps). Owners a forward changed are queued on the workspace for the hosts to refresh. Closing the owner flushes its linked tabs outside the workspace lock, then closes them with it. With every pack editor a tab, `PackHost` and the in-deck workshop go; `Mode::Pack.stash` becomes `preview`.

**Tech Stack:** Rust 2024 (snafu, speculoos, rstest, nextest, OpenDAL `Storage`; clippy `pedantic`, `missing_docs`), React/TypeScript (Bun), Playwright e2e, Fluent.

**Spec:** `docs/superpowers/specs/2026-10-24-onglets-pack-theme-design.md` — "Tab model and resume" (`DeckPack`, the only linked tab), "Sessions by document kind" (deck-only pack: `ReceiveDeckPack`, one undo step per burst, lock order), "Lots" (lot 5) and its lot-3 and lot-4 implementation notes; this plan's "Decisions" amend the spec where they differ (Task 9 writes the amendment). Oku: t-1152 › t-1200 › Task 1 t-1224, Task 2 t-1234, Task 3 t-1226, Task 4 t-1227, Task 5 t-1228, Task 6 t-1235, Task 7 t-1229, Task 8 t-1230, Task 9 t-1232. Branch `t-1152-continuation-6fea98` = `main` 756aa9ec. Every `file:line` below was read at 756aa9ec; a line that drifted is found by the quoted text. Reviews: v1 `.superpowers/review/lot5-plan-v1-review.md`, v2 `.superpowers/review/lot5-plan-v2-review.md`, v3 `.superpowers/review/lot5-plan-v3-review.md`; v4 applies every critical, major and minor item of the v3 report (its one false positive, the first seq, gets a test).

**Out of this lot:** tab borders, item pile, preview-theme selector, "Ouvrir" on packs in "Packs du deck", app-menu entries, `PackTabs` removal (lot 6, t-1157); tool filtering by kind, agent-opened tabs beyond today's gate (lot 7, t-1191); offering a theme or a copy to the opener (t-1207); an owner absorbing an external change on save while linked keeps its links and later sends are refused (ADR-0013 known limit; an owner that opens another document fails closed, decision 8); an agent task that dies before `end_turn` keeps its group open and parks forwards, as it already blocks undo (ADR-0013); out-of-order concurrent forwards tested at workspace level (needs a hook; the session-level stale test covers the logic); the tab-id counter clamp at `MAX_NUMBER` (pre-existing, backlog).

## Global Constraints

- **Commits: Conventional Commits in English, `type(scope): description (t-N)`, no space before the colon, `!` for breaking**; every commit cites t-1200 and its task's sub-task; `just check` runs `commit-lint`. Never a `Co-Authored-By` line. Under SDD implementers stage only; the controller commits after the task review with a `Reviewed-by:` trailer naming the review round. Commit message files live under `.superpowers/sdd/2026-10-27-onglets-pack-theme-lot5-deck-pack-tabs/`, written with the Write tool, committed with `git commit -F <file>`.
- **Commit order.** First the controller commits this plan alone: `docs(plan): lot 5 of pack and theme tabs, deck-only pack tabs (t-1200)` with a `Review-waived:` trailer (later rules cite `plan:2026-10-27-onglets-pack-theme-lot5-deck-pack-tabs`, which `agents-lint` checks on disk). **Tasks 3, 4 and 5 are implemented and reviewed one by one but committed once, after Task 5's review** (their `pub(crate)` items have no production caller before Task 5, and `PackRoute::DeckTab` only exists with its workspace arm). Tasks 3 and 4 end staged; their gate is `cargo check --workspace` plus their own tests; dead-code findings on items Task 5 calls are accepted until then and listed in the review notes — **no temporary `#[expect(dead_code)]` or `allow` is added**. After each staged review the controller records `git write-tree` in the review notes; the next task's review diffs against that tree, never the whole index. Task 7 is two commits (7a migrates tests off the workshop, 7b deletes it); every other task is one commit. The branch reaches `main` as **one squash commit**; from the Tasks 3-5 commit to Task 8 the e2e specs on the deck-only workshop (`220-import.cjs`, `lib/atelier.cjs:30`) are red (bisect note in the review notes), so `just e2e` runs only at the end of Tasks 8 and 9; every commit keeps the Rust and UI unit gates green.
- **New or edited code comments and docs in English** (a French doc block that is edited is translated whole: `with_session` and `request`'s docs are rewritten in Task 5); tool output and user-facing strings stay French / Fluent.
- **No manifest change** in this lot (no new dependency).
- **Gates.** End of each commit: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace --exclude oku-slides-app` (the Tasks 3-5 commit and Task 7 add `-p oku-slides-app`), the UI gate. End of Task 8: the same plus `just e2e 4999`. End of Task 9: `just ci`, `just e2e 4999`. The UI gate is `cd ui && bun run typecheck` and both UI test processes. Long runs run detached (`nohup` script, logs and an end marker under `.superpowers/sdd/<plan>/`); under load re-run timed-out tests with `nextest --retries 2` before blaming a change. **Every test that takes two sessions, flushes, closes, settles or forwards runs its body in a spawned thread and waits with `recv_timeout`** (`workspace_tests.rs:585-614`); none asserts on timing.
- **Process-wide statics in tests.** `PACK_TABS`, `OPENING`, `DRAFTS`, `WRITES` and the new `DECK_PACK_SEQ` are per process; a test that builds two sessions on one library drops the first before the second restores and asserts the pack is unleased at that point (`test_support_packs::assert_unleased(scope, id)`, `test_support_packs.rs:112`). Assertions on `WRITES` and seqs compare before/after values, never absolute ones, and rely on nextest's one process per test (said in a comment on each). `assert_unleased` and every `test_support*` helper are unit-test only; integration tests show a lease's absence by a write of the pack from another session succeeding.
- **Pedantic lints:** new pub getters, pure constructors and `Touched`-like results `#[must_use]`; pub fallible functions get `# Errors`, blocking ones `# Blocking` (separate headings); an existing pub method that gains an error kind or a wait has its doc updated in the same task; `#[expect(…, reason)]` only, never `allow`; no function over 6 arguments (with `self`); `too_many_lines` (100) is fatal: `adopt_session_version` (`pack_mode_save.rs:189-291`) calls one helper for the outbox, never inline code; `receive_deck_pack` is `validate` + `apply`, each under 100 lines.
- No `unwrap`/`expect` outside tests; no silent fallback (a refused forward is a notice and an unsent tab, never dropped work; a real fault is never reported as a benign refusal); `lock().unwrap_or_else(PoisonError::into_inner)`; never a std Mutex guard across an await.
- **Lock order** (lot 3's, plus the forward): `OPENING` (per pack id) → the workspace, then one tab's session, in sequence and never nested → `DRAFTS` → `PACK_TABS`. A forward locks the pack tab's session (take the due send, mark it in flight), releases it, locks the owner's session (receive, **blocking until it is free**), releases it, locks the pack tab's session again (acknowledge): never two sessions at once, never the workspace during a session call, never under `OPENING`. Flushes before a close run in `apply`'s pre-phase, outside the workspace lock, like `to_wake` / `wake`. **No `Drop` locks the workspace**: cleanup that needs it (`PendingLink`) runs explicitly once the guard is released; a `Drop` that must reach it (a panic net) uses `try_lock` and logs `warn` when held. Slow work (building a session, rendering, reading the library) runs outside the workspace lock (ADR-0001).
- No `std::fs` in production code. **`workspace.json` and `state.json` are untrusted**: every value read is validated; what the owner deck knows (what it pins, what it embeds, the manifests of its pinned versions) is recomputed from it, never taken from the pack tab.
- **Formats.** `workspace.json` stays `LAYOUT_VERSION = 2`; a deck-pack entry carries `document: {kind: "deck_pack", deck_tab, pack}` plus the legacy keys, all `null` (6.0.0's `take_info` fails on the unknown kind and drops only that entry). `state.json` stays `EDITOR_STATE_VERSION = 5`; a pack state gains `unforwarded` (additive, `serde(default)`) in Task 5 and loses `host` in Task 7; 6.0.0 refuses version 5, so a downgrade sets these tabs aside, never misreads them; lot 4's own build was never released and is not a downgrade target (ADR-0013). A `workspace.json` with `editing_pack` keys still reads (no `deny_unknown_fields`).
- Strings from manifests, decks or requests that reach a title, a dialog, an error or a log go through `agent::bounds::label` (both deck-pack titles: the tab's and the one in `PackOpenInTab` / `UndoInPackTab`).
- Every pub item and new module documented, first sentence under 15 words (≤ 14); **every field and variant of a new struct, enum or request documented with the doc lines this plan writes**; every new type derives `Debug` (a hand-written `Debug` where a field cannot); no dead code; crate-internal API `pub(crate)`; new public types re-exported explicitly in `lib.rs` and listed in its module doc.
- **Errors:** Snafu, a backtrace in every variant, Fluent-localized; a new session error variant is **six edits** in `error.rs` (`SessionErrorKind` variant, `SessionInner` variant with backtrace, both `use SessionInner::{…}` lists, the arms of `kind()` and of `localize`) plus a key in **all 20 locales**, each translated (never English copied), in the same key order: session keys after `session-pack-open-in-tab`, notices after `notice-theme-draft-dropped`. Tested on the kind/key plus `fr`. Removed keys leave all 20 locales in the same commit.
- Unit tests in sibling `x_tests.rs` registered with `#[cfg(test)] #[path = "x_tests.rs"] mod x_tests;`; speculoos; names `unit_scenario_result`; one behavior per test; rstest for tables; temp dirs via `test_support::fresh_dir`. **Integration tests** use only the public API and their crate's `tests/common` (session: `tests/common/workspace_support.rs`; mcp: `tests/common/workspace_harness.rs`), never crate-private helpers (`test_support*` is `#[cfg(test)]`, invisible downstream). A test of removed behavior is deleted and named in the task's review notes; a test whose name stays true keeps it. Every task ends with `wc -l` on the files it touched (none above 1000, a new one aimed at 500).
- **Files at their caps** get only match arms, variants, fields, module declarations, re-exports and one-line calls: `lib.rs` (830; `unsaved()`'s new logic lives in `deck_pack_tab.rs` as `unsent()`, called in one line), `resume.rs` (839; the guard rewrite at `:707-717` becomes a one-line call into `resume_workshop.rs`), `error.rs` (757), `workspace_tests.rs` (950: only the `.1` → kept `bool` stays; no new test there), `App.tsx` (584: Tasks 7-8 must not grow it).
- Mirror every `Request` variant in `ui/src/api.ts` / `ui/src/packTypes.ts` and every `WorkspaceRequest` / `TabDocument` / `TabInfo` change in `ui/src/workspaceApi.ts` in the same commit (rule t-936); `src-tauri/src/commands.rs` `classify` and `broadcast` and `workspace_tabs.rs` `change` are exhaustive over `WorkspaceRequest`. UI test files, fakes (`ui/src/test/*`), e2e helpers and JSON fixtures are not typechecked: every changed call shape is grepped in `ui/src`, `ui/e2e`, `fixtures/` and `crates/*/tests`, including indexed JSON (`["host"]`, `["editing_pack"]`).
- Docs at their caps: session and mcp `ARCHITECTURE.md` 50/50, session, mcp and ui `AGENTS.md` 60/60, root `AGENTS.md` at its cap: **every docs edit replaces or condenses text, never a net new line** (words added to an existing rule line keep the count). ui has no `ARCHITECTURE.md`. **A change that creates, changes or voids a rule or an `ARCHITECTURE.md` statement edits it in the same commit**; rules cite only ADRs that exist in that commit.
- Runs that start the app outside `just e2e` set `OKU_SLIDES_PATHS__DATA` and `OKU_SLIDES_PATHS__DECKS` to a fresh temp dir. Do not set `CARGO_TARGET_DIR`.

## Decisions (binding; 1-4 and 6, 11's "close others" the user's on 2026-10-27, the rest settled by this plan)

1. **Undo stops at a linked pack (user).** While a `DeckPack` tab is linked (pending or confirmed), an undo or redo in its owner deck that would change the version the owner pins for that pack is refused (`UndoInPackTab { pack, tab }`). Checked after the fact, so it covers steps made before the tab opened and `undo_agent_turn` (`agent/turn_undo.rs:92-95`, same `replay`). `replay(do_step, opposite)` (`undo.rs:39`, the pair passed, never a fn-pointer comparison) records `linked_pins()` (`BTreeMap<Uuid, LinkedPins { widgets, layouts }>`, deck mode only), runs the step, and when one changed runs the opposite step, leaves `dirty` as it was and returns the error; nothing rerenders. **If the opposite step fails or `linked_pins()` is not restored, `UndoCompensationFailed` is returned and the real cause logged `error`, never `UndoInPackTab`.** The compensation writes two Loro changes and resets the restored step's merge key to `None` (`replica/undo.rs:174-190`, in `oku-slides-core`): after a refused undo the next receive opens a new step; after a refused redo the compensating undo cuts the merge of the step underneath. Both pinned by tests, written into ADR-0013. Once the tab closes, the merged step undoes from the deck as after today's "Terminer".
2. **Old workshops reopen on their deck (user).** A deck tab saved with its in-deck workshop open resumes in deck mode: `deck/` is the document, `pack/` is ignored, no lease is taken. Classification (`resume_workshop::classify`): v3 with a `Pack` mode → workshop; v4 → pack tab if `host == "tab"`, else unreadable (as today); v5 → workshop if `host == "deck"`, else pack tab (no `host`, or `"tab"`). The notice `notice-workshop-reopened-on-deck` is shown when the saved pack document was dirty. Lot 4's legacy restyle of version-3 workshops (`resume_pack.rs`, its theme-draft notice) is deleted with them; version-4 pack tabs keep it.
3. **The owner treats its linked pack as leased (user).** While linked, the owner refuses with `PackOpenInTab { pack, tab }` (`tab` = the link's title) every change of the version it pins for that pack id, in either kind: deck-mode widget and layout writes (`commit_files`' `Mode::Deck` arm, `widgets_edit.rs:534`, and the deck's own-pack session in `widgets_deck.rs`), raw `Request::Apply` ops (`apply_ops`, `deck_widgets.rs:320`: `refuse_linked_pins(&pins_after(..))` on the one combined map `pins_after(&combined_pins(..))` builds (`:333`), right after it, before `embed_from_library` and `embed_op_assets`, compared **by pack id**, so an `AddPack` of the linked id under a new namespace is refused), `remove_deck_pack` (`deck_packs.rs:171`), `merge_file` (`merge.rs:65`: `absorb` runs on a clone of the document first; `linked_pins()` of the clone must equal the owner's, else refused, failing closed, the owner untouched), and `history_restore` (`history.rs:260`). The on-save absorb (`merge.rs:82,84`, merges in place) is **not** refused: it merges, and a link whose pin it moved refuses the tab's next send (ADR-0013 known limit). `add_library_pack` already refuses a deck-only id. "Éditer le widget" on one of its elements in the owner (`OpenPack`) focuses the pack tab on that widget.
4. **t-1224 is Task 1 (user):** a guard test first; every public pin path already freezes the pinned version (`refresh_pack_updates` → `adopt_pins`, `deck_packs.rs:294-296`, `deck_adopt.rs:18-40`), which is the safety; a defence-in-depth freeze in `write_deck_widgets` covers a failed `adopt_pins` (only logged, `deck_adopt.rs:37`): it adopts **the parent** of the first write of a widget session, never the session's own draft, under `adopt_pins`' guards.
5. **The link lives in the workspace; its state in the owner.** `TabDocument::DeckPack { deck_tab, pack }` in `workspace.json` is the link's only persisted form. The workspace fixes `deck_tab` at open and resume (`note` refuses — logs `warn`, keeps the cached document — an info whose `deck_tab` differs) and moves the cached `pack` only through `retarget_deck_pack` after an `Applied` copy (decision 9), which then calls `save_layout`. **Every session acquisition** of a deck tab (`with_session`, `request`, the forward's `with_quiet`, `wake`'s `ready`, `post_notice`, carry) hands it `deck_links_of(tab)` before any work: `Slot::ready` becomes private and `Slot::ready_with_links` is the only way in, so the compiler enforces it. The owner keeps per tab id a `DeckLink { title, pack, deck_uid, parent: LinkParent, expect, last_seq: Option<u64>, pending }`: `pack` is the owner's (after a copy, the copy's; the layout's `pack` only seeds a new link), `deck_uid` the owner's document when the link was made, `expect` the version it must currently pin for it, `parent` what the tab's sends carry (below), `last_seq` the last applied seq (`None` before the first). `set_deck_links(generation, &[LinkSpec]) -> Vec<String>` ignores a call whose layout `generation` (a `u64` the workspace bumps on every layout change, copied with the links) is below the last one it applied — a stale copy taken before a close never brings a closed tab's link back; it keeps a known link of a named tab whatever pack the layout says, adds a link for a newly named tab (decision 13's re-seed), keeps a pending link until it is named or unlinked, drops a confirmed link no longer named, and **returns the tab ids whose new link the re-seed refused**; the workspace closes those tabs after the call (folder kept aside, `notice-tab-resume-failed`, logged), so no deck-pack tab stays in the layout without a link.
6. **Forwarding waits for the owner (user).** The caller of a public call on the deck-pack tab waits for its forward: the tab's session is released first, so a concurrent call on the tab (a `Snapshot`) still answers while the owner is held; the owner's lock is awaited like any request. **One forward in flight per tab**: taking the due send marks it `InFlight` under the tab's lock; another call meanwhile finds nothing due, and a save meanwhile sets the outbox's `newer` flag without leaving `InFlight` (decision 7). `with_session`, `request` and `with_quiet` share one private `acquire(ws, tab, f, Forward::{Yes, No})` (G11); the returned `bool` also covers the tab's refresh after its acknowledgement (its `TabInfo.dirty` may flip). Internal workspace calls (`workspace_packs.rs:218,259,279,335,337`, `workspace_themes.rs:49,76`, `workspace_carry.rs:25`, `workspace_help.rs:36`) use `with_quiet` (links pushed, `closed` checked, no forward), so a forward never runs under `OPENING` and never loses its owner.
7. **Outbox and acknowledgement.** In a deck-pack tab, `adopt_session_version` adopts into the preview as today, writes nothing to the library and calls `fill_outbox(base, files)` (`deck_pack_tab.rs`): it replaces the outbox's files; **its base is the one of the oldest unacknowledged save** (`SendBase { pack, parent }` = the tab's `(edit.pack, edit.parent)` before that save — so a second save after an unacknowledged copy still names the original pack); state `InFlight` stays `InFlight` with `newer = true`, any other state becomes `Idle`. `realign_pack` (undo/redo in the tab) fills it too. The outbox becomes **due** at a `SavePack` request (the UI autosave), a flush (closing the tab or its owner, quitting) or `end_turn`: `mark_due` (from `Idle`, `Refused` or `Parked`; a still-busy owner just parks it again) builds the `Arc<DeckPackSend>` once (`seq` from `DECK_PACK_SEQ`, which starts at 1 and is never reset) — not on every widget write. Answers: `Applied` and `Delivered` → acknowledged `Ok`: the outbox is cleared if its seq is still that one, or, when `newer` is set, becomes `Due` with a new send whose base is the tab's current `(edit.pack, edit.parent)` (what the owner now holds for the link); `Stale` → nothing (a newer send exists); `Busy` → **parked** (no notice); the owner's `end_turn` and `back_to_deck` (`history.rs:169`, which both `history_close` and `history_restore` run) set its pull flag, and the next public call on the owner forwards every parked send of its links; an `unpark` that finds the outbox `InFlight` sets `repull`, and the park that follows turns it straight into `Due`; an error → `Refused` (files and unsent flag kept) until the next save, flush or `end_turn`, the error's text becomes the tab's notice, logged `warn`. Acknowledge, park and `fill_outbox` call `persist(false)` (the tab's `state.json`, whose pack state carries `unforwarded: Option<SendBase>`, `Some` while the outbox holds files). `Snapshot.dirty` stays `doc.dirty` (the autosave contract); `EditingView.unsent` (new; TS `PackMode.unsent`) is true while the outbox holds unacknowledged files; `TabInfo.dirty` and `unsaved()` are `doc.dirty || unsent` (via `Session::unsent()`). `ensure_closable` refuses an unsent tab.
8. **Receiving (`Session::receive_deck_pack(tab, &DeckPackSend) -> Result<Received>`)**, `validate` then `apply`. Mode: `Deck` proceeds; `History` → `Ok(Busy)`; else `WrongDocument { kind }`. Order: `tab` is a link (`DeckPackMismatch`); the owner's `deck_uid` is the link's (an owner that opened another document refuses, `DeckPackMismatch`); `Some(send.seq) < last_seq` → `Ok(Stale)`; `Some(send.seq) == last_seq`, or the files' version is already `link.expect` → `Ok(Delivered)`; `replica().in_group()` (`sources.rs:214`, an agent turn) → `Ok(Busy)`; the owner's current pin of `link.pack` is `link.expect`, deck-scoped (`deck_scope(..) == Some(true)`, fail closed) and the library has no current version of that id; **the link's namespace** is the deck key that pins `link.expect` (never the manifest's word); `send.base` matches `link.parent` (below); the files' manifest has `scope: Deck`, the pinned version's kind and the link's namespace; then **same pack** (`id == link.pack`, and `parent == send.base.parent` or the version is `send.base.parent` itself — an undo back to the opened version) or **a copy** (decision 9) — anything else `DeckPackMismatch`, the deck unchanged. The send carries no closure files: the closure (`closure_with_files` over the owner's own `sources.packs` plus the send's files) resolves from what the owner holds and the library, as any deck pin does; **the owner's source bytes after the apply** (its current total plus the added files) stay within its `source_limits` (`files.rs:118`, `total_bytes`, the budget `sources_zip.rs:129-167` enforces on reopen), else `DeckPackMismatch` logged with both sizes. `apply` decides everything on clones before writing (as `pack_mode_save.rs:234-238` pins copies first): `pin_deck` on a clone of the deck (for a copy, the original's pins removed first, decision 9); the clone's pins differ from the owner's only at the link's namespace (and newly required namespaces that were absent), else `DeckPackMismatch` with nothing written; only then embed, `sources.apply(clone, "pack-adopt-label", Some(&deck_pack_merge_key(tab)))` with `deck_pack_merge_key(tab) = format!("pack-adopt:{tab}")` (tab ids are monotonic and persisted: one step per editing session); when the apply merged, the previous `expect` leaves the owner's sources only if it is not the new version, not `send.base.parent`, and pinned or required nowhere else (a history point saved between two merged sends then no longer renders that version: ADR-0013 known limit, as today's `pack_mode_save.rs:259`); `sync_widgets()`; `dirty = true`; `refresh_pack_updates`; `rerender` (a failed render keeps the change, notice as `save_pack` does); `persist(true)`. Then `expect` = the new version, `parent = Known(send.base.parent)`, `last_seq = Some(send.seq)`, `Ok(Applied)`. **`LinkParent`**: `Known(Option<PackVersion>)` (set by `link_deck_pack` to `Some(pin)`, or by an applied send); `Reseeded { pin, pin_parent }` for a link rebuilt from the layout (decision 13): `send.base.parent` must be `Some(pin)` or `pin_parent`; the first applied send makes it `Known`.
9. **Copies.** "Save as copy" in a deck-pack tab keeps today's rule (`pack_mode_save_tests.rs:234`, `pack_mode_save.rs:82-86`: a deck-only copy, `parent: None`, **keeping the original's namespace**, which the check below relies on). A copy is accepted when its id is fresh (not pinned by the owner in either kind, not in its sources, not in the library, not built-in), `parent` is `None`, its kind is the link's and **its namespace is the link's** (the owner pins the copy in place of the original: a copy under another namespace would overwrite another pin, `pin_deck`, `pack_mode.rs:147-166`): the owner removes the original's pins from **both** maps (`deck.widgets` and `deck.layouts`, `retain(|_, p| p.id != link.pack)`, as `pack_mode_save.rs:219-220`), then `pin_deck`s the copy under the link's namespace; `refuse_linked_pins` runs for the other links. The link becomes `{ pack: copy, parent: Known(None), expect: copy version }`. The workspace, on that `Applied` of a send whose files' id differs from `base.pack`, calls `retarget_deck_pack(tab, copy)` inside the same `forward`, before releasing control; until acknowledged, `pack_tab_info` reports `base.pack` (the cached document never names an unaccepted copy). An undo in the tab past the copy is refused by the owner (ADR-0013 known limit).
10. **Routing** (Task 5; `route_pack`, `pack_tab.rs:119`), written out:

    | Requester | Pack | Route |
    |---|---|---|
    | deck tab | library pack | `Tab { name }` (today) |
    | deck tab | deck-only pack it pins | `DeckTab { name }` (was `enter_here`) |
    | any pack tab | the pack it edits | `Here { entered: false }` (its widget opens there) |
    | any pack tab | another library pack | `Tab { name }` (today, `pack_tab.rs:141`) |
    | any pack tab | another deck-only pack | `InPackTab` (today) |
    | session without a lease scope (single-document) | library pack | `enter_here` until Task 7, then `SingleDocument` (decision 14) |
    | session without a lease scope (single-document) | deck-only pack | `DeckOnly` refusal (explicit arm) |

    `name` is the pack's name through `label`. `open_pack_here` (`pack_tab.rs:243-246`) maps `DeckTab` to `enter_here` until Task 7 deletes both. `WorkspaceRequest::OpenPack` and the agent's `Workspace::open_pack` open or focus `TabDocument::DeckPack { deck_tab: <requester>, pack }` under `with_opening(pack)`, right after the owner, the owner as opener (memory); `candidates` / `focus_candidate` look for that document from the owner and from the deck-pack tab itself; the agent's gate (`PackGate::NewTab`) applies as for a library pack. No lease. `pack_opener` answers `None` for a deck-pack tab. `take_preview` keeps refusing deck-only (`DeckOnly`); the owner's seed comes from `link_deck_pack`, which records a **pending** link. `PendingLink` tracks it: every error path after `link_deck_pack` (failed build, failed insert) returns the guard, and the caller calls `PendingLink::cancel()` once the workspace guard is released (`with_quiet(owner, unlink_deck_pack)`); the guard's `Drop` is only a panic net (`try_lock` on the workspace, `warn` when held) and is disarmed once the tab is in the layout. Links live only in memory: after `settle` nothing needs unlinking. `OpenCtx` and `Ask` (`workspace_packs.rs:89,103`) become `pub(super)` for `open_deck_pack`.
11. **Closing.** `Close` gets its own arm in `apply` (as `CloseWindow` has, `workspace.rs:230`): with `discard: false`, the pre-phase, outside the lock, flushes every tab linked to the closed deck tab through the public forward path (wakes it, saves when dirty, forwards); under the lock `close_with_links` runs `ensure_closable` on the owner and each linked tab (each refuses while dirty or unsent), then removes them together, the owner first detached by `detach_to_opener`; with `discard: true` nothing is flushed and every linked tab is discarded too — **the server inherits a discard from its own layout; the UI never adds linked ids**. `CloseWindow` does the same for every owner in the window, linked tabs in other windows included. **If the closed set is every tab of the workspace, `Close` and `CloseWindow` answer `LastWindow` with nothing closed** (Tauri quits keeping everything, `serve` maps the outcome to 409; serve's `would_empty` keeps its plain last-tab pre-check, unchanged); with `discard: true` the workspace first records the linked tabs as `discard_at_quit`, which `flush_all_deck_packs` skips, so a quit never flushes edits the user discarded. Closing a deck-pack tab alone flushes it, removes it, then (lock released) `with_quiet(owner, unlink_deck_pack(tab))`, and activates its owner; the outcome is `DeckChanged { tab: owner }` when the flush reached the owner. **"Close others" on a deck-pack tab (user) keeps its owner open: the UI closes every other tab of the window except the kept tab's owner; sibling deck-pack tabs of that owner close normally.** `to_wake` lists linked tabs. Quit: `settle` first flushes every awake deck-pack tab (`flush_all_deck_packs`, before setting `closed`; it returns at once when `closed` is already set, so the second `settle` on the main thread, `src-tauri/src/main.rs:292`, never forwards), then persists as today; a forward that loses the race with `closed` leaves the tab `unforwarded`, and its resend at resume answers `Delivered` (version already `expect`). `settle` does not end open agent turns.
12. **Hosts learn the owner changed.** `with_session` and `request` keep their `(Result<R>, bool)` signatures; a forward that applied pushes the owner's id onto `Workspace.touched` and forces the owner's info refresh. `Workspace::take_touched(ws) -> Vec<String>` (pub, `#[must_use]`) drains it, and `Workspace::drain_touched(ws, emit: impl FnOnce(), mut after: impl FnMut(&str))` (pub) is the one testable host step: when the queue is non-empty it calls `emit` once, then `after` per owner. A window shows an owner whose `TabInfo` changed by re-reading its snapshot when it is that window's active tab (Task 8 checks the existing view handler and tests it). Hosts drain after every call: Tauri `commands.rs` `request` and `workspace` (emit the view when non-empty, `state.stack.after_change(owner)` each); `AppSessions::with` (`state.rs:105`, called by the stack) emits the view and defers `after_change` to `tauri::async_runtime::spawn_blocking` through `app.state::<AppState>()` — never inline (the stack calls `AppSessions`); `serve.rs` after `/api`, `/api/workspace` and `/api/agent` (an agent turn's owners are drained at the next request: dev-server limit, ADR-0013). `WorkspaceOutcome::DeckChanged` keeps its producer (closing a deck-pack tab that flushed); the `serve.rs:174` comment ("A workshop entered in place") is reworded in Task 7.
13. **Resume.** `read_info` (`workspace_resume.rs:90`) accepts `deck_pack` when `deck_tab` passes `is_tab_id` and differs from the entry's own id. After all entries are read, a pre-pass sets aside every deck-pack entry whose owner is not a restored entry of kind exactly `Deck`, or which repeats an earlier `(deck_tab, pack)` (its folder kept aside, `notice-tab-resume-failed`), before windows and `active` are rebuilt, so no window ends empty or with a stale active tab. A deck-pack tab restores like a pack tab (`restore_pack_tab(s, resume, state, doc: &TabDocument)`, `resume_pack.rs:105`, which now takes the cached document for `deck_tab`, `pack` and the `TabLink` owner), `host: Tab` until Task 7, no lease; `check_pack_doc(…, doc: &TabDocument)` (`resume_pack.rs:222`, today it refuses every deck-only pack with host `Tab`, `:246`) fails closed: a deck-pack restore requires `edit.deck_only` and the cached document's `pack` equal to `unforwarded.base.pack` when the state is unforwarded, else to `edit.pack`, and keeps refusing a deck-only `Pack` document. `unforwarded: Some(base)` in its state rebuilds an outbox at wake from the preview's embedded session version with that base, marked due; when no session version survives `check_pack_doc`, the tab shows `notice-deck-pack-unforwarded-lost` (logged). `settle_lease` (`pack_mode_lease.rs:75`) answers `Settled::Free` for a deck-pack tab (it has a `TabLink`), never `LeaseMissing` (`:90`); its argument count stays at 5 by reading the link from `self.mode`. **Re-seed:** a link added by `set_deck_links` for a newly named tab requires the owner's current pin of the layout's `pack` to be deck-scoped (fail closed) with no library current, else the link is dropped (`warn`); it gets `expect = pin`, `deck_uid` = the owner's, `last_seq = None`, `parent = Reseeded { pin, pin_parent: <the pinned manifest's parent> }` — owner-side data only; a refused re-seed is returned to the workspace, which closes that tab (decision 5). The owner's undo stack is empty after resume (ADR-0002), so resume tests assert refusals of writes, not of undo.
14. **Single-document hosts open library pack tabs in place** (Task 7). `Session::open_pack_tab_here(pack, widget)` (public, modelled on `open_theme_here`, `theme_tab.rs:107`) turns a fresh session into a library pack tab holding the lease; deck-only refused (`DeckOnly`). The `impl Sessions for Mutex<Session>` `open_pack` (`agent/tabs.rs:105`) answers `Here` when its session already edits that pack (`route_pack` → `Here`), else `SingleDocument`. MCP tests gain `start_pack_tab(pack)` beside `start_theme_tab` (`tests/common/mod.rs:173`); the agent harness's `open_pack` (`crates/oku-slides-agent/tests/common/mod.rs:313-324`) mirrors the `Mutex<Session>` rule (`Here` when its session already edits that pack, else `SingleDocument`), and tests that need a pack session start one with `open_pack_tab_here` on a fresh session (`stack.rs:675,709` checked to start fresh). `Session::open_pack_here`, `enter_here`, `edit_pack`, `close_pack`, `Request::{EditPack, ClosePack}` and the MCP `close_pack` tool are deleted.
15. **Leftovers settled by removal:** `PackHost` (Rust and the TS type, `packTypes.ts:65-66`) and `PackMode.host` are deleted; `Mode::Pack.stash` is renamed `preview` (Task 2; `Mode::History { stash }` keeps its name); `TabInfo.editing_pack` is deleted (Task 8; the UI reads `document.kind`); the lot-3 review's leftover (an in-deck workshop's agent reaching `save_copy` and `new_deck`) disappears with the workshop.

## File Structure

Session (`crates/oku-slides-session/src/`):
- Create `deck_pack_tab.rs` — the tab side: `DeckPackSend`, `SendBase`, `Outbox`, `OutboxState`, `TabLink`, `DECK_PACK_SEQ`, `Session::{init_deck_pack_tab, take_deck_pack_send, acknowledge_deck_pack, park_deck_pack, flush_deck_pack, mark_due, fill_outbox, unsent, deck_pack_unforwarded}`. Tests: `deck_pack_tab_tests.rs`.
- Create `deck_pack_link.rs` — the owner's link lifecycle and refusals: `DeckLink`, `LinkParent`, `LinkSpec`, `LinkedPins`, `Session::{link_deck_pack, unlink_deck_pack, set_deck_links, refuse_linked, refuse_linked_pins, linked_pins, take_link_pull}`. Tests: `deck_pack_link_tests.rs`, `deck_pack_undo_tests.rs`, `deck_pack_refuse_tests.rs`.
- Create `deck_pack_receive.rs` — `Received`, `deck_pack_merge_key`, `Session::receive_deck_pack` (`validate`, `apply`, copies), `settle_answer`. Tests: `deck_pack_validate_tests.rs`, `deck_pack_receive_tests.rs`.
- Create `workspace_deck_packs.rs` — `acquire`, `with_quiet`, `open_deck_pack`, `forward`, `pull_linked`, `linked_tabs`, `deck_links_of`, `retarget_deck_pack`, `take_touched`, `drain_touched`, `PendingLink`. Create `workspace_deck_packs_close.rs` — `flush_linked`, `flush_all_deck_packs`, `close_with_links`, `Closed`. Tests: `workspace_deck_packs_tests.rs`, `workspace_deck_packs_close_tests.rs`, `workspace_deck_packs_resume_tests.rs`.
- Create `resume_workshop.rs` (Task 6, included from `resume.rs` like `resume_pack.rs`) — `SavedPack`'s legacy host, `LegacyHost`, `PackResume`, `classify`, `reopen_on_deck`. Tests: `resume_workshop_tests.rs`. `SavedPack { edit, unforwarded: Option<SendBase> }` itself is created in Task 5 in `resume_pack.rs` (`SendBase` derives `Serialize`/`Deserialize`).
- Modify: `lib.rs` (`Mode::Pack { preview, edit, scratch, lease, orphaned, link: Option<Box<TabLink>> }`, `Session.deck_links`, `Session.link_pull`, modules, re-exports), `tab_document.rs`, `views.rs` (`EditingView.unsent`), `pack_mode.rs`, `pack_mode_save.rs`, `pack_mode_lease.rs`, `pack_tab.rs`, `undo.rs`, `widgets_edit.rs`, `widgets_deck.rs`, `deck_widgets.rs`, `deck_packs.rs`, `merge.rs`, `history.rs`, `request.rs`, `request_class.rs`, `error.rs`, `resume.rs`, `resume_pack.rs`, `workspace.rs`, `workspace_session.rs`, `workspace_slot.rs`, `workspace_packs.rs`, `workspace_themes.rs`, `workspace_carry.rs`, `workspace_help.rs`, `workspace_same_deck.rs`, `workspace_tabs.rs`, `workspace_windows.rs`, `workspace_resume.rs`, `workspace_agent.rs`, `agent/tabs.rs`, `agent/context.rs`, `test_support*.rs`, the stash users listed in Task 2.

Elsewhere: `crates/oku-slides-mcp/src/{handler.rs, edit_tools.rs, tools.rs}` and tests; `crates/oku-slides-local-agent/src/families.rs` + tests; `crates/oku-slides-agent/src/instructions/agent.md` + `instructions_tests.rs`, `crates/oku-slides-agent/tests/common/mod.rs`; `src-tauri/src/{commands.rs, state.rs}`; `crates/oku-slides-cli/src/serve.rs`; i18n `crates/oku-slides-i18n/src/locales/*/session.ftl`; `ui/src/{api.ts, packTypes.ts, workspaceApi.ts, Workspace.tsx, App.tsx, packGestures.ts, close.ts, closeFlow.ts, TabBar.tsx, usePackWidgetEditor.ts}`, their tests and `ui/src/test/closeFlowFake.ts`; `ui/e2e/{220-import.cjs, lib/atelier.cjs}`; `docs/aide/{fr,en}.json`; `docs/adr/0013-deck-pack-tabs.md` + `docs/adr/README.md`; the spec; README lines 110 and 201; `AGENTS.md` / `ARCHITECTURE.md` of session, mcp and agent, `ui/AGENTS.md`.

---

### Task 1: Deck-mode pack writes keep the version they build on (t-1224)

**Files:** a regression `tests/regressions/issue_1224_deck_pack_write_keeps_its_parent.rs` declared in `tests/regressions/main.rs`; `tests/common/mod.rs` (new helper `add_user_pack(dir: &Path, name: &str) -> Uuid`, a user widget pack with one widget written to the data dir's library, beside `add_user_theme`, `:124`); `widgets_deck.rs` (`write_deck_widgets`, `:282`), `deck_adopt.rs` (`adoptable`), `widgets_deck_tests.rs`; session `AGENTS.md` line 21.

The suspected exposure: a deck-mode widget write builds `v2` on the version the deck pins and saves it with `DraftWrite::New`; if that pinned version were a dormant pack tab's unadopted draft, the tab's next save would replace and delete it, leaving `v2`'s parent dangling. At HEAD every public path that makes a deck pin a version runs `refresh_pack_updates` → `adopt_pins` (`deck_packs.rs:294-296`, `deck_adopt.rs:18-40`; load, first `Snapshot`, undo, merge and history too), which freezes it first; only a failed `adopt` (logged, `deck_adopt.rs:37`) leaves it unfrozen.

- [ ] **Step 1: Guard test** (`issue_1224_deck_pack_write_keeps_its_parent.rs`, public API only, modelled on `issue_1223_deck_theme_edit_freezes_its_parent.rs`), in a spawned thread with `recv_timeout`:
  1. `add_user_pack(dir, "acme")`; a workspace (`workspace_support::workspace`) on that data dir with a deck tab `t-1` whose deck pins `acme` at its current `v0`: `on(ws, "t-1", |s| s.add_library_pack(PackKind::Widgets, &ns("acme")))` (`deck_packs.rs:145`; `workspace_support::on`, `:34`; `send`, `:25`, takes a workspace request as JSON, not a session `Request`).
  2. `WorkspaceRequest::OpenPack { tab: "t-1", pack: acme }` opens pack tab `t-2`; set a widget there and `SavePack` → `v1`, a draft, current.
  3. `settle`, reopen the workspace on the same dir: `t-2` is dormant (no lease; on resume `freeze_base` runs only without a session version, `resume_pack.rs:167`, so `v1` stays unfrozen by the tab).
  4. In `t-1`, accept the update to `v1`: `on(ws, "t-1", |s| s.pack_use_mine(PackKind::Widgets, &ns("acme")))` (`deck_packs.rs:187`, moves the deck to the library's current; **not** `pack_make_current`, `:208`, which leaves the deck's pin unchanged); then write a widget of `acme` in deck mode (`Request::SetWidget`) → `v2`.
  5. Wake `t-2` (a `Snapshot`), set a widget and `SavePack` → `v3`.
  Assert: `PackLibrary::version(acme, v1)` is `Some` (`library.rs:340`), `v2`'s manifest `parent` is `v1`, and `v3`'s manifest `parent` is `v1` (a sibling, not a replacement of `v1`: `adopt_session_version` writes `Child` when `v1` is frozen and `Replace` otherwise, `library_draft.rs:80-86`, so these asserts tell the two apart).
- [ ] **Step 2: Run it.** Expected: PASS at HEAD (the deck's pin of `v1` froze it). Write in the review notes that `adopt_pins` on every pin change is the safety. If it FAILS, stop and report: the diagnosis was wrong (G15).
- [ ] **Step 3: Defence in depth.** `write_deck_widgets` (`widgets_deck.rs:282`, called at `:272` only for library-scope packs) gains `parent: Option<&PackVersion>` (6 arguments with `self`). Under the same `draft_lock`, after the lease check and before `draft_write`, **only when `replaced` is `None`** (the first write of a widget session; a later write's base is the session's own draft, which `adopt_pins` skips on purpose, `deck_adopt.rs:26-28`, and which must stay a draft), it adopts the parent's files from `self.doc.sources.packs.get(parent)` under `adopt_pins`' guards, factored into one `pub(crate) fn adoptable(&self, pin_id, version) -> Option<&PackFiles>` in `deck_adopt.rs` that both call (not a widget draft, listed by the library, not frozen, not deck-only, not built-in; an unlisted, received version is left alone so it keeps its `received` mark, `library_draft.rs:114-118`). On `Err` it logs `error`, sets `session-widgets-not-saved` and returns without writing (the function returns `()`; never a write over an unfrozen parent). Unit tests in `widgets_deck_tests.rs`: `deck_mode_widget_write_freezes_an_unfrozen_parent` (a draft `v1` written with `draft_write`/`save_draft`, pinned by writing the deck's sources directly — crate-internal, without `refresh_pack_updates`, so `adopt_pins` does not run — `library.is_frozen(acme, &v1)`, `library_draft.rs:66`, false; a deck-mode widget write → true); the existing `two_widget_writes_in_one_session_keep_one_version_that_stays_a_draft` (`:483`) still passes, named in the review notes. Check every other deck-mode library write found by `grep -n "save_draft\|set_current\|add_version" crates/oku-slides-session/src/*.rs` and name each in the review notes (safe and why, or fixed).
- [ ] **Step 4: Rule.** Session `AGENTS.md` line 21: "freeze with adopt every library version a deck in deck mode pins (…)" gains "and the parent of a deck-mode write" inside the parenthesis (same line).
- [ ] **Step 5: Gate**, then commit `fix(session): deck-mode pack writes freeze their parent before writing (t-1200, t-1224)`.

### Task 2: `Mode::Pack.stash` is renamed `preview` (t-1234)

No behavior change. **Files:** `lib.rs` (`Mode::Pack`'s field; `lib.rs:524,535,575,603`), and every `Mode::Pack` pattern binding `stash`: `pack_mode.rs:271-389`, `pack_mode_save.rs:225-265`, `pack_tab.rs:277,281`, `theme_composed.rs:54,72`, `deck_theme.rs:384-413`, `widget_preview.rs:176`, `pack_mode_lease.rs:54`, `agent/turn_undo.rs:58`, `agent/context.rs:111` (`:134` is the host line, Task 7's), `resume.rs:806`, `resume_pack.rs:174,176`, `test_support.rs:621` (`stash_of` → `preview_of`) and their callers, and the tests that bind it: `resume_pack_legacy_tests.rs:81,93,105`, `widgets_edit_tests.rs:67,282,286,296`; session `ARCHITECTURE.md:9` ("set the deck aside (`stash`; …)" → "(`preview` for a pack, `stash` for history; …)", same line). `Mode::History { stash }` keeps its name; the or-patterns `Mode::Pack { .. } | Mode::History { .. }` bind `Mode::Pack { preview: stash, .. } | Mode::History { stash, .. }` (permanent).

- [ ] **Step 1:** rename; the check `grep -rn "Mode::Pack {[^}]*\bstash\b\|stash_of" crates/oku-slides-session` ends with only the `preview: stash` or-patterns; comments that say "stash" for the pack's preview deck say "preview".
- [ ] **Step 2: Gate** (`cargo nextest run --workspace --exclude oku-slides-app`, clippy), then commit `refactor(session): Mode::Pack.stash becomes preview (t-1200, t-1234)`.

### Task 3: A deck-only pack is edited in a pack tab with an outbox (t-1226) — staged

**Files:** create `deck_pack_tab.rs`, `deck_pack_tab_tests.rs`; modify `lib.rs` (`Mode::Pack` gains `link`; module; `unsaved()` one-line `|| self.unsent()`), `tab_document.rs` (+ `tab_document_tests.rs`), `views.rs` (`EditingView.unsent`, `TabInfo.dirty` doc), `pack_mode.rs` (`enter_pack` signature below; `realign_pack` calls `fill_outbox`), `pack_mode_save.rs` (`adopt_session_version` calls `fill_outbox` after the stash branch), `pack_mode_lease.rs` (`settle_lease`, `:75`: a deck-pack tab — a `TabLink` in `self.mode` — is `Settled::Free`, never `DeckOnly` nor `LeaseMissing`, `:90`; arguments unchanged), `pack_tab.rs` (`preview_seed()` extracted from `take_preview`, `:159-174`; `pack_tab_info` reports `DeckPack` with `base.pack` until acknowledged, its title through `label`), `request.rs` (`SavePack` calls `mark_due` after a successful `save_pack`), `agent/context.rs` (`end_turn` calls `mark_due`); `ui/src/workspaceApi.ts` (`TabDocument` union gains `{ kind: "deck_pack"; deck_tab: string; pack: string }`, `tabPath` arm `null`) + `workspaceApi.test.ts` (`tabPath of a deck_pack tab is null`) and `ui/src/packTypes.ts` (`PackMode.unsent: boolean`).

**Interfaces — produces:**
```rust
// tab_document.rs
/// A pack embedded only in one deck, edited in its own tab.
DeckPack {
    /// The owner deck's tab id.
    deck_tab: String,
    /// The edited pack's id.
    pack: Uuid,
},
/// The owner deck's tab, for a deck-pack tab.
#[must_use] pub fn deck_tab(&self) -> Option<&str>;   // path(), deck_uid(), help_locale() answer None

// pack_mode.rs (until Task 7 removes `host`)
pub(crate) fn enter_pack(&mut self, pack: Uuid, host: PackHost, lease: Option<PackLease>, link: Option<Box<TabLink>>) -> Result<()>;
// a deck-pack tab enters with host: PackHost::Tab, lease: None, link: Some(..); a deck-only pack without a link keeps today's paths

// pack_tab.rs
/// The seed of a preview deck copied from this deck.
pub(crate) fn preview_seed(&mut self) -> Result<Box<PreviewSeed>>;

// deck_pack_tab.rs
/// Sequence numbers of deck-pack sends, unique for the process.
static DECK_PACK_SEQ: AtomicU64 = AtomicU64::new(1);

/// The pack id and parent a deck-pack save was built on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SendBase {
    /// The tab's pack before the save.
    pub pack: Uuid,
    /// The parent its saves carry.
    pub parent: Option<PackVersion>,
}
/// A deck-pack tab's saved pack, waiting for its owner deck.
#[derive(Debug)]
pub(crate) struct DeckPackSend {
    /// Unique per process; the owner ignores one below its last.
    pub seq: u64,
    /// What the save was built on.
    pub base: SendBase,
    /// The saved version's files; its closure resolves on the owner.
    pub files: PackFiles,
}
/// Where an outbox stands with its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutboxState {
    /// Saved, not yet asked to go.
    Idle,
    /// To forward at the next public call.
    Due,
    /// Taken by a forward that has not answered.
    InFlight,
    /// The owner was busy: waits for its pull.
    Parked,
    /// Refused: waits for the next save, flush or turn end.
    Refused,
}
/// The last save of a deck-pack tab not yet acknowledged.
#[derive(Debug, Clone)]
pub(crate) struct Outbox {
    /// What the oldest unacknowledged save was built on.
    base: SendBase,
    /// The latest saved files.
    files: PackFiles,
    /// The send built when it became due.
    send: Option<Arc<DeckPackSend>>,
    /// Where it stands.
    state: OutboxState,
    /// A save landed while in flight: send again after the answer.
    newer: bool,
    /// The owner pulled while in flight: a park goes straight to due.
    repull: bool,
}
/// What a deck-pack tab knows of its owner and its unsent work.
#[derive(Debug, Clone)]
pub(crate) struct TabLink {
    /// The owner deck's tab id.
    pub owner: String,
    /// The unacknowledged save, if any.
    outbox: Option<Outbox>,
}
impl Session {
    /// Turns this fresh session into a deck-pack tab of `owner` on `pack`.
    pub(crate) fn init_deck_pack_tab(&mut self, seed: Box<PreviewSeed>, owner: &str, pack: Uuid, widget: Option<&str>) -> Result<()>;
    /// Takes the due send and marks it in flight.
    pub(crate) fn take_deck_pack_send(&mut self) -> Option<Arc<DeckPackSend>>;
    /// The owner's answer to send `seq`: clears the outbox, or notices the refusal.
    pub(crate) fn acknowledge_deck_pack(&mut self, seq: u64, answer: Result<(), &SessionError>);
    /// Parks send `seq`: the owner was busy.
    pub(crate) fn park_deck_pack(&mut self, seq: u64);
    /// Makes a parked send due again (the owner's pull).
    pub(crate) fn unpark_deck_pack(&mut self);
    /// Saves when dirty, then marks the outbox due.
    pub(crate) fn flush_deck_pack(&mut self) -> Result<()>;
    /// Makes an idle, refused or parked outbox due, building its send.
    pub(crate) fn mark_due(&mut self);
    /// Takes the session version just adopted, keeping the oldest base.
    pub(crate) fn fill_outbox(&mut self, base: SendBase, files: &PackFiles);
    /// Whether this tab holds a save its owner has not acknowledged.
    #[must_use] pub(crate) fn unsent(&self) -> bool;
    /// For the state file: the base of an unacknowledged save.
    #[must_use] pub(crate) fn deck_pack_unforwarded(&self) -> Option<SendBase>;
}
```
`acknowledge_deck_pack`, `park_deck_pack` and `fill_outbox` end with `self.persist(false)`. `unpark_deck_pack` on an `InFlight` outbox sets `repull`; `park_deck_pack` with `repull` set makes it `Due` at once. An `Ok` with `newer` set makes it `Due` with a fresh send based on the tab's current `(edit.pack, edit.parent)`.
`init_deck_pack_tab` mirrors `init_pack_tab` (`pack_tab.rs:209`): fresh session required (`PackTabNotFresh`), preview installed from the seed, `enter_pack(pack, PackHost::Tab, None, Some(link))`; `widget` opens that widget's sub-tab as `init_pack_tab` does. On any error the session stays a pristine deck tab, as `init_pack_tab`. `mark_due` builds the send only when there is none or the files changed since (`seq` = `DECK_PACK_SEQ.fetch_add(1, Ordering::Relaxed)`; the send carries no closure files, decision 8). `acknowledge_deck_pack` and `park_deck_pack` act only when `seq` equals the outbox's send; `Ok` clears it; `Err(e)` sets `Refused`, `set_notice(e.localize(..))`, `tracing::warn!`. `PackMode.deck_only` is `true` in a deck-pack tab.

- [ ] **Step 1: Write the failing tests** (`deck_pack_tab_tests.rs`, `tab_document_tests.rs`), on a hifi import deck (`test_support::adopt_deck_only_pack`, `test_support.rs:294`) and a second fresh session from the owner's seed by `test_support_packs::deck_pack_tab_of(owner: &mut Session, pack) -> Session` (uses `preview_seed()`; Task 4 swaps it for `link_deck_pack`):
  - `deck_pack_tab_info_names_its_owner_and_pack` — `document` = `DeckPack { deck_tab: "t-1", pack }`, `title` = the pack's name.
  - `deck_pack_save_fills_a_due_outbox_and_writes_no_library_version` — `SavePack` → `take_deck_pack_send()` is `Some`, scope `Deck`, `base` = `{ pack, parent: Some(pinned) }`; `WRITES` unchanged; the library has no version of the pack.
  - `deck_pack_widget_write_fills_the_outbox_but_is_not_due` — `set_widget` → `take_deck_pack_send()` is `None`; after `SavePack`, `Some` carrying that widget.
  - `deck_pack_taken_send_is_not_taken_twice` — `SavePack`, take → `Some`; take again → `None`.
  - `deck_pack_seq_never_repeats_after_acknowledgement` — save, take, acknowledge `Ok`, save, take → the second seq is above the first.
  - `deck_pack_tab_is_unsent_until_acknowledged` — after `SavePack`, `tab_info().dirty` and `snapshot.editing.unsent` true, `snapshot.dirty` false; after acknowledge `Ok`, all false.
  - `deck_pack_refused_send_waits_for_the_next_save_and_notices` — acknowledge `Err(&mismatch)` → take is `None`, `unsent` true, the notice is the error's `fr` text; after the next `SavePack`, take is `Some`.
  - `deck_pack_parked_send_goes_after_unpark` — take, `park_deck_pack(seq)` → take `None`, no notice; `unpark_deck_pack()` → take `Some` with the same seq.
  - `deck_pack_parked_send_is_due_after_flush` — take, park; `flush_deck_pack()` → take `Some`.
  - `deck_pack_unpark_while_in_flight_makes_the_park_due` — take (in flight), `unpark_deck_pack()`, then `park_deck_pack(seq)` → take `Some`.
  - `deck_pack_save_while_in_flight_sends_again_after_acknowledgement` — save, take (s1), save, `acknowledge_deck_pack(s1, Ok)` → take `Some` with a seq above s1 carrying the second save's files; a take before the acknowledgement is `None`.
  - `deck_pack_second_save_keeps_the_first_unacknowledged_base` — save as copy, take, park, save again → the next send's `base.pack` is the original pack, not the copy.
  - `deck_pack_undo_fills_the_outbox_with_the_version_before_the_last_edit` — save (v1), edit, save (v2), acknowledge, `Undo` (`realign_pack`, no `SavePack`), `mark_due()` → the send's `files.version()` is v1 (versions compared, never bytes: `created` differs).
  - `deck_pack_end_turn_marks_the_outbox_due`.
  - `deck_pack_flush_saves_a_dirty_tab_first` — an unsaved edit, `flush_deck_pack()` → a due send carrying it.
  - `init_deck_pack_tab_on_a_used_session_is_refused` (`PackTabNotFresh`).
  - `deck_pack_tab_document_round_trips_serde` (`tab_document_tests.rs`): `{"kind":"deck_pack","deck_tab":"t-1","pack":"…"}`.
- [ ] **Step 2: Run them; they fail to compile** (`cargo nextest run -p oku-slides-session deck_pack_tab tab_document`).
- [ ] **Step 3: Implement** as the interfaces say. `TabDocument::DeckPack` arms in every match `cargo check` reveals (`read_info` refuses the kind until Task 5, one comment naming Task 5).
- [ ] **Step 4: Run** `cargo nextest run -p oku-slides-session`; all pass. `cargo check --workspace`; UI gate.
- [ ] **Step 5: Stage** (no commit); the review notes list the dead-code findings Task 5 resolves.

### Task 4: The owner deck receives, locks its linked pack and stops undo (t-1227) — staged

**Files:** create `deck_pack_link.rs`, `deck_pack_receive.rs`, `deck_pack_link_tests.rs`, `deck_pack_validate_tests.rs`, `deck_pack_receive_tests.rs`, `deck_pack_undo_tests.rs`, `deck_pack_refuse_tests.rs`; modify `lib.rs` (`Session.deck_links: BTreeMap<String, DeckLink>`, `Session.link_pull: bool`, `Session.links_generation: u64`; modules), `undo.rs` (decision 1; a `#[cfg(test)] thread_local! { static FAIL_OPPOSITE: Cell<bool> }` read only under `cfg(test)`, documented), `widgets_edit.rs` and `widgets_deck.rs` (one-line `refuse_linked` calls beside `refuse_leased`), `deck_widgets.rs` (`apply_ops`), `deck_packs.rs` (`remove_deck_pack`), `merge.rs` (`merge_file`: absorb on a clone, compare `linked_pins()`), `history.rs` (`history_restore`; `back_to_deck`, `:169`, sets `link_pull` when the deck has links), `agent/context.rs` (`end_turn` sets `link_pull` when the deck has links), `error.rs` (`UndoInPackTab`, `DeckPackMismatch`, `UndoCompensationFailed`), i18n (three keys after `session-pack-open-in-tab`, 20 locales), `test_support_packs.rs` (`deck_pack_tab_of` uses `link_deck_pack`; `forward_to_owner(tab: &mut Session, tab_id: &str, owner: &mut Session) -> Result<Received>` does take / receive / acknowledge or park, exactly the answer mapping of decision 7 — the workspace's `forward` (Task 5) calls the same `pub(crate) fn settle_answer(tab: &mut Session, seq, answer: &Result<Received>)` so the mapping is written once).

**Interfaces — produces:**
```rust
/// What the owner must accept as a linked tab's parent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkParent {
    /// Fixed by the link's start or its last applied send.
    Known(Option<PackVersion>),
    /// Rebuilt at resume: the pin itself or its manifest's parent.
    Reseeded {
        /// The version pinned when the link was rebuilt.
        pin: PackVersion,
        /// That version's manifest parent.
        pin_parent: Option<PackVersion>,
    },
}
/// A deck-pack tab linked to this owner deck, keyed by its tab id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeckLink {
    /// The tab's title, cleaned with `label`, for refusals.
    pub title: String,
    /// The pack it edits (a copy's id once accepted).
    pub pack: Uuid,
    /// This deck's `replica().deck_uid()` when the link was made.
    pub deck_uid: String,
    /// What the tab's sends must carry as parent.
    pub parent: LinkParent,
    /// The version this deck must pin for it now.
    pub expect: PackVersion,
    /// The last applied seq; `None` before the first.
    pub last_seq: Option<u64>,
    /// Recorded by `link_deck_pack`, not yet named by the layout.
    pub pending: bool,
}
/// A link as the workspace layout names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkSpec {
    /// The deck-pack tab's id.
    pub tab: String,
    /// Its cleaned title.
    pub title: String,
    /// The pack its cached document names.
    pub pack: Uuid,
}
/// A linked pack's pins in both kinds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct LinkedPins {
    /// Its widgets pin, if any.
    pub widgets: Option<PackVersion>,
    /// Its layouts pin, if any.
    pub layouts: Option<PackVersion>,
}
/// What the owner did with a send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Received {
    /// Applied as (part of) one undo step.
    Applied,
    /// Already applied: acknowledge it.
    Delivered,
    /// Older than the last applied send: ignored.
    Stale,
    /// In an agent turn or history: retried at the owner's pull.
    Busy,
}
/// The merge key of one deck-pack tab's editing session.
pub(crate) fn deck_pack_merge_key(tab: &str) -> String;
impl Session {
    /// Starts linking `tab` to deck-only `pack`: records it pending, returns the seed.
    pub(crate) fn link_deck_pack(&mut self, tab: &str, title: &str, pack: Uuid) -> Result<Box<PreviewSeed>>;
    /// Drops `tab`'s link, pending or not.
    pub(crate) fn unlink_deck_pack(&mut self, tab: &str);
    /// Takes the layout's links; returns the tabs whose new link was refused.
    #[must_use] pub(crate) fn set_deck_links(&mut self, generation: u64, links: &[LinkSpec]) -> Vec<String>;
    /// Validates then applies a linked tab's send (`deck_pack_receive.rs`).
    pub(crate) fn receive_deck_pack(&mut self, tab: &str, send: &DeckPackSend) -> Result<Received>;
    /// Refuses a write of `pack` while a tab of this deck edits it.
    pub(crate) fn refuse_linked(&self, pack: Uuid) -> Result<()>;
    /// Refuses combined `pins` when they move a linked pack's pin, by id.
    pub(crate) fn refuse_linked_pins(&self, pins: &BTreeMap<Namespace, PackPin>) -> Result<()>;
    /// Each linked pack's pins in both kinds, deck mode only.
    #[must_use] pub(crate) fn linked_pins(&self) -> BTreeMap<Uuid, LinkedPins>;
    /// Whether a turn end or history close asked for parked sends; clears it.
    pub(crate) fn take_link_pull(&mut self) -> bool;
}
// error.rs
UndoInPackTab { pack: String, tab: String, backtrace: Backtrace },
DeckPackMismatch { pack: String, backtrace: Backtrace },
UndoCompensationFailed { backtrace: Backtrace },
```
fr texts:
- `session-undo-in-pack-tab = Cette étape change le pack { $pack }, en cours d'édition dans l'onglet « { $tab } » : annulez-la dans cet onglet, ou fermez-le d'abord.`
- `session-deck-pack-mismatch = Le pack { $pack } ne correspond plus à ce que la présentation embarque : ses changements restent dans son onglet.`
- `session-undo-compensation-failed = L'annulation n'a pas pu être défaite : rouvrez la présentation avant de continuer.`

`link_deck_pack`: deck mode only (`WrongDocument`); the deck's current pin of `pack` is deck-scoped (`deck_scope == Some(true)`) and the library has no current version of it, else `UnknownPack`; another link already on `pack` → `PackOpenInTab` (the workspace focuses that tab first: this guards a race); ends the deck's widget session first (as `take_preview`); `preview_seed()`; records `DeckLink { pending: true, parent: Known(Some(pin)), expect: pin, last_seq: None, deck_uid: <the deck's>, .. }`. `set_deck_links` follows decision 5; a new link follows decision 13's re-seed. `refuse_linked` iterates the links (pending included).

- [ ] **Step 1: Write the failing tests** (each refusal also asserts the deck and `undo_steps()` unchanged):
  - `deck_pack_validate_tests.rs` (every case built from one `valid_send(&owner, tab)` helper changed in **exactly one** field or one owner fact, so a case cannot pass because an earlier check refused): `receive_deck_pack_valid_send_is_applied` (the positive control, the helper unchanged); `receive_deck_pack_first_send_is_applied` (seq 1 against `last_seq: None`); `receive_deck_pack_from_an_unlinked_tab_is_refused`; `receive_deck_pack_older_seq_is_stale`; `receive_deck_pack_same_seq_is_delivered`; `receive_deck_pack_of_the_expected_version_is_delivered`; `receive_deck_pack_during_an_agent_turn_is_busy` (`begin_turn`); `receive_deck_pack_in_history_is_busy`; `receive_deck_pack_after_the_owner_opened_another_document_is_refused` (`deck_uid` changed); rstest `receive_deck_pack_mismatch_is_refused` with cases: wrong `base.pack`; wrong `base.parent`; library-scope manifest; same id with another parent; same id another namespace; another kind; a copy whose id the library has; a copy whose id is built-in; a copy whose id the owner pins in the other kind; a copy with a parent; a copy under another namespace; a send whose closure would move another namespace's pin (asserting the owner's sources are byte-for-byte unchanged: refused before anything is written); the owner's pin moved from `expect` (set up by writing the owner's sources directly, crate-internal, since every public path is refused while linked); a pin the library now has a current version for; an owner near `total_bytes` whose total plus the send would pass it (each also asserting the deck and `undo_steps()` unchanged); `receive_deck_pack_back_to_the_opened_version_is_accepted`; `receive_deck_pack_outside_deck_mode_is_wrong_document` (a theme tab); rstest `deck_pack_errors_localize_in_fr` (one case per key).
  - `deck_pack_link_tests.rs`: `link_deck_pack_of_a_library_pack_is_refused`; `link_deck_pack_twice_on_one_pack_is_refused`; `set_deck_links_keeps_a_pending_link_until_named`; `set_deck_links_keeps_a_known_link_whatever_pack_the_layout_names`; `set_deck_links_drops_a_confirmed_link_no_longer_named`; `set_deck_links_reseeds_a_new_link_from_the_pin_and_its_parent`; `set_deck_links_returns_a_new_link_on_a_library_pin` (dropped and returned); `set_deck_links_ignores_an_older_generation` (unlink at generation 2, then a call at generation 1 naming the tab → no link).
  - `deck_pack_receive_tests.rs`: `deck_pack_save_reaches_owner_as_one_undo_step` (two sends, `undo_steps()` grows by one, the owner pins the second version and is dirty); `receive_deck_pack_drops_the_replaced_version_when_merged`; `receive_deck_pack_keeps_the_opened_version_after_a_send_back_to_it` (send v2, send the opened v1, send v3 → v1 still in the owner's sources); `receive_deck_pack_installs_the_new_widgets` (`sync_widgets`: a widget changed in the send renders in the owner); `receive_deck_pack_of_a_copy_replaces_the_original_pins_in_both_kinds` (the link names the copy; a later send of the copy is accepted); `receive_deck_pack_of_a_copy_changes_no_other_pin`; `receive_deck_pack_failed_render_keeps_the_change_and_notices`; `owner_history_restore_pulls_parked_sends` (`back_to_deck` sets the pull); `reseeded_link_accepts_the_tab_parent_then_fixes_it` (a link rebuilt with `Reseeded`: a send with `base.parent == pin_parent` is applied, then a send with `base.parent == pin` is refused).
  - `deck_pack_undo_tests.rs`: `owner_undo_stops_at_linked_pack_step` (link, receive, owner `Undo` → `UndoInPackTab`; `linked_pins()`, `undo_steps()`, redo depth and `dirty` unchanged); `owner_undo_of_a_pre_link_pack_step_is_refused` (a deck-mode write to `deck-layouts`, then link, then `Undo`); `owner_redo_of_a_linked_pack_step_is_refused`; `owner_undo_agent_turn_over_a_linked_pack_step_is_refused`; `owner_undo_of_another_step_still_passes`; `owner_undo_after_unlink_undoes_the_pack_step`; `owner_undo_compensation_failure_is_not_undo_in_pack_tab` (`FAIL_OPPOSITE` set → `UndoCompensationFailed`); `owner_receive_after_a_refused_undo_starts_a_new_step`; `owner_refused_redo_cuts_the_merge_of_the_step_below`.
  - `deck_pack_refuse_tests.rs` (rstest `owner_refuses_changes_to_a_linked_pack_pin`, each → `PackOpenInTab` with `tab` = the link's title, a pending link included): `set_widget_by_agent` on the linked `deck-layouts` pack; a raw `Apply` of `AddPack` on its namespace; a raw `Apply` of `AddPack` of its id under a new namespace; a raw `Apply` of `RemovePack`; `remove_deck_pack`; `merge_file` of a copy whose pin differs; `merge_file` of a bundle that pins the linked id under a second namespace; `history_restore` to a point before the receive; the deck's own-pack widget session. Plus `owner_writes_to_an_unlinked_pack_still_pass` and `owner_apply_of_unrelated_ops_still_passes`.
- [ ] **Step 2: Run; they fail.**
- [ ] **Step 3: Implement** decisions 1, 3, 8, 9 in `deck_pack_link.rs` and `deck_pack_receive.rs` (`replay(do_step, opposite)` in `undo.rs` calls one helper `stop_at_linked_pins(before)`).
- [ ] **Step 4: Rule, same staging.** Session `AGENTS.md` line 15 (the lease rule): after "keep an editor without the lease read-only," insert "make an owner deck refuse every change of the pin of a pack its linked deck-pack tab edits (undo and redo included)," and add `plan:2026-10-27-onglets-pack-theme-lot5-deck-pack-tabs` to its sources (same line).
- [ ] **Step 5: Run** `cargo nextest run -p oku-slides-session -p oku-slides-i18n`; all pass. **Stage** (no commit).

### Task 5: The workspace opens, links, forwards, closes and resumes deck-pack tabs (t-1228) — commits Tasks 3-5

**Files:** create `workspace_deck_packs.rs`, `workspace_deck_packs_close.rs`, `workspace_deck_packs_tests.rs`, `workspace_deck_packs_close_tests.rs`, `workspace_deck_packs_resume_tests.rs`; modify `workspace_session.rs` (`with_session` and `request` become one-line calls to `acquire`; English docs), `workspace_slot.rs` (`Slot::ready` private, `Slot::ready_with_links`), `workspace.rs` (`touched`, `layout_generation`, `discard_at_quit` fields; `apply`'s `Close` arm and pre-phase; `settle`), `workspace_layout.rs` (bump `layout_generation` on every layout change), `workspace_packs.rs` (`OpenCtx`, `Ask` `pub(super)`; `open_new` handles `PackRoute::DeckTab`; `candidates` / `focus_candidate` for `DeckPack`; `pack_opener` → `None` for a deck-pack tab; internal calls → `with_quiet`), `workspace_themes.rs`, `workspace_carry.rs`, `workspace_help.rs`, `workspace_same_deck.rs` (internal calls → `with_quiet` / links pushed), `pack_tab.rs` (`route_pack`, decision 10's table; `open_pack_here`'s `DeckTab` arm), `workspace_tabs.rs` (`Opening::DeckPack`, `Pending::build_deck_pack`, `to_wake`; `change`'s `Close` arm now unreachable like `CloseWindow`'s), `workspace_windows.rs` (`close_window`: linked tabs, `LastWindow` when all would close), `workspace_resume.rs` (`read_info`, the pre-pass, `restore_tab`), `resume_pack.rs` (decision 13's restore arm, `restore_pack_tab` and `check_pack_doc` taking the cached `TabDocument`, `SavedPack { edit, unforwarded: Option<SendBase> }` — `SavedMode::Pack(SavedPack)` in `resume.rs` is a one-line change, its struct literal at `resume.rs:806` uses `SavedPack::new`), `resume_tests.rs:165-181` (struct literals of the saved pack mode), `workspace_agent.rs`, i18n (`notice-deck-pack-unforwarded-lost` after `notice-theme-draft-dropped`, 20 locales), `src-tauri/src/{commands.rs, state.rs}`, `crates/oku-slides-cli/src/serve.rs`, `tests/common/workspace_support.rs` (helpers `open_pack(ws, tab, pack) -> String` and `import_hifi(ws) -> String` for the regressions), `workspace_tests.rs` (none: signatures kept), the Tasks 3-4 files; session `ARCHITECTURE.md`, mcp `ARCHITECTURE.md:14`, README line 110.

**Interfaces — produces:**
```rust
// workspace_session.rs (signatures unchanged; docs rewritten in English)
/// A request for tab `tab`; `true`: the tab bar must be redrawn.
///
/// Locks the workspace before and after, never during the call. A deck-pack
/// tab's due save is then forwarded to its owner deck: the call returns once
/// the owner has answered. Owners a forward changed go to `take_touched`.
///
/// # Blocking
/// Waits for the tab's session, then, for a forward, for its owner's.
pub fn request(ws: &Mutex<Self>, tab: &str, req: Request) -> (Result<String>, bool);
/// `f` on the tab's session, woken if needed; `true`: redraw the tab bar.
/// (same `# Blocking` section)
pub fn with_session<R>(ws: &Mutex<Self>, tab: &str, f: impl FnOnce(&mut Session) -> R) -> (Result<R>, bool);

// workspace_deck_packs.rs
/// Whether an acquisition forwards a due deck-pack save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Forward {
    /// Public calls: forward, then pull.
    Yes,
    /// Workspace-internal calls and the forward's own steps.
    No,
}
impl Workspace {
    /// The one session acquisition: links pushed, `f`, then the forward if asked.
    fn acquire<R>(ws: &Mutex<Self>, tab: &str, forward: Forward, f: impl FnOnce(&mut Session) -> R) -> (Result<R>, bool);
    /// `f` on a session with its links, without forwarding; `true`: its info changed.
    pub(super) fn with_quiet<R>(ws: &Mutex<Self>, tab: &str, f: impl FnOnce(&mut Session) -> R) -> (Result<R>, bool);
    /// The deck tabs a forward changed since the last call; drains them.
    #[must_use] pub fn take_touched(ws: &Mutex<Self>) -> Vec<String>;
    /// Drains touched owners: `emit` once if any, then `after` per owner.
    pub fn drain_touched(ws: &Mutex<Self>, emit: impl FnOnce(), after: impl FnMut(&str));
    /// The deck-pack tabs whose owner is `deck_tab`, in window order.
    pub(super) fn linked_tabs(&self, deck_tab: &str) -> Vec<String>;
    /// The links of `tab` as the layout names them.
    pub(super) fn deck_links_of(&self, tab: &str) -> Vec<LinkSpec>;
    /// Hands `tab`'s taken send to its owner, then settles the answer.
    pub(super) fn forward(ws: &Mutex<Self>, tab: &str, send: Arc<DeckPackSend>);
    /// Forwards the parked sends of `owner`'s linked tabs.
    pub(super) fn pull_linked(ws: &Mutex<Self>, owner: &str);
    /// Opens or focuses `pack`'s deck-pack tab beside its owner `ctx.tab`.
    pub(super) fn open_deck_pack(ctx: &OpenCtx<'_>, window: String, name: String, ask: &mut Ask<'_>) -> Result<PackOpened>;
    /// Points a deck-pack tab's cached document at the accepted copy, then saves the layout.
    pub(super) fn retarget_deck_pack(&mut self, tab: &str, pack: Uuid);
    /// Closes the tabs whose link the owner refused, keeping their folders.
    pub(super) fn set_aside_unlinked(&mut self, tabs: &[String]);
}
// workspace_deck_packs_close.rs
impl Workspace {
    /// Flushes and forwards every tab linked to `deck_tab`, waking them.
    pub(super) fn flush_linked(ws: &Mutex<Self>, deck_tab: &str);
    /// Flushes every awake deck-pack tab not discarded; no-op once closed.
    pub(super) fn flush_all_deck_packs(ws: &Mutex<Self>);
    /// Closes `tab` with its linked tabs, or keeps all when that empties everything.
    pub(super) fn close_with_links(&mut self, tab: &str, discard: bool) -> Result<Closed>;
}
/// What closing a tab with its links did.
#[derive(Debug)]
pub(super) enum Closed {
    /// These tab ids left the layout; their slots are in `trash`.
    Gone(Vec<String>),
    /// Every tab would have closed: nothing was.
    LastWindow,
}
/// A pending deck-pack link to cancel unless the tab reaches the layout.
#[derive(Debug)]
pub(super) struct PendingLink<'a> {
    /// The workspace to reach the owner through.
    ws: &'a Mutex<Workspace>,
    /// The owner deck's tab.
    owner: String,
    /// The tab being opened.
    tab: String,
    /// Cleared by `disarm` (in the layout) or `cancel` (unlinked).
    armed: bool,
}
impl PendingLink<'_> {
    /// The tab is in the layout: nothing to undo.
    pub(super) fn disarm(mut self);
    /// Unlinks the tab from its owner; call with no workspace guard held.
    pub(super) fn cancel(mut self);
}
// Drop: only a panic net — `try_lock` on the workspace, `warn` when held, never blocks.
// workspace_tabs.rs
/// A deck-only pack's tab, linked to its owner deck.
Opening::DeckPack {
    /// The owner deck's tab.
    deck_tab: String,
    /// The edited pack.
    pack: Uuid,
    /// The widget to open first, if any.
    widget: Option<String>,
},
/// Builds a deck-pack tab session from its owner's seed.
fn build_deck_pack(&self, seed: Box<PreviewSeed>) -> Result<Session>;
// pack_tab.rs
/// A deck-only pack of this deck: its own tab, linked to it.
PackRoute::DeckTab {
    /// The pack's name, cleaned with `label`.
    name: String,
},
// workspace_slot.rs
/// The ready session after handing it `links`, plus the tabs it refused.
pub(super) fn ready_with_links(&mut self, generation: u64, links: &[LinkSpec]) -> Result<(&mut Session, Vec<String>)>;
```
`acquire`: under the workspace lock copy `deck_links_of(tab)` and `layout_generation` (empty and cheap unless the tab owns deck-pack tabs); under the session lock `ready_with_links(generation, links)` (whose refused tabs are set aside by `set_aside_unlinked` once the session is released), `f`, then, for `Forward::Yes`, `take_deck_pack_send()` and `take_link_pull()`; after releasing the session, `forward` when a send was taken, `pull_linked(tab)` when the pull was set. `forward`: `with_quiet(owner)` → `receive_deck_pack(tab, &send)` (blocking on the owner's lock; a `#[cfg(test)]` hook `before_owner_lock` lets tests order themselves); an owner that is gone answers `UnknownTab`; then `with_quiet(tab)` → `settle_answer` (Task 4's helper); on `Applied`: a copy → `retarget_deck_pack` in the same call, the owner pushed onto `touched`, both infos refreshed (the owner's forced); `acquire`'s `bool` is true when the tab's info changed through the acknowledgement too. `with_quiet` keeps the `closed` check of `with_session`. `Pending::build_deck_pack` runs outside the workspace lock (a `#[cfg(test)]` failure flag makes it fail on demand); `PendingLink` is created right after `link_deck_pack` succeeds, **before any `MutexGuard<Workspace>` is taken**, disarmed once the tab is inserted; a failed build or insert returns it to `open_deck_pack`, which calls `cancel()` after the guard is dropped.

- [ ] **Step 1: Write the failing tests** (on a workspace fixture with a hifi deck tab, `workspace_packs_fixtures.rs`; every body in a spawned thread with `recv_timeout`):
  - `workspace_deck_packs_tests.rs`: `open_pack_of_a_deck_only_pack_opens_a_deck_pack_tab_after_its_owner` (document `DeckPack { deck_tab: owner, pack }`, right after, active, the owner unchanged in deck mode); `open_pack_twice_focuses_existing_tab`; `open_pack_from_the_owner_with_a_widget_focuses_it_on_that_widget`; `open_pack_from_the_deck_pack_tab_opens_the_widget_there`; `deck_pack_save_reaches_owner_through_the_workspace` (`request(pack_tab, SavePack)` → the owner pins the new version and is dirty, the pack tab not unsent, `take_touched()` == `[owner]`, then empty); `deck_pack_forward_wakes_a_dormant_owner` (settle, reopen, save in the pack tab first: the owner, never called since, receives); `resumed_deck_pack_tab_save_reaches_owner` (save → forward → settle → reopen → save → applied); `deck_pack_copy_through_the_workspace_retargets_the_tab` (the cached document's `pack` is the copy, a later save is applied); `agent_open_pack_of_a_deck_only_pack_counts_as_a_new_tab` (gate asked `NewTab`; refused → no tab and no pending link: an owner write of the pack passes); `failed_deck_pack_build_unlinks_the_owner` (the `#[cfg(test)]` failure flag in `build_deck_pack`; then an owner deck-mode write of the pack passes); `two_owners_of_one_pack_get_two_tabs` (the owner deck saved as a copy with `save_copy`, so both decks embed the same deck-only pack id; both opened, each opens its tab); `forward_waits_for_a_held_owner` (ordering, no timing: thread A holds the owner's session; `SavePack` on the pack tab runs in thread B; the `before_owner_lock` hook signals once B's send is taken; only then a `Snapshot` on the pack tab answers on the test thread; A sets a `released` flag then releases; B asserts `released` on return and `take_touched()` names the owner; channels and `recv_timeout` only); `refused_send_is_not_forwarded_again` (after a refusal, a thread holds the owner; a `Snapshot` on the pack tab answers, and the hook never fires); `forward_during_owner_turn_is_pulled_at_end_turn` (owner `begin_turn`, pack tab saves → parked, no notice; owner `end_turn` through `with_session` → the owner pins it); `internal_calls_never_forward` (a due send made crate-internally with `with_quiet(tab, mark_due)`, then a second `OpenPack` from the owner, which reaches `focus_candidate` → the hook never fires and `with_quiet(tab, take_deck_pack_send)` is still `Some`); `forward_is_persisted_on_both_sides` (forward, `settle`, reopen: the owner's file pins the new version and the tab's state has `unforwarded: None`); `links_reach_the_owner_through_carry_and_notices` (a `CarryAssets` and a `post_notice` on a woken owner, then an owner write of the pack is refused); `drain_touched_emits_once_then_runs_after_per_owner`; `owner_absorb_on_save_then_refuses_the_next_send` (an external change absorbed on save moves the pin; the tab's next send → refused, notice, tab unsent); `pack_opener_of_a_deck_pack_tab_is_none`; `note_refuses_a_changed_deck_tab` (the cached document kept); `note_keeps_the_pack_without_an_accepted_copy`.
  - `workspace_deck_packs_close_tests.rs`: `deck_pack_tab_closes_with_owner_deck`; `closing_owner_flushes_a_dirty_deck_pack_tab_first` (an unsaved edit in the pack tab → `Unsaved` for the owner, now dirty and pinning the flushed version; both tabs remain); `closing_owner_with_discard_closes_both_without_flushing`; `closing_owner_and_links_that_are_all_tabs_is_last_window` (nothing closed); `close_window_emptying_every_window_is_last_window` (window A the owner, window B only its linked tab); `close_window_closes_linked_tabs_in_other_windows` (an emptied window goes); `close_window_flushes_linked_tabs_first`; `closing_a_deck_pack_tab_alone_unlinks_and_activates_its_owner` (outcome `DeckChanged { tab: owner }` when it flushed; then an owner write of the pack passes); `closing_a_just_opened_deck_pack_tab_drops_its_pending_link`; `settle_flushes_deck_pack_tabs_before_closing` (an unsaved edit, `settle` → the owner's state on disk pins it); `second_settle_forwards_nothing`; `discarded_close_that_is_last_window_is_not_flushed_at_quit` (`Close { discard: true }` → `LastWindow`, then `settle` → the owner's file does not pin the discarded edit); `closing_owner_after_a_park_during_history_forwards_it` (owner in history, the tab parks; `history_restore`; closing the owner → applied, both closed); `ensure_closable_refuses_an_unsent_deck_pack_tab`.
  - `workspace_deck_packs_resume_tests.rs`: `resume_restores_a_linked_deck_pack_tab` (save, settle, reopen: a dormant `DeckPack`; once woken, the owner refuses a deck-mode write of the pack); `resume_unforwarded_deck_pack_tab_forwards_on_flush` (owner `begin_turn`, the pack tab saves → parked; `settle`, which does not end the turn, writes `unforwarded`; reopen; closing the owner flushes and applies it); `resume_unforwarded_copy_is_restored_and_applied` (save as copy while the owner is busy, `settle`, reopen → the tab restores, the cached document names the original, closing the owner applies the copy and both close); `unacknowledged_copy_then_save_is_applied` (copy while busy, save again, `end_turn` → `Applied`, the cached document names the copy); `resume_unforwarded_without_a_session_version_notices`; `resume_owner_closes_a_tab_whose_link_it_refused` (the owner's pin of the pack replaced in its file outside the app → the tab is set aside with `notice-tab-resume-failed`, every window keeps a valid `active`); rstest `resume_sets_aside_a_bad_deck_pack_entry` with cases: owner absent, owner a `Pack` tab, `deck_tab` not a tab id, `deck_tab` == own id, duplicate `(deck_tab, pack)`, cached `pack` ≠ the state's pack, a non-deck-only state → set aside, folder kept, notice shown, every window keeps a valid `active`; `legacy_reader_drops_only_the_deck_pack_entry` (as `workspace_resume_tests.rs` checks for themes); `pack_state_unforwarded_round_trips`.
  - Routing (`pack_tab_route_tests.rs`): `route_pack_of_a_deck_only_pack_from_a_deck_tab_is_deck_tab` (the deck unchanged: mode `Deck`, nothing persisted); `route_pack_of_another_deck_only_pack_from_a_pack_tab_is_in_pack_tab`; `route_pack_of_another_library_pack_from_a_pack_tab_is_tab`; `route_pack_of_a_deck_only_pack_without_lease_scope_is_refused`.
  - Hosts: `crates/oku-slides-cli` serve tests `api_forward_runs_after_change_on_the_owner` and `close_of_an_owner_whose_links_are_all_tabs_is_409`; Tauri and `AppSessions` call `drain_touched` (tested in session), with no logic of their own.
  - Rewritten here (name kept when true): `workspace_packs_tests.rs:164` rstest `deck_only` case (now a deck-pack tab); mcp `tests/edit_pack_tabs.rs:129` (a deck-only `edit_pack` from the agent opens a tab, through `tests/common/workspace_harness.rs`).
- [ ] **Step 2: Run; they fail.**
- [ ] **Step 3: Implement** decisions 5-7 and 10-13, then the hosts per decision 12 through `drain_touched`: `commands.rs` `request` and `workspace` (emit the view; `state.stack.after_change(owner)` each); `state.rs` `AppSessions::with` (emit, and `after_change` deferred through `tauri::async_runtime::spawn_blocking` with `app.state::<AppState>()`); `serve.rs` after `/api`, `/api/workspace` and `/api/agent`.
- [ ] **Step 4: Rules and docs, same commit.** Session `AGENTS.md` line 9 (lock order): "never hold two sessions (CarryAssets takes one, then the other)" → "never hold two sessions (CarryAssets and a deck-pack forward take one, then the other)"; line 33 (`Reach a tab session from the agent only via Workspace::with_session …`) gains ", which also forwards a deck-pack tab's due save; workspace-internal calls use with_quiet"; both add the plan to their sources (same lines). Session `ARCHITECTURE.md`: the `pack_tab.rs` mention in line 9 gains "a deck-only pack's tab (`deck_pack_tab.rs`) is linked to its owner (`deck_pack_link.rs`, `deck_pack_receive.rs`), forwarded by `workspace_deck_packs*.rs`", condensing the line's list of `pack_mode_*` files to "`pack_mode*.rs`" (no net line). mcp `ARCHITECTURE.md:14`: "a pack tab per turn or a deck-only workshop" → "a pack tab per turn, a deck-only pack's tab linked to its deck". README line 110 if it names the workshop (replaced text).
- [ ] **Step 5: Run** `cargo nextest run --workspace --exclude oku-slides-app` then `-p oku-slides-app`; all pass; the Tasks 3-4 dead-code findings are gone (`cargo clippy … -D warnings` clean).
- [ ] **Step 6: Gate**, then **one commit for Tasks 3-5**: `feat(session)!: deck-only packs open in a tab linked to their deck (t-1200, t-1226, t-1227, t-1228)`, body: `OpenPack` of a deck-only pack opens a `deck_pack` tab instead of the in-deck workshop; hosts drain `Workspace::take_touched`.

### Task 6: Old workshops reopen on their deck (t-1235)

**Files:** create `resume_workshop.rs`, `resume_workshop_tests.rs`; modify `resume_pack.rs` (`SavedPack` gains the legacy host; lot 4's v3-workshop restyle path and its theme-draft notice deleted), `resume.rs` (the guards at `:707-717` → one call to `resume_workshop::classify`; module line), i18n (`notice-workshop-reopened-on-deck` after `notice-theme-draft-dropped`, 20 locales), the tests that index `["host"]` and expect the old outcomes: `pack_tab_tests.rs:76`, `pack_tab_read_only_tests.rs:419` (a v5 workshop without `host` → now the pack-tab outcome, per decision 2), `test_support_packs.rs:213`, `resume_pack_tests.rs:192`, `resume_pack_refusal_tests.rs:68`.

**Interfaces — produces:**
```rust
// resume_pack.rs (from Task 5, extended here)
/// A saved pack mode with what older versions and the outbox add.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SavedPack {
    /// The pack being edited; its `host` is set from `classify` on read.
    #[serde(flatten)]
    pub edit: PackMode,
    /// The saved host, read here; `edit.host` writes it.
    #[serde(default, rename = "host", skip_serializing)]
    pub(super) legacy: Option<LegacyHost>,
    /// The base of a deck-pack tab's save its owner had not acknowledged.
    #[serde(default)]
    pub unforwarded: Option<SendBase>,
}
impl SavedPack {
    /// A pack state to write, from the live mode.
    #[must_use] pub(super) fn new(edit: PackMode, unforwarded: Option<SendBase>) -> Self;
}
// resume_workshop.rs
/// Where an older state edited its pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum LegacyHost {
    /// In its deck tab's workshop.
    Deck,
    /// In its own tab.
    Tab,
}
/// What a saved pack state resumes as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PackResume {
    /// A deck with its workshop open: reopened on the deck.
    Workshop,
    /// A pack tab.
    PackTab,
    /// Neither: set aside as unreadable.
    Unreadable,
}
/// Classifies a pack state by version and saved host.
///
/// v3 is a workshop; v4 a pack tab only with host `tab`; v5 a workshop
/// only with host `deck`, else a pack tab.
pub(super) fn classify(version: u32, legacy: Option<LegacyHost>) -> PackResume;
/// Restores a workshop's deck in deck mode, noticing dropped pack edits.
pub(super) fn reopen_on_deck(s: &mut Session, resume: &Resume, state: State) -> Result<()>;
```
"Dropped pack edits" is decided without a `dirty` flag (`DocHeader` has none, `resume_pack.rs:79-93`): the notice is shown when the `pack/` document's pack version (its session version if any, else its pinned version) differs from the version the restored deck pins for that pack id — the edits the user would lose, as decision 2 says ("if it differed").
Until Task 7, `edit.host` is set after reading (`PackTab` → `PackHost::Tab`; a live workshop is never written as a pack tab because `edit.host` still serializes the live host), so states keep carrying `host` until Task 7 deletes the field.

fr text: `notice-workshop-reopened-on-deck = Cet onglet éditait un pack dans sa présentation avec une version plus ancienne d'Oku Slides : il rouvre sur la présentation et les changements du pack non enregistrés n'ont pas pu être gardés. Un pack s'ouvre désormais dans son propre onglet.`

- [ ] **Step 1: Write the failing tests** (`resume_workshop_tests.rs`, states as JSON in their old shapes): rstest `classify_pack_state_by_version_and_host_resumes_as` (v3 any host → Workshop; v4 Tab → PackTab; v4 none / Deck → Unreadable; v5 Deck → Workshop; v5 none / Tab → PackTab); `resume_v3_workshop_reopens_on_deck` (mode `Deck`, document = `deck/`, `assert_unleased`); `resume_v5_deck_host_workshop_reopens_on_deck`; `resume_workshop_with_a_changed_pack_notices_the_dropped_edits` (the key, `fr`); `resume_workshop_with_an_unchanged_pack_says_nothing`; `resume_v5_pack_tab_without_host_is_a_pack_tab` (its `edit.host` is `Tab`); `resume_v4_deck_host_state_is_set_aside_as_unreadable` (end to end through the workspace: folder kept, notice); `resume_v4_pack_tab_keeps_its_theme_restyle`; `live_workshop_state_keeps_host_deck` (a live workshop saved by this build resumes on its deck, not as a pack tab).
- [ ] **Step 2: Run; they fail.** **Step 3: Implement**; delete lot 4's v3-workshop restyle and its tests (named in the review notes); update the five `["host"]` tests above.
- [ ] **Step 4: Rule, same commit.** Session `AGENTS.md` line 15's tail "reading pack states of versions 3 and 4 as legacy" → "reopening v3 and v5-deck workshops on their deck" (same line).
- [ ] **Step 5: Gate**, then commit `feat(session): workshops saved by older versions reopen on their deck (t-1200, t-1235)`.

### Task 7: The in-deck workshop, EditPack and ClosePack are removed (t-1229)

Two commits. **7a** moves every test off the workshop while it still exists; **7b** deletes it and migrates production consumers. Steps 1-2 below are 7b's; 7a is Step 0.

- [ ] **Step 0 (7a): migrate the tests.** Add `Session::open_pack_tab_here` first (its interface and two tests below, Step 1's first two), so library pack sessions have a public way in. Then, by this replacement rule, rewrite each site found by `grep -rlE "\.edit_pack\(|\.close_pack\(|EditPack|ClosePack|\"edit_pack\"|\"close_pack\"" crates fixtures` (at 756aa9ec: 79 files, 269 sites):
  - a test of library-pack editing → `test_support_packs::pack_tab_of(pack)` (unit) or `open_pack_tab_here` on a fresh session (integration, mcp `start_pack_tab`, agent harness);
  - a test of deck-only pack editing → `test_support_packs::deck_pack_tab_of(owner, pack)` (unit) or `workspace_support::{import_hifi, open_pack}` (integration; mcp `tests/common/workspace_harness.rs`);
  - a test of the workshop itself (entering, leaving, `close_pack` refusals, the `Here { entered: true }` route, host `Deck`) → deleted, named in the review notes;
  - `ClosePack` used to leave pack mode before a deck assertion → the assertion moves to the owner (or the test ends on the pack tab).
  The files, by crate (count of sites): session unit — `pack_mode_save_tests.rs` (33), `pack_mode_tests.rs` (25), `pack_tab_lease_tests.rs` (18), `pack_tab_tests.rs` (15), `layout_examples_tests.rs` (14), `widget_examples_tests.rs` (7), `widgets_edit_tests.rs` (7), `pack_tab_read_only_tests.rs` (6), `pack_mode_preview_tests.rs` (5), `resume_pack_tests.rs`, `pack_tab_route_tests.rs`, `workspace_packs_tests.rs`, `test_support_packs.rs` (4 each), `add_grid_tests.rs`, `deck_adopt_tests.rs`, `deck_packs_tests.rs`, `deck_widgets_tests.rs`, `help_tests.rs`, `lib_tests.rs`, `widget_preview_tests.rs` (3 each), `agent/context_tests.rs`, `agent/turn_undo_tests.rs`, `deck_packs_updates_tests.rs`, `pack_tab_open_tests.rs`, `resume_tests.rs`, `theme_picker_tests.rs`, `widget_params_tests.rs`, `workspace_packs_close_tests.rs` (2 each), `deck_offer_tests.rs`, `deck_theme_edit_tests.rs`, `document_kind_tests.rs`, `history_tests.rs`, `import/queue_tests.rs`, `request_tests.rs`, `resume_pack_legacy_tests.rs`, `resume_pack_refusal_tests.rs`, `test_support.rs`, `theme_composed_tests.rs`, `theme_tab_ops_tests.rs`, `widget_examples_list_tests.rs`, `widget_preview_items_tests.rs`, `widgets_deck_lease_tests.rs`, `workspace_packs_race_tests.rs`, `workspace_tests.rs` (1 each); session integration — `tests/session.rs` (13), `tests/workspace.rs` (3), `tests/agent.rs`, `tests/errors.rs`, `tests/history.rs`, `tests/widgets.rs` (2 each), `tests/merge.rs` (1), regressions `issue_0524_…`, `issue_0902_deck_only_workshop` (name kept), `issue_1068_legacy_essentiel_copy`, `issue_1091_…` (2 each), `issue_1068_layout_example_emptied_by_element_op`, `issue_1093_…` (1 each); mcp — `tests/edit_pack_tabs.rs` (5), `tests/widgets.rs` (5), `tests/edit.rs` (4), `tests/app_tools.rs` (3), `tests/import.rs`, `tests/themes.rs`, `src/tools_tests.rs` (2 each), `tests/packs.rs`, `tests/server.rs`, `src/tool_error_tests.rs` (1 each); agent `tests/stack.rs` (2); local-agent `families_tests.rs` (2). Production files (`request.rs`, `request_class.rs`, `pack_tab.rs`, mcp `handler.rs`, `edit_tools.rs`, `tools.rs`, local-agent `families.rs`, `ui/src/api.ts`, `fixtures/agent/e2e.json`) are 7b's. Gate, then commit `test(session,mcp,agent): pack tests open pack tabs instead of the workshop (t-1200, t-1229)`; every test of still-valid behavior keeps its name.

**Files (7b):**
- Session: `request.rs` (variants, dispatch, `handle` preamble), `request_class.rs` (+ tests), `pack_mode.rs` (`edit_pack`, `close_pack`; `enter_pack` loses `host`), `pack_tab.rs` (`enter_here`, `open_pack_here` deleted; `open_pack_tab_here` added; `is_pack_tab` → `matches!(self.mode, Mode::Pack { .. })`; `route_pack`'s `Here { entered: true }` gone), `pack_mode_lease.rs` (`settle_lease`'s Deck branch, `deck_label`'s host test, `copy_lease`'s holder), `views.rs` (`PackHost`, `PackMode.host` deleted), `lib.rs` (`unsaved`'s workshop branch; re-exports), `agent/context.rs:16` (`PackHost` import) and `:134-135` (the agent context line becomes `pack(id={pack})`; the rstest `context_tests.rs:202` `context_of_a_pack_says_its_host` becomes `context_of_a_pack_names_it`), `pack_mode_save.rs:280` (`host: edit.host`), `resume_pack.rs` / `resume_workshop.rs` (`edit.host` assignment gone: states are now written without `host`), `workspace_packs.rs` (`PackOpened::Here { entered }` → `Here`), `agent/tabs.rs` (decision 14), `test_support_packs.rs` (`live_editor` / `park` without host; `pack_tab_of(pack) -> Session` via `open_pack_tab_here`), i18n: the keys of deleted error variants removed from all 20 locales (each named in the review notes from `grep` of the variants deleted from `error.rs`), and `session-in-pack-tab` rewritten to name the pack's own tab instead of the workshop (fr: `session-in-pack-tab = Ce pack s'édite dans son propre onglet : utilisez cet onglet.`, 20 locales).
- Consumers (production; their tests moved in 7a): mcp `handler.rs:61-62`, `edit_tools.rs` (`close_pack` deleted, `edit_pack`'s `PackOpen::Here` branch and `default_pack`'s host test), `tools.rs:278-279` (descriptions: a deck-only pack opens in its own tab linked to the deck), `tests/server.rs` (tool list loses `close_pack`), `tests/app_tools.rs:271,415,524-537` (`edit_pack {}` on the single-session harness now answers `SingleDocument`: asserted), `tests/common/mod.rs` (`start_pack_tab`, added in 7a); agent `tests/common/mod.rs:313-324` (`open_pack` mirrors the `Mutex<Session>` rule, decision 14); local-agent `families.rs:66`; `crates/oku-slides-agent/src/instructions/agent.md:3` + `instructions_tests.rs:43`; `crates/oku-slides-cli/src/serve.rs:174` comment.
- UI: `ui/src/api.ts` (`edit_pack` / `close_pack` cmds and `closePack` deleted, rule t-936); `ui/src/packTypes.ts` (`host` and the `PackHost` type deleted) and its five readers: `packGestures.ts:31,85` (`showFinish` = `!!snap.editing && canClose`; `finishPack` = `onClose`; the "Entered here" branch `:73-79` deleted, since `widgetGesture` never answers `open` in pack mode; the context fields left unused — `snap`, `autosave`, `widgets`, `confirm`, and `run`, `snapRef`, `api`, `setCurrent`, `setSelected` if unused after that — and the `saveForClose` import go; `finishPack`'s doc rewritten), `close.ts:40,98` (a pack tab is `snap.editing != null`; the workshop branch of `saveForClose`, `closePack` included, gone; `SaveForCloseOptions.confirm` gone; `saveForClose` stays exported, tested directly; the `:29-35` doc rewritten), `closeFlow.ts:135` (`isPackTab` = library pack tabs: `snap ? !!snap.editing && !snap.editing.deck_only : tab.document?.kind === "pack"`); `App.tsx:261-262` (the dropped context arguments; App.tsx shrinks); `usePackWidgetEditor.ts` (`finish` deleted; `settle` checked: kept only if still called, named in the review notes); `usePackWidgetEditor.render.test.tsx:375-385` (the `.finish(` regression moved onto the close guard, `useCloseGuard` settling `widgets.files`); `close.test.ts` (header comment and the workshop tests `:37`, `:61`, `:224`, `:241`); `packGestures.test.ts` (workshop tests `:58`, `:92-136` deleted; `:86` `finishPack of a pack tab closes the tab` kept and adapted to `onClose`); `ui/src/test/closeFlowFake.ts` (`closePack`); `fixtures/agent/e2e.json:14` unchanged (`edit_pack {}` still opens a tab). UI tests and `ui/src/test/**` are excluded from `tsc` (`ui/tsconfig.json`): only grep and the runtime tests catch them.

**Interfaces — produces:**
```rust
// pack_tab.rs (public)
/// Turns this fresh session into the tab of library pack `pack`.
///
/// # Errors
/// `PackTabNotFresh`, `DeckOnly` for a deck-only pack, `PackOpenInTab`, the `init_pack_tab` errors.
///
/// # Blocking
/// Reads the library, takes `DRAFTS` for the lease, renders the preview.
pub fn open_pack_tab_here(&mut self, pack: Uuid, widget: Option<&str>) -> Result<()>;
```

- [ ] **Step 1: Write the failing tests** (the first two in 7a, before Step 0's migration): `open_pack_tab_here_opens_a_library_pack_tab` and `open_pack_tab_here_of_a_deck_only_pack_is_refused` (`pack_tab_tests.rs`); `single_document_host_opens_its_own_pack_widget_here` and `single_document_host_refuses_another_pack` (`agent/tabs_tests.rs`: `SingleDocument`); `pack_states_are_written_without_host`; `request_class` tables lose `EditPack` / `ClosePack`; mcp `tests/edit_pack_tabs.rs` gains `start_pack_tab_edits_widgets_of_its_pack`; UI: `packGestures.test.ts` `finishPack of a pack tab calls onClose` (the existing `:86` test rewritten: it fails while `finishPack` still saves first), `close.test.ts` `saveForClose of a pack tab answers its dirty flag`, `closeFlow.test.ts` `closeTabFlow of a deck-pack tab offers no opener` (through the public flow; `isPackTab` stays private).
- [ ] **Step 2: Run; they fail.**
- [ ] **Step 3: Delete** per decisions 14-15, then migrate every production consumer listed. Two checks: `grep -rnE "\.edit_pack\(|\.close_pack\(|EditPack|ClosePack|\"edit_pack\"|\"close_pack\"|PackHost|edit\.host|editing\.host|\[\"host\"\]|editing_layout_pack|open_pack_here|enter_here|closePack|\.finish\(" crates ui/src ui/e2e fixtures` ends with only the MCP tool name `edit_pack` in tool definitions and agent prompts and `fixtures/agent/e2e.json:14`; `grep -rn "host" crates/oku-slides-session/src/{views,pack_mode*,pack_tab*,resume_pack*,resume_workshop*}.rs ui/src/{packTypes,packGestures,close,closeFlow}.ts` has no pack-host hit left (each remaining hit named in the review notes). Tests of removed behavior (entering or leaving a workshop, `close_pack` refusals, `edit_pack_request_of_a_library_pack_in_a_deck_tab_still_enters_the_workshop`, `end_turn_after_edit_pack_closes_stashed_group_too`, the `deck_workshop_*` lease tests, the UI workshop branches) were deleted in 7a or here, listed in the review notes.
- [ ] **Step 4: Rules and docs, same commit.** Session `AGENTS.md` line 15: "write pack and theme states at EDITOR_STATE_VERSION (5)" → "… (5), pack states without host"; line 21: "(the session version only on close_pack, …)" → "(the session version only when its pack tab saves, …)"; every other rule matched by `grep -n "workshop\|EditPack\|ClosePack\|PackHost\|close_pack" AGENTS.md crates/*/AGENTS.md crates/*/src/*/AGENTS.md ui/AGENTS.md docs/AGENTS.md` rewritten in place to the deck-pack tab (each named in the review notes). Session, mcp and agent `ARCHITECTURE.md`: the workshop sentences replaced (no net line). README line 201 if it names the workshop.
- [ ] **Step 5: Run** `cargo nextest run --workspace --exclude oku-slides-app` then `-p oku-slides-app`; UI gate; all pass.
- [ ] **Step 6: Gate**, then commit (7b) `refactor(session,ui)!: the in-deck workshop, EditPack and ClosePack are removed (t-1200, t-1229)`, body naming the breaking changes: `Request::{EditPack, ClosePack}`, `PackHost`, `EditingView.host`, the MCP `close_pack` tool, `edit_pack` on a single-document host.

### Task 8: The interface and e2e on deck-pack tabs (t-1230)

**Files:**
- Rust: `views.rs` / `lib.rs` (`TabInfo.editing_pack` deleted) and its consumers: `pack_tab.rs:286`, `theme_tab.rs:219`, `workspace_same_deck.rs:67`, `views_tests.rs`, `theme_tab_tests.rs:135`, `history_tests.rs:105`, `workspace_resume_tests.rs:38,64-68,164` (the saved entry no longer has the key; a legacy entry with it still reads), `tests/session.rs:494`, `tests/workspace.rs:133`; with `ui/src/workspaceApi.ts` (`TabInfo` mirror) in the same commit.
- UI: `workspaceApi.ts` (new `linkedTabs(windows: WindowView[], deckTab: string): TabInfo[]` — the `deck_pack` tabs whose `document.deck_tab` is `deckTab`, across every window — tested in `workspaceApi.test.ts`); `workspaceApi.ts` also gains `closeStays(view: WorkspaceView, win: string, tab: string, inTauri: boolean): boolean` — the one rule for "closing this tab would empty the workspace": not in Tauri, and the tab plus its `linkedTabs` are every tab of every window — tested beside `linkedTabs`; `TabBar.tsx` (badge `ui-tab-pack` for `document.kind` `pack` or `deck_pack`, `:218-219`; `lastStays` becomes per tab through `closeStays`, used by the × (`:227`) and the menu's "Fermer" (`:267`)); `Workspace.tsx` (`closeOthers`, `:167-171`: leaves out the kept tab's owner; `flowDeps` gains `freshView(): Promise<WorkspaceView>` from `:139-142`, and `findTab` / `freshTabs` derive from that one read instead of a second request; App's `onClose`, `:302`, uses `closeStays` instead of `inTauri || win.tabs.length > 1`; when a broadcast view changes the `TabInfo` of the window's active tab, the window re-reads its snapshot — the existing view handler is checked first and the behavior tested either way); `closeFlow.ts`:
  - `closeTabFlow` and `closeWindowFlow` (decision 11): from one `freshView()` read, before the first `tab.dirty` test of a deck tab, for each `linkedTabs(view.windows, id)` **not already in `deps.closing`** (its own flow runs) and not in this flow's `discarded`: add it to `deps.closing` and to this flow's `added` list, run its close guard (`runGuard`, this window's tabs only; another window's tab relies on its own autosave — documented in the flow's doc), then `tabApi(linked).savePack()` when its snapshot is dirty or `editing.unsent`, then **re-read `tabApi(linked).snapshot()`** (the save's answer predates the forward) before testing `editing.unsent`; then re-read the owner and decide as today; a linked tab still `unsent` asks `ui-confirm-quit-pack` for it — no: keep → the whole close stops; yes → the owner closes with `discard: true` after its own verdict (the server discards its links); the UI never adds linked ids to `discarded` or calls `close(linked, true)`; the flow's `finally` removes from `deps.closing` exactly the ids in `added` (in the window flow they also join `visited`);
  - `stalledClose` (`:118-130`): when the owner has linked tabs, its question names each one whose `TabInfo.dirty` is true (unsent included) — read from the view, no `savePack`, since a forward would wait on a possibly stuck owner;
  - `unreadVerdict` (`:146-160`): `deck_pack` takes the pack path (`savePack`, `ui-confirm-quit-pack`).
- `close.ts` (`packTabVerdict`, `:67`: the early return is `!snap.dirty && !editing.unsent`; after `savePack`, re-read the snapshot as `themeTabVerdict` does (`:86`) before testing `unsent`; an `unsent` tab asks `ui-confirm-quit-pack`).
- `App.tsx`: the "Terminer" button (`:408-410`) unchanged in shape; no growth; the "Entered here" branch lives in `packGestures.ts:73-79` and goes with Task 7's `Here` change if not already gone.
- Tests: `TabBar.render.test.tsx` (`TabBar badge follows editing_pack, not the document kind` (`:92`) replaced by `TabBar badge follows the document kind, pack or deck_pack`; `TabBar of a deck tab whose linked tab is the only other tab keeps its close disabled`; reuse the `badges` helper), `workspaceApi.test.ts` (`linkedTabs of a deck tab lists its deck_pack tabs in every window`), `closeFlow.test.ts` (`closeTabFlow of a clean owner saves its unsent linked tab first` — the save delivers, no confirm in the log; `closeTabFlow of an owner whose linked tab stays unsent asks ui-confirm-quit-pack`; `closeWindowFlow skips a linked tab the user discarded`; `closeTabFlow of an owner marks its linked tabs closing`; `closeTabFlow of an owner that stops clears its linked tabs from closing`; `closeTabFlow of an owner leaves a linked tab already closing alone`), `workspaceApi.test.ts` (`closeStays of an owner whose linked tab is the only other tab is true in the browser`; `closeStays in Tauri is false`), `close.test.ts` (`packTabVerdict of an unsent clean pack tab asks ui-confirm-quit-pack`; `packTabVerdict re-reads the snapshot after savePack`), `Workspace.render.test.tsx` or the nearest existing test of `closeOthers` (`closeOthers from a deck-pack tab keeps its owner`; `Workspace re-reads the active tab snapshot when its TabInfo changes`); `ui/src/test/closeFlowFake.ts`: `freshView` derived from `state.tabs` plus an optional `state.otherWindows` (no static `VIEW` holding a linked tab, so existing deck-close tests keep their exact logs), `ws.close(owner)` drops its linked tabs, `savePack(linked)` sets the owner dirty (so the owner re-read is exercised), `tabOf`'s `editing_pack` (`:22`) removed; remaining `editing_pack` readers: `close.test.ts:125`, `workspaceApi.test.ts:9`, `TabBar.render.test.tsx:22,29`.
- e2e: `220-import.cjs:160-235`, `lib/atelier.cjs` (`openAtelier`, `:17`, expects a new tab of kind `pack` or `deck_pack`, never the same tab; the workshop check at `:30` deleted; `closePackTabs`, `:51-59`, doc at `:45`, includes `deck_pack` with `save_pack`).

- [ ] **Step 1: Write the failing tests** (render and unit, named above), then the e2e rewrite of `220-import.cjs`'s deck-only part:
  1. open the hifi deck-only layouts pack with `openAtelier` (a layouts pack has no stage button) → one more tab, kind `deck_pack`, right after hifi, active;
  2. rename a layout's example title in it; wait (`until`) until hifi's snapshot (HTTP, `/api` on hifi's tab) pins a version of the pack other than the one read before the edit — the autosave and the forward done;
  3. activate hifi: its pin is that new version; `GET` of the library lists no new pack;
  4. in hifi, click the stage then `Control+z` (as `220-import.cjs:156-157`) → the refusal toast, compared after stripping U+2068/U+2069 (as `:174-176`) with `session-undo-in-pack-tab`'s fr text filled with the pack's and the tab's names; the pin unchanged;
  5. in the pack tab, "Terminer" → the tab closes, hifi is active; `Control+z` in hifi now undoes the pack step (pin back);
  6. `Control+Shift+z` (redo) in hifi; reopen the pack tab with `openAtelier`; refocus through HTTP `open_pack` from hifi → the pack tab is active, the tab count unchanged;
  7. the spec's theme steps, unchanged: `insert_tab` with `TabPlace::After` (`workspace_layout.rs:135`) puts the theme tab at hifi+1 and pushes the deck-pack tab right, so the assertion at `220-import.cjs:236` stays as is;
  8. close hifi with its × → both tabs gone, hifi's file saved on disk (a dirty deck with a path saves silently, `close.ts:47`).
- [ ] **Step 2: Run** the UI tests; they fail. **Step 3: Implement.** **Step 4:** UI gate, Rust gate, `just e2e 4999` (detached); all pass.
- [ ] **Step 5: Rule, same commit.** `ui/AGENTS.md`: a rule naming pack tabs by `editing.host` or `editing_pack` is rewritten to `document.kind` / `editing` (replaced text; none → nothing, said in the review notes).
- [ ] **Step 6: Commit** `feat(session,ui)!: deck-only packs edit in their own tab, closed with their deck (t-1200, t-1230)`.

### Task 9: Help, ADR, spec note, full gate (t-1232)

**Files:** `docs/aide/fr.json`, `docs/aide/en.json` (the deck-only pack's workshop → its own tab beside the deck, saved into the deck, closed with it; undo of its changes in its tab); `docs/adr/0013-deck-pack-tabs.md` (English: the link's identity in the workspace layout, its state in the owner keyed by tab id, the re-seed rule from the owner's pin, and why not persisted in the session state; the outbox, due points, process-wide seq, in-flight and parked states, acknowledgement; forwards that make the caller wait; the touched-owner queue drained by hosts and the dev server's agent-turn limit; validation against the owner's pins; copies keep the namespace; owner refusals as a lease, the undo stop after the fact and its two merge-cut effects; `LastWindow` on an emptying close; old workshops reopened on their deck; `state.json` without `host`, with `unforwarded`, and the unreleased lot-4 build; the lock order and why no `Drop` locks the workspace; the layout generation that keeps stale link copies out; persistence of both sides of a forward; checks on clones before any write, the owner's cumulative byte budget, no closure files in a send; known limits: an owner absorbing an external change on save keeps its links and later sends are refused (an owner that opens another document fails closed by `deck_uid`); one step per editing session no longer holds when an owner edit lands between two sends; an undo in the tab past a copy is refused by the owner; a history point saved between two merged sends no longer renders the dropped intermediate version; an agent task that dies before `end_turn` parks forwards as it blocks undo; the dev server drains an agent turn's owners at the next request) and its row in `docs/adr/README.md`; the spec (`Lots` gains an implementation note for lot 5 recording decisions 1-15 where they amend the spec: `DeckPack { deck_tab, pack }` by uuid, `ReceiveDeckPack` as a session method, the undo stop, old workshops, blocking forwards).

- [ ] **Step 1:** write the docs; `just agents-lint`; grep the rules for any leftover workshop wording (none: Tasks 1-8 rewrote them).
- [ ] **Step 2:** `just ci` and `just e2e 4999` (detached).
- [ ] **Step 3: Commit** `docs(help): deck-only pack tabs in the help and ADR-0013 (t-1200, t-1232)`.
- [ ] **Step 4:** final whole-branch review, fixes, then the squash onto `main` (memory: commit hook practice; the user decides the push).

## Self-review (done while writing)

- Spec coverage: `TabDocument::DeckPack` linked to its owner (Tasks 3, 5); saves forwarded, acknowledged, ordered, undo included (Tasks 3-5, decisions 1, 6-8); closed with its deck — Close, CloseWindow, dormant tabs, quit, never an empty workspace (decision 11, Tasks 5 and 8); `ReceiveDeckPack` validated (Task 4); render cost (decision 7); the in-deck workshop and `EditPack` / `ClosePack` removed (Tasks 2, 6, 7); the spec's tests `deck_pack_tab_closes_with_owner_deck`, `deck_pack_save_reaches_owner_as_one_undo_step`, `open_pack_twice_focuses_existing_tab`, `issue_0902_deck_only_workshop` rewritten (Tasks 4, 5, 7); help and ADR (Task 9). t-1224 (Task 1).
- v2 report: critical 1 (links at every acquisition, `with_quiet`), 2 (`LinkParent::Reseeded`), 3 (Task 1 guard test), 4 (`SavedPack.legacy` + `edit.host` set on read; "without host" in Task 7), 5 (`freshView`, `linkedTabs`), 6 (flush before the verdict; `packTabVerdict`); majors 1 (decision 6), 2 (`OutboxState`, `Delivered`), 3 (`take_touched`), 4 (copy namespace, both maps, pins diff), 5 (deck-scoped current pin), 6 (drop guards), 7 (`sync_widgets`), 8 (links by tab id, `PendingLink`, unlink on close), 9 (`host: Tab`, `enter_pack`), 10 (`open_pack_here` arm), 11 (`Close` arm, `CloseWindow` `LastWindow`, no `would_close_all`), 12 (`SavedPack` in Task 5, `begin_turn` setup), 13 (Task 2 first, History keeps `stash`), 14 (`merge_file` only; History → `Busy`), 15 (no ADR cited before Task 9; lock order inline), 16 (consumer lists in Tasks 7-8), 17 (e2e files and steps); minors as listed in the report.
- v3 report: critical 1 (Task 1 step 4 `pack_use_mine`, `on(…)`); majors 1 (persist both sides), 2 (oldest base, `unforwarded: Option<SendBase>`, `check_pack_doc`), 3 (`mark_due` from `Parked`, `back_to_deck` pull, `repull`), 4 (clones before writing, the link's namespace from the deck), 5 (cumulative bytes), 6 (adopt the parent only, `adoptable`), 7 (`set_deck_links` returns refused tabs, `set_aside_unlinked`, `retarget` saves the layout), 8 (`PendingLink::cancel` outside the lock, `try_lock` net), 9 (Task 7a, the file list), 10-11 (UI `added` / re-read after `savePack`), the three test majors (hook ordering, `valid_send()`, `realign_pack` by version); minors as listed in the report; the false positive gets `receive_deck_pack_first_send_is_applied`. `held` is dropped from the send (security minor 4).
- Deliberate deviations from the spec: the link is identified by the pack uuid, not the namespace (lot 3's note); `ReceiveDeckPack` is a session method, not a `Request` (lot 3's note); `Mode::Pack` keeps a document beside the pack, renamed `preview`.
