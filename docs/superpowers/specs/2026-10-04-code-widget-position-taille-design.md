# Position et taille du code d'un widget, gardées par l'hôte — design

Date : 2026-10-04 · Tâche : t-474

## Objectif

Dans un sous-onglet de widget, le code déplié (t-461) occupe aujourd'hui
toujours les trois quarts du bas, sous l'aperçu. On veut lui donner plus
de place, à la demande :

- le placer **dessous** (comme aujourd'hui) ou **à droite** de l'aperçu ;
- régler sa **taille** au séparateur entre l'aperçu et le code ;
- **garder** position et taille dans `UiState`, reprises au lancement,
  comme les largeurs de la pile et du fil (`pile`, `notes`).

Prérequis : t-461 (code replié par défaut, champ `UiState.code`).

Hors périmètre : la refonte « Scène + Planche » maquettée le 2026-10-04
(édition sur l'aperçu, cas d'exemple) ; une taille par widget ; un
réglage de la taille quand le code est replié.

## Décisions

1. **Un choix commun à tous les widgets**, comme `code` : un seul
   placement et une taille par placement, pas un réglage par fichier.
2. **Deux tailles gardées, une par placement** : la hauteur du code
   dessous et sa largeur à droite. Passer de l'un à l'autre retrouve la
   taille déjà choisie pour ce placement.
3. **En px, comme `pile` et `notes`** ; `None` : la proportion par défaut
   (dessous : aperçu 1 / code 3, comme t-461 ; à droite : moitié /
   moitié). La valeur gardée est bornée à l'affichage : l'aperçu et le
   code gardent chacun au moins 120 px, quelle que soit la fenêtre.
4. **Bascule dans la barre d'outils**, juste à droite de `</>` et
   visible seulement quand le code est déplié : un bouton à bascule
   « Code à droite » (`aria-pressed`). Pas d'entrée de menu en plus.
5. **Séparateur entre aperçu et code**, fait comme `ColumnDivider` :
   glisser au pointeur (pas de glisser-déposer HTML5), flèches du clavier
   par pas de 16 px, double-clic pour revenir à la taille par défaut.
   Horizontal dessous (↑/↓), vertical à droite (←/→, inversé en RTL).
   Pendant le glisser, la taille ne touche que le DOM ; elle n'est
   retenue (et sauvée) qu'au relâcher. Absent quand le code est replié.
6. **Repli faute de place** : si le sous-onglet fait moins de 560 px de
   large, « à droite » s'affiche dessous (requête de conteneur). Le choix
   gardé ne change pas ; le séparateur suit la disposition affichée.
   (Seuil d'abord prévu à 720 px : entre la pile et le fil, la feuille ne
   fait que 644 px dans une fenêtre de 1600 px, « à droite » n'aurait
   presque jamais servi.)
7. **Transition** : déplier / replier garde l'animation de t-461 ;
   changer de placement n'anime pas (pas d'animation de grille entre
   rangées et colonnes).

## Session (Rust)

`UiState` (`crates/oku-slides-session/src/views.rs`) gagne trois champs,
`#[serde(default)]` — un ancien `state.json` se relit tel quel :

- `code_place: CodePlace` — `Below` (défaut) ou `Right`, sérialisé
  `"below"` / `"right"` ;
- `code_height: Option<u32>` — hauteur du code dessous, en px ;
- `code_width: Option<u32>` — largeur du code à droite, en px.

Rien d'autre côté hôte : pas de bornes (elles dépendent de la fenêtre et
sont appliquées par l'interface), `sanitized` inchangé.

## Interface (TypeScript)

- `api.ts` : `UiState` reprend les trois champs (`code_place: "below" |
  "right"`, `code_height` / `code_width: number | null`).
- `useViewOptions` : état `codeLayout: { place, height, width }`, repris
  au premier snapshot, sauvé avec le reste ; `setCodeLayout`.
- `CodePlaceSwitch.tsx` : le bouton à bascule (décision 4), libellé
  `ui-widgets-code-right`.
- `CodeDivider.tsx` : le séparateur (décision 5), libellé
  `ui-divider-code`. Il pose `--code-h` / `--code-w` sur le
  `.widget-tab`, mesure la grille calculée pour connaître l'axe affiché
  (décision 6) et borne la taille (décision 3) par une fonction pure
  `fitCode(total, size)` testée seule.
- `WidgetTab` : `data-place="below|right"` à côté de `data-code`, et le
  séparateur entre aperçu et code quand le code est déplié.
- `styles.css` : `.widget-tab` devient `container-type: inline-size` ;
  dessous, `grid-template-rows: minmax(120px, 1fr) var(--code-h, minmax(0, 3fr))` ;
  à droite, colonnes `minmax(120px, 1fr) var(--code-w, minmax(0, 1fr))` ;
  `@container (min-width: 560px)` permet la disposition à droite.
- Libellés, dans les 20 langues : `ui-widgets-code-right` (« Code à
  droite »), `ui-divider-code` (« Taille du code »).

## Tests

- Rust (`views_tests.rs`, `resume_tests.rs`) : un `state.json` sans les
  champs se relit avec `Below` / `None` / `None` ; les trois champs font
  l'aller-retour JSON avec les valeurs `"below"` / `"right"`.
- `fitCode` : bornes basses des deux côtés, fenêtre trop petite pour
  les deux minimums (moitié / moitié), valeur `null` intacte.
- `CodeDivider.render.test.tsx` : orientation et `aria-orientation`
  selon l'axe ; flèches → `onChange` avec ±16 px ; double-clic →
  `null` ; glisser → DOM seul, puis `onChange` au relâcher.
- `CodePlaceSwitch.render.test.tsx` : `aria-pressed`, bascule.
- `useViewOptions` : reprise des trois champs et sauvegarde après un
  changement.
- `WidgetTab` : `data-place` suit le choix ; pas de séparateur code replié.
- e2e (avec `OKU_SLIDES_PATHS__DATA` / `__DECKS` temporaires) : passer
  à droite, glisser le séparateur, relancer ; placement et largeur
  retrouvés.
