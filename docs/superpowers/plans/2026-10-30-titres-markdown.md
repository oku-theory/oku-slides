# Markdown headings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `#` to `######` followed by a space in a text editor make a heading paragraph, kept in the deck, written back as `#`, styled by theme tokens.

**Architecture:** `Paragraph` gains `heading: Option<HeadingLevel>` (1–6), stored as a second paragraph mark in the Loro replica. The UI reads line prefixes with one `blockPrefix` rule; Rust writes them back in `paragraphs_to_text`. Typst renders a heading as a styled `par` (never a Typst `heading`) from a new `heading` token group, exposed in the theme editor.

**Tech Stack:** Rust (serde, schemars, Snafu, Loro 1.16.2, speculoos, rstest, nextest), TypeScript (Bun test, CodeMirror 6, `@lezer/markdown`), Typst, Fluent, Playwright e2e.

**Spec:** `docs/superpowers/specs/2026-10-30-titres-markdown-design.md` (task t-1284).

## Global Constraints

- A heading prefix is `^#{1,6} ` (space required); `#` alone, `#tag` and seven `#` are plain text.
- One prefix per line: `# - x` is a level-1 heading of text `- x`.
- Heading wins over list wherever both are present (Typst rendering, `paragraphs_to_text`).
- A body heading is never a Typst `heading` element.
- Default heading tokens: h1 `title/title/display/ink`, h2 `subtitle/title/display/ink`, h3 `lead/strong/body/ink`, h4 `body/strong/body/ink`, h5 `small/strong/body/ink`, h6 `caption/strong/body/ink` (size/weight/font/color).
- Editor heading em scale: h1 1.6, h2 1.45, h3 1.3, h4 1.2, h5 1.1, h6 1.0; the `#` prefix always visible, dimmed.
- No `unwrap`/`expect` outside tests; Snafu errors with a backtrace; no `std::fs` in production code.
- Rust unit tests in a sibling `x_tests.rs`, speculoos assertions, names `unit_scenario_result`, rstest for tables.
- Every pub item and new module documented, first sentence under 15 words; files under 500 lines when possible, never over 1000.
- Every new Fluent key in all 20 locales, same key order, translated (never English copies).
- Commits: Conventional Commits in English citing `t-1284` and the sub-task id; message written to a file in the scratchpad and committed with `git commit -F`; **no `Co-Authored-By` line**.
- Run outside `just e2e` only with fresh `OKU_SLIDES_PATHS__DATA` / `OKU_SLIDES_PATHS__DECKS` temp dirs.
- First nextest build in this worktree is cold (over 10 min): run it in the background.

## Spec corrections (made with this plan)

- `HeadingLevelError` follows `NamespaceError`: a serde-level error with a Snafu display and a backtrace, no Fluent message of its own (a bad deck surfaces through the existing localized parse errors). Tests check the error type, not a locale.
- The theme editor section is `Section::Headings` (`"headings"`), placed right after Typography.
- The e2e checks that the editor, reopened, reads `## Titre` back from the host (`paragraphs_to_text`).

---

### Task 1: `HeadingLevel` and `Paragraph.heading` in the core model (t-1285)

**Files:**
- Create: `crates/oku-slides-core/src/heading_level.rs`
- Create: `crates/oku-slides-core/src/heading_level_tests.rs`
- Modify: `crates/oku-slides-core/src/lib.rs` (module, re-exports, crate doc list)
- Modify: `crates/oku-slides-core/src/error/mod.rs` (after `NamespaceError`, ~line 787)
- Modify: `crates/oku-slides-core/src/model.rs:414-431` (`Paragraph`)
- Modify: `crates/oku-slides-core/src/params.rs:64` (`PARAGRAPH_KEYS`)
- Modify: `crates/oku-slides-core/src/params_tests.rs`
- Modify: every `Paragraph { … }` literal in the workspace (add `heading: None`): `crates/oku-slides-core/src/{canonical_tests.rs,model.rs,replica/mod_tests.rs,replica/rich_tests.rs}`, `crates/oku-slides-import/src/{coverage_tests.rs,hifi/element_text.rs,hifi/element_text_tests.rs,pptx/cascade.rs,pptx/cascade_tests.rs,pptx/text.rs,retrofit/measure_tests.rs,retrofit/text.rs,test_support.rs}`, `crates/oku-slides-render/tests/render.rs`, `crates/oku-slides-session/src/{lib_tests.rs,text_tests.rs}`, `crates/oku-slides-session/tests/{agent.rs,session.rs,regressions/issue_0630_link_angle_bracket.rs}`

**Interfaces:**
- Produces: `oku_slides_core::HeadingLevel` (`Copy`, `Eq`, serde as a JSON number 1–6, `JsonSchema` as an integer), `HeadingLevel::new(u8) -> Option<HeadingLevel>`, `HeadingLevel::get(self) -> u8`, `TryFrom<u8>` with `Error = HeadingLevelError`, `From<HeadingLevel> for u8`; `oku_slides_core::HeadingLevelError`; `Paragraph.heading: Option<HeadingLevel>` (JSON key `heading`, skipped when `None`).

- [ ] **Step 1: Write the failing tests** — `crates/oku-slides-core/src/heading_level_tests.rs`:

