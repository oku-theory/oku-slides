# ADR-0002 : Les sources du deck en CRDT Loro, historique et fusion

- **Statut :** accepté
- **Date :** 2026-09-27
- **Tâches :** t-198 … t-207 (plan 1 : fondation) ; t-243 … t-248 (plan 3 :
  mode historique)
- **Spec :** [sources-loro-design.md](../superpowers/specs/2026-09-27-sources-loro-design.md)

## Contexte

Le blob des sources rangé dans le PDF (`/PieceInfo/OkuSlides/Private`)
était un simple zip du `Deck` en JSON : aucune version passée ne s'y
retrouvait, et deux copies d'un même deck modifiées séparément (envoyées
par mail, partagées dans un dossier) ne pouvaient se recombiner autrement
qu'en écrasant l'une des deux. Trois objectifs se posaient à la fois :

1. **Historique dans le fichier** : revoir et restaurer toute version
   passée d'un deck à partir du seul PDF, avec un curseur temporel.
2. **Fusion** : deux copies modifiées séparément se fusionnent sans
   dialogue de conflit.
3. **Temps réel plus tard** : le format ne doit pas l'empêcher (mises à
   jour échangées entre pairs), même si le temps réel lui-même reste hors
   périmètre de ce plan.

## Décision

**Le deck est un document Loro (CRDT), pas un JSON figé.**

- `oku-slides-core::replica::DeckReplica` enveloppe un `LoroDoc`. Il fait
  foi ; le `Deck` (le modèle applicatif existant, inchangé) n'en est qu'une
  **vue** matérialisée, tenue à jour par `SourceBundle`, et toujours égale
  à la relecture du replica. Cette relecture est canonique (runs vides
  retirés, runs voisins de même format fusionnés, prose blanche absente) :
  `canonicalize` met chaque deck produit par `apply_all` ou reçu par
  `SourceBundle::new` sous cette forme.
- **Réconciliation, pas traduction des `Op`.** Les opérations d'édition
  s'appliquent sur `Deck` comme avant (`apply_all`). Le `LoroDoc` est
  ensuite réconcilié avec le `Deck` obtenu par diff structurel
  (`DeckReplica::reconcile`). Les variantes d'`Op` (déplacer, insérer,
  changer un champ…) ne sont pas traduites une à une vers des mutations
  Loro : plus petite surface à maintenir, un seul chemin de vérité.
- **Quatre catégories de champs**, posées une fois pour toutes dans le
  schéma du replica (`replica::schema`) :
  - collections ordonnées à `id` (`slides`, `blocks`, `elements`,
    `children`) → `LoroMovableList` de `LoroMap` ;
  - prose simple → `LoroText` ;
  - prose riche (`text: Vec<Paragraph>`) → `LoroText` avec marques par
    bloc de paragraphes (`bold`, `italic`, `color`, `size`, `highlight`,
    `link`, `list`) ;
  - le reste → scalaire ou atome JSON en dernier écrivain gagne (LWW), sauf
    `chart.option` : l'[ADR-0004](0004-donnees-des-hotes-fusionnees-par-cellule.md)
    remplace en partie cette règle, en fusionnant les données des hôtes par
    cellule.
