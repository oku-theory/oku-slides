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
- `peers` : `{ <PeerID>: "<nom d'auteur>" }`, une entrée par pair, inscrite
  avant sa première édition, dans le même commit (`claim_author`). Le nom
  vient du réglage `[author] name`, sinon de l'utilisateur système
  (`USER`/`USERNAME`) ; blanc des deux côtés → pair sans entrée, ses
  modifications restent anonymes dans un `MergeReport`. L'inscription fait
  partie de l'étape annulable : annuler la première édition d'un pair retire
  aussi son entrée (une édition entièrement annulée n'a rien laissé à
  signer), jusqu'à sa prochaine édition, qui l'inscrit de nouveau.

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
  - `DeckReplica::new(deck)` (nouveau `deck_uid`), `from_snapshot(bytes)`
    (rend aussi la première lecture, faite sous garde), `snapshot()`.
  - `reconcile(&Deck)` puis `commit(label)`.
  - `deck() -> (Deck, Vec<Issue>)` : **lecture totale**.
  - `merge(&DeckReplica)` : vérifie `deck_uid`, importe.
  - Historique : `versions()` (ordre topologique, repères de session et de
    fusion), `deck_at(index)`, `restore(index)`.
- **`SourceBundle`** gagne `replica: DeckReplica`. `deck` reste une vue
  matérialisée utilisée par le rendu, `validate`, `locate` et les ops.
  **Invariant : la vue est égale à la relecture du replica**
  (`replica().deck().0 == *deck()`). La relecture étant canonique, la vue
  doit l'être : `oku_slides_core::canonicalize(&mut Deck)` (`canonical.rs`)
  est appelé en fin d'`apply_all` et dans `SourceBundle::new`. Règles :
  runs vides retirés, runs voisins de même format fusionnés, `U+2029` dans
  le texte d'un run remplacé par `\n` (le contenu est gardé, sans nouveau
  paragraphe) ; prose facultative faite d'espaces → `None`, autre prose
  rognée (règle d'`opt_string`) ; notes faites d'espaces → `""` ; entier
  JSON au-delà d'`i64` (`vars`, extras de `params`, `tokens`) → flottant,
  comme Loro le stocke ; `tokens` vide → `null`. `SourceBundle::apply`
  attend un deck canonique et le vérifie par `debug_assert!` (builds de
  débogage). Une propriété (`canonical_tests.rs`, proptest) le garantit :
  pour tout deck canonique `d`, `DeckReplica::new(&d).deck().0 == d`.
  Reste hors de l'invariant : une valeur imbriquée au-delà de
  `MAX_DEPTH` (64) dans `vars`, `tokens` ou `params`, tronquée à la lecture.
- **`oku-slides-core` supprime** `History` (`history.rs`) et les inverses :
  `apply_all` applique atomiquement et ne renvoie rien.
- Dépendance nouvelle : `loro` (espace de travail). Dev : `proptest`.

## Flux d'écriture

`Session::apply` :

1. `canonicalize_charts(ops)`.
2. `deck' = apply_all(clone(deck), ops)` (forme canonique) ; erreur → rien n'a bougé.
3. Rendu de `deck'` ; erreur → rien n'a bougé.
4. `replica.apply(&deck', label)`, `deck = deck'`.

Le rendu passe **avant** le commit : l'historique Loro est définitif, un
rendu raté ne doit pas y laisser de trace.

- **Un commit = un *change* Loro.** `set_change_merge_interval(-1)` désactive
  la fusion automatique des commits locaux d'une fenêtre de temps (1000 s par
  défaut) ; sans elle, Loro regrouperait plusieurs commits en un seul
  *change* et « N modifications » (fusion, curseur du mode historique) ne
  voudrait plus rien dire. Le libellé de l'application (`apply(deck, label)`)
  est aussi posé comme message du *change* (`set_next_commit_message`), pour
  qu'un outil externe qui lit l'historique Loro retrouve l'intention de
  chaque étape sans passer par l'app.
