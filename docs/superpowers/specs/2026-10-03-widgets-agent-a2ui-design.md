# Editing widgets with the agent, in the A2UI area — design

Task: t-423. Depends on t-416 (`2026-10-03-agent-a2ui-design.md`), step 5
(dock) merged, for the dock and the rich text A2UI component. Follow-up to
t-394 (widgets in sub-tabs) and t-422 (preview on top, code below).

## Problem

In templates mode, the right panel shows "Ce modèle" (This template) and "Grille" (Grid)
even when a widget sub-tab is active: these settings apply to the
template hidden behind, not to the edited widget. The widget itself has no
settings: its preview always takes the default or sample values from
`preview_params`. The agent writes widgets (`set_widget`) without the
editor following it, and does not know which widget the user is looking at.

## Decisions

- **A single settings area: the t-416 dock.** The host's live settings
  (template, widget) and the agent's (`show_params`) live there.
  `PanelSurface` and the separate `panel` path in `App.tsx` go away.
- **A single field builder**: the one from `show_params`
  (`agent/forms.rs`). The template and widget settings are
  `ShowParams` built by the host, not a second A2UI builder.
- **Each host field is addressed by a key**, never by its
  position: `name`, `description`, `columns`, `rows` for the template, the
  parameter name for a widget. The key travels in the field's action.
- **Trial values in the session**, per widget, in memory, in
  `Mode::Templates`: never written to the pack, lost when leaving
  templates or switching pack. The preview, the interface and the agent read
  the same source. A trial is not a change to the deck.
- **The agent sets trials through a dedicated tool**, `set_widget_trial`
  (see below), and not through a widget target of `show_params`: the host
  knows the schema, the agent gives only values.
- **`set_grid` is a deck op** (`Op::SetGrid`), visible to the agent in
  the ops schema: it changes the grid and refits the elements. This
  deliberately reverses t-294, which had removed `set_grid` from the schema
  when the grid became a `grid` parameter.
- **The user's keystrokes win** over an agent write to
  an unsaved file; the agent is told on the next turn. An
  agent **deletion**, however, wins over the pending keystrokes
  of that file.

## Template settings

- `panel.rs` no longer builds an A2UI surface: it produces a `ShowParams`
  (name, description as `Text`; columns and rows as `Stepper`, 1 to 24),
  whose fields carry the op they apply, like an agent `show_params`.
  Name and description: `set_slide_field` (`name`, `description`).
  A block slide has no grid fields.
- **Grid.** New op `set_grid {slide, columns?, rows?}` in
  `oku-slides-core`: an absent field stays unchanged; 1 to 24 tracks; same
  grid: nothing changes. It writes the `grid` parameter (the gutter,
  including that of the declared default, is kept; the parameter is read back
  as a `Grid`, other keys do not survive in it) and refits, with the
  same rounded proportions (rule of the old `rescaleCell`), the cell
  of the elements of the slide's free slots (`SlotMode::Free`); only the
  cells move, never the frames. It undoes in one step.
- **Starting grid**: the slide's `grid` parameter, otherwise the default grid
  declared by the slide's widget (`p.grid(default: …)`, read
  from the registry), otherwise 12 × 6 (`Slide::resolved_grid`). `set_cell`, deck
  validation and the widget preview use the same grid, and the
  panel shows it. The grid fields appear only if the slide's widget
  declares a `grid` parameter, the slide has a free zone, and
  it has no blocks.
- `rescaleCell` moves to Rust and disappears from `grid.ts` (only
  `panelActions.ts` uses it): a single implementation.
- `panelActionOps` (`ui/src/a2ui/panelActions.ts`) goes away: the surface
  applies its ops like any settings surface.
- Surface with id `host-template`, `live: true`, `turn: None`, shown as long
  as a template is displayed and no widget sub-tab is active.

## Widget settings

