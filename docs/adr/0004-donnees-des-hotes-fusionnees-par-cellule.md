# ADR-0004 : Les données des hôtes fusionnées par cellule

- **Statut :** accepté
- **Date :** 2026-10-08
- **Tâches :** t-589
- **Remplace en partie :** [ADR-0002](0002-sources-du-deck-en-crdt-loro.md)
  (l'option d'hôte en atome JSON)
- **Spec :** [fusion-cellules-design.md](../superpowers/specs/2026-10-08-fusion-cellules-design.md)

## Contexte

L'option d'un paramètre `host(chart)` ou `host(table)` est un `Atom` Loro,
comme le décidait l'ADR-0002 : la dernière écriture remplace toute l'option.
Deux personnes qui modifient en même temps des cellules différentes d'un même
tableau ou graphique perdent donc l'une des deux modifications, sans
avertissement. Il en va de même pour deux insertions de ligne, deux insertions
de colonne ou une suppression de ligne face à une édition de cellule.

## Décision

- **Identifiants stables de lignes et de colonnes**, dans deux tableaux
  parallèles du `dataset` : `ids` (une entrée par ligne de `source`, en-tête
  comprise) et `columns` (une entrée par colonne). Le cœur les crée : 8
  caractères hexadécimaux tirés d'un `uuid` v4, tirés à nouveau en cas de
  collision. Format admis : `[A-Za-z0-9_-]{1,64}`, uniques par liste.
- **`dataset` en conteneurs.** Dans le replica, le `dataset` devient une
  `LoroMap` `{ columns, rows }` : `columns` est une liste ordonnée
  d'identifiants, `rows` une liste ordonnée de lignes `{ id, cells }`, et
  `cells` une map de scalaires par identifiant de colonne. Lignes, colonnes
  et cellules se rapprochent par identifiant, donc les éditions concurrentes
  de cellules, de lignes et de colonnes différentes se fusionnent.
- **Réparation à la relecture.** Après une fusion, l'état relu est remis en
  règle : colonne en double, et ligne en double aux mêmes cellules, écartées
  (aucun doublon après une double annulation) ; cellule absente lue comme
  `null` ; autre identifiant en double ou invalide
  remplacé par un identifiant déterministe tiré de la position, `align`
  tronqué à la largeur.
- **Les autres clés de l'option** (`series`, couleurs, drapeaux) restent en
  dernière écriture gagnante, clé par clé, chacune en atome.

**Invariant global.** Le cœur garde l'alignement de `ids` et `columns` avec
`source` à chaque écriture. La relecture répare un état fusionné, et la
validation (100 lignes × 20 colonnes) s'applique aux ops. Un tableau qui
dépasse ces bornes après fusion est rendu en substitut jusqu'à ce qu'on
supprime des lignes ou des colonnes.

## Compatibilité

Aucune. `DECK_VERSION` passe de 4 à 5 et `SOURCES_VERSION` de 6 à 7. Une
ancienne version de l'app refuse le nouveau format avec son erreur de version
habituelle, et la nouvelle ne lit aucun ancien format : pas de chemin de
lecture de la forme `Atom`. Les fixtures sont migrées.

## Conséquences

- Les ops de données ne rendent plus d'inverse : l'historique passe par le
  replica.
- Le `dataset` d'une forme inattendue est écrit en `Opaque`, sans perte, et ne
  fusionne pas finement.

## Limites

- Une cellule est un scalaire en dernière écriture gagnante : pas de fusion
  caractère par caractère dans une cellule.
- Un collage garde les identifiants par position (lignes et colonnes
  `0..min(ancien, nouveau)`) et en tire de neufs pour l'excédent.
- Deux collages concurrents (`Replace`) fusionnent cellule par cellule
  (dernière écriture gagnante par cellule) : les identifiants étant gardés par
  position, le résultat peut mêler les deux collages.