```rust
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::{Paragraph, Run};

#[rstest]
#[case(1)]
#[case(6)]
fn heading_level_new_in_range_is_some(#[case] n: u8) {
    assert_that!(HeadingLevel::new(n).map(HeadingLevel::get)).is_equal_to(Some(n));
}

#[rstest]
#[case(0)]
#[case(7)]
fn heading_level_try_from_out_of_range_is_heading_level_error(#[case] n: u8) {
    let err: Result<HeadingLevel, HeadingLevelError> = HeadingLevel::try_from(n);
    assert_that!(err.is_err()).is_true();
}

#[test]
fn paragraph_with_heading_round_trips_as_a_number() {
    let p = Paragraph { runs: vec![Run::plain("Plan")], list: None, heading: HeadingLevel::new(2) };
    let v = serde_json::to_value(&p).unwrap();
    assert_that!(v).is_equal_to(json!({ "runs": [{ "t": "Plan" }], "heading": 2 }));
    assert_that!(serde_json::from_value::<Paragraph>(v).unwrap()).is_equal_to(p);
}

#[test]
fn paragraph_without_heading_has_no_heading_key() {
    let v = serde_json::to_value(Paragraph::plain("a")).unwrap();
    assert_that!(v.get("heading")).is_none();
}

#[rstest]
#[case(json!(0))]
#[case(json!(7))]
#[case(json!("2"))]
fn paragraph_with_invalid_heading_is_refused(#[case] heading: serde_json::Value) {
    let r = serde_json::from_value::<Paragraph>(json!({ "runs": [], "heading": heading }));
    assert_that!(r.is_err()).is_true();
}
```

Append to `crates/oku-slides-core/src/params_tests.rs`:

```rust
#[test]
fn deserialize_paragraphs_with_heading_gives_rich() {
    let p = params(json!({ "body": [{ "runs": [{ "t": "Plan" }], "heading": 1 }] }));
    assert_that!(matches!(p.get("body"), Some(ParamValue::Rich(_)))).is_true();
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-core heading_level deserialize_paragraphs_with_heading` (background on a cold build)
Expected: compile error, `HeadingLevel` not found / no field `heading`.

- [ ] **Step 3: Implement**

`crates/oku-slides-core/src/heading_level.rs`:

```rust
//! Heading level of a paragraph, from 1 (largest) to 6.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{HeadingLevelError, HeadingLevelSnafu};

/// Highest heading level, as in CommonMark ATX headings.
const MAX_LEVEL: u8 = 6;

/// Heading level of a paragraph: 1 to 6, written `#` to `######`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "u8", into = "u8")]
#[schemars(with = "u8")]
pub struct HeadingLevel(u8);

impl HeadingLevel {
    /// Level `n`, or `None` outside 1 to 6.
    #[must_use]
    pub fn new(n: u8) -> Option<Self> {
        (1..=MAX_LEVEL).contains(&n).then_some(Self(n))
    }

    /// The level as a number, 1 to 6.
    #[must_use]
    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for HeadingLevel {
    type Error = HeadingLevelError;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        Self::new(n).ok_or_else(|| HeadingLevelSnafu { value: n }.build())
    }
}

impl From<HeadingLevel> for u8 {
    fn from(h: HeadingLevel) -> Self {
        h.0
    }
}

#[cfg(test)]
#[path = "heading_level_tests.rs"]
mod heading_level_tests;
```

`crates/oku-slides-core/src/error/mod.rs`, after `NamespaceError`:

```rust
/// A paragraph heading level outside 1 to 6.
#[derive(Debug, Snafu)]
#[snafu(display("invalid heading level: {value}"), context(suffix(Snafu)), visibility(pub(crate)))]
pub struct HeadingLevelError {
    value: u8,
    backtrace: Backtrace,
}
```

`crates/oku-slides-core/src/lib.rs`: add `mod heading_level;` (alphabetical, after `mod fields;`), `pub use heading_level::HeadingLevel;`, add `HeadingLevelError` to the `pub use error::{…}` list (alphabetical), and in the crate doc list next to `Deck` mention `[`HeadingLevel`]: heading level of a paragraph`.

`crates/oku-slides-core/src/model.rs`, `Paragraph`:

```rust
/// A paragraph: a sequence of runs, possibly a list item or a heading.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Paragraph {
    /// Runs of the paragraph, in text order.
    #[serde(default)]
    pub runs: Vec<Run>,
    /// List the paragraph is an item of; absent: plain paragraph.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<ListKind>,
    /// Heading level; absent: body text. Wins over `list` when both are set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<HeadingLevel>,
}
```

and `Paragraph::plain` returns `Self { runs: vec![Run::plain(t)], list: None, heading: None }`; add `use crate::HeadingLevel;` at the top of `model.rs`.

`crates/oku-slides-core/src/params.rs`: `const PARAGRAPH_KEYS: [&str; 3] = ["runs", "list", "heading"];`

Then run `cargo check --workspace --all-targets` and add `heading: None` to each `Paragraph { … }` literal it reports (the files listed above).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p oku-slides-core` then `cargo check --workspace --all-targets`
Expected: PASS, no compile error.

- [ ] **Step 5: Commit**

```bash
git add crates/
git commit -F "$SCRATCH/msg.txt"   # feat(core): heading level on paragraphs (t-1284, t-1285)
```

---

### Task 2: `heading` as a paragraph mark in the Loro replica (t-1286)

**Files:**
- Modify: `crates/oku-slides-core/src/replica/rich.rs:22,27`
- Modify: `crates/oku-slides-core/src/replica/rich_tests.rs`
- Modify: `crates/oku-slides-core/src/replica/ARCHITECTURE.md` (paragraph marks line, if it names `list`)

**Interfaces:**
- Consumes: `Paragraph.heading` (Task 1), JSON key `heading`.
- Produces: `PARA_MARKS = ["list", "heading"]`; `heading` does not expand to typed text.

- [ ] **Step 1: Write the failing tests** — in `rich_tests.rs`, change the paragraph of `every_run_and_paragraph_field_has_a_mark` to `crate::Paragraph { runs: vec![], list: Some(crate::ListKind::Bullet), heading: crate::HeadingLevel::new(2) }`, and append:

