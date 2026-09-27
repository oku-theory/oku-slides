# Markdown d'édition via @lezer/markdown — design

Date : 2026-09-27 · Tâche : t-180

## Objectif

Remplacer le parseur markdown maison de l'interface (`ui/src/markdown.ts`)
par `@lezer/markdown`, en passant à la sémantique CommonMark. Garantie
stricte : l'aller-retour runs → markdown → runs reste sans perte, sauf le
cas documenté plus bas (bornes ponctuation + lettre).

Le markdown n'est qu'une vue d'édition : le deck stocke des `Run`. Aucune
migration de données.

## Décisions

- **TS : `@lezer/markdown`.** Le spike a passé les 12 cas du fixture. On
  peut désactiver les blocs, et l'arbre donne des positions exactes pour les
  décorations CodeMirror.
- **Rust : `text.rs` reste maison.** Crates évaluées et écartées :
  - `comrak` 0.55 : correct, mais son échappement n'est pas minimal et ne
    se règle pas (`Bonjour \!`, `snake\_case`, `\#tag`). Il ne gère pas `{n}`.
  - `pulldown-cmark-to-cmark` 22 : avec perte. Il n'échappe que le premier
    caractère d'un texte, si bien qu'un `*b*` littéral est relu en italique.
    Pas de `==`.
  - `mdast_util_to_markdown` 0.0.2 : produit des entités numériques
    (`&#x78;`), n'a pas de `==` et reste immature.
- **Syntaxe inchangée** : `**gras**`, `*italique*` / `_italique_`,
  `***les deux***`, `[texte](url)`, `==pastille==`, `==pastille=={n}`
  (n de 0 à 255), `- ` et `1. ` en début de ligne.

## TS

- Dépendance : `@lezer/markdown` (qui amène `@lezer/common`). Pas de
  `@codemirror/lang-markdown`.
- `ui/src/markdownHighlight.ts` (nouveau) : `MarkdownConfig` avec les nœuds
  `Highlight` et `HighlightMark`, et des délimiteurs `==` aux règles
  flanking de CommonMark (calqués sur `Strikethrough` de Lezer, après
  `Emphasis`). Un `===` n'ouvre rien.
- `ui/src/markdown.ts` : l'API publique ne change pas (`markdownToRuns`,
  `markdownSpans`, `listPrefix`, `encodeLink`, `sameFormatting`, types
  `Span` et `SpanKind`), si bien que `deck.ts`, `textEdits.ts` et
  `markdownDecorations.ts` restent tels quels. En interne :
  - le parseur est configuré une seule fois au niveau du module, avec
    `Highlight` et `remove` de tous les blocs (`LinkReference`,
    `IndentedCode`, `FencedCode`, `Blockquote`, `HorizontalRule`,
    `BulletList`, `OrderedList`, `ATXHeading`, `HTMLBlock`,
    `SetextHeading`) et des inlines hors dialecte (`HTMLTag`, `Autolink`,
    `InlineCode`, `Image`, `Entity`, `HardBreak`) ;
  - un seul parcours de l'arbre produit les runs et les plages.
    `EmphasisMark`, `HighlightMark`, `LinkMark` et `Escape` (le `\`)
    deviennent des plages `mark` ; `URL` devient `url`. `{n}` est lu juste
    après un nœud `Highlight` et donne une plage `mark` ; au-delà de 255, il
    reste du texte ;
  - `listPrefix` et le découpage par ligne restent maison : dans l'éditeur,
    une ligne est un paragraphe, ce qui n'est pas un bloc CommonMark.
  - Les helpers maison (`scan`, `closing`, `closingUnderscore`, `link`,
    `repeated`) sont supprimés.

## Rust

`crates/oku-slides-session/src/text.rs`, même API (`paragraphs_to_text`).
Seul l'échappement ou le placement des délimiteurs change, là où la
sémantique CommonMark l'exige :

- les espaces en bord de run formaté sortent des délimiteurs (`" x"` en
  gras donne ` **x**`). C'est sans perte, puisque la mise en forme d'une
  espace ne se voit pas ;
- **perte acceptée, et seulement celle-là** : un run formaté qui finit par
  une ponctuation suivie directement d'une lettre (`*"x"*a`) ne s'écrit
  pas en CommonMark. On écrit l'approximation la plus proche, et un test
  fige le résultat. Le paragraphe garde ses runs tant que sa ligne n'est
  pas éditée (`sameFormatting` dans `textToParagraphs`).

## Tests (TDD)

- `fixtures/markdown.json` reste le contrat : Rust vérifie paragraphes →
  markdown (`text_tests.rs`), TS vérifie markdown → paragraphes
  (`markdown.test.ts`).
- Nouveaux cas du fixture : `*b*` et `[x](y)` littéraux, `# ` et `> ` en
  début de ligne, espaces en bord de gras, `Bonjour !`, `a == b`,
  `==a===`, ponctuation + lettre (perte documentée).
- Les tests existants de `markdown.test.ts` et
  `markdownDecorations.test.ts` restent. On ne met à jour que ceux dont la
  sémantique change volontairement, et chaque changement est justifié par
  la règle CommonMark concernée.
- e2e `bun e2e/editor.cjs` vert.

## Hors périmètre

Import ou export de fichiers `.md`, nouvelles syntaxes (titres, code,
tableaux), masquage des `\` dans l'éditeur.
