---
task: t-678
---

# Petits tickets du 9 octobre — 2026-10-09

Tickets revérifiés contre main @ a0a33c94 : t-630, t-635, t-637, t-647, t-649, t-674, t-675, t-676
s'appliquent encore. t-466 : correctif présent (`th/cli.ftl`, forme « nom { $n } รายการ » pour
cli-deck-valid et cli-opened ; aucun autre compteur thaï de la forme « { $n } nom » dans les .ftl) ;
il reste ouvert pour la relecture d'un locuteur. t-648 : vérification manuelle dans l'app de bureau,
préparée (lot H : commentaire du ticket, ni code ni branche), pas close. t-649 et t-675 (attentes
fixes en e2e) forment un seul lot (G).

Plan relu par l'équipe commune (8 relecteurs) et corrigé : lot E repensé (⌘Q et la dernière fenêtre ne
passent pas par la page), test Rust du lot A corrigé (pas de lecteur markdown en Rust), arrêt borné et
foyer du lot B fixés, forme du lot C et portée du lot G fixées.

## Règles

- Un worktree par lot dans `.worktrees/678<lettre>-…`, `git config --worktree oku.review deferred`,
  un `CARGO_TARGET_DIR` privé par worktree (`.worktrees/target-678<lettre>`), jamais partagé.
- Les lots UI seuls (F, G) ne compilent pas Rust : pour l'e2e, ils utilisent `oku-slides` et
  `oku-slides-fake-agent` construits une fois depuis main, copiés dans le bloc-notes de session ; le
  serveur sert le `ui/dist` du worktree du lot.
- TDD ; bug corrigé = test de régression qui échoue avant le correctif à l'exécution (pas seulement à la
  compilation) : `tests/regressions/issue_<N>_*.rs` côté Rust, test bun nommé côté UI, assertion e2e
  côté mise en page.
- Déplacement pur = son propre commit, aucun test modifié, suites vertes avant et après.
- Manifestes : seul le lot B les modifie (feature de `oku-slides-platform`, retrait de hyper /
  hyper-util de la CLI si inutilisés) ; aucune nouvelle crate dans Cargo.lock.
- Sessions parallèles : t-654 (zone agent détachable : `ui/src/App.tsx`, `Workspace.tsx`, `api.ts`,
  `styles.css`, `ui/src/agent/*`, `src-tauri`, `oku-slides-session/src/workspace*`,
  `oku-slides-agent/src/tab_entry.rs`, `oku-slides-mcp/ARCHITECTURE.md`) et t-677 (tableaux : core
  sources/canonicalize, `ops/host.rs` possible, chart, grille UI). Contacts acceptés et minimes :
  lot B (`oku-slides-mcp/ARCHITECTURE.md`, une ligne), lot C (`ops/host.rs`, constructions d'erreur),
  lot E (`ui/src/api.ts`, la commande `ui`), lot F (`styles.css`, règles de la feuille). App.tsx,
  Workspace.tsx, tab_entry.rs et src-tauri ne sont pas touchés.
- e2e à la main : serveur neuf et dossier de données neuf (`mktemp -d`) pour chaque spec, port libre
  (jamais 4872).
- Une revue par lot (conformité au plan + qualité), corrections relues.

## Lot A — Liens commençant par « < » (t-630)

`write_barrier` (`crates/oku-slides-session/src/inline.rs`) et `encodeLink` (`ui/src/markdown.ts`)
encodent espace et parenthèses : une adresse `<x` s'écrit `[a](<x)`, que CommonMark lit comme une
destination entre chevrons non fermée (le lecteur UI, `markdown.ts`, la garde en texte) ; une adresse
finissant par `\` écrit `[a](x\)`, où `\)` ne ferme pas la destination. Des deux côtés : encoder un `<`
**initial** en `%3C` (un `<` au milieu d'une adresse se relit déjà tel quel : ne pas le dégrader) et
tout `\` en `%5C`. Comme pour `%20`, la relecture ne décode pas : le lien relu vaut `%3Cx`.

Tests (rouges avant) :
- `crates/oku-slides-session/tests/regressions/issue_0630_link_angle_bracket.rs` :
  `paragraphs_to_text` d'un run lié à `<x` donne `[a](%3Cx)` ; lié à `x\` donne `[a](x%5C)`.
- `fixtures/markdown.json`, section `lossy` (lue par les suites Rust et bun) : les deux cas, avec le
  markdown écrit et le lien relu (`%3Cx`, `x%5C`), plus `https://a.b/?q=<x>` qui reste exact.
