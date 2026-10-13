# ADR-0007 : Compaction des nœuds morts de l'arbre du deck

- **Statut :** accepté
- **Date :** 2026-10-11
- **Tâches :** t-770
- **Remplace en partie :**
  [ADR-0006](0006-deck-en-un-seul-arbre-suppression-douce.md) (« jamais
  `tree.delete` »)

## Contexte

Les nœuds marqués `@deleted/<pair>` s'accumulent (ADR-0006) : la lecture et
chaque `apply` coûtent O(vivants + marqués), et le snapshot ne cesse de croître.

## Décision

- **Marque datée.** La valeur d'une marque est l'heure Unix de sa pose (`i64`,
  horloge de l'auteur) : Loro n'expose que le dernier auteur d'une clé, pas la
  date de chaque marque.
- **Limite.** `limit = min(now, change le plus récent) − compact_after_days`.
  Une marque n'est donc compactée qu'après 30 jours d'**activité** du document,
  pas de simple horloge : une horloge locale très en avance ne compacte rien
  de plus.
- **Compacté :** un nœud qui porte une marque datée de `]0, limit]` (une seule
  suffit : seul son pair pourrait la retirer, et sa fenêtre est passée), ou un
  nœud vide dont le dernier `mov` est daté de `]0, limit]`. `tree.delete`
  l'emporte avec son sous-arbre, déjà caché.
- **Quand.** À l'ouverture d'un fichier, sous la garde (les changes de Loro ne
  sont pas fiables), avant la pile d'annulation : la compaction n'est pas
  annulable.
- **Commit silencieux** de message `oku-compact`, daté comme le change le plus
  récent (`set_next_commit_timestamp`), pour qu'ouvrir un fichier ne compte pas
  comme une activité. Ni cran d'historique, ni compte dans le rapport de fusion.
  L'historique reste entier.

## Conséquences

- Le nombre de nœuds suit les nœuds vivants et ceux de moins de 30 jours
  d'activité.
- Mesures (release, 40 slides × 20 éléments, 10 000 nœuds marqués, avant puis
  après compaction) : ouverture (`from_snapshot`) 104 ms puis 25 ms ; `apply`
  37 ms puis 11 ms ; `merge` 151 ms puis 83 ms ; `deck()` inchangé (11 ms).
  La taille du snapshot ne baisse pas (2,24 Mo puis 2,28 Mo) : l'historique
  Loro est conservé, seul le coût de parcours de l'arbre diminue.
- Une copie compactée et une copie qui ne l'est pas convergent à la fusion.

## Limites

- Une autre copie, ouverte plus longtemps, qui annule alors sa marque ou le
  `mov` d'un élément sous ce nœud, perd ce nœud.
- Une marque remise par ⌘Z ou rétablir garde sa date d'origine.
- Une horloge d'auteur en retard raccourcit la fenêtre.
- Un fichier forgé peut faire compacter un nœud qu'il pouvait déjà cacher, ou
  porter un message `oku-compact` qui cache un change de l'historique et du
  rapport (son contenu reste visible dans les versions suivantes).
- Le snapshot ne rétrécit pas : l'historique est gardé.
