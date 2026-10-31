# Batch F: pack tab follow-ups (t-1237, t-1207, t-1239, t-1238, t-1251, t-1255)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Parent task:** t-1329. **Sub-tasks:** Task 1 t-1334, Task 2 t-1336, Task 3 t-1338, Task 4 t-1339,
Task 5 t-1340, Task 6 t-1341, Task 7 t-1342.

**Goal:** Close the follow-ups the pack and theme tab lots (t-1152 epic) left open.

**Context:** Lots A to G of the 2026-10-31 backlog run in parallel. Another session writes t-1249 (pack pile
duplicate and reorder): stay out of `PackPile` and the pack-tab pile requests.

**User decisions** (2026-10-31, binding):
- **t-1207:** the opener of a pack tab that saved a copy is offered to switch to the copy: the original's pin is
  replaced by the copy's and the slides' widget references are renamed to the copy's namespace, in one undo step.
- **t-1251:** accept the cross-process race and document it as a rule; collect orphan empty drafts at resume.

**v2** follows a 15-agent plan review: `.superpowers/sdd/lot-f/plan-review.md`.

## Global constraints

- Every root, `crates/` and `crates/oku-slides-session/` `AGENTS.md` invariant applies; `ui/AGENTS.md` for Task 6.
- TDD: a failing test first; a bug fix brings a regression test in `tests/regressions/issue_NNNN_topic.rs`.
- New Fluent keys go to all 20 locales (`crates/oku-slides-i18n/src/locales/*/`), in the same place; English
  first, each locale translated, `{ $variables }` kept.
- `ARCHITECTURE.md` (session: at its 50-line cap, trim to add) and the nearest `AGENTS.md` change in the same commit.
- Paths below are under `crates/oku-slides-session/src/` unless said otherwise.

## Task 1: t-1237 — a save after an agent turn in a pack tab keeps the turn's version

Cause: `begin_turn` opens the undo group on `doc.sources` only; `end_turn` already closes it on the preview.
A save during the turn and one after it join one preview step (`pack-adopt` merge key), and
`adopt_session_version` drops the turn's version, deciding "merged" by an unchanged step count.

- [ ] Tests in `pack_mode_save_tests.rs` (RED first): `save_pack_after_a_turn_in_a_pack_tab_keeps_the_turn_version`
  (library and preview hold both); `save_pack_twice_in_one_turn_keeps_only_the_last_version_in_the_preview`;
  `save_pack_in_a_turn_after_a_save_before_it_keeps_the_earlier_version`. The existing
  `saving_twice_in_one_session_…` stays the negative case (two saves, no turn, one step).
- [ ] Regression `tests/regressions/issue_1237_save_after_turn_keeps_turn_version.rs` (library versions), declared in `main.rs`.
- [ ] Fix: `begin_turn` opens the group on `Mode::Pack { preview } | Mode::History { stash }`, as `end_turn`
  closes it; `PackScratch::adopt_step` records the preview step that pinned the session version, and a save drops
  the replaced version only when it lands in that same step (else restores it); `realign_pack` clears it;
  `replay` (undo/redo) ends the set-aside deck's group as the replica ends the document's.
- In-turn saves now carry the turn's label on the preview: intended (the turn is one undo step).

## Task 2: t-1238 — the flaky badge test

- [ ] Reproduce `request_tests.rs::snapshot_request_without_an_in_process_write_keeps_the_badge_unlit`: run it
  in a loop under load (`cargo nextest run -p oku-slides-session -E 'test(=…)' --retries 0`, 50 runs beside a
  parallel full run), and read which `copy_over` entry vanishes. Lead (t-1239): `acme_pack` drops a pack-tab
  session on the same data dir just before `copy_over`.
- [ ] Fix the cause (no retry, sleep or looser assertion); if no reproduction, fix the identified race and
  say so on the ticket. 50 green runs under the same load afterwards.

**Found (v3):** not a load race. `copy_over` never creates its top target (`scratch/widgets`), and the library
root holds `GENERATION_FILE` beside the `<uuid>/` folders: APFS lists entries by name hash, the uuids are random,
so the file comes first in some runs (NotFound). Fix in the helper, with a deterministic test
(`copy_over_a_top_level_file_into_a_missing_folder_copies_it`); the `acme_pack` lead was wrong.

## Task 3: t-1239 — lot 5 final-review follow-ups (Rust)

Each item gets a failing-first test asserting on kinds or state, never on English text.
- [ ] Split `workspace_tests.rs` (946 lines): `workspace_tabs_tests.rs`, `workspace_windows_tests.rs`,
  `workspace_session_tests.rs` (and locale/scheme ones if needed), helpers in `workspace_fixtures.rs`; same test count.
- [ ] `apply_send` (`deck_pack_receive.rs`) keeps a closure or pin failure's own kind instead of `DeckPackMismatch`.
- [ ] A deck-pack tab set aside (`set_aside`, from `acquire`, `workspace_session.rs`, `workspace_same_deck.rs`)
  counts in `changed` / reaches `tab_closed`.
