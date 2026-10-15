op-slide-not-found = Slide not found: { $subject }
op-element-not-found = Element not found: { $subject }
op-duplicate-id = Identifier already in use: { $subject }
op-index-out-of-range = Position out of range
op-invalid-value = Invalid value for { $subject }
op-invalid-value-because = Invalid value for { $subject }: { $reason }
op-value-no-value = the parameter has no value
op-value-no-dataset = the chart has no dataset
op-value-grid-tracks = a grid has 1 to 24 columns and rows
op-value-bad-item-id = an item id is a non-empty text without “/”
op-value-missing-item-id = every item needs an id
op-value-too-deep = elements nest too deeply
op-value-empty-path = the path is empty
op-value-not-text = a text or null is expected
op-value-empty-frame = the width and height must be greater than 0
op-value-blocks-off-grid = flow slots are laid out by the slide and a grid hides them; move each element to a free slot first
op-value-outside-grid = the cell is outside the grid
op-value-not-tokens = an object or null is expected
op-value-not-a-slot = this parameter is not a slot
op-value-out-of-bounds = the number is outside its allowed range
op-value-unreadable = the value does not have the expected form
op-value-valign = top, middle or bottom is expected
op-value-path = the path is made of M, L, C, Q and Z commands followed by their numbers, starting with M
op-unknown-widget = Unknown widget: { $subject }
op-unknown-param = Unknown parameter: { $subject }
op-param-type-because = Invalid value for { $subject }: { $reason }
op-param-shape-text = { $nullable ->
    [yes] a string or null is expected
   *[no] a string is expected
}
op-param-shape-choice = { $nullable ->
    [yes] one of its values or null is expected
   *[no] one of its values is expected
}
op-param-shape-paragraphs = { $nullable ->
    [yes] a list of paragraphs or null is expected
   *[no] a list of paragraphs is expected
}
op-param-shape-elements = { $nullable ->
    [yes] a list of elements or null is expected
   *[no] a list of elements is expected
}
op-param-shape-flag = { $nullable ->
    [yes] a boolean or null is expected
   *[no] a boolean is expected
}
op-param-shape-number = { $nullable ->
    [yes] a number or null is expected
   *[no] a number is expected
}
op-param-shape-bounded-number = { $nullable ->
    [yes] a number within its bounds or null is expected
   *[no] a number within its bounds is expected
}
op-param-shape-grid = { $nullable ->
    [yes] a grid or null is expected
   *[no] a grid is expected
}
op-param-shape-chart-option = { $nullable ->
    [yes] a chart option or null is expected
   *[no] a chart option is expected
}
op-param-shape-table-option = { $nullable ->
    [yes] a table option or null is expected
   *[no] a table option is expected
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] a chart or table option or null is expected
   *[no] a chart or table option is expected
}
op-param-shape-items = { $nullable ->
    [yes] a list of items or null is expected
   *[no] a list of items is expected
}
op-param-shape-list = { $nullable ->
    [yes] a list or null is expected
   *[no] a list is expected
}
op-param-shape-add-element-only = { $nullable ->
    [yes] elements are added through add_element, or null
   *[no] elements are added through add_element
}
op-param-shape-set-param-only = { $nullable ->
    [yes] a value is set through set_param, or null
   *[no] a value is set through set_param
}
op-param-shape-path-option = { $nullable ->
    [yes] a shape path (an object with d) or null is expected
   *[no] a shape path (an object with d) is expected
}
op-item-not-found = No item { $subject }
op-list-full = The list { $subject } is full ({ $max ->
   [one] { $max } item
  *[other] { $max } items
} at most)
op-list-minimum = The list { $subject } needs at least { $min ->
   [one] { $min } item
  *[other] { $min } items
}
op-list-cell-too-long = A cell of the list { $subject } is too long (column { $column })
op-list-id-too-long = An item id of the list { $subject } is too long
op-list-too-many-items = The list { $subject } has too many items ({ $max } at most)
op-slot-rejects = { $subject } is not accepted here
op-wrong-category = { $subject } cannot be used here
op-choice-out-of-range = Value not allowed for { $subject }
transaction-refused = Operation { $index } refused: { $reason }

