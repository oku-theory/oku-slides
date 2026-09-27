# Markdown d'édition via @lezer/markdown — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** le markdown d'édition est lu par `@lezer/markdown` (sémantique
CommonMark) et écrit par un sérialiseur Rust à pile qui ne perd jamais de
texte.

**Architecture :** côté TS, `markdown.ts` parcourt l'arbre Lezer (blocs
désactivés, extension `==pastille==` à règles permissives) et en tire runs et
plages. Côté Rust, un nouveau module `inline.rs` normalise les bords des runs
puis écrit l'inline avec une pile d'emphases. Le contrat est tenu par deux
fixtures : `markdown.json` (cas écrits à la main) et `markdown-corpus.json`
(corpus aléatoire généré par Rust, relu par TS). `textToParagraphs` garde un
paragraphe dont la ligne n'a pas changé.

**Tech Stack :** TypeScript, Bun test, `@lezer/markdown` 1.7, `@lezer/common`
1.5 ; Rust 2024, nextest, speculoos.

**Spec :** `docs/superpowers/specs/2026-09-27-markdown-lezer-design.md`
(tâche t-180 ; sous-tâches t-183 à t-187, une par Task).

## Global Constraints

- Syntaxe inchangée : `**gras**`, `*italique*` / `_italique_`, `***les deux***`, `[texte](url)`, `==pastille==`, `==pastille=={n}` (n de 0 à 255), `- ` et `1. ` en début de ligne.
- Aucune perte de texte à l'aller-retour ; perte de mise en forme permise seulement sur espaces et ponctuation.
- Pas de nouvelle dépendance Rust. En TS, seulement `@lezer/markdown` et `@lezer/common`, pas de `@codemirror/lang-markdown`.
- Pas de `.unwrap()` / `.expect()` hors tests (`unwrap_used = "deny"`).
- Fichiers : 500 lignes visées, 1000 au maximum. Tests à côté des sources (`foo_tests.rs`, `foo.test.ts`).
- Commits : `<type>(<scope>) : <description> (t-180)`, sans ligne `Co-Authored-By`.
- Commentaires et docs en français, comme le reste du dépôt.

---

### Task 1 : lecture du markdown par @lezer/markdown (TS)

**Files :**
- Create : `ui/src/markdownHighlight.ts`
- Modify : `ui/src/markdown.ts` (tout sauf `listPrefix`, `encodeLink`, `sameFormatting`)
- Modify : `ui/package.json`, `ui/bun.lock` (via `bun add`)
- Test : `ui/src/markdown.test.ts`

**Interfaces :**
- Consumes : rien.
- Produces : `Highlight: MarkdownConfig` (`markdownHighlight.ts`). Signatures
  inchangées dans `markdown.ts` : `markdownToRuns(line: string): Run[]`,
  `markdownSpans(line: string): Span[]`, `listPrefix(line)`,
  `encodeLink(url)`, `sameFormatting(a, b)` (supprimée à la Task 4).

- [ ] **Step 1 : ajouter les dépendances**

```bash
cd ui && bun add @lezer/markdown@^1.7.2 @lezer/common@^1.5.0
```

Expected : `package.json` gagne `"@lezer/common": "^1.5.0"` et
`"@lezer/markdown": "^1.7.2"` dans `dependencies`.

- [ ] **Step 2 : écrire les tests qui échouent**

Dans `ui/src/markdown.test.ts`, remplacer le test « triple étoile » de
`describe("markdownSpans")` par :

```ts
  test("triple étoile : CommonMark imbrique l'italique autour du gras", () => {
    expect(markdownSpans("***x***")).toEqual([
      { from: 0, to: 1, kind: "mark" },
      { from: 1, to: 6, kind: "italic" },
      { from: 1, to: 3, kind: "mark" },
      { from: 3, to: 4, kind: "bold" },
      { from: 4, to: 6, kind: "mark" },
      { from: 6, to: 7, kind: "mark" },
    ]);
  });
```

Puis ajouter à la fin de `describe("markdownToRuns")` :

```ts
  test("délimiteur suivi d'une espace : pas de gras (CommonMark)", () => {
    expect(markdownToRuns("** x**")).toEqual([{ t: "** x**" }]);
  });

  test("étoile contre une ponctuation, après une lettre : pas d'italique (CommonMark)", () => {
    expect(markdownToRuns("a*!b*")).toEqual([{ t: "a*!b*" }]);
  });

  test("gras-italique puis italique : l'italique partagé reste ouvert", () => {
    expect(markdownToRuns("***X**Y*")).toEqual([
      { t: "X", bold: true, italic: true },
      { t: "Y", italic: true },
    ]);
  });

  test("pastille en gras collée à du japonais", () => {
    expect(markdownToRuns("上==**a**=={2}です")).toEqual([
      { t: "上" },
      { t: "a", bold: true, highlight: 2 },
      { t: "です" },
    ]);
  });

  test("pastille dont le texte est un égal échappé", () => {
    expect(markdownToRuns("==\\===")).toEqual([{ t: "=", highlight: 1 }]);
  });

  test("lien sans adresse : texte littéral", () => {
    for (const s of ["[a]", "[a][b]", "[a]()", "voir [note] ici"]) {
      expect(markdownToRuns(s)).toEqual([{ t: s }]);
    }
  });

  test("titres, citations, code, html et entités restent du texte", () => {
    for (const s of ["# titre", "> citation", "    indenté", "`code`", "<b>x</b>", "&amp;", "<https://a.b>", "***"]) {
      expect(markdownToRuns(s)).toEqual([{ t: s }]);
    }
  });
```

- [ ] **Step 3 : vérifier l'échec**

Run : `cd ui && bun test src/markdown.test.ts`
Expected : FAIL sur « triple étoile », « délimiteur suivi d'une espace » et
« étoile contre une ponctuation ». L'ancien parseur lit `** x**` en gras et
`a*!b*` en italique.

- [ ] **Step 4 : créer `ui/src/markdownHighlight.ts`**