- `markdown.test.ts` : `encodeLink` des deux cas. Note : `encodeLink` n'est atteint qu'après la garde
  `https?:|mailto:` de `insertLink` ; ce test fixe la parité avec Rust, pas un chemin utilisateur.

## Lot B — Serveur MCP : délai d'en-têtes et plafond de connexions (t-637)

`crates/oku-slides-mcp/src/server.rs` sert par `axum::serve`. Un seul axum dans Cargo.lock (0.8.9,
partagé par `rust-mcp-axum`) : `serve_connections(Router, …)` convient tel quel.

1. Déplacement pur : `crates/oku-slides-cli/src/listen.rs` + `listen_tests.rs` → module `listen` de
   `oku-slides-platform`, derrière une feature optionnelle `listen` (axum, hyper, hyper-util et les
   features tokio `net`, `sync`, `time`, `macros`, et `io-util` en dev) ; seules la CLI et `oku-slides-mcp`
   l'activent ; réexport explicite ; visibilité `pub`. La CLI retire hyper / hyper-util s'ils ne servent
   plus. Description de la crate platform mise à jour.
2. `serve_connections` prend en plus un délai de grâce et possède tout l'arrêt : connexions suivies
   (`JoinSet` ou `TaskTracker`), à l'arrêt `graceful.shutdown()` borné par la grâce, puis abandon de ce
   qui reste. L'appelant de la CLI (`serve.rs`) perd son `timeout` propre. Test : une connexion qui ne
   finit jamais sa réponse est fermée au plus tard à la fin de la grâce (rouge avant).
3. Serveur MCP : les bornes sont celles de `[serve]` (`header_timeout_ms`, `max_connections`, déjà
   `NonZero`), lues par `Tools::new` depuis `platform` (pas de nouvelle clé, signature de
   `ToolServer::start` inchangée : `tab_entry.rs` et les deux harnais de test non touchés) ; grâce
   courte (constante documentée, ~1 s : seuls des flux SSE restent). Le 401 de `require_bearer` porte
   `Connection: close` (un client non authentifié ne garde pas de place).
4. Tests, dans `crates/oku-slides-mcp/tests/regressions/issue_0637_*.rs`, via
   `common::start_with_settings` (délai 300 ms, plafond 2), rouges avant à l'exécution :
   connexion muette fermée après le délai ; connexion au-delà du plafond fermée aussitôt ; flux SSE
   authentifié (GET `/mcp`) fermé dans la grâce après `cancel` ; une connexion keep-alive restée
   inactive au-delà du délai puis réutilisée : documenter ce que fait hyper 1.11 (si le délai ferme les
   connexions inactives, le client — undici, keep-alive 4 s par défaut — ferme avant ; le noter dans le
   rapport) ; 401 suivi de la fermeture de la connexion.