- `Session::widget_settings(name)`: one field per declared parameter, in
  schema order, and read-only rows. `None` for Commun (Common)
  (the template card then stays displayed).

  | Type | Field |
  |---|---|
  | `plain` | `Text` |
  | `rich` | `RichText` (new A2UI component, see below) |
  | `number` with `min` and `max` | `Slider` (step: a hundredth of the range if it is ≤ 10, otherwise 1) |
  | `number` otherwise | `Text` (the text is read as a number) |
  | `choice` | `Select` (its values) |
  | `flag` | `Select` (yes / no) |
  | `asset`, `slot`, `grid`, `host` | read-only row "non réglable dans l'aperçu" (not adjustable in the preview) |

- Value of each field: the trial value if it exists, otherwise the one from
  `preview_params` (schema default, localized sample, neutral value).
  What the field shows is therefore what the preview draws.
- A widget field carries its parameter's key; it carries no deck
  op: nothing goes into the history.
- Widget without parameters: the card says "Ce widget n'a pas de paramètre"
  (This widget has no parameters).
- **Invalid file** (while typing): the `host-widget` card stays,
  with the single row "Fichier invalide : réglages indisponibles" (Invalid file:
  settings unavailable); the trials are kept.
- Surface with id `host-widget`, `live: true`, `turn: None`, shown as long as
  this widget's sub-tab is active; it replaces `host-template`.

## Rich text in the dock

- New A2UI catalog component, `RichTextField {label, value,
  action}`: a multiline editor where the editing markdown is formatted
  as you type (bold, italic, links, highlights, `- ` and `1. ` lists),
  with ⌘B and ⌘I. It reuses what the scene editor uses
  (`markdownDecorations`, `toggleEmphasis`).
- The field reads two paths of the data model: the editing markdown
  (`paragraphs_to_text`, shown) and the original paragraphs. On
  commit (focus loss, ⌘↩), it writes both and the action goes out with
  the paragraphs, built by `textToParagraphs` from the
  original paragraphs: the colour and size of an unchanged line are
  kept. The markdown path stays a text.
- `show_params` gains the `rich` field kind: the agent gives its value as
  paragraphs; it can thus offer a rich text setting of the deck.

## Trial values (session)

- `Mode::Templates` gains each widget's trials and the widgets written
  by the agent (see "Agent"), both reset on entry, on
  pack switch and on tab resume.
- `set_widget_trials(name, {param: value})`: all or nothing; checks
  each value against the declared type (Snafu error otherwise, naming the
  parameter). `null`, an empty text, an empty paragraph list, or a
  value equal to the one from `preview_params` (numbers compared as numbers)
  clears the trial. A `plain` text of more than
  4 KiB, or a `rich` of more than 64 paragraphs or 16 KiB, is refused.
- `preview_widget(name, params)`: given parameters, then trials, then
  `preview_params`. The interface no longer sends parameters.
- Renaming a widget: its trials follow it. Deletion: they are
  cleared. Writing a widget: the trials of parameters that are no longer
  declared, or whose type changed, are cleared.
- A trials revision (`Snapshot.widget_trials`) reruns the previews.

## Agent

- **Context**: `AgentUi` gains `widget: Option<String>` (the active
  sub-tab). The `<oku-slides-context>` block states the open widget and its
  trial values (`widget: carte · trial: "title" = "Météo"`), names
  in JSON quotes, each value cut at 80 characters, 10 trials
  at most; an invalid widget name is not quoted.
- **Tool `set_widget_trial`** `{name, values: {param: value}}`: sets
  the trials (all or nothing), returns the retained values; the
  `host-widget` card and the preview follow. Refused outside templates mode.
- **`preview_widget`** without parameters takes the trials: the agent sees what
  the user sees.
- **Provenance**: the agent's `set_widget` tool calls a dedicated
  session function (`set_widget_by_agent`); the interface request
  carries no origin flag. A saved agent write
  marks the widget; the user's first saved write to
  a marked widget notes for the agent "user edited widget <nom> after
  you". Renaming moves the mark, deletion clears it.
