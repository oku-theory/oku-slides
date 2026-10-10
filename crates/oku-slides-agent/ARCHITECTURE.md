# oku-slides-agent

Agent vocal d'Oku Slides : un agent ACP en sous-processus (stdio) pour
l'app, une session ACP par onglet avec ses outils MCP (HTTP local, jeton),
les consignes dictées relayées avec le contexte de l'interface.

## Modules

- `config` : `Launch` (commande, arguments, dossier de repli, `ready_within`), tiré de
  `AgentSettings::active()` (l'agent choisi parmi `[agent.presets]`).
- `error` : `AgentError`, opaque, `Localize` (`agent-not-found`, `-no-http`, `-stopped`, `-failed`).
- `events` : `AgentEvent` (`UserText` ouvre le tour `turn` ; chaque évènement d'un tour porte
  son `turn`), `AgentState`, `PlanItem`, `from_update` (mise à jour ACP → évènement du tour).
- `link` : `Link`, la connexion ACP partagée (sous-processus, poignée de main,
  notifications et permissions routées par lancement et `SessionId`).
- `tab_agent` : `AgentHandle`, l'acteur d'un onglet (`prompt`, `cancel`,
  `new_conversation`) : sa session ACP et ses outils (`ToolsAccess`).
- `picker` : surface du panneau des agents, pure (`picker_surface`, `PICKER_ID`).
- `instructions` : consignes du premier tour, en anglais seul (`instructions/*.md`) ; la langue de
  ce que lit l'utilisateur vient de `interface language` du contexte de chaque tour.
- `turn` : un tour, simultané entre onglets ; `permission` : `PermissionChoice` ;
  `settings_port` : `AppSettings`, les réglages de l'app pour `set_setting`.
- `stack` : `Stack`, table des onglets (1re commande → `tab_closed`, puis commandes ignorées ;
  onglets démarrés et fermés sous un seul verrou) ; `forward` relaie `{tab, event}` ;
  `resync` : état de l'agent puis surfaces ; `shutdown` : fin de l'app, agent ACP tué ;
  `follow` (une autre vue suit l'onglet, `Keep::Replay`) et `set_draft` (brouillon tenu par
  `TabLink`, `AgentEvent::Draft`) ; `picker` (panneau des agents, `AgentEvent::Picker`) et
  `select_agent` (choix de l'agent de l'app, refusé s'il est inconnu, imposé ou non enregistré). Chaque `resync` ou `follow` est poussé à toutes les vues
  de l'onglet : leur compteur de resynchronisations avance aussi ; seules les annonces lues
  du fil hors du tour ouvert sont tenues pour lues, le défilement ne bouge pas.
- `tab_entry` : un onglet ; outils puis agent démarrés dans `rt`, commandes
  en file d'ici là ; relais muets une fois fermé (retard : `changed`, `surfaces`) ;
  fermé, son hub refuse toute question (`SurfaceHub::close`).
- `turns` : `Turns`, réducteur des évènements en surfaces de tour A2UI (`TurnOut`) ; le relais les émet par le hub.
  Chaque évènement ne touche que son tour : un tour dont `UserText` s'est perdu (relais en
  retard, `Lagged`) s'ouvre sans consigne au premier évènement qui le porte ; l'évènement
  tardif d'un tour clos est ignoré.
- `bin/fake_agent` : agent ACP minimal piloté par un scénario JSON (`--script`, `--log`) ; tests seulement.

## Tours

Le hub (`SurfaceHub::begin_turn`) numérote les tours. L'acteur estampille ce qu'il émet
(`UserText`, `TurnEnded`, `Error`) avec le numéro du tour qu'il joue ; `link` estampille
les mises à jour ACP avec le tour ouvert du hub de la session (l'agent envoie les mises à
jour d'un tour avant d'en rendre la réponse, sur la même connexion ; hors tour, ignorées).
Porter le tour sur l'évènement plutôt que le déduire de l'ordre rend le réducteur sûr
face aux pertes du canal `broadcast`. Les appels MCP, eux, prennent leur tour à l'arrivée
(voir `oku-slides-mcp`, « Tour d'un appel d'outil ») : l'agent ne connaît pas nos numéros.

## Cycle de vie

Au repos, l'agent n'est pas lancé. Le premier `prompt` d'un onglet le démarre (un seul pour
l'app ; pas prêt dans `ready_within` : tué) puis ouvre la session de l'onglet. Un arrêt fait échouer les tours de tous les onglets
(`agent-stopped`), referme leurs groupes d'annulation et relâche leurs
questions ; le prompt suivant relance l'agent. Une requête refusée
(`agent-failed`) garde la session. Lâcher le dernier `Link` arrête l'agent ;
`Stack::shutdown` (fin de l'app) le tue et n'en relance plus.
`select_agent` (panneau) écrit `agent.selected` puis `Link::switch` : la connexion vivante
s'arrête (tours → `agent-stopped`), le prompt suivant lance le nouvel agent ; pas de reprise
d'une session de l'autre agent (empreinte différente).
`open` de l'agent ouvre le fichier dans un nouvel onglet (`Sessions::open_tab`),
dont l'agent démarre à sa première commande ; le sien garde son deck et sa
conversation. `save {path}` refuse le fichier d'un autre onglet.

## Sécurité

L'agent n'a ni `fs` ni `terminal` (annoncés à `false`) : il n'agit que par
les outils d'`oku-slides-mcp`, en HTTP local avec un jeton en en-tête
`Authorization`, jamais en URL ni en journal. Un agent sans MCP HTTP
(`agent_capabilities.mcp.http`) est refusé avant toute session.
`set_setting` refuse `agent.*` : l'agent ne choisit pas l'agent.

## Dépendances

`agent-client-protocol` 2.2, `oku-slides-mcp`, `oku-slides-session`.

Une session ACP par portée (`AgentScope`) ; le gardien (`keeper.rs`) relit, écrit et retrouve les conversations ; `Resync` démarre l'onglet.

## Reprise d'une session connue

Au premier prompt d'une portée après le lancement de l'agent, `reopen` rend `Reopened` : reprise
(`session/resume`, sinon `session/load`), non tentée (autre agent, ou aucune des deux capacités)
ou refusée (l'agent ne connaît plus la session). Les deux derniers cas ouvrent une nouvelle
session ; seul le refus montre la carte « mémoire perdue » dans le tour (t-611).
La route d'une session chargée n'est posée qu'après la réponse à `session/load` (ACP rejoue
l'historique avant) ; garde en plus, pour un agent non conforme : tant que son premier
`session/prompt` n'est pas parti (`Link::prompting`), les mises à jour qui rejouent
l'historique (messages, pensées, appels d'outils, plan) sont ignorées, les autres passent (t-612).
