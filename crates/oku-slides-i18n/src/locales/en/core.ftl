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
knowledge-path-empty = the path is empty
knowledge-path-not-absolute = the path must start with /
knowledge-path-bad-segment = a segment is empty, ends with a dot or uses a character other than a-z, 0-9, dot, underscore and hyphen
knowledge-path-dot-segment = a segment must not start with a dot
knowledge-path-reserved = a segment is a reserved system name
knowledge-path-too-deep = the path has too many segments
knowledge-path-too-long = the path is too long
knowledge-path-not-markdown = the file name must end in .md
knowledge-frontmatter-missing = the file must start with a --- frontmatter block
knowledge-frontmatter-unterminated = the frontmatter block has no closing ---
knowledge-frontmatter-too-large = the frontmatter block is too large
knowledge-frontmatter-not-yaml = the frontmatter is not valid YAML
knowledge-frontmatter-not-mapping = the frontmatter must be a mapping with text keys
knowledge-frontmatter-duplicate-key = the frontmatter key "{ $key }" appears twice
knowledge-frontmatter-alias = the frontmatter must not use YAML anchors or aliases
knowledge-frontmatter-tag = the frontmatter must not use YAML tags
knowledge-frontmatter-too-deep = the frontmatter is nested too deeply
knowledge-frontmatter-type-missing = the frontmatter needs a non-empty type
knowledge-frontmatter-bad-timestamp = the frontmatter value "{ $key }" must be an RFC 3339 date-time with an offset
knowledge-frontmatter-not-allowed = the frontmatter key "{ $key }" is not allowed in this file
knowledge-reason-not-found = no file at this path
knowledge-reason-exists = a file already exists at this path
knowledge-reason-id-missing = a new file needs an id
knowledge-reason-id-invalid = a file id is 32 lowercase hexadecimal digits
knowledge-reason-id-taken = another file already has this id
knowledge-reason-id-mismatch = the file at this path has another id
knowledge-reason-edit-empty-old = the text to replace must not be empty
knowledge-reason-edit-no-match = the text to replace is not in the file
knowledge-reason-edit-ambiguous = the text to replace appears in the file more than once
knowledge-reason-too-many-files = the bundle has too many files
knowledge-reason-file-too-large = the file is too large
knowledge-reason-total-too-large = the bundle is too large
op-knowledge = Knowledge file { $path }: { $reason }
op-unknown-widget = Unknown widget: { $subject }
op-unknown-namespace = This deck has no pack { $pack }.
op-pack-required = The pack { $pack } is needed by { $by }; remove that one first.
op-pack-in-use = The pack { $pack } is still used by { $count ->
    [one] { $count } item
   *[other] { $count } items
}.
op-pack-requirement-missing = The pack { $pack } needs { $required }, which this deck does not have.
op-base-required = Every deck keeps the base widget pack.
op-pack-kind-conflict = This deck already has a pack { $pack } of the other kind (widgets or layouts).
op-theme-cycle = Themes extend each other in a loop: { $chain }.
op-theme-closure = The themes pinned by this deck are not exactly its theme and its parents ({ $namespace }).
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
issue-invalid-style = invalid style value: { $field } ({ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    })
issue-missing-asset = missing asset: { $asset }
issue-pack-version-mismatch = altered pack version discarded: { $version }
issue-pack-version-missing = pack version missing from the sources: { $version }
issue-pack-pin-mismatch = The embedded pack { $pack } does not match what the deck expects.
issue-pack-tokens-invalid = The embedded pack { $pack } has an invalid component token: { $reason }
issue-pack-kind-mismatch = The embedded pack { $pack } is not of the kind the deck expects (widgets or layouts).
issue-pack-kind-conflict = The pack namespace { $pack } is pinned both as a widget pack and as a layout pack: only one can be used.
issue-pack-requirement-missing = The pack { $pack } needs { $required }, which this deck does not have.
issue-theme-version-mismatch = altered theme version discarded: { $version }
issue-theme-version-missing = theme version missing from the sources: { $version }
issue-theme-pin-mismatch = The embedded theme { $namespace } does not match what the deck expects.
issue-theme-closure = The themes pinned by this deck are not exactly its theme and its parents ({ $namespace }).
issue-usage-line-invalid = usage ledger line { $line } unreadable, discarded
issue-usage-line-forged = usage turn { $turn } recorded twice with different content, one discarded
issue-usage-too-large = usage ledger too large, discarded
issue-usage-unreadable = usage ledger unreadable, discarded
issue-usage-lines-dropped = { $count ->
    [one] { $count } more usage ledger line discarded
   *[other] { $count } more usage ledger lines discarded
}
usage-range-bad-date = “{ $date }” is not a date: use YYYY-MM-DD.
usage-range-reversed = The start date { $since } is after the end date { $until }.
pack-token-entry-invalid = Invalid component token “{ $path }” in the pack manifest.
pack-manifest-json = The pack manifest cannot be read: { $reason }
pack-manifest-order = The pack's item order is invalid at “{ $name }”.
issue-unsupported-link = unsupported link: { $link }
issue-highlight-zero = highlight colors are numbered from 1
issue-unreadable-element = unreadable element, set aside
issue-slot-repaired = element without a known slot, placed in { $slot }
issue-unreadable-item = unreadable list item, set aside
issue-unreadable-row = unreadable table row, set aside
issue-data-truncated = data too large, truncated on read
issue-unreadable-slide = unreadable slide, set aside
issue-unreadable-field = unreadable deck field { $field }, reset
issue-knowledge-unreadable = unreadable knowledge entry { $id }, set aside
issue-knowledge-over-limit = knowledge bundle past a limit: { $limit ->
        [knowledge-reason-too-many-files] { knowledge-reason-too-many-files }
        [knowledge-reason-file-too-large] { knowledge-reason-file-too-large }
       *[knowledge-reason-total-too-large] { knowledge-reason-total-too-large }
    } (files past it: { $count }, first { $id }); every file is kept, but writes past the limit are refused