- **The editor follows the agent**: in templates mode only, successful `set_widget` and
  `remove_widget` emit `McpEvent::WidgetTouched {name,
  removed}`. The interface opens the written widget's sub-tab and activates it,
  unless the active sub-tab has unsent or unsaved
  keystrokes: it then opens in the background. A widget created by
  the agent opens as soon as the file list contains it.
- **Conflict**: if the file written by the agent has unsent
  or unsaved keystrokes, those keystrokes are written (they win); the note
  above informs the agent. **Deletion**: the pending keystrokes of the
  deleted file are abandoned, the sub-tab closes, and a message
  says the agent deleted the widget.
- Agent instructions: one sentence for `set_widget_trial` and
  `preview_widget` in templates mode, and one for `set_grid`, in the 20
  languages.

## Dock (UI)

- `AgentDock` (t-416) accepts, besides the agent's live surfaces,
  the host settings surface (`host-template` or `host-widget`),
  provided by `App`.
- Order, from the microphone upwards: pending question, agent settings,
  host settings. When a question is pending, the settings collapse
  to their header row (t-416 rule), the host's included.
- An agent `show_params` on the displayed slide and `host-template`
  coexist; there are never two settings surfaces for the same
  widget (the agent goes through `set_widget_trial`, which updates
  `host-widget`).
- The host settings have no "×": they follow what is
  displayed. They never freeze in a turn. They are not shown
  during history.
- The surface is read again only if the slide, the widget, the pack, the trials
  revision or the deck state (fingerprint and name of the slide, undo)
  change; an action goes out with the slide and widget of the displayed
  surface, never those of a more recent state.
- A widget whose sub-tab is stale (deleted, renamed) gives the
  template card, without error. Each committed template setting makes one
  undo step.

## Errors

- Trial value of the wrong type, out of bounds or too long: localized Snafu
  error naming the parameter, toast on the UI side; on the agent side, tool
  result in error.
- `set_widget_trials` on an unknown widget or Commun (`UnknownWidget`),
  invalid (`WidgetInvalid`) or outside templates mode (`NotEditingTemplates`):
  explicit refusal.
- Host setting outside templates mode, or unknown key: refusal.
- Failed preview: diagnostics as today; the settings remain.

## Tests

- Rust (`oku-slides-core`): `set_grid` refits like the old
  `rescaleCell` (rounding included), starts from the grid declared by the
  widget, touches neither the frames nor the gutter, refuses 0 and 25,
  changes nothing for an equal grid; the ops schema names `set_grid`.
- Rust (`oku-slides-session`): `widget_settings` gives the right field per
  type, the values equal to `preview_params`, nothing for Commun, the empty
  card without parameters, the "invalid file" row; `set_widget_trials`
  checks type and length, clears a trial equal to the default or empty,
  follows a rename, prunes on write; `preview_widget` renders the
  trials; the context block quotes the widget and its trials, without
  injection; the "after you" note; `template_params` and its keys;
  `set_grid` through a host setting undoes in one step.
- Rust (`oku-slides-mcp`): `set_widget_trial` refused outside templates and
  emits `Changed`; `set_widget`/`remove_widget` emit `WidgetTouched` in
  templates mode only.
- UI: the dock shows `host-template`, then `host-widget` on sub-tab
  change; a changed field requests the preview again; `WidgetTouched` opens
  and activates the sub-tab, in the background if keystrokes are pending,
  including for a new widget; unsaved keystrokes are not
  overwritten; an agent deletion abandons the pending keystrokes;
  `RichTextField` formats and returns paragraphs.
- i18n: new keys in the 20 languages (`i18n-keys.test` and
  `oku-slides-i18n`).
- e2e: `panel.cjs` goes through the dock; a widget walkthrough (setting →
  preview content changed). Visual check in light and dark.

## Out of scope

- Keeping trials between two sessions, or writing them as samples in
  the pack.
- Setting `asset`, `slot`, `grid` or `host` in the preview.
- Writing a run's colour and size in editing markdown.
