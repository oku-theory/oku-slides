# typst/oku-slides — agent rules

## Invariants

- Render table cell texts as plain strings, never as Typst markup or through eval. (spec:2026-10-08-widget-tableau-design, plan:2026-10-08-widget-tableau)
- Wrap every plain and rich widget param in field() so its editable box is emitted. (spec:2026-09-27-widgets-typst-design)
- Restrict p.list columns to plain, rich, number, flag, choice, asset or one nested p.list level; never name one id, kind or with a slash. (spec:2026-10-04-parametre-liste-design, t-495)
- Ensure an empty field never changes layout, so export output stays identical. (spec:2026-09-28-champs-editables-design)
- Emit list items as enum.item and list.item, never one enum or list per paragraph. (t-173)
- When shipping a font for a new script, add it to script-fallback in tokens.typ and to the issue_0176 test. (t-177)
- Declare zone sizing and typography in the widget schema (grow, typography); never test a kind name in the core library. (plan:2026-10-18-packs-lot2-base)
- Append the new base version to packs/base/versions.txt whenever a file of packs/base changes; never remove a line, so decks embedding an older base stay trusted. (plan:2026-10-18-packs-lot2-base)
- Append the new essentiel version to packs/essentiel/versions.txt whenever a file of packs/essentiel changes. (plan:2026-10-19-packs-lot3-essentiel)
- Keep the core library free of widgets: slide layouts live in packs/essentiel, element widgets in packs/base. (plan:2026-10-19-packs-lot3-essentiel)

## Pitfalls

- Write a conditional set rule as set rule if cond; a set inside an if block applies to nothing. (t-841)
- Change the example widget fixtures and the Typst README code blocks together; a test compares them verbatim. (plan:2026-10-02-creation-widgets)
- Scale a table down only if it still overflows after wrapping, never from a zero measurement. (spec:2026-10-08-widget-tableau-design)
- Add new component tokens to themes/pro.json too; it is a full snapshot, not a partial override. (plan:2026-10-08-widget-tableau)
- Stack template fields with fields-stack, not stack, or empty-field markers get spacing or stay inert. (spec:2026-09-28-champs-editables-design)
- Attach empty-field markers with place() to neighbor items; never wrap the stack in layout(), it shifts rendering. (spec:2026-09-28-champs-editables-design)

## Testing

- Keep the golden render test pixel-identical on Typst changes; treat any diff as a bug, never re-record it. (plan:2026-09-28-champs-editables)

## Settled decisions

- Take chart colors from the theme chart.palette token, overridable by the deck, never from hardcoded palettes. (t-77)
- Make widgets read colors and sizes from theme tokens via color-of and size-of, never hard-coded values. (plan:2026-10-02-creation-widgets)
- Model repeated widget items with p.list; never simulate a list with numbered params like date1, date2. (plan:2026-10-04-parametre-liste)
- Do not wrap the cover author and date in field(); they share one line and field() renders a block. (plan:2026-09-28-champs-editables)
- Declare each widget's params in its own .typ file as the single source of truth; add no JSON manifest. (spec:2026-09-27-widgets-typst-design, t-270)
- Fix a slot's flow or free mode in the schema, never through another param's value. (spec:2026-09-27-widgets-typst-design)
