# Markdown d'édition via @lezer/markdown — design

Date : 2026-09-27 · Tâche : t-180

## Objectif

Remplacer le parseur markdown maison de l'interface (`ui/src/markdown.ts`)
par `@lezer/markdown`, en passant à la sémantique CommonMark. Garantie
stricte : l'aller-retour runs → markdown → runs ne perd jamais de texte ;
seule la mise en forme des bords de runs délimités peut se perdre (voir
« Rust »).

Le markdown n'est qu'une vue d'édition : le deck stocke des `Run`. Aucune
migration de données.

## Décisions

- **TS : `@lezer/markdown`.** Le spike a passé les 12 cas du fixture. On
  peut désactiver les blocs, et l'arbre donne des positions exactes pour les
  décorations CodeMirror.
- **Rust : sérialiseur maison.** Crates évaluées et écartées :
  - `comrak` 0.55 : correct, mais son échappement n'est pas minimal et ne
    se règle pas (`Bonjour \!`, `snake\_case`, `\#tag`). Il ne gère pas `{n}`.
  - `pulldown-cmark-to-cmark` 22 : avec perte. Il n'échappe que le premier
    caractère d'un texte, si bien qu'un `*b*` littéral est relu en italique.
    Pas de `==`.
  - `mdast_util_to_markdown` 0.0.2 : produit des entités numériques
    (`&#x78;`), n'a pas de `==` et reste immature.
- **`==` a des règles permissives** (ce n'est pas du CommonMark, on en
  fixe le sens) : ouvre s'il n'est pas suivi d'une espace, ferme s'il n'est
  pas précédé d'une espace, comme le parseur actuel. En flanking strict,
  une pastille en gras collée à une lettre (`上==**a**==です`, courant en
  japonais) serait impossible à écrire.
- **Syntaxe inchangée** : `**gras**`, `*italique*` / `_italique_`,
  `***les deux***`, `[texte](url)`, `==pastille==`, `==pastille=={n}`
  (n de 0 à 255), `- ` et `1. ` en début de ligne.

## TS

- Dépendance : `@lezer/markdown` (qui amène `@lezer/common`). Pas de
  `@codemirror/lang-markdown`.
- `ui/src/markdownHighlight.ts` (nouveau) : `MarkdownConfig` avec les nœuds
  `Highlight` et `HighlightMark`, et des délimiteurs `==` résolus par Lezer
  comme `Strikethrough` (après `Emphasis`), avec les règles permissives
  ci-dessus. Un `===` n'ouvre rien ; un `\=` échappé juste avant un `==`
  ne compte pas dans la suite de `=` (`==\===` est une pastille « = »).
- `ui/src/markdown.ts` : l'API publique ne change pas (`markdownToRuns`,
  `markdownSpans`, `listPrefix`, `encodeLink`, types `Span` et `SpanKind`),
  si bien que `textEdits.ts` et `markdownDecorations.ts` restent tels quels.
  `sameFormatting` disparaît (voir `deck.ts`). En interne :
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
  - Un lien sans adresse (`[a]`, `[a][b]`, `[a]()`) reste du texte littéral,
    comme aujourd'hui.
  - `***x***` : CommonMark l'imbrique en `<em><strong>`, donc la plage
    italique va de 1 à 6 (et non plus de 3 à 4). Le test existant est mis à
    jour.
- `ui/src/deck.ts` : `textToParagraphs(text, previous, previousText)` garde
  le paragraphe `previous[i]` dont la ligne d'origine `previousText`
  (écrite par `paragraphs_to_text`, soit `Target.text`) est identique à la
  nouvelle ligne. On compare des lignes, et non plus des runs relus
  (`sameFormatting`) : une ligne non éditée garde sa couleur et sa taille
  même quand ses bords ont subi la perte documentée. `textOps` passe
  `t.text`.

## Rust

- `crates/oku-slides-session/src/inline.rs` (nouveau, tests
  `inline_tests.rs`) : `runs_to_markdown(&[Run]) -> String`, déplacé de
  `text.rs` et réécrit. `text.rs` garde `paragraphs_to_text`,
  `escape_list_marker` et `interp`.
