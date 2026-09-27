# Sources du deck en CRDT Loro : historique et fusion — design

Date : 2026-09-27 · Tâche : t-188

## Objectif

Le blob des sources rangé dans le PDF devient un document **Loro** (CRDT) :

1. **Historique dans le fichier** : toute version passée d'un deck se revoit
   et se restaure à partir du seul PDF, avec un curseur temporel.
2. **Fusion** : deux copies d'un même deck modifiées séparément (mail,
   dossier partagé) se fusionnent sans dialogue de conflit.
3. **Temps réel plus tard** : le format ne l'empêche pas (mises à jour Loro
   échangées entre pairs). Le temps réel lui-même est hors périmètre.

L'historique complet voyage **toujours** avec le PDF (pas d'export
« propre », pas d'aplatissement).

Hors périmètre : temps réel, diff visuel de slides, aplatissement de
l'historique, lecture des PDF au format v1.

## Décisions

- **Loro plutôt que Git ou un journal d'ops maison.**
  - `git bundle` + merge à trois voies : fusion structurelle et résolution
    de conflits à écrire nous-mêmes, aucune issue vers le temps réel.
  - Journal d'`Op` rejoué : les ops ne commutent pas (déplacer ‖ supprimer,
    éditer le même texte) ; ce serait réinventer un CRDT.
  - Loro (Rust, 1.x) : historique, fusion, `checkout`, annulation par pair
    et protocole de synchronisation dans un seul format.
- **Réconciliation plutôt que traduction des ops.** Les `Op` s'appliquent
  sur `Deck` comme aujourd'hui ; le `LoroDoc` est ensuite réconcilié avec le
  nouveau `Deck` par diff structurel. Les variantes d'`Op` ne sont pas
  traduites une à une.
- **Vide = absent** pour toute prose (voir « Schéma »).
- **Annulation par `UndoManager` de Loro**, non persistée : la pile
  d'annulation repart de zéro au relancement (recul assumé sur t-145).
- **PDF v1 ignorés** : un blob v1 est traité comme absent. Pas de
  conversion.

## Spike (jetable, hors dépôt)

Loro 1.16.2, `fixtures/roadmap.json` (8 slides) et un deck synthétique de
40 slides :

| Mesure | Résultat |
|---|---|
| Aller-retour `Deck → Loro → Deck` | identique |
| Réconcilier le même deck | 0 op, ~1 ms (40 slides) |
| Fusion de deux branches (titre édité des deux côtés, déplacement, suppression, ajout) | convergence, `Brouillon : Contexte (révisé)`, `validate` sans problème |
| Retour à la version d'origine depuis le snapshot | identique |
| `UndoManager` après fusion | défait le changement local, garde le distant |
| 539 commits d'un caractère | +~330 o, ~40 µs par commit |
| Snapshot avec historique | 5,5 Ko (roadmap), contre 1 Ko pour le zip v1 |

Limites observées, reproduites sur Loro brut :

1. Runs de texte riche réconciliés par index : gras ‖ frappe donne
   `Le  [ajout B]**marché** a doublé…`. **Parade vérifiée** : un
   `LoroText` avec marques par bloc de paragraphes.
2. Création concurrente d'un même champ texte absent : un seul texte
   survit. Un texte préexistant, même vide, fusionne (`AB`). **Parade** :
   champs de prose toujours créés.
3. Déplacer ‖ supprimer : l'élément réapparaît.
4. Éditer ‖ supprimer : la suppression gagne.
5. Deux ajouts avec le même `id` : deux éléments ; `validate` le signale.

## Schéma du `LoroDoc`

Racine `deck` (`LoroMap`), plus deux maps racines :

- `meta` : `deck_uid` (aléatoire, posé à la création, jamais modifié).
- `peers` : `{ <PeerID>: "<nom d'auteur>" }`, une entrée par pair, posée à
  son premier commit (nom tiré des réglages, sinon de l'utilisateur
  système).

Chaque champ du modèle relève d'une catégorie :

