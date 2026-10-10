# Agent: surfaces, confirmations, thread.

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
agent-confirm-edit-theme = Save a new version of the theme “{ $name }”?
agent-confirm-copy-theme = Save an edited copy of the built-in theme “{ $name }”?
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
agent-memory-lost = The agent no longer remembers this conversation: it starts over.
agent-turn-max-tokens = The agent reached its length limit.
agent-turn-max-turns = The agent reached its step limit.
agent-turn-refusal = The agent refused.
agent-turn-ended = Turn ended ({ $stop }).
agent-choice-cancelled = Question cancelled
agent-choice-answered = ✓ { $answer }
agent-report-why = Why

# Errors of the agent's tools and cards (sent to the agent or shown in a card).
agent-error-file = The file could not be read or written.
agent-error-unexpected-action = Unexpected action: { $action }.
agent-error-unknown-surface = This card is no longer open.
agent-error-expired = The actions of a past turn have expired.
agent-error-incomplete = This card's action is incomplete.
agent-error-no-field = This setting no longer exists.
agent-error-no-variant = This variant no longer exists.
agent-error-arguments = The tool's arguments are invalid ({ $detail }): check them against its schema.
agent-error-unknown-tool = Unknown tool: { $name }.
agent-error-denied = The user declined.
agent-error-no-turn = No turn in progress: { $tool } is only shown during a turn.
agent-error-turn-over = The turn of this call is over: nothing was shown.
agent-error-anchor-scope = A question in a widget conversation cannot be anchored on a slide: ask it without an anchor.
agent-error-no-file = The presentation has no file yet: give an absolute path.
agent-error-path-relative = The path must be absolute.
agent-error-path-parent = The path must not contain “..”.
agent-error-setting-locked = The setting { $key } cannot be changed by the agent.
agent-error-setting = The setting { $key } could not be changed: check its value.
agent-error-rewrite-text = The rewording came back without text.
agent-error-paragraphs = The paragraphs of the rewording are invalid.
agent-error-draft-too-long = The draft is too long.
agent-error-draft-origin = Draft refused: unknown view.
agent-error-draft-no-agent = This tab's agent has not started yet: draft refused.
