# Fil de l'agent — suites de t-416 (t-448 à t-452)

Spec : `docs/superpowers/specs/2026-10-03-agent-a2ui-design.md` (autorité). Commit d'origine : 39d34c50 (t-416).

## Global Constraints

- Rust : guidelines du dépôt (Snafu, pas de `unwrap`/`expect` hors tests, tests côte à côte `_tests.rs`, speculoos, `insta`, nommage `<unit>_<scenario>_<expected>`). Fichiers < 500 lignes visés, 1000 max.
- TS : tests Bun côte à côte (`*.test.tsx`, `*.render.test.tsx`), pas de `export *`.
- TDD : test rouge d'abord. Chaque correction de bug a un test de régression.
- i18n : tout texte visible passe par Fluent, 20 langues (`crates/oku-slides-i18n/src/locales/*/`). Toute clé ajoutée l'est dans les 20 langues.
- Commentaires et docs en français, comme le code environnant.
- Commits : Conventional Commits en français, avec le ticket `(t-N)` ; jamais de ligne « Co-Authored-By ». `git add` dans un appel séparé de `git commit`.
- Toute exécution de l'app, du serveur ou de `just e2e` : `OKU_SLIDES_PATHS__DATA` et `OKU_SLIDES_PATHS__DECKS` pointent vers un dossier temporaire.
- Vérification avant commit : `cargo nextest run` sur les crates touchées, `cd ui && bun run test` si l'UI change, `cargo clippy` sans avertissement sur les crates touchées.
- Pas de shim ni de compatibilité ascendante : on met à jour tous les consommateurs.

## Task 1: Rattacher chaque appel d'outil à son tour (t-449)

Ticket : « un report/render tardif d'un tour annulé est rangé dans le tour suivant (`tag_in` lit le tour courant) et supprime son reçu de repli. Passer le tour explicitement à chaque appel MCP, ou porter le tour sur chaque AgentEvent (cas du UserText perdu par Lagged). Aussi : `isQuestion` repose sur le préfixe d'identifiant `ask-` : ajouter un champ typé sur Surface. »

Fichiers de départ : `crates/oku-slides-mcp/src/hub_turns.rs` (`tag_in`, `show_static`, `show_report`, `end_turn`), `crates/oku-slides-mcp/src/hub.rs`, `crates/oku-slides-mcp/src/surface_tools.rs`, `crates/oku-slides-agent/src/turns.rs`, `crates/oku-slides-agent/src/events.rs`, `crates/oku-slides-agent/src/tab_entry.rs`, `crates/oku-slides-session/src/agent/` (type `Surface`), `ui/src/a2ui/` (recherche de `isQuestion` / `ask-`).

