# Agent thread fully in A2UI — design

Date: 2026-10-03 · Task: t-416

## Goal

Make the agent thread markedly more compact without losing readability,
through a massive shift to A2UI: the thread is no longer Markdown in post-its
but a sequence of A2UI surfaces built by the host. The agent ends
each turn with a compact receipt; its explanations remain accessible,
collapsed. The post-it style disappears from the whole application.

Starting observation (screenshot of 2026-10-03): the agent's intermediate messages
carry the same weight as the answer; each post-it costs
padding, margins, tilt, shadow and 12–18 % of width; the 15/21
font is heavy; the final answer is a report where a single concluding
line is enough.

Out of scope: the "Cancel the turn" action on a receipt (t-417).

## Decisions

- **Host-side projection (Rust).** The host builds each turn as an
  A2UI surface; the UI only renders surfaces. A single
  A2UI builder (Rust), like the templates panel (`panel.rs`).
- **Hybrid composition.** Typed tools (`report`, `ask_*`,
  `show_*`) for the frequent cases, plus a free-form `render` tool validated
  on the Rust side for unforeseen cases.
- **The agent's free text is demoted to narration**: one-line progress
  notes in the turn's timeline. It is never rendered
  as an answer, except in the fallback receipt.
- **Surfaces in their turn.** A frozen surface is shown in the turn that
  created it; a live surface (pending question, settings) lives in
  a dock above the microphone, then freezes into its turn.
- **Catalog v3**, a deliberate break (no v2 kept).
- **End of the post-it style everywhere**: thread, dock, sheets, drawer, modal,
  templates panel, dictation draft, `ChartPanel`. A single notion
  replaces it: the card.

## Turn model (session, agent)

A turn = a prompt, its progress (tools, plan, narration), its
outcome (done, cancelled, error, abnormal end) and the surfaces it
created.

- `oku-slides-session/src/agent/turn_view.rs` (pure, next to `forms.rs`):
  `TurnView { n, prompt, steps, narration, state, outcome }` and its
  A2UI builders: the `turn-N` surface (Prompt, Timeline, notes) and
  the receipt (see `report`). No I/O, no state.
- `oku-slides-agent/src/turns.rs` replaces `ThreadLog`: keeps the turns
  of a tab (at most 50, config key `agent.kept_turns`), applies the
  `AgentEvent`s, emits `UpdateComponents` when a turn opens, then
  `UpdateDataModel` for each step or chunk (`/steps`, `/narration`,
  `/state`). Replaying a tab sends back the surfaces of the kept turns.
- `SurfaceHub` (mcp): `begin_turn(n)` is called by `turn.rs` at the start of
  each turn; every thread surface created by a tool call that arrived during
  that turn carries `turn: Some(n)`, even if it is shown after the next turn
  has opened (t-449: the turn is read when the call arrives and passed to the hub).
- `Surface` gains `turn: Option<u32>` and `live: bool`. `Place` becomes
  `Sheet | Drawer | Modal | Thread` (`Postit` and `Suggestions` removed).
- End of turn (`turn_ended`): `turn-N` switches to "done", the timeline
  collapses into one line ("14 tools · 2 notes · 38 s"). **Without `report`
  during the turn**, the host creates a fallback receipt: `title` = first
  sentence of the last narration, `why` = the rest (Markdown), no
  actions. Without narration, no receipt.

Dependency order unchanged: `oku-slides-agent` already depends on `-mcp`
and `-session`.

## Catalog v3 (a2ui, UI)

`urn:oku-slides:a2ui:catalog:v3`. The Rust mirror (`ComponentKind`) and zod
(`catalog.tsx`) change in the same commit.

Kept (only the style changes): Column, Row, Text, Button, TextField,
Stepper, Slider, ColorField, Select, ChoiceChips, SlideThumb, DiffText.

Modified:
- `Card`: plain card, `tone?: neutral | accent | danger`.
- `Text`: `note` variant (one muted line, ellipsis).

New:

| Component | Properties | Role |
|---|---|---|
| `Prompt` | `text` | The user's prompt, right-aligned bubble. Host only. |
| `Timeline` | `steps` (binding → `{label, status, count?}[]`), `collapsed` (dynamic bool), `summary` (dynamic text) | Progress of a turn; collapsed into one expandable line. Host only. |
| `Receipt` | `title`, `facts[1-4]`, `slide?` (`{id, hash}`: a reference, never the SVG; t-450), `swatch?` (hex), `child?` | Concluding card of a turn. |
| `Disclosure` | `summary`, `child` | Content collapsed by default ("▸ why"). |
| `Badge` | `text`, `tone` | Counter or status. |
| `Markdown` | `text` | Rich text, rendered by `Markdown.tsx` (no raw HTML). |

