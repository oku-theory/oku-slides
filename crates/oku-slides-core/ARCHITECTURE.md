# oku-slides-core

Modèle du deck, opérations, replica Loro (CRDT), validation et zip des
sources. Aucune dépendance à Typst ni à Tauri.

## Types clés

- `Deck` (`model.rs`) : slides et éléments génériques `{id, kind, params}`
  (`Params`/`ParamValue`, `params.rs`), thème, `vars`, `templates` ; schéma JSON (`schemars`).
- `WidgetRegistry` (`widget.rs`) : le schéma des widgets (`WidgetSchema`, `ParamSchema`,
  `ParamType`, `Category`, `HostType`, `SlotMode`) déclarés côté Typst, lu par le rendu ; les
  ops et `validate` s'y réfèrent pour ce qu'un `kind` accepte.
- `Op` + `apply_all` (`ops/mod.rs`) : opérations d'édition, appliquées atomiquement sur un
  `Deck` cloné avec le `WidgetRegistry` du deck (erreur → rien n'a bougé) ; tout sous-arbre
  inséré est typé contre le registre (`ops/check.rs`).
- `DeckReplica` (`replica/`) : le deck dans un `LoroDoc`, historique complet et identité
  (`deck_uid`). `apply(&Deck, label)` diffuse un `Deck` modifié en une étape d'annulation
  (`UndoManager`, `begin_group`/`end_group`) ; un commit par `mov` (bogue d'annulation, t-263).
  `deck()` : **lecture totale** (sous-arbre illisible → `Issue` et défaut, slots bornés par
  `MAX_SLOT_DEPTH`). `merge` : deux replicas de même `deck_uid`, `MergeReport` (`authors.rs`) ;
  racine `peers` : nom de chaque pair, inscrit à sa première édition. `history` : un `Version`
  par édition ; `restore` = `apply` de `deck_at`, pas de `revert_to` (perd les `mov`).
  Modules : `schema`, `read`/`write` (clés triées), `rich`, `undo`, `authors`, `history`.
- `SourceBundle` (`sources.rs`) : `replica` fait foi, `deck` n'en est qu'une **vue** (rendu,
  `validate`, `locate`) ; `new` refuse un deck plus profond que `MAX_SLOT_DEPTH`, `merge` et
  `restore` passent `validate` avec le registre du bundle. Zip : snapshot Loro, assets en ajout seul.
- `validate` (`validate.rs`) : cohérence du deck (styles, graphiques, identifiants) contre le
  `WidgetRegistry`, signale plutôt qu'il n'échoue quand c'est possible.
- `locate` (`locate.rs`) : chemin d'un élément dans le deck (édition ciblée, agent vocal).
- `canonicalize` (`canonical.rs`) : forme canonique d'un `Deck` (clés triées, slots vides
  supprimés) pour comparer deux decks ou stabiliser un rendu.

## Erreurs

`error.rs` : `OpError`, `ReplicaError` (`Corrupted`, `TooLarge`, `Write`, `Unrelated`),
`SourcesError`, `TransactionError` — Snafu, `Localize`, messages Fluent.

## Points d'extension

- Un nouveau champ du modèle : `model.rs`, classé dans une catégorie du replica
  (`schema.rs`), couvert dans `replica/read_tests.rs` / `mod_tests.rs`.
- Une nouvelle opération : `Op` (`ops/mod.rs`) + `ops_schema`, appliquée sur `Deck`
  (jamais directement sur le replica, `apply` s'en charge).

## Dépendances

`oku-slides-i18n`, `oku-slides-chart` (schéma des graphiques), `loro`, `zip`,
`sha2`/`hex` (identité des assets) ; dev : `speculoos`, `rstest`, `proptest`.