sources-unreadable = Unreadable sources
sources-not-sources = This is not an Oku Slides sources archive
sources-unsupported-version = These sources were written by a newer version of Oku Slides
sources-bad-name = Rejected file name in the sources
sources-missing-deck = The deck is missing from the sources
sources-too-large = The sources in this file are too large to open
sources-too-deep = This deck nests elements too deeply

issue-empty-slide-id = empty slide identifier
issue-duplicate-slide-id = duplicate slide identifier
issue-duplicate-element-id = duplicate element identifier
issue-empty-element-id = empty identifier
issue-cell-outside-grid = cell outside the grid
issue-empty-frame = zero-size frame
issue-free-without-frame = free element without a frame
issue-unknown-widget = unknown widget: { $kind }
issue-wrong-category = { $kind } cannot be used here
issue-unknown-param = unknown parameter: { $param }
issue-param-type = invalid value for { $param }
issue-choice-out-of-range = value not allowed for { $param }
issue-slot-rejects = { $kind } is not accepted here
issue-placed-in-flow = element placed with a frame in a laid-out list
issue-unknown-bind = { $param } is linked to an unknown deck setting
issue-slot-too-deep = elements nest too deeply
issue-invalid-style = invalid style value: { $field }
issue-missing-asset = missing asset: { $asset }
issue-pack-version-mismatch = altered pack version discarded: { $version }
issue-pack-version-missing = pack version missing from the sources: { $version }
issue-theme-version-mismatch = altered theme version discarded: { $version }
issue-theme-version-missing = theme version missing from the sources: { $version }
issue-unsupported-link = unsupported link: { $link }
issue-highlight-zero = highlight colors are numbered from 1
issue-unreadable-element = unreadable element, set aside
issue-slot-repaired = element without a known slot, placed in { $slot }
issue-unreadable-item = unreadable list item, set aside
issue-unreadable-row = unreadable table row, set aside
issue-data-truncated = data too large, truncated on read
issue-unreadable-slide = unreadable slide, set aside
issue-unreadable-field = unreadable deck field { $field }, reset
op-invalid-chart = Chart { $subject }: { $reason }
op-invalid-table = Table { $subject }: { $reason }

replica-corrupted = The presentation history in this file is damaged
replica-too-large = The presentation history in this file is too large to open
replica-write = The presentation history could not be updated
replica-unrelated = These two presentations do not share a common origin
replica-unsupported-version = This presentation uses a deck format this version of Oku Slides cannot read
replica-too-deep = This presentation cannot be saved: removed elements are nested too deeply in its history. Undo your last moves, then save again.

widget-unreadable = The widget declarations could not be read
widget-fault-empty-name = Widget { $widget }: a widget or parameter has no name
widget-fault-duplicate-widget = Widget { $widget } is declared twice
widget-fault-duplicate-param = Widget { $widget } declares the same parameter twice
widget-fault-empty-choice = Widget { $widget }: a choice has no values
widget-fault-default-not-in-choice = Widget { $widget }: a choice default is not one of its values
widget-fault-default-wrong-type = Widget { $widget }: a default value does not match its parameter type
widget-fault-bind-not-plain = Widget { $widget }: only plain text parameters can be bound to the deck
widget-fault-unknown-slot-kind = Widget { $widget }: a slot accepts an unknown element widget
widget-fault-empty-list = Widget { $widget }: a list needs at least one column
widget-fault-column-type = Widget { $widget }: a list column can be plain, rich, number, flag, choice or asset; for free content per item, use p.slot with a child widget
widget-fault-reserved-name = Widget { $widget }: reserved name: id, kind, or a name containing /
widget-fault-list-bounds = Widget { $widget }: a list max is below its min
widget-fault-list-default-too-large = Widget { $widget }: a list default exceeds the list size limits
widget-incompatible-override = Widget { $widget } replaces the built-in one but changes or drops its parameter { $param }
widget-incompatible-category = Widget { $widget } replaces the built-in one with a different category
issue-empty-item-id = list { $param } has an item without identifier
issue-duplicate-item-id = list { $param } has a duplicate item identifier
issue-list-bounds = list { $param } has too few or too many items
