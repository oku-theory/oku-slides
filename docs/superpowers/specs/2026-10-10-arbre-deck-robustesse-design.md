# Arbre du deck : défaire un ajout, compaction, profondeur — design

- **Date :** 2026-10-10, révisé le 2026-10-11 après deux relectures du plan et les sondes Loro
- **Tâches :** t-776 (haute), t-770, t-809 (haute, sécurité)
- **Contexte :** [ADR-0006](../../adr/0006-deck-en-un-seul-arbre-suppression-douce.md)
- **Ordre des lots :** t-776, puis t-770, puis t-809. La sonde de t-809 ouvre son lot
  ; un arrêt n'y bloque pas les deux premiers.

## Décisions de cadrage (utilisateur)

1. Compaction : à l'ouverture, seulement les nœuds dont la marque a plus de
   N jours, N = 30 (réglage).
2. Historique Loro gardé entier : pas de snapshot *shallow* produit par l'app.
3. Les nœuds vides (nés d'un `mov` défait) sont compactés par la même règle.
4. t-809, option D : l'état d'un snapshot reçu n'est jamais lu ; le document
   est rejoué à partir de ses opérations, puis borné. (L'option 1, grande pile
   et plafond d'octets, est écartée : un snapshot *shallow* de Loro tient déjà
   une chaîne en 7,5 o par niveau, un état forgé bien moins.)
5. Profondeur maximale de l'arbre : 1024 (une suppression garde son
   sous-arbre caché, qu'un `mov` emporte : 64 était atteignable par l'usage).
6. Horloges : `limit = min(now, change le plus récent du document) − N jours`.
   Conséquence voulue : une marque n'est compactée qu'après N jours
   d'activité du document. Une date nulle ou négative n'est jamais ancienne.
7. Une marque remise par ⌘Z ou rétablir garde sa date d'origine : limite
   documentée, pas de redatation.

## Lot 1 — t-776 : défaire un ajout ne perd plus le sous-arbre

**Problème.** Loro inverse `create` par `delete` : défaire un ajout emporte
le sous-arbre qu'un pair a déplacé dessous (B crée une slide, A y déplace un
élément par `move_element`, B fait ⌘Z : l'élément est perdu, le ⌘Z de A ne
le rend pas).

**Naître marqué.** Dans une réconciliation annulable (`apply`,
`apply_moving`), quand un nœud doit être créé :

1. ce qui précède est committé en partie (`oku-part:<libellé>`, annulable) ;
2. le nœud est créé avec la marque du pair courant (`Writer::mark`), dans un
   commit d'origine `SILENT` (exclu de l'`UndoManager`, qui n'y pousse aucun
   item) ;
3. la réconciliation reprend : méta, `@slot` et **retrait de la marque**
   tombent dans les commits annulables de l'étape.

Défaire l'ajout remet la marque (suppression douce) : le nœud est caché, son
sous-arbre aussi, rien n'est perdu. Le ⌘Z de A ramène son élément ; le
rétablir de B rend la slide et ce qui est dessous. `new` et `apply_silent`
créent directement ; l'essai de `restore` suit le même chemin que l'`apply`
qui le suit.

**Effet de bord.** Plus aucun chemin du code ne fait de `tree.delete` hors
compaction. Les commits de naissance sont des parties : ni cran d'historique,
ni compte dans le rapport de fusion (qui ne compte plus aucune partie).

