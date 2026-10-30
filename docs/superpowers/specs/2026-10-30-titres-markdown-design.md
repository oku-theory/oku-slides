# Markdown headings in the text editor

Task t-1284. Extends the editing dialect of `plan:2026-09-27-markdown-lezer`.

Today a line starting with `#` in a text editor stays plain text: `ui/src/markdown.ts`
removes `ATXHeading` from the Lezer parser, and `Paragraph` (`runs`, `list`) has no field
for a heading. After this change **`#` to `######` followed by a space make a heading
paragraph**, kept in the deck, written back as `#`, and styled by the theme.

## Decisions (made on 2026-10-30)

1. **A heading is a paragraph property**, like `list`, not a formatting shortcut on runs:
   `#` survives a reopen and a theme change restyles every heading.
2. **Six levels**, the CommonMark ATX range.
3. **Styles come from a `heading` token group** whose values name existing tokens
   (the `kicker` / `list` pattern). Rejected: em multipliers relative to the element's
   size (diverge from the theme's title scale, new kind of token); a fixed mapping in
   Typst (themes could not restyle, values in code instead of JSON).
4. **A body heading is not a Typst `heading`.** Slide titles are Typst headings (chapters
   level 1, slide titles level 2, outlined and bookmarked) and the TOC layout queries
   `heading.where(level: 1)`; a `#` in a text box must stay out of the TOC, the PDF
   bookmarks and chapter numbering.
5. **Heading wins over list** when a paragraph carries both (only a concurrent merge can
   produce it).

## Model and storage

- `Paragraph` gains `heading: Option<HeadingLevel>` (Rust `oku-slides-core/src/model.rs`,
  TS `ui/src/api.ts`). `HeadingLevel` is a newtype over `u8` accepting 1 to 6; serde
  rejects any other number with `HeadingLevelError`, a Snafu error with a backtrace that
  follows `NamespaceError` (no Fluent message of its own: a bad deck surfaces through the
  existing localized parse errors). JSON:
  `{"runs": [...], "heading": 2}`, key absent for a plain paragraph.
- `is_paragraphs` (`oku-slides-core/src/params.rs`) and `_is-paragraphs`
  (`typst/oku-slides/src/data.typ`) accept the `heading` key.
- Replica (`replica/rich.rs`): `PARA_MARKS = ["list", "heading"]`; `heading` joins
  `NOT_EXPANDING`. Undo, history and merges behave as for `list`.
- Both marks on one separator are kept as merged; rendering and `paragraphs_to_text` apply
  decision 5. The editor and the agent never write both.
- Not breaking: existing decks have no `heading` key.

## Markdown dialect

All editing markdown is read by the UI (`textToParagraphs`, also for agent rewrites) and
written by Rust (`paragraphs_to_text`).

- **Reading (TS).** `listPrefix` becomes `blockPrefix(line): {list?, heading?, length}`,
  the single rule for line prefixes. A heading prefix is `^#{1,6} ` (space required).
  `#` alone, `#tag` and seven `#` stay plain text. One prefix per line: `# - x` is a
  level-1 heading of text `- x`. `textToParagraphs`, `literalToParagraphs` (paste,
  dictation) and the editor decorations all use `blockPrefix`.
- **Writing (Rust, `oku-slides-session/src/text.rs`).** A heading writes `#` × level and a
  space before its inline markdown. A heading ends a numbered list, like any non-list
  paragraph.
- **Escaping, both sides.** Text that starts like a heading prefix gets a `\` before it,
  beside the list-marker escaping (`escape_list_marker` in Rust, `literalText` in TS);
  Lezer reads `\#` as an escape, so it comes back literal. `fixtures/markdown.json`
  gains heading cases, run by both sides.
- **Editor display (`markdownDecorations.ts`).** The `#` prefix is always visible, dimmed
  (`cm-md-mark`); the line gets `cm-md-h1` to `cm-md-h6`: bold, fixed em scale from 1.6
  down to 1.0 (`styles.css`). The editor is a source view and does not mirror theme sizes.
- **Settled decision** in the root `AGENTS.md` becomes: bold, italic, links,
  ==highlight== with optional {n} 0-255, `#` to `######` headings, - and 1. lists.

## Rendering and tokens

- `typst/oku-slides/src/default-tokens.json` gains a `heading` group keyed `h1` to `h6`,
  each `{size, weight, font, color}` naming other tokens:

  | level | size | weight | font | color |
  |---|---|---|---|---|
  | h1 | `title` | `title` | `display` | `ink` |
  | h2 | `subtitle` | `title` | `display` | `ink` |
  | h3 | `lead` | `strong` | `body` | `ink` |
  | h4 | `body` | `strong` | `body` | `ink` |
  | h5 | `small` | `strong` | `body` | `ink` |
  | h6 | `caption` | `strong` | `body` | `ink` |

  The group list in the `tokens.typ` header comment names it. Themes override any key.
- `_paragraphs` (`typst/oku-slides/src/content.typ`) checks `heading` before `list` and
  renders `par(text(size, weight, font, fill, body))` through `size-of`, `weight-of`,
  `font-of` and the color lookup. A run's own size or color still wins. A bold run inside
  a heading takes the heavier of the heading weight and `strong`.
- Spacing around a heading is the element's paragraph spacing; no new spacing token.
- Theme editor: `oku-slides-session/src/theme_schema.json` gains a `headings` section
  (`Section::Headings`, right after Typography),
  four entries per level modeled on the `kicker` entries (size `pt` with `refs: "size"`,
  `weight`, `font` choice display/body/mono, `color`). The section label is a Fluent key
  in every existing locale.

## Other consumers

- `ui-stage-hint-rich` (every locale) mentions `# ` for a heading.
- Help deck `docs/aide/*.json`: the `heading` key in the file format, `#` in editing.
- MCP tool schemas and descriptions listing paragraph fields gain `heading`.
- `ARCHITECTURE.md` / `AGENTS.md` of core, session, `ui/`, `typst/oku-slides/` stay current.

Out of scope: mapping PPTX outline levels to headings on import, spacing tokens around
headings, hiding `#` in the editor.

## Testing (test first)

- Core: `HeadingLevel` serde (1 and 6 accepted; 0 and 7 rejected with `HeadingLevelError`); `is_paragraphs` with `heading`; replica round trip;
  concurrent heading + list merge keeps both marks.
- Session: `paragraphs_to_text` writes `## `; heading wins over list; heading ends a
  numbered list; plain `# x` text is written `\# x`; shared `fixtures/markdown.json`.
- TS: `blockPrefix` table cases (`#`, `#tag`, `####### x`, `# - x`, `###### x`);
  `textToParagraphs` and `literalToParagraphs`; decorations give `cm-md-h2`.
- Render: a heading paragraph uses the `heading.hN` size; a body heading is absent from
  `query(heading)` (TOC unaffected); a theme override of `heading.h2.color` applies.
- Theme schema: the `headings` entries resolve against `default-tokens.json`.
- e2e: typing `## Titre` in a text element is kept as a heading, and the editor reopened
  reads `## Titre` back from the host.
