# oku-slides-mcp

Sert les outils MCP de l'agent d'Oku Slides, en HTTP streamable sur
`127.0.0.1` avec un jeton par lancement.

## Modules

- `error` : `McpError`, opaque, `Localize` → `agent-tools-unavailable`.
- `settings` : `SettingsPort` (lu/écrit par l'hôte), `Applied`, la liste
  blanche des clés écrites par l'agent.
- `hub` : `SurfaceHub` (surfaces ouvertes, identifiants, évènements vers
  l'interface via `McpEvent`). La liste des surfaces ouvertes n'est changée
  que par `emit` (et par `act`, qui en retire une question répondue ou y
  recopie un réglage) ; `resync` la renvoie entière (`McpEvent::Surfaces`),
  remplie par `emit` sous le même verrou.
- `tools` : `Tools`, la liste des outils et leurs schémas d'entrée
  (`nest_schema` adapte JSON Schema à MCP 1.x).
- `read_tools` : les outils de lecture (`deck_outline`, `slide`,
  `render_slide`, `templates`, `settings`).
- `handler` : `ServerHandler` pour `Tools` (liste et appel, journalisés).
- `server` : `ToolServer`, le serveur HTTP (port choisi par le système,
  jeton porteur).

## Points d'extension

- Un nouvel hôte de réglages : implémenter `SettingsPort`.
- Un nouvel outil : une entrée dans `tools::list` et un bras dans
  `Tools::call` (`handler.rs`).

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