```rust
#[test]
fn sync_heading_then_read_returns_same_paragraphs() {
    let d = doc();
    let t = d.get_text("t");
    let v = json!([{ "runs": [{ "t": "Plan" }], "heading": 2 }, { "runs": [{ "t": "corps" }] }]);
    sync(&t, &v).unwrap();
    d.commit();
    assert_that!(Value::Array(read(&t))).is_equal_to(&v);
}

#[test]
fn styles_heading_does_not_expand() {
    assert_that!(styles().get(&"heading".into()).map(|c| c.expand)).is_equal_to(Some(ExpandType::None));
}

#[test]
fn concurrent_heading_and_list_keep_both_marks() {
    let base = doc();
    base.set_peer_id(0).unwrap();
    sync(&base.get_text("t"), &json!([{ "runs": [{ "t": "Plan" }] }])).unwrap();
    base.commit();
    let (a, b) = (base.fork(), base.fork());
    a.config_text_style(styles());
    b.config_text_style(styles());
    sync(&a.get_text("t"), &json!([{ "runs": [{ "t": "Plan" }], "heading": 1 }])).unwrap();
    a.commit();
    sync(&b.get_text("t"), &json!([{ "runs": [{ "t": "Plan" }], "list": "bullet" }])).unwrap();
    b.commit();
    a.import(&b.export(loro::ExportMode::all_updates()).unwrap()).unwrap();
    assert_that!(Value::Array(read(&a.get_text("t"))))
        .is_equal_to(json!([{ "runs": [{ "t": "Plan" }], "list": "bullet", "heading": 1 }]));
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-core rich`
Expected: FAIL — `every_run_and_paragraph_field_has_a_mark` (key `heading` not in `PARA_MARKS`), `sync_heading…` (heading dropped), `styles_heading…` (`None`).

- [ ] **Step 3: Implement** — `rich.rs`:

```rust
/// Champs d'un `Paragraph`, hors `runs`.
pub(super) const PARA_MARKS: &[&str] = &["list", "heading"];
```

```rust
/// Marques qui ne s'étendent pas à la frappe qui les suit.
const NOT_EXPANDING: &[&str] = &["link", "list", "heading"];
```

If `replica/ARCHITECTURE.md` lists paragraph marks, add `heading` there.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p oku-slides-core`
Expected: PASS.

- [ ] **Step 5: Commit** — `feat(core): store paragraph headings as replica marks (t-1284, t-1286)`

---

### Task 3: Markdown dialect — Rust writer, TS reader, shared fixture (t-1287)

**Files:**
- Modify: `fixtures/markdown.json` (`cases`)
- Modify: `crates/oku-slides-session/src/text.rs:11-50`
- Modify: `crates/oku-slides-session/src/text_tests.rs`
- Modify: `ui/src/api.ts:237-240`
- Modify: `ui/src/markdown.ts:1-12,205-210` (`listPrefix` → `blockPrefix`)
- Modify: `ui/src/deck.ts:7,64-93`
- Modify: `ui/src/markdownDecorations.ts` (import and calls of `listPrefix` → `blockPrefix`; behavior unchanged in this task)
- Modify: `ui/src/markdown.test.ts`

**Interfaces:**
- Consumes: `HeadingLevel::get()` (Task 1).
- Produces (TS): `export type HeadingLevel = 1 | 2 | 3 | 4 | 5 | 6;` in `api.ts`; `Paragraph.heading?: HeadingLevel`; `blockPrefix(line: string): { list?: NonNullable<Paragraph["list"]>; heading?: HeadingLevel; length: number }` in `markdown.ts` (replaces `listPrefix`, which is deleted).
- Produces (Rust): `paragraphs_to_text` writes `#`×level + space; private `escape_block_marker` (replaces `escape_list_marker`).

- [ ] **Step 1: Add the shared fixture cases** — append to `cases` in `fixtures/markdown.json`:

```json
{ "name": "titres de niveaux 1 à 6", "markdown": "# Un\n## Deux\n### Trois\n#### Quatre\n##### Cinq\n###### Six",
  "paragraphs": [{ "runs": [{ "t": "Un" }], "heading": 1 }, { "runs": [{ "t": "Deux" }], "heading": 2 },
    { "runs": [{ "t": "Trois" }], "heading": 3 }, { "runs": [{ "t": "Quatre" }], "heading": 4 },
    { "runs": [{ "t": "Cinq" }], "heading": 5 }, { "runs": [{ "t": "Six" }], "heading": 6 }] },
{ "name": "titre en gras, puis un titre qui reprend la liste numérotée à 1", "markdown": "## **Plan**\n1. un\n# Fin\n1. deux",
  "paragraphs": [{ "runs": [{ "t": "Plan", "bold": true }], "heading": 2 }, { "runs": [{ "t": "un" }], "list": "number" },
    { "runs": [{ "t": "Fin" }], "heading": 1 }, { "runs": [{ "t": "deux" }], "list": "number" }] },
{ "name": "texte qui ressemble à un titre", "markdown": "\\# pas un titre\n\\###### pas non plus\n#mot\n####### sept",
  "paragraphs": [{ "runs": [{ "t": "# pas un titre" }] }, { "runs": [{ "t": "###### pas non plus" }] },
    { "runs": [{ "t": "#mot" }] }, { "runs": [{ "t": "####### sept" }] }] },
{ "name": "titre dont le texte commence comme une puce", "markdown": "# \\- x",
  "paragraphs": [{ "runs": [{ "t": "- x" }], "heading": 1 }] }
```

