# Instance unique, onglets et fenêtres — design

Date : 2026-09-26 · Tâche : t-142

## Objectif

Plusieurs présentations ouvertes en même temps, dans des onglets, répartis
dans plusieurs fenêtres pour profiter de plusieurs écrans. Une seule instance
de l'app : « ouvrir avec… » alors qu'elle tourne ouvre un onglet dans la
fenêtre existante.

Modèle « navigateur » : chaque fenêtre a sa barre d'onglets ; un onglet (un
deck) n'est ouvert qu'à un endroit ; il se déplace d'une fenêtre à l'autre par
menu ou par glisser-déposer.

Hors périmètre : un même deck affiché dans deux fenêtres ; mode présentation
ou vue présentateur ; menu d'application propre (seul le menu macOS par
défaut est reconstruit, sans « Fermer la fenêtre ») ; onglets fermés
récupérables ; reprise de l'ancien format (v1).

## Architecture

- **Une `Session` par onglet.** `Session` ne change presque pas : chaque
  onglet garde son rendu Typst, son cache SVG, son historique et son mode
  modèles.
- Nouveau module `crates/oku-slides-session/src/workspace.rs` (tests :
  `workspace_tests.rs`) : `Workspace` tient les fenêtres, leurs onglets
  ordonnés, l'onglet actif de chacune et la dernière fenêtre active, et les
  sessions (`Arc<Mutex<Session>>` par onglet). Il ouvre, ferme, réordonne et
  déplace les onglets.
- Verrous. Celui du `Workspace` ne couvre que sa structure : une requête
  d'onglet ne verrouille que sa session, deux fenêtres rendent en parallèle.
  Ordre : l'espace de travail puis une session, jamais l'inverse. Il vit dans
  quatre fonctions du crate (`Workspace::apply`, `Workspace::request`,
  `Workspace::open_files`, `Workspace::settle`) que le serveur de dev et
  l'hôte appellent. Aucun
  travail lent sous le verrou de l'espace de travail : une session neuve se
  construit hors du verrou (identifiant réservé, session créée et ouverte,
  puis insérée), et les dossiers des onglets fermés sont supprimés après.
  Seule exception : la fermeture lit l'état réel des sessions concernées. Un
  verrou empoisonné est repris (`PoisonError::into_inner`) : une panique ne
  bloque pas l'app.
- Identifiants : onglets `t-<n>`, fenêtres `w-<n>` (le label Tauri de la
  fenêtre), `n` de 1 à 18 chiffres. Compteurs gardés dans `workspace.json`,
  jamais réutilisés.
- `Workspace::new(fonts, platform, file)` : une fenêtre et l'onglet `t-1`,
  vierge ou avec `file` ouvert ; rien n'est écrit (serveur de dev, tests).
  `Workspace::resume` : l'app de bureau.
- Ouvrir un fichier (dialogue, argument, seconde instance, macOS
  `RunEvent::Opened`) : s'il est déjà ouvert, son onglet est activé (et sa
  fenêtre mise au premier plan), y compris si une autre ouverture du même
  fichier s'est terminée pendant la construction. Sinon un onglet est créé
  dans la dernière fenêtre active ; si l'onglet actif est vierge (deck neuf
  jamais touché : ni ouvert, ni enregistré, ni modifié, ni annulé — l'état
  réel de sa session fait foi, lu sans attendre : une session occupée n'est
  pas vierge), il est fermé une fois le nouvel onglet prêt, qui prend sa
  place. Les chemins sont comparés absolus et normalisés (`.`,
  `..`) ; les liens symboliques ne sont pas suivis.
