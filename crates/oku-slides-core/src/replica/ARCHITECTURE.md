# replica

Le deck dans un `LoroDoc` (CRDT) : historique complet, fusion, annuler. `DeckReplica` fait foi.

## Modules

- `mod.rs` : `DeckReplica` (`new`, `from_snapshot`, `apply`, `merge`, `restore`).
- `schema` : catégorie (`Kind`) de chaque champ, selon son contexte (`Ctx`) et le registre.
- `write` : `Writer`, réconciliation des conteneurs vers un `Deck` JSON.
- `tree` : éléments, placement par (parent, slot) ; `tree/deck` : slides, appariement.
- `read` : lecture totale, sans registre ; l'illisible est écarté et signalé (`Issue`).
- `host` : option `host(chart|table)` ; `rich` : paragraphes en `LoroText` à marques.
- `undo` : annulation, groupes, clés de fusion, rang de l'étape du sommet (`undo_step`) ; `authors` : pairs, `MergeReport`.
- `compact` : compaction à l'ouverture (ADR-0007), seule à faire un `tree.delete`.
- `entry` : rejeu, arbre unique, profondeur bornée (t-809) ; `Ancestry` : une remontée par nœud (t-865).
- `history` : une `Version` par édition ; une version passée se lit par rejeu (`updates_till`
  dans un document neuf, contrôlé), jamais par `checkout` (t-809).

## Catégories (`Kind`)

- `Prose` : `LoroText`, fusion au caractère. `Rich` : `LoroMap { rich: LoroText }`.
- `Slides` : un seul `LoroTree` (`slides`) ; racines = slides (méta = la slide, `@slide`),
  descendants = éléments ; ordre fractionnaire (ADR-0006).
- `Slot` : nœuds éléments ; méta = l'élément sans ses slots, `@slot` = carte fusionnable
  { `TreeID` du parent → slot }. `List` : `LoroMap { list: LoroMovableList }` (aussi en colonne).
- `Host` : `LoroMap` ; `dataset` en `{columns, rows}` (`rows` de `{id, cells}`), autres clés
  LWW ; autre forme de `dataset` : opaque. `Atom` : JSON entier, LWW. `Nested` : `LoroMap`.

## Règles

- **Diff du JSON du deck** : `apply` amène chaque conteneur à l'état du `Deck` sérialisé ;
  rien n'est écrit sans changement.
- **Suppression douce** : un nœud retiré reçoit `@deleted/<pair>` = l'heure Unix de sa pose et se
  cache avec son sous-arbre ; annuler retire la marque ; id réajouté : nœud neuf ; création
  annulable née marquée (t-776) ; seule la compaction fait `tree.delete`.
- **Entrée stricte** : `@slide` hors racine, marque ou `@slot` mal formés : refus. L'état d'un
  snapshot reçu n'est jamais lu : ses opérations sont rejouées (shallow refusé), puis tout arbre
  hors de `deck.slides` et tout nœud, supprimé compris, au-delà de 1024 niveaux sont refusés (t-809),
  et `snapshot` refuse (`TooDeep`) d'enregistrer un tel arbre (t-866).
- **Lecture sans registre** : le type se lit dans le conteneur ; `{columns, rows}` ne se lit
  que sous `dataset` d'un paramètre à valeur de carte ; clés triées (forme canonique).
- **Réparation à la relecture** : colonne ou ligne (mêmes cellules) en double écartée ; cellule
  absente lue `null` ; autre id en double ou invalide remplacé (`r{i}`/`c{j}`, puis `-{n}`) ;
  bornes 10 000 lignes, 1 000 colonnes, 1 000 000 cellules (`issue-data-truncated`), au-delà
  `Opaque` ; profondeur par `MAX_DEPTH`, éléments par `MAX_SLOT_DEPTH`. Slides, éléments ou
  items au même id (t-652) : copie exacte écartée, autre occurrence renommée `{id}-{n}`. Sans
  `@slot` du parent : slot d'un frère, sinon slot inscrit, sinon `orphans` (`issue-slot-repaired`).
  Nœud vide (né d'une annulation) : ignoré.
- **Parades** : t-188, conteneurs et arbre créés d'avance ; t-263, un commit par `mov` de liste.
- **Format** : `meta.format = 4`, vérifié à l'import et à la fusion ; changer `SOURCES_VERSION` avec.