**Coût.** Un change Loro de plus par nœud créé, un item d'annulation de plus
par partie non vide. Mesure : collage de 50 et 500 éléments, à plat et
imbriqués. Une étape de plus de 64 items relève `ITEMS_PER_STEP` ; un ⌘Z de
500 éléments au-delà de 100 ms arrête le lot pour décision (regrouper les
créations d'un `arrange` en un commit silencieux est la piste prévue).

**Tests.** Régression `issue_0776_undo_add_keeps_moved_subtree.rs` (précondition
vérifiée ; nœud de la slide non supprimé par Loro après le ⌘Z de B ; ⌘Z de A ;
rétablir de B). Tests existants inversés (`soft_delete_tests`). Un collage se
défait et se rétablit d'un bloc, nœuds marqués et non supprimés. `apply_silent`
n'ajoute qu'un change ; `apply` en ajoute un de plus par nœud créé.

## Lot 2 — t-770 : compaction des nœuds morts

**Marque datée.** La valeur de `@deleted/<pair>` passe de `true` à l'heure
Unix de la pose (`i64`, horloge de l'auteur ; Loro n'expose que le dernier
auteur d'une clé, pas son op). `REPLICA_FORMAT` 3 → 4, `SOURCES_VERSION`
9 → 10, sans compatibilité. `check_trees` refuse une marque qui n'est pas un
entier.

**Nœud vide daté** par `get_last_move_id` puis `get_change(id).timestamp`.

**Règle** (`compact_doc`, tout entière dans `replica/compact.rs`) : avec
`limit = min(now, horodatage le plus récent des changes) − N jours`, est
supprimé par `tree.delete`, avec son sous-arbre déjà caché :
- tout nœud qui porte au moins une marque datée de `]0, limit]` ;
- tout nœud vide daté de `]0, limit]`.
Un nœud emporté par un ancêtre compacté n'est pas compté. Ordre de
suppression déterministe.

**Où.** Dans l'ouverture (`from_snapshot`), sous la garde, après la première
lecture, avant `Undo::new`. Commit silencieux, message `oku-compact` : ni
cran d'historique, ni compte dans le rapport de fusion. Loro date un change
au moins comme ses prédécesseurs : le commit de compaction porte la date du
change le plus récent. Aucune méthode publique ne compacte en session.

**Risques assumés :** une autre copie ouverte plus de N jours d'activité,
qui annule alors sa marque ou le `mov` d'un élément sous ce nœud, le perd à
la fusion ; une horloge d'auteur en retard raccourcit la fenêtre ; une marque
remise par ⌘Z garde sa date ; un fichier forgé peut faire compacter un nœud
qu'il pouvait déjà cacher ; un message `oku-compact` forgé cache un change de
l'historique et du rapport (le contenu reste dans les versions suivantes).

**Réglage.** `compact_after_days = 30` dans `[sources]` (0 : jamais).
`ReplicaLimits { max_bytes, compact_after_secs }` remplace l'argument
d'octets de `from_snapshot` et de `Resume::read_doc` (`From<usize>` : sans
compaction, pour les tests et outils) ; `SourceLimits` gagne
`compact_after_secs` et `replica()`.

**Mesure** (ADR-0007) : `deck()`, `apply`, `from_snapshot`, `merge`, taille,
avant et après compaction, 40×20 avec ~10 000 nœuds marqués.

**Tests.** Datés par la valeur des marques et par
`set_next_commit_timestamp` (les dates de changes ne reculent pas) : marque
ancienne compactée, récente gardée, à la limite exacte compactée et une
seconde après gardée ; une ancienne parmi des récentes ; horloge locale en
avance bornée par le change le plus récent ; nœud vide compacté après
N jours d'activité ; prédicat d'âge sur 0 et négatif ; parent et enfant morts
comptés une fois ; aucun commit sans rien à compacter ; ouverture non
annulable, sans cran ; convergence avec une copie non compactée ; rapport de
fusion vide ; marque non entière refusée ; `from_zip` et reprise.

**ADR-0007** « Compaction des nœuds morts » (remplace en partie ADR-0006 :
« jamais `tree.delete` »).

## Lot 3 — t-809 : chaîne d'arbre forgée très profonde

**Problème.** Loro 1.16.2 récurse sur la profondeur d'un arbre : au premier
accès de l'état décodé d'un snapshot, puis, même supprimé, dans
`is_node_deleted`, l'export et `fork`. Un débordement de pile avorte le
processus ; `catch_unwind` ne le rattrape pas. L'état d'un snapshot n'est pas
vérifié par Loro contre son historique : il peut être forgé, dense, cyclique.

**Sonde du 2026-10-11** (release, pile de 256 Kio, 100 000 niveaux) :
- snapshot importé : 1er accès, `is_node_deleted`, export, `fork`, drop
  débordent ;
- **document rejoué** (import des updates exportées d'un document dont l'état
  n'est jamais lu) : 1er accès, remontée `parent()`, drop passent ; seul
  l'export d'un arbre profond déborde ;
- version passée profonde relue par rejeu (`updates_till`) : passe, 2 ms ;
- 50 000 cartes imbriquées rejouées : import, export, `fork`, drop passent
  (`get_deep_value` déborde ; jamais appelé hors tests) ; au-delà de 65 535,
  Loro panique (rattrapé) ;
- snapshot *shallow* : `export(all_updates)` impossible → refusé par
  `is_shallow()` ;
- coût du rejeu : chaîne de 100 000 ops 60 ms (direct 4 ms) ; 1 M ops 0,8 s
  (5 ms) ; arbre large d'un million de nœuds ~10 s (relecture n°3).

**Ouverture par rejeu.** Sous `guarded` (`catch_unwind`, inchangé) :
1. le snapshot est importé dans un document A dont l'état n'est **jamais lu** ;
   refusé s'il est *shallow* ; ses opérations sont exportées
   (`all_updates`), puis A est libéré (son drop ne récurse pas : état non
   décodé ; après une panique, A est oublié) ;
2. les opérations sont importées dans le document neuf du replica ;
3. **contrôles d'entrée, dans cet ordre, même sans `deck.slides`** : aucun
   arbre autre que `deck.slides` dans les conteneurs touchés par l'historique
   (tout arbre s'il manque) ; profondeur de **tous** les nœuds, supprimés
   compris (remontée de `parent()`, jamais `is_node_deleted`), au plus
   `MAX_TREE_DEPTH = 1024`. La même passe, linéaire, rend l'ensemble des nœuds
   supprimés par Loro : `check_trees` et la compaction le lisent au lieu
   d'appeler `is_node_deleted` en boucle (O(nœuds × profondeur) : ~29 s pour un
   balai d'un million de feuilles). Puis les contrôles existants, la première
   lecture, la compaction.

Un document accepté n'a qu'un arbre, de 1024 niveaux au plus (export,
`fork` et drop tiennent dans ~1 Mio de pile, sous les 2 Mio des fils de
l'app ; fusion, restauration et historique à la borne testés sur 2 Mio) :
rien ne déborde ensuite. Refusé : `Corrupted`.

**Fusion.** L'essai (copie de `self` + updates de l'autre) passe les mêmes
contrôles (`check_trees` porte les deux nouveaux).

**Version passée.** `deck_at` ne fait plus de `checkout` : il exporte les
opérations jusqu'à la version (`updates_till`), les rejoue dans un document
neuf, le contrôle comme à l'ouverture, le lit et le libère. Une version
refusée rend `Corrupted` ; les autres restent lisibles.

**Résidus documentés.** Valeurs plates imbriquées (t-223) ; ne jamais appeler
`get_deep_value` sur un document reçu ; coût du rejeu à l'ouverture (un
fichier hostile de ~18 Mio peut geler l'ouverture ~10 s, sans plantage).

**Tests.** Sonde committée (`tests/loro_depth_probe.rs`). Régression
`issue_0809_deep_tree_chain.rs` sur pile de 256 Kio : chaîne vivante, chaîne à
tête supprimée, snapshot *shallow*, arbre hors `deck.slides` (avec ou sans
`deck.slides`) refusés ; chaîne à la borne acceptée puis enregistrée, fusionnée,
restaurée et libérée sur une pile de 2 Mio ; fichier à historique profond et
état plat ouvert sur 2 Mio. Fusion
d'un pair forgé (chaîne profonde, arbre étranger) refusée, `self` intact.
Version passée profonde refusée, première et dernière lues à l'identique.

## Hors champ

- Snapshot *shallow* produit par l'app ; compaction en session.
- Valeurs plates imbriquées (t-223) ; débordement d'entier sur un compteur de
  change dans `history::build` (préexistant) ; épingler `loro = "=1.16.2"` dans
  le workspace : tâches à part.
