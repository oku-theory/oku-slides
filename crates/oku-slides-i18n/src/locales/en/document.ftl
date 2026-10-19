pdf-unreadable = Unreadable PDF
pdf-unsupported-version = This PDF was written by a newer version of Oku Slides
pdf-corrupted = The sources in this PDF are damaged
pdf-write = The PDF could not be written

render-compile = The deck could not be rendered
render-pdf = The PDF export failed
render-no-page = This page does not exist
render-setup = The renderer could not start
render-image = The image of the slide could not be produced
render-missing-image = missing image
diagnostic-error = error
diagnostic-warning = warning
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = hint: { $hint }
render-chart-failed = chart could not be drawn

registry-bad-pack-id = Invalid template pack identifier: { $id }
registry-base-pack-broken = The built-in widget pack is damaged ({ $file }); reinstall the app.
registry-duplicate-pack = Two widget packs of the presentation share { $pack }.
registry-compile = The widgets of the template pack could not be compiled
registry-missing = The template pack does not declare its widgets
render-timeout = Rendering took more than { $seconds } s and was stopped
render-out-of-memory = Rendering needed more than { $mib } MiB of memory and was stopped
render-worker-lost = The rendering process stopped
render-frame = Unreadable message from the rendering process
render-page-too-large = This slide is too large to preview (more than { $mib } MiB, often because of a very large image). It can still be exported as PDF or PNG.
render-stale = The render was replaced; try again
render-pack-disabled = The widgets of pack “{ $pack }” are disabled for this session
render-protocol = The rendering process belongs to another version of Oku Slides; restart the application
render-remote = The rendering process could not complete the request
registry-schema-remote = Invalid widget registry: { $message }
probe-not-probed = Not checked yet: the time allowed for the pack's widgets ran out. It will be checked at the next render.
probe-too-many-files = The pack has more than { $max } widget files: this one is not loaded.
probe-too-large = Larger than { $max } KiB: not loaded.

