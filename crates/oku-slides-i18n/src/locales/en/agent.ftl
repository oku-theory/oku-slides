# Agent: surfaces, confirmations, thread.

agent-surface-cancel = Cancel
agent-surface-confirm = Confirm
agent-surface-confirm-danger = Yes, do it
agent-surface-rewrite = Rewording
agent-surface-pick = Choose { $n }
agent-surface-keep = Keep the original
agent-surface-edit = Edit before applying
agent-surface-close = Close
agent-surface-themes = Themes
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
agent-confirm-copy-pack = Save a copy of the pack “{ $name }”? The pack itself stays as it is.
agent-confirm-copy-edited-theme = Save a copy of the theme “{ $name }”? The theme itself stays as it is.
agent-confirm-delete-pack = Delete the pack “{ $name }”? This cannot be undone.
agent-confirm-edit-theme = Save a new version of the theme “{ $name }”?
agent-confirm-copy-theme = Create a variant of the built-in theme “{ $name }”? The built-in theme stays unchanged.
agent-confirm-setting = Change { $key }: { $old } → { $new }?
agent-confirm-move = Move the presentation to { $path }? The old file will be deleted.
agent-confirm-merge = Merge { $path } into this presentation? This cannot be undone.
agent-confirm-save-copy = Save a copy of the presentation as { $path }?
agent-confirm-dictation-api = Send what you dictate to { $host } ({ $url }), model { $model }, with the secret of { $var }?
agent-confirm-dictation-download = Use { $name } and download it now ({ $size })?
agent-confirm-dictation-language = Use Apple dictation and download its language now?
agent-confirm-agent = Switch the app agent to { $name } ({ $command }) once no reply is running?
agent-confirm-agent-builtin = Switch the app agent to { $name } once no reply is running?
agent-confirm-agent-model = Download the local agent model ({ $size })?

