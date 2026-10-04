# Atelier de widget : Scène et Planche — design

Date : 2026-10-04 · Tâche : t-497

Maquette : https://claude.ai/artifact/E1NrXEUe4f5NFe5R4YJpRv (propositions A + C).

## Objectif

Dans un sous-onglet de widget, l'aperçu devient l'endroit où l'on règle
les valeurs d'exemple, et l'on éprouve le widget sur plusieurs **cas
d'exemple** à la fois :

- **Scène** : l'aperçu en grand ; ses champs texte s'éditent sur place,
  avec le mécanisme d'édition des champs de la scène des slides
  (double-clic, `TextEditor`, Tab pour passer au suivant) ;
- **Cas** : des jeux de valeurs nommés (« Court », « Long », « Vide »…),
  **enregistrés dans le pack** avec le widget ; on passe de l'un à
  l'autre par des pastilles au-dessus de l'aperçu ;
- **Planche** : tous les cas côte à côte, en vignettes, avec des alertes
  « Déborde » et « Champ vide » ; un clic ouvre le cas dans la scène.

Prérequis : t-474 (place et taille du code), t-488 (réglages sous
l'aperçu).

Hors périmètre : déplacer ou redimensionner un champ dans l'aperçu ;
éditer sur place un paramètre non texte (nombre, couleur, booléen : il
reste dans la fiche de réglages, t-488) ; plusieurs thèmes du pack dans
la planche.

## Décisions

1. **Les cas remplacent les valeurs d'essai.** Les essais
   (`TemplateScratch`, perdus en quittant le mode modèles) deviennent les
   valeurs du **cas courant**, enregistrées dans le pack. Un widget a
   toujours au moins un cas ; le premier s'appelle « Exemple » et part
   des valeurs neutres du schéma (comportement actuel).
2. **Stockage dans `pack.json`**, champ typé `examples` de
   `PackManifest` : `{ "<widget>": [ { "name": "Court", "values":
   { "<param>": <valeur> } } ] }`. Il est versionné, haché et exporté
   avec le pack sans changer le format des fichiers (les fichiers à part
   casseraient la lecture d'un deck, `sources.rs`). Les valeurs suivent
   le format de `set_widget_trial` (texte riche : tableau de
   paragraphes). Au plus 12 cas par widget, noms uniques, 40 caractères.
3. **Les cas suivent le widget** : renommer un widget renomme sa clé,
   le supprimer supprime ses cas. Une valeur que le schéma n'admet plus
   est ignorée à l'aperçu (comme `trial_fits` aujourd'hui) et retirée au
   prochain enregistrement du cas.
4. **Cas courant hors du pack** : le cas affiché est un état de
   l'interface, gardé par l'hôte dans `UiState` (`case: Map<widget,
   nom>`), pas dans le pack.
5. **Écriture** : chaque changement d'un cas (valeur, nom, ajout,
   suppression) passe par le chemin des écritures du mode modèles
   (`commit_files` → version de séance, une étape d'annulation).
   `commit_widgets` compare désormais aussi le manifeste.
   `save_templates` garde les `examples` de la séance.
6. **Aperçu avec champs** : `preview_widget` rend aussi, pour le cas
   demandé, une `SlideView` de la slide d'aperçu (`fields_on(1)` +
   `field_views` avec le registre sondé), et le décalage du recadrage
   (`bbox`) pour placer les champs sur l'image recadrée.
7. **La scène réutilise `Stage`** en mode « aperçu » : couche des champs
   et édition actives, sans sélection d'objet, ni glisser, ni
   redimensionnement, ni grille. L'application d'un champ envoie
   `set_example_value(widget, cas, param, valeur)` au lieu de `Apply`.
   Un champ lié au deck (`bind`) reste en lecture : il montre la valeur
   du deck.
8. **Planche** : une grille de vignettes, une par cas, rendues par
   `preview_widget` (un appel par cas, en parallèle, au plus 12) ; bascules
   Clair / Sombre et 16:9 / 4:3, propres à la vue et non gardées.
   Alertes calculées par l'hôte et renvoyées avec chaque aperçu :
   **Champ vide** (champ texte sans valeur) et **Déborde** (contenu qui
   dépasse le cadre du widget : boîte du champ hors de la zone, ou
   `layout` qui signale un débordement).
9. **Bascule Scène / Planche** dans la barre d'outils du sous-onglet,
   à côté de `</>` ; Scène par défaut, choix gardé dans `UiState`
   (`board: bool`), commun à tous les widgets.

## Session (Rust)

- `PackManifest.examples: BTreeMap<WidgetFile, Vec<Example>>`,
  `#[serde(default)]` ; `Example { name: String, values: Map<String, Value> }`.
- Requêtes : `add_example(widget, name, from?)`,
  `rename_example(widget, from, to)`, `remove_example(widget, name)`,
  `set_example_value(widget, name, param, value)` ; elles remplacent
  `set_widget_trial`, et `TemplateScratch.trials` disparaît (la révision
  des essais devient celle du manifeste).
- `preview_widget(name, case, format)` → `WidgetPreview` gagne
  `view: Option<SlideView>`, `offset: (x, y)` du recadrage et
  `alerts: Vec<ExampleAlert>`.
- `UiState` : `case: BTreeMap<String, String>`, `board: bool`.

## Interface (TypeScript)

- `WidgetTab` : pastilles des cas (ajouter, renommer au double-clic,
  supprimer au survol avec confirmation) ; Scène = `Stage` en mode
  aperçu sur l'image recadrée ; Planche = `ExampleBoard`.
- `useFieldEditing` réutilisé tel quel ; `Stage` gagne une prop
  `mode: "deck" | "preview"` qui coupe sélection, glisser et grille.
- La fiche de réglages (t-488) reste pour les paramètres non texte et
  suit le cas courant.

## Tests

- Rust : `examples` aller-retour JSON et hachage de version ; renommage
  et suppression d'un widget ; valeurs hors schéma ignorées ; les
  requêtes refusées hors du mode modèles ; `preview_widget` rend des
  champs avec les bonnes boîtes et des alertes « vide » / « déborde ».
- UI : pastilles (ajout, renommage, suppression) ; édition d'un champ
  dans la scène → `set_example_value` ; planche : une vignette par cas,
  alertes, clic → scène sur ce cas.
- e2e : créer un widget, ajouter un cas « Long », taper un titre long
  dans la scène, voir « Déborde » dans la planche ; relancer et
  retrouver les cas.

## Découpage

1. Session : `examples` dans le manifeste, requêtes, suivi des
   renommages et suppressions, suppression des essais.
2. Session : aperçu avec `SlideView`, décalage et alertes.
3. UI : pastilles des cas et fiche de réglages branchée sur le cas.
4. UI : `Stage` en mode aperçu, édition sur place.
5. UI : planche, bascule Scène / Planche, `UiState`.
6. Libellés (20 langues), e2e.