issue-knowledge-path-invalid = knowledge file { $id } moved to /recovered: { $reason ->
        [knowledge-path-empty] { knowledge-path-empty }
        [knowledge-path-not-absolute] { knowledge-path-not-absolute }
        [knowledge-path-bad-segment] { knowledge-path-bad-segment }
        [knowledge-path-dot-segment] { knowledge-path-dot-segment }
        [knowledge-path-reserved] { knowledge-path-reserved }
        [knowledge-path-too-deep] { knowledge-path-too-deep }
        [knowledge-path-too-long] { knowledge-path-too-long }
       *[knowledge-path-not-markdown] { knowledge-path-not-markdown }
    }
issue-knowledge-path-collision = several knowledge files were at { $path }: the others were renamed
issue-knowledge-invalid = knowledge file { $path } is invalid: { $reason ->
        [knowledge-frontmatter-missing] { knowledge-frontmatter-missing }
        [knowledge-frontmatter-unterminated] { knowledge-frontmatter-unterminated }
        [knowledge-frontmatter-too-large] { knowledge-frontmatter-too-large }
        [knowledge-frontmatter-not-yaml] { knowledge-frontmatter-not-yaml }
        [knowledge-frontmatter-not-mapping] { knowledge-frontmatter-not-mapping }
        [knowledge-frontmatter-duplicate-key] { knowledge-frontmatter-duplicate-key }
        [knowledge-frontmatter-alias] { knowledge-frontmatter-alias }
        [knowledge-frontmatter-tag] { knowledge-frontmatter-tag }
        [knowledge-frontmatter-too-deep] { knowledge-frontmatter-too-deep }
        [knowledge-frontmatter-type-missing] { knowledge-frontmatter-type-missing }
        [knowledge-frontmatter-bad-timestamp] { knowledge-frontmatter-bad-timestamp }
       *[knowledge-frontmatter-not-allowed] { knowledge-frontmatter-not-allowed }
    }
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
widget-fault-column-type = Widget { $widget }: a list column can be plain, rich, number, flag, choice or asset (or a nested p.list, one level); for free content per item, use p.slot with a child widget
widget-fault-reserved-name = Widget { $widget }: reserved name: id, kind, or a name containing /
widget-fault-list-bounds = Widget { $widget }: a list max is below its min
widget-fault-list-default-too-large = Widget { $widget }: a list default exceeds the list size limits
widget-fault-list-depth = Widget { $widget }: a nested list cannot itself contain a list column; lists nest one level only
widget-fault-list-default-below-min = Widget { $widget }: a nested list default has fewer items than its minimum
widget-fault-slide-sizing = Widget { $widget } is a slide: only element widgets declare grow or typography
widget-fault-pack-category = Widget { $widget } does not match its pack: layout packs hold slide widgets, widget packs hold element widgets
widget-fault-unqualified = Widget { $widget } has no pack namespace
widget-fault-too-long = Widget { $widget }: a name, label, value, description or list exceeds its limit
issue-empty-item-id = list { $param } has an item without identifier
issue-duplicate-item-id = list { $param } has a duplicate item identifier
issue-invalid-item-id = list { $param } has an item identifier containing a slash
issue-list-bounds = list { $param } has too few or too many items
