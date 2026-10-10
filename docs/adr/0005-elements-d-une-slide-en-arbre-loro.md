# ADR-0005 : Les éléments d'une slide en arbre Loro

- **Statut :** accepté
- **Date :** 2026-10-09
- **Tâches :** t-694 (t-740 à t-745)
- **Remplace en partie :** [ADR-0002](0002-sources-du-deck-en-crdt-loro.md)
  (les slots en `LoroMovableList`)
- **Remplacée en partie par :** [ADR-0006](0006-deck-en-un-seul-arbre-suppression-douce.md)
  (un arbre pour tout le deck, `@slot` par `TreeID`, suppression douce)
- **Sonde :** `crates/oku-slides-core/tests/loro_tree_probe.rs`

## Contexte

Un slot (`blocks`, `elements`, `children`) est une `LoroMovableList` de
`LoroMap`. Changer un élément de slot, ou de profondeur, revient donc à le
retirer d'une liste et à l'insérer dans une autre : l'identité Loro est
perdue, et l'édition concurrente de l'élément déplacé aussi.

Par ailleurs, Loro 1.16.2 défait mal plusieurs `mov` d'un même commit (t-263).
Le replica écrit donc un commit par `mov`.

## Décision

- **Un `LoroTree` par slide**, sous la clé `@elements` de la carte de la
  slide. Les racines sont les éléments des slots de la slide, les enfants
  d'un nœud sont les éléments des slots de cet élément.
- **La méta d'un nœud est la carte de l'élément**, sans ses slots.
- **`@slot`**, dans la méta, est une carte { id du parent (`""` : la slide)
  → nom du slot }. Il y a une entrée par parent, jamais effacée, et seule
  celle du parent réel compte : les autres servent de repli.
- **Ordre fractionnaire**, avec un jitter de 0.
- **Placement par ordre relatif** dans chaque (parent, slot), par
  `mov_after` et `mov_before`. Les nœuds d'une plus longue sous-suite
  croissante ne bougent pas, pour ne pas toucher ceux qu'un pair déplace.
- **Réconciliation par id d'élément**, jamais par `TreeID`. L'identité d'un
  élément est l'`id` de sa méta.
- **Le JSON du deck ne change pas.** Slides et items de liste restent en
  `LoroMovableList`.

### Réparation à la relecture

La lecture est totale et ne se fie pas au registre. Le slot résolu d'un
nœud sous son parent réel est, dans l'ordre :

1. `@slot[pid]`, si c'est une chaîne non vide ;
2. sinon, le slot du premier frère qui en a un ;
3. sinon, la première valeur valide de `@slot`, dans l'ordre des clés ;
4. sinon `orphans`.

Les cas 2 à 4 émettent `issue-slot-repaired`. Un élément n'est jamais écarté
pour son slot.

- **Ids en double.** `unique_ids` s'applique à tout l'arbre (t-652) : une
  copie exacte est écartée dans un même slot, et une occurrence d'un autre
  slot est renommée `{id}-{n}`.
- **Profondeur.** Elle est bornée par `MAX_SLOT_DEPTH` à la lecture **et** à
  l'écriture : un deck JSON plus profond est refusé par `SlotsTooDeep`, avant
  toute écriture dans la slide. `issue-slot-too-deep` est émise une fois par
  nœud.
- **Éléments non rendus.** Un élément rangé dans `orphans`, ou dans un slot
  que son parent ne déclare pas, est gardé dans le JSON mais n'est pas
  rendu : la mise en page ne lit que les slots déclarés. `validate` signale
  `issue-unknown-param`, et l'`apply` suivant inscrit la réparation
  (`@slot[pid] = "orphans"`). Une réparation à la relecture s'écrit donc une
  fois ; l'invariant « rien n'est écrit quand rien ne change » vaut à partir
  du second `apply`.

## Compatibilité

Aucune. `SOURCES_VERSION` passe de 7 à 8, et `DECK_VERSION` ne change pas
(5).

- Le marqueur `meta.format = 2` est vérifié à l'import et à la fusion.
- Un `deck.loro` de reprise v7 est refusé (`replica-unsupported-version`) et
  la session met son dossier de côté (`.failed-*`). Les modifications non
  enregistrées d'une session reprise sont donc gardées, pas relues, au
  premier lancement après la mise à jour.
- Règle : changer `REPLICA_FORMAT` impose de changer `SOURCES_VERSION`.

## Conséquences

- Un déplacement entre slots ou entre profondeurs garde les éditions
  concurrentes de l'élément.
- La parade t-263 disparaît pour les éléments : l'arbre s'écrit sans commit par
  `mov`. Elle reste pour les items de liste seulement.

## Limites

- Un élément déplacé sous un élément supprimé en parallèle est caché avec lui ;
  l'annulation du retrait le rend (ADR-0006) (`loro_tree_probe.rs`, point 3a).
- Deux imbrications croisées qui formeraient un cycle : le premier `mov`
  dans l'ordre (lamport, pair) gagne. Le perdant reste visible à son ancienne
  place, car son entrée `@slot` pour l'ancien parent est intacte.
- Avec un jitter de 0, deux insertions concurrentes au même endroit reçoivent
  le même fractionnaire. Une insertion entre elles fait émettre par Loro un
  `Rearrange`, c'est-à-dire des `mov` implicites de frères, qui peuvent
  ramener un frère qu'un pair déplaçait en parallèle.
- Loro est figé par `Cargo.lock` (1.16.2) : pas de `cargo update -p loro`
  sans relancer les sondes (`tests/loro_tree_probe.rs`,
  `tests/loro_deck_tree_probe.rs`).
