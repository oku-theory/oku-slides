# ADR-0006 : Le deck en un seul arbre Loro, suppression douce

- **Statut :** accepté
- **Date :** 2026-10-10
- **Tâches :** t-746, t-747
- **Remplace en partie :**
  [ADR-0005](0005-elements-d-une-slide-en-arbre-loro.md) (un arbre par
  slide, `@slot` par id de parent, suppression par `tree.delete`) et
  [ADR-0002](0002-sources-du-deck-en-crdt-loro.md) (slides en
  `LoroMovableList`)
- **Sonde :** `crates/oku-slides-core/tests/loro_deck_tree_probe.rs`
  (complète `loro_tree_probe.rs`)

## Contexte

- **Un arbre par slide.** Passer un élément d'une slide à l'autre est une
  copie suivie d'une suppression : l'édition concurrente est perdue.
  L'interface ne déplaçait d'ailleurs jamais un élément d'une slide à l'autre :
  couper retirait l'élément, coller posait une copie aux ids renouvelés.
- **`tree.delete`.** Loro ne ressuscite jamais un nœud. Défaire un retrait
  recrée une copie, et le `mov` ou l'édition concurrente d'un pair est perdue.
- **Slides en `LoroMovableList`.** Défaire un retrait face au `mov` d'un pair
  laisse deux slides (t-652), et la parade t-263 impose un commit par `mov`.

## Décision

- **Un seul `LoroTree`**, sous `deck.slides`. Les racines, et elles seules,
  sont les slides : leur méta est la slide sans ses slots, plus
  `@slide = true`. Les descendants sont les éléments. Ordre fractionnaire,
  jitter 0.
- **`@slot`** est une carte **fusionnable** (`ensure_mergeable_map`) :
  { `TreeID` du parent, en `counter@peer` → nom du slot }, à tous les
  niveaux (slide, élément, élément imbriqué). Deux pairs qui la créent en
  parallèle gardent leurs deux entrées (t-188).
- **Suppression douce.** Retirer un nœud pose une marque `@deleted/<pair>` =
  `true` dans sa méta, jamais `tree.delete`. La marque est par pair, et non
  un booléen : avec un booléen, l'annulation de A effacerait la suppression
  concurrente de B (sonde, `p1_concurrent_double_soft_delete_then_one_undo`
  contre `p1_per_peer_mark_keys_survive_a_concurrent_undo`). Un nœud est
  supprimé tant qu'il porte une marque. La réconciliation ne retire jamais une
  marque, et un id réajouté crée un nœud neuf.
- **Lecture et écriture ne descendent jamais sous un nœud marqué** : un nœud
  vivant sous un parent marqué est caché avec lui.
- **Entrée stricte.** À l'import et à la fusion, sont refusés (`Corrupted`) :
  une `@slide` hors racine, une marque ou un `@slot` mal formé. Seul ce
  qu'aucun chemin du code ne produit, annuler et rétablir compris, est
  refusé ; chaque refus se justifie ainsi. Une racine sans `@slide` est
  acceptée et ignorée avec son sous-arbre : c'est le nœud vide que Loro crée en
  défaisant un `mov` dont la cible a disparu.
- **Appariement.** L'identité d'un objet est l'`id` de sa méta, jamais le
  `TreeID`. Les slides s'apparient d'abord, puis les éléments dans leur slide,
  puis d'une slide à l'autre, **seulement** pour un déplacement annoncé par
  `move_element`, ou déduit sans ambiguïté : un id unique des deux côtés, entre
  deux slides gardées, de même `kind`. Un nœud marqué n'est jamais apparié.
- **Un déplacement annoncé fait foi**, même si la slide d'origine est retirée
  dans la même transaction (vider une slide dans une autre, puis la retirer) :
  l'élément en sort avec l'édition concurrente du pair, et ses descendants
  non voulus sont marqués. Une slide retirée n'est jamais l'origine d'un
  déplacement déduit.
- **Chemins qui produisent un déplacement entre slides :** l'op
  `move_element` (outil `apply` de l'agent, couper-coller de l'interface), et
  un `apply` qui retire un id d'une slide et l'ajoute dans une autre, sous les
  conditions du déplacement déduit.
- **La parade t-263 ne reste que pour les listes** (items, `dataset`).

### Lecture

- **Nœud vide.** Un nœud dont la méta n'a aucune clé est ignoré avec son
  sous-arbre, sans `Issue` : il naît d'un chemin normal (deux annulations
  croisées) et ne porte aucune donnée.
- **Slot résolu.** Les clés de `@slot` sont relues par `TreeID::try_from` et
  ne comptent que si elles sont canoniques. Quand le parent a changé (parent
  recréé par « rétablir »), la réparation est silencieuse si les clés des
  parents disparus donnent toutes le même slot ; les entrées de parents
  vivants mais périmés sont ignorées. Sinon `issue-slot-repaired` est émise
  (sonde, `p9_tree_id_string_key_is_stable_across_snapshot_and_peers`).
- **Ordre des slides.** Les racines non marquées qui portent `@slide = true`,
  dans l'ordre de Loro.

