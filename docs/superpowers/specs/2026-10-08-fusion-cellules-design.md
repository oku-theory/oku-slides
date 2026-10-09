# Fusion fine des éditions de cellules — design (v2)

Date : 2026-10-08 · Tâche : t-589 · ADR : 0004 (remplace en partie l'ADR-0002)
Révisé après la revue d'équipe du plan v1 et les choix de l'utilisateur :
- pas de rétrocompatibilité ;
- réparation à la relecture ;
- identifiants de colonnes dès maintenant ;
- un collage garde les identifiants par position.

## Objectif

Deux personnes qui modifient en même temps les données d'un tableau ou d'un
graphique gardent toutes les deux leurs modifications. Aujourd'hui, un
paramètre `host(chart)` ou `host(table)` est un `Atom` Loro : la dernière
écriture remplace toute l'option.

Cas visés, qui doivent tous fusionner sans perte :
- deux cellules différentes, même dans la même ligne ou la même colonne ;
- deux insertions de ligne, ou deux insertions de colonne ;
- une suppression de ligne d'un côté, une cellule d'une autre ligne
  modifiée de l'autre ;
- une colonne insérée d'un côté, une cellule modifiée de l'autre ;
- **une ligne insérée d'un côté, une colonne insérée de l'autre.** La
  nouvelle ligne n'a pas de valeur dans la nouvelle colonne, et ses valeurs
  ne sont pas décalées.

Hors périmètre :
- la fusion caractère par caractère dans une cellule. Une cellule est un
  scalaire, la dernière écriture gagne ;
- la fusion fine des autres clés de l'option (`series`, couleurs, drapeaux),
  qui restent en dernière écriture gagnante, clé par clé.

## Décisions

- **Identifiants stables de lignes et de colonnes**, dans deux tableaux
  parallèles du `dataset` :

  ```json
  { "source": [["Pays","Ventes"],["FR",10]], "ids": ["h3k9…","a71c…"], "columns": ["c0f2…","b8e1…"] }
  ```

  - `source` garde le format ECharts.
  - `ids` a la longueur de `source`. La ligne d'en-tête a, elle aussi, un
    identifiant.
  - `columns` a la largeur de la ligne 0.
- **C'est le cœur qui crée les identifiants** : 8 caractères hexadécimaux pris
  dans un `uuid` v4, en tirant à nouveau en cas de collision avec un
  identifiant existant.
  - Une donnée qui arrive sans identifiants les reçoit à la première écriture
    par le cœur. C'est le cas d'un graphique écrit par le LLM, d'un tableau de
    départ ou d'un `set_param`. Le point de passage obligé est
    `ops/host.rs::host_option`, avec `chart_data` et `table_data`.
  - Le deck stocké porte **toujours** `ids` et `columns` pour chaque paramètre
    d'hôte qui a un dataset. Le replica relu est donc identique au deck écrit
    (`debug_assert_view_matches_replica`).
- **Pas de rétrocompatibilité.** `DECK_VERSION` passe de 4 à 5, et
  `SOURCES_VERSION` de 6 à 7. Un deck ou un zip plus ancien est refusé avec
  l'erreur de version existante. Il n'y a aucun chemin de lecture de la
  forme `Atom`, et les fixtures sont migrées.
- **Ops**, avec le même JSON qu'aujourd'hui, plus des champs facultatifs :
  - `InsertRow { at, cells, id? }` et `InsertColumn { at, cells, id? }` :
    sans `id`, un identifiant neuf est tiré ;
  - `RemoveRow` et `RemoveColumn` retirent l'identifiant ;
  - `Replace { source }` garde les identifiants **par position**
    (lignes `0..min(ancien, nouveau)`, colonnes de même) et en tire de neufs
    pour l'excédent ;
  - les inverses ne sont pas exposés : `DataOp::apply` n'en rend plus. Le
    cœur et la session ne s'en servaient pas, puisque l'historique passe par
    le replica. Les tests d'inverse du crate deviennent des tests d'état.
  - `id` est documenté dans le schéma LLM comme facultatif (« ne pas le
    fournir »).
- **Réparation à la relecture** (`replica/host.rs`, lecture). Après une
  fusion, l'état relu est remis en règle :
  - une cellule absente d'une ligne vaut `null` ;
  - les doublons d'une même ligne ou colonne sont d'abord écartés : une
    colonne dont l'identifiant est déjà vu, une ligne de même identifiant et
    aux mêmes cellules qu'une ligne gardée (double retrait puis double
    annulation) ; la prochaine écriture les retire du replica ;
  - un autre identifiant de ligne ou de colonne en double, ou invalide, est
    remplacé par un identifiant déterministe tiré de la position
    (`r{i}`/`c{j}`, puis un suffixe numérique s'il est déjà pris) ;
  - `align` plus long que les colonnes est tronqué. `parse_table` le tronque
    au lieu de refuser.

  Un tableau qui dépasse 100 × 20 après fusion reste refusé par la
  validation : il s'affiche en substitut. La suppression d'une ligne ou d'une
  colonne reste permise, ce qui suffit à le rétablir.
- **Robustesse aux données non fiables** :
  - `DataOp::apply` ne panique jamais : longueurs vérifiées avant tout
    accès ;
  - la lecture borne la profondeur (`MAX_DEPTH`) et le nombre de lignes et de
    colonnes (10 000 × 1 000), avec un signalement ;
  - un dataset JSON qui n'a pas la forme attendue (lignes qui ne sont pas des
    listes, clés en trop) est écrit en `Opaque`, sans perte. Il ne fusionne
    pas finement.