```ts
/**
 * Extension @lezer/markdown des pastilles : `==texte==`.
 *
 * `==` n'est pas du CommonMark : ses règles sont les nôtres, plus simples
 * que le « flanking » de `*`. Il ouvre s'il n'est pas suivi d'une espace et
 * ferme s'il n'est pas précédé d'une espace, si bien qu'une pastille en gras
 * collée à une lettre (`上==**a**==です`) s'écrit. Une suite de trois `=`
 * n'ouvre ni ne ferme rien ; un `\=` échappé juste avant ne compte pas.
 * La couleur `{n}` qui suit la fermeture est lue par `markdown.ts`.
 */
import type { MarkdownConfig } from "@lezer/markdown";

const EQUALS = 61;
const BACKSLASH = 92;
const DELIMITER = { resolve: "Highlight", mark: "HighlightMark" };

/** Nœuds `Highlight` et `HighlightMark` pour `parser.configure`. */
export const Highlight: MarkdownConfig = {
  defineNodes: ["Highlight", "HighlightMark"],
  parseInline: [
    {
      name: "Highlight",
      parse(cx, next, pos) {
        if (next !== EQUALS || cx.char(pos + 1) !== EQUALS) return -1;
        const equalsBefore = cx.char(pos - 1) === EQUALS && cx.char(pos - 2) !== BACKSLASH;
        if (equalsBefore || cx.char(pos + 2) === EQUALS) return -1;
        const spaceBefore = /\s|^$/.test(cx.slice(pos - 1, pos));
        const spaceAfter = /\s|^$/.test(cx.slice(pos + 2, pos + 3));
        return cx.addDelimiter(DELIMITER, pos, pos + 2, !spaceAfter, !spaceBefore);
      },
      after: "Emphasis",
    },
  ],
};
```

- [ ] **Step 5 : réécrire le parcours dans `ui/src/markdown.ts`**

Remplacer le commentaire d'en-tête, les imports, et tout ce qui va de
`const PUNCTUATION` jusqu'à la fin de `markdownSpans` (helpers `isWord`,
`repeated`, `scan`, `closing`, `closingUnderscore`, `link`, `parse`). Garder
`interface Format`, `SpanKind`, `Span`, `run`, `merge`, puis `listPrefix`,
`encodeLink` et `sameFormatting` tels quels. Le début du fichier devient :

```ts
/**
 * Markdown inline d'édition → runs : l'inverse de `runs_to_markdown` côté Rust.
 *
 * Analyse CommonMark de `@lezer/markdown`, blocs désactivés (une ligne de
 * l'éditeur est un paragraphe) : `**gras**`, `*italique*` ou `_italique_`,
 * `[texte](url)`, `\` devant une ponctuation, plus les pastilles
 * `==texte==` / `==texte=={n}` (n-ième couleur de la palette du thème, voir
 * `markdownHighlight.ts`). Le même parcours de l'arbre donne les plages
 * positionnées que l'éditeur met en forme (`markdownSpans`) ; `listPrefix`
 * est la règle unique des préfixes de liste.
 */
import type { SyntaxNode } from "@lezer/common";
import { parser as commonmark } from "@lezer/markdown";
import type { Paragraph, Run } from "./api";
import { Highlight } from "./markdownHighlight";
```

Après `merge`, insérer :

```ts
/** Blocs et inlines CommonMark hors du dialecte d'édition. */
const DISABLED = [
  "LinkReference",
  "IndentedCode",
  "FencedCode",
  "Blockquote",
  "HorizontalRule",
  "BulletList",
  "OrderedList",
  "ATXHeading",
  "HTMLBlock",
  "SetextHeading",
  "HTMLTag",
  "Autolink",
  "InlineCode",
  "Image",
  "Entity",
  "HardBreak",
];

const parser = commonmark.configure([Highlight, { remove: DISABLED }]);

/** `{0}`..`{255}` : la couleur est un u8 côté Rust ({0} prend la dernière, comme Typst). */
const COLOR = /^\{(\d|[1-9]\d|1\d\d|2[0-4]\d|25[0-5])\}/;

/** Runs et plages d'une ligne, remplis par le parcours de l'arbre. */
interface Walk {
  line: string;
  runs: Run[];
  spans: Span[];
}

function span(w: Walk, from: number, to: number, kind: SpanKind, highlight?: number) {
  w.spans.push(highlight === undefined ? { from, to, kind } : { from, to, kind, highlight });
}

/** Enfants de `node` compris dans `[from, to]` ; le texte entre eux est du texte au format `f`. */
function inline(w: Walk, node: SyntaxNode, from: number, to: number, f: Format) {
  let pos = from;
  for (let c = node.firstChild; c; c = c.nextSibling) {
    if (c.from < pos || c.to > to) continue;
    w.runs.push(run(w.line.slice(pos, c.from), f));
    pos = element(w, c, f);
  }
  w.runs.push(run(w.line.slice(pos, to), f));
}

/** Un nœud inline : ajoute runs et plages, rend la position où reprend le texte. */
function element(w: Walk, n: SyntaxNode, f: Format): number {
  switch (n.name) {
    case "Escape":
      span(w, n.from, n.from + 1, "mark");
      w.runs.push(run(w.line.slice(n.from + 1, n.to), f));
      return n.to;
    case "Emphasis":
    case "StrongEmphasis": {
      const open = n.firstChild;
      const close = n.lastChild;
      if (!open || !close) return n.from;
      const kind = n.name === "Emphasis" ? "italic" : "bold";
      span(w, open.from, open.to, "mark");
      span(w, open.to, close.from, kind);
      inline(w, n, open.to, close.from, kind === "bold" ? { ...f, bold: true } : { ...f, italic: true });
      span(w, close.from, close.to, "mark");
      return n.to;
    }
    case "Highlight": {
      const open = n.firstChild;
      const close = n.lastChild;
      if (!open || !close) return n.from;
      const color = COLOR.exec(w.line.slice(n.to));
      const highlight = color ? Number(color[1]) : 1;
      const end = n.to + (color?.[0].length ?? 0);
      span(w, open.from, open.to, "mark");
      span(w, open.to, close.from, "highlight", highlight);
      inline(w, n, open.to, close.from, { ...f, highlight });
      span(w, close.from, end, "mark");
      return end;
    }
    case "Link": {
      // Sans adresse (`[a]`, `[a][b]`, `[a]()`), ce n'est pas un lien : texte littéral.
      const url = n.getChild("URL");
      const marks = n.getChildren("LinkMark");
      if (!url || marks.length < 2) return n.from;
      const [open, close] = marks;
      span(w, open.from, open.to, "mark");
      span(w, open.to, close.from, "link");
      inline(w, n, open.to, close.from, { ...f, link: w.line.slice(url.from, url.to) });
      span(w, close.from, url.from, "mark");
      span(w, url.from, url.to, "url");
      span(w, url.to, n.to, "mark");
      return n.to;
    }
    default:
      // Document, Paragraph : conteneurs transparents.
      inline(w, n, n.from, n.to, f);
      return n.to;
  }
}

function walk(line: string): Walk {
  const w: Walk = { line, runs: [], spans: [] };
  inline(w, parser.parse(line).topNode, 0, line.length, { bold: false, italic: false });
  return w;
}

/** Une ligne de markdown inline en runs ; une ligne vide donne un run vide. */
export function markdownToRuns(line: string): Run[] {
  const runs = merge(walk(line).runs);
  return runs.length > 0 ? runs : [{ t: "" }];
}

/** Plages non vides d'une ligne, l'englobante avant l'englobée. */
export function markdownSpans(line: string): Span[] {
  return walk(line)
    .spans.filter((s) => s.to > s.from)
    .sort((a, b) => a.from - b.from || b.to - a.to);
}
```