## Compatibilité

Aucune. `REPLICA_FORMAT` passe de 2 à 3 et `SOURCES_VERSION` de 8 à 9.

- Un `deck.loro` v8 relu hors zip est refusé
  (`replica-unsupported-version`), et la reprise de session est mise de côté
  (`.failed-*`).
- Le JSON du deck ne change pas (`DECK_VERSION` reste 5). Le comportement de
  « couper » change : couper marque l'élément, coller le déplace.

## Conséquences

- Un déplacement entre slides, ou entre slots, garde les éditions
  concurrentes.
- Annuler un retrait rend le même nœud, avec le `mov` et l'édition d'un pair,
  même reçus entre le retrait et l'annulation
  (`p1_undo_soft_delete_after_receiving_move_does_not_move_back`).
- Un enfant qu'un pair a déplacé sous un parent retiré revient avec lui à
  l'annulation (`p3_move_under_soft_deleted_parent_reappears_on_undo`).
- Réordonner les slides se fait en un commit.
- Coût mesuré par la sonde (`p8_cost_single_tree_vs_tree_per_slide`, deck de
  40 slides × 20 éléments, une exécution debug et une release) : construction
  6-7 ms contre 4-15 ms pour un arbre par slide, lecture brute de l'arbre
  0,4-1,0 ms contre 0,6-1,3 ms, snapshot après 200 passages d'environ 100 Ko
  contre 120 Ko. Ces chiffres sont indicatifs et ne couvrent pas les nœuds
  marqués. La lecture complète du deck (`deck()`, 40×20, release) mesure
  environ 6,5 ms, comme avant la migration.

## Limites

- **Les nœuds marqués ne sont jamais libérés.** Le snapshot grandit, et la
  lecture comme chaque `apply` coûtent O(vivants + marqués), car `children()`
  rend aussi les marqués. `max_replica_bytes` n'est vérifié qu'à l'import : un
  deck très retouché peut un jour être refusé à la reprise. Compaction : t-770.
- **Le contrôle d'entrée (`check_trees`) coûte O(tous les nœuds)**, marqués
  et descendants de marqués compris, à chaque import et chaque fusion. Mesure
  de la revue finale, 40×20 après environ 10 000 nœuds marqués :
  `from_snapshot` 69 ms contre 22 ms avant la migration, `merge` 96 ms contre
  55 ms.
- Une marque d'une session close ne s'annule plus : la pile d'annulation repart
  de zéro à la reprise.
- **Défaire un ajout supprime le nœud pour de bon**, car Loro inverse `create`
  par `delete` : un sous-arbre qu'un pair a déplacé dessous disparaît avec lui.
  Aucun réglage simple ne l'évite, l'inverse d'un `create` appartient à
  l'`UndoManager`. Suivi : t-776.
- **Régression visible, acceptée.** B crée une slide ; A y déplace un
  élément existant par couper-coller (`move_element`) ; B fait ⌘Z. L'élément
  est perdu pour de bon, et le ⌘Z de A ne le rend pas : Loro recrée un nœud
  vide, ignoré. Avant cette décision, couper retirait l'élément et coller
  posait une copie, et le ⌘Z de A le rendait. Cause : défaire un ajout reste
  un `tree.delete` de Loro, qui emporte le sous-arbre déplacé dessous.
  Suivi : t-776, en haute priorité.
- Un élément déplacé sous un élément, ou dans une slide, retiré en parallèle
  est caché avec lui ; l'annulation du retrait le rend.
- Sans `move_element`, un id qui figure dans plusieurs slides n'est jamais
  apparié d'une slide à l'autre : son édition concurrente suit la règle
  d'avant (perdue).
- Retirer un id d'une slide et l'ajouter dans une autre, dans un même
  `apply`, vaut déplacement quand le déplacement est déduit : l'édition
  concurrente du pair suit le nœud.
- Un id réajouté, puis l'annulation de l'ancien retrait, laissent deux nœuds
  au même id, réparés à la lecture (t-652).
- Le jitter 0 et le `Rearrange` implicite touchent désormais aussi **l'ordre
  des slides** : deux insertions concurrentes de slides au même rang, puis une
  insertion entre elles, peuvent ramener une slide qu'un pair déplaçait.
- Un replica forgé dont l'arbre est mal formé est refusé en bloc, à l'import
  comme à la fusion.
- **Limite de confiance.** Un pair peut supprimer n'importe quel nœud, slide
  ou élément, par une marque `@deleted/<pair>`, comme il le pouvait par
  `tree.delete`. La marque est bien formée, donc acceptée. Seule l'annulation
  du pair qui l'a posée la retire : la marque d'un pair absent, ou d'une
  session close, cache le nœud pour de bon, sans `Issue`.
- Un nœud vide que Loro crée en défaisant un `mov` (cible supprimée entre-temps)
  reste dans l'arbre, ignoré, et s'ajoute aux nœuds morts.
- Loro reste figé par `Cargo.lock` (1.16.2). Pas de `cargo update -p loro`
  sans relancer `tests/loro_tree_probe.rs` et
  `tests/loro_deck_tree_probe.rs`.
