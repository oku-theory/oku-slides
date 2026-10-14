# Editing markdown via @lezer/markdown — design

Date: 2026-09-27 · Task: t-180

## Goal

Replace the UI's home-grown markdown parser (`ui/src/markdown.ts`)
with `@lezer/markdown`, switching to CommonMark semantics. Strict
guarantee: the runs → markdown → runs round trip never loses text;
only the formatting at the edges of delimited runs may be lost (see
"Rust").

Markdown is only an editing view: the deck stores `Run`s. No
data migration.

## Decisions

- **TS: `@lezer/markdown`.** The spike passed the 12 fixture cases. Blocks
  can be disabled, and the tree gives exact positions for the
  CodeMirror decorations.
- **Rust: home-grown serializer.** Crates evaluated and rejected:
  - `comrak` 0.55: correct, but its escaping is not minimal and cannot
    be tuned (`Bonjour \!`, `snake\_case`, `\#tag`). It does not handle `{n}`.
  - `pulldown-cmark-to-cmark` 22: lossy. It escapes only the first
    character of a text, so a literal `*b*` is read back as italic.
    No `==`.
  - `mdast_util_to_markdown` 0.0.2: produces numeric entities
    (`&#x78;`), has no `==` and is still immature.
- **`==` has permissive rules** (this is not CommonMark, we
  define its meaning): it opens if not followed by a space, closes if not
  preceded by a space, like the current parser. With strict flanking,
  a bold highlight glued to a letter (`上==**a**==です`, common in
  Japanese) would be impossible to write.
- **Unchanged syntax**: `**gras**`, `*italique*` / `_italique_`,
  `***les deux***`, `[texte](url)`, `==pastille==`, `==pastille=={n}`
  (n from 0 to 255), `- ` and `1. ` at the start of a line.

## TS

- Dependency: `@lezer/markdown` (which brings `@lezer/common`). No
  `@codemirror/lang-markdown`.
- `ui/src/markdownHighlight.ts` (new): `MarkdownConfig` with the
  `Highlight` and `HighlightMark` nodes, and `==` delimiters resolved by Lezer
  like `Strikethrough` (after `Emphasis`), with the permissive rules
  above. A `===` opens nothing; an escaped `\=` right before a `==`
  does not count in the run of `=` (`==\===` is a "=" highlight).
