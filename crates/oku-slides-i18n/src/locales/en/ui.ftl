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
ui-status-builtin = built-in: saving it creates a copy
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

ui-toast-saved = Your presentation is saved
ui-toast-templates-saved = Templates saved: { $name }
ui-toast-select-zone = Select an area to place: “{ $text }”

## Toolbar

ui-toolbar-undo = Undo
ui-toolbar-undo-label = Undo: { $label }
ui-toolbar-redo = Redo
ui-toolbar-redo-label = Redo: { $label }
ui-toolbar-grid-hide = Hide the grid
ui-toolbar-grid-show = Show the grid
ui-toolbar-grid-hide-title = Hide the grid (⌘')
ui-toolbar-grid-show-title = Show the grid (⌘')
ui-toolbar-theme = Theme
ui-toolbar-theme-default = Default
ui-toolbar-save-copy = Save a copy
ui-toolbar-save = Save
ui-toolbar-finish = Done
ui-toolbar-templates = Templates
ui-toolbar-templates-title = Create and edit slide templates
ui-toolbar-open = Open
ui-toolbar-add-label = Add to the slide
ui-toolbar-add = Add
ui-add-kind = { $kind ->
    [card] Card
    [shape] Shape
    [chart] Chart
   *[text] Text
}
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
ui-undo-new-template = New template: { $name }
ui-undo-new-slide = New slide: { $name }
ui-undo-grid = Grid { $columns } × { $rows }
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
ui-undo-edit-text = Edit the text
ui-undo-dictation = Dictation
ui-undo-rename-set = Rename the set
ui-undo-rename-deck = Rename the presentation
ui-undo-theme = Change theme
ui-undo-rename-template = Rename the template
ui-undo-describe-template = Describe the template
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
ui-stage-edit-text = Edit the text
ui-stage-hint-field = Enter to confirm · Esc to cancel
ui-stage-hint-block = ⌘ Enter to confirm · one line per paragraph, “- ” for a bullet · ⌘B bold, ⌘I italic, ⌘K link
ui-field = { $kind ->
    [title] Title
    [subtitle] Subtitle
    [kicker] Kicker
    [text] Text
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

## Slide notes

ui-notes = Slide tools
ui-stepper-less = { $label }: one fewer
ui-stepper-more = { $label }: one more

## Dictation

ui-dictation = Dictation
ui-dictation-start = Dictate
ui-dictation-start-title = Dictate (⌘D)
ui-dictation-finish = Finish dictation
ui-dictation-finish-title = Finish (⌘D)
ui-dictation-cancel = Cancel
ui-dictation-preparing = Preparing dictation…
ui-dictation-heard-nothing = Nothing heard: speak a little closer to the microphone
ui-dictation-to-cursor = at the cursor
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
ui-dictation-auto-stop-label = Stop by itself after a silence of
ui-dictation-silence = Silence duration
ui-dictation-phrases-note = Only spoken passages are transcribed, sentence by sentence, while you speak.
ui-dictation-download-now = Download now ({ $size })
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
ui-chart-add-row = + Row
ui-chart-add-column = + Column
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
ui-close-unsaved = Save the changes to “{ $name }” before closing?
ui-close-save = Save
ui-close-discard = Don't save
ui-close-cancel = Cancel