| Catégorie | Conteneur | Champs |
|---|---|---|
| Collection ordonnée à `id` | `LoroMovableList` de `LoroMap` | `slides`, `blocks`, `elements`, `children` |
| Prose simple | `LoroText` | slide : `name`, `description`, `title`, `subtitle`, `kicker`, `text`, `notes` ; `params` : `title`, `subtitle`, `author`, `organization`, `date` ; carte : `title`, `kicker` |
| Prose riche | un `LoroText` avec marques | `text: Vec<Paragraph>` des éléments `text` et `card` |
| Scalaire LWW | valeur | `id`, `kind`, `layout`, `theme`, `templates`, `version`, `params.lang`, `asset`, `fit`, `shape`, chaque champ de `frame`, `cell`, `style`, `grid`, `layout` de groupe ; chaque entrée de `vars` ; chaque feuille de `tokens` et de `params.extra` |
| Atome JSON LWW | une valeur (JSON entier) | `chart.option` |

- **Prose riche.** Paragraphes joints par `\n`. Marques de run : `bold`,
  `italic`, `color`, `size`, `highlight` (`expand: After`), `link`
  (`expand: None`). Le `list` d'un paragraphe est une marque sur son `\n`.
  La lecture reconstruit les `Run` depuis `to_delta()`.
- **Prose et collections toujours présentes.** À la création du deck, d'une
  slide ou d'un élément, tous ses `LoroText` de prose et toutes ses
  collections à `id` (`slides`, `blocks`, `elements`, `children`) sont
  créés, vides au besoin. Une valeur absente du JSON vide le conteneur,
  sans jamais retirer la clé. Sinon, deux pairs qui créent la même clé
  en concurrence (premier élément d'une slide vide, premier titre) font
  chacun leur conteneur, et un seul survit.
- **Séparateur de paragraphes** : `U+2029` (et non `\n`, qu'un run peut
  contenir). Chaque paragraphe se termine par ce séparateur, le dernier
  compris. Les runs adjacents de même mise en forme sont fusionnés à la
  lecture (forme canonique).
- **Vide = absent, invariant du modèle.** `opt_string` (`ops.rs`) convertit
  une chaîne vide ou faite d'espaces en `None` (même règle que l'UI) (couvre `SetSlideField` et `SetParam`) ; la lecture Loro
  ramène un texte vide à `None`. Les normalisations de l'UI
  (`deck.ts:76`, `a2ui/panelActions.ts:25`, `App.tsx:494`) disparaissent.
  Audit : aucun modèle ni test ne s'appuie sur `Some("")` ; seul effet
  perdu, forcer un titre vide sur une slide de titre.
- **Graphique en atome** : `consistent_chart` refuse des options
  incohérentes ; une fusion champ par champ pourrait en produire. Règle
  générale : un sous-arbre qui porte un invariant global est un atome.
- **`frame` champ par champ** : déplacer ‖ redimensionner garde les deux.

## Code

- **`oku-slides-core::replica`** (`replica.rs`, tests `replica_tests.rs`) :
  `DeckReplica`, enveloppe d'un `LoroDoc`.
  - `DeckReplica::new(deck)` (nouveau `deck_uid`), `from_snapshot(bytes)`,
    `snapshot()`.
  - `reconcile(&Deck)` puis `commit(label)`.
  - `deck() -> (Deck, Vec<Issue>)` : **lecture totale**.
  - `merge(&DeckReplica)` : vérifie `deck_uid`, importe.
  - Historique : `versions()` (ordre topologique, repères de session et de
    fusion), `deck_at(index)`, `restore(index)`.
- **`SourceBundle`** gagne `replica: DeckReplica`. `deck` reste une vue
  matérialisée utilisée par le rendu, `validate`, `locate` et les ops.
- **`oku-slides-core` supprime** `History` (`history.rs`) et les inverses :
  `apply_all` applique atomiquement et ne renvoie rien.
- Dépendance nouvelle : `loro` (espace de travail). Dev : `proptest`.

## Flux d'écriture

`Session::apply` :

1. `canonicalize_charts(ops)`.
2. `deck' = apply_all(clone(deck), ops)` ; erreur → rien n'a bougé.
3. Rendu de `deck'` ; erreur → rien n'a bougé.
4. `replica.reconcile(&deck')`, `commit(label)`, `deck = deck'`.

Le rendu passe **avant** le commit : l'historique Loro est définitif, un
rendu raté ne doit pas y laisser de trace.