- Fermer le dernier onglet d'une fenêtre ferme la fenêtre ; fermer le
  dernier onglet de la dernière fenêtre termine l'app (cet onglet, fermé
  exprès, n'est pas gardé : au lancement suivant, un onglet vierge). Ce
  `Close` renvoie `Applied` avec une vue sans fenêtre ; l'hôte quitte l'app
  en la voyant vide (`windows::sync` appelle `state::shutdown`). Le serveur
  de dev, sans fenêtre native à quitter, le refuse d'avance (`409`,
  `session-last-window`) sans rien fermer : l'interface resterait sinon sur
  « chargement ». `WorkspaceOutcome::LastWindow` ne concerne que
  `CloseWindow` sur la seule fenêtre (voir plus bas) : l'hôte quitte en
  gardant tout, le serveur de dev répond aussi `409`.
- Quitter l'app garde tout : ⌘Q, et fermer la **dernière** fenêtre (seul
  moyen de quitter sous Windows et Linux) ; aucune question n'est posée.
  L'hôte applique cette règle lui-même. Fermer une fenêtre qui n'est pas la
  dernière ferme ses onglets.
- Un seul chemin de sortie, `state::shutdown` : il marque `AppState::closing`
  (`request`, `workspace` et `drop_tab` refusent tout ensuite, sans plus
  toucher l'espace de travail), appelle `Workspace::settle` (attend les
  requêtes en cours puis enregistre l'état, sous un verrou qui pose aussi
  `closed` : un `OpenTab` ensuite est refusé, et un onglet dont la
  construction chevauche `settle` n'est pas inséré, sa session part à la
  corbeille), puis `app.exit(0)`. Il est appelé depuis `commands::quit`, le
  menu « Quitter » (item propre à macOS, reconstruit par-dessus le menu par
  défaut de Tauri), la fermeture de la dernière fenêtre, et — filet de
  sécurité seulement, car `Workspace::settle` ne doit jamais être invoqué
  depuis le fil principal — `RunEvent::Exit`, où plus aucun fil ne peut
  détenir ces verrous.
- `Close { tab, discard }` et `CloseWindow { window, discard }` vérifient
  l'état réel de chaque session : un onglet modifié que l'utilisateur n'a pas
  choisi d'abandonner (`discard`) est refusé (erreur `Unsaved`). L'indicateur
  de l'UI peut être en retard sur un rendu en cours.
- Modèles partagés : rien à invalider. Le cache d'aperçus d'une session
  (`template_cache`) compare l'ensemble relu sur le disque à chaque appel :
  un ensemble enregistré dans un onglet se voit aussitôt dans les autres. Un
  test le garantit. Deux onglets qui enregistrent le même fichier en même
  temps ne se gênent pas : `Storage::write_atomic` prend un fichier
  temporaire propre à chaque écriture.
- `Request` perd `New`, `Open` (l'ouverture passe par l'espace de travail)
  et `Messages` (qui ne dépend d'aucun onglet : il passe à
  `WorkspaceRequest`).
- `open_at_launch` et les clés `notice-resume-kept-unsaved` et
  `session-busy` sont supprimés : un fichier reçu n'écrase plus rien, il
  ouvre un onglet.
- Serveur de développement (`serve.rs`) : `Workspace::new` avec le fichier
  donné, une seule fenêtre, sans reprise.

## Fichiers de reprise

Sous `<data>/resume/`, via `Storage::write_atomic` :

```
resume/
  workspace.json    { version: 2, next_tab, next_window, focused,
                      windows: [{ label, tabs: [id…], active }], kept: [id…] }
  tabs/<id>/        la forme de l'ancien resume/ (v1) :
    state.json      { version, mode, ui }
    deck/ …         document.json (avec `blank`), assets/, fonts/
    templates/ …
```

- Chaque `Session` n'écrit que `tabs/<id>/` (un écrivain par dossier) ; seul
  le `Workspace` écrit `workspace.json`.