- **Graphiques convertis** (`convert`, `canonicalize`) : les identifiants de
  lignes et de colonnes sont gardés si le nombre de lignes, ou de colonnes,
  ne change pas, et tirés à neuf sinon. Un passage en nuage, par exemple,
  regroupe les lignes. `chart_to_table` et `table_to_chart` les gardent.

## Replica (`oku-slides-core/src/replica`)

- `Kind::Host` pour `ParamType::Host { .. }`. L'option est une `LoroMap`, et
  chaque clé hors `dataset` est écrite comme `Nested(Opaque)`.
- `dataset` devient
  `LoroMap { columns: LoroMovableList<id>, rows: LoroMovableList<LoroMap { id, cells: LoroMap<id de colonne, scalaire> }> }`.
  - Les lignes sont rapprochées par identifiant, avec `Writer::sync_list`
    réutilisé tel quel.
  - Les colonnes sont rapprochées par identifiant, dans une liste ordonnée.
  - Les cellules sont rapprochées par clé : `set` si la valeur change,
    `delete` si la colonne a disparu.
  - Rien n'est écrit quand rien ne change.
- La lecture reconstruit `source` dans l'ordre de `columns`, puis applique la
  réparation décrite plus haut.
- Le marqueur est une carte `{columns, rows}` sous la clé `dataset` d'un
  paramètre d'hôte, et seulement là.

## Tests

- **Modèle** :
  - les ops : identifiants tirés, gardés, sans collision ;
  - `Replace` qui garde par position ;
  - pas de panique sur des longueurs incohérentes ;
  - la validation des identifiants (longueur, doublon, format, bornes 64/65) ;
  - les conversions.
- **Cœur** : `host_option` pose les identifiants, et le deck relu est
  identique au deck écrit.
- **Replica** :
  - aller-retour graphique et tableau ;
  - une édition de cellule ne touche qu'une entrée (`len_ops` + 1, même
    conteneur de ligne) ;
  - forme inattendue écrite en `Opaque` ;
  - lecture d'états forgés : doublons, cellules conteneurs, profondeur,
    taille ;
  - annuler et rétablir chaque op.
- **Convergence** :
  - les cinq cas de l'objectif ;
  - mêmes suppressions des deux côtés, puis double annulation, sans
    doublon après fusion ;
  - le proptest étendu (cellules, lignes, colonnes), qui exige qu'après
    fusion `parse_table` réussisse et que `ids` et `columns` soient alignés.

  Ces tests sont écrits avant le replica et doivent échouer sur le code
  actuel.
- **Version** : un deck v4 et un zip v6 sont refusés.
