# Paramètres à la sélection — design

Date : 2026-10-05. Tâche parente : t-548 (sous-tâches t-549 à t-553) (voir le plan `docs/superpowers/plans/2026-10-05-parametres-a-la-selection.md`).

## But

Les paramètres d'un widget s'éditent comme les données d'un graphique : quand un
widget est la cible, un panneau à droite montre tous ses paramètres réglables, une
liste en tableau (`ListTable`). L'ajout d'item sur la slide (les « + ») disparaît :
on ajoute par le tableau.

## Comportement

- **Cible** : l'élément sélectionné ; rien de sélectionné, le widget de la slide.
- **Panneau** : la carte « Paramètres de *nom* », en tête de la colonne des notes
  (`aside.notes`), là où se monte `ChartPanel`. Un champ par paramètre réglable :
  `plain` (texte), `rich` (texte riche), `number` (curseur si bornes valides, sinon
  texte), `choice` et `flag` (sélection), `list` (tableau).
  - Valeurs lues dans le deck ; absentes, le `default` du schéma ; jamais de valeur
    d'exemple ni neutre. Une liste absente du deck est vide (comme au rendu) : tableau vide.
  - Paramètre lié à une variable du deck (`bind`) : pas de champ, une ligne de note
    « *label* suit la variable *clé* ».
  - Types non réglables (`asset`, `slot`, `grid`, `host`) : omis, sans note.
  - Aucun champ (des notes seules ne comptent pas) : pas de panneau.
  - Un graphique (widget à paramètre `host(chart)`) garde `ChartPanel`, sans panneau de paramètres.
  - Sélection périmée (élément disparu) : l'interface résout la sélection dans la vue ;
    un élément disparu n'est plus sélectionné et le panneau montre celui de la slide,
    sans erreur. La session, elle, ne renvoie aucune surface pour un id d'élément inconnu.
  - Nom montré : le `label` du widget et de chaque paramètre ; sans `label` (widgets de
    base), un nom traduit (`session-widget-kind-<kind>`, `session-param-<kind>-<nom>`),
    sinon le nom brut.
  - Le graphique sélectionné ne demande pas de surface de paramètres à la session.
- **Modes** : deck et Modèles (slides modèles). En mode Modèles la carte « Ce
  modèle » reste sous la scène ; le panneau de paramètres est à droite. L'atelier de
  widget (`host-widget`) ne change pas et, ouvert, masque le panneau de droite. Pas de panneau en lecture seule ni dans
  l'historique.
- **Scène et aperçu de l'atelier** : plus de « + » (entre items, avant, après, au
  centre d'une liste vide). Les poignées « × » et ⋮⋮ restent.
- **Annulation** : chaque réglage est un pas, « Réglage : *label* » ; seuls les réglages
  successifs d'un même curseur (glissé) fusionnent en un pas. Les autres valeurs et les
  remplacements de liste ne fusionnent jamais.
- **Insertion** : le tableau ajoute en fin de liste ; l'insertion à une position précise
  (les « + » entre items) disparaît, on glisse la ligne ensuite.

## Architecture

Approche retenue : une surface de réglages construite par la session (comme
`host-template` et `host-widget`), une seule correspondance type → champ.

### Session (Rust, `oku-slides-session`)

- `param_fields.rs` (nouveau) : `field()`, `column()`, `label()`, `slider_step()`,
  `settable()` déplacés de `widget_params.rs`, plus `coerce()` (conversion pure tirée de
  `checked_value`) et `schema_default()` (tiré de `preview_params`), partagés entre
  l'atelier (valeurs d'exemple) et la cible (valeurs du deck). Aucune copie.
- Nouveau `target_params.rs` :
  - `Session::target_settings(slide, element) -> Result<Option<Surface>>` : surface
    `host-target` (titre `session-widget-params`) ; `None` hors édition (historique,
    aide), cible disparue, sans schéma, graphique, sans champ.
  - `Session::target_param(slide, element, key, value)` : `Op::SetParam`, valeur
    convertie par `coerce` (texte vide : efface) ; une liste attend un tableau d'items
    (collage) ; fusion d'annulation pour un curseur seulement (`FieldKind::Slider`).
  - `Session::target_list(slide, element, key, change: ListOp)` : `Op::EditList`.
  - Clé : celle d'un champ montré (`bind`, non réglable, inconnue → `HostKey`) ; slide
    inconnue → `NoSlide`. Validation par `apply` (refus `Op` du cœur, localisés). Un pas,
    libellé `session-undo-param`. Pas de nouvelle variante d'erreur.