- Relu, `workspace.json` n'est pas digne de confiance : seuls les
  identifiants `t-<chiffres>` et les labels `w-<chiffres>` sont acceptés,
  sans doublon ; les compteurs sont ramenés au-delà du plus grand identifiant
  relu (ou vu sur le disque), et un compteur au-delà de 18 chiffres est
  remis à ce minimum. Sinon un chemin (`../..`) sortirait du dossier, un
  label invalide ferait paniquer la création de fenêtre à chaque lancement,
  et deux onglets partageraient un dossier.
- `kept` : onglets cités mais non restaurés dont le dossier existe encore
  (illisible et impossible à mettre de côté, fenêtre refusée…). Ils restent
  cités de lancement en lancement : leur dossier n'est jamais supprimé.
- `document.json` garde `blank` : un onglet vierge reste vierge au
  relancement.
- Changer de fenêtre active n'écrit rien : l'ordre est enregistré avec le
  changement suivant.
- Création d'un onglet : son dossier d'abord, `workspace.json` ensuite.
  Fermeture : `workspace.json` d'abord, le dossier ensuite. Un arrêt entre
  les deux laisse au pire un dossier orphelin, supprimé au lancement suivant.
- Déplacer un onglet ne touche que `workspace.json`.
- Ancien format (v1 : `resume/state.json` sans `workspace.json`) : non géré,
  pas de shim. On efface `<data>/resume/` à la main, une fois, avant le
  premier lancement de cette version ; sinon il reste là, ignoré.

## Restauration

1. `workspace.json` absent : une fenêtre, un onglet vierge.
2. Illisible ou version inconnue : tout `resume/` mis de côté
   (`resume.failed-…`), une fenêtre, un onglet vierge.
