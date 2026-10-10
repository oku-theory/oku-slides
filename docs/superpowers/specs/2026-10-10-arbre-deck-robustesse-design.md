# Arbre du deck : profondeur, défaire un ajout, compaction — design

- **Date :** 2026-10-10
- **Tâches :** t-809 (haute, sécurité), t-776 (haute), t-770
- **Contexte :** [ADR-0006](../../adr/0006-deck-en-un-seul-arbre-suppression-douce.md)
- **Ordre des lots :** t-809, puis t-776, puis t-770 (t-770 date la marque
  que t-776 pose aussi à la création).

## Décisions de cadrage (utilisateur)

1. Compaction : à l'ouverture, seulement les nœuds dont la marque a plus de
   N jours, N = 30 (réglage).
2. Historique Loro gardé entier : pas de snapshot *shallow*. t-770 vise le
   coût de lecture et d'écriture ; la taille est mesurée et notée.
3. Les nœuds vides (nés d'un `mov` défait) sont compactés par la même règle.

## Lot 1 — t-809 : chaîne d'arbre forgée très profonde

**Problème.** Loro 1.16.2 décode l'arbre récursivement au premier accès ;
une chaîne forgée de ~500 niveaux déborde une pile de 256 Kio en debug. Un
débordement de pile n'est pas rattrapé par `catch_unwind` : un `.slider` ou
un `deck.loro` hostile fait avorter le processus.

**Sonde d'abord** (`tests/loro_depth_probe.rs`, release et debug) : pile
consommée par niveau au décodage, à l'export et au drop ; nombre de niveaux
qu'une chaîne peut atteindre sous `max_replica_mib = 32`. Elle fixe la
taille de pile du fil de garde. **Arrêt :** si la pile nécessaire dépasse
environ 1 Gio de réservation virtuelle, le lot s'arrête et l'utilisateur
décide (G9).

**Garde à grande pile.** `guarded` exécute sa fermeture (import, contrôles,
première lecture) dans un fil dédié, `std::thread::Builder::stack_size`
fixé par la sonde. Le `LoroDoc` accepté revient au fil appelant.

**Borne de profondeur.** `check_trees` mesure, de façon itérative, la
profondeur de chaque nœud non supprimé par Loro. Au-delà de
`MAX_TREE_DEPTH = 64` (racine = 1), le replica est refusé
(`MalformedDeckTree { what: "deck tree too deep" }`, donc `Corrupted`).
64 laisse de la marge à `MAX_SLOT_DEPTH = 16` : deux pairs qui imbriquent en
parallèle additionnent leurs profondeurs. Un document refusé, ou dont la
garde a paniqué, est oublié (`forget`) ou libéré **dans le fil de garde**,
jamais dans l'appelant : son drop est récursif.

Un document accepté a un arbre peu profond : ni l'export, ni la
réconciliation, ni l'annulation, ni le drop ne débordent ensuite.

**Points d'entrée couverts :** `from_snapshot`, l'essai de `merge`,
l'essai de `restore`, et la lecture d'une version passée dans
l'historique (`history.rs`) : un fichier forgé peut cacher une chaîne
profonde dans une version ancienne, aplatie ensuite. La lecture d'une
version passe par la même garde et la même borne ; une version trop
profonde est refusée (`Corrupted`), le panneau d'historique reste utilisable.

**Tests.**
- Régression `crates/oku-slides-core/tests/regressions/issue_0809_deep_tree_chain.rs` :
  sur un fil à pile de 256 Kio, un snapshot à chaîne de plusieurs milliers
  de niveaux est refusé (`Corrupted`) sans débordement ; idem pour `merge`.
- `deep_element_chain_…` (deck_tree_tests) passe à une chaîne de
  `MAX_TREE_DEPTH - 1` niveaux, acceptée sur petite pile.
- Une version d'historique trop profonde est refusée, les autres se lisent.

## Lot 2 — t-776 : défaire un ajout ne perd plus le sous-arbre

**Problème.** Loro inverse `create` par `delete` : défaire un ajout emporte
le sous-arbre qu'un pair a déplacé dessous (B crée une slide, A y déplace un
élément par `move_element`, B fait ⌘Z : l'élément est perdu, le ⌘Z de A ne
le rend pas).

**Naître marqué.** Dans une réconciliation annulable (`apply`,
`apply_moving`), quand `place.rs` doit créer un nœud :

1. ce qui précède est committé en partie (`oku-part:<libellé>`, origine
   normale, annulable) ;
2. le nœud est créé (`create_at`) avec la marque `@deleted/<pair>` du pair
   courant, dans un commit d'origine `SILENT` (exclu de l'`UndoManager`) ;