- **Vide = absent**, invariant du modèle : toute prose vide (chaîne vide ou
  faite d'espaces) se lit comme `None`, s'écrit comme un `LoroText` vide,
  jamais comme une clé retirée. Prose et collections à `id` sont **toujours
  présentes** dès la création de leur parent (deck, slide, élément), même
  vides : sans cela, deux pairs qui créent la même clé en concurrence
  (premier élément d'une slide vide, premier titre) créeraient chacun leur
  conteneur, et un seul survivrait — silencieusement.
- **Annulation par l'`UndoManager` de Loro**, un par session, jamais
  persisté : la pile repart de zéro à chaque ouverture (recul assumé, voir
  Conséquences). Une étape d'annulation (une application, ou un tour de
  l'agent) regroupe plusieurs items Loro dans une pile à nous, avec son
  libellé : Loro 1.16.2 défait mal un item qui porte plusieurs `mov`, d'où
  un commit par `mov` (t-263).
- **Lecture totale.** `DeckReplica::deck()` ne renvoie jamais d'erreur : un
  sous-arbre illisible (élément, slide, champ de haut niveau) est écarté et
  signalé par un `Issue`, remis à sa valeur par défaut si nécessaire. Un
  deck partiellement corrompu ou fusionné avec un pair malveillant reste
  ouvrable.
- **PDF v1 ignorés.** Une enveloppe `/Version 1` (`deck.json`) est traitée
  comme absente par `extract_sources` (`Ok(None)`), jamais convertie. Le
  format du zip passe à la version 2 (`manifest.json`, `deck.loro` en
  remplacement de `deck.json`, `assets/`, `fonts/`).
- **Mode historique livré (plan 3).** L'historique complet du replica se
  parcourt et se restaure directement, sans conversion : chaque cran du
  curseur est un préfixe causal (`vv_to_frontiers` du vecteur de versions,
  pas seulement l'histoire causale du dernier changement, sinon une branche
  concurrente plus ancienne dans l'ordre disparaîtrait d'un cran à l'autre) ;
  restaurer une version fait une **seule** nouvelle étape annulable (un
  cran), en réconciliant le deck de cette version comme toute édition (la map
  `peers` n'est pas touchée). Pas de `revert_to` : il perdait des slides
  déplacées dans la `LoroMovableList` (`mov`), ou échouait. Détail dans la
  spec, section « Mode historique ».

## Conséquences

**Positives**

- Historique et fusion dans un seul format, sans dialogue de conflit :
  chaque situation de concurrence a une issue déterministe (voir
  « Limites connues » de la spec), corrigible après coup ou en revenant à
  une version antérieure.
- Le temps réel (mises à jour Loro échangées entre pairs) devient un ajout,
  pas une réécriture du format.
- Un deck partiellement illisible (corruption, fusion hostile) reste
  ouvrable : lecture totale par construction.
- **Un commit applicatif est un *change* Loro**, avec son libellé comme
  message (`set_change_merge_interval(-1)`, `set_next_commit_message`) :
  l'historique garde un grain fin (une étape d'annulation = un *change*),
  au lieu que Loro en regroupe plusieurs dans sa fenêtre de fusion par
  défaut. Un compte de « N modifications » (fusion, curseur du mode
  historique) reste donc significatif.

**Reculs assumés**

- **La pile d'annulation ne survit plus au relancement de l'app** (recul
  sur t-145, qui l'avait fait persister). Elle repart de zéro à chaque
  ouverture d'un onglet, comme si le deck venait d'être ouvert.
- **Les PDF au format v1 ne s'ouvrent plus** : leur blob de sources est
  traité comme absent, sans conversion possible. Un deck v1 sans copie du
  `Deck` ailleurs (par ex. dans l'historique applicatif retiré en t-206)
  perd son édition — le rendu figé dans les pages du PDF reste consultable
  comme n'importe quel PDF.
- **Le PDF ne maigrit jamais** : l'historique Loro et les assets sont en
  ajout seul (jamais retirés, pour qu'une ancienne version se rende
  toujours). Un deck très édité peut voir son fichier grossir sans borne.
- **Confidentialité : chaque PDF partagé porte tout son passé.** Le
  snapshot complet embarqué contient tout contenu ayant existé — slides,
  éléments et textes supprimés, versions antérieures de chaque texte,
  assets retirés — ainsi que l'horodatage de chaque modification
  (`set_record_timestamp`). Les noms d'auteur voyagent eux aussi dans le
  document, depuis le plan 2 : la map racine `peers` associe le `PeerID` de
  chaque pair au nom tiré de `[author] name` ou de l'utilisateur système, à
  sa première édition. N'importe quel destinataire peut relire cet
  historique avec un outil Loro. Accepté par conception : pas
  d'aplatissement (snapshot sans historique) à l'enregistrement, qui
  casserait l'historique et la fusion. Atténuation : l'export sans sources,
  `oku-slides strip <entrée> <sortie>` (`oku_slides_pdf::strip_sources`),
  qui ne garde que les pages rendues ; pour l'instant en ligne de commande
  seulement, pas encore dans l'app.

**Neutres / limites connues** (voir la spec pour le détail complet)

- Déplacer ‖ supprimer un élément en concurrence : l'élément réapparaît.
  Éditer ‖ supprimer : l'édition est perdue. Deux ajouts concurrents au
  même `id` : doublon signalé par `validate`.
- Le décodeur Loro lui-même peut faire déborder la pile sur un snapshot
  forgé, avec une valeur plate imbriquée sur des dizaines de milliers de
  niveaux (quelques Kio compressés) : Loro la décode récursivement au
  premier accès et la libère au drop, pas à l'`import`, hors de notre code,
  hors de portée de `catch_unwind` et du plafond d'octets. Déni de service
  local à l'ouverture d'un PDF forgé. Suivi comme correctif à part.
- La reprise écrit `deck.loro` puis `document.json` : un arrêt brutal entre
  les deux peut laisser un replica neuf à côté d'un `document.json`
  d'avant (`dirty` / `saved_hash` périmés) — l'ancien format n'écrivait
  qu'un seul fichier.

## Alternatives écartées

- **`git bundle` + fusion à trois voies.** La fusion structurelle (quel
  élément a bougé, quel champ a changé) et la résolution de conflits
  seraient à écrire nous-mêmes par-dessus un format texte ; aucune voie
  vers le temps réel (pas de mises à jour incrémentales échangeables entre
  pairs).
- **Journal d'`Op` rejoué entre pairs.** Les `Op` ne commutent pas
  (déplacer un élément pendant qu'un autre pair le supprime, éditer le même
  texte à deux endroits) : les réconcilier correctement reviendrait à
  réinventer un CRDT, avec moins de garanties que Loro n'en offre déjà.
- **Aplatissement de l'historique à l'export.** Rejeté avec l'objectif :
  l'historique complet doit voyager **toujours** avec le PDF, pas
  seulement à la demande.
