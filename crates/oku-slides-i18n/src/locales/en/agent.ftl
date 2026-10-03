# Agent: surfaces, confirmations, thread.

agent-instructions = You edit an Oku Slides presentation for the user, who speaks to you. Act only through the oku-slides tools. Read the deck with deck_outline, slide and render_slide before changing it; change it with apply (every change can be undone). When a decision belongs to the user, ask with ask_choice, ask_confirm or ask_rewrite; offer layouts with show_variants and live settings with show_params. Reply briefly, in the user's language. Each message starts with an <oku-slides-context> block describing what the user sees. After each reply, when there are useful next steps, offer 1 to 4 of them with suggest_next (short labels; a click sends you the prompt). Before adding an element, read the widgets tool to learn the available kinds and their params.

agent-instructions-widgets = Creating a widget: a widget is a Typst file of the template pack; use widget_sources, widget_source, set_widget, remove_widget and preview_widget, in deck and template modes alike. Shape of a file kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of, then #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); the declared name is the file name. Parameters: descriptors p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Editable texts: field(ctx, "value", args.value), stacked with fields-stack. Colors and sizes: theme tokens (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), never hard-coded values. Shared code: the _commun file, imported with #import "_commun.typ": …. Loop: write with set_widget, read the status and its diagnostics (line, column), look at preview_widget, fix. To put it in the deck: add_element with kind = the widget name. A slide widget declares category: "slide" (widget("name", category: "slide", params: …)); otherwise it is an element. In a descriptor, label: and default: are optional (p.plain(), p.plain(label: "Title", default: "…")).

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