- **Pile d'emphases** : une emphase partagée par des runs voisins reste
  ouverte (gras-italique `X` puis italique `Y` donne `***X**Y*`, et non
  `***X****Y*`, que CommonMark relit mal). La plus longue est à
  l'extérieur. Pastilles et liens sont des barrières : ils ferment les
  emphases ouvertes et portent leurs propres emphases à l'intérieur
  (`==**a**==`, `[**a**](u)`).
- **Normalisation des bords**, avant l'écriture : à chaque jonction où des
  `*` ferment ou ouvrent, fermetures et ouvertures forment une seule suite
  de `*` entre deux caractères de texte `a` et `b`. Elle doit fermer
  (`a` pas une espace ; si `a` est une ponctuation, `b` n'est pas une
  lettre ou un chiffre) et ouvrir (symétrique). Pastille hors lien : pas
  d'espace contre `==`. Sinon le caractère fautif sort de ses délimiteurs :
  il devient un run qui garde les emphases encore ouvertes autour, sans
  pastille ni lien. On recommence jusqu'à ce qu'il n'y ait plus de faute.
  Chaque déplacement retire de la mise en forme, donc la boucle termine.
  Dans un lien, les espaces de bord restent dans les crochets, hors
  délimiteurs. Les crochets, `==` et `{n}` comptent comme de la ponctuation.
- **La seule perte acceptée** est donc de la mise en forme sur des espaces
  et de la ponctuation, jamais sur une lettre ou un chiffre, jamais sur le
  texte. Elle se voit pour la pastille (fond) et presque pas pour le gras et
  l'italique. Exemples : `a` + `" x "` gras + `b` donne `a **x** b` ;
  `-` gras + `a` italique donne `-*a*`. Prototype (Rust → Lezer,
  500 000 runs aléatoires) : 0 perte de texte, 0 perte sur une lettre ou un
  chiffre. Une ligne non éditée garde de toute façon son paragraphe
  d'origine (voir `deck.ts`).

## Tests (TDD)

- `fixtures/markdown.json` reste le contrat : Rust vérifie paragraphes →
  markdown (`text_tests.rs`), TS vérifie markdown → paragraphes
  (`markdown.test.ts`).
- Nouveaux cas du fixture : `*b*` et `[x](y)` littéraux, `# ` et `> ` en
  début de ligne, `Bonjour !`, pastille « = » (`==\===`), italique puis
  gras collés (`*a***b**`), gras-italique puis italique (`***X**Y*`),
  pastille en gras collée à du japonais (`上==**a**=={2}です`).
- Section `lossy` du fixture (paragraphes → markdown → paragraphes relus) :
  espaces en bord de gras, ponctuation collée à une lettre, pastille réduite
  à une espace, gras sur un tiret collé à de l'italique. Rust vérifie le
  markdown, TS vérifie les paragraphes relus.
- **Test croisé sur corpus** : `inline_tests.rs` génère un corpus
  déterministe de runs aléatoires (générateur maison, sans dépendance) et
  l'écrit avec leur markdown dans `fixtures/markdown-corpus.json`. Le test
  échoue si le fichier diffère ; `UPDATE_CORPUS=1` le réécrit.
  `markdown.test.ts` relit chaque ligne et vérifie l'invariant : même texte,
  même mise en forme sur chaque lettre et chaque chiffre.
- `textToParagraphs` : une ligne identique à sa ligne d'origine garde son
  paragraphe (couleur, taille), même après une perte de bord.
- Les tests existants de `markdown.test.ts` et
  `markdownDecorations.test.ts` restent. On ne met à jour que ceux dont la
  sémantique change volontairement, et chaque changement est justifié par
  la règle CommonMark concernée.
- e2e `bun e2e/editor.cjs` vert.

## Hors périmètre

Import ou export de fichiers `.md`, nouvelles syntaxes (titres, code,
tableaux), masquage des `\` dans l'éditeur.