3. Chaque onglet se restaure comme la reprise v1 (fichier modifié ailleurs,
   historique illisible…). Onglet illisible : son dossier seul est mis de
   côté (`t-3.failed-…`), l'onglet est retiré, les autres restent. Onglet
   cité sans `state.json` : retiré (dossier gardé s'il existe, via `kept`).
   Fenêtre restée sans onglet : retirée. Plus aucune fenêtre : une fenêtre,
   un onglet vierge.
4. Dossiers `tabs/t-<n>` que `workspace.json` ne cite pas (ni dans une
   fenêtre, ni dans `kept`) : supprimés. Les dossiers mis de côté et les
   noms étrangers restent.
   Un onglet neuf ne relit rien : un reste trouvé à son numéro est mis de
   côté, l'onglet garde son deck (le fichier tout juste ouvert) et sa
   reprise démarre à neuf ; seul un reste impossible à déplacer le laisse
   sans reprise.
5. Fichier(s) passé(s) en argument : ouverts ensuite, hors du fil principal,
   selon la règle d'ouverture ci-dessus. Illisible : journalisé, avis dans
   l'onglet actif.

## Hôte ↔ interface

- `request(tab, req)` : la `Request` existante, adressée à un onglet.
- `workspace(req)` avec `WorkspaceRequest` : `View`, `Messages`,
  `NewTab { window }`, `OpenTab { window, path }`, `Activate { tab }`,
  `Close { tab, discard: bool }`, `Reorder { tab, index }`,
  `MoveTab { tab, window: Option<String>, index: Option<usize> }` (`None` :
  nouvelle fenêtre ; sans index : à la fin ; même fenêtre sans index :
  l'onglet est seulement activé),
  `NewWindow`, `CloseWindow { window, discard: Vec<String> }`. Réponse : la
  vue, en JSON (`Messages` : langue et ressources Fluent). La fenêtre active
  est suivie par l'hôte (`Workspace::focus`, en mémoire), pas par une
  requête. Côté Tauri, chaque événement de focus reçoit un numéro pris sur le
  fil principal (`AppState::focus_seq`) ; l'application (hors fil principal)
  compare ce numéro à `AppState::focus_applied` sous le verrou de l'espace de
  travail et ignore un événement plus vieux que le dernier appliqué : l'ordre
  d'arrivée fait foi même si les tâches qui l'appliquent se terminent dans le
  désordre.
- `drop_tab(tab)` (Tauri seulement) : onglet lâché hors de sa fenêtre. L'hôte
  lit lui-même le curseur (tao, jamais `screenX`/`screenY` : t-165) et le
  compare aux rectangles des fenêtres dans son repère : les points sous
  macOS, les pixels physiques du bureau sous Windows et X11, communs à tous
  les écrans même de densités différentes (t-160). Sous Wayland, curseur et
  position des fenêtres ne se lisent pas (tao renvoie `(0, 0)`, qui compte
  pour illisible sous Linux) : le lâcher ouvre une nouvelle fenêtre, placée
  par le compositeur. Hors macOS, une fenêtre neuve est placée cachée, sa
  taille réappliquée sur l'écran visé, puis positionnée d'après sa hauteur
  réelle (barre de titre comprise) avant d'être montrée.
- `quit` (Tauri seulement) : quitter en gardant tout.
- `WorkspaceView` : `{ focused, windows: [{ label, active, tabs: [{ id,
  title, dirty, editing_templates, path, pristine }] }] }`. Titre : nom du
  fichier, sinon titre du deck (le deck mis de côté pendant l'édition des
  modèles).
- L'hôte émet l'événement `workspace` (la vue) à toutes les fenêtres après
  toute commande qui change un onglet (titre, `dirty`) ou la structure. Les
  fenêtres suivent la vue la plus récente, dans l'ordre (mises à jour
  sérialisées). À chaque événement, l'onglet actif de chaque fenêtre relit
  aussi son snapshot (`nonce` passé de `Workspace.tsx` à `App`) : un avis
  posé sans changer d'onglet (fichier reçu qui n'a pas pu s'ouvrir)
  s'affiche aussitôt. Une réponse plus ancienne qu'une autre déjà appliquée
  est écartée (`latest.ts`) ; son avis s'affiche quand même. En
  développement, la relecture de la vue après chaque commande ne relance pas
  le snapshot (pas de boucle).
- `drop_tab` : la fenêtre dont le rectangle contient le pointeur reçoit
  l'onglet en dernier ; en cas de chevauchement, la plus récemment active.
  Aucune : nouvelle fenêtre dont la barre d'onglets tombe sous le pointeur.
- `Close` d'un onglet modifié (deck, ou ensemble de modèles en édition) :
  l'UI relit la vue puis demande — dialogue natif Enregistrer / Ne pas
  enregistrer / Annuler ; « Enregistrer » sans chemin ouvre le dialogue
  d'enregistrement.
- Fermeture native d'une fenêtre (bouton, ⌘W du système, Alt+F4) : l'hôte
  l'empêche toujours. La dernière fenêtre : l'hôte quitte en gardant tout.
  Une autre : l'hôte prévient la page (`close-requested`), qui pose la même
  question pour chacun de ses onglets modifiés, puis `CloseWindow`. Une page
  pas encore prête : la fenêtre ne se ferme pas (rien n'est perdu). ⌘Q ne
  demande rien.
- Menu d'app sous macOS : celui de Tauri sans « Fermer la fenêtre », dont le
  ⌘W passerait avant la page. Titres « Édition » et « Fenêtre » via Fluent.
- Instance unique : `tauri-plugin-single-instance` (tous systèmes : une
  seconde instance transmet ses arguments) et `RunEvent::Opened` (macOS,
  Finder) → `OpenTab` dans la dernière fenêtre active, mise au premier plan,
  hors du fil principal. Un `Opened` arrivé avant la fin du démarrage attend
  dans une file (`pending.rs`, plafonnée à 64 fichiers par appel : l'excédent
  est journalisé et abandonné, contre un argument abusif ou une liste macOS
  anormalement longue), vidée à la fin de `setup`. Seuls les fichiers
  existants sont retenus. Sous Linux, `tauri-plugin-single-instance` 2.4.5
  fait un `unwrap()` sur le bus de session D-Bus et panique s'il n'y en a pas
  (CI headless, conteneur sans D-Bus) ; le greffon n'est chargé que si
  `DBUS_SESSION_BUS_ADDRESS` est posé ou que `$XDG_RUNTIME_DIR/bus` existe
  (`XDG_RUNTIME_DIR` seul ne prouve pas qu'un bus tourne ; toujours chargé
  ailleurs), sinon l'instance unique est simplement désactivée (journalisé
  en `warn`).
- Fenêtres créées par `WebviewWindowBuilder` avec leur label ;
  `tauri-plugin-window-state` restaure taille et position par label (celles
  de l'ancien label `main` sont perdues une fois). `tauri.conf.json` ne
  déclare plus de fenêtre ; la capacité couvre `w-*`, sans
  `allow-destroy` : l'hôte seul ferme les fenêtres.
- Serveur de dev : `POST /api` reçoit `{ tab, req }`, `POST /api/workspace`
  une `WorkspaceRequest` ; pas d'événement (une seule fenêtre : l'UI relit la
  vue après ses propres commandes). Il refuse (403) une requête dont l'en-tête
  `Host` n'est pas un nom de bouclage (`localhost`, `127.0.0.1`, `[::1]`,
  port quelconque) ou dont l'`Origin` n'en est pas un : une page web ne peut
  pas le piloter (CSRF, DNS rebinding), mais `just web` (Vite sur 5173)
  passe. Une requête sans `Origin` est acceptée volontairement (un client qui
  n'envoie jamais cet en-tête n'est pas un navigateur pilotable par un CSRF,
  et un navigateur envoie toujours `Origin` sur un `POST`). Le corps d'une
  requête `/api*` est plafonné à 64 Mio ; au-delà, `413`. Dans le navigateur,
  les actions de fenêtre (nouvelle fenêtre, déplacer) sont masquées et le
  dernier onglet ne se ferme pas.

