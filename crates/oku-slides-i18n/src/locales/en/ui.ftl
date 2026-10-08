# Interface de l’app (React). Chargé par ui/src/i18n.ts via la requête « messages ».
ui-loading = Loading…

## Document

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · templates
ui-title-rename = Rename the presentation
ui-title-field = Presentation title
ui-status-unsaved = unsaved changes
ui-status-saved = saved
ui-status-never-saved = not saved yet
ui-caption-templates = { $count ->
    [one] { $count } template
   *[other] { $count } templates
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } slide
   *[other] { $count } slides
} · { $status }
ui-template-default-name = Template
ui-template-variant-name = { $name } (variant)
ui-confirm-quit-templates = Quit without saving the modified templates?

## Toasts

ui-toast-templates-saved = Templates saved: { $name }
ui-toast-moved = Presentation moved
ui-toast-copied = Copy saved
ui-toast-select-zone = Select an area to place: “{ $text }”
ui-toast-field-gone = The field you were editing no longer exists: your input is lost.

## Toolbar

ui-toolbar-undo = Undo
ui-toolbar-undo-label = Undo: { $label }
ui-toolbar-redo = Redo
ui-toolbar-redo-label = Redo: { $label }
ui-toolbar-grid-hide = Hide the grid
ui-toolbar-grid-show = Show the grid
ui-toolbar-ghosts-hide = Hide the ghosts
ui-toolbar-ghosts-show = Show the ghosts
ui-divider-pile = Width of the slide pile
ui-divider-notes = Width of the agent thread
ui-divider-code = Code size
ui-toolbar-theme = Theme
ui-toolbar-theme-embedded = File theme
ui-toolbar-save-copy = Save a copy
ui-toolbar-finish = Done
ui-menu-workshop = Workshop
ui-toolbar-open = Open
ui-merge = Merge with a PDF…
ui-menu = Menu
ui-menu-move = Move…
ui-language = Language
ui-language-system = System language
ui-scheme = Appearance
ui-scheme-system = System
ui-scheme-light = Light
ui-scheme-dark = Dark
ui-toolbar-add-label = Add to the slide
ui-add-kind = { $kind ->
    [card] Card
    [shape] Shape
    [chart] Chart
   *[text] Text
}
ui-add-widget = Pack widget
ui-toolbar-remove-template = Delete the template
ui-toolbar-remove-slide = Delete the slide
ui-toolbar-remove-hint = Slide { $n } · undo with ⌘Z

## Undo labels

ui-undo-add = { $kind ->
    [card] Add a card
    [shape] Add a shape
    [chart] Add a chart
   *[text] Add a text
}
ui-undo-add-widget = Add “{ $name }”
ui-undo-new-template = New template: { $name }
ui-undo-new-slide = New slide: { $name }
ui-undo-templates = Templates: { $name }
ui-undo-move-template = Move the template
ui-undo-move-slide = Move the slide
ui-undo-remove-slide = Delete the slide
ui-undo-remove = Delete
ui-undo-cut = Cut
ui-undo-paste = Paste
ui-undo-place-on-grid = Place on the grid
ui-undo-move-on-grid = Move on the grid
ui-undo-resize-on-grid = Resize on the grid
ui-undo-change-anchor = Change the anchoring
ui-undo-move = Move
ui-undo-resize = Resize
ui-undo-edit-slide = Edit slide
ui-undo-dictation = Dictation
ui-undo-rename-set = Rename the set
ui-undo-rename-deck = Rename the presentation
ui-undo-chart-kind = Change the chart type
ui-undo-chart-data = Edit the data
ui-undo-chart-color = Color a series
ui-undo-chart-stack = { $stacked ->
    [yes] Stack the series
   *[no] Unstack the series
}

## New elements

ui-new-text = New text
ui-new-card-title = Card title
ui-new-card-detail = Detail

## Slide pile

ui-pile-slides = Slides
ui-pile-templates = Templates
ui-pile-add-slide = + New slide
ui-pile-add-template = + New template
ui-pile-drag-hint = Drag to move · ⌥↑ ⌥↓
ui-slide-alt = Slide { $n }

## Stage

