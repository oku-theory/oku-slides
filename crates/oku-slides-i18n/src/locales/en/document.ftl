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
registry-compile = The widgets of the template pack could not be compiled
registry-missing = The template pack does not declare its widgets
render-timeout = Rendering took more than { $seconds } s and was stopped
render-out-of-memory = Rendering needed more than { $mib } MiB of memory and was stopped
render-worker-lost = The rendering process stopped
render-frame = Unreadable message from the rendering process
render-page-too-large = This slide is too large to preview (more than { $mib } MiB, often because of a very large image). It can still be exported as PDF or PNG.
render-stale = The render was replaced; try again
render-pack-disabled = The widgets of pack “{ $pack }” are disabled for this session
registry-schema-remote = Invalid widget registry: { $message }
probe-not-probed = Not checked yet: the time allowed for the pack's widgets ran out. It will be checked at the next render.
probe-too-many-files = The pack has more than { $max } widget files: this one is not loaded.
probe-too-large = Larger than { $max } KiB: not loaded.

import-unreadable = This presentation could not be read
import-not-a-presentation = This file is not a PowerPoint presentation
import-protected = This presentation is encrypted or in the old .ppt format: save it as .pptx first
import-too-large = This presentation is too large to import
import-loss-list-levels = List levels flattened to one level
import-loss-merged-cells = Merged cells flattened
import-loss-percent-stacked = 100% stacked chart shown as stacked
import-loss-run-style = { $effect ->
    [underline] Underline lost
    [strike] Strikethrough lost
    [baseline] Superscript or subscript lost
    [caps] Capitals lost
   *[highlight] Highlight lost
}
import-loss-transform = Rotation, flip or crop lost
import-loss-color-dropped = Color { $color } has no close theme color: theme color used
import-loss-text-truncated = Text shortened to fit the limits
import-loss-table-truncated = Table shortened to fit the limits
import-loss-opaque = { $reason ->
    [media] Video or sound not imported (no fallback image)
    [ole] Embedded object not imported (no fallback image)
    [math] Equation not imported (no fallback image)
    [smartart] SmartArt not imported (no fallback image)
   *[unknown] Unsupported object not imported (no fallback image)
}
import-loss-unsupported-image = Image in { $format } format not imported
import-loss-hidden-slide = Hidden slide imported as visible
import-loss-effects = Shadows, reflections and 3D effects lost
import-loss-series-dropped = A pie chart shows a single series: { $kept } kept, { $dropped } dropped
import-loss-repeated-decor = { $slides ->
   [one] Header or footer “{ $text }” repeated on { $slides } slide left out: the theme draws its own
  *[other] Header or footer “{ $text }” repeated on { $slides } slides left out: the theme draws its own
}
import-loss-no-slot = Content left out: the chosen template has no place for it
import-warning-effect-ignored = Effect not imported: { $detail }
import-warning-approximated = Approximated: { $detail }
import-warning-unreadable-item = Unreadable item skipped: { $detail }
import-warning-unreadable-part = Unreadable part: { $detail }
import-warning-missing-part = Missing part: { $detail }
import-warning-unsupported-chart = Unsupported chart: { $detail }
import-warning-unsupported-image = Image format not supported: { $detail }
import-warning-unknown-table-style = Unknown table style: { $detail }
import-warning-unsafe-link = Link refused: { $detail }
import-warning-nesting-too-deep = Nesting too deep, cut off: { $detail }
import-warning-font-embedded-unreadable = Embedded font unreadable: { $detail }
import-warning-truncated = Too many warnings: the rest are omitted
