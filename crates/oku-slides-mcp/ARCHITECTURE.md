# oku-slides-mcp

Sert les outils MCP de l'agent d'Oku Slides, en HTTP streamable sur
`127.0.0.1` avec un jeton par lancement.

## Modules

- `error` : `McpError`, opaque, `Localize` → `agent-tools-unavailable`.
- `settings` : `SettingsPort` (hôte), `Applied`, liste blanche des clés écrites par l'agent.
- `hub` : `SurfaceHub` (surfaces ouvertes, identifiants, évènements vers
  l'interface via `McpEvent`). La liste des surfaces ouvertes n'est changée
  que par `emit` (et par `act`, qui en retire une question répondue ou y
  recopie un réglage) ; `resync` la renvoie entière (`McpEvent::Surfaces`),
  remplie par `emit` sous le même verrou. `close` (onglet fermé) relâche les
  questions en attente et rend `Cancelled` à toute question ultérieure.
  Plusieurs questions peuvent attendre ensemble (appels `ask_*` parallèles) : chacune est
  vivante au dock et répond à son identifiant (`act`) ; une consigne en réponse (`answer`)
  nomme aussi la question : celle que l'interface désigne aux touches 1–4 (`quickTarget`), le
  hub ne devine pas (il ne voit pas la slide affichée). Une question entre dans la liste des
  ouvertes sous le verrou qui l'enregistre : une question en attente est toujours affichée.
  Découpé en `hub/asks.rs` (questions en attente, confirmations, permissions) et
  `hub/actions.rs` (actions des surfaces persistantes, `SurfaceAction`, élagage `prune`).
  Une action refusée rend une `HubError` (`hub/error.rs`, `HubErrorKind`) : `Localize` la dit dans
  la langue de l'utilisateur (clés `agent-error-*`), avec des arguments sûrs seulement (nom d'action) ;
  l'hôte l'envoie telle quelle à l'interface, qui l'affiche, et journalise l'erreur complète.
- `hub_turns` : les tours du fil (numéros, tour ouvert `turn`, rangement dans le tour de l'appel,
  gel des questions répondues, éviction, `STATIC_PER_TURN`) ; `McpEvent::SurfacePatch` met à
  jour une surface ouverte.
- `surface_tools` : `report` (fin de tour, reçu ; hors tour : refusé) et `render` (composition A2UI
  libre, validée par `oku-slides-session`) avec les outils `ask_*` et `show_*`.
  `ask_choice` et `ask_rewrite` ancrés vérifient d'abord la portée du tour (`check_anchor_scope` : une portée
  Widget est refusée par `ToolError::AnchorScope`), puis l'adresse contre le deck ; ces contrôles précèdent
  l'affichage de toute surface.
- `tools` : `Tools`, la liste des outils et leurs schémas d'entrée
  (`nest_schema` adapte JSON Schema à MCP 1.x).
- `read_tools` : outils de lecture (`deck_outline`, `slide`, `render_slide`, `templates`, `widgets`, `settings`).
- `widget_tools.rs` : sources, écriture (`widget_touched` en mode modèles), aperçu et essais
  (`set_widget_trial`) des widgets. Un `widget_touched` perdu quand le relais prend du retard n'est
  pas rejoué (seuls `Changed` et `Surfaces` le sont) ; le widget s'ouvre alors au prochain clic.