- [ ] **Step 2: Write the failing unit tests**

`crates/oku-slides-session/src/text_tests.rs` (add `HeadingLevel, ListKind` to the `oku_slides_core` import):

```rust
#[test]
fn paragraphs_to_text_heading_and_list_writes_the_heading() {
    let p = Paragraph { runs: vec![Run::plain("x")], list: Some(ListKind::Bullet), heading: HeadingLevel::new(2) };
    assert_that!(paragraphs_to_text(&[p])).is_equal_to("## x".to_owned());
}

#[rstest::rstest]
#[case("# x", "\\# x")]
#[case("###### x", "\\###### x")]
#[case("####### x", "####### x")]
#[case("#x", "#x")]
fn paragraphs_to_text_plain_text_like_a_heading_is_escaped(#[case] t: &str, #[case] expected: &str) {
    assert_that!(paragraphs_to_text(&[Paragraph::plain(t)])).is_equal_to(expected.to_owned());
}
```

`ui/src/markdown.test.ts`: import `blockPrefix` instead of `listPrefix`; in the `escapeInline` fixture test, add `.replace(/^\\(#{1,6} )/, "$1")` to the `lines` mapping, and make the `plain` filter exclude headings (`c.paragraphs.every((p) => !p.list && !p.heading && …)`); rename the `describe("listPrefix")` block to `blockPrefix` (same three tests, calling `blockPrefix`) and add:

```ts
  test("titre : dièses et espace", () => {
    expect(blockPrefix("## a")).toEqual({ heading: 2, length: 3 });
    expect(blockPrefix("###### a")).toEqual({ heading: 6, length: 7 });
  });

  test("dièse seul, collé, échappé ou sept dièses : pas de titre", () => {
    for (const line of ["#", "#a", "\\# a", "####### a"]) {
      expect(blockPrefix(line)).toEqual({ length: 0 });
    }
  });

  test("un seul préfixe : un titre dont le texte commence par un tiret", () => {
    expect(blockPrefix("# - x")).toEqual({ heading: 1, length: 2 });
  });
```

and in a `describe("literalToParagraphs")` (import it from `./deck`):

```ts
describe("literalToParagraphs", () => {
  test("un titre collé reste un titre, son inline littéral", () => {
    expect(literalToParagraphs("# 2 * 3 * 4")).toEqual([{ runs: [{ t: "2 * 3 * 4" }], heading: 1 }]);
  });
});
```

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-session text` and `cd ui && bun test src/markdown.test.ts`
Expected: FAIL — fixture cases (Rust writes no `#`, TS reads no heading), `blockPrefix` not exported.

- [ ] **Step 4: Implement**

`crates/oku-slides-session/src/text.rs`:

```rust
use oku_slides_core::{ListKind, Paragraph};

/// Texte éditable en markdown : un paragraphe par ligne, `#` à `######` pour un titre,
/// `- ` pour une puce, `1. ` pour une liste numérotée, le reste en markdown inline (`inline.rs`).
#[must_use]
pub fn paragraphs_to_text(paras: &[Paragraph]) -> String {
    let mut out = String::new();
    let mut n = 0;
    for (i, p) in paras.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        // Un titre l'emporte sur une liste (fusion concurrente des deux marques).
        let list = if p.heading.is_some() { None } else { p.list };
        if let Some(h) = p.heading {
            out.push_str(&"#".repeat(usize::from(h.get())));
            out.push(' ');
        }
        match list {
            Some(ListKind::Bullet) => out.push_str("- "),
            Some(ListKind::Number) => {
                n += 1;
                out.push_str(&n.to_string());
                out.push_str(". ");
            }
            None => {}
        }
        // Comme au rendu : tout autre paragraphe clôt la liste numérotée.
        if list != Some(ListKind::Number) {
            n = 0;
        }
        out.push_str(&escape_block_marker(&runs_to_markdown(&p.runs)));
    }
    out
}