- **Pair** : `PeerID` aléatoire à chaque chargement (ouverture, reprise,
  nouvel onglet), jamais persisté. Deux écrivains d'un même pair
  corrompraient le document.
- **Horodatage** : `set_record_timestamp(true)`.
- **Annuler / rétablir** : un `UndoManager` par session, `merge_interval
  = 0` (un apply = une étape), `set_max_undo_steps(settings.history.limit)`.
  Le libellé de transaction va dans les métadonnées (`on_push`) et remonte
  par `top_undo_meta` / `top_redo_meta` vers `undo_label` / `redo_label`.
  Après un undo ou un redo : relecture du deck, rendu ; un rendu raté garde
  l'état et remonte l'erreur.
- **Mode modèles** : même `SourceBundle`, replica jamais enregistré en PDF.

## Reprise

`tabs/t-<n>/deck/` :

```
deck.loro       snapshot, écriture atomique
document.json   { path, dirty, saved_hash }
assets/<hex>    inchangé
fonts/<nom>     inchangé
```

`history` disparaît de `document.json` et de `resume.rs`. La spec de
reprise (`2026-09-26-resume-state-design.md`) est mise à jour : la pile
d'annulation ne survit plus au relancement. `TabInfo` gagne `deck_uid`
(dans `workspace.json`), pour détecter un deck déjà ouvert sans réveiller
d'onglet (ADR-0001).

## Format dans le PDF

Enveloppe inchangée (`/PieceInfo/OkuSlides/Private`), `/Version 2`.
`extract_sources` : version < 2 → `Ok(None)` (ouverture : `NotOkuSlides`) ;
version > 2 → `UnsupportedVersion`.

Zip des sources v2 :

```
manifest.json    { format: "oku-slides-sources", version: 2 }
deck.loro        snapshot complet (ExportMode::Snapshot)
assets/<hex>     tels quels
fonts/<nom>      tels quels
```

- Plus de `deck.json` : le deck se reconstruit depuis `deck.loro`.
- **Assets et polices en ajout seul** : jamais retirés, une ancienne
  version doit se rendre. Fusion = union.
- **Blob non fiable** : import sous `catch_unwind` et taille maximale
  (réglage) ; panique ou octets refusés → `Corrupted`.
- **Lecture totale, trois niveaux** : un élément illisible est écarté
  (`issue-unreadable-element`), puis une slide (`issue-unreadable-slide`),
  puis un champ de haut niveau du deck (`theme`, `params`, `vars`,
  `templates`), remis à sa valeur par défaut (`issue-unreadable-field`).

## Fusion

- **Parenté** : deux replicas ne fusionnent que si leur `deck_uid` est égal ;
  sinon `Unrelated` (« ces deux présentations n'ont pas d'origine
  commune »).
- **Étapes** : lire le blob entrant, vérifier `deck_uid`, `import`, union
  des assets et polices, relire (lecture totale), rendre. Avis « Fusionné
  avec *x.pdf* : N changements de Julie ». Le document devient `dirty`.
- **Déclencheurs** :
  1. *Fichier → Fusionner avec un PDF…* sur l'onglet actif.
  2. **À l'enregistrement** : si le fichier sur disque a changé depuis
     l'ouverture (`saved_hash`) et porte le même `deck_uid`, fusion puis
     écriture. S'il porte un autre deck (ou aucun) : `DiskReplaced`, l'UI
     propose d'écraser ou d'enregistrer sous un autre nom.
  3. **À l'ouverture** : un PDF dont le `deck_uid` est déjà ouvert dans un
     onglet (lu dans `TabInfo`) propose « Fusionner dans cet onglet » ou
     « Ouvrir à part ».
- Pas de dialogue de conflit : chaque conflit a une issue déterministe ;
  on corrige après coup, ou on restaure la version d'avant la fusion.

## Mode historique

`Mode::History { stash, preview, order }`, sur le patron de
`Mode::Templates` : le deck courant attend de côté, la version choisie
s'affiche en lecture seule.

- **Ordre** : tri topologique des changements (Lamport, puis pair). Tout
  préfixe est une coupe causale valide : chaque cran du curseur est un
  état cohérent. L'heure est affichée, jamais utilisée pour trier
  (horloges divergentes).
