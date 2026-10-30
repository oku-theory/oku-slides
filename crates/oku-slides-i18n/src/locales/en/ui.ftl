# Interface de l’app (React). Chargé par ui/src/i18n.ts via la requête « messages ».
ui-loading = Loading…

## Document

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · layout pack
   *[other] { $name } · widget pack
}
ui-title-rename = Rename the presentation
ui-title-field = Presentation title
ui-status-unsaved = unsaved changes
ui-status-saved = saved
ui-status-never-saved = not saved yet
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count } layout
       *[other] { $count } layouts
    }
   *[other] { $count ->
        [one] { $count } widget
       *[other] { $count } widgets
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } slide
   *[other] { $count } slides
} · { $status }
ui-confirm-quit-pack = Quit without saving the modified pack?
ui-confirm-quit-theme = Quit without saving the modified theme?
ui-confirm-quit-read-only-theme = { $theme } cannot be saved here. Close this tab and lose its changes?
ui-confirm-update-opener = Update “{ $deck }” to this version of the pack “{ $pack }”?
ui-close-writes-pending = This tab is still saving: close it again in a moment.
ui-confirm-close-writes-pending = This tab is still saving. Close it anyway and lose what has not been saved?
ui-confirm-close-writes-pending-linked = This tab is still saving, and these pack tabs close with it, unsaved: { $tabs }. Close them anyway and lose what has not been saved?
ui-confirm-quit-read-only-pack = { $pack } cannot be saved here. Close this tab and lose its changes? To keep them, ask the agent to save it as a new pack.

## Toasts

ui-toast-copied = Copy saved
ui-toast-field-gone = The field you were editing no longer exists: your input is lost.
ui-toast-edit-replaced = This text changed while you were editing it: your version replaced it. Undo restores the other one.

## Toolbar

ui-toolbar-undo = Undo
ui-toolbar-undo-label = Undo: { $label }
ui-toolbar-redo = Redo
ui-toolbar-redo-label = Redo: { $label }
ui-divider-pile = Width of the slide pile
ui-divider-notes = Width of the agent thread
ui-divider-code = Code size
ui-toolbar-theme = Theme
ui-toolbar-finish = Done
ui-pack-deck-only = Saved in this presentation only
ui-toolbar-open = Open
ui-menu = Menu
ui-menu-new-widget-pack = New widget pack
ui-menu-new-layout-pack = New layout pack
ui-menu-new-theme = New theme
ui-toolbar-add-label = Add to the slide
ui-toolbar-packs = Deck packs
ui-toolbar-packs-update = Deck packs — updates available
ui-toolbar-history = Show the history
ui-toolbar-thumbs-show = Show thumbnails
ui-toolbar-thumbs-hide = Hide thumbnails
ui-toolbar-remove-slide = Delete the slide
ui-toolbar-remove-hint = Slide { $n } · undo with ⌘Z

## Undo labels

ui-undo-add-widget = Add “{ $name }”
ui-undo-new-slide = New slide: { $name }
ui-undo-move-slide = Move the slide
ui-undo-remove-slide = Delete the slide
ui-undo-remove = Delete
ui-undo-paste = Paste
ui-undo-place-on-grid = Place on the grid
ui-undo-move-on-grid = Move on the grid
ui-undo-resize-on-grid = Resize on the grid
ui-undo-change-anchor = Change the anchoring
ui-undo-move = Move
ui-undo-resize = Resize
ui-undo-edit-slide = Edit slide
ui-undo-rename-pack = Rename the pack
ui-undo-rename-deck = Rename the presentation

## Slide pile

ui-pile-slides = Slides
ui-pile-layouts = Layouts
ui-pile-add-slide = New slide
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
    [chart] Chart
    [table] Table
    [stack] Stack
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
ui-undo-list-move = Move { $item }
ui-agent-color-other = Other color
ui-agent-thread = Agent
ui-agent-new-conversation = New conversation
ui-agent-undo-turn = Undo this turn
ui-agent-detach = Open the agent in a separate window
ui-agent-attach = Bring the agent back to the main window
ui-agent-window-empty = No agent for this tab
ui-agent-kept-in-draft = Nowhere to write: text kept in the draft
ui-draft = Draft
ui-draft-send = Send to the agent
ui-draft-send-title = Send to the agent (⌘↩)
ui-draft-discard = Discard
ui-pasted-image = { $type } { $width }×{ $height }
ui-draft-images = Pasted images
ui-draft-remove-image = Remove this image
ui-draft-send-needs-text = Say what to do with it
ui-agent-state-idle = Agent
ui-agent-state-starting = Starting the agent…
ui-agent-state-working = The agent is working…
ui-agent-dock = In progress
ui-agent-dock-settings = Settings
ui-agent-jump = ↓ Current turn
ui-agent-tool-input = Arguments
ui-agent-tool-output = Result
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
ui-dictation-downloading = Downloading the { $name } model
ui-dictation-download-progress = { $percent }% · { $received } of { $total } · only once
ui-dictation-listening = Listening · { $time }
ui-dictation-transcribing = Transcribing the last sentence…
ui-dictation-stop-in = Stopping in { $duration }
ui-dictation-speak = Speak, the text appears here…
ui-dictation-auto-stop = stops after { $duration } of silence
ui-dictation-to-download = { $size } to download
ui-dictation-settings = Settings
ui-dictation-language-auto = Automatic detection
ui-dictation-model-unavailable = Language not supported
ui-dictation-language-auto-resolved = Automatic ({ $language })
ui-dictation-to-download-language = Language to download
ui-dictation-api-key-missing = { $var } not set
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Layout picker

