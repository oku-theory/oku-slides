# Hub: object state leaves the agent thread (inspector, data pane, popovers, traces)

Task t-1502. Plan: `docs/superpowers/plans/2026-11-02-hub-inspector-traces.md`.

## Problem

The hub's dock mixes events and object state. Today it holds, besides the agent's questions:

- `host-data` (selected chart or table), injected by `App.tsx` as `hostSurfaces`;
- the agent's live `show_params` forms;
- the requested boxes `host-packs`, `host-voice`, `host-theme-picker`.

They push the conversation away, never age like it, and lack room. The user's main complaint
is the clutter of the thread. (`host-widget` lives under a pack tab's preview and `host-theme`
in a theme tab's column: neither is in the thread, both stay as they are.)

## Rule

**What waits for an answer lives in the thread; what describes an object lives next to it.**

## Decisions (made on 2026-11-02, user; revised after the plan review the same day)

1. **The thread keeps events only**: prompts, replies, receipts, and agent questions (live at
   the dock, near the mic). No settings surface and no requested box is shown in the thread of a
   deck tab. Pack, theme and help tabs have no inspector: there, the agent's `show_params` stays
   in the thread as today.
2. **The agent never changes what the user looks at.** It never selects, never opens the
   inspector, the data pane or a popover. The `focus` tool shows the slide only: it no longer
   selects the element. Anything the agent opens shows as a chip in its turn.
3. **Traces list the agent's edits only** (user decision): one chip per element the agent's
   tools changed during the turn, "object: summary". The object is the element's first
   non-empty text (title, label, heading), shortened to 40 characters; without text, its kind's
   name (user decision). Edits the user makes during a turn, including clicks on the agent's
   own forms and variants, are not traced.
   - A click shows the slide, selects the element and opens the inspector, or the data pane
     when the agent edited its chart or table data.
   - An element gone when the receipt is built gives a plain text line; one removed later
     answers its click with a notice.
   - At most 6 chips per receipt, then a caption "and N others".
   - Receipts of past turns keep their traces clickable. A restored thread (after a restart)
     shows each trace as text: restoring drops actions.
   - Slide-level edits (new slide, removed slide) give no trace: a later task.
4. **Inspector** (deck tab): a collapsible column beside the stage showing the selection's state.
   - one element: its settings, a new host surface `host-element` built from the element's
     schema and current values (list params included), one undo step per change; plus a "Data"
     button for a chart or a table;
   - nothing selected: the deck: its theme's name and a "Packs" button (opens the popover);
   - an agent offer opened from its chip comes first, framed "Offered by the agent".
5. **Data pane** (deck tab): a wide row under the stage for `host-data`, opened from the
   inspector's "Data" button or a data trace; collapsible.
6. **The agent's `show_params`** (deck tab) gets `Place::Inspector`: the thread shows a chip
   "Settings offered: title" in its turn; a click opens the inspector on that surface. Selecting
   another element closes the offer. A frozen offer goes back to the thread, in its turn.
7. **Requested boxes become popovers** (`Place::Popover`): `host-voice` next to the dictation
   gear, `host-packs` next to the Packs button, `host-theme-picker` next to a new Themes button
   of the tools tray (`AgentCommand::ShowThemes`, carrying the shown slide).
   - Opened by the user (button, spoken shortcut), a box carries no turn and opens.
   - Opened by an agent tool, it always carries a turn (user decision): the open turn, or the
     last one for a late call. It shows as a chip in that turn and opens on click.
   - A refresh of the box keeps its turn. A popover closes on Escape, an outside click, or a
     second click on its button; closing dismisses it on the hub.
8. **Opening**: toolbar button, ⌘⇧E (inspector, deck tabs only; ⌘⇧I is taken by DevTools on
   Windows and Linux), the inspector's "Data" button, a trace. Double-click keeps editing text.
   Once open, the inspector and the data pane stay open and follow the selection until closed.
9. **Open state is saved** in `UiState` (`inspector`, `data_pane`, booleans, closed by
   default), ordered by page and seq like the rest.
10. **Narrow window** (below 1100 px): the inspector overlays the stage instead of a column.
11. **Detached agent window**: a trace clicked there acts in the main window, over the agent
    link (`reveal`); a chip clicked there opens its surface in the agent window itself, under
    the chip; the dictation popover opens in the window that shows the gear.

## Contract

- `Place` gains `Inspector` and `Popover` (Rust `agent/forms.rs`, TS `api.ts`), same commit.
- `routeSurfaces` returns `popovers: Surface[]`, `offers: { turn, surface }[]` (live popover or
  inspector surfaces with a turn) and `dock` (live questions only). Every other surface keeps a
  visible place: frozen ones in their turn, turnless ones in `loose`. `isHostSettings`,
  `isRequestedBox` and the `hostSurfaces` dock merge are deleted, no shim.
- New requests `element_panel {slide, element}` (read), `element_param {slide, element, key,
  value}` and `element_list {slide, element, key, change}` (writes: one `SetParam` or `EditList`,
  undo label `agent-undo-param`, validated by core).
- **Traces are built by the session from the agent's own writes**: MCP tools that change the deck
  go through `Session::handle_by_agent`, which records each applied op's element in
  `TurnTouches`, keyed by the open turn, only once the transaction succeeds. `Session::traces(turn)`
  resolves them (`Trace`); `report_surface` and the fallback receipt render them as buttons with
  the action `trace {slide, element, pane}`. The fallback gets them pushed by the tab's
  `end_turn`, stored per turn in the hub. The UI handles `trace` from receipts only; it never
  reaches the agent.

## Testing

- Rust: `element_panel` (values, unknown element, read-only, theme tab), `element_param` and
  `element_list` (undo restores, core refusal kind, gating); `TurnTouches` per op variant,
  failure, user edits not recorded; receipt traces (button vs text, cap, plain copy, restore as
  text, Fluent fr plurals); places of `show_params` and the three boxes; boxes keep their turn
  across refreshes and late calls.
- UI: `routeSurfaces` (one case per routing row); render tests of the inspector, data pane,
  popovers (Escape, outside click, anchor toggle), trace clicks and offers; the agent-surfaces
  fixture regenerated.
- e2e: a new spec: the fake agent edits a chart; nothing opens; a trace shows in the receipt;
  its click opens the data pane on that chart. Specs that read `.agent-dock` boxes follow.

## As built (deviations from the text above)

- A turn whose agent edited elements but wrote no narration and no report still gets a fallback
  receipt, holding its traces only.
- A removed element keeps its name in its trace (a text line, with the name read before the removal).
- An agent box call (`host-packs`, `host-voice`, `host-theme-picker`) is refused with `TurnOver` when
  its turn is over and with `NoBoxTurn` when the tab never had one: every agent box carries a turn.
- Closing the inspector keeps an opened offer; only selecting another element closes it.
- The narrow overlay (below 1100 px) covers rows 1 and 2 of the layout: the stage and the drawer.
- The `focus` tool has no `element` argument any more.
- The end-to-end spec `360-hub-traces.cjs` creates its own slide `s-ventes` and chart `e-ventes`; the fake
  agent's `modifie le graphique des ventes` edits that chart with a `set_data` op.

## Out of scope

Multiple selection; slide-level traces; a merge key for slider drags in `host-element`; live
highlighting of the agent's edits in an open inspector.