- [ ] **Step 6 : vérifier que tout passe**

Run : `cd ui && bun test && ./node_modules/.bin/tsc -b`
Expected : PASS pour tous les tests, dont le fixture partagé, `markdownDecorations.test.ts`
et `deck.test.ts`. `tsc` sans erreur.

- [ ] **Step 7 : commit**

```bash
git add ui/package.json ui/bun.lock ui/src/markdown.ts ui/src/markdownHighlight.ts ui/src/markdown.test.ts
git commit -m "refactor(ui) : markdown d'édition lu par @lezer/markdown (t-180)"
oku cli task done --project slider --id t-183
```

---

### Task 2 : sérialiseur Rust à pile d'emphases et bords normalisés

**Files :**
- Create : `crates/oku-slides-session/src/inline.rs`
- Create : `crates/oku-slides-session/src/inline_tests.rs`
- Modify : `crates/oku-slides-session/src/text.rs` (retire `runs_to_markdown` et `escape_inline`)
- Modify : `crates/oku-slides-session/src/lib.rs:34` (déclare `mod inline;`)
- Modify : `crates/oku-slides-session/src/text_tests.rs`
- Modify : `fixtures/markdown.json` (nouveaux cas, section `lossy`)
- Test : `ui/src/markdown.test.ts` (relecture de `lossy`)

**Interfaces :**
- Consumes : `markdownToRuns` (Task 1) via `textToParagraphs` dans les tests TS.
- Produces : `pub(crate) fn runs_to_markdown(runs: &[Run]) -> String` dans
  `inline.rs`, appelée par `text.rs`. Section `lossy` du fixture :
  `[{ name, paragraphs, markdown, reread }]`.

- [ ] **Step 1 : écrire les tests Rust qui échouent**

Créer `crates/oku-slides-session/src/inline_tests.rs` :

```rust
use oku_slides_core::Run;
use speculoos::prelude::*;

use super::*;

fn run(t: &str) -> Run {
    Run::plain(t)
}

fn bold(t: &str) -> Run {
    Run { bold: true, ..Run::plain(t) }
}

fn italic(t: &str) -> Run {
    Run { italic: true, ..Run::plain(t) }
}

#[test]
fn runs_to_markdown_shared_italic_stays_open() {
    let runs = [Run { italic: true, ..bold("X") }, italic("Y")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("***X**Y*".to_owned());
}

#[test]
fn runs_to_markdown_bold_inside_italic_nests() {
    let runs = [italic("a"), Run { bold: true, ..italic("b") }, italic("c")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("*a**b**c*".to_owned());
}

#[test]
fn runs_to_markdown_bold_highlight_glued_to_letters_keeps_both() {
    let runs = [run("上"), Run { highlight: Some(2), ..bold("a") }, run("です")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("上==**a**=={2}です".to_owned());
}

#[test]
fn runs_to_markdown_space_at_bold_edge_moves_outside() {
    let runs = [run("a"), bold(" x "), run("b")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("a **x** b".to_owned());
}

#[test]
fn runs_to_markdown_punctuation_glued_to_letter_moves_outside() {
    let runs = [run("a"), italic("\"x\""), run("b")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("a\"*x*\"b".to_owned());
}

#[test]
fn runs_to_markdown_punctuation_between_emphases_loses_bold() {
    let runs = [bold("-"), italic("a")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("-*a*".to_owned());
}

#[test]
fn runs_to_markdown_space_only_highlight_is_plain() {
    let runs = [run("a"), Run { highlight: Some(2), ..run(" ") }, run("b")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("a b".to_owned());
}

#[test]
fn runs_to_markdown_spaces_in_link_stay_in_brackets() {
    let runs = [Run { link: Some("https://a.b".into()), ..bold(" x ") }];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("[ **x** ](https://a.b)".to_owned());
}

#[test]
fn runs_to_markdown_italic_highlight_between_italics_closes_italic() {
    let runs = [italic("a"), Run { highlight: Some(1), ..italic("b") }, run("c")];
    assert_that!(runs_to_markdown(&runs)).is_equal_to("*a*==*b*==c".to_owned());
}
```

- [ ] **Step 2 : ajouter les cas au fixture**

Dans `fixtures/markdown.json`, ajouter à la fin de `cases` :