## Interface

- `main.tsx` monte `Workspace.tsx` : barre d'onglets dans la zone de titre
  (`Overlay`, à droite des boutons de fenêtre macOS) et l'`App` de l'onglet
  actif, `key={tab}`. Seul l'onglet actif est monté : slide courante et
  grille reviennent de l'hôte (t-87). Ouvrir un fichier change toujours
  d'onglet actif, ce qui remonte l'`App`.
- `App` reçoit `tab` ; `tabApi(tab).*` remplace `api.*` (aussi dans
  `TemplatePicker`, qui reçoit l'API de son onglet).
- Onglet : titre, point si modifié, bouton fermer ; menu contextuel
  « Nouvelle fenêtre », « Déplacer vers <fenêtre> », « Fermer »,
  « Fermer les autres ».
- Glisser dans la barre : réordonner. Glisser hors de la fenêtre (dans
  n'importe quelle direction) : au lâcher, `drop_tab`. En développement
  (navigateur), rien. En RTL (`document.dir`), la barre s'affiche de droite à
  gauche (l'ordre du tableau ne change pas) : la place visée par le pointeur
  se calcule en miroir (`slotAt(mids, x, rtl)`).
- Raccourcis : ⌘T nouvel onglet, ⌘W fermer l'onglet, ⌘⇧N nouvelle fenêtre,
  ⌃Tab / ⌃⇧Tab onglet suivant / précédent, ⌘1…⌘9 onglet n (⌘9 : le
  dernier), ⌘O ouvrir (règle d'ouverture). ⌃ remplace ⌘ hors macOS. Lettres
  lues par caractère (`key`) : ⌘Z reste « annuler » en AZERTY. Chiffres par
  touche physique (`code`) : ⌘1 marche en AZERTY. Répétitions ignorées ;
  pendant une saisie de texte, seul ⌃Tab reste actif.
- Textes nouveaux via Fluent, dans les 20 locales.

## Erreurs

- `UnknownTab { id }`, `UnknownWindow { label }` (course entre une fermeture
  et une requête), `Unsaved { title }` (fermeture refusée) : traduites ;
  l'UI relit la vue `workspace`.
- Échec d'écriture de `workspace.json` : journalisé (`warn`), jamais renvoyé
  à l'UI, comme la reprise v1.

## Tests (TDD)

- `workspace_tests` (unitaires) : nouvel onglet ; espace de travail créé
  avec un fichier ; ouvrir un fichier déjà ouvert (même écrit autrement)
  l'active ; onglet vierge remplacé, onglet modifié ou enregistré gardé ;
  fermer (voisins, dernier onglet, `Unsaved`, modèles seuls modifiés) ;
  fermer une fenêtre (rien n'est fermé si un onglet est refusé) ;
  réordonner ; déplacer (autre fenêtre, nouvelle fenêtre, même fenêtre,
  fenêtre inconnue) ; fenêtre sous un point (une seule, chevauchement, hors
  de toute fenêtre, fenêtre inconnue) ; verrou empoisonné repris ; requête
  et fermeture en parallèle sans interblocage ; messages ; validation des
  identifiants ; construction qui chevauche `settle` (refusée, mise à la
  corbeille) ; onglet neuf sur un reste de reprise (fichier gardé).
- `tests/session.rs` : infos d'onglet (vierge, modifié, enregistré, JSON
  ouvert, modèles) ; avis montré une fois ; un ensemble de modèles
  enregistré dans une session se voit dans une autre.
- `tests/workspace.rs` (reprise) : aller-retour complet (2 fenêtres, 3
  onglets, actifs, un onglet en mode modèles, historique et `undo`) ;
  `workspace.json` corrompu ; un onglet corrompu parmi trois ; onglet cité
  sans dossier ; onglet cité sans état gardé sur deux lancements ; fenêtre
  refusée ; identifiants altérés ; compteurs trop bas ou trop hauts ;
  orphelins supprimés, dossiers mis de côté et étrangers gardés ; fichiers
  au lancement (un fichier deux fois, fichier illisible et son avis).
- `storage_tests` : `resolve`, `list_dirs`, `delete_dir`, écritures
  atomiques simultanées.
- Serveur de dev (`cli.rs`) : routage par onglet, espace de travail, onglet
  inconnu, origine étrangère, DNS rebinding, origine Vite ; fermer la seule
  fenêtre ou son dernier onglet (409, rien n'est fermé) ; corps de plus de
  64 Mio (413).
- `src-tauri` (unitaires, ce qui se teste sans fenêtre) : `main_tests`
  (fichiers de lancement, présence du bus D-Bus), `commands_tests`
  (classement des requêtes, ordre des vues), `pending_tests` (file des
  fichiers reçus au démarrage, plafond), `windows_tests` (point de dépôt).
- UI (`bun test`) : places de dépôt, raccourcis (AZERTY compris),
  navigation, fermeture (choix du dialogue, enregistrement avant fermeture),
  réponses concurrentes (`latest.ts` : une réponse dépassée n'est pas
  appliquée).
- e2e (Playwright, serveur de dev) : ouvrir des onglets, basculer, fermer ;
  question pour un onglet modifié.
- Manuel (le reste de `src-tauri`, fenêtres natives) : deux écrans, glisser
  un onglet d'une fenêtre à l'autre et hors des fenêtres ; seconde instance
  avec un fichier, y compris un fichier illisible (son avis s'affiche
  aussitôt) ; `open -a` sous macOS ; fermeture d'un onglet modifié ;
  dernière fenêtre ; relancement ; clavier AZERTY ; `just web`.

## Limites connues

- « Enregistrer sous » vers un fichier déjà ouvert dans un autre onglet
  n'est pas détecté : deux onglets visent alors le même fichier, et le
  dernier enregistré l'emporte. Ouvrir ce fichier ensuite active l'un des
  deux.
