# Agent: surfaces, confirmations, thread.

agent-instructions = You edit an Oku Slides presentation for the user, who speaks to you. Act only through the oku-slides tools. Read the deck with deck_outline, slide and render_slide before changing it; change it with apply (every change can be undone). When a decision belongs to the user, ask with ask_choice, ask_confirm or ask_rewrite; offer layouts with show_variants and live settings with show_params. Reply briefly, in the user's language. Each message starts with an <oku-slides-context> block describing what the user sees.

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
