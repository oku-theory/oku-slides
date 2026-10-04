# Réglages du modèle et du widget sous leur aperçu — design

Date : 2026-10-04 · Tâche : t-488

## Objectif

En mode modèles, les réglages de l'hôte (t-423 phase B) s'affichent au
dock de l'agent, loin de ce qu'ils règlent : « Ce modèle » (nom,
description, grille…) et « Paramètres de <widget> » (valeurs d'essai).
On les place **sous leur aperçu** :

- sur le sous-onglet Modèles, « Ce modèle » sous la slide ;
- sur un sous-onglet de widget, « Paramètres de <widget> » sous l'aperçu
  du widget, dans la même zone (ils défilent avec lui) ; le code garde sa
  place (dessous ou à droite, t-474) et sa taille.

Le dock de l'agent ne garde que les surfaces de l'agent (questions).

Hors périmètre : la vue Scène / Planche et l'édition sur la diapo (t-497) ;
le contenu des surfaces, construites par la session, inchangé.

## Décisions

1. **Mêmes surfaces, autre place** : `useHostSettings` est inchangé
   (lecture, routage des actions, relecture après refus). `App` ne les
   ajoute plus au dock (`dockWithHost` disparaît).
2. **Un composant `HostSettings`** (`ui/src/HostSettings.tsx`) rend une
   surface de l'hôte avec `SurfaceView`, dans un conteneur
   `.host-settings` qui porte `data-surface-id` (les e2e et les tests
   visent toujours `[data-surface-id="host-…"]`). Pas d'annonce
   (`aria-live="off"`), comme au dock.
3. **Modèles** : `.templates-panel` passe en colonne ; la scène prend la
   place, les réglages dessous, au plus 40 % de la hauteur, défilant
   seuls au-delà.
4. **Widget** : `WidgetPane` passe un nœud `settings` au sous-onglet
   actif ; `WidgetTab` le pose dans `.widget-tab-side`, entre l'aperçu et
   les diagnostics. Largeur bornée (`min(100%, 560px)`), centrée sous
   l'aperçu. L'aperçu garde au moins 96 px de haut ; au-delà, la zone
   défile. Code dessous, la part par défaut de l'aperçu passe de 1/4
   (t-474) à 2/5 : elle porte désormais les réglages ; une hauteur réglée
   au séparateur reste prioritaire.
5. **Style** : carte posée, pas de verre flottant (voir t-486) : fond
   `--paper-100`, rayon moyen, ombre d'objet.

## Tests

- `HostSettings.render.test.tsx` : `data-surface-id`, action routée à
  `onAction` avec la surface.
- `WidgetTab.render.test.tsx` : le nœud `settings` est dans
  `.widget-tab-side`, après l'aperçu ; absent d'un onglet inactif.
- e2e `panel.cjs` : « Ce modèle » est dans `.templates-panel`, pas dans
  `.notes` ; `widgets.cjs` : « Paramètres de » dans `.widget-tab-side`,
  pas dans `.notes`.