ui-stage-rendering = Rendering…
ui-stage-elements = Slide elements
ui-stage-fields = Slide fields
ui-stage-edit-text = Edit the text
ui-stage-hint-plain = Enter or Tab for next · Esc to cancel
ui-stage-hint-rich = Tab for next · ⌘ Enter to confirm · “- ” for a bullet · ⌘B bold, ⌘I italic, ⌘K link
ui-field = { $kind ->
    [title] Title
    [subtitle] Subtitle
    [kicker] Kicker
    [text] Text
    [body] Body
    [author] Author
    [date] Date
    [organization] Organization
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Text
    [card] Card
    [image] Image
    [shape] Shape
    [group] Group
   *[other] { $kind }
}
ui-grid-cell = col. { $cols } · row { $rows }
ui-ghost-zone = Zone

## Slide notes

ui-notes = Slide tools
ui-stepper-less = { $label }: one fewer
ui-stepper-more = { $label }: one more
ui-list-item = item
ui-list-add = Add { $item }
ui-list-remove = Remove { $item }
ui-list-move = Move { $item }
ui-list-full = The list is full ({ $max })
ui-undo-list-remove = Remove { $item }
ui-undo-list-move = Move { $item }
ui-agent-color-other = Other color
ui-agent-thread = Agent
ui-agent-new-conversation = New conversation
ui-draft = Draft
ui-draft-insert = Insert
ui-draft-insert-title = Insert as is (⌘⇧↩)
ui-draft-send = Send to the agent
ui-draft-send-title = Send to the agent (⌘↩)
ui-draft-discard = Discard
ui-agent-state-idle = Agent
ui-agent-state-starting = Starting the agent…
ui-agent-state-working = The agent is working…
ui-agent-dock = In progress
ui-agent-dock-settings = Settings
ui-agent-jump = ↓ Current turn
ui-agent-sheet = Agent question
ui-agent-sheet-keys = 1–4 choose · Enter confirm · E edit · Esc cancel
ui-agent-variants = Variants
ui-agent-drawer-keys = Click or 1–4 · Esc closes
ui-agent-confirm = Agent confirmation

## Dictation

ui-dictation = Dictation
ui-dictation-start = Dictate
ui-dictation-start-title = Dictate (⌘D)
ui-dictation-type = Write to the agent
ui-dictation-type-title = Write to the agent (⌘K)
ui-dictation-agent-stop = Stop the agent (⌘⇧D or Esc)
ui-dictation-agent-working = The agent is working…
ui-dictation-agent-working-hint = ⌘⇧D or Esc stops it
ui-dictation-finish = Finish dictation
ui-dictation-finish-title = Finish (⌘D)
ui-dictation-cancel = Cancel
ui-dictation-preparing = Preparing dictation…
ui-dictation-heard-nothing = Nothing heard: speak a little closer to the microphone
ui-dictation-to-cursor = at the cursor
ui-dictation-to-draft = into the draft
ui-dictation-to-field = replace the { $field }
ui-dictation-to-block = after the selected block
ui-dictation-new-block = new text block
ui-dictation-select-zone = select an area
ui-dictation-downloading = Downloading the { $name } model
ui-dictation-download-progress = { $percent }% · { $received } of { $total } · only once
ui-dictation-listening = Listening · { $time }
ui-dictation-transcribing = Transcribing the last sentence…
ui-dictation-stop-in = Stopping in { $duration }
ui-dictation-speak = Speak, the text appears here…
ui-dictation-auto-stop = stops after { $duration } of silence
ui-dictation-to-download = { $size } to download
ui-dictation-settings = Dictation settings
ui-dictation-model-title = Recognition model
ui-dictation-model-intro = Whisper, on this computer (CTranslate2). Every model understands several languages.
ui-dictation-note-turbo = Recommended, fast and accurate
ui-dictation-note-large = The most accurate, slower
ui-dictation-note-base = Light, for small machines
ui-dictation-installed = installed
ui-dictation-not-installed = to download
ui-dictation-language = Spoken language
ui-dictation-language-auto = Automatic detection
ui-dictation-note-apple = The fastest, built into the system
ui-dictation-model-unavailable = Language not supported
ui-dictation-model-intro-apple = Apple, built into the system, or Whisper, on this computer (CTranslate2). All of them understand several languages.
ui-dictation-language-auto-resolved = Automatic ({ $language })
ui-dictation-to-download-language = Language to download
ui-dictation-auto-stop-label = Stop by itself after a silence of
ui-dictation-silence = Silence duration
ui-dictation-phrases-note = Only spoken passages are transcribed, sentence by sentence, while you speak.
ui-dictation-download-now = Download now ({ $size })
ui-dictation-download-language = Download the language
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Template picker

