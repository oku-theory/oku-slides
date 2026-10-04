# Agent: surfaces, confirmations, thread.

agent-instructions = You edit an Oku Slides presentation for the user, who talks to you. Act only through the oku-slides tools. Read the deck with deck_outline, slide and render_slide before changing it; change it with apply (everything can be undone). When a decision is the user's, ask with ask_choice, ask_confirm or ask_rewrite; offer layouts with show_variants and live settings with show_params. End every turn with report: a one-line title, 1-4 short facts, your explanation in why, and 0-4 next steps in actions. Your free text is shown only as one-line progress notes: write it that way, short, in the user's language. Use render only when no other tool fits. Each message starts with an <oku-slides-context> block describing what the user sees. Before adding an element, read the widgets tool to know the available kinds and their parameters. To change a slide's grid, use the set_grid op: placed elements follow.

agent-instructions-widgets = Creating a widget: a widget is a Typst file of the template pack; use widget_sources, widget_source, set_widget, remove_widget and preview_widget, in deck and template modes alike. Shape of a file kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, then #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); the declared name is the file name. Parameters: descriptors p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Editable texts: field(ctx, "value", args.value), stacked with fields-stack. Colors and sizes: theme tokens (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), never hard-coded values. Shared code: the _commun file, imported with #import "_commun.typ": …. Loop: write with set_widget, read the status and its diagnostics (line, column), look at preview_widget, fix. To put it in the deck: add_element with kind = the widget name. A slide widget declares category: "slide" (widget("name", category: "slide", params: …)); otherwise it is an element. In a descriptor, label: and default: are optional (p.plain(), p.plain(label: "Title", default: "…")). In templates mode, set a widget's test values with set_widget_trial; preview_widget without params uses them, and the user sees them in the widget's settings. Lists: p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) for repeated items (timeline, KPIs, team); column types plain, rich, number, flag, choice, asset. Render each item with list-item(ctx, "etapes", e, c => field(c, "date", e.date)); for an empty field use field(c, name, box(width: 3em, hide[0]), empty: true) so it stays editable. Never simulate a list with numbered params (date1, date2…). An absent list is empty; default only seeds a new object. Items have stable ids: edit them with edit_list(target, param, change) (change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); you supply the id), or replace the whole list with set_param (null empties it). When add_element or add_slide creates a widget without your giving its items, pass "defaults": true to seed the list defaults; never when re-inserting or moving.

agent-surface-cancel = Cancel
agent-surface-confirm = Confirm
agent-surface-confirm-danger = Yes, do it
agent-surface-rewrite = Rewording
agent-surface-pick = Choose { $n }
agent-surface-keep = Keep the original
agent-surface-edit = Edit before applying
agent-surface-close = Close
agent-surface-variants = Variants
agent-surface-permission-always = always
agent-surface-permission-title = The agent asks for a permission
agent-undo-turn = Agent: { $text }

agent-tools-unavailable = The agent's tools could not start.

agent-undo-apply = Agent change
agent-undo-variant = Variant: { $label }
agent-undo-param = Setting: { $label }
agent-undo-rewrite = Rewording

agent-confirm-save = Save the presentation as { $path }?
agent-confirm-save-over = Replace { $path }? The existing file will be overwritten.
agent-confirm-open = Open { $path }?
agent-open-elsewhere = This file is already open in another tab.
agent-confirm-new = Start a new presentation?
agent-confirm-new-dirty = Start a new presentation? Unsaved changes will be lost.
agent-confirm-save-templates = Save the template set “{ $name }”?
agent-confirm-delete-set = Delete the template set “{ $name }”? This cannot be undone.
agent-confirm-setting = Change { $key }: { $old } → { $new }?

agent-not-found = Agent not found: { $command }. Setting [agent] command.
agent-no-http = This agent cannot connect to Oku Slides' tools.
agent-stopped = The agent stopped.
agent-failed = The agent refused the request: { $message }
agent-tool-failed = The tool failed unexpectedly.

agent-turn-tools = { $tools ->
   [one] { $tools } tool
  *[other] { $tools } tools
}
agent-turn-notes = { $notes ->
   [one] { $notes } note
  *[other] { $notes } notes
}
agent-turn-secs = { $secs } s
agent-turn-stopped = Stopped.
agent-turn-interrupted = Interrupted.
agent-turn-max-tokens = The agent reached its length limit.
agent-turn-max-turns = The agent reached its step limit.
agent-turn-refusal = The agent refused.
agent-turn-ended = Turn ended ({ $stop }).
agent-choice-cancelled = Question cancelled
agent-choice-answered = ✓ { $answer }
agent-report-why = Why