`Prompt` and `Timeline` are reserved for the host: `agent_allowed(&ComponentKind)`
rejects them in `render`.

## Agent tools (MCP)

New:

- **`report`** — called last, at most once per turn (a second
  call replaces the first):
  `{ title, facts[1-4], slide?, swatch?, why?, actions?[0-4]{label, prompt?} }`.
  The host assembles Receipt (the slide is a `{id, hash}` reference in it:
  identifier and fingerprint of its page; the UI resolves its thumbnail
  through its SVG cache, the state at the time of the receipt if known, otherwise the current state
  marked "modified since"; t-450),
  Disclosure(`why` as Markdown) and the row of actions. Frozen surface
  (`live: false`); its actions stay clickable as long as it is the
  last turn, and send `prompt` (or `label`) as a new turn,
  like the suggestions today.
- **`render`** — `{ components[], data? }`, catalog v3 except `Prompt` and
  `Timeline`. Bounds: 40 components, depth 6. Serde validation +
  `agent_allowed`; **an error is returned to the agent as a tool
  result**, readable, so that it can fix it. Only allowed action:
  `prompt {text}` (new turn). Frozen surface.

Changed:

| Before | After |
|---|---|
| `suggest_next` | Removed; replaced by `report.actions`. |
| `show_params` (`Postit`) | `Thread`, `live`; a new `show_params` or closing it freezes it with its last values. |
| `ask_choice` without an anchor | `Thread`, `live` while pending; once answered, frozen showing only the answer; `cancel_asks` freezes it as "cancelled". |
| `ask_confirm`, `ask_rewrite`, `show_variants` | Unchanged (modal, sheet, drawer). |

Instructions (`agent.ftl`, all languages): end each turn with
`report`; free text is only shown as a one-line progress note,
write it as such; `render` only if no typed tool
fits. Mentions of `suggest_next` disappear.

## Interface: thread and dock (UI)

Panel: header · thread of turns (scrolls) · dock (fixed) · microphone.

- **A turn**: surface `turn-N` (Prompt, Timeline), then its frozen
  surfaces in creation order (answered questions, `render`, receipt
  last).
- **Dock**: the `live` surfaces, at most two by construction (an `ask_*`
  blocks the agent; `show_params` replaces the previous one). The question is
  closest to the microphone (a dictated answer is possible: the prompt names the question
  that receives the keys, `answer`), the
  settings above it. When a question is pending, the settings collapse into
  a header line (which reopens them). Height ≤ 45 % of the panel: beyond that,
  the dock stops at its ceiling and scrolls (`overflow: auto`). The settings
  have a "×" (→ `live: false`); a question cannot be closed. Empty: 0 px.
- **Live → frozen**: same `id`, same `messages`, `live` changes; the
  surface joins its turn. Short transition if the turn is visible,
  removed under `prefers-reduced-motion`.
- **Scrolling**: follows the bottom during a turn unless the user has
  scrolled up; a "↓ turn in progress" pill brings it back to the bottom.
- `routeSurfaces`: sheet, drawer, modal unchanged; `Thread` is split into
  `dock` and `byTurn: Map<turn, Surface[]>`.
- `AgentThread` becomes the list of turns plus `<AgentDock>` (new).
  `thread.ts`, `threadItems` and the current JSX of the entries are removed.
  The `AgentEvent`s Chunk, ToolCall and Plan no longer go to the UI; they
  are removed from the UI channel if they have no other consumer.

## Style

A single notion: **the card**, defined in `ui/src/a2ui/a2ui.css` and
exposed as a `.card` class for components outside A2UI (dictation
draft, `ChartPanel`).

- Tokens derived from the existing ones: `--a2ui-surface` (`--paper-100`),
  `--a2ui-border` (`--ink` at 12 %, 1 px rule), `--a2ui-accent-bg`
  (`--accent-soft`).
- Card: `--radius-sm`, padding 8 × 10 px, no tilt nor shadow, no
  margin (spacing comes from the parent `gap`, `--space-1`).
- Typography: body 400 14/20; notes, timeline, captions 400 12/16 in
  `--ink-muted`; titles (`h4`, receipt) 600 14/20.