```json
    {
      "name": "texte qui ressemble à de la mise en forme",
      "markdown": "a \\*b\\* c \\[x\\](y)",
      "paragraphs": [{ "runs": [{ "t": "a *b* c [x](y)" }] }]
    },
    {
      "name": "titre et citation en début de ligne",
      "markdown": "# pas un titre\n> pas une citation",
      "paragraphs": [{ "runs": [{ "t": "# pas un titre" }] }, { "runs": [{ "t": "> pas une citation" }] }]
    },
    {
      "name": "ponctuation française",
      "markdown": "Bonjour ! Ça va ?",
      "paragraphs": [{ "runs": [{ "t": "Bonjour ! Ça va ?" }] }]
    },
    {
      "name": "pastille réduite à un égal",
      "markdown": "==\\===",
      "paragraphs": [{ "runs": [{ "t": "=", "highlight": 1 }] }]
    },
    {
      "name": "italique puis gras collés",
      "markdown": "*a***b**",
      "paragraphs": [{ "runs": [{ "t": "a", "italic": true }, { "t": "b", "bold": true }] }]
    },
    {
      "name": "gras-italique puis italique",
      "markdown": "***X**Y*",
      "paragraphs": [{ "runs": [{ "t": "X", "bold": true, "italic": true }, { "t": "Y", "italic": true }] }]
    },
    {
      "name": "pastille en gras collée à du japonais",
      "markdown": "上==**a**=={2}です",
      "paragraphs": [{ "runs": [{ "t": "上" }, { "t": "a", "bold": true, "highlight": 2 }, { "t": "です" }] }]
    }
```

Et après `cases`, une nouvelle section :

```json
  "lossy": [
    {
      "name": "espaces en bord de gras",
      "paragraphs": [{ "runs": [{ "t": "a" }, { "t": " x ", "bold": true }, { "t": "b" }] }],
      "markdown": "a **x** b",
      "reread": [{ "runs": [{ "t": "a " }, { "t": "x", "bold": true }, { "t": " b" }] }]
    },
    {
      "name": "ponctuation collée à une lettre",
      "paragraphs": [{ "runs": [{ "t": "a" }, { "t": "\"x\"", "italic": true }, { "t": "b" }] }],
      "markdown": "a\"*x*\"b",
      "reread": [{ "runs": [{ "t": "a\"" }, { "t": "x", "italic": true }, { "t": "\"b" }] }]
    },
    {
      "name": "pastille réduite à une espace",
      "paragraphs": [{ "runs": [{ "t": "a" }, { "t": " ", "highlight": 2 }, { "t": "b" }] }],
      "markdown": "a b",
      "reread": [{ "runs": [{ "t": "a b" }] }]
    },
    {
      "name": "gras sur un tiret collé à de l'italique",
      "paragraphs": [{ "runs": [{ "t": "-", "bold": true }, { "t": "a", "italic": true }] }],
      "markdown": "-*a*",
      "reread": [{ "runs": [{ "t": "-" }, { "t": "a", "italic": true }] }]
    }
  ]
```

Mettre à jour `_comment` : « Aller-retour paragraphes ↔ markdown d'édition :
paragraphs_to_text (Rust) écrit `markdown`, textToParagraphs (TS) le relit en
`paragraphs`. `lossy` : perte documentée des bords, relue en `reread`. »

- [ ] **Step 3 : ajouter les tests de fixture (Rust et TS)**

Dans `crates/oku-slides-session/src/text_tests.rs`, remplacer `cases()` par
une lecture commune et ajouter le test `lossy` :

```rust
fn fixture() -> serde_json::Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/markdown.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("fixture {path} illisible : {e}"));
    serde_json::from_str(&json).unwrap()
}

fn cases() -> Vec<Case> {
    serde_json::from_value(fixture()["cases"].clone()).unwrap()
}

#[test]
fn paragraphs_to_text_lossy_cases_give_their_markdown() {
    let lossy: Vec<Case> = serde_json::from_value(fixture()["lossy"].clone()).unwrap();
    for c in lossy {
        asserting(&c.name).that(&paragraphs_to_text(&c.paragraphs)).is_equal_to(c.markdown);
    }
}
```

Dans `ui/src/markdown.test.ts`, remplacer les lignes 7 à 14 (interface
`Case`, `fixture`, `cases`) par :

```ts
interface Case {
  name: string;
  markdown: string;
  paragraphs: Paragraph[];
}

interface Lossy extends Case {
  reread: Paragraph[];
}

const fixture = JSON.parse(readFileSync(new URL("../../fixtures/markdown.json", import.meta.url), "utf8"));
const cases: Case[] = fixture.cases;
const lossy: Lossy[] = fixture.lossy;
```

et ajouter après le `describe` de la fixture :

```ts
describe("perte documentée : bords des runs délimités", () => {
  for (const c of lossy) {
    test(`${c.name} se relit sans ses bords`, () => {
      expect(textToParagraphs(c.markdown)).toEqual(c.reread);
    });
  }
});
```

- [ ] **Step 4 : vérifier l'échec**

Run : `cargo nextest run -p oku-slides-session inline text`
Expected : FAIL. `inline_tests.rs` n'est pas encore compilé (le module
`inline` n'existe pas avant le Step 5), mais les tests de fixture échouent :
`paragraphs_to_text_fixture_cases_give_their_markdown` échoue sur
« gras-italique puis italique » (`***X****Y*` au lieu de `***X**Y*`) et
`paragraphs_to_text_lossy_cases_give_their_markdown` sur
« espaces en bord de gras » (`a** x **b`).

Run : `cd ui && bun test src/markdown.test.ts`
Expected : PASS. Le lecteur Lezer de la Task 1 relit déjà ces markdowns.

- [ ] **Step 5 : créer `crates/oku-slides-session/src/inline.rs`**