5. `oku-slides-mcp/ARCHITECTURE.md` : une ligne (bornes du serveur d'outils).

Hors périmètre (ticket de suivi) : comparaison du jeton en temps constant.

## Lot C — op-param-type dit pourquoi (t-676)

`OpInner::ParamType { expected: &'static str }` : 12 constructions `ParamTypeSnafu` (check 1, host 4,
list 3, mod 3, value 1 — sa fermeture `wrong` appelée 9 fois), ~21 chaînes anglaises dont beaucoup ne
diffèrent que par « or null ». Remplacer par une raison typée à deux axes, à côté de `ValueReason` :
`ExpectedType { shape: Shape, nullable: bool }` (nom libre s'il évite la confusion avec
`crate::ParamType`), `Shape` énumérant les formes distinctes (chaîne, nombre, booléen, choix, liste
d'éléments, liste d'items, option de graphique ou de tableau, valeur via set_param…, liste exacte à
dresser depuis les chaînes actuelles et donnée dans le rapport), `key()` privée comme `ValueReason`,
`Display` anglais identique aux chaînes actuelles (test de non-régression). Fluent : `op-param-type`
remplacé par `op-param-type-because = … { $subject } : { $reason }`, une clé par `Shape` avec un
sélecteur sur `$nullable`, dans les 20 langues. `Shape::ALL`, tenu exhaustif par un `match`, nourrit
le test « chaque clé existe dans chaque langue ».

Tests : par une vraie opération (paramètre nombre reçu `"4"` via `apply`), la traduction française
contient la raison (rouge avant ; table rstest sur plusieurs formes) ; mettre à jour les deux
assertions de texte exact existantes (`crates/oku-slides-core/tests/charts.rs:146`,
`crates/oku-slides-core/tests/ops.rs:698`).

## Lot D — Test instable des polices embarquées (t-635)

`save_with_embed_fonts_puts_the_fonts_in_the_sources` (`crates/oku-slides-cli/tests/cli.rs`) a échoué
une fois, cause inconnue. Dans tous les cas : les assertions affichent stdout et stderr des deux
commandes en cas d'échec. Reproduction : 200 passes de
`cargo nextest run -p oku-slides-cli --test cli -E 'test(save_with_embed_fonts)'` pendant que la suite
`-p oku-slides-cli` complète tourne en boucle en parallèle (charge). Reproduit et compris en un cycle
de diagnostic : test de régression et correctif. Sinon : le ticket reste ouvert avec un commentaire
(passes, charge, résultat, sortie capturée le cas échéant).

## Lot E — État d'interface jamais perdu à la fermeture (t-674)

⌘Q, le Dock et la fermeture de la dernière fenêtre vont droit à `state::shutdown` côté hôte
(`src-tauri/src/main.rs`, `commands.rs::close_requested`), sans passer par la page ; `shutdown` attend
les requêtes déjà parties (`settle`). Ce qui se perd, c'est la sauvegarde **gardée en attente** dans la
file de `useKeptUi` (t-668). Correctif : plus de file côté UI. Chaque changement part tout de suite,
avec un numéro d'ordre ; l'hôte ignore une sauvegarde plus ancienne que la dernière appliquée. L'ordre
est `{ page, seq }` : `page` tiré au chargement de la page (aléatoire), `seq` compteur du module (pas
du composant : il survit au remontage d'`App` d'un onglet à l'autre) ; une autre `page` est toujours
acceptée (rechargement), un `seq` inférieur ou égal pour la même `page` est ignoré. Le numéro n'est pas
gardé dans l'état repris.

Fichiers : `ui/src/useKeptUi.ts` (file retirée), `ui/src/api.ts` (la commande `ui` porte l'ordre),
`crates/oku-slides-session/src/request.rs` (et le type de la requête), sans toucher App.tsx,
Workspace.tsx ni src-tauri.

Tests (rouges avant) : session — deux requêtes `ui` reçues dans le désordre (`seq` 2 puis 1) gardent
celle de `seq` 2 ; une autre `page` avec `seq` 1 est acceptée ; bun — deux changements rapprochés
partent tous deux sans attendre la réponse du premier, avec des `seq` croissants ; le test de t-668
(une sauvegarde plus ancienne n'écrase pas la dernière) reste vert, réécrit sur l'ordre.

## Lot F — La slide ne rétrécit plus à la sélection dans une fenêtre basse (t-647)

À 1024×640, la hauteur de la boîte borne la slide ; la carte des paramètres change de hauteur à la
sélection et réduit la slide (2 px mesurés), qui se décale aussi horizontalement.

1. Test d'abord, `ui/e2e/editor.cjs` étape 0 rejouée à 1024×640 en plus de 1440×900 : la boîte de la
   slide est relevée pendant ~500 ms (sondage) après le changement de carte et ne doit jamais différer
   (largeur, hauteur, x, y) ; flèches à mi-hauteur de la slide aux deux tailles. Confirmer qu'il est
   rouge avant.
2. Correctif, dans `ui/src/styles.css` (règles de la feuille seulement) : la carte des paramètres occupe
   une bande de hauteur fixe sous la scène (`clamp()` sur la hauteur de la fenêtre, défilement propre
   au-delà), si bien que la boîte de la scène ne dépend plus du contenu de la carte.
3. Garde-fous : à 1024×640 sans sélection, la slide n'est pas plus petite qu'avant le correctif
   (mesure de main relevée dans le rapport, tolérance indiquée) ; désélection et passage à l'élément à
   la carte la plus haute ne déplacent pas la slide. Suites bun et e2e editor, toolbar, layout vertes.

## Lot G — Attentes fixes en e2e (t-649, t-675)

Portée, celle des deux tickets :
- `toolbar.cjs` en entier (t-649) : après ajout, suppression et annulation, attendre la vérité de
  l'hôte (`state(p)` : nombre de slides **et** libellé d'annulation attendu, ex. « Annuler : Supprimer
  la slide » avant ⌘Z) ; après redimensionnement, une boîte identique sur deux `requestAnimationFrame`
  consécutifs ; corriger le commentaire périmé de la ligne 19.
- Toute attente fixe avant un `reload` ou une navigation (t-675) : `layout.cjs` (avant le reload),
  `agent.cjs` (avant le reload qui suit les deux ⌘Z), `agent-scopes.cjs` (`settle` avant reload), et
  toute autre trouvée à l'audit : attendre la réponse de l'hôte à la sauvegarde (promesse
  `waitForResponse` créée **avant** l'action) ou l'état de l'hôte. agent.cjs et agent-scopes.cjs :
  attentes seulement.
- Attentes négatives (laisser un effet fautif arriver) : gardées, commentées `// négatif :`.
- Les autres `waitForTimeout` (smoke, panel, help, widgets…) : inventaire dans le rapport et ticket de
  suivi, pas de changement ici.
Preuve : chaque spec modifiée passe 3 fois de suite sur un serveur neuf, et une fois sous
ralentissement CPU ×4 (CDP `Emulation.setCPUThrottlingRate`) ; pour chaque nouvelle condition,
vérifier une fois qu'elle échoue (délai dépassé) si l'effet attendu est supprimé.

## Lot H — Ancrage CSS des flèches dans WKWebView (t-648, préparation seulement)

Fait : procédure en commentaire du ticket (lancer l'app, `CSS.supports('anchor-name: --a')`, écart
entre la mi-hauteur de `.slide-nav.next` et celle de `.stage` — l'ancre est `.stage-frame`, de même
hauteur — ≤ 1 px avec support, à 1440×900 et 1024×640, avant et après sélection). Ticket ouvert.

## Fin

Intégration de tous les lots sur une branche partant du main du moment ; vérification complète :
`cargo fmt --check`, `cargo clippy --workspace --exclude oku-slides-app -- -D warnings`,
`cargo nextest run`, `cd ui && bun run test`, `bun ./node_modules/typescript/bin/tsc -b`,
`bunx --bun vite build`, puis chaque spec e2e sur un serveur neuf avec un dossier de données neuf.
Revue finale de la branche, squash sur main, sans push.

Suivis à créer : jeton MCP comparé en temps constant ; attentes fixes restantes hors portée de G ;
copies locales de `until` (theme-editor.cjs, widget-values.cjs).