ui-picker-new-slide = New slide
ui-picker-set-mine = { $name } · mine
ui-picker-set-received = { $name } · received
ui-picker-keys = ← → to browse · Enter to add · Esc to close
ui-picker-close = Close
ui-picker-list = Layouts
ui-widget-picker-filter = Filter by name, label or description
ui-widget-picker-empty = No widget matches “{ $query }”.
ui-widget-picker-list = Widgets
ui-widget-picker-keys = Arrows to browse · Enter to add · Esc to close

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

## Table

ui-table-panel = Table
ui-table-options = Table options
ui-table-header-row = Header row
ui-table-header-column = First column as header
ui-table-striped = Striped rows
ui-table-alignment = Column alignment
ui-table-align-of = Alignment of { $name }
ui-table-align = { $align ->
    [start] Start
    [center] Center
    [end] End
   *[auto] Automatic
}
ui-table-column = Column { $n }
ui-table-no-data = This table has no data.
ui-show-as = Show as
ui-show-as-table = Show as a table
ui-show-as-chart = Show as a chart

## Files

ui-file-open-prompt = Path of the file to open (.oku-slides.pdf, .json or .pptx)
ui-file-filter-presentations = Presentations
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = Save as (full path)

## Tabs

ui-tabs = Open presentations
ui-tab-new = New tab
ui-tab-close = Close tab
ui-tab-unsaved = Unsaved changes
ui-tab-kind-pack = { $title }, pack
ui-tab-kind-theme = { $title }, theme
ui-tab-theme = Theme
ui-preview-theme = Preview theme
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
ui-present = Presentation
ui-present-slide = Slide { $index } of { $count }
ui-present-screen = Present full screen
ui-help-close = Close the help
ui-help-search = Search the help
ui-help-search-placeholder = Search…
ui-help-search-empty = No results
ui-help-search-count = { $count ->
    [one] 1 result
   *[other] { $count } results
}
ui-widgets-common = Shared
ui-widgets-new = New widget
ui-widgets-name = Widget name (lowercase letters, digits and hyphens)
ui-layouts-name = Layout name (lowercase letters, digits and hyphens)
ui-widgets-name-taken = This name is already taken
ui-widgets-name-letter = The name must start with a letter
ui-widgets-empty = This pack has no widget yet.
ui-layouts-new = New layout
ui-layouts-empty = This pack has no layout yet.
ui-confirm-delete-ok = Delete
ui-widgets-preview = Widget preview
ui-widgets-no-preview = No preview
ui-widgets-field-bound = Linked to the presentation: read-only here
ui-widgets-source = Source code
ui-widgets-unsaved = Not saved
ui-widgets-in-error = Widgets with errors
ui-widgets-none-in-error = No widget has errors
ui-widgets-open = Open widget { $name }
ui-widgets-delete-named = Delete { $name }
ui-widgets-confirm-delete = Delete the widget “{ $name }”?
ui-layouts-confirm-delete = Delete the layout “{ $name }”?
ui-widgets-rename-hint = Double-click or F2 to rename
ui-theme-name = Theme name
ui-theme-unsaved = Unsaved theme changes
ui-widgets-removed-by-agent = The agent deleted widget { $name }.
ui-layouts-removed-by-agent = The agent deleted layout { $name }.

## Import

ui-import-title = Import a PowerPoint presentation
ui-import-mode = Import mode
ui-import-hifi = Faithful copy
ui-import-hifi-note = Keeps the original look, with a theme and layouts made for this presentation.
ui-import-retrofit = Use my layouts
ui-import-retrofit-note = Rebuilds the slides with layout packs and a theme from the library.
ui-import-layout-packs = Layout packs
ui-import-theme-default = Default theme
ui-import-no-packs = No layout pack in the library.
ui-import-tune = Fine-tune with the agent
ui-import-tune-note = The agent compares each slide with the original and corrects the gaps.
ui-import-tune-note-retrofit = The agent compares each slide with the original and designs widgets for the shapes your layouts cannot show.
ui-import-tune-unavailable = No usable agent: { $reason }
ui-import-submit = Import
ui-import-busy = Importing…
ui-update-available = Oku Slides { $version } is available.
ui-update-install = Install and Restart
ui-update-see-release = See the Release
ui-update-later = Later
ui-update-installing = Downloading the update… { $percent }%
ui-update-downloading = Downloading the update…
ui-update-up-to-date = Oku Slides is up to date.
ui-update-check = Check for Updates…
ui-about = About Oku Slides
ui-about-version = Version { $version }
ui-about-changelog = What's new
ui-about-breaking = breaking
ui-about-close = Close

# Words that, alone in an agent prompt, open the theme picker; comma-separated, the English ones included.
ui-shortcut-theme = theme, themes