Attendu :
1. Un appel d'outil `report`/`render` (et toute surface du fil) est rangé dans le tour **pendant lequel il a été émis**, pas dans le tour courant au moment où il arrive. Un appel tardif d'un tour annulé/clos est refusé (ou rangé dans son propre tour s'il est encore dans le fil) et ne supprime jamais le reçu (de repli ou non) d'un autre tour. Choisir le mécanisme (jeton par tour transmis à l'appel MCP, ou tour porté par chaque `AgentEvent`) d'après le code ; documenter le choix dans l'ARCHITECTURE.md concerné.
2. Le cas `UserText` perdu par `Lagged` (broadcast) ne doit plus faire mal ranger les surfaces.
3. `Surface` gagne un champ typé indiquant qu'il s'agit d'une question (remplace le test du préfixe `ask-` côté UI et côté Rust) ; mettre à jour sérialisation, fixtures (`ui/src/a2ui/fixtures/agent-surfaces.json`, snapshots insta) et tous les consommateurs.
4. Tests de régression : un report tardif d'un tour clos n'atterrit pas dans le tour suivant et n'efface pas son reçu de repli ; idem `render` ; `isQuestion` repose sur le champ.

## Task 2: Miniatures de reçu légères (t-450)

Ticket : « chaque reçu porte le SVG complet de la slide (images base64), gardé deux fois (surface + version sans actions) sur 50 tours, dans `hub.open`, l'état UI et chaque resynchronisation. Miniature réduite, ou référence par slide/empreinte. »

Fichiers de départ : `crates/oku-slides-session/src/agent/receipt.rs`, `crates/oku-slides-mcp/src/surface_tools.rs`, `crates/oku-slides-mcp/src/hub.rs`, composant miniature du catalogue `ui/src/a2ui/catalog.tsx` / `turnComponents.tsx`.

Attendu : le reçu ne transporte plus le SVG complet. Il porte une référence (index/id de slide + empreinte de version, ou URL servie par le serveur/Tauri) que l'UI résout vers une miniature, ou une miniature réduite bornée en taille. Préférer la référence si l'UI sait déjà rendre une miniature de slide par id (chercher l'existant avant de créer, G1). La miniature d'un reçu ancien doit rester juste (montrer l'état de la slide au moment du reçu, ou documenter explicitement le choix contraire dans l'ARCHITECTURE.md). Test : la taille sérialisée d'un reçu ne dépend plus des images de la slide ; snapshots mis à jour ; rendu UI testé.

## Task 3: Fil de l'agent : finitions dock et accessibilité (t-451)

Ticket : « touches rapides 1-4/Entrée pour une question du dock ; aria-live persistant (`aria-relevant=additions`) au lieu du dernier tour ; hauteur max du dock relative au panneau entier ; text-align des pastilles multi-lignes ; plusieurs `ask_choice` parallèles au dock ; l'éviction ferme des réglages encore vivants (documenter) ; vérification visuelle des fiches, tiroir, modale et du thème Verre ; ouverture du détail « Pourquoi » perdue à l'expiration des actions. »

Fichiers de départ : `ui/src/a2ui/` (dock, `SurfaceView.tsx`, `catalog.tsx`, `a2ui.css`, `turnComponents.tsx`), `ui/src/App.tsx`, `crates/oku-slides-mcp/src/hub.rs` (ask_choice / dock).

Attendu, chaque point avec un test :
1. Question au dock : touches `1`–`4` choisissent l'option correspondante, `Entrée` valide l'option par défaut/focalisée — seulement quand le focus n'est pas dans un champ texte.
2. Une région `aria-live="polite"` `aria-relevant="additions"` persistante qui annonce les ajouts du fil, au lieu de ré-annoncer le dernier tour.
3. Hauteur max du dock exprimée relativement au panneau entier (pas au viewport).
4. Pastilles (chips) multi-lignes : `text-align` correct (début de ligne, respectant RTL).
5. Plusieurs `ask_choice` en parallèle au dock : toutes visibles/empilées et répondables indépendamment (Rust + UI).
6. Éviction d'un tour qui ferme des réglages encore vivants : documenter le comportement dans l'ARCHITECTURE.md (et le doc-commentaire de `close_turn`).
7. L'état ouvert du détail « Pourquoi » survit au remplacement de la surface par sa version sans actions (expiration).
8. Vérification visuelle (fiches, tiroir, modale, thème Verre, clair/sombre) : via le navigateur avec données isolées ; corriger ce qui est cassé ; consigner le résultat dans le rapport avec captures dans le dossier de travail.

## Task 4: Fil de l'agent : tests manquants (t-452)

Ticket : « aller-retour Rust de TimelineStep/Receipt ; pluriels hors fr ; turn_patches (non-objet, plusieurs clés, inchangé) ; first_line max=1/blancs ; report_surface au niveau session (hex, titre long, bidi) ; reported vidé par close_turn/clear_thread ; ré-épinglage à un nouveau tour ; légende d'état ; e2e 1 et 4 sur le texte du reçu ; boucle sans délai dans le test des questions fermées ; facts toutes blanches. »

Écrire chacun de ces tests (après les tâches 1–3, contre le code à jour). Fichiers : `crates/oku-slides-session/src/agent/*_tests.rs`, `crates/oku-slides-mcp/tests/surfaces.rs`, `crates/oku-slides-agent/src/turns_tests.rs`, `crates/oku-slides-i18n/src/lib_tests.rs`, `ui/e2e/agent.cjs`, `ui/src/a2ui/*.test.tsx`. « boucle sans délai » : le test des questions fermées attend en boucle active ; remplacer par une attente bornée avec délai/notification. Si un test révèle un bug, le corriger (test d'abord) dans le même commit et le signaler. Lancer `just e2e` (données isolées) si l'e2e change.

## Task 5: Relecture native des traductions de l'agent (t-448)

Ticket : « `agent-instructions` et clés `agent-turn-*` traduites sans relecture native dans 18 langues ; espacement des compteurs en ko/ja/zh ; terme « agent » incohérent en zh-Hans/zh-Hant/vi. »

Fichiers : `crates/oku-slides-i18n/src/locales/*/agent.ftl` et `ui.ftl` (clés ajoutées/modifiées par 39d34c50, plus celles des tâches 1–4). Relire chaque langue comme un relecteur natif : sens fidèle au français (référence) et à l'anglais, registre de l'interface existante, pluriels Fluent corrects pour la langue, pas d'espace entre nombre et compteur en ja/zh (et usage coréen correct), terme unique pour « agent » dans zh-Hans/zh-Hant/vi aligné sur le reste de l'interface de ces langues. Les tests i18n existants passent ; ajouter un test qui vérifie l'absence d'espace nombre-compteur dans les clés concernées en ja/zh si faisable simplement.