- `Prompt` and dictation draft: prompt variant, `--paper-200` background,
  right-aligned, `max-width: 85 %`. Receipts at full width.
- Action chips: style of the current suggestions, `min-height: 28px`.
- Glass (`glass.css`): dock, sheets and floating drawers only; the
  thread cards stay opaque.
- Removed: `--postit`, `--postit-you`, `--postit-you-ink`,
  `--app-postit` (three theme blocks); `.postit*`, `--tilt`;
  `--shadow-note` and `--edge-light` if they have no use left; the
  `.notes … .postit` tilts; `.agent-entry .you/.msg`, `.tools`,
  `.tool-list`; the `.agent-sheet-box .postit` overrides and
  following ones. The catalog classes become `a2ui-card`,
  `a2ui-title`, `a2ui-hint`.

## Errors

- Invalid `render`: MCP tool error with the path of the faulty component
  and the violated rule; nothing is displayed.
- Invalid `report` (bounds): tool error, same principle.
- Surface rejected by zod on the UI side (catalog divergence): existing
  toast (`onError`); the contract test (below) must prevent it.
- Turn without `report`: fallback receipt (see Turn model).

## Migration

MVP, without a compatibility layer:
- catalog v2 → v3 in one go;
- removal in the same change: `suggest_next`,
  `Place::Postit/Suggestions`, `ThreadLog`, `thread.ts`, `threadItems`, the
  JSX of the `AgentThread` entries;
- kept thread: in memory according to `thread_log.rs`; to be checked at the start of the
  plan. If it is persisted, old threads are erased, not converted.

## Tests

TDD: each step starts with a red test.

Rust:
- `turn_view_tests.rs`: `insta` snapshots of `turn-N` (in progress; done
  and collapsed; cancelled; fallback receipt; no narration hence no receipt) and of the
  `report` receipt (complete, minimal).
- `render` (`rstest`): rejection of `Prompt`/`Timeline`, of more than 40
  components, of a depth > 6, of an unknown property, of an action
  other than `prompt`; snapshot of the error message.
- `Place` and `live`: serialization.
- `turns_tests.rs` (replaces `thread_log_tests`): existing cases carried over
  (chunk merging, tool collapsing, last plan, eviction); fallback
  receipt at `turn_ended`; replay of a tab.
- mcp: `report` and `render` exposed, `suggest_next` absent;
  `begin_turn` assigns the turn; `show_params` freezes the previous one; an
  answered question is frozen with the answer; `cancel_asks` freezes it as
  "cancelled".

UI:
- `catalog.render.test`: each new component (Prompt, Timeline
  collapsed/expanded, Receipt, Disclosure, Badge, Markdown, Text `note`) and the
  strict rejection by zod.
- `surfaces.test`: `routeSurfaces` → `dock` and `byTurn`; question
  closest to the microphone.
- `AgentThread.render.test` rewritten: order of turns, live →
  frozen transition, empty dock at 0 px.
- `DictationDraft`: card, prompt variant.
- **Rust ↔ TS contract**: `ui/src/a2ui/fixtures/agent-surfaces.json` is
  produced by the Rust tests (`turn_view`, `report`, tools) and validated by
  the zod catalog v3.
- `i18n-keys.test`: new keys present in all languages.

Visual check: the conversation from the screenshot and a pending question
in the dock, in light and dark, Paper and Glass.

## Delivery order

1. Catalog v3, card, removal of the post-it everywhere (Rust, zod,
   `a2ui.css`, draft, `ChartPanel`, templates panel).
2. `turn_view`: pure construction of turns and of the receipt.
3. `turns.rs` in place of `ThreadLog`; hub: `begin_turn`, `turn`,
   `live`.
4. `report` and `render` tools; removal of `suggest_next`;
   agent instructions.
5. UI: turns, `AgentDock`, `routeSurfaces`.
6. Visual check and final cleanup.

Step 1 can be delivered alone. Steps 3 to 5 form a whole: no
merge before the end of step 5.

## Implementation decisions

Refined by the plan (`docs/superpowers/plans/2026-10-03-agent-a2ui.md`) and its review:

