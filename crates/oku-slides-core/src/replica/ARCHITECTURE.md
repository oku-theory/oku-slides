# replica

Le deck dans un `LoroDoc` (CRDT) : historique complet, fusion, annuler.
`DeckReplica` fait foi ; le `Deck` n'en est qu'une vue.

## Modules

- `mod.rs` : `DeckReplica` (`new`, `from_snapshot`, `apply`, `merge`, `restore`, annuler).
- `schema` : catégorie (`Kind`) de chaque champ, selon son contexte (`Ctx`) et le registre.
- `write` : `Writer`, réconciliation des conteneurs vers un `Deck` JSON ; `tree` : arbre des éléments.
- `read` : lecture totale, sans registre ; l'illisible est écarté et signalé (`Issue`).
- `host` : option `host(chart|table)` ; `rich` : paragraphes dans un `LoroText` à marques.
- `undo` : étapes d'annulation, groupes, clés de fusion.
- `authors` : nom de chaque pair (racine `peers`), `MergeReport`.
- `history` : une `Version` par édition, aperçu d'une version passée.

## Catégories (`Kind`)

- `Prose` : `LoroText`, fusion au caractère. `Rich` : `LoroMap { rich: LoroText }` à marques.
- `Collection` : `LoroMovableList` de `LoroMap` à `id` (slides).
- `Slot` : nœuds du `LoroTree` de la slide (`@elements`) ; méta = l'élément sans ses slots,
  `@slot` = carte { id du parent (`""` : la slide) → slot } ; ordre fractionnaire (ADR-0005).
- `List` : `LoroMap { list: LoroMovableList }`, items d'un paramètre `list`.
- `Host` : `LoroMap` ; `dataset` en `{columns, rows}`, `rows` de `{id, cells}`, `cells` par
  colonne ; autres clés LWW. Un `dataset` d'une autre forme s'écrit opaque, sans perte.
- `Atom` : valeur JSON entière, LWW. `Nested` : objet en `LoroMap`, sinon LWW.

## Règles

- **Diff du JSON du deck** : `apply` amène chaque conteneur à l'état du `Deck` sérialisé ;
  rien n'est écrit quand rien ne change (arbre : ordre comparé par parent et slot).
- **Lecture sans registre** : le type se lit dans le conteneur ; `{columns, rows}` ne se
  lit que sous `dataset` d'un paramètre à valeur de carte (seul un hôte s'y écrit).
- **Réparation à la relecture** : colonne en double, et ligne en double aux mêmes
  cellules, écartées ; cellule absente lue `null` ; autre identifiant en double ou invalide
  remplacé (`r{i}`/`c{j}`, puis `-{n}`) ; bornes de 10 000 lignes, 1 000 colonnes et
  1 000 000 cellules (`issue-data-truncated`), au-delà `Opaque` ; profondeur bornée par
  `MAX_DEPTH`, arbre par `MAX_SLOT_DEPTH` (écriture comprise). Slides, éléments d'une slide
  (tout l'arbre) ou items au même id (t-652) : copie exacte d'un voisin écartée, autre
  occurrence renommée `{id}-{n}` ; id vide laissé. Sans `@slot` pour son parent : slot d'un
  frère, sinon slot inscrit, sinon `orphans` (`issue-slot-repaired`) ; jamais écarté.
- **Parade t-188** : conteneurs déclarés et arbre d'une slide créés d'avance, vides.
- **Parade t-263** : un commit par `mov` de slide ou d'item (`split`) ; l'arbre, sans commit par `mov`.
- **Format** : `meta.format = 2`, vérifié à l'import et à la fusion ; changer ce format
  impose de changer `SOURCES_VERSION`.
- **Clés triées** à la lecture, comme la forme canonique.