- [ ] `after_change(owner)` runs once on a close that flushed (`crates/oku-slides-cli` serve.rs, `src-tauri` commands.rs).
- [ ] A dormant owner is not woken just to unlink a closing deck-pack tab (it stays dormant; reread-before-destructive kept).
- [ ] A test asserts `PendingLink::cancel` after a finish error.
- An item whose fix needs a design choice beyond the ticket is split into a new ticket named here, with the reason.
- **v3:** the set-aside tab reaches `tab_closed` through `Workspace::closed_by` around session requests in the three
  host paths (serve `/api`, Tauri `request`, the agent's `Sessions::with`), and `acquire` counts it in `changed`.
  `after_change` runs once: `WorkspaceOutcome::DeckChanged` is deleted (a close's flush already puts the owner in
  `touched`, drained by every host). `PendingLink::cancel` gets a direct test: every finish error leaves the owner
  closed or the workspace settled, where its unlink cannot run.

## Task 4: t-1207 — opener offers for a copy and for a theme

- [ ] `workspace_opener.rs` takes `pack_opener` and `OpenerOffer` out of `workspace_packs.rs`; `OpenerOffer`
  gains a kind: `version` (today), `copy` (switch to the copy), `theme` (take the theme version). TS mirror in
  `ui/src/workspaceApi.ts`, `ui/src/closeFlow.ts` `askOffer`, `ui/src/test/closeFlowFake.ts`.
- [ ] Copy: the pack tab records the original id it copied (`wrote_in_pack_tab` carries `copied_from`); the
  offer holds when the opener pins the original, the copy is the library's current, its namespace is free in the
  other kind and its requirements are pinned. Accepting (new `Request::UsePackCopy` + `ui/src/api.ts`) rechecks
  all that and the deck uid, then replaces the pin and renames the slides' references (`rename_namespace`) in one
  undo step. Deck-only copies get no offer (a deck-pack tab forwards them itself).
- [ ] Theme: `wrote_in_theme_tab` (`theme_tab_save.rs`) gives the version a theme tab wrote; the offer holds when
  the opener pins that theme at an older version; accepting (`Request::UseThemeVersion`) rechecks that the version
  is still current and pins exactly it (`update_deck_theme` path).
- [ ] Fluent keys `ui-confirm-update-opener-copy`, `ui-confirm-update-opener-theme` (20 locales).
- [ ] Tests: each offer positive, plus a taken namespace, an opener without the original, an expired version.
- [ ] The ticket comment (a theme-only save writes a pack version) is obsolete since lot 4: comment so.

## Task 5: t-1251 — cross-process draft races (accepted)

- [ ] Rule in `crates/oku-slides-session/AGENTS.md`: `draft_lock` and leases are per process; the app is
  single-instance, so only the CLI or `serve` beside it races on one data dir; accepted.
- [ ] Orphan empty drafts (a crash between `create_empty_*` and the tab open): at workspace resume, delete a
  library pack or theme that holds only its creation version, was created empty, is older than one day and is
  shown by no tab. Test with a forged old draft and a fresh one.
- **v3:** "created empty" is a `created-empty` mark `create_empty_*` writes in the version folder, not a content
  guess: an empty pack made any other way (issue_1093's fixture, an import, a CLI write) is the user's and stays.
  Opening the tab freezes the version (adopted), so a marked unfrozen draft is one whose tab never opened.

## Task 6: t-1255 and t-1239 — close flow leftovers (`ui/src/closeFlow.ts`)

Rules of `ui/AGENTS.md` kept (bounded guards, refuse on a first stall, ask on a second within 30 s). Tests in
`closeFlow.test.ts` / `closeWindowFlow.test.ts`, failing first.
- [ ] `closeWindowFlow` restores the focus (`stay()`, no second report) when the host refuses `closeWindow`.
- [ ] `askStuck` names each stuck tab: key `ui-confirm-close-tab-writes-pending` with `{ $tab }` (20 locales),
  each title through `isolate()`; a tab gone from the read is skipped.
- [ ] A stall counts once when a tab close and a window close run at once: the window round skips ids in
  `deps.closing` (their own flow handles them).
- [ ] A stalling linked-tab guard goes through the second-stall logic for that linked tab.
- [ ] A rejected linked-tab snapshot falls back to `unreadVerdict` (looked up across windows).
- [ ] `isolate()` strips bidi controls inside the label before wrapping it (review: a title with PDI+RLO).
- Successor ticket for what stays: a pack deleted from another process while a pack tab edits it (cross-process,
  as t-1251); a widget opened on a pack tab mounted in another Tauri window; the 18 machine translations to review.

## Task 7: final review and gates

- [ ] Multi-angle review of the branch diff; fix what it finds.
- [ ] `just ci`, `just e2e`; `git fetch` and merge `main` right before one squash commit citing every ticket.

**Review (v4), 15 reviewers.** Fixed before merge:
- The copy switch refused nothing when another pin required the original's namespace, or the
  original was `base`: `copy_switch` now runs core's `remove_pack_refusal` on the renamed deck
  (no offer, `PackOfferExpired` at take). Tests: taken namespace, required original, copy not current.
- A window close re-guarded a linked tab the user had given up in its owner's flush, and refused:
  those ids go to `givenUp`, out of the last round.
- The orphan collection rechecks its predicate under `DRAFTS` (`discard_if`), and `Library::is_orphan`
  counts version folders on disk (failing closed) and requires the version to be current.
- Wire-parse tests for `use_pack_copy` and `use_theme_version`, both in the classification tables;
  `shown_drafts` unit test; `issue_1239_*` regression for the double after-change (the dormant-owner
  fix keeps its inline test: it reads crate-internal slot state).
- ADR-0013 gets a superseding line for `DeckChanged`.
Accepted or deferred (t-1378): serve's agent `Sessions` impl does not stop set-aside tabs (dev server
only, documented on the impl); `use_theme_version` pins the theme's current version, rechecked in the
same call (no exact-version pin); the switch does not prove `copy` descends from `from`.
