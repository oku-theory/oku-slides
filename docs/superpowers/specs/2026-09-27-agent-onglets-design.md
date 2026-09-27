# Agent vocal dans l'espace de travail (onglets, fenêtres, markdown)

Complément de `2026-09-26-agent-vocal-design.md` (t-224, sous t-89). La
branche de l'agent suppose un seul document (`SharedSession =
Arc<Mutex<Session>>`) ; `main` a depuis des fenêtres et des onglets, une
session par onglet (`Workspace`, t-142, t-158), et un texte d'édition en
markdown lu par l'interface seule (t-180). Ce document dit comment l'agent
s'y branche ; le reste de la spec d'origine vaut toujours.

## Décisions (choisies le 2026-09-27)

1. **Un sous-processus ACP pour l'app, un agent par onglet.** Chaque onglet a
   sa session ACP (`session/new`, créée à son premier prompt) et ses tours ;
   les tours de deux onglets tournent en même temps sur la même connexion.
   « Nouvelle conversation » ne touche que l'onglet courant.
2. **Un serveur d'outils MCP par onglet.** Lancé avec la session ACP de
   l'onglet (port libre, jeton propre), lié à cet onglet : ses outils ne
   voient que ce document. Il s'arrête quand l'onglet se ferme.
3. **Un tour par onglet.** Dans un onglet, un prompt pendant un tour
   l'annule puis repart (comme aujourd'hui) ; les autres onglets ne sont
   pas touchés.
4. **Le fil de chaque onglet est gardé par l'hôte** (200 entrées du fil,
   morceaux fusionnés ; état courant rejoué) et rejoué par `resync` : l'interface d'un onglet est remontée
   à chaque changement d'onglet (`<App key={tab}>`) et un onglet peut
   changer de fenêtre (autre webview).
5. **La reformulation passe par le parseur de l'interface.** L'action
   `rewrite` d'un élément porte `paragraphs` (lus par `textToParagraphs`,
   `deck.ts`) ; le serveur n'a plus de lecteur de texte
   (`text_to_paragraphs` disparaît). Un champ (`@title`…) garde `text`.

## Session : accès à un onglet

`Workspace::with_session(ws, tab, f: impl FnOnce(&mut Session) -> R) ->
Result<R>` (nouveau, public) : clone l'`Arc` du slot sous le verrou de
l'espace de travail, le relâche, verrouille le slot, le réveille
(`Slot::ready`), appelle `f`. Même ordre des verrous que `request` ; un
onglet fermé donne `UnknownTab`. C'est la seule porte de l'agent vers une
session.

## MCP (`oku-slides-mcp`)

- `SharedSession` devient `TabSession` : un onglet et un accès à sa
  session. Trait `Sessions` (« Implement this trait to give the agent a
  tab's session ») : `fn with(&self, tab: &str, f: &mut dyn FnMut(&mut
  Session)) -> Result<(), SessionError>` ; les hôtes l'implémentent sur
  `Mutex<Workspace>` (via `with_session`), les tests sur une session seule.
- `SurfaceHub`, `Tools` et `ToolServer` restent **par onglet** et gardent
  leur logique ; seul leur accès à la session change (`TabSession`).

## Agent (`oku-slides-agent`)

- `Stack` : la connexion ACP partagée (sous-processus lancé au premier
  prompt de n'importe quel onglet, relancé après un arrêt) et une table
  `onglet → TabAgent { hub, serveur d'outils, session ACP, fil, acteur }`,
  créée au premier prompt de l'onglet.
- Connexion : les notifications et demandes de permission sont routées par
  `SessionId` vers le `TabAgent` ; `cwd` de `session/new` : dossier du
  document de l'onglet (sinon le repli). Un arrêt du sous-processus termine
  les tours de tous les onglets (erreur dans chaque fil concerné) et oublie
  leurs sessions ACP.
- Tour : `begin_turn`, `agent_context`, `end_turn` via `Sessions::with` sur
  l'onglet. Onglet fermé pendant un tour : le tour est annulé.
- `AgentCommand` : `prompt`, `cancel`, `new_conversation`, `surface_action`,
  `resync` portent `tab`. Évènements relayés dans une enveloppe
  `{ tab, event }` (`mcp` et `agent`) ; `resync{tab}` renvoie
  `Thread{entries}` puis `Surfaces{list}`.
- Cycle de vie : `tab_closed(tab)` (tour annulé, session ACP oubliée,
  serveur arrêté, questions relâchées ; une commande tardive pour cet
  onglet est ensuite ignorée, les identifiants ne revenant jamais) ;
  `after_change(tab)` (élagage des surfaces). Un autre document s'ouvre dans
  un nouvel onglet : pas de « document changé » côté hôte. Les outils
  `open` / `new_deck` de l'agent remplacent le deck de son onglet (la
  conversation continue, les post-its sont élagués) ; `open` refuse un
  fichier déjà ouvert dans un autre onglet (`Sessions::open_elsewhere`).
  Suite (tâche t-242) : `open` de l'agent devrait ouvrir un nouvel onglet,
  et `save {path}` vers un fichier ouvert ailleurs être refusé.
- Démarrage borné : l'agent doit être prêt (poignée de main faite) dans
  `READY_WITHIN` (120 s, `npx` peut télécharger) ; au-delà il est tué et
  le prompt reçoit `agent-stopped`.

## Hôtes

- Tauri : `AppState` garde `stack: Stack` ; la commande `request`
  appelle `after_change(tab)` ; `workspace` et `open_files` appellent
  `tab_closed` pour chaque onglet disparu de la vue ; commande `agent`
  (hors du fil principal).
  Évènements `oku-slides:mcp` / `oku-slides:agent` émis à toutes les
  fenêtres (enveloppe avec l'onglet), comme `workspace`.
- `oku-slides serve` : le serveur de `main` est une boucle synchrone sans
  flux ; il gagne ceux de la branche : `/api/events` (SSE, fils dédiés) et
  `/api/agent`, à côté de `/api`, `/api/workspace`, `same_origin` et
  `MAX_BODY` de `main`, avec la garde anti-CSRF de la branche.

## Interface

- `agent.*` (`api.ts`) prend l'onglet ; `tabApi(tab)` en fournit un lié.
- `App` (un onglet) : filtre les enveloppes sur son onglet, `resync` au
  montage : fil et surfaces rechargés depuis l'hôte.
- Dictée ⌘⇧D : prompt pour l'onglet actif ; `agentBusy` suit le tour de
  cet onglet.
- Reformulation : `sendAction` ajoute `paragraphs = textToParagraphs(text)`
  quand l'action est `rewrite` sur un élément (pas `original`).

## Tests

- Session : `with_session` (onglet dormant réveillé, onglet fermé →
  `UnknownTab`, verrou de l'espace de travail relâché pendant `f`).
- MCP : tests existants sur `TabSession` ; reformulation avec paragraphes
  (gras gardé) ; régression t-222.
- Agent (faux agent) : deux onglets, deux sessions ACP sur une connexion,
  tours simultanés, notifications et permissions routées ; `tab_closed`
  pendant un tour ; arrêt du sous-processus pendant deux tours ; fil rejoué
  par `resync`.
- Interface : filtrage par onglet, `sendAction` rewrite → paragraphes.
- Bout à bout : le scénario `agent.cjs` dans un onglet, puis un second
  onglet au fil vide dont le prompt tourne pendant que le premier attend
  une réponse.
