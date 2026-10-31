# Lot G: theme picker and add carousel

**Parent task:** t-1352. Tasks 1–7 are the tickets themselves (linked to t-1352); task 8 is
its sub-task. One squash commit on `main`.

| Task | Task id |
|---|---|
| 1 | t-1086 |
| 2 | t-1076 |
| 3 | t-1085 |
| 4 | t-1077 |
| 5 | t-962 |
| 6 | t-963 |
| 7 | t-964 |
| 8 | t-1354 |

## Task 1: t-1086, one stage preview for variants and themes

Already done by e8aa25dc (`stagePreview.ts` store used by `SlideThumb` and `ThemeChoice`,
`slidePreview.ts` gone, one `.stage-preview` rule). Comment and close the ticket; no code.

## Task 2: t-1076, theme thumbnails faithful to the whole deck

`theme_choices` renders the whole deck (not a one-slide clone) under each theme and takes the
page of the shown slide, as `render_variants` does: page number, outline and links match.
Test: a thumbnail of the second slide shows its page number "2".

## Task 3: t-1085, theme thumbnails rendered outside the session lock

`Session::theme_job(slide)` prepares, under the lock, an owned `ThemeJob` (deck, layers,
assets, the preview render slot taken from the session). `ThemeJob::run` renders with no lock
held and returns the choices plus the slot, handed back by `Session::restore_preview_slot`
(dropped if the session's renderers were reset meanwhile: preview epoch). `SurfaceHub::show_themes`
uses the three steps. Tests: the session stays usable while a job is out; a reset during the
job drops the returned slot.

## Task 4: t-1077, quick-key digits on theme cards

`ThemeChoice` shows its digit 1–4 when `QuickHintsContext` is true, computed with the same
numbering as `quickAction` (`quickDigit` exported from `agent/quick.ts`). Render tests.

## Task 5: t-962, cache pack widget previews in the add carousel

`AddPackView` gains `version` (the pin's pack version) and the snapshot gains `preview_set`
(the deck's `PackSet::fingerprint`: a preview renders with every pack of the deck, t-1057). The
carousel reads previews from an App-lifetime bounded cache keyed on pack version, deck look,
pack set, examples revision, language and widget name; only misses go to `preview_widget`.
Tests: a second opening sends no request; another version or render key does.

## Task 6: t-963, shortcut and last used widget

User choice (2026-10-31): the `A` key (no modifier, outside text editing, deck mode) opens the
carousel with the focus on the last widget added from it in this window; order stays fixed.
Help and shortcut docs updated. Tests: key opens; focus starts on the last used card.

## Task 7: t-964, localizable widget labels and descriptions

`widget(label:, description:)` accepts a string or a dictionary by locale
(`(en: "…", fr: "…")`). Core `WidgetSchema.label/description` become a `LocalizedText`
(plain or by locale) resolved with the user's locale (exact, language, `en`, first). Typst,
core, session (`kind_name`, `kind_description`) and README updated. Tests in Rust and Typst probe.

## Task 8: review and gates

Multi-angle review, fixes, `just ci`, `just e2e`, squash commit citing every ticket, push.
