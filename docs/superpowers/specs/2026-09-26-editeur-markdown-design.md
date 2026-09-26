# Éditeur de texte avec rendu markdown — design

Date : 2026-09-26

## Objectif

Dans l'éditeur posé sur une zone de texte, le markdown reste écrit tel quel
(les marques restent visibles) mais s'affiche déjà mis en forme :
`**montrer**` apparaît en gras, `*mot*` en italique, un lien en couleur
accent, une pastille sur son fond coloré. Ce que montre l'éditeur est
exactement ce que la slide rendra.

Hors périmètre : masquer les marques hors de la ligne du curseur (façon
Obsidian) ; mise en forme dans les champs (titre, sous-titre, kicker), qui
restent du texte brut.

## Choix : CodeMirror 6

Une `<textarea>` n'affiche qu'un seul style. Écartés :

- `contentEditable` maison : réécrire le DOM à chaque frappe casse la
  composition IME (japonais) et l'annulation native.
- Calque stylé sous une textarea transparente : le vrai gras est plus large,
  le curseur se décale.

CodeMirror 6 gère IME, sélection, collage et historique. On ne prend que
`@codemirror/state`, `@codemirror/view` et `@codemirror/commands` ; pas de
`@codemirror/lang-markdown` : les styles viennent de notre parseur.

## Architecture

- `ui/src/markdown.ts` : le parcours du parseur enregistre des positions.
  Nouvelle fonction `markdownSpans(line): Span[]`, où
  `Span = { from, to, kind }` et `kind` vaut `mark` (délimiteurs, `\`,
  `](url)`, `{n}`), `bold`, `italic`, `link`, `url` ou `highlight`
  (avec `highlight: n`). `markdownToRuns` repose sur le même parcours :
  une seule implémentation, pas de second parseur.
- `ui/src/markdownDecorations.ts` (nouveau) : extension CodeMirror
  (`ViewPlugin`) qui décore les lignes visibles à partir de `markdownSpans`,
  plus les préfixes de liste (`- `, `1. `) et l'échappement de marqueur de
  liste (`\-`, `1\.`) en `mark`. Prend la palette en paramètre.
- `ui/src/TextEditor.tsx` (nouveau) : enveloppe React de CodeMirror. Props :
  valeur initiale, `markdown: boolean`, palette, style (géométrie), rappels
  `onChange`, `onCommit`, `onCancel`, `commitOnEnter`. Expose via `ref` :
  `insertAtCursor(text)` (dictée) et `focus()`.
- `ui/src/Stage.tsx` : garde `editorGeometry`, le masque, l'aide ; remplace
  la `<textarea>` par `TextEditor`. Les raccourcis ⌘B / ⌘I / ⌘K appellent
  `toggleEmphasis` / `insertLink` (inchangés, purs) sur le texte et la
  sélection de CodeMirror, puis appliquent le résultat par une transaction.

## Rendu dans l'éditeur

| Plage | Style |
|---|---|
| `mark` | couleur du texte à ~40 % d'opacité |
| `bold` | `font-weight: bolder` |
| `italic` | `font-style: italic` |
| `link` | couleur accent, souligné |
| `url` | comme `mark` |
| `highlight` | fond `palette[(n − 1) mod longueur]`, texte clair ou foncé selon la luminance, coins arrondis, petite marge horizontale |

Les styles se cumulent (gras dans une pastille dans un lien). La palette est
`appearance.chart_palette`, déjà envoyée à l'interface.

## Comportement

- Bloc : ⌘Entrée valide ; Entrée insère une ligne. Champ : Entrée valide.
  Échap annule. Perte du focus : valide (comme aujourd'hui).
- ⌘Z / ⇧⌘Z : historique de CodeMirror, limité à l'édition en cours ; les
  touches ne remontent pas au ⌘Z global du deck.
- Collage : texte brut seulement.
- L'éditeur grandit avec son contenu (hauteur automatique), fond
  `editorBackground`, retour à la ligne dans la largeur de la boîte pour un
  bloc ; un champ s'élargit en tapant comme aujourd'hui.
- Dictée : le texte dicté s'insère à la sélection de CodeMirror, avec les
  mêmes règles d'espaces qu'aujourd'hui.
- `dir="auto"`, correcteur orthographique, `aria-label` conservés sur le
  `contenteditable`.

## Tests (TDD)

- `markdown.test.ts` : `markdownSpans` — plages exactes pour gras,
  italique, imbrication, lien (texte + adresse), pastille avec et sans `{n}`,
  échappements, délimiteur sans fermeture (aucune plage). La fixture
  partagée avec Rust reste le contrat de `markdownToRuns`.
- `markdownDecorations` : fonction pure `lineDecorations(text, palette)`
  (plages + classes / styles) testée sans DOM ; le `ViewPlugin` n'en est
  qu'une enveloppe.
- e2e `smoke.cjs` : la saisie passe par le `contenteditable` de CodeMirror
  au lieu de `textarea.editor`.
- Vérification dans l'app : gras / italique / lien / pastille visibles dans
  l'éditeur, ⌘B, ⌘Z local, Échap, dictée, rendu identique après validation.
