# crates/oku-slides-import — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Keep oku-slides-import free of any dependency on oku-slides-session, oku-slides-render or the CLI. (plan:2026-10-10-import-pptx-lot2)
- Keep the retrofit projector deterministic: no randomness and no clock, ids are s plus slide number and slide-rank. (plan:2026-10-10-import-pptx-lot2)
- Emit a text color token only if it reaches WCAG contrast 3:1 on its actual background, otherwise emit none. (plan:2026-10-10-import-pptx-lot2)
- Use Decor::skips as the single predicate for omitting decor items in both projection and coverage. (plan:2026-10-10-import-pptx-lot2)
- Never put a shape without text into a flow slot; place it as a free element. (plan:2026-10-10-import-pptx-lot2)
- Never copy a template's example text into imported slides; reuse only its layout or grid. (plan:2026-10-10-import-pptx-lot2)
- Keep imported links only when they start with http, https or mailto, ignoring case. (plan:2026-10-10-import-pptx-lot2)
- Do no I/O inside oku-slides-import; it receives in-memory bytes only. (plan:2026-10-09-import-pptx-lot1)
- Cut groups and AlternateContent at MAX_NESTING, SmartArt node trees at MAX_NODES and MAX_NODE_DEPTH, and never follow a SmartArt inside a SmartArt drawing. (plan:2026-10-09-import-pptx-lot1, t-722)
- Use attr_r only for relationship-namespace attributes; the other attr_ helpers read only attributes without namespace. (plan:2026-10-09-import-pptx-lot1)
- Propagate TooLarge from any part, linked parts included, as fatal via cx.fail, never as a warning; count decompressed bytes even when a read fails, and after a first TooLarge fail every read without decompressing. (spec:2026-10-09-import-pptx-design, plan:2026-10-09-import-pptx-lot1)
- Choose the AlternateContent branch before reading and never parse the discarded one, so it consumes no id or asset. (plan:2026-10-09-import-pptx-lot1)
- Pass every string taken from the file through collect::clip before putting it in a warning or error. (plan:2026-10-09-import-pptx-lot1)
- Store Frame in absolute source points with group transforms applied, always finite and bounded. (plan:2026-10-09-import-pptx-lot1)
- Keep fs_type and subset on embedded fonts so later lots never embed preview-only or partial fonts. (plan:2026-10-09-import-pptx-lot1)
- Resolve PPTX text style inheritance only in the single cascade function, never ad hoc elsewhere. (spec:2026-10-09-import-pptx-design)
- Clamp numeric text and fill values to their OOXML schema range and custGeom values to MAX_PATH_SPAN, listed in bounds::clamped to warn Approximated; parse a theme part shared by masters once. (t-755, t-892)
- Type every asset with media::sniff, refuse a raster whose header is unreadable or above max_image_pixels, and pass every SVG through svg::sanitize (literal internal entities only). (t-752, t-892, t-730)
- Apply or write only a CheckedPlan, which only ImportPlan::check_budget returns. (t-753, t-892)

## Pitfalls

- Emit only color names that exist in the target theme; Typst rendering panics on an unknown color name. (plan:2026-10-10-import-pptx-lot2)
- Emit no color in chart options; chart color fields accept only hex and the target theme palette decides. (plan:2026-10-10-import-pptx-lot2)
- Emit chart options already in canonical form, because the core checks host params but does not canonicalize. (plan:2026-10-10-import-pptx-lot2)
- Give every emitted frame sides of at least 1 pt, since the core rejects zero width or height. (plan:2026-10-10-import-pptx-lot2)
- Emit images only in formats Typst draws (png, jpeg, gif, webp, svg); report UnsupportedImage for any other. (plan:2026-10-10-import-pptx-lot2)
- Keep retrofit snap in sync with snapRect in ui/src/grid.ts; the Rust tests are its only oracle. (plan:2026-10-10-import-pptx-lot2)
- Reject XML deeper than MAX_XML_DEPTH or above MAX_XML_ATTRIBUTES per element as Unreadable before roxmltree. (spec:2026-10-09-import-pptx-design, t-754)
- Identify SmartArt shapes by their modelId, because PowerPoint writes cNvPr id 0 on all of them. (plan:2026-10-09-import-pptx-lot1)
- Apply a table part's outer borders at its own region edges; read merged cells' right, bottom, lastCol and lastRow at the merge end. (plan:2026-10-09-import-pptx-lot1, t-725)
- Bound the sum of per-face font copies by max_total_bytes, since many faces can share one large part. (plan:2026-10-09-import-pptx-lot1)

## Testing

- In end-to-end import tests, apply plan ops with apply_all and assert validate reports no Issue. (plan:2026-10-10-import-pptx-lot2)
- Never change expected values of color_modifiers_match_powerpoint; a gap above 1 means a conversion bug. (plan:2026-10-09-import-pptx-lot1)
- When a reviewed import snapshot is wrong, fix the code, never the snapshot. (plan:2026-10-09-import-pptx-lot1)

## Settled decisions

- Map source colors to target theme tokens or drop them; never emit raw hex colors or token overrides. (spec:2026-10-09-import-pptx-design)
- Fail only on unreadable package, non-presentation, protection, no master or limit; report every other loss. (spec:2026-10-09-import-pptx-design)
- Do not flat re-export retrofit from the import lib.rs; re-export only ImportPlan, ImportReport and RetrofitTarget. (plan:2026-10-10-import-pptx-lot2)
- Derive a template profile from its example slide and widget schema, never from template names. (plan:2026-10-10-import-pptx-lot2)
- Break template score ties by pack order so the template choice stays deterministic. (plan:2026-10-10-import-pptx-lot2)
- Only the first source title slide fills deck title and subtitle; later title slides become chapter slides. (plan:2026-10-10-import-pptx-lot2)
- Report a group fallback, and thus a WidgetRequest, only when the fallback actually loses something. (plan:2026-10-10-import-pptx-lot2)
- Read colors, fills and lines unresolved and bounded; resolve them with the color context of the displaying part. (plan:2026-10-09-import-pptx-lot1)
- Read multi-level chart categories from their leaf level and warn Approximated for the outer levels. (t-755)
- Do not mark the source representation non_exhaustive; projectors match exhaustively, only ImportErrorKind is. (plan:2026-10-09-import-pptx-lot1)
- Keep a conservative Default on ReadLimits so adding a limit later does not break callers. (plan:2026-10-09-import-pptx-lot1)
- In retrofit, never modify the target pack; put agent-created widgets in a child pack, one widget per pattern. (spec:2026-10-09-import-pptx-design)
- Always apply a deterministic retrofit fallback that never blocks and leaves the deck complete and renderable. (spec:2026-10-09-import-pptx-design)