- **Un `mov` par commit** (t-263). Loro 1.16.2 défait mal un item
  d'annulation qui porte plusieurs `mov` sur une liste de conteneurs : un
  élément est supprimé (durablement, dans l'oplog), ou Loro panique. La
  réconciliation committe donc juste avant chaque `mov` ; une application
  qui réordonne `k` fois fait `k + 1` *changes*. Les commits
  intermédiaires portent le message `oku-part:<libellé>` : l'état qui les
  suit n'est pas une version du deck (le mode historique les saute).
- **Pair** : `PeerID` aléatoire à chaque chargement (ouverture, reprise,
  nouvel onglet), jamais persisté. Deux écrivains d'un même pair
  corrompraient le document.
- **Horodatage** : `set_record_timestamp(true)`.
- **Annuler / rétablir** : un `UndoManager` par session, `merge_interval
  = 0`, sous une pile d'**étapes** à nous (`undo.rs`) : une étape = une
  application (ou un groupe), faite des items Loro de ses commits (comptés
  par `on_push`), défaits ou rétablis tous ensemble. Libellés, groupes (un
  tour de l'agent) et limite (`settings.history.limit` étapes) vivent dans
  cette pile ; jamais de `group_start` de Loro, qui fusionnerait les items
  d'un groupe en un seul et ramènerait le bogue des `mov`. Loro garde au
  plus 64 items par étape de la limite ; une étape qu'il a en partie
  écartée n'est plus annulable (jamais défaite à moitié).
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
- **Blob non fiable** : import **et première lecture complète**
  (`deck_uid`, tout l'arbre `deck`) sous `catch_unwind`, car Loro ne
  décode l'état d'un conteneur qu'au premier accès (`from_snapshot` rend
  cette lecture, que `SourceBundle::from_replica` reprend sans relire) ;
  taille maximale de `deck.loro` (`[sources] max_replica_mib = 32`) ;
  panique ou octets refusés → `Corrupted`. Après une panique, le doc
  (verrous empoisonnés) est abandonné sans drop. `merge` essaie l'import et
  la relecture sous garde sur une copie avant d'importer. Restent hors garde
  les conteneurs que la première lecture n'atteint pas (hors `deck`/`meta`,
  au-delà de `MAX_DEPTH`).
- **Zip non fiable** : chaque entrée est lue avec une capacité initiale
  plafonnée (1 Mio, la taille annoncée n'est qu'indicative) et une lecture
  bornée (`take`) ; le total décompressé est borné par `[sources]
  max_sources_mib = 256` (anti-bombe), dépassement → `SourcesErrorKind::TooLarge`
  (`sources-too-large`). Les entrées inconnues ne sont pas lues.
- **Lecture totale, trois niveaux** : un élément illisible est écarté
  (`issue-unreadable-element`), puis une slide (`issue-unreadable-slide`),
  puis un champ de haut niveau du deck (`theme`, `params`, `vars`,
  `templates`), remis à sa valeur par défaut (`issue-unreadable-field`).

## Fusion

- **Parenté** : deux replicas ne fusionnent que si leur `deck_uid` est égal ;
  sinon `Unrelated` (« ces deux présentations n'ont pas d'origine
  commune »).
- **Étapes** (`SourceBundle::merge`, appelé par `Session::absorb`) : lire le
  blob entrant, vérifier `deck_uid`, `import`, union des assets et polices,
  relire (lecture totale), rendre. L'avis et `dirty` sont posés dès la
  fusion faite, avant le rendu : un rendu raté ne doit pas laisser passer un
  contenu fusionné pour enregistré. Une fusion sans rien de nouveau (0
  modification, aucun asset ni police ajouté) ne marque pas le document
  modifié ; demandée (`merge_file`), elle pose quand même l'avis « 0 » ; à
  l'enregistrement, elle n'en pose aucun (le toast « Enregistré » suffit).
- **Avis** : `notice-merged` (« Fusionné avec *x.pdf* — modifications
  importées : N (auteurs) ») quand la fusion importe au moins un pair nommé,
  `notice-merged-anonymous` (sans la liste d'auteurs) sinon — pairs sans
  entrée dans `peers`, ou fusion à 0 modification. N compte les *changes*
  importés, pas seulement les éditions : annulations et rétablissements,
  canonicalisations silencieuses et inscriptions d'auteur en sont aussi.
  L'avis ne nomme que les auteurs inscrits et ne mentionne pas les pairs
  anonymes (le `MergeReport` les compte, `anonymous`). Un avis s'ajoute à
  celui qui attend encore (onglet endormi réveillé par la fusion et relu
  avec son propre avis, fichier ouvert avec des parties illisibles…),
  séparé par une ligne vide ; chacun ne se montre qu'une fois.
- **Déclencheurs**, tels que construits (trois écarts à la conception
  initiale, ci-dessous) :
  1. *Fichier → Fusionner avec un PDF…* sur l'onglet actif (`merge_file`,
     refusé en mode modèles).
  2. **À l'enregistrement** (`save_checked`) : si le fichier sur disque a
     changé depuis l'ouverture ou le dernier enregistrement (empreinte de
     `saved_hash`, comparée aux octets avant tout décodage) et porte le même
     `deck_uid`, fusion puis écriture. S'il porte un autre deck, plus de
     sources Oku Slides, ou est illisible : rien n'est écrit, le prochain
     snapshot porte `disk_replaced: true` (une fois, comme `notice`) ; l'UI
     propose d'écraser ou d'enregistrer sous un autre nom (`ui/src/merge.ts`,
     `saveGuarded`). Les deux choix rappellent `save` avec `overwrite: true` :
     après « Enregistrer sous… », le dialogue système d'enregistrement a déjà
     fait confirmer le remplacement du fichier choisi, même quand c'est le
     même fichier qu'avant. Une réponse qui porte encore `disk_replaced`
     (l'utilisateur a visé un chemin lui aussi remplacé entre-temps) vaut
     pour `saveGuarded` « rien n'a été enregistré ». Seul un fichier absent
     (`StorageErrorKind::NotFound`) compte comme inchangé — toute autre
     erreur de lecture (droits, disque réseau, fichier trop gros) est
     traitée comme un remplacement, par prudence. Limite acceptée : la
     vérification et l'écriture (`write_atomic`) ne sont pas atomiques
     ensemble ; sur un disque synchronisé, un autre écrivain qui dépose sa
     version entre les deux est écrasé sans fusion.
  3. **À l'ouverture**, deux voies selon que l'hôte peut poser une question :
     - *Ouvrir…*, glisser-déposer sur la fenêtre : la requête en lecture
       seule `SameDeck { path }` (`Workspace::same_deck`, sans effet) cherche
       l'onglet non vierge qui tient déjà ce `deck_uid` sous un autre
       chemin (un onglet en mode modèles ne compte pas : il refuserait la
       fusion). S'il y en a un, l'UI demande « Fusionner dans cet onglet » ou
       « Ouvrir à part » puis, selon le choix, envoie `Request::Merge` à cet
       onglet ou ouvre un nouvel onglet. L'avis de fusion s'affiche dans
       l'onglet fusionné ; s'il est dans une autre fenêtre, dans l'onglet
       actif de la fenêtre d'où l'on a ouvert (`mergedNotice`), sans amener
       l'autre fenêtre au premier plan.
     - **Fichiers remis par l'OS** (Finder, glisser-déposer sur l'icône,
       ligne de commande — `open_files`) : l'hôte ne peut pas poser de
       question à l'ouverture, le fichier s'ouvre toujours à part ; l'avis
       `notice-same-deck-open` se pose sur le nouvel onglet et nomme
       l'onglet déjà ouvert (« Cette présentation est déjà ouverte dans
       l'onglet « … » »), sans rien fusionner.