```rust
//! Runs → markdown inline d'édition, relu par `@lezer/markdown` (TS).
//!
//! Les emphases partagées par des runs voisins restent ouvertes (pile, la
//! plus longue à l'extérieur) ; pastilles et liens portent leurs propres
//! délimiteurs. Avant d'écrire, les bords que CommonMark ne lirait pas
//! comme écrits sortent de leurs délimiteurs : seule de la mise en forme
//! sur des espaces et de la ponctuation peut s'y perdre, jamais du texte.
//! Contrat : `fixtures/markdown.json` et `fixtures/markdown-corpus.json`.

use oku_slides_core::Run;

/// Runs en markdown inline ; `color` et `size` n'ont pas d'écriture markdown.
pub(crate) fn runs_to_markdown(runs: &[Run]) -> String {
    let mut runs = runs.to_vec();
    merge(&mut runs);
    while let Some(fix) = violation(&runs) {
        apply(&mut runs, fix);
    }
    let steps = steps(&runs);
    let mut out = String::new();
    let mut after_highlight = false;
    for (i, r) in runs.iter().enumerate() {
        let step = &steps[i];
        for m in step.close.iter().chain(&step.open) {
            out.push_str(m.delimiter());
        }
        if !step.open.is_empty() {
            after_highlight = false;
        }
        if barrier(r) {
            out.push_str(&write_barrier(r, runs.get(i + 1)));
            after_highlight = r.link.is_none();
        } else {
            let text = escape_inline(&r.t);
            // `{n}` juste après une pastille serait lu comme sa couleur.
            if after_highlight && text.starts_with('{') {
                out.push('\\');
            }
            out.push_str(&text);
            after_highlight = false;
        }
    }
    for m in &steps[runs.len()].close {
        out.push_str(m.delimiter());
    }
    out
}

/// Emphase écrite avec `*` : gras ou italique.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mark {
    Bold,
    Italic,
}

impl Mark {
    const ALL: [Mark; 2] = [Mark::Bold, Mark::Italic];

    fn delimiter(self) -> &'static str {
        match self {
            Mark::Bold => "**",
            Mark::Italic => "*",
        }
    }

    fn of(self, r: &Run) -> bool {
        match self {
            Mark::Bold => r.bold,
            Mark::Italic => r.italic,
        }
    }
}

/// Une pastille ou un lien a ses propres délimiteurs : l'emphase ne le traverse pas.
fn barrier(r: &Run) -> bool {
    r.link.is_some() || r.highlight.is_some()
}

/// Emphases fermées puis ouvertes avant un run.
#[derive(Default, Debug)]
struct Step {
    close: Vec<Mark>,
    /// Emphases encore ouvertes entre les fermetures et les ouvertures.
    kept: Vec<Mark>,
    open: Vec<Mark>,
}

/// Transitions de la pile : `steps[i]` précède le run `i`, `steps[n]` clôt la ligne.
fn steps(runs: &[Run]) -> Vec<Step> {
    let mut stack: Vec<Mark> = Vec::new();
    let mut out = Vec::with_capacity(runs.len() + 1);
    for (i, r) in runs.iter().enumerate() {
        let mut step = Step::default();
        if let Some(depth) = stack.iter().position(|&m| barrier(r) || !m.of(r)) {
            step.close = stack.drain(depth..).rev().collect();
        }
        step.kept = stack.clone();
        if !barrier(r) {
            // La plus longue à l'extérieur : elle reste ouverte le plus longtemps.
            let span = |m: Mark| runs[i..].iter().take_while(|r| !barrier(r) && m.of(r)).count();
            let mut open: Vec<Mark> = Mark::ALL.into_iter().filter(|&m| m.of(r) && !stack.contains(&m)).collect();
            open.sort_by_key(|&m| std::cmp::Reverse(span(m)));
            stack.extend(&open);
            step.open = open;
        }
        out.push(step);
    }
    out.push(Step { close: stack.into_iter().rev().collect(), ..Step::default() });
    out
}

/// Nature d'un caractère pour les règles de délimiteurs de CommonMark.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    /// Espace, ou bord de ligne.
    Space,
    Punct,
    Alnum,
}

fn class(c: Option<char>) -> Class {
    match c {
        None => Class::Space,
        Some(c) if c.is_whitespace() => Class::Space,
        Some(c) if c.is_alphanumeric() => Class::Alnum,
        Some(_) => Class::Punct,
    }
}

/// Caractère à sortir de ses délimiteurs : premier ou dernier du run `run`.
struct Fix {
    run: usize,
    first: bool,
    /// Emphases restées ouvertes autour : le caractère sorti les garde.
    kept: Vec<Mark>,
}

/// Premier caractère de bord que CommonMark ne lirait pas comme écrit.
fn violation(runs: &[Run]) -> Option<Fix> {
    // Pastille hors lien : pas d'espace contre `==` (voir markdownHighlight.ts).
    for (i, r) in runs.iter().enumerate() {
        if r.highlight.is_some() && r.link.is_none() {
            if class(r.t.chars().next()) == Class::Space {
                return Some(Fix { run: i, first: true, kept: Vec::new() });
            }
            if class(r.t.chars().last()) == Class::Space {
                return Some(Fix { run: i, first: false, kept: Vec::new() });
            }
        }
    }
    for (k, step) in steps(runs).into_iter().enumerate() {
        // Crochets, `==` et `{n}` des barrières comptent comme de la ponctuation.
        let a = match k.checked_sub(1).map(|j| &runs[j]) {
            Some(r) if barrier(r) => Class::Punct,
            Some(r) => class(r.t.chars().last()),
            None => Class::Space,
        };
        let b = match runs.get(k) {
            Some(r) if barrier(r) => Class::Punct,
            Some(r) => class(r.t.chars().next()),
            None => Class::Space,
        };
        // Fermetures et ouvertures sont des `*` : une seule suite entre `a` et `b`,
        // qui doit pouvoir fermer (côté `a`) et ouvrir (côté `b`).
        if !step.close.is_empty() && (a == Class::Space || (a == Class::Punct && b == Class::Alnum)) {
            return Some(Fix { run: k - 1, first: false, kept: step.kept });
        }
        if !step.open.is_empty() && (b == Class::Space || (b == Class::Punct && a == Class::Alnum)) {
            return Some(Fix { run: k, first: true, kept: step.kept });
        }
    }
    None
}

/// Sort le caractère : il devient un run avec les seules emphases `kept`.
/// Chaque déplacement retire de la mise en forme, la normalisation termine.
fn apply(runs: &mut Vec<Run>, fix: Fix) {
    let Fix { run, first, kept } = fix;
    let r = &mut runs[run];
    let c = if first { r.t.chars().next() } else { r.t.chars().last() };
    let Some(c) = c else { return };
    if first {
        r.t.drain(..c.len_utf8());
    } else {
        r.t.pop();
    }
    let moved = Run {
        bold: kept.contains(&Mark::Bold),
        italic: kept.contains(&Mark::Italic),
        ..Run::plain(c)
    };
    runs.insert(if first { run } else { run + 1 }, moved);
    merge(runs);
}

/// Runs voisins de même format fusionnés, runs vides retirés.
fn merge(runs: &mut Vec<Run>) {
    let same =
        |a: &Run, b: &Run| a.bold == b.bold && a.italic == b.italic && a.link == b.link && a.highlight == b.highlight;
    let mut out: Vec<Run> = Vec::with_capacity(runs.len());
    for r in runs.drain(..).filter(|r| !r.t.is_empty()) {
        match out.last_mut() {
            Some(last) if same(last, &r) => last.t.push_str(&r.t),
            _ => out.push(r),
        }
    }
    *runs = out;
}

/// Pastille ou lien, avec ses propres emphases à l'intérieur.
fn write_barrier(r: &Run, next: Option<&Run>) -> String {
    // Dans un lien, les espaces de bord restent dans les crochets, hors délimiteurs.
    let core = r.t.trim_matches(char::is_whitespace);
    let start = r.t.len() - r.t.trim_start_matches(char::is_whitespace).len();
    let (lead, trail) = (&r.t[..start], &r.t[start + core.len()..]);
    let marks = match (r.bold, r.italic) {
        (true, true) => "***",
        (true, false) => "**",
        (false, true) => "*",
        (false, false) => "",
    };
    let mut body = if core.is_empty() { String::new() } else { format!("{marks}{}{marks}", escape_inline(core)) };
    if let Some(n) = r.highlight.filter(|_| !core.is_empty()) {
        // Une pastille collée à la suivante garde sa couleur, même `{1}` :
        // sinon `==a====b==` se lirait comme une seule pastille.
        let before_highlight = r.link.is_none() && next.is_some_and(|r| r.highlight.is_some());
        let color = if n == 1 && !before_highlight { String::new() } else { format!("{{{n}}}") };
        body = format!("=={body}=={color}");
    }
    let body = format!("{}{body}{}", escape_inline(lead), escape_inline(trail));
    match &r.link {
        Some(url) => {
            let url = url.replace(' ', "%20").replace('(', "%28").replace(')', "%29");
            format!("[{body}]({url})")
        }
        None => body,
    }
}

/// Échappe ce qui se lirait comme du markdown ; `_` au milieu d'un mot et
/// `=` isolé restent tels quels.
fn escape_inline(t: &str) -> String {
    let chars: Vec<char> = t.chars().collect();
    let mut out = String::with_capacity(t.len());
    for (i, &c) in chars.iter().enumerate() {
        let intraword = || {
            let alnum = |j: Option<usize>| j.and_then(|j| chars.get(j)).is_some_and(|c| c.is_alphanumeric());
            alnum(i.checked_sub(1)) && alnum(Some(i + 1))
        };
        // `=` contre un autre `=` ou au bord du run pourrait former `==`.
        let equals = || {
            [i.checked_sub(1), Some(i + 1)].into_iter().any(|j| j.and_then(|j| chars.get(j)).is_none_or(|&n| n == '='))
        };
        if matches!(c, '\\' | '*' | '[' | ']') || (c == '_' && !intraword()) || (c == '=' && equals()) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
#[path = "inline_tests.rs"]
mod inline_tests;
```