import-unreadable = This presentation could not be read
import-not-a-presentation = This file is not a PowerPoint presentation
import-protected = This presentation is encrypted or in the old .ppt format: save it as .pptx first
import-too-large = This presentation is too large to import
import-output-too-large = The resulting deck would exceed the size allowed for an import
import-loss-list-levels = List levels flattened to one level
import-loss-merged-cells = Merged cells flattened
import-loss-percent-stacked = 100% stacked chart shown as stacked
import-loss-chart-format = Chart number format { $format } not kept
import-loss-run-style = { $effect ->
    [underline] Underline lost
    [strike] Strikethrough lost
    [baseline] Superscript or subscript lost
    [caps] Capitals lost
    [font] Font of a passage replaced by the box font
    [tracking] Letter spacing of a passage replaced by the box spacing
    [weight] Regular passage made bold like the box
    [slant] Upright passage made italic like the box
   *[highlight] Highlight lost
}
import-loss-transform = Rotation, flip or crop lost
import-loss-color-dropped = Color { $color } has no close theme color: theme color used
import-loss-text-truncated = Text shortened to fit the limits
import-loss-table-truncated = Table shortened to fit the limits
import-loss-chart-truncated = Chart data shortened to fit the limits
import-loss-opaque = { $reason ->
    [media] Video or sound not imported (no fallback image)
    [ole] Embedded object not imported (no fallback image)
    [math] Equation not imported (no fallback image)
    [smartart] SmartArt not imported (no fallback image)
   *[unknown] Unsupported object not imported (no fallback image)
}
import-loss-smartart-layout = SmartArt without its drawing: its text is laid out as cards, its diagram layout is lost
import-loss-unsupported-image = Image in { $format } format not imported
import-loss-hidden-slide = Hidden slide imported as visible
import-loss-effects = Shadows, reflections and 3D effects lost
import-loss-series-dropped = A pie chart shows a single series: { $kept } kept, { $dropped } dropped
import-loss-repeated-decor = { $slides ->
   [one] Header or footer “{ $text }” repeated on { $slides } slide left out: the theme draws its own
  *[other] Header or footer “{ $text }” repeated on { $slides } slides left out: the theme draws its own
}
import-loss-no-slot = Content left out: the chosen template has no place for it
import-loss-moved-to-notes = Text moved to the speaker notes: the cover has no place for it
import-loss-items-dropped = Too many items on the slide: { $dropped } items left out
import-loss-page-fitted = Source page { $width } × { $height } pt is not 16:9: content scaled and centered
import-loss-paragraph-style = Paragraph alignment, line spacing, paragraph spacing or indent changed to the box's
import-loss-shape-style = { $effect ->
    [gradient] Gradient rendered as a solid color
    [picture] Fill picture stretched to the shape's frame
    [dash] Dashed line rendered solid
    [arrow] Line arrowheads not drawn
   *[geometry] Unknown shape rendered as a rectangle
}
import-loss-layout-dropped = Too many layouts for one pack: “{ $layout }” has no template; its slides use the blank widget and get its decoration as elements
import-loss-decor-truncated = { $omitted ->
    [one] Decoration cut to fit the size of a pack file: { $omitted } item left out
   *[other] Decoration cut to fit the size of a pack file: { $omitted } items left out
}
import-loss-table-style = Table style approximated: its own cell backgrounds, borders, column widths, row heights or text sizes are lost
import-loss-overflow = Element “{ $element }” runs past the edge of the slide
import-loss-empty-data = Table or chart without data: nothing placed
import-reference-unavailable = No visual score: { $cause ->
    [timeout] the reference render took too long
    [memory] the reference render used too much memory
    [pages] the reference render has { $reference ->
        [one] { $reference } page
       *[other] { $reference } pages
    } for { $deck } in the deck
    [missing] LibreOffice (soffice) was not found; set its path in import.soffice
   *[failed] the reference renderer could not read the presentation
}
import-region-suspect = Gap set aside, a known error of the reference renderer: { $limit ->
    [autofit] it shrinks text set to fit its box when PowerPoint saved no scale
    [tracking] it draws letter-spaced text wider
    [chart] it draws charts only roughly
    [default_font] it draws text whose font only the presentation defaults name in its fallback font
    [no_substitute] it has no substitute for this font and draws it in another
    [smart_art_style] it draws SmartArt text in its own style
   *[smart_art] it draws SmartArt text too small
}
import-font-substituted = { $family } replaced by { $by }, with matching metrics
import-font-missing = { $family } not found: the text uses { $by }
import-font-restricted = { $family }: its license forbids embedding some of its faces in the deck; text in those styles uses its other faces
import-font-restricted-substituted = { $family } cannot be embedded in the deck (license): replaced by { $by }, with matching metrics
import-font-restricted-missing = { $family } cannot be embedded in the deck (license): the text uses { $by }
import-font-too-large = { $family }: files too large to embed in the deck; the text uses { $by }
import-font-missing-no-fallback = { $family } not found, nor the theme's body font: the text is rendered with the engine's default font
import-font-too-large-no-fallback = { $family }: files too large to embed in the deck, and the theme's body font is not available either: the text is rendered with the engine's default font
import-font-restricted-no-fallback = { $family } cannot be embedded in the deck (license), and the theme's body font is not available either: the text is rendered with the engine's default font
import-warning-effect-ignored = Effect not imported: { $detail }
import-warning-approximated = Approximated: { $detail }
import-warning-unreadable-item = Unreadable item skipped: { $detail }
import-warning-unreadable-part = Unreadable part: { $detail }
import-warning-missing-part = Missing part: { $detail }
import-warning-unsupported-chart = Unsupported chart: { $detail }
import-warning-unsupported-image = Image format not supported: { $detail }
import-warning-oversized-image = Image too large to import (pixel count): { $detail }
import-warning-external-image = Linked image outside the file, not imported: { $detail }
import-warning-unsafe-svg = SVG image cleaned of unsafe content, or dropped: { $detail }
import-warning-unknown-table-style = Unknown table style: { $detail }
import-warning-unsafe-link = Link refused: { $detail }
import-warning-nesting-too-deep = Nesting too deep, cut off: { $detail }
import-warning-font-embedded-unreadable = Embedded font unreadable: { $detail }
import-warning-truncated = Too many warnings: the rest are omitted