- Sous Tauri, ce détour par une requête en lecture seule est nécessaire
  parce qu'une requête d'espace de travail qui changerait la structure rend
  la vue synchronisée à la place de sa réponse
  (`src-tauri/src/commands.rs:77-95`) : `SameDeck` ne change rien, seule la
  fusion qui suit (sur l'onglet trouvé) le fait.
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
| `oku-slides-core::replica` | `ReplicaError` | `Corrupted`, `TooLarge`, `Write` ; `Unrelated` avec la fusion |
| `oku-slides-core::sources` | `SourcesError` | existants ; `MissingDeck` couvre l'absence de `deck.loro` |
| `oku-slides-session` | erreur de session | `MergeUnrelated`, `HistoryNotOpen` (mode historique, plan 3) |

- **`MergeUnrelated`** reprend le message existant `replica-unrelated`
  (« ces deux présentations n'ont pas d'origine commune ») : pas de nouvelle
  clé Fluent.
- **`DiskReplaced` n'est pas une erreur.** Les erreurs de session n'arrivent
  à l'UI que sous forme de texte déjà traduit (`state.refuse` → `localize`),
  qui ne se distingue pas d'un autre message ; or l'UI doit réagir
  différemment (proposer d'écraser ou d'enregistrer sous). Le disque
  remplacé devient donc l'indicateur `disk_replaced: bool` du `Snapshot`
  (`views.rs`), posé une fois par `save_checked` et effacé au prochain
  rendu — même patron que `notice`.

Ne sont pas des erreurs : un sous-arbre illisible après fusion (écarté,
signalé par un `Issue`) ; un rendu raté pendant un apply (rien n'est
validé, l'erreur remonte comme aujourd'hui) ; un disque remplacé par un
autre deck à l'enregistrement (`disk_replaced`, ci-dessus). Messages en
Fluent : anglais d'abord, puis les langues existantes.

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
  l'enregistrement (fichier remplacé : `Saved::DiskReplaced`, indicateur
  `disk_replaced`) ; mode historique (ouvrir, `seek`, restaurer = nouveau
  commit, fermer).
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
- **Confidentialité** : chaque PDF partagé porte tout le contenu passé du
  deck (slides, éléments et textes supprimés, anciennes versions des
  textes, assets retirés), l'horodatage de chaque modification et le nom
  d'auteur de chaque pair qui a édité (map `peers` : réglage `[author]
  name`, sinon le nom d'utilisateur système). Tout destinataire peut le
  relire.
  Accepté par conception (pas d'aplatissement de l'historique).
  Atténuation : l'export sans sources, `oku-slides strip <entrée> <sortie>`
  (`oku_slides_pdf::strip_sources`), pour l'instant en ligne de commande
  seulement.
- La pile d'annulation est perdue au relancement (recul sur t-145).
- Les PDF v1 ne s'ouvrent plus.
- Le décodeur de Loro lui-même peut faire déborder la pile sur un snapshot
  forgé, avec une valeur plate imbriquée sur des dizaines de milliers de
  niveaux (quelques Kio compressés) : Loro la décode récursivement au
  premier accès et la libère au drop, pas à l'`import`, hors de notre code,
  hors de portée de `catch_unwind` et du plafond d'octets. Déni de service
  local à l'ouverture d'un PDF forgé. Suivi comme un correctif à part
  (piste : accéder et libérer dans un fil à grosse pile, ou correctif
  amont Loro).
- La reprise écrit `deck.loro` puis `document.json` : un arrêt brutal entre
  les deux peut laisser un replica neuf à côté d'un `document.json`
  d'avant (`dirty` / `saved_hash` périmés). L'ancien format n'écrivait
  qu'un seul fichier.

Les trois premiers points sont des reculs assumés, à consigner dans un ADR.