- [ ] **Step 6 : brancher `inline.rs`**

Dans `crates/oku-slides-session/src/lib.rs`, après `mod files;` :

```rust
mod inline;
```

Dans `crates/oku-slides-session/src/text.rs` : supprimer `runs_to_markdown`
et `escape_inline` (lignes 38 à 101), ajouter `use crate::inline::runs_to_markdown;`
aux imports, retirer `Run` de `use oku_slides_core::{ListKind, Paragraph, Run};`,
et remplacer la doc de `paragraphs_to_text` par :

```rust
/// Texte éditable en markdown : un paragraphe par ligne, `- ` pour une puce,
/// `1. ` pour une liste numérotée, le reste en markdown inline (`inline.rs`).
```

La doc du module (`//!`) devient : « Texte affiché dans l'interface :
paragraphes en markdown éditable (l'inline est écrit par `inline.rs`), et
variables du deck remplacées. »

- [ ] **Step 7 : vérifier que tout passe**

Run : `cargo nextest run -p oku-slides-session && cargo clippy -p oku-slides-session --all-targets -- -D warnings`
Expected : PASS, dont les 9 tests `inline_tests` et les deux tests de fixture. Aucun avertissement.

Run : `cd ui && bun test`
Expected : PASS.

- [ ] **Step 8 : commit**

```bash
git add crates/oku-slides-session/src/inline.rs crates/oku-slides-session/src/inline_tests.rs crates/oku-slides-session/src/text.rs crates/oku-slides-session/src/text_tests.rs crates/oku-slides-session/src/lib.rs fixtures/markdown.json ui/src/markdown.test.ts
git commit -m "refactor(session) : markdown inline écrit avec une pile d'emphases, bords normalisés (t-180)"
oku cli task done --project slider --id t-184
```

---

### Task 3 : test croisé sur un corpus aléatoire (Rust → TS)

**Files :**
- Create : `fixtures/markdown-corpus.json` (généré)
- Modify : `crates/oku-slides-session/src/inline_tests.rs`
- Test : `ui/src/markdown.test.ts`

**Interfaces :**
- Consumes : `runs_to_markdown` (Task 2), `markdownToRuns` (Task 1).
- Produces : `fixtures/markdown-corpus.json` :
  `{ "_comment": string, "cases": [{ "runs": Run[], "markdown": string }] }`.

- [ ] **Step 1 : écrire le test Rust qui échoue**

À la fin de `inline_tests.rs` :

```rust
/// Générateur déterministe (SplitMix64) : le corpus ne dépend d'aucune crate.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) % n as u64) as usize
    }
}

/// Lettres, chiffres, espaces et toute la ponctuation qui compte en markdown.
const ALPHABET: &[char] = &[
    'a', 'b', 'é', '上', '1', '2', ' ', ' ', '\t', '\u{a0}', '!', '"', '*', '_', '=', '[', ']', '{', '}', '\\', '#',
    '-', '.', '(', ')', '<', '`', '~', '|',
];

