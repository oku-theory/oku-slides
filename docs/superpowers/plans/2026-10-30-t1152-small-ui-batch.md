# t-1152 small UI batch (t-1254, t-1266, t-1252, t-1250, t-1265) Implementation Plan

**Parent task:** t-1152. **Sub-tasks:** the five tickets below, one per task. t-1253 closed by
user decision (2026-10-30): keep lot 6's selection move on agent writes.

**Goal:** Close five small UI follow-ups of the pack and theme tabs.

## Task 1: Workspace close confirmation through ConfirmDialog (t-1254)

- `close.ts`: `Confirm` becomes `(q: Question) => Promise<boolean>`, `Question` = message, confirm
  label, `danger`, optional cancel label. Every `confirm(...)` in `close.ts` and `closeFlow.ts` is
  awaited (`askStuck` becomes async).
- Discard questions (quit pack/theme, read-only, writes pending, linked) confirm with
  `ui-confirm-discard-ok` ("Close without saving", danger); the opener offer with
  `ui-confirm-update-ok` ("Update") and `ui-confirm-update-later` ("Not now") as cancel.
- New `useConfirm` hook (`ui/src/useConfirm.tsx`): a FIFO of questions, the first shown in a
  `ConfirmDialog`; questions left on unmount are answered no. `PackPanels` and `Workspace` use it
  (one implementation). Workspace holds tab shortcuts and App keys while it asks.
- `merge.ts`'s browser-only `window.confirm` fallbacks stay (they stand for Tauri's three-button
  native dialogs); the ui `AGENTS.md` rule is rewritten to say so.
- Tests: `close.test.ts`, `closeFlow*.test.ts` and the fake go async; a `useConfirm` render test
  (queue order, unmount answers no); e2e 040, 160, 200 answer or assert the in-page dialog.

## Task 2: Tab bar: pack and theme tab names drop the unsaved state (t-1266)

- `ui-tab-kind-pack` / `ui-tab-kind-theme` take `$dirty` (`yes`/`no`) and append
  `, { ui-tab-unsaved }` when dirty, in all 20 locales. `TabBar.tabName` passes it.
- Render test: a dirty pack tab's accessible name contains the unsaved text, in English and French.

## Task 3: Packs panel: Ouvrir opens the library's current version (t-1252)

- `deck_packs_panel.rs`: a card with a drift labels its open button `packs-pack-open-mine`
  ("Open my version"); without drift it stays `packs-pack-open`. 20 locales.
- Test in `deck_packs_panel_tests.rs` on the key, English and French.

## Task 4: Context menu on pack pile items (t-1250)

- Extract TabBar's measured, portalled context menu into `ContextMenu.tsx` (Escape and outside
  click close it, placed by `menuAbove`); TabBar uses it.
- `PackPile`: right-click on an item (or the shared row) opens Rename (not for the shared file)
  and Delete; `SlidePile` gains `onThumbContextMenu`.
- Render tests: menu entries, Rename starts naming, Delete asks the confirmation.

## Task 5: Pack tabs: mouse-reachable session undo (t-1265)

- `WidgetTools` drives the item's editor history only while the code is shown for a selected
  item; otherwise it shows the session's undo and redo (`snap.can_undo`, labels) as deck tabs do.
- Render tests for both routes.

## Task 6: Small UI batch review and gates

- Review of the whole diff, `just ci`, `just e2e`, one squash commit citing every ticket.
