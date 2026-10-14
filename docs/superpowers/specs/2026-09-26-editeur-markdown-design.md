# Text editor with markdown rendering — design

Date: 2026-09-26 (revised after the team review of the plan)

## Goal

In the editor laid over a text box, markdown stays written as is
(the marks stay visible) but is already displayed formatted:
`**montrer**` appears in bold, `*mot*` in italic, a link in the theme
accent, a highlight on its colored background. The editor shows the same
formatting as the slide; only line breaks may differ (the visible marks take
up space).

Out of scope: hiding marks outside the cursor's line (Obsidian
style); formatting in fields (title, subtitle, kicker), which stay plain
text; following a theme change during an edit (the palette is read on open,
`t-127`); arbitrating a concurrent write to the open text (voice agent,
`t-130`): today the last commit wins.

## Choice: CodeMirror 6

A `<textarea>` displays only one style. Rejected:

- Home-made `contentEditable`: rewriting the DOM on every keystroke breaks
  IME composition (Japanese) and native undo.
- A styled layer under a transparent textarea: real bold is wider,
  the cursor drifts.

CodeMirror 6 handles IME, selection, paste and history. Direct dependencies:
`@codemirror/state`, `@codemirror/view`, `@codemirror/commands` (all MIT).
`commands` pulls in `@codemirror/language` and the Lezer runtime (`@lezer/common`,
`@lezer/highlight`, `@lezer/lr`): they are installed, but we load no grammar
— no `@codemirror/lang-markdown`, the styles come from our parser.
`useEffectEvent` requires React 19.2 at least: the minimum versions of
`react`, `react-dom` and their types move to `^19.3.0`.

## Architecture

- `ui/src/markdown.ts`: the parser walk records positions.
  `markdownSpans(line): Span[]`, where `Span = { from, to, kind }` and `kind`
  is `mark` (delimiters, `\`, `](`, `)`, `{n}`), `bold`, `italic`,
  `link`, `url` or `highlight` (with `highlight: n`). `markdownToRuns`
  relies on the same walk: a single implementation. New
  `listPrefix(line)` (prefix `- ` / `1. `), shared by `textToParagraphs`
  (`deck.ts`) and the editor: a single list rule.
- `ui/src/textEdits.ts` (new): pure edits on (text, selection) —
  `toggleEmphasis` and `insertLink` (moved from `markdown.ts`, unchanged),
  `insertDictated` (taken out of the dictation effect in `Stage.tsx`) and
  `minimalChange` (the smallest replacement between two texts).
- `ui/src/color.ts` (new): `luminance` (moved from `Stage.tsx`, also reads
  `#rgb`) and the light background / dark background threshold, computed
  exactly like Typst's `ink-on` (luma > 60 %, i.e. ((0.6 + 0.055) / 1.055)^2.4
  in WCAG).
- `ui/src/markdownDecorations.ts` (new): pure `lineDecorations(line, palette)`,
  and the CodeMirror extension (`ViewPlugin`) that applies it to the whole
  document. The palette is accepted only in `#rgb` / `#rrggbb` form.
- `ui/src/TextEditor.tsx` (new): React wrapper around CodeMirror. Props:
  `initial`, `field` (one-line field, otherwise a markdown block), `palette`,
  `label` (aria-label), `style` (geometry), `onChange(text)`,
  `onCommit(text)`, `onCancel()`, and `ref` (React 19 prop) to
  `{ insertAtCursor(text) }` for dictation. The ⌘B / ⌘I / ⌘K shortcuts
  live here (CodeMirror keymap), not in `Stage`. Base keys:
  `standardKeymap` (not `defaultKeymap`, which is made for code).
- `ui/src/Stage.tsx`: keeps `editorGeometry`, the mask, the help; replaces
  the `<textarea>` with `TextEditor`. `commit(text)` receives the text from
  CodeMirror: it is the single source of truth; `draft` now only serves for
  the width of a field.

## Rendering in the editor

| Span | Style |
|---|---|
| `mark`, `url`, list prefix (`- `, `1. `) | text color at 40 % opacity |
| `bold` | `font-weight: bolder` |
| `italic` | `font-style: italic` |
| `link` | theme accent (`--t-accent`), underlined |
| `highlight` | background `palette[(n − 1) mod length]` (positive modulo, as in Typst: `{0}` gives the last color), text `--t-ink` or `--t-bg` depending on luminance |

Styles combine. A highlight is a plain background, with no rounded corners
or margin: CodeMirror splits a decorated span around nested spans, so
rounded corners would repeat on each piece. The palette is
`appearance.chart_palette` (`#rgb` or `#rrggbb`). CodeMirror settings
(font, line height, margins, cursor color) go through
`EditorView.theme`, with selectors at least as specific as those of its
base theme.

## Behavior

- Block: ⌘Enter commits; Enter inserts a plain line break (without
  copying indentation). Field: Enter and Shift+Enter commit; a pasted,
  dictated or already present line break becomes a space, and the
  change keeps its undo step. Escape cancels. Focus loss:
  commits.
- An Enter that confirms an IME conversion does not commit: CodeMirror
  already ignores it (including the Enter WebKit sends right after
  `compositionend`); no home-made guard.
- No native formatting: ⌘U everywhere, ⌘B / ⌘I / ⌘K in a field, do
  nothing.
- A click in the editor's margin keeps focus (does not commit).
- ⌘Z / ⇧⌘Z: CodeMirror history, limited to the current edit; each
  shortcut (⌘B, ⌘I, ⌘K) and each dictation is a separate undo step.
  Code editor commands (move a line, indent…) are not
  bound.
- Paste: plain text only (CodeMirror reads only `text/plain`).
- The editor grows with its content, background `editorBackground`, line
  wrapping within the box width; a field widens as you type.
- Dictation: dictated text is inserted at the selection, same spacing rules.
- `spellcheck`, `autocorrect`, `aria-label`, `aria-multiline` (false for a
  field), per-line direction (`perLineTextDirection`).
- The component supports React StrictMode's double mount.
- Defense in depth: the global shortcuts in `App.tsx` also ignore
  a `[contenteditable="true"]` (they already stop during editing).

## Tests (TDD)

- `markdown.test.ts`: `markdownSpans` (bold, italic, nesting, link,
  empty link, highlight with and without `{n}`, escapes, emoji, delimiters
  without closing) and `listPrefix`. The fixture shared with Rust remains the
  contract of `markdownToRuns`.
- `textEdits.test.ts`, `color.test.ts`, `markdownDecorations.test.ts`:
  pure functions, no DOM. `TextEditor.test.ts`: the one-line filter
  on an `EditorState` (paste, undo).
- e2e `editor.cjs` (Chromium, standalone, run first): formatting
  while typing and highlight color, CodeMirror font, line height and cursor
  equal to the host's, Escape closes without changing anything, click
  in the margin, ⌘B isolated in the history, paste, Enter in a block,
  focus loss, round trip (reopening gives back the same text), field
  (Enter, Shift+Enter, multiline paste), ⌘Z that does not touch the
  deck history.
- Check in development mode (`just web`, StrictMode on):
  open, cancel, reopen, commit → a single undo entry.
- Check in the desktop app (WebKit, `just dev`): Japanese IME conversion
  with Enter in a title and a block, system dictation,
  Édition > Annuler (Edit > Undo) from the menu (after a keystroke and right
  after ⌘B).