fn corpus() -> Vec<Vec<Run>> {
    let mut rng = Rng(180);
    (0..1000)
        .map(|_| {
            (0..1 + rng.below(6))
                .map(|_| {
                    let t: String = (0..1 + rng.below(6)).map(|_| ALPHABET[rng.below(ALPHABET.len())]).collect();
                    Run {
                        bold: rng.below(3) == 0,
                        italic: rng.below(3) == 0,
                        link: (rng.below(5) == 0).then(|| ["https://a.b/x", "https://a.b/y"][rng.below(2)].to_owned()),
                        highlight: (rng.below(4) == 0).then(|| 1 + rng.below(3) as u8),
                        ..Run::plain(t)
                    }
                })
                .collect()
        })
        .collect()
}

#[test]
fn runs_to_markdown_corpus_matches_fixture() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/markdown-corpus.json");
    let cases: Vec<String> = corpus()
        .iter()
        .map(|runs| serde_json::json!({ "runs": runs, "markdown": runs_to_markdown(runs) }).to_string())
        .collect();
    let json = format!(
        "{{\n  \"_comment\": \"Généré par inline_tests.rs (UPDATE_CORPUS=1 pour le réécrire) ; relu par markdown.test.ts.\",\n  \"cases\": [\n    {}\n  ]\n}}\n",
        cases.join(",\n    ")
    );
    if std::env::var_os("UPDATE_CORPUS").is_some() {
        std::fs::write(path, &json).unwrap();
    }
    let expected = std::fs::read_to_string(path).unwrap_or_default();
    asserting("corpus à jour (UPDATE_CORPUS=1 pour le réécrire)").that(&(json == expected)).is_true();
}
```

- [ ] **Step 2 : vérifier l'échec**

Run : `cargo nextest run -p oku-slides-session runs_to_markdown_corpus`
Expected : FAIL, « corpus à jour ». Le fichier n'existe pas encore.

- [ ] **Step 3 : générer le corpus**

Run : `UPDATE_CORPUS=1 cargo nextest run -p oku-slides-session runs_to_markdown_corpus && cargo nextest run -p oku-slides-session runs_to_markdown_corpus`
Expected : PASS deux fois. `fixtures/markdown-corpus.json` fait environ
1000 lignes de cas.

- [ ] **Step 4 : écrire le test TS de l'invariant**

À la fin de `ui/src/markdown.test.ts` (ajouter `Run` à `import type { Paragraph } from "./api"`) :

```ts
const corpus: { runs: Run[]; markdown: string }[] = JSON.parse(
  readFileSync(new URL("../../fixtures/markdown-corpus.json", import.meta.url), "utf8"),
).cases;

/** Chaque caractère et son format, compté seulement sur les lettres et chiffres. */
function formatOfLetters(runs: Run[]) {
  return runs.flatMap((r) =>
    [...r.t].map((c) => [c, /[\p{L}\p{N}]/u.test(c) ? `${!!r.bold}${!!r.italic}${r.link ?? ""}|${r.highlight ?? ""}` : ""]),
  );
}

describe("corpus écrit par Rust : texte intact, lettres et chiffres gardent leur format", () => {
  corpus.forEach((c, i) => {
    test(`cas ${i}`, () => {
      expect(formatOfLetters(markdownToRuns(c.markdown))).toEqual(formatOfLetters(c.runs));
    });
  });
});
```

- [ ] **Step 5 : vérifier que tout passe**

Run : `cd ui && bun test src/markdown.test.ts`
Expected : PASS, dont les 1000 cas du corpus. Si un cas échoue, c'est un
défaut du sérialiseur ou du lecteur : le corriger dans `inline.rs` ou
`markdown.ts`, avec un test ciblé qui le reproduit (G13), jamais en
retouchant le corpus à la main.

- [ ] **Step 6 : commit**

```bash
git add fixtures/markdown-corpus.json crates/oku-slides-session/src/inline_tests.rs ui/src/markdown.test.ts
git commit -m "test(session) : corpus aléatoire écrit par Rust, relu par Lezer (t-180)"
oku cli task done --project slider --id t-185
```

---

### Task 4 : une ligne inchangée garde son paragraphe

**Files :**
- Modify : `ui/src/deck.ts:28-43` (`textToParagraphs`), `ui/src/deck.ts:69-73` (`textOps`)
- Modify : `ui/src/markdown.ts` (supprime `sameFormatting`)
- Test : `ui/src/deck.test.ts`, `ui/src/markdown.test.ts`

**Interfaces :**
- Consumes : `markdownToRuns`, `listPrefix` (Task 1) ; `Target.text` (`api.ts:157`, `string | null`).
- Produces : `textToParagraphs(text: string, previous?: { paragraphs: Paragraph[]; text: string }): Paragraph[]`.

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `ui/src/deck.test.ts`, remplacer le test « un paragraphe inchangé garde
sa mise en forme » par :

```ts
  test("une ligne inchangée garde son paragraphe", () => {
    const bold: Paragraph = { runs: [{ t: "Gras", bold: true, color: "accent" }] };
    const previous = { paragraphs: [bold], text: "**Gras**" };
    expect(textToParagraphs("**Gras**\nNouveau", previous)[0]).toBe(bold);
  });

  test("une ligne aux bords perdus garde son paragraphe tant qu'elle n'est pas éditée", () => {
    const edges: Paragraph = { runs: [{ t: "a" }, { t: " x ", bold: true, color: "accent" }, { t: "b" }] };
    const previous = { paragraphs: [edges], text: "a **x** b" };
    expect(textToParagraphs("Avant\na **x** b", previous)[1]).toBe(edges);
  });

  test("une ligne éditée est relue", () => {
    const bold: Paragraph = { runs: [{ t: "Gras", bold: true, color: "accent" }] };
    const previous = { paragraphs: [bold], text: "**Gras**" };
    expect(textToParagraphs("**Gras** !", previous)).toEqual([{ runs: [{ t: "Gras", bold: true }, { t: " !" }] }]);
  });