- **Channel**: the `turn-N` surfaces go through the `SurfaceHub` like any surface; progress goes out as `McpEvent::SurfacePatch` (root data model of the turn), which `SurfaceView` applies without rebuilding the surface. The `agent` channel to the UI now carries only `Status`.
- **Opening the turn**: `begin_turn` and `UserText` at the very start of `TabAgent::prompt`, before connecting: a startup error is shown in its turn.
- **Kept turns**: constant `KEPT_TURNS = 50` (no configuration key).
- **Actions**: the follow-ups of a receipt and the only action allowed in `render` reuse the existing `suggest {prompt}` event. They expire when the next turn opens: the receipt is replaced by its version without actions and the hub rejects a late action.
- **`Receipt`** has no child: the follow-ups are a sibling `Row` in the receipt's column. The fallback receipt is built in `receipt.rs`.
- **`render`** requires a tree: unique identifiers, each component visited once (neither sharing nor cycle), no orphan, depth ≤ 6, 40 components at most.
- **Frozen questions**: a cancelled question is frozen only if its turn is still in the thread and the tab is open; otherwise it is closed.
- **`render`, safeguards**: no `primary`/`danger` button, no `tone: danger` card, no `SlideThumb` nor receipt thumbnail (rendered by the host); a `suggest` button sends exactly its label; 64 KiB at most per call; `AgentForm` errors (Snafu). Same content checks as `report` (hexadecimal color, 1 to 4 facts, lengths).
- **Lengths** of the agent's texts: title and facts 120 characters, follow-up label 60, explanation 4000, follow-up prompt 2000.
- **Frozen surfaces**: 8 at most per turn (the oldest gives way); frozen settings are read-only (fields without action, buttons removed).
- **Dock**: when a question is pending, the settings collapse into a line that reopens them (instead of collapsing beyond 45 %); the dock scrolls if it exceeds 45 % of the panel.
- **Progress**: a turn update is sent only if its data changes; a turn whose end was lost is closed when the agent goes back to idle, or interrupted by the next turn.
- **Hub**: turn handling is in `hub_turns.rs`; `hub.rs` keeps the questions, the actions and the settings.
- **New conversation**: the tab actor clears the thread itself (`clear_thread`), synchronously, before any following prompt; the relay no longer touches it.
- **Outside an open turn** (between two turns, cancelled turn), `render` and `report` are rejected: a late call is shown in no turn.
- **`render` without a data model**: literal values only (neither `{path}` nor `data`), only a button carries an action; field components (`TextField`, `Stepper`, `Slider`, `ColorField`, `Select`, `ChoiceChips`) are rejected (settings go through `show_params`); `Receipt` and `SlideThumb` are reserved for the host (via `report`); so is a `danger` `Badge`; the agent's texts contain neither bidirectional controls nor invisible characters.
- **Expiry of follow-ups**: those of a `render` card expire on the next turn, like those of a receipt.
- **Frozen question**: it shows the question (as a caption) and the answer ("✓ …"), not the answer alone; it keeps its place in the thread.
- **Turn updates**: one write per model key that changes (`/narration` while the agent writes); kept narration bounded to 4000 characters, 60 steps at most per turn.
- **Keyboard for a dock question** (t-457): 1–4 wherever the focus is; Escape abandons it only with the focus in the dock (elsewhere, Escape stays with the app: deselect); Enter stays with the app, and on a chip that has the focus, it is its click. Enter never picks a "default option" of the chips: `ask_choice` starts from an empty choice and `render` rejects chips, this path had no use. Keys cut off (editing a text, pointing, active widget): no surface shows a key reminder; a prompt always answers the question that would receive them.
- **The agent waits for the user** (t-454): only when a question receives the prompt (`answerTarget`, the same as the keys). ⌘⇧D and Escape then answer it; otherwise, during a turn, they stop it. An invisible question (a sheet whose element is not on the displayed slide) therefore does not wait: a prompt does not answer it, ⌘⇧D stops the turn.
- **Thread announcements** (t-457): the list of turns is not an `aria-live` region (a rebuilt A2UI surface recreates its whole DOM). A persistent hidden region reads, for each thread surface, its new readable text (texts, prompt, badge, title and facts of a receipt; neither buttons, nor collapsed detail, nor timeline; of a growing narration, only the continuation): a receipt replaced by its version without follow-ups, or an identical resynchronization, reads nothing. The new text is read in one go at the end of a sentence, otherwise after one second without new text (a streaming narration is not read piece by piece); a line cut by the host ("…") whose beginning was read is not read again. A whole list received (resynchronization: tab reopened, reconnection) is treated as read; a dock question that freezes into its turn reads only its answer again. A resynchronization keeps the object of an unchanged surface (`surfacesReducer`), whose view is therefore not rebuilt.
