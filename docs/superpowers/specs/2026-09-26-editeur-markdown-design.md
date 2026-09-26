# Éditeur de texte avec rendu markdown — design

Date : 2026-09-26 (revu après la revue d'équipe du plan)

## Objectif

Dans l'éditeur posé sur une zone de texte, le markdown reste écrit tel quel
(les marques restent visibles) mais s'affiche déjà mis en forme :
`**montrer**` apparaît en gras, `*mot*` en italique, un lien dans l'accent du
thème, une pastille sur son fond coloré. L'éditeur montre la même mise en
forme que la slide ; seuls les retours à la ligne peuvent différer (les
marques visibles prennent de la place).

Hors périmètre : masquer les marques hors de la ligne du curseur (façon
Obsidian) ; mise en forme dans les champs (titre, sous-titre, kicker), qui
restent du texte brut ; suivre un changement de thème pendant une édition
(la palette est lue à l'ouverture, `t-127`) ; arbitrer une écriture
concurrente sur le texte ouvert (agent vocal, `t-130`) : aujourd'hui la
dernière validation l'emporte.

## Choix : CodeMirror 6

Une `<textarea>` n'affiche qu'un seul style. Écartés :

- `contentEditable` maison : réécrire le DOM à chaque frappe casse la
  composition IME (japonais) et l'annulation native.
- Calque stylé sous une textarea transparente : le vrai gras est plus large,
  le curseur se décale.

CodeMirror 6 gère IME, sélection, collage et historique. Dépendances directes :
`@codemirror/state`, `@codemirror/view`, `@codemirror/commands` (toutes MIT).
`commands` tire `@codemirror/language` et le runtime Lezer (`@lezer/common`,
`@lezer/highlight`, `@lezer/lr`) : ils sont installés, mais on ne charge
aucune grammaire — pas de `@codemirror/lang-markdown`, les styles viennent de
notre parseur. `useEffectEvent` demande React 19.2 au moins : les versions
minimales de `react`, `react-dom` et de leurs types passent à `^19.3.0`.

## Architecture

- `ui/src/markdown.ts` : le parcours du parseur enregistre des positions.
  `markdownSpans(line): Span[]`, où `Span = { from, to, kind }` et `kind`
  vaut `mark` (délimiteurs, `\`, `](`, `)`, `{n}`), `bold`, `italic`,
  `link`, `url` ou `highlight` (avec `highlight: n`). `markdownToRuns`
  repose sur le même parcours : une seule implémentation. Nouvelle
  `listPrefix(line)` (préfixe `- ` / `1. `), partagée par `textToParagraphs`
  (`deck.ts`) et l'éditeur : une seule règle des listes.
- `ui/src/textEdits.ts` (nouveau) : éditions pures sur (texte, sélection) —
  `toggleEmphasis` et `insertLink` (déplacés de `markdown.ts`, inchangés),
  `insertDictated` (sorti de l'effet de dictée de `Stage.tsx`) et
  `minimalChange` (le plus petit remplacement entre deux textes).
- `ui/src/color.ts` (nouveau) : `luminance` (déplacée de `Stage.tsx`, lit
  aussi `#rgb`) et le seuil fond clair / fond foncé, calculé exactement comme
  `ink-on` de Typst (luma > 60 %, soit ((0,6 + 0,055) / 1,055)^2,4 en WCAG).
- `ui/src/markdownDecorations.ts` (nouveau) : `lineDecorations(line, palette)`
  pure, et l'extension CodeMirror (`ViewPlugin`) qui l'applique à tout le
  document. La palette n'est acceptée que sous forme `#rgb` / `#rrggbb`.
- `ui/src/TextEditor.tsx` (nouveau) : enveloppe React de CodeMirror. Props :
  `initial`, `field` (champ d'une ligne, sinon bloc markdown), `palette`,
  `label` (aria-label), `style` (géométrie), `onChange(text)`,
  `onCommit(text)`, `onCancel()`, et `ref` (prop React 19) vers
  `{ insertAtCursor(text) }` pour la dictée. Les raccourcis ⌘B / ⌘I / ⌘K
  vivent ici (keymap CodeMirror), pas dans `Stage`. Touches de base :
  `standardKeymap` (pas `defaultKeymap`, fait pour le code).
- `ui/src/Stage.tsx` : garde `editorGeometry`, le masque, l'aide ; remplace
  la `<textarea>` par `TextEditor`. `commit(text)` reçoit le texte de
  CodeMirror : c'est la seule source de vérité ; `draft` ne sert plus qu'à la
  largeur d'un champ.

## Rendu dans l'éditeur

| Plage | Style |
|---|---|
| `mark`, `url`, préfixe de liste (`- `, `1. `) | couleur du texte à 40 % d'opacité |
| `bold` | `font-weight: bolder` |
| `italic` | `font-style: italic` |
| `link` | accent du thème (`--t-accent`), souligné |
| `highlight` | fond `palette[(n − 1) mod longueur]` (modulo positif, comme Typst : `{0}` donne la dernière couleur), texte `--t-ink` ou `--t-bg` selon la luminance |

Les styles se cumulent. Une pastille est un simple fond, sans coins arrondis
ni marge : CodeMirror découpe une plage décorée autour des plages imbriquées,
des coins arrondis se répéteraient sur chaque morceau. La palette est
`appearance.chart_palette` (`#rgb` ou `#rrggbb`). Les réglages de
CodeMirror (police, interligne, marges, couleur du curseur) passent par
`EditorView.theme`, avec des sélecteurs au moins aussi précis que ceux de son
thème de base.

## Comportement

- Bloc : ⌘Entrée valide ; Entrée insère un simple retour à la ligne (sans
  recopier d'indentation). Champ : Entrée et Maj+Entrée valident ; un retour
  à la ligne collé, dicté ou déjà présent devient une espace, et la
  modification garde son pas d'annulation. Échap annule. Perte du focus :
  valide.
- Une Entrée qui confirme une conversion IME ne valide pas : CodeMirror
  l'ignore déjà (y compris l'Entrée que WebKit envoie juste après
  `compositionend`) ; pas de garde maison.
- Pas de mise en forme native : ⌘U partout, ⌘B / ⌘I / ⌘K dans un champ, ne
  font rien.
- Un clic dans la marge de l'éditeur garde le focus (ne valide pas).
- ⌘Z / ⇧⌘Z : historique de CodeMirror, limité à l'édition en cours ; chaque
  raccourci (⌘B, ⌘I, ⌘K) et chaque dictée est un pas d'annulation à part.
  Les commandes d'éditeur de code (déplacer une ligne, indenter…) ne sont pas
  liées.
- Collage : texte brut seulement (CodeMirror ne lit que `text/plain`).
- L'éditeur grandit avec son contenu, fond `editorBackground`, retour à la
  ligne dans la largeur de la boîte ; un champ s'élargit en tapant.
- Dictée : le texte dicté s'insère à la sélection, mêmes règles d'espaces.
- `spellcheck`, `autocorrect`, `aria-label`, `aria-multiline` (faux pour un
  champ), direction par ligne (`perLineTextDirection`).
- Le composant supporte le double montage du StrictMode de React.
- Défense en profondeur : les raccourcis globaux de `App.tsx` ignorent aussi
  un `[contenteditable="true"]` (ils s'arrêtent déjà pendant l'édition).

## Tests (TDD)

- `markdown.test.ts` : `markdownSpans` (gras, italique, imbrication, lien,
  lien vide, pastille avec et sans `{n}`, échappements, emoji, délimiteurs
  sans fermeture) et `listPrefix`. La fixture partagée avec Rust reste le
  contrat de `markdownToRuns`.
- `textEdits.test.ts`, `color.test.ts`, `markdownDecorations.test.ts` :
  fonctions pures, sans DOM. `TextEditor.test.ts` : le filtre d'une ligne
  sur un `EditorState` (collage, annulation).
- e2e `editor.cjs` (Chromium, autonome, lancé en premier) : mise en forme
  pendant la saisie et couleur de la pastille, police, interligne et curseur
  de CodeMirror égaux à ceux de l'hôte, Échap ferme sans rien changer, clic
  dans la marge, ⌘B isolé dans l'historique, collage, Entrée dans un bloc,
  perte du focus, aller-retour (rouvrir redonne le même texte), champ
  (Entrée, Maj+Entrée, collage multiligne), ⌘Z qui ne touche pas
  l'historique du deck.
- Vérification en mode développement (`just web`, StrictMode actif) :
  ouvrir, annuler, rouvrir, valider → une seule entrée d'annulation.
- Vérification dans l'app de bureau (WebKit, `just dev`) : conversion IME
  japonaise avec Entrée dans un titre et un bloc, dictée système,
  Édition > Annuler du menu (après une frappe et juste après ⌘B).