- `panel.rs` : `HOST_TARGET = "host-target"`, `slide_palette()`. `Request::HostSettings`
  et `HostParam` inchangées ; nouvelles `Request::TargetSettings { slide, element? }`,
  `TargetParam { slide, element?, key, value }`, `TargetList { slide, element?, key,
  change }`.
- i18n : `session-undo-param`, `session-target-bound` dans les 20 langues.

### Interface (`ui/src`)

- `useHostSettings.ts` : le cycle commun (lire, garder si même contenu, relire après
  refus, ignorer une surface remplacée) devient `useSessionSurface`, utilisé par
  `useHostSettings` et par `useTargetSettings.ts`, qui lit la surface de la cible
  (élément résolu dans la vue, sinon la slide ; inactif en lecture seule, historique,
  atelier ouvert) et envoie vers la cible **de la surface affichée** : valeur →
  `targetParam`, `ListOp` → `targetList`, remplacement → `targetParam` avec les items.
- `TargetPanel.tsx` : la carte en tête d'`aside.notes` — `ChartPanel` pour un
  graphique, sinon la surface `host-target` (`HostSettings.tsx`). `App.tsx` ne fait
  que câbler.
- `api.ts` : `targetSettings`, `targetParam`, `targetList`.

### Retrait des « + »

- `ItemLayer.tsx` : `plus`, `insert`, `INSERT_SIZE`, le « + » d'une liste vide.
- `stageItems.ts` : `within`, `onInserted`, `onEditWhenShown` ; `insertPoints`
  reste (repère du glisser, `dropTarget`).
- `Stage.tsx`, `App.tsx`, `useFieldEditing.ts` (`editKeyWhenShown`),
  `WidgetTab.tsx` : props et état qui n'existaient que pour ouvrir l'item inséré.
- Rust : `first_text` des vues de liste (seul lecteur : l'insertion).
- `styles.css` : `.item-insert`. i18n : `ui-undo-list-add` (20 langues, et la liste
  de `oku-slides-i18n/src/lib_tests.rs`). Aide `docs/aide/{en,fr}.json`.
- Tests et e2e : `ItemLayer`, `Stage`, `WidgetTab`, `stageItems`, `useFieldEditing` ;
  `lists.cjs` ajoute par le tableau du panneau, `lists-dock.cjs` perd le « + » de l'aperçu.

## Erreurs

Valeur refusée (type, bornes de liste, taille) : `apply` la refuse (erreur `Op` du
cœur), le panneau relit la surface (le champ reprend la valeur du deck), message
d'erreur comme les autres réglages. Clé inconnue, liée, non réglable, ou élément
disparu : `HostKey` ; slide inconnue : `NoSlide`. Lire une cible disparue n'est pas une
erreur : pas de panneau.

## Tests

- Rust (`target_params_tests.rs`) : champs par type pour un élément et pour une
  slide ; `bind` en note ; types omis ; `None` pour un graphique et sans paramètre ;
  `target_param` : valeur, flag, nombre, effacement, chaque `ListOp`, `replace`, un
  seul pas d'annulation libellé, refus (valeur invalide, clé inconnue, liste pleine).
- UI (rendu) : panneau suit la sélection ; slide quand rien n'est sélectionné ;
  graphique → `ChartPanel` seul ; pas de panneau en lecture seule ; routage vers
  `targetParam` et relecture après refus ; `ItemLayer` / `Stage` / `WidgetTab` sans
  « + ».
- e2e : `lists.cjs`, `lists-dock.cjs` ; un e2e `target-panel.cjs` (sélection d'une
  frise, ajout d'une étape au tableau, la slide la montre, ⌘Z l'annule).