- `ui/src/markdown.ts`: the public API does not change (`markdownToRuns`,
  `markdownSpans`, `listPrefix`, `encodeLink`, types `Span` and `SpanKind`),
  so `textEdits.ts` and `markdownDecorations.ts` stay as they are.
  `sameFormatting` goes away (see `deck.ts`). Internally:
  - the parser is configured once at module level, with
    `Highlight` and `remove` of all blocks (`LinkReference`,
    `IndentedCode`, `FencedCode`, `Blockquote`, `HorizontalRule`,
    `BulletList`, `OrderedList`, `ATXHeading`, `HTMLBlock`,
    `SetextHeading`) and of the inlines outside the dialect (`HTMLTag`, `Autolink`,
    `InlineCode`, `Image`, `Entity`, `HardBreak`);
  - a single tree walk produces the runs and the spans.
    `EmphasisMark`, `HighlightMark`, `LinkMark` and `Escape` (the `\`)
    become `mark` spans; `URL` becomes `url`. `{n}` is read right
    after a `Highlight` node and yields a `mark` span; above 255, it
    stays text;
  - `listPrefix` and the per-line splitting stay home-grown: in the editor,
    a line is a paragraph, which is not a CommonMark block.
  - The home-grown helpers (`scan`, `closing`, `closingUnderscore`, `link`,
    `repeated`) are removed.
  - A link without an address (`[a]`, `[a][b]`, `[a]()`) stays literal text,
    as today.
  - `***x***`: CommonMark nests it as `<em><strong>`, so the italic
    span goes from 1 to 6 (no longer from 3 to 4). The existing test is
    updated.
- `ui/src/deck.ts`: `textToParagraphs(text, previous, previousText)` keeps
  the paragraph `previous[i]` whose original line `previousText`
  (written by `paragraphs_to_text`, i.e. `Target.text`) is identical to the
  new line. We compare lines, no longer read-back runs
  (`sameFormatting`): an unedited line keeps its color and size
  even when its edges suffered the documented loss. `textOps` passes
  `t.text`.

## Rust

- `crates/oku-slides-session/src/inline.rs` (new, tests
  `inline_tests.rs`): `runs_to_markdown(&[Run]) -> String`, moved from
  `text.rs` and rewritten. `text.rs` keeps `paragraphs_to_text`,
  `escape_list_marker` and `interp`.
- **Emphasis stack**: an emphasis shared by neighboring runs stays
  open (bold-italic `X` then italic `Y` gives `***X**Y*`, not
  `***X****Y*`, which CommonMark reads back wrongly). The longest one is on the
  outside. Highlights and links are barriers: they close the
  open emphases and carry their own emphases inside
  (`==**a**==`, `[**a**](u)`).
- **Edge normalization**, before writing: at each junction where
  `*` close or open, closings and openings form a single run
  of `*` between two text characters `a` and `b`. It must close
  (`a` not a space; if `a` is punctuation, `b` is not a
  letter or a digit) and open (symmetric). Highlight outside a link: no
  space against `==`. Otherwise the offending character leaves its delimiters:
  it becomes a run that keeps the emphases still open around it, without
  highlight or link. This repeats until there is no fault left.
  Each move removes formatting, so the loop terminates.
  In a link, edge spaces stay inside the brackets, outside the
  delimiters. Brackets, `==` and `{n}` count as punctuation.
- **The only accepted loss** is therefore formatting on spaces
  and punctuation, never on a letter or a digit, never on the
  text. It shows for the highlight (background) and barely for bold and
  italic. Examples: `a` + bold `" x "` + `b` gives `a **x** b`;
  bold `-` + italic `a` gives `-*a*`. Prototype (Rust → Lezer,
  500,000 random runs): 0 text loss, 0 loss on a letter or a
  digit. An unedited line keeps its original paragraph anyway
  (see `deck.ts`).

## Tests (TDD)

- `fixtures/markdown.json` stays the contract: Rust checks paragraphs →
  markdown (`text_tests.rs`), TS checks markdown → paragraphs
  (`markdown.test.ts`).
- New fixture cases: literal `*b*` and `[x](y)`, `# ` and `> ` at the
  start of a line, `Bonjour !`, "=" highlight (`==\===`), italic then
  bold glued (`*a***b**`), bold-italic then italic (`***X**Y*`),
  bold highlight glued to Japanese (`上==**a**=={2}です`).
- `lossy` section of the fixture (paragraphs → markdown → read-back paragraphs):
  spaces at bold edges, punctuation glued to a letter, highlight reduced
  to a space, bold on a dash glued to italic. Rust checks the
  markdown, TS checks the read-back paragraphs.
- **Cross test on a corpus**: `inline_tests.rs` generates a deterministic
  corpus of random runs (home-grown generator, no dependency) and
  writes it with their markdown into `fixtures/markdown-corpus.json`. The test
  fails if the file differs; `UPDATE_CORPUS=1` rewrites it.
  `markdown.test.ts` reads back each line and checks the invariant: same text,
  same formatting on every letter and every digit.
- `textToParagraphs`: a line identical to its original line keeps its
  paragraph (color, size), even after an edge loss.
- The existing tests of `markdown.test.ts` and
  `markdownDecorations.test.ts` stay. Only those whose
  semantics change on purpose are updated, and each change is justified by
  the relevant CommonMark rule.
- e2e `bun e2e/editor.cjs` green.

## Out of scope

Import or export of `.md` files, new syntaxes (headings, code,
tables), hiding the `\` in the editor.