ui-picker-new-slide = New slide
ui-picker-new-template = New template, based on…
ui-picker-set = Templates
ui-picker-set-mine = { $name } · mine
ui-picker-keys = ← → to browse · Enter to add · Esc to close
ui-picker-close = Close
ui-picker-previous = Previous template
ui-picker-next = Next template
ui-picker-list = Slide templates

## Chart

ui-chart-panel = Chart
ui-chart-kinds = Chart type
ui-chart-kind = { $kind ->
    [line] Lines
    [pie] Pie
    [scatter] Scatter
    [radar] Radar
    [waterfall] Waterfall
   *[bar] Bars
}
ui-chart-no-data = This chart has no data table.
ui-chart-options = Chart options
ui-chart-stack = Stacked
ui-chart-waterfall-hint = Tick Total for a total bar.
ui-chart-colors = Series colors
ui-chart-color-of = Color of { $name }
ui-chart-color-auto = Automatic
ui-chart-color-pick = Color { $n }
ui-chart-column-name = Name of column { $col }
ui-chart-remove-column = Delete the column
ui-chart-cell = Row { $row }, column { $col }
ui-chart-remove-row = Delete the row
ui-chart-add-row = Add row
ui-chart-add-column = Add column
ui-chart-series = Series { $n }

## Files

ui-file-open-prompt = Path of the file to open (.oku-slides.pdf or .json)
ui-file-save-prompt = Save as (full path)

## Tabs

ui-tabs = Open presentations
ui-tab-new = New tab
ui-tab-close = Close tab
ui-tab-unsaved = Unsaved changes
ui-tab-templates = Templates
ui-tab-move-new-window = Move to new window
ui-tab-move-to = Move to window { $n }
ui-tab-close-others = Close other tabs
ui-close-cancel = Cancel
ui-disk-replaced-question = { $file } was replaced on disk by another presentation.
ui-disk-replaced-overwrite = Overwrite
ui-disk-replaced-save-as = Save as…
ui-same-deck-question = { $file } is a version of the presentation open in the tab “{ $title }”.
ui-same-deck-merge = Merge into that tab
ui-same-deck-apart = Open separately
ui-history = History
ui-history-version = Version of { $date }
ui-history-restore = Restore
ui-history-back = Back
ui-history-anonymous = Anonymous
ui-history-slider = Versions of the presentation
ui-history-removed-since = deleted since
ui-history-changed-since = changed since
ui-history-added-since = slides added since: { $count }
ui-help = Help
ui-help-caption = Slide { $index } of { $count } · read-only
ui-slide-previous = Previous slide
ui-slide-next = Next slide
ui-toolbar-params-hide = Hide settings
ui-toolbar-params-show = Show settings
ui-help-close = Close the help
ui-help-search = Search the help
ui-help-search-placeholder = Search…
ui-help-search-empty = No results
ui-help-search-count = { $count ->
    [one] 1 result
   *[other] { $count } results
}
ui-pack-notice-close = Close
ui-widgets-files = Widget files
ui-widgets-common = Shared
ui-widgets-new = New widget
ui-widgets-name = Widget name (lowercase letters, digits and hyphens)
ui-widgets-name-taken = This name is already taken
ui-widgets-name-letter = The name must start with a letter
ui-widgets-valid = Valid
ui-widgets-errors = Errors
ui-widgets-empty = This pack has no widget yet.
ui-widgets-preview = Widget preview
ui-widgets-no-preview = No preview
ui-widgets-field-bound = Linked to the presentation: read-only here
ui-widgets-source = Source code
ui-widgets-unsaved = Not saved
ui-widgets-in-error = Widgets with errors
ui-widgets-none-in-error = No widget has errors
ui-widgets-section = Widgets
ui-widgets-open = Open widget { $name }
ui-widgets-delete-named = Delete { $name }
ui-widgets-confirm-delete = Delete the widget “{ $name }”?
ui-widgets-rename-hint = Double-click or F2 to rename
ui-widgets-tabs = Template tabs
ui-widgets-templates-tab = Templates
ui-theme-tab = Theme
ui-theme-picker = Theme
ui-theme-name = Theme name
ui-theme-unsaved = Unsaved theme changes
ui-widgets-close-tab = Close { $name }
ui-widgets-removed-by-agent = The agent deleted widget { $name }.