/// Un texte qui commence comme un préfixe (`# `, `- `, `1. `) ne doit pas en devenir un.
fn escape_block_marker(line: &str) -> String {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    if (1..=6).contains(&hashes) && line[hashes..].starts_with(' ') {
        return format!("\\{line}");
    }
    if line.starts_with("- ") {
        return format!("\\{line}");
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && line[digits..].starts_with(". ") {
        return format!("{}\\{}", &line[..digits], &line[digits..]);
    }
    line.to_owned()
}
```

`ui/src/api.ts`:

```ts
/** Niveau de titre d'un paragraphe : 1 (le plus grand) à 6. */
export type HeadingLevel = 1 | 2 | 3 | 4 | 5 | 6;

export interface Paragraph {
  runs: Run[];
  list?: "bullet" | "number";
  /** Titre ; l'emporte sur `list` si les deux sont là. */
  heading?: HeadingLevel;
}
```

`ui/src/markdown.ts` — module doc: replace "`listPrefix` est la règle unique des préfixes de liste" with "`blockPrefix` est la règle unique des préfixes de ligne (titre, liste)"; replace `listPrefix` with:

```ts
/** Préfixe d'une ligne : `#` à `######` et espace (titre), `- ` (puce), `1. ` (numéro), sinon rien. */
export function blockPrefix(line: string): { list?: NonNullable<Paragraph["list"]>; heading?: HeadingLevel; length: number } {
  const heading = /^(#{1,6}) /.exec(line);
  if (heading) return { heading: heading[1].length as HeadingLevel, length: heading[0].length };
  if (line.startsWith("- ")) return { list: "bullet", length: 2 };
  const num = /^\d+\. /.exec(line);
  return num ? { list: "number", length: num[0].length } : { length: 0 };
}
```

(import type `HeadingLevel` from `./api`.)

`ui/src/deck.ts` — import `blockPrefix`; in `textToParagraphs`:

```ts
    const { list, heading, length } = blockPrefix(line);
    const runs = markdownToRuns(line.slice(length));
    return heading ? { runs, heading } : list ? { runs, list } : { runs };
```

in `literalText`: `const { length } = blockPrefix(line);`; update the doc of `literalToParagraphs` to say "seuls les préfixes `#`, `- ` et `1. ` font des titres ou des listes".

`ui/src/markdownDecorations.ts`: replace every `listPrefix` with `blockPrefix` (import, `LineParts.prefix` type, `lineParts`, `listOrdinal`, `build`); module doc "`markdownLayout` et `blockPrefix`".

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo nextest run -p oku-slides-session` and `cd ui && bun test && bunx tsc --noEmit`
Expected: PASS.

- [ ] **Step 6: Commit** — `feat(session,ui): # to ###### headings in editing markdown (t-1284, t-1287)`

---

### Task 4: Heading lines in the editor (t-1288)

**Files:**
- Modify: `ui/src/markdownDecorations.ts` (`lineParts`, `preview`, `PLAIN`)
- Modify: `ui/src/markdownDecorations.test.ts`
- Modify: `ui/src/styles.css:359-362`

**Interfaces:**
- Consumes: `blockPrefix` (Task 3).
- Produces: CSS classes `cm-md-h1` … `cm-md-h6` on heading lines.

- [ ] **Step 1: Write the failing tests** — `markdownDecorations.test.ts`:

```ts
describe("titres", () => {
  test("ligne de titre : classe du niveau sur toute la ligne, dièses atténués", () => {
    expect(lineDecorations("## Plan", [])).toEqual([
      { from: 0, to: 7, className: "cm-md-h2" },
      { from: 0, to: 3, className: "cm-md-mark" },
    ]);
  });

  test("dièses visibles même hors de la sélection", () => {
    expect(linePreview("# Plan", [], [], 0)).toEqual([
      { from: 0, to: 6, className: "cm-md-h1" },
      { from: 0, to: 2, className: "cm-md-mark" },
    ]);
  });

  test("un titre clôt la liste numérotée", () => {
    const lines = ["1. a", "# T", "1. b"];
    expect(listOrdinal((k) => lines[k - 1], 3)).toBe(1);
  });
});
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cd ui && bun test src/markdownDecorations.test.ts`
Expected: FAIL — no `cm-md-h*` decoration; the prefix of `# Plan` is turned into a marker widget.

- [ ] **Step 3: Implement** — in `lineParts`:

```ts
  const prefix = blockPrefix(line);
  const { length } = prefix;
  const out: LineDecoration[] = [];
  if (prefix.heading) out.push({ from: 0, to: line.length, className: `cm-md-h${prefix.heading}` });
  if (length > 0) out.push({ from: 0, to: length, className: "cm-md-mark" });
```

in `preview`, inside the prefix branch, before computing `shown`:

```ts
    if (prefix.length > 0 && d.from === 0 && d.to === prefix.length) {
      // Les dièses d'un titre restent visibles : le niveau est une information.
      if (prefix.heading) {
        out.push(d);
        continue;
      }
      const shown = touches(selection, d);
```

`PLAIN` kinds: `["mark", "url", "bold", "italic", "link", "highlight", "h1", "h2", "h3", "h4", "h5", "h6"]`. Update the module doc: "Un préfixe de liste devient sa puce ou son rang ; un titre garde ses dièses, atténués, et grandit."

`styles.css`, after line 362:

```css
.editor .cm-md-h1, .editor .cm-md-h2, .editor .cm-md-h3,
.editor .cm-md-h4, .editor .cm-md-h5, .editor .cm-md-h6 { font-weight: bolder; }
.editor .cm-md-h1 { font-size: 1.6em; }
.editor .cm-md-h2 { font-size: 1.45em; }
.editor .cm-md-h3 { font-size: 1.3em; }
.editor .cm-md-h4 { font-size: 1.2em; }
.editor .cm-md-h5 { font-size: 1.1em; }
```

(h6 stays at 1em.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd ui && bun test && bunx tsc --noEmit`
Expected: PASS.

- [ ] **Step 5: Commit** — `feat(ui): show heading lines in the text editor (t-1284, t-1288)`

---

### Task 5: `heading` design tokens and the theme editor section (t-1289)

**Files:**
- Modify: `typst/oku-slides/src/default-tokens.json` (new `heading` group after `list`)
- Modify: `typst/oku-slides/src/tokens.typ:11-24` (header comment, group list)
- Modify: `crates/oku-slides-session/src/theme_schema.json` (24 entries, after the `list` entries)
- Modify: `crates/oku-slides-session/src/theme_schema.rs:58-82` (`Section::Headings`) and its module doc
- Modify: `crates/oku-slides-session/src/theme_panel.rs:55-64` (`SECTIONS_BEFORE`)
- Modify: `crates/oku-slides-session/src/theme_schema_tests.rs`
- Modify: `crates/oku-slides-i18n/src/locales/*/session.ftl` (20 locales, after `theme-section-typography`)

**Interfaces:**
- Produces: tokens `heading.h1` … `heading.h6`, each `{size, weight, font, color}`; `Section::Headings` serialized `"headings"`; Fluent key `theme-section-headings`.

- [ ] **Step 1: Write the failing test** — `theme_schema_tests.rs`:

```rust
#[rstest]
#[case("size", |c: &Control| matches!(c, Control::Pt { .. }))]
#[case("weight", |c: &Control| matches!(c, Control::Weight))]
#[case("color", |c: &Control| matches!(c, Control::Color))]
fn schema_heading_levels_are_in_the_headings_section(#[case] key: &str, #[case] is: fn(&Control) -> bool) {
    for level in ["h1", "h2", "h3", "h4", "h5", "h6"] {
        let s = spec(full(), &p(&["heading", level, key])).unwrap_or_else(|| panic!("heading.{level}.{key} absent"));
        asserting(&format!("heading.{level}.{key}")).that(&(s.section.clone(), is(&s.control)))
            .is_equal_to((Section::Headings, true));
    }
}
```

(`Control` is `oku_slides_core::Control`; add it and `spec` to the test imports if missing.)

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo nextest run -p oku-slides-session theme_schema`
Expected: FAIL to compile (`Section::Headings` missing).

- [ ] **Step 3: Implement**

`default-tokens.json`, new group after `list`:

```json
"heading": {
  "h1": { "size": "title", "weight": "title", "font": "display", "color": "ink" },
  "h2": { "size": "subtitle", "weight": "title", "font": "display", "color": "ink" },
  "h3": { "size": "lead", "weight": "strong", "font": "body", "color": "ink" },
  "h4": { "size": "body", "weight": "strong", "font": "body", "color": "ink" },
  "h5": { "size": "small", "weight": "strong", "font": "body", "color": "ink" },
  "h6": { "size": "caption", "weight": "strong", "font": "body", "color": "ink" }
}
```

`tokens.typ` header: add `heading` to the group list ("…, list, kicker, heading (styles des titres d'un texte, h1 à h6, qui référencent les jetons par nom)").

`theme_schema.json`, after the `list` entries (keep the file's one-entry-per-line style), for each level `h1`…`h6`:

```json
  {"path": ["heading", "h1", "size"], "section": "headings", "control": "pt", "min": 0, "max": 400, "refs": "size"},
  {"path": ["heading", "h1", "weight"], "section": "headings", "control": "weight"},
  {"path": ["heading", "h1", "font"], "section": "headings", "control": "choice", "options": ["display", "body", "mono"]},
  {"path": ["heading", "h1", "color"], "section": "headings", "control": "color"},
```

(24 lines in all; the last entry of the array has no trailing comma.)

`theme_schema.rs`, in `Section` after `Typography`:

```rust
    /// Heading styles of a text, levels 1 to 6: `heading.*`.
    Headings,
```

`theme_panel.rs`:

```rust
const SECTIONS_BEFORE: [(Section, &str); 9] = [
    (Section::Colors, "colors"),
    (Section::Typography, "typography"),
    (Section::Headings, "headings"),
    (Section::Scripts, "scripts"),
    …
```

`session.ftl`, line after `theme-section-typography` in each locale:

| locale | value | locale | value |
|---|---|---|---|
| en | Headings | ar | العناوين |
| fr | Titres | hi | शीर्षक |
| de | Überschriften | bn | শিরোনাম |
| es | Títulos | ja | 見出し |
| it | Titoli | ko | 제목 |
| pt | Títulos | zh-Hans | 标题 |
| nl | Koppen | zh-Hant | 標題 |
| pl | Nagłówki | th | หัวเรื่อง |
| ru | Заголовки | vi | Tiêu đề |
| uk | Заголовки | tr | Başlıklar |

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo nextest run -p oku-slides-session -p oku-slides-i18n -p oku-slides-render`
Expected: PASS (`schema_covers_exactly_default_tokens_paths` and `schema_accepts_every_default_value` cover the new group).

- [ ] **Step 5: Commit** — `feat(session,typst): heading design tokens and theme editor section (t-1284, t-1289)`

---

### Task 6: Typst rendering of heading paragraphs (t-1290)

**Files:**
- Modify: `typst/oku-slides/src/content.typ:32-63` (`_runs`, `_paragraphs`)
- Modify: `typst/oku-slides/src/data.typ:73-81` (`_is-paragraphs`) and `crates/oku-slides-core/src/params.rs` doc of `is_paragraphs` if it names the keys
- Modify: `crates/oku-slides-render/tests/common/mod.rs` (move `pixels` and `is_red` here from `text_indent.rs`)
- Modify: `crates/oku-slides-render/tests/text_indent.rs` (use `common::pixels`, `common::is_red`)
- Create: `crates/oku-slides-render/tests/headings.rs`

**Interfaces:**
- Consumes: tokens `heading.hN` (Task 5); JSON key `heading` (Task 1).
- Produces: `common::pixels_of(&Compiled, usize) -> (Vec<[u8; 4]>, usize)`, `common::pixels(&Compiled)`, `common::is_red([u8; 4]) -> bool` for render tests.

- [ ] **Step 1: Move the pixel helpers** — cut `pixels` and `is_red` from `text_indent.rs` into `tests/common/mod.rs` as `pub fn` (doc comments kept, `use oku_slides_render::Compiled;` added there). The PNG decoding body becomes `pub fn pixels_of(c: &Compiled, page: usize) -> (Vec<[u8; 4]>, usize)` (`c.png(page, 1.0)`), and `pub fn pixels(c: &Compiled) -> (Vec<[u8; 4]>, usize) { pixels_of(c, 0) }`. Call them as `common::pixels` / `common::is_red` in `text_indent.rs`. Run `cargo nextest run -p oku-slides-render text_indent` — PASS.

- [ ] **Step 2: Write the failing tests** — `crates/oku-slides-render/tests/headings.rs`:

```rust
//! Paragraphes titres (t-1284) : taille et style des jetons `heading.hN`, hors du sommaire.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_core::kinds::{base, essentiel};
use oku_slides_render::Compiled;
use serde_json::{Value, json};
use speculoos::prelude::*;

/// Jetons de test : titre de niveau 2 rouge de 120 pt, puce verte.
fn tokens() -> Value {
    json!({ "heading": { "h2": { "size": 120, "color": "#ff0000", "weight": 800 } },
            "weight": { "strong": 300 }, "list": { "marker": "•", "marker-color": "#00ff00" } })
}

/// Les slides `before` puis une slide libre d'un texte de paragraphes `body` (cadre 1600 × 800).
fn compile(before: &[Value], body: &Value) -> Compiled {
    let mut slides = before.to_vec();
    slides.push(json!({ "id": "s", "kind": essentiel::BLANK, "params": { "elements": [
        { "id": "t", "kind": base::TEXT, "frame": { "x": 100, "y": 100, "w": 1600, "h": 800 },
          "params": { "body": body } }] } }));
    let d = json!({ "tokens": tokens(), "slides": slides });
    let deck = Deck::from_json(&d.to_string()).unwrap_or_else(|e| panic!("{e}"));
    let mut r = common::renderer();
    r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

/// Nombre de lignes de pixels de la dernière page qui ont un pixel vérifiant `pred`.
fn rows(c: &Compiled, pred: fn([u8; 4]) -> bool) -> usize {
    let (px, width) = common::pixels_of(c, c.page_count() - 1);
    px.chunks(width).filter(|row| row.iter().any(|p| pred(*p))).count()
}

/// Nombre de pixels de la dernière page qui vérifient `pred`.
fn ink(c: &Compiled, pred: fn([u8; 4]) -> bool) -> usize {
    let (px, _) = common::pixels_of(c, c.page_count() - 1);
    px.into_iter().filter(|p| pred(*p)).count()
}

fn is_green(p: [u8; 4]) -> bool {
    p[1] > 200 && p[0] < 80 && p[2] < 80
}

#[test]
fn heading_h2_renders_at_its_token_size() {
    let c = compile(&[], &json!([{ "runs": [{ "t": "H" }], "heading": 2 }]));
    assert_that!(rows(&c, common::is_red)).is_greater_than(60);
}

#[test]
fn paragraph_without_heading_renders_at_body_size() {
    let c = compile(&[], &json!([{ "runs": [{ "t": "H", "color": "#ff0000" }] }]));
    assert_that!(rows(&c, common::is_red)).is_less_than(40);
}

#[test]
fn heading_and_list_render_as_a_heading_without_marker() {
    let c = compile(&[], &json!([{ "runs": [{ "t": "H" }], "heading": 2, "list": "bullet" }]));
    assert_that!(rows(&c, common::is_red)).is_greater_than(60);
    assert_that!(ink(&c, is_green)).is_equal_to(0);
}

#[test]
fn bold_run_in_a_heading_is_never_lighter_than_the_heading() {
    let plain = ink(&compile(&[], &json!([{ "runs": [{ "t": "Haut" }], "heading": 2 }])), common::is_red);
    let bold = ink(&compile(&[], &json!([{ "runs": [{ "t": "Haut", "bold": true }], "heading": 2 }])), common::is_red);
    assert_that!(bold * 100).is_greater_than_or_equal_to(plain * 95);
}

#[test]
fn body_heading_leaves_the_toc_page_unchanged() {
    let before = [
        json!({ "id": "s-toc", "kind": essentiel::TOC }),
        json!({ "id": "s-ch", "kind": essentiel::CHAPTER, "params": { "title": "Chapitre" } }),
    ];
    let plain = compile(&before, &json!([{ "runs": [{ "t": "Intrus" }] }]));
    let heading = compile(&before, &json!([{ "runs": [{ "t": "Intrus" }], "heading": 1 }]));
    assert_that!(heading.page_hash(0)).is_equal_to(plain.page_hash(0));
}
```

- [ ] **Step 3: Run them to verify they fail**

Run: `cargo nextest run -p oku-slides-render headings`
Expected: FAIL — `heading_h2…` (heading key rejected by `_is-paragraphs`, so the body falls back to its default and no red ink), `heading_and_list…`, `bold_run…`; `body_heading_leaves_the_toc_page_unchanged` may already pass (it guards the decision; keep it).

- [ ] **Step 4: Implement**

`data.typ`:

```typst
// Paragraphes (côté Typst) : chaque objet n'a que des clés `runs`/`list`/`heading`, et
// ses `runs`, quand présents, sont un tableau de runs valides. Mirroir de
// `is_paragraphs` côté Rust.
#let _is-paragraphs(v) = type(v) == array and v.all(x => {
  type(x) == dictionary and x.keys().all(k => k in ("runs", "list", "heading")) and {
```

`content.typ`, `_runs` gains a `strong` argument (weight of a bold run):

```typst
#let _runs(t, runs, vars, strong: auto) = runs.map(r => {
  let body = [#interp(r.at("t", default: ""), vars)]
  let a = _text-args(t, r)
  if r.at("bold", default: false) {
    a.insert("weight", if strong == auto { weight-of(t, "strong") } else { strong })
  }
```

(rest unchanged), and `_paragraphs`:

```typst
// Style d'un titre de niveau `h` (1 à 6, jetons `heading.hN`), ou none hors de ce domaine.
#let _heading-style(t, h) = if type(h) == int and h >= 1 and h <= 6 { t.heading.at("h" + str(h)) }

// Des éléments (`list.item`, `enum.item`), pas des listes : Typst regroupe
// les éléments voisins en une seule liste, numérotée 1, 2, 3. Un titre est un
// paragraphe mis en forme, jamais un `heading` Typst : il resterait au sommaire
// et aux signets du PDF. Il l'emporte sur `list`.
#let _paragraphs(t, paras, vars) = {
  let out = ()
  for p in paras {
    let runs = p.at("runs", default: ())
    let h = _heading-style(t, p.at("heading", default: none))
    if h != none {
      let style = text-style(t, h)
      // Un run gras ne rend pas le titre plus maigre.
      let strong = weight-of(t, "strong")
      let w = style.at("weight", default: none)
      let strong = if type(w) == int and type(strong) == int { calc.max(w, strong) } else { strong }
      out.push(par(text(..style, _runs(t, runs, vars, strong: strong))))
      continue
    }
    let body = _runs(t, runs, vars)
    let kind = p.at("list", default: none)
    if kind == "bullet" { out.push(list.item(body)) }
    else if kind == "number" { out.push(enum.item(body)) }
    else { out.push(par(body)) }
  }
  out.join()
}
```

In `params.rs`, update the `PARAGRAPH_KEYS` doc if it lists keys; the Rust/Typst mirror comments stay true.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo nextest run -p oku-slides-render`
Expected: PASS (golden snapshots unchanged: no existing deck has `heading`).

- [ ] **Step 6: Commit** — `feat(typst,render): render heading paragraphs from heading tokens (t-1284, t-1290)`

---

### Task 7: Agent tool, hint, help, rules and e2e (t-1291)

**Files:**
- Modify: `crates/oku-slides-mcp/src/tools.rs:307` (`apply` description)
- Modify: `crates/oku-slides-i18n/src/locales/*/ui.ftl` (`ui-stage-hint-rich`, 20 locales)
- Modify: `docs/aide/en.json`, `docs/aide/fr.json` (editing bullet in block `b-editer`)
- Modify: `AGENTS.md:85` (settled decision)
- Modify: `crates/oku-slides-session/ARCHITECTURE.md:24` (markdown view mentions headings), `ui/AGENTS.md` only if a rule names `listPrefix`
- Modify: `ui/e2e/010-editor.cjs` (new step after step 7)

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Write the failing e2e step** — in `ui/e2e/010-editor.cjs`, right after step 7 (`await closed(p);` following `` `${BASE}\n- deux` ``):

```js
  // 7b. Un titre : « ## » met la ligne en titre, l'hôte le garde et le réécrit (t-1284).
  await open(p, 'Corps');
  await p.keyboard.press('ControlOrMeta+End');
  await p.keyboard.press('Enter');
  await p.keyboard.type('## Titre');
  assert.equal(await p.locator('.editor .cm-md-h2').count(), 1);
  await commitWith(p, () => p.locator('.editor .cm-content').evaluate((el) => el.blur()));
  assert.deepEqual(await points(p), [
    { runs: RUNS }, { runs: [{ t: 'deux' }], list: 'bullet' }, { runs: [{ t: 'Titre' }], heading: 2 },
  ]);
  await open(p, 'Corps');
  assert.equal(await text(p), `${BASE}\n- deux\n## Titre`);
  await p.keyboard.press('Escape');
  await closed(p);
```

- [ ] **Step 2: Run it** — `just e2e 4999` (background). Expected: PASS already if Tasks 1–6 are in (this step proves the end-to-end path); if it fails, debug before going on.

- [ ] **Step 3: Update the agent tool description** — in `tools.rs`, replace `list?: bullet|number;` with `list?: bullet|number; heading?: 1-6, a heading paragraph styled by the theme, which wins over list;`. Run `cargo nextest run -p oku-slides-mcp` — fix any test that pins the description text.

- [ ] **Step 4: Update the stage hint** — in each `ui.ftl`, insert a segment after the bullet segment of `ui-stage-hint-rich`, using the same quote marks as that locale's bullet segment and the noun of the table in Task 5 in its singular form. en:

```
ui-stage-hint-rich = Tab for next · ⌘ Enter to confirm · “- ” for a bullet · “# ” for a heading · ⌘B bold, ⌘I italic, ⌘K link
```

fr: `… · « - » pour une puce · « # » pour un titre · …`. Keep code tokens (`- `, `# `, `⌘B`) unchanged; ja/ko/zh without added spaces around CJK text.

- [ ] **Step 5: Update the help deck** — `docs/aide/en.json`, block `b-editer`: "One line per paragraph; a leading “- ” makes a bullet." → "One line per paragraph; a leading “- ” makes a bullet, “# ” to “###### ” a heading." `docs/aide/fr.json`: "Une ligne par paragraphe ; « - » en tête fait une puce." → "Une ligne par paragraphe ; « - » en tête fait une puce, « # » à « ###### » un titre." Run `cargo nextest run -p oku-slides-session help` — PASS.

- [ ] **Step 6: Update the rules and architecture docs** — root `AGENTS.md:85`:

```
- Keep the editing markdown syntax fixed: bold, italic, links, ==highlight== with optional {n} 0-255, # to ###### headings, - and 1. lists. (plan:2026-09-27-markdown-lezer, plan:2026-10-30-titres-markdown)
```

`crates/oku-slides-session/ARCHITECTURE.md:24`: "markdown (`text.rs`: headings, lists; `inline.rs`)". Add a Typst rule to `typst/oku-slides/AGENTS.md` under Invariants:

```
- Render a heading paragraph as a styled par, never a Typst heading, so it stays out of the TOC and PDF bookmarks. (plan:2026-10-30-titres-markdown)
```

- [ ] **Step 7: Full gate**

Run: `just check` then `just test` (background)
Expected: PASS (agents-lint accepts the new rules; tsc, clippy, fmt clean).

- [ ] **Step 8: Commit** — `feat(mcp,ui,docs): document headings for the agent, hint and help; e2e (t-1284, t-1291)`
