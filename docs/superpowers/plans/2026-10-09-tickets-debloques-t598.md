---
task: t-638
---

# Tickets débloqués par t-598 et lot G — 2026-10-09

Suite du tri de la nuit (`2026-10-08-tri-backlog-nuit.md`). t-598 est sur main (9518cbb2) ; les tickets mis
de côté parce qu'ils touchaient App.tsx, hub.rs, la session ou l'agent sont revérifiés contre main @ aa0ddbb0 :
tous s'appliquent encore. Plan relu par l'équipe commune (8 relecteurs) et corrigé.

Décisions de l'utilisateur (2026-10-09) :
- t-463 : erreurs du hub traduites en Rust via Fluent (même voie que t-403).
- t-611 : la carte « mémoire perdue » seulement quand une reprise a été tentée et refusée.
- t-326 : adressage {objet, champ} sans compatibilité avec « @param ».
- t-489 : Mode::Help sans liste blanche de requêtes ; les gardes actuelles suffisent.

Hors de ce plan : t-417 (annuler un tour, fonctionnalité à concevoir), t-364 (décision en attente).

## Règles

- Un worktree et un `CARGO_TARGET_DIR` privé par lot ; TDD ; une revue par lot.
- Bug corrigé = test de régression qui échoue avant le correctif (`tests/regressions/issue_<N>_*.rs` côté
  Rust, test bun nommé côté UI).
- Déplacement pur = son propre commit, aucun test modifié dans ce commit, suites vertes avant et après.
- Aucune nouvelle dépendance attendue (Cargo.toml, package.json) : toute modification de manifeste est signalée.
- `ui/src/styles.css` et `ui/e2e/layout.cjs` sont en cours de modification par une autre session : ne pas y toucher.
- Ordre : vague 1 = I, J, L en parallèle ; vague 2 = M puis K (tous deux touchent `ui/e2e/agent.cjs`) ;
  vague 3 = N (touche forms.rs, agent/mod.rs, tools.rs, hub et UI : après tous les autres).
- Fin : intégration de tous les lots sur une branche, vérification complète (fmt, clippy, nextest, bun test,
  typecheck, e2e isolée par spec sur un dossier neuf), revue finale, squash sur main sans push.

## Lot I — Agent dans l'UI (App.tsx)

App.tsx fait 989 lignes (limite 1000).
- t-208 d'abord, puis t-462 (partie UI) dans la même structure : l'état des surfaces de l'agent (réducteur des
  surfaces, abonnement hôte, resyncs, routage) passe dans un **composant conteneur** `ui/src/agent/AgentPanel.tsx`
  (pas un hook appelé par App, qui garderait les re-rendus dans App) ; App ne reçoit que ce qu'il route (busy,
  promptAgent, focus). Le reste du câblage de l'agent d'App.tsx (≈l.365-457) suit dans ce conteneur ou dans
  `ui/src/agent/useAgentPanel.ts` ; les aides de lecture A2UI de quick.ts (≈l.55-89) vont dans
  `ui/src/agent/a2uiModel.ts`. Critère t-208 : un morceau de réponse ne re-rend ni App ni Stage/SlidePile
  (test : compteur de rendus de Stage mémoïsé inchangé après un `surface_patch`). App.tsx < 850 lignes.
- t-210 : un focus de l'agent reçu pendant l'édition d'un texte est **différé** : gardé (seul le dernier compte)
  et appliqué quand l'édition se termine (validation ou annulation) ; identifiants de slide ou d'élément
  inconnus ignorés ; `editing` lu par une ref. Tests : focus pendant l'édition appliqué à la fin, id inconnu ignoré.
- t-211 : le contrôle concerné aujourd'hui est « Envoyer » de DictationDraft (le micro de l'agent n'apparaît
  plus que comme bouton stop) : désactivé quand aucun agent n'est disponible (`ready` de useAgentScope) ;
  `promptAgent` renvoie le succès et le brouillon dicté n'est fermé que si le prompt réussit. Tests : envoi
  désactivé sans agent, brouillon gardé quand le prompt échoue, fermé quand il réussit.
