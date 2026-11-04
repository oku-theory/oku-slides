# Agent: surfaces, confirmations, thread.

agent-surface-cancel = Cancel
agent-surface-confirm = Confirm
agent-surface-confirm-danger = Yes, do it
agent-surface-heard = Heard: “{ $text }”
agent-surface-say-or-click = Say your answer or click a button.
agent-surface-said-too-early = Said too early: wait for the window, then say it again.
agent-surface-click-to-confirm = Click the button to confirm.
agent-surface-verb-replace = Replace
agent-surface-verb-merge = Merge
agent-surface-verb-discard = Discard
agent-surface-verb-delete = Delete
agent-surface-verb-use-api = Use API
agent-surface-verb-install = Install
agent-surface-verb-quit = Quit
agent-surface-verb-close = Close
agent-surface-verb-import = Import
agent-surface-rewrite = Rewording
agent-surface-pick = Choose { $n }
agent-surface-keep = Keep the original
agent-surface-edit = Edit before applying
agent-surface-close = Close
agent-surface-themes = Themes
agent-surface-variants = Variants
agent-surface-permission-always = always
agent-surface-permission-title = The agent asks for a permission
agent-surface-permission-guides = This turn includes the received guide of { $names }.
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

agent-not-found = Agent not found: { $command } exists neither at that path nor in the app's PATH. Give its full path in [agent.presets].
agent-spawn-failed = The agent could not start: { $command }. Setting [agent.presets].
agent-no-http = This agent cannot connect to Oku Slides' tools.
agent-local-no-program = The local agent can only run inside the Oku Slides app.
agent-local-needs-download = The local agent needs its model ({ $gigabytes } GB) to be downloaded first.
agent-local-intel-mac = The local agent needs a Mac with Apple silicon.
agent-local-unsupported-os = The local agent is not available on this system yet.
agent-local-apple-off = The local agent needs Apple Intelligence: turn it on in System Settings.
agent-local-apple-not-ready = Apple's model is still getting ready: try the local agent again in a few minutes.
agent-local-apple-unsupported = Apple's model needs a Mac eligible for Apple Intelligence, with macOS 26 or later.
agent-model-download = Download the model
agent-model-downloading = Downloading the model… { $percent } %
agent-model-cancel = Cancel
agent-model-download-failed = The model could not be downloaded.
agent-model-download-integrity = The downloaded model was damaged and has been deleted.
agent-model-download-incomplete = Incomplete download: { $got } bytes out of { $expected }.
agent-model-download-cancelled = Download cancelled.
agent-fallback-notice = Local agent — { $reason }
agent-fallback-download = { $reason } The local agent can take over once its model ({ $model }, { $gigabytes } GB) is downloaded. Download it now?
agent-fallback-unusable = No agent can run. Main agent: { $main } Local agent: { $local }
local-agent-no-route = The local agent did not understand what to do. Try saying it another way.
local-agent-tool-limit = The local agent stopped after { $n } steps without finishing.
local-agent-invalid-call = The local agent could not use the tools correctly.
local-agent-engine-failed = The local agent's model failed.
local-agent-retry = Try again, or choose another agent in the agent panel.
agent-stopped = The agent stopped.
agent-failed = The agent refused the request: { $message }
agent-error-work-dir = The agent's private working folder for this tab could not be prepared.
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
agent-trace = { $name }: { $count ->
    [one] { $count } change
   *[other] { $count } changes
}
agent-trace-data = { $name }: data changed
agent-trace-removed = { $name }: no longer exists
agent-trace-more = { $count ->
    [one] and { $count } other
   *[other] and { $count } others
}
agent-trace-title = Elements changed

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
agent-error-tab-no-turn = Open, show or move a tab only while answering the user.
agent-error-tab-limit = You already opened, showed or moved a tab in this turn; finish there, or ask the user before another.
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
agent-error-no-box-turn = No turn to attach this box to: nothing was shown.
agent-error-no-file = The presentation has no file yet: give an absolute path.
agent-error-path-relative = The path must be absolute.
agent-error-path-parent = The path must not contain “..”.
agent-error-setting-locked = The setting { $key } cannot be changed by the agent.
agent-error-setting = The setting { $key } could not be changed: check its value.
agent-error-history-index = Moving through the history needs the index of a version.
agent-error-no-pack = This deck pins no pack: give pack (see packs).
agent-error-history-not-opened = The tab did not open the history in time: ask again.
agent-error-locale = Unknown language code. Give a shipped language code or system.
agent-error-empty-interface = Nothing to change: give scheme, locale, guides, folded, pile, notes or code.
agent-error-layout-range = { $field } must be from { $min } to { $max } px.
agent-error-layout-min = { $field } must be at least { $min } px.
agent-error-empty-dictation = Nothing to change: give at least one dictation setting.
agent-error-empty-speech = Nothing to change: give at least one read-aloud setting.
agent-error-not-shown = This tab is not the one shown in its window: ask again once the user shows it.
agent-error-not-presenting = The deck is not being presented here: start it with the action start first.
agent-error-present-slide = Showing a slide needs the id of a slide of this deck (see deck_outline).
agent-error-undo-open-turn = undo_turn does not undo the turn it runs in: use undo for this turn's own changes.
agent-error-app-unavailable = Only the Oku Slides app can do this; it is not available here.
agent-error-app-closing = Oku Slides is already closing.
agent-error-app-no-offer = There is no update to put off or install here: check for updates first.
agent-error-app-failed = The Oku Slides updater failed: check the connection, or use the update banner.
agent-error-after-turn-taken = This reply already ends with quitting, installing an update or closing its tab.
agent-error-no-pack-update = The packs panel offers no such update for this pack: see show_packs.
agent-confirm-update-install = Install Oku Slides { $version }? The app restarts once this reply ends.
agent-after-turn-quit-unsaved = Oku Slides did not quit: a tab has had unsaved changes since you asked.
agent-error-workspace-unavailable = Only the Oku Slides app can change tabs and windows; it is not available here.
agent-error-workspace-unknown-tab = This window has no such tab or window.
agent-error-workspace-unknown-window = There is no such window.
agent-error-workspace-index = That position is past the end of the tab bar.
agent-error-workspace-closing = Oku Slides is closing: tabs and windows no longer change.
agent-confirm-close-tab = Close the tab “{ $tab }”?
agent-confirm-discard-tab = Close the tab “{ $tab }”? Its unsaved changes will be lost.
agent-confirm-close-window = Close this window and its tabs?
agent-confirm-discard-window = Close this window? Unsaved changes in { $tabs } will be lost.
agent-error-window-last = This is the only window: close its tabs instead, or quit Oku Slides.
agent-error-tab-index = Say where to put the tab: reordering needs an index.
agent-error-tab-unused = Only reorder and move take an index, and only move takes a window.
agent-after-turn-close-unsaved = The tab was not closed: it has changed since you asked.
agent-after-turn-close-not-shown = The tab was not closed: another tab was shown in its window when your reply ended.
agent-confirm-quit = Quit Oku Slides? The app quits once this reply ends.
agent-error-quit-unsaved = Tabs with unsaved work: { $count }. Oku Slides did not quit: the user saves or closes them first.
agent-error-tune-unavailable = The fine-tuning could not start: the agent is not available.
agent-error-tune-not-started = The fine-tuning did not start: that tab's agent is unusable for now; the tab says why.
agent-confirm-import = Import { $path } in a new tab?
agent-import-opened-in-tab = { $shown ->
   [yes] The presentation opened in tab { $tab }, now shown.
  *[no] The presentation opened in tab { $tab }, behind: the user's window shows another tab.
} { $tune ->
   [started] Its fine-tuning starts there, in that tab's own conversation.
   [unavailable] Its fine-tuning could not start: tell the user to ask for it in that tab.
  *[no] Tell the user to continue there.
} This tab stays as it is.
agent-error-import-hifi-target = A hi-fi import takes no pack or theme: drop them, or import in retrofit mode.
agent-error-import-extension = Only a PowerPoint .pptx file can be imported.
agent-error-a2ui-unknown = { $name } is not a component render accepts. Components: { $known }.
agent-error-a2ui-elsewhere = render does not show { $name }: use { $tool }.
agent-error-a2ui-host-only = { $name } is reserved to the app: render cannot show it.
agent-error-themes-deck-mode = The theme picker opens only while editing the deck's slides.
agent-error-packs-deck-mode = The deck packs panel opens only while editing the deck's slides.
agent-error-usage-no-deck = This tab's usage belongs to no deck: only a pack or theme tab opened from a deck (by the user or the agent), or a deck's own pack, counts toward that deck.
agent-error-rewrite-text = The rewording came back without text.
agent-error-paragraphs = The paragraphs of the rewording are invalid.
agent-error-draft-too-long = The draft is too long.
agent-error-report-elsewhere = This window does not show that tab.
agent-error-ui-too-large = The page context is too large.
agent-error-seq-exhausted = The page report number is out of range.
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
voice-error-hands-free-reserved = Only the user turns hands-free listening on: open the box with show_dictation and ask them
voice-error-changed = The settings changed since the question: ask again
voice-error-nothing-to-download = Nothing to download for this agent
voice-error-bad-rate = The speech rate must be 0.5 to 2
voice-error-unknown-voice = Unknown voice for the interface language
voice-error-speech-unavailable = Read-aloud is not available on this computer
agent-tune-prompt = Fine-tune the imported presentation
agent-settings-unknown-selected = Setting agent.selected = { $selected } names no agent of [agent.presets] (known: { $known })
agent-settings-preset-launch = Agent [agent.presets.{ $id }] needs exactly one of command or builtin