- **Curseur** : un cran par commit ; repères au début de chaque session
  (un pair = une session d'édition) et à chaque fusion. ←/→ : un commit ;
  ⇧←/⇧→ : une session. Bulle : auteur, heure, libellé.
- **Bandeau** : « Version du … · auteur · libellé », **Restaurer**,
  **Revenir**.
- **Pile** : slides ajoutées, supprimées, modifiées par rapport au deck
  courant (`doc.diff`, conteneur modifié → sa slide).
- **Rendu** : `preview` est un fork fait à l'entrée ; `checkout` d'un cran
  à l'autre. Pendant le glissement, seul le dernier cran demandé est rendu
  (patron de `ui/src/latest.ts`) ; le cache SVG par empreinte évite de
  redessiner les pages inchangées.
- **Restaurer** : `revert_to(frontiers)`, un **nouveau** commit « Restaurer
  la version du … », annulable.
- **Requêtes** : `HistoryOpen`, `HistorySeek { index }`, `HistoryRestore`,
  `HistoryClose`.

## Erreurs

| Où | Type | `kind` |
|---|---|---|
| `oku-slides-core::replica` | `ReplicaError` | `Corrupted`, `TooLarge` ; `Unrelated` avec la fusion |
| `oku-slides-core::sources` | `SourcesError` | existants ; `MissingDeck` couvre l'absence de `deck.loro` |
| `oku-slides-session` | erreur de session | `MergeUnrelated`, `HistoryNotOpen`, `DiskReplaced` |

Ne sont pas des erreurs : un sous-arbre illisible après fusion (écarté,
signalé par un `Issue`) ; un rendu raté pendant un apply (rien n'est
validé, l'erreur remonte comme aujourd'hui). Messages en Fluent : anglais
d'abord, puis les langues existantes.

## Tests (TDD)

Les scénarios du spike sont les premiers tests rouges.

- **`replica_tests.rs`** : aller-retour ; réconciliation sans changement =
  0 op ; vide = absent ; prose toujours présente (création concurrente sans
  perte) ; prose riche (gras ‖ frappe, lien qui ne s'étend pas, `list` sur
  le `\n`) ; graphique atome ; lecture totale (`kind` inconnu, graphique
  incohérent → `Issue`, reste du deck lu) ; `deck_uid` différent →
  `Unrelated`.
- **`tests/convergence.rs`** (proptest) : N pairs, ops aléatoires, fusions
  dans des ordres aléatoires → tous convergent, la lecture ne panique
  jamais.
- **`tests/untrusted.rs`** : snapshot tronqué, aléatoire, énorme →
  `Corrupted` / `TooLarge`, jamais de panique.
- **`oku-slides-pdf`** : aller-retour v2 ; blob v1 → `Ok(None)` ; v3 →
  `UnsupportedVersion`.
- **Session** : rendu raté → replica intact ; libellés d'annulation ;
  reprise sans pile d'annulation ; fusion à l'ouverture et à
  l'enregistrement (`DiskReplaced`) ; mode historique (ouvrir, `seek`,
  restaurer = nouveau commit, fermer).
- **UI (Bun)** : ordre et repères du curseur ; rendu du dernier cran
  pendant le glissement.
- **Mesure au plan** : coût d'un rendu en mode historique ; au-delà de
  ~100 ms, vignette basse résolution pendant le glissement.

## Limites connues

- Déplacer ‖ supprimer : l'élément réapparaît.
- Éditer ‖ supprimer : l'édition est perdue.
- Deux ajouts concurrents au même `id` : doublon, signalé par `validate`.
  Les ids de l'UI sont aléatoires (`deck.ts:newId`) ; à vérifier pour les
  ids produits par l'agent vocal.
- Déplacer un élément d'un groupe ou d'une slide à l'autre = suppression +
  insertion : doublon possible en concurrence (non testé au spike).
- Texte inséré au bord d'une marque : il en hérite.
- Le PDF ne maigrit jamais (historique et assets en ajout seul).
- La pile d'annulation est perdue au relancement (recul sur t-145).
- Les PDF v1 ne s'ouvrent plus.

Les trois derniers points sont des reculs assumés, à consigner dans un ADR.