- t-377 : renommer `useViewOptions` en `useKeptUi` (le hook garde l'état d'interface, slide courante comprise ;
  `save` prend déjà un objet UiState). Imports et tests mis à jour.
Vérification : bun test, typecheck, e2e isolée.

## Lot J — Hub et MCP dans la langue de l'utilisateur (lot G, t-628)

- t-462 (partie hub), déplacement pur d'abord : hub.rs (596 lignes) → demandes en attente dans `hub/asks.rs`,
  actions des surfaces dans `hub/actions.rs` ; hub.rs < 500 lignes ; les ré-exports de
  `crates/oku-slides-mcp/src/lib.rs` (Answer, Freeze, McpEvent, STATIC_PER_TURN, SurfaceAction, SurfaceHub,
  THREAD_BYTES) gardent leurs chemins.
- t-463 : erreurs du hub (surface inconnue, actions d'un tour passé, action inattendue, et les autres de
  hub.rs) en type d'erreur Snafu localisé par Fluent dans les 20 langues ; l'UI affiche le texte reçu.
- t-403 : toutes les erreurs des outils MCP (tools.rs, surface_tools.rs, guarded_tools.rs, edit_tools.rs,
  read_tools.rs) passent par une aide de localisation commune côté MCP (modèle de widget_tools.rs) ; ne pas
  modifier `one_line` de la session (le lot L touche ces fichiers). Critère : plus aucun appel à `one_line` ni
  de texte anglais en dur dans les réponses d'erreur des outils MCP.
