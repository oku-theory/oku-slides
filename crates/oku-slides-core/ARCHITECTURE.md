# oku-slides-core

Modèle du deck, opérations, replica Loro (CRDT), validation et zip des
sources. Aucune dépendance à Typst ni à Tauri.

## Types clés

- `Deck` (`model.rs`) : slides, éléments, thème, `params`, `vars`,
  `templates` ; schéma JSON (`schemars`).
- `Op` + `apply_all` (`ops.rs`) : les opérations d'édition, appliquées
  atomiquement sur un `Deck` cloné (erreur → rien n'a bougé).
- `DeckReplica` (`replica/`) : le deck dans un `LoroDoc`, historique complet et identité
  (`deck_uid`). `reconcile(&Deck)` diffuse les changements d'un `Deck` modifié (pas de
  traduction des `Op`) ; `commit(label)` clôt une transaction — un commit = un *change* Loro,
  `label` en message (`set_change_merge_interval(-1)`) ; `deck()` fait une **lecture totale**
  (jamais d'erreur : sous-arbre illisible → `Issue` et valeur par défaut). `UndoManager`
  (`undo.rs`) non persisté pour annuler / rétablir ; `begin_group`/`end_group` : un tour de
    l'agent = une étape. `merge` réconcilie deux replicas de même
  `deck_uid`, rend un `MergeReport` (`authors.rs`) ; racine `peers` : nom de chaque pair,
  inscrit à sa première édition (réglage `[author] name`, sinon utilisateur système).
  Modules : `schema` (catégories de champs), `read`/`write` (aller-retour `Deck` ↔ `LoroDoc`),
  `rich` (texte à marques), `undo`, `authors`.
- `SourceBundle` (`sources.rs`) : `replica` fait foi, `deck` n'est qu'une
  **vue** matérialisée tenue à jour par `apply` — utilisée par le rendu,
  `validate`, `locate`. Zip des sources (`to_zip` / `from_zip`) : replica en
  snapshot Loro, assets en ajout seul.
- `validate` (`validate.rs`) : cohérence du deck (styles, graphiques,
  identifiants), signale plutôt qu'il n'échoue quand c'est possible.
- `locate` (`locate.rs`) : chemin d'un élément dans le deck (édition ciblée,
  agent vocal).

## Erreurs

`error.rs` : `OpError`, `ReplicaError` (`Corrupted`, `TooLarge`, `Write`,
`Unrelated`), `SourcesError`, `TransactionError` — Snafu, `Localize`,
messages Fluent.

## Points d'extension

- Un nouveau champ du modèle : l'ajouter dans `model.rs`, le classer dans
  une des quatre catégories du replica (`schema.rs`), couvrir l'aller-retour
  dans `replica/read_tests.rs` / `mod_tests.rs`.
- Une nouvelle opération : `Op` (`ops.rs`) + `ops_schema` ; elle s'applique
  sur `Deck`, jamais directement sur le replica (`reconcile` s'en charge).

## Dépendances

`oku-slides-i18n`, `oku-slides-chart` (schéma des graphiques), `loro`, `zip`,
`sha2`/`hex` (identité des assets) ; dev : `speculoos`, `rstest`, `proptest`.