agent-not-found = Agent not found: { $command }. Setting [agent.presets].
agent-spawn-failed = The agent could not start: { $command }. Setting [agent.presets].
agent-no-http = This agent cannot connect to Oku Slides' tools.
agent-local-no-program = The local agent can only run inside the Oku Slides app.
agent-local-needs-download = The local agent needs its model ({ $gigabytes } GB) to be downloaded first.
agent-local-intel-mac = The local agent needs a Mac with Apple silicon.
agent-local-unsupported-os = The local agent is not available on this system yet.
agent-model-download = Download the model
agent-model-downloading = Downloading the model… { $percent } %
agent-model-cancel = Cancel
agent-model-download-failed = The model could not be downloaded.
agent-model-download-integrity = The downloaded model was damaged and has been deleted.
agent-model-download-incomplete = Incomplete download: { $got } bytes out of { $expected }.
agent-model-download-cancelled = Download cancelled.
local-agent-no-route = The local agent did not understand what to do. Try saying it another way.
local-agent-tool-limit = The local agent stopped after { $n } steps without finishing.
local-agent-invalid-call = The local agent could not use the tools correctly.
local-agent-engine-failed = The local agent's model failed.
local-agent-retry = Try again, or choose another agent in the agent panel.
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
agent-error-pack-id = This is not the id of a pack: call packs to list the packs and their ids.
agent-error-asset-unknown = This image is neither in the deck nor among the pasted images: use the id of a pasted image.
agent-error-denied = The user declined.
agent-error-no-turn = No turn in progress: { $tool } is only shown during a turn.
agent-error-tab-no-turn = Open or show a pack or theme tab only while answering the user.
agent-error-tab-limit = You already opened or showed a pack or theme tab in this turn; finish there, or ask the user before another.
agent-error-continue-in-other-tab = This request continues in the tab you opened or showed: ask the user to go on there; this tab changes no widget for now.
agent-error-pack-open-in-tab = This pack is being edited in another tab: ask the user to finish there first.
agent-pack-opened-in-tab = { $shown ->
   [yes] The pack opened in tab { $tab }, now shown.
  *[no] The pack opened in tab { $tab }, behind: the user's window shows another tab.
} { $draft ->
   [set] The request waits in that tab's draft: tell the user to review it there, then send it.
   [kept] That tab's draft already held something, kept as is. Tell the user to continue there: that tab's agent does not know this request.
   [too_long] The request was too long for that tab's draft. Tell the user to continue there: that tab's agent does not know this request.
   [unavailable] The request could not reach that tab's draft. Tell the user to continue there: that tab's agent does not know this request.
  *[none] Tell the user to continue there: that tab's agent does not know this request.
} This tab stays as it is.
agent-theme-opened-in-tab = { $shown ->
   [yes] The theme opened in tab { $tab }, now shown.
  *[no] The theme opened in tab { $tab }, behind: the user's window shows another tab.
} { $draft ->
   [set] The request waits in that tab's draft: tell the user to review it there, then send it.
   [kept] That tab's draft already held something, kept as is. Tell the user to continue there: that tab's agent does not know this request.
   [too_long] The request was too long for that tab's draft. Tell the user to continue there: that tab's agent does not know this request.
   [unavailable] The request could not reach that tab's draft. Tell the user to continue there: that tab's agent does not know this request.
  *[none] Tell the user to continue there: that tab's agent does not know this request.
} This tab stays as it is.
agent-error-turn-over = The turn of this call is over: nothing was shown.
agent-error-no-file = The presentation has no file yet: give an absolute path.
agent-error-path-relative = The path must be absolute.
agent-error-path-parent = The path must not contain “..”.
agent-error-setting-locked = The setting { $key } cannot be changed by the agent.
agent-error-setting = The setting { $key } could not be changed: check its value.
agent-error-history-index = Moving through the history needs the index of a version.
agent-error-no-pack = This deck pins no pack: give pack (see packs).
agent-error-history-not-opened = The tab did not open the history in time: ask again.
agent-error-present = Nothing to present here: no slide, or the tab is in pack, history or read-only mode.
agent-error-locale = Unknown language code. Give a shipped language code or system.
agent-error-empty-interface = Nothing to change: give scheme, locale or guides.
agent-error-empty-dictation = Nothing to change: give at least one dictation setting.
agent-error-not-shown = This tab is not the one shown in its window: ask again once the user shows it.
agent-error-themes-deck-mode = The theme picker opens only while editing the deck's slides.
agent-error-packs-deck-mode = The deck packs panel opens only while editing the deck's slides.
agent-error-rewrite-text = The rewording came back without text.
agent-error-paragraphs = The paragraphs of the rewording are invalid.
agent-error-draft-too-long = The draft is too long.
agent-error-draft-origin = Draft refused: unknown view.
agent-error-draft-no-agent = This tab's agent has not started yet: draft refused.
agent-error-too-many-images = The draft already has all its images.
agent-error-image-not-base64 = The pasted image did not arrive whole.
agent-error-image-gone = A pasted image is no longer available: paste it again.
agent-error-unknown-agent = This agent is no longer in the configuration.
agent-error-agent-locked = The agent choice is set by the environment.
agent-error-agent-not-saved = The agent choice could not be saved.
agent-error-unknown-model = This agent does not offer that model.
agent-error-model-not-saved = The model choice could not be saved.
voice-error-no-dictation = Dictation is not available here
voice-error-busy = Dictation is in use: try again once it is idle
voice-error-unknown-model = Unknown dictation model
voice-error-unavailable = This model does not support the chosen language
voice-error-installed = This model is already installed
voice-error-not-downloadable = This model is not downloaded
voice-error-nothing-to-cancel = No download is running
voice-error-bad-silence = The silence must last 0.5 to 10 seconds
voice-error-bad-language = Unknown dictation language
voice-error-bad-url = The API address must start with https:// (http:// only for this computer), without user name, parameters or special characters
voice-error-bad-key-env = The key variable must be a name like OPENAI_API_KEY: capitals, digits and _
voice-error-bad-api-model = The API model name may only use letters, digits and . _ : / -; an Azure deployment name only letters, digits and . _ -
voice-error-bad-api-version = The Azure API version uses only letters, digits, . and -, 32 characters at most, such as 2025-03-01-preview
voice-error-bad-api-headers = Headers are written Name: value; Name: $VARIABLE, 8 at most. Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection and duplicates are refused
voice-error-api-reserved = Only the user sets the API address, key variable, Azure version and headers: open the box with show_dictation and ask them
voice-error-changed = The settings changed since the question: ask again
voice-error-nothing-to-download = Nothing to download for this agent
agent-tune-prompt = Fine-tune the imported presentation
agent-settings-unknown-selected = Setting agent.selected = { $selected } names no agent of [agent.presets] (known: { $known })
agent-settings-preset-launch = Agent [agent.presets.{ $id }] needs exactly one of command or builtin