3. la réconciliation reprend : méta, `@slot` et **retrait de la marque**
   tombent dans les commits annulables de l'étape.

Défaire l'ajout remet la marque (suppression douce) et retire la méta : le
nœud est caché, son sous-arbre aussi, rien n'est perdu. Le ⌘Z de A ramène son
élément vers son ancien parent ; le rétablir de B rend la slide et ce qui
est dessous.

`DeckReplica::new` et `apply_silent` (non annulables) créent directement,
sans marque. Le `Writer` reçoit pour cela une stratégie de création
(directe ou « naître marqué »).

**Coût.** Deux commits de plus par nœud créé, sans cran d'historique
(parties). Mesure sur un collage de 50 éléments (durée de `apply`, items
Loro de l'étape). Si une étape ordinaire dépasse `ITEMS_PER_STEP = 64`
items, la constante est relevée et la raison notée dans `undo.rs`.

**Tests.**
- Régression `tests/regressions/issue_0776_undo_add_keeps_moved_subtree.rs` :
  le scénario ci-dessus ; après le ⌘Z de B, l'élément est caché ; après le
  ⌘Z de A, il est revenu dans sa slide d'origine ; variante : le rétablir de
  B rend la slide avec l'élément.
- Les tests d'annulation existants (undo_tests, soft_delete_tests) restent
  verts : défaire un ajout cache le nœud, rétablir le rend avec le même
  `TreeID`.
- ADR-0006, Limites : retirer « défaire un ajout supprime pour de bon » et
  la régression acceptée ; décrire « naître marqué ».

## Lot 3 — t-770 : compaction des nœuds morts

**Marque datée.** La valeur de `@deleted/<pair>` passe de `true` à l'heure
Unix de la pose, en secondes (`i64`, horloge de l'auteur). Loro n'expose pas
l'op qui a écrit une clé de carte (seulement son dernier auteur) : la date
est donc portée par la valeur. Une marque de naissance (lot 2) porte l'heure
de la création ; défaire l'ajout la remet avec cette heure-là.
`REPLICA_FORMAT` 3 → 4, `SOURCES_VERSION` 9 → 10, sans compatibilité (un
`deck.loro` v3 est refusé, la reprise est mise de côté comme en ADR-0006).
`check_trees` refuse une marque dont la valeur n'est pas un entier.

**Nœud vide daté** par `tree.get_last_move_id(node)` puis
`doc.get_change(id).timestamp`.

**Règle.** À l'ouverture (`from_snapshot`, après la garde et avant
`Undo::new`), un `tree.delete` silencieux retire, avec son sous-arbre :
- tout nœud qui porte **au moins une** marque de plus de N jours (seul le
  pair d'une marque peut la retirer ; sa fenêtre est passée, le nœud resterait
  caché de toute façon) ;
- tout nœud vide daté de plus de N jours.

À l'ouverture, le pair est neuf : toutes les marques viennent de sessions
closes. Les descendants vivants d'un nœud compacté étaient déjà cachés.
**Risque assumé :** une autre copie restée ouverte plus de N jours, qui
annule alors sa marque ou le `mov` d'un élément sous ce nœud, perd ce nœud
à la fusion.

**Réglage.** `compact_after_days = 30` dans `[sources]` de
`config.default.toml` ; passé à `from_snapshot` avec la limite d'octets
(une structure de limites du replica remplace l'argument `max_bytes` seul).
L'heure courante est un paramètre de la fonction interne (tests).

**Historique.** Le commit de compaction est une partie (`oku-part:`,
origine `SILENT`) : pas de cran propre, il rejoint la prochaine édition.
Les versions passées gardent les nœuds compactés.

**Mesure** (notée dans l'ADR) : `deck()`, `apply`, `from_snapshot`, `merge`
et taille du snapshot, avant et après compaction, sur 40×20 avec ~10 000
nœuds marqués.

**Tests.**
- Marque de 31 jours : nœud supprimé par Loro après l'ouverture ; marque de
  29 jours : gardé. Deux marques, une ancienne une récente : supprimé.
- Nœud vide ancien supprimé, récent gardé.
- Compaction non annulable (`can_undo` faux après l'ouverture), deck lu
  identique avant et après.
- Convergence : la copie compactée fusionnée avec une copie non compactée
  donne le même deck des deux côtés.
- Marque non entière refusée à l'import.
- ADR-0006 : marque datée, compaction, risque des N jours, mesures ; Limites
  « les nœuds marqués ne sont jamais libérés » et « nœud vide » mises à jour.

## Hors champ

- Snapshot *shallow* (taille du fichier) : tâche à part si la taille gêne.
- Compaction en cours de session ou à l'enregistrement.
