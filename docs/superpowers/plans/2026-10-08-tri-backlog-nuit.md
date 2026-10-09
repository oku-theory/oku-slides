---
task: t-620
---

# Tri et traitement du backlog — nuit du 2026-10-08

72 tickets ouverts dans `slider` (hors t-620 → t-629, créés pour ce plan). Triage par 5 agents
contre `main` @ ff331aee, plan relu par l'équipe commune (8 relecteurs). Les plages « → » sont
inclusives.

Règles de la nuit (choix de l'utilisateur) : chaque lot dans son worktree, TDD, revue, puis squash
sur `main` **sans push**. Si le hook de revue bloque le squash : exemption temporaire
`oku.review=deferred` sur main pour ce seul commit, retirée aussitôt. Avant chaque commit sur main :
index vide des autres sessions, `git diff --cached --stat`, puis comparer l'arbre à la branche.
e2e : jamais `just e2e` brut — port libre et `OKU_SLIDES_PATHS__DATA/DECKS` dans un dossier temporaire.

## Hors périmètre (autres sessions en cours)

- **t-598**, t-608 : session « Perspective slides » (t-606, t-607 déjà clos).
- **t-611, t-612, t-613, t-614, t-615** : suites de t-598, dépendent de son code.
- **t-589**, t-617, t-618, t-619 : session « Widgets tableau » (t-616 déjà clos).

## Étape 0 — Clore les tickets obsolètes (11)

Chaque clôture reçoit un commentaire avec la preuve.

| Ticket | Preuve |
|---|---|
| t-263 | Corrigé par 2576f61d (un mov par commit), t-265…268 faits ; t-269 reste seul (voir « en attente ») |
| t-188 | Épique CRDT livrée (59f0d5e, 8346fd85, 4dc17098), sous-tâches toutes closes |
| t-190 | serve.rs réécrit sur axum (b5126dda, t-419) : `MAX_BODY`, 413 ; tests `cli.rs` 344 et 381 |
| t-191 | sse.rs : canal borné 256, `try_send`, 8 clients max ; `sse_tests.rs` |
| t-192 | `stays_inside` n'accepte que `Component::Normal` ; test `#[cfg(windows)]` jamais exécuté hors release : non vérifié sous Windows, noms réservés (`CON`) notés |
| t-610 | `guard::check` en tête de `handle` pour toute route ; `guard_tests.rs` |
| t-496 | Doublon de t-523 (2490a5f3) |
| t-209 | Inspection du code : région `aria-live` unique dans AgentThread.tsx |
| t-212 | Inspection du code : `thread.ts` supprimé, fil en surfaces, ajout incrémental |
| t-221 | Inspection du code : SurfaceView ne nourrit que les nouveaux messages (`prolongs`) |
| t-194 | `palette.rs` vérifie l'hexadécimal (148d13a6, t-554) |

## Lots de la nuit (ordre d'exécution)

### Lot A — Markdown d'édition (t-622)

t-161 (ajouter le cas `**a***b*` dans `fixtures/markdown.json`, test rouge d'abord, clore en fin de
lot), t-214, t-215, t-216, t-127.
Vérification : `fixtures/markdown.json` partagé et `ALPHABET` de `inline_tests.rs` mis à jour ;
suites Rust (`oku-slides-session`) et TS (`markdown.test.ts`, `textEdits.test.ts`) vertes.

### Lot B — Outillage et sécurité (t-623)

- t-189 : ADR-0003 « règle d'origine des serveurs locaux », guard.rs y renvoie.
- t-193 (durcissement seulement) : corriger le commentaire `[serve]` de `config.default.toml`
  (le réseau local n'est pas refusé) et avertir au démarrage pour toute adresse hors loopback.
  t-193 reste ouvert pour la décision « jeton ».
- t-197 : `overrides` dompurify `^3.4.16` dans `ui/package.json`, lock régénéré ; source-map-js
  seulement si `bun audit` cite un avis ; `bun audit` propre en preuve.
- t-218 : `.gitattributes` `fixtures/**/*.json text eol=lf`.
- t-324 : la recette `e2e` crée un dossier temporaire pour DATA/DECKS et le supprime dans le `trap`.
- t-446 : boucle d'accept hyper-util avec délai de lecture des en-têtes et sémaphore de connexions
  (dépendances déclarées dans le workspace) ; tests 413 et SIGTERM durcis ; le test
  `save_with_embed_fonts` instable est diagnostiqué (correctif ou ticket dédié).
- t-599 (corrections e2e seulement) : fields masque les paramètres avant de mesurer, target-panel
  attend « Paramètres de X ». La mise en page est réglée hors du lot (4f8cef00, t-599 clos) : la
  slide se pose en haut de sa boîte, la carte des paramètres change dessous sans la déplacer, les
  flèches restent à sa mi-hauteur (ancrage CSS) ; e2e editor (et toolbar, t-636) réparées. Suivis :
  fenêtre basse t-647, WKWebView t-648.
Vérification : tests `oku-slides-cli`, e2e isolée sur port libre.

### Lot C — Rendu Typst (t-624)

t-325, t-609. Vérification : test de rendu par cas (couverture sans texte ; tableau suivi d'un bloc).

### Lot H — Packs (t-629)

- t-355, partie CLI/registre : `report_issues` sonde le pack du deck, `beside_deck` vérifie
  `manifest.id`. t-355 est réécrit au reste de sa portée et reste ouvert.
- t-524, point 2 : `PackManifest::from_files(..).ok()` ne perd plus les cas en silence
  (`templates_mode.rs` 213, 380). t-524 est réécrit aux points 1, 3, 4, 5 et reste ouvert.
Vérification : test de régression par correctif dans `tests/regressions/`.
Avant de commencer : vérifier que la branche t-589 ne touche plus `cli/tests/cli.rs` ni `pack_broken.rs`.

### Lot E — Session et i18n (t-626)

t-490 (`resolve` sans Localizer complet), t-491 (découper `workspace.rs`, 969 lignes), t-466
(classificateurs thaïs, correctif appliqué puis ticket laissé ouvert pour relecture native).
Vérification : tests `oku-slides-i18n` et `oku-slides-session`, `workspace.rs` < 700 lignes.

### Lot D — UI hors App.tsx (t-625)

t-219, t-220, t-464, t-546. Vérification : tests de rendu bun ; e2e isolée.

### Lot F — Processus de rendu (t-627)

- t-344, restes : supprimer `WireError::RegistrySetup` mort, `try_from` pour `max_frame_mib`,
  allers-retours Pdf/NoPage/Image, test d'environnement MEMORY_MIB/MAX_FRAME_MIB, test EOF dans le
  préfixe, message PackDisabled redondant. Clos si tout est fait.
- t-342 : renderer créé au premier rendu.
- t-341, seulement les petits points : stdout du worker redirigé, limites de trame par type,
  `phys_footprint` sous macOS (libc, `proc_pid_rusage`, SAFETY + test). Pas de changement de
  protocole cette nuit. t-341 est réécrit au reste (retrait de polices, Hello, Windows) et reste ouvert.
Vérification : tests `oku-slides-render-worker` et `oku-slides-render`.

## Après la fusion de t-598 (touchent App.tsx, hub.rs, mcp, agent/, forms.rs, resume)

- **Lot G — MCP en langue de l'utilisateur (t-628)** : t-403 en entier (outils et hub.rs, tests
  qui comparent du texte anglais), t-463.
- t-462, t-208, t-210, t-465, t-326, t-489, t-195, t-377 (renommer en `useKeptUi`),
  t-211 (masquer le micro de l'agent quand aucun agent n'est disponible ; brouillon perdu si le
  prompt échoue).

## En attente d'une décision ou d'une action de l'utilisateur

| Ticket | Question |
|---|---|
| t-269 | Feu vert pour ouvrir l'issue sur loro-dev/loro |
| t-223 | Issue amont ou fork de Loro (un snapshot piégé fait tomber l'app) |
| t-130 | Quelle édition gagne quand l'agent écrit dans le champ ouvert |
| t-162 | Coller du markdown : garder la mise en forme ou l'échapper |
| t-364 | Raccourcis d'affichage actifs pendant l'édition ? (après t-598) |
| t-363 | « fantômes » ou « repères » dans les 20 langues |
| t-193 | Jeton d'accès si le serveur sort de la machine (durcissement fait au lot B) |
| t-495 | Listes imbriquées : conception de la saisie |
| t-272 | Quasi-rendu : épique, spec à écrire |
| t-417 | Annuler un tour : conception (après t-598) |
| t-179, t-213 | Vérifications manuelles (Windows/Wayland, app Tauri) |
| t-343 | Relecture native des traductions du rendu borné |

Laissé au backlog sans action : t-217 (optimisation sans impact mesurable).