- Sécurité : les messages localisés ne prennent que des arguments sûrs (id de surface, nom d'action), jamais
  une chaîne d'erreur complète ; la chaîne complète reste dans `tracing`. Test : aucune erreur envoyée à
  l'agent ou à l'UI ne contient de chemin absolu.
- Tests : ceux qui comparaient du texte anglais (guarded.rs, edit.rs, surfaces, hub) vérifient la clé ou la
  variante et au moins une langue autre que l'anglais ; test automatique que chaque nouvelle clé existe une
  fois dans les 20 langues (ou test de complétude existant du crate i18n).
Clôture : t-628, t-403, t-463 à la fin du lot ; t-462 clos quand I et J sont livrés.
Vérification : nextest oku-slides-mcp et crates touchés.

## Lot L — Cœur de Session

- t-615 d'abord, déplacement pur : lib.rs (987 lignes) → `handle` dans `request.rs`, chargement / ouverture /
  écriture du deck dans `deck_file.rs`, graphiques (`set_chart_kind`) dans `charts.rs` ; lib.rs < 700 lignes.
- t-489 : `Mode::Help { locale }` remplace le champ `help` à côté de `mode` ; `Pending` du workspace devient
  une énumération `Blank | File | Help` ; pas de liste blanche (décision). Le format des fichiers de reprise ne
  change pas (`SavedMode::Help` sérialisé comme aujourd'hui) ; `TabInfo.help` reste, rempli depuis le mode.
- t-613 : undo/redo comparent les noms de widgets avant et après `sync_widgets` ; un renommage défait ou
  refait (un nom retiré et un ajouté dans le même pack) déplace la conversation de l'agent en sens inverse et
  émet le même `ScopeMove` que `rename_widget` (consommé par keeper.rs). Règle de collision : si le nom de
  destination a déjà une conversation, rien n'est écrasé : la conversation déplacée reste où elle est et un
  diagnostic est journalisé. Le nettoyage de la reprise n'efface plus une conversation ramenée.
  Régression `issue_0613_*` : renommer A→B, discuter sous B, annuler → conversation sous A ; rétablir → sous B ;
  collision → rien d'écrasé ; reprise → conversation gardée.
Vérification : nextest oku-slides-session (et oku-slides-agent pour ScopeMove).

## Lot M — Validation A2UI et e2e (avant K)

- t-195 : côté Rust, là où arrivent les entrées de l'agent (session agent/forms.rs, agent/variants.rs
  `check_field`, et le crate oku-slides-a2ui s'il valide) : options à identifiants uniques (refus, pas de
  dédoublonnage), Slider min < max, plafonds de nombre et de longueur d'options s'ils manquent ; côté TS, zod
  `.refine` min < max et options uniques dans catalog.tsx (SliderApi, listes d'options). Tests négatifs des
  deux côtés.
- t-614 : `ui/e2e/http.cjs` existe déjà et exporte `post` : migrer dessus les copies d'agent.cjs, fields.cjs,
  lists.cjs, lists-dock.cjs, target-panel.cjs.
Vérification : nextest oku-slides-session et oku-slides-mcp (tests ask_choice), bun test, e2e isolée.

## Lot K — Sessions de l'agent (après M)

- t-611 : `reopen` renvoie non tentée / refusée / reprise ; la carte « mémoire perdue » seulement pour un refus
  (décision). Régression `issue_0611_*` : agent sans capacité de reprise → pas de carte ; reprise refusée → carte.
- t-612 : par session reprise (session/load), les mises à jour qui rejouent l'historique (agent_message_chunk,
  user_message_chunk, tool_call, tool_call_update, plan) sont ignorées tant que le premier `session/prompt` de
  cette session n'est pas parti ; les autres (commandes disponibles, mode) passent. Drapeau par session, pas
  « tour ouvert ». Régression `issue_0612_*` : rejeu reçu après la réponse de session/load → rien dans le tour.
- t-465, chaque point du ticket : faits vides filtrés avant `check_len("facts")` (receipt.rs) ; attente fixe de
  `ui/e2e/agent.cjs` remplacée par une attente sur la condition ; tests CSS par texte (styles.test.ts) remplacés
  par des tests de comportement quand c'est possible ; test d'espacement CJK : liste de clés tenue à la main
  remplacée, U+200B détecté ; titre du fixture de receipt_tests.rs ; test du verrou de `ask_with` ; test direct
  qu'une question gelée du fil laisse l'agent occupé.
Vérification : nextest oku-slides-agent, oku-slides-session, oku-slides-mcp ; e2e agent et agent-scopes.

## Lot N — Adressage des champs par l'agent (en dernier)

- t-326 : `{owner, field}` remplace « @param » dans le protocole de l'agent, sans compatibilité. Forme choisie :
  `Anchor` et `AskRewrite` portent `owner` (id d'élément, ou de slide pour un paramètre de slide) et `field`
  optionnel (nom de paramètre). Consommateurs à mettre à jour : session agent/forms.rs (Anchor, AskRewrite,
  schéma JSON), agent/mod.rs (`strip_prefix('@')`), mcp tools.rs (description d'ask_rewrite), ui/src/api.ts
  (anchor), ui/src/agent/rewrite.ts (`isField`), ui/src/Stage.tsx (`startsWith("@")`),
  ui/src/a2ui/fixtures/agent-surfaces.json, crates/oku-slides-agent/examples/smoke.rs, fixtures/agent/e2e.json
  et fake_agent.rs si la forme les touche, et les tests (session tests/agent.rs, forms_tests.rs, mcp
  tests/surfaces.rs). Réutiliser fields.rs / target_params.rs.
- Sécurité : `owner`/`field` reçus de l'agent sont résolus contre le deck actif et la portée du tour ; owner
  inconnu ou hors portée, champ inconnu → refus ; le nom de champ n'est jamais interpolé dans du Typst. Tests
  négatifs (owner étranger, champ inconnu).
- Commit `feat!` avec pied BREAKING CHANGE (protocole de l'agent).
Vérification : nextest des crates touchés, bun test, e2e agent.
