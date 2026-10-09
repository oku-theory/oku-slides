# replica

Le deck dans un `LoroDoc` (CRDT) : historique complet, fusion, annuler.
`DeckReplica` fait foi ; le `Deck` n'en est qu'une vue.

## Modules

- `mod.rs` : `DeckReplica` (`new`, `from_snapshot`, `apply`, `merge`, `restore`, annuler).
- `schema` : catégorie (`Kind`) de chaque champ, selon son contexte (`Ctx`) et le registre.
- `write` : `Writer`, réconciliation des conteneurs vers un `Deck` JSON.
- `read` : lecture totale, sans registre ; l'illisible est écarté et signalé (`Issue`).
- `host` : option d'un paramètre `host(chart|table)`, `dataset` en colonnes et lignes.
- `rich` : paragraphes riches dans un `LoroText` à marques.
- `undo` : étapes d'annulation, groupes, clés de fusion.
- `authors` : nom de chaque pair (racine `peers`), `MergeReport`.
- `history` : une `Version` par édition, aperçu d'une version passée.

## Catégories (`Kind`)

- `Prose` : `LoroText`, fusion au caractère.
- `Rich` : `LoroMap { rich: LoroText }` à marques.
- `Collection` : `LoroMovableList` de `LoroMap` à `id` (slides, slots).
- `List` : `LoroMap { list: LoroMovableList }`, items d'un paramètre `list`.
- `Host` : `LoroMap` ; `dataset` en `{columns, rows}`, `rows` de `{id, cells}`,
  `cells` par identifiant de colonne ; autres clés opaques ou LWW. Un `dataset`
  d'une autre forme (lignes irrégulières, clé en trop, identifiants non
  alignés) s'écrit opaque, sans perte, et ne fusionne pas finement.
- `Atom` : valeur JSON entière, LWW. `Nested` : objet en `LoroMap`, sinon LWW.

## Règles

- **Diff du JSON du deck** : `apply` sérialise le `Deck`, puis amène chaque
  conteneur à son état. Rien n'est écrit quand rien ne change.
- **Lecture sans registre** : le type se lit dans le conteneur. Le marqueur
  `{columns, rows}` est lu sous la clé `dataset` d'un paramètre à valeur de
  carte, et là seulement ; sûr, car seul un paramètre d'hôte s'écrit en carte.
- **Réparation à la relecture** : colonne en double, et ligne en double aux
  mêmes cellules, écartées ; cellule absente ou conteneur lue `null` ; autre
  identifiant en double ou invalide remplacé (`r{i}`/`c{j}`, puis `-{n}`),
  bornes de 10 000 lignes, 1 000 colonnes et 1 000 000 cellules
  (`issue-data-truncated`), profondeur bornée par `MAX_DEPTH` ; un `dataset`
  hors de ces bornes s'écrit en `Opaque`.
- **Parade t-188** : les conteneurs déclarés sont créés d'avance, vides ; deux
  premières éditions concurrentes visent le même conteneur.
- **Parade t-263** : chaque `mov` part dans son propre commit (`split`) ; Loro
  défait mal plusieurs `mov` d'une même étape.
- **Clés triées** à la lecture, comme la forme canonique.
