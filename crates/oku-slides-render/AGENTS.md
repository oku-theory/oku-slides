# crates/oku-slides-render — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Draw charts at their box size, never at a fixed size scaled by Typst, so text size stays constant. (t-544)
- Keep each widget file name equal to its declared widget name; _commun is shared code, never a registry widget. (spec:2026-10-02-creation-widgets-design)
- Compile each widget alone so a broken one is dropped with diagnostics while the others keep rendering. (spec:2026-10-02-creation-widgets-design)
- Sanitize every table param with parse_table before Typst; on bad data render null and warn, never refuse the deck. (spec:2026-10-08-widget-tableau-design)
- Decide pack trust only with is_trusted_pack: namespace base and a version listed in packs/base/versions.txt, never namespace or id alone. (plan:2026-10-18-packs-lot2-base)

## Pitfalls

- Always include the _commun hash in the widget probe cache key; never detect its import by text. (spec:2026-10-02-creation-widgets-design)
- Name Snafu variants so their context selectors do not collide with existing ones, like RegistryCompile vs CompileSnafu. (plan:2026-09-27-widgets-typst-1)

## Testing

- Put Typst widget tests in oku-slides-render tests; there is no native Typst test runner. (spec:2026-10-04-parametre-liste-design)

## Settled decisions

- Do not add a line height to FieldBox; the UI derives it from TextStyle with a single formula. (plan:2026-09-28-champs-editables)