- `tool_error` : `ToolError` (Snafu), l'erreur de tout outil. `Tools::fail_with` la journalise en
  entier et la dit à l'agent dans la langue de l'utilisateur (clés `agent-error-*`, arguments sûrs
  seulement : nom d'outil, clé de réglage, nom d'action, et pour des arguments invalides le
  reproche de serde sans position, pour que l'agent corrige son appel) ; `Tools::parse`, `with_session`,
  `json_result` et `reply` y passent. `error::say_session` dit une erreur de la session sans chemin
  de fichier : aucune erreur envoyée à l'agent ou à l'interface ne contient de chemin absolu.
- `handler` : `ServerHandler` pour `Tools` (liste et appel, journalisés).
- `server` : `ToolServer`, le serveur HTTP (port choisi par le système,
  jeton porteur).

## Tour d'un appel d'outil

Une surface du fil va dans le tour **pendant lequel son appel est arrivé**, jamais dans celui
qui est ouvert quand elle s'affiche. `Tools::call` lit le tour ouvert (`SurfaceHub::turn`)
avant tout travail et le passe explicitement au hub : `show_static(turn, …)` (`render`),
`show_report(turn, …)`, `ask_then(turn, …)` (question sans ancre), `show_params(turn, …)`.
Le hub n'accepte une surface d'outil que si `turn` est encore le tour ouvert : l'appel tardif
d'un tour annulé ou clos (resté bloqué sur la session pendant que le tour suivant s'ouvrait)
est refusé, et ne touche jamais au reçu (de repli ou non) d'un autre tour. Seuls le gel d'une
question et le reçu de repli (`end_turn`) visent un tour qui n'est plus ouvert, tant qu'il est
dans le fil. Une question hors fil (`ask`) n'a pas de tour.
`show_params` et une `ask_choice` sans ancre qui arrivent entre deux tours sont toujours acceptés, sans tour (comportement inchangé).

Choix du mécanisme : un jeton de tour que l'agent renverrait à chaque appel (argument d'outil,
ou une session MCP par tour) dépendrait de l'agent et alourdirait chaque schéma ; le serveur
estampille donc l'appel à son arrivée. Reste une fenêtre étroite : une requête envoyée par
l'agent avant la fin de son tour mais reçue après l'ouverture du suivant (en pratique, l'agent
envoie ses appels avant de rendre la réponse du tour). Les évènements ACP, eux, portent leur
tour (`oku-slides-agent`, `AgentEvent`).

## Éviction d'un tour

L'éviction se fait **par portée**, dans `begin_turn` : au-delà de `KEPT_TURNS` tours
(`oku-slides-session::agent`) d'une même portée, tours relus compris, les plus anciens de cette
portée sont évincés ; les autres portées gardent les leurs. `close_turn` ferme **toutes** les
surfaces d'un tour évincé, vivantes comprises. L'éviction du réducteur des tours de l'agent
(`TurnOut::Evict`, même règle par portée) rappelle `close_turn` sur un tour déjà fermé : redondant
mais sans effet (idempotent). Des réglages (`show_params`) restés au dock
depuis ce tour disparaissent donc sans être figés (leur tour n'est plus là pour les recevoir) ;
une action tardive sur eux est refusée. C'est voulu : un réglage n'appartient qu'à son tour, et
ce cas suppose `KEPT_TURNS` tours sans nouveaux réglages ni fermeture. Une question en attente
d'un tour évincé n'existe pas en pratique : chaque nouveau tour relâche les questions (`cancel_asks`).

## Points d'extension

- Un nouvel hôte de réglages : implémenter `SettingsPort`.
- Un nouvel outil : une entrée dans `tools::list` et un bras dans
  `Tools::call` (`handler.rs`) ; ses erreurs passent par `ToolError` et `Tools::fail_with`
  (une variante, sa clé Fluent dans les 20 langues).

## Dépendances

`oku-slides-session` pour la session éditée : `Tools` et `SurfaceHub` la
reçoivent sous la forme d'une `TabSession` (un onglet, fourni par l'hôte) ;
`rust-mcp-sdk` et
`rust-mcp-axum` en 1.x, car cette version du protocole (`2025-11-25`) est
celle que les clients des agents embarquent encore, alors que la 2.0 du SDK
n'accepte que le protocole `2026-07-28`. La migration vers 2.x restera
locale à `server.rs`, `handler.rs` et `tools.rs`.

## Sécurité

Le serveur n'écoute que sur `127.0.0.1`, sur un port choisi par le système.
Chaque lancement tire un jeton aléatoire, exigé en en-tête `Authorization`
(`Bearer <jeton>`) ; il n'est jamais journalisé.

Le hub range chaque tour dans sa portée et n'envoie que le fil vu (`hub_scopes.rs`) ; il ne touche pas à la session.