```

Dans `ui/src/markdown.test.ts` : supprimer `describe("sameFormatting")` et
`sameFormatting` de l'import, puis remplacer les deux tests
`textToParagraphs garde…` par :

```ts
test("textToParagraphs garde tel quel un lien non encodé d'une ligne inchangée", () => {
  const raw: Paragraph = { runs: [{ t: "w", link: "https://w.org/A (b)", color: "accent" }] };
  const previous = { paragraphs: [raw], text: "[w](https://w.org/A%20%28b%29)" };
  expect(textToParagraphs("[w](https://w.org/A%20%28b%29)\nNouveau", previous)[0]).toBe(raw);
});
```

- [ ] **Step 2 : vérifier l'échec**

Run : `cd ui && bun test src/deck.test.ts src/markdown.test.ts`
Expected : FAIL. `textToParagraphs` ne connaît pas encore
`{ paragraphs, text }`, donc « une ligne inchangée » et « bords perdus » ne
retrouvent pas leur paragraphe. `tsc` refuserait aussi l'appel.

- [ ] **Step 3 : implémenter**

Dans `ui/src/deck.ts`, l'import devient
`import { listPrefix, markdownToRuns } from "./markdown";` et
`textToParagraphs` devient :

```ts
/**
 * Texte éditable → paragraphes : l'inverse de `paragraphs_to_text` côté Rust
 * (une ligne par paragraphe, `- ` pour une puce, `1. ` pour une liste
 * numérotée, markdown inline pour le reste). Une ligne identique à une ligne
 * de `previous.text` (écrit par Rust pour `previous.paragraphs`) garde ce
 * paragraphe tel quel : couleur, taille et bords perdus à l'écriture compris.
 */
export function textToParagraphs(text: string, previous?: { paragraphs: Paragraph[]; text: string }): Paragraph[] {
  const before = previous ? previous.text.split("\n") : [];
  return text.split("\n").map((line) => {
    const same = previous?.paragraphs[before.indexOf(line)];
    if (same) return same;
    const { list, length } = listPrefix(line);
    const runs = markdownToRuns(line.slice(length));
    return list ? { runs, list } : { runs };
  });
}
```

Dans `textOps`, la ligne `set_text` devient :

```ts
    const previous = { paragraphs: e?.text ?? [], text: t.text ?? "" };
    return [{ op: "set_text", slide: slide.id, element: t.id, text: textToParagraphs(value, previous) }];
```

Dans `ui/src/markdown.ts`, supprimer `sameFormatting` et sa doc (`encodeLink`
reste, `textEdits.ts` l'utilise).

- [ ] **Step 4 : vérifier que tout passe**

Run : `cd ui && bun test && ./node_modules/.bin/tsc -b`
Expected : PASS, `tsc` sans erreur (`noUnusedLocals` confirme que `merge`
et `run` servent encore).

- [ ] **Step 5 : commit**

```bash
git add ui/src/deck.ts ui/src/deck.test.ts ui/src/markdown.ts ui/src/markdown.test.ts
git commit -m "fix(ui) : une ligne de texte inchangée garde son paragraphe (t-180)"
oku cli task done --project slider --id t-186
```

---

### Task 5 : documentation, consolidation et vérification complète

**Files :**
- Modify : `crates/oku-slides-session/ARCHITECTURE.md`

**Interfaces :**
- Consumes : Tasks 1 à 4.
- Produces : rien de nouveau.

- [ ] **Step 1 : documenter**

Dans `crates/oku-slides-session/ARCHITECTURE.md`, section « Types clés »,
ajouter :

```markdown
- `paragraphs_to_text` (`text.rs`, inline dans `inline.rs`) : texte d'un
  élément en markdown d'édition, relu par `@lezer/markdown` côté interface.
  Contrat : `fixtures/markdown.json` (cas choisis, pertes documentées) et
  `fixtures/markdown-corpus.json` (corpus aléatoire, `UPDATE_CORPUS=1`).
```

- [ ] **Step 2 : passe de consolidation (G10)**

Run :

```bash
grep -rn "sameFormatting\|runs_to_markdown\|escape_inline\|closingUnderscore\|PUNCTUATION" ui/src crates --include=*.ts --include=*.rs
```

Expected : `runs_to_markdown` et `escape_inline` seulement dans `inline.rs`
(et l'import dans `text.rs`), plus aucune trace de `sameFormatting`, de
`closingUnderscore` ou de `PUNCTUATION` dans `ui/src`.

- [ ] **Step 3 : vérification complète**

Run : `just check && just test`
Expected : fmt, clippy et tests Rust et TS au vert.

Run : `cd ui && bun e2e/editor.cjs`
Expected : PASS (saisie, gras et pastilles dans l'éditeur réel).

- [ ] **Step 4 : commit et clôture**

```bash
git add crates/oku-slides-session/ARCHITECTURE.md
git commit -m "docs(session) : markdown d'édition et ses fixtures dans ARCHITECTURE.md (t-180)"
oku cli task done --project slider --id t-187
oku cli task done --project slider --id t-180
```
