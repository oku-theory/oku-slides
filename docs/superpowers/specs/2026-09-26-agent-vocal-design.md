# Agent vocal : ACP, MCP et surfaces A2UI — design

Date : 2026-09-26 · Tâche : t-89

## Objectif

L'utilisateur parle à un agent : ⌘⇧D dicte une consigne (Whisper local),
envoyée par ACP à un agent qui agit sur Oku Slides uniquement par des outils
MCP qu'Oku Slides lui injecte. Ces outils couvrent toutes les modifications
du deck, les réglages et les actions de l'application. Quand une décision
revient à l'utilisateur, l'agent répond par une surface A2UI : choix,
confirmation, reformulation, paramètres, variantes de design.

Hors périmètre : relais MCP en stdio pour les agents sans HTTP ; A2UI brut
écrit par l'agent ; conversation enregistrée dans le document ; synthèse
vocale des réponses ; `undo` / `redo` pour l'agent.

## Décisions

| # | Décision |
| --- | --- |
| 1 | Un seul micro, un brouillon à deux sorties (révisé le 2026-09-27 ; d'abord deux modes, ⌘D texte et ⌘⇧D agent, mais l'arrêt sur silence envoyait des consignes partielles). ⌘D écrit au curseur pendant l'édition d'un texte, sinon dans le brouillon ; ⌘⇧D écrit toujours dans le brouillon. Le brouillon s'**insère** tel quel (local, hors ligne) ou s'**envoie à l'agent**. |
| 2 | N'importe quel agent ACP, déclaré dans `[agent]` (`command`, `args`) ; `claude-code-acp` par défaut. |
| 3 | L'agent n'agit que par les outils MCP d'Oku Slides : le client ACP n'annonce ni `fs` ni `terminal`. |
| 4 | ACP en stdio (Oku Slides lance l'agent) ; MCP en HTTP streamable, dans le processus Oku Slides, sur `127.0.0.1`, jeton par lancement. Un agent sans `mcpCapabilities.http` est refusé avec une erreur claire. |
| 5 | Ce qui s'annule s'applique directement ; un tour de l'agent = une entrée ⌘Z. Ce qui ne s'annule pas demande d'abord confirmation à l'utilisateur : le serveur MCP la demande lui-même avant d'exécuter l'outil ; les `session/request_permission` de l'agent s'affichent dans la même fenêtre. |
| 6 | Deux familles d'outils de surface : `ask_*` bloquants, `show_*` persistants. Une action de surface qui correspond à une `Op` connue s'applique sans passer par l'agent. |
| 7 | Outils de haut niveau : l'agent décrit l'intention, la session compose l'A2UI. Cinq formes : choix, confirmation, reformulation, paramètres, variantes. |
| 8 | Une session ACP par document, non enregistrée ; bouton « Nouvelle conversation ». Contexte automatique joint à chaque prompt. |
| 9 | Interface hybride : choix et reformulations en fiche sur l'élément, variantes dans un tiroir sous la scène, paramètres et fil dans le panneau de droite. |
| 10 | Deux nouveaux crates Rust : `oku-slides-mcp` (`rust-mcp-sdk` + `rust-mcp-axum`) et `oku-slides-agent` (`agent-client-protocol`). `oku-slides-session` reste synchrone. |
| 11 | Un seul outil `apply(ops)`, dont le schéma est celui de `Request::Schema`. `set_setting` limité à une liste blanche. |

## Architecture

```text
UI (React) ──invoke / HTTP──▶ Hôte (src-tauri | oku-slides serve)
   ▲  évènements (Tauri emit | SSE)      │
   │                                     ├─▶ oku-slides-agent ──stdio ACP──▶ agent (sous-processus)
   │                                     │                                     │
   │                                     └─▶ oku-slides-mcp ◀──HTTP MCP────────┘
   │                                               │
   └──────────── surfaces, changements ◀──── oku-slides-session (synchrone)
```

### `oku-slides-mcp` (nouveau)

- Serveur `rust-mcp-axum` (`create_axum_server`) sur `127.0.0.1`, port
  choisi par le système, jeton aléatoire de 256 bits par lancement exigé en
  `Authorization: Bearer`. Sans jeton ou avec un mauvais jeton : 401, rien
  n'est exécuté.
- Tient un `Arc<Mutex<Session>>` partagé avec l'hôte. Un outil prend le
  verrou le temps d'un `Session::handle` ; jamais au travers d'un `.await`.
- Après chaque changement, notifie l'hôte (canal `tokio::sync::broadcast`
  d'évènements `Changed`, `Surface`, `Focus`).
- `ask_*` : crée un `oneshot`, pousse la surface, attend la réponse sans
  tenir le verrou ; `surface_action` de l'hôte complète le `oneshot`.
- Outils non annulables : le serveur demande lui-même confirmation avant
  d'exécuter, par le trait `PermissionGate` fourni par l'hôte ; l'hôte
  l'implémente avec la forme confirmation (même `oneshot` que
  `ask_confirm`). `session/request_permission` va de l'agent au client et ne
  peut pas servir ici.
- Arrêt par le `CancellationToken` de l'hôte.

### `oku-slides-agent` (nouveau)

- Lancement paresseux au premier ⌘⇧D : `[agent] command` / `args`.
  `initialize` avec `clientCapabilities.fs` et `terminal` absents ; vérifie
  `agentCapabilities.mcpCapabilities.http`.
- `session/new` : `cwd` = dossier du document (sinon `paths.data`),
  `mcpServers = [{ type: "http", name: "oku-slides", url, headers:
  [{ name: "Authorization", value: "Bearer <jeton>" }] }]`. Le jeton n'est
  jamais dans l'URL ni dans les journaux.
- Une session par document : `New`, `Open` et « Nouvelle conversation »
  en ouvrent une neuve ; le processus reste vivant.
- Un tour : ouvre le groupe d'annulation « Agent : <début de la consigne> »,
  envoie `session/prompt` avec deux blocs texte (contexte, puis consigne),
  relaie `session/update` (`agent_message_chunk`, `tool_call`,
  `tool_call_update`, `plan`) vers le fil, présente les
  `session/request_permission` de l'agent dans la fenêtre de confirmation
  (options fournies par l'agent), ferme le groupe au
  `stopReason`. Échap ou ⌘⇧D pendant un tour : `session/cancel`.
- Plantage (stdout fermé) : erreur localisée, groupe refermé avec ce qui a
  été appliqué ; le prochain ⌘⇧D relance l'agent.
- Premier prompt de chaque session : instructions de rôle (Fluent) — éditer
  une présentation Oku Slides par les outils, préférer `ask_*` quand la
  décision revient à l'utilisateur, répondre dans sa langue.

### `oku-slides-session` (étendu)

- `History` (dans `oku-slides-core`) : groupe ouvert. `begin_group(label)`,
  `end_group()` ; les `commit` d'un groupe ouvert fusionnent en une seule
  transaction ; un groupe vide ne crée rien. Le groupe se sérialise avec
  l'historique (compatible avec la reprise d'état, spec
  `2026-09-26-resume-state-design.md`).
- Nouveau module `forms.rs` : compose les cinq formes en messages A2UI.
- Variantes : applique les `ops` de chaque variante sur un clone du deck,
  compile, rend le SVG de la slide, jette le clone. Une variante invalide
  fait échouer l'appel entier.
- Différences mot à mot de la reformulation calculées avec `similar`.
- Contexte automatique : `agent_context()` rend le bloc texte ; les actions
  faites sans l'agent (variante retenue, paramètre réglé) s'accumulent dans
  `since_last_turn` et sont vidées après envoi.

### `oku-slides-render` (étendu)

- `render_slide` en PNG avec `typst-render`, à partir du document déjà
  compilé.

### `oku-slides-a2ui` (étendu)

- `CATALOG_ID` = `urn:oku-slides:a2ui:catalog:v2` (remplace
  `urn:echo:a2ui:catalog:panel:v1`, sans alias).
- Nouveaux composants : `Button`, `ChoiceChips`, `Slider`, `ColorField`,
  `Select`, `SlideThumb` (SVG rendu par la session), `DiffText` (segments
  `keep | del | ins`). Décrits en zod dans `ui/src/a2ui`.

### Hôtes

- Nouvelles commandes de l'hôte (hors `Session::handle`, qui ne connaît pas
  l'agent) : `agent_prompt { text }`, `agent_cancel`,
  `agent_new_conversation`, `surface_action { surface, action, context }`.
- `src-tauri` : `AppState` gagne l'agent et le serveur MCP ; les évènements
  partent par `emit` (`session-changed`, `agent-update`, `surface`).
- `oku-slides serve` : même câblage ; évènements par SSE (`/api/events`).

### Réglages

```toml
[agent]
command = "claude-code-acp"
args = []
```

`OKU_SLIDES_AGENT__COMMAND` et `OKU_SLIDES_AGENT__ARGS` les remplacent.

## Outils MCP

### Lecture (libre)

| Outil | Rôle |
| --- | --- |
| `deck_outline` | Slides (id, type, titre, nombre d'éléments), thème, ensemble de modèles |
| `slide(id)` | Slide détaillée, comme `Request::Slide` |
| `render_slide(id)` | Image PNG de la slide |
| `templates` | Ensembles et modèles |
| `settings` | Réglages de l'app (lecture) |

### Modifications annulables (directes, dans le groupe du tour)

| Outil | Rôle |
| --- | --- |
| `apply(ops)` | Toutes les `Op` ; schéma d'entrée = `Request::Schema` |
| `chart_kind`, `chart_palette` | Requêtes de graphique existantes |
| `edit_templates(set)`, `close_templates` | Mode modèles (le deck est mis de côté) |
| `focus(slide, element?)` | Montre une slide ou un élément dans l'interface |

### Non annulables (confirmation d'abord, demandée par le serveur MCP)

| Outil | La confirmation montre |
| --- | --- |
| `save(path?)`, `open(path)`, `new` | Le chemin complet ; les modifications non enregistrées |
| `save_templates(as_new)`, `delete_template_set(set)` | Le nom de l'ensemble |
| `set_setting(key, value)` | Clé, ancienne et nouvelle valeur |

Chemins : absolus, normalisés, sans `..` ; refusés sinon. Liste blanche de
`set_setting` : `i18n.locale`, `stt.model`, `stt.language`, `stt.vad.*`,
`history.limit`. `[paths]` est hors de portée.

### Surfaces

| Outil | Arguments | Où | Retour |
| --- | --- | --- | --- |
| `ask_choice` | `question`, `options[{id, label, hint?}]`, `anchor?{slide, element}` | Fiche sur l'élément, sinon dans le fil | `{choice}` · `{answer_text}` · `{cancelled}` |
| `ask_confirm` | `message`, `danger?` | Fenêtre modale | `{confirmed}` · `{cancelled}` |
| `ask_rewrite` | `slide`, `element`, `proposals[]` (2 à 3) | Fiche sur l'élément, différences surlignées | `{kept: "original" \| index, text}` ; Oku Slides applique la `SetText` retenue |
| `show_params` | `title`, `fields[{label, kind, value, op_template}]` | Post-it du panneau de droite | Rien ; chaque changement applique l'`Op` |
| `show_variants` | `slide`, `variants[{label, ops}]` (2 à 4) | Tiroir sous la scène, miniatures rendues | Rien ; le choix applique les `ops` |

- `kind` : `stepper | slider | color | select | text`.
- `op_template` : une `Op` avec `"$value"` à la place de la valeur ; validé
  à l'appel (rempli avec la valeur initiale, vérifié contre le schéma).
- `ask_*` n'expire pas. Échap, un nouveau tour ou la fin de la conversation
  renvoient `{cancelled: true}`. Un ⌘⇧D pendant qu'une fiche attend renvoie
  `{answer_text}` à cet outil au lieu d'ouvrir un tour.
- `show_*` survit à la fin du tour : un post-it jusqu'à sa fermeture ou la
  suppression de sa slide ; un nouveau `show_variants` remplace le tiroir.
- Une action directe hors d'un tour est sa propre entrée ⌘Z, nommée d'après
  la surface (« Variante : image à gauche »).

## Contexte automatique

Premier bloc de chaque `session/prompt`, court et stable :

```text
<oku-slides-context>
mode: deck | templates(set=…)
locale: fr · theme: papier · template_set: essentiel
slide: s-4 (3/12) kind=content title="Nos objectifs"
selection: element=e-2 field=title editing=false
since_last_turn:
  - variante retenue : « image à gauche » (s-4)
  - paramètre : grille colonnes 12 → 8 (s-4)
</oku-slides-context>
```

Le deck entier n'est jamais envoyé ; l'agent le lit par les outils.

## Interface

- **Micro** : ⌘D texte (sombre), ⌘⇧D agent (couleur d'accent, libellé
  « Agent »).
- **Fil** (panneau de droite) : agent actif et « Nouvelle conversation » en
  tête ; consigne dite ; plan en cases ; une ligne par appel d'outil ; texte
  de l'agent en continu. `aria-live="polite"`. État React, non enregistré.
- **Fiche** (`ask_choice`, `ask_rewrite`) : ancrée sous l'élément, en
  miroir en arabe ; `1`–`3` choisir, `↵` valider, `E` retoucher, Échap
  abandonner.
- **Tiroir** (`show_variants`) : sous la scène ; clic ou `1`–`4` ; Échap
  ferme.
- **Post-it** (`show_params`) : dans le panneau de droite ; ✕ ferme.
- **Fenêtre modale** (`ask_confirm`, permissions) : chemin complet et
  conséquence ; options fournies par l'agent.
- Fiches et fenêtres : `role="dialog"`, focus gardé puis rendu à l'élément.
- Toutes les chaînes par Fluent, 20 langues.

## Erreurs

`AgentError` et `McpError` : Snafu opaques, backtrace par variante,
`Localize` comme `SessionError`. Pas de `.unwrap()` ni `.expect()`.

| Cas | Utilisateur (Fluent) | Agent |
| --- | --- | --- |
| Commande introuvable | « Agent introuvable : `<command>`. Réglage `[agent] command`. » | — |
| Pas de `mcpCapabilities.http` | « Cet agent ne sait pas se connecter aux outils d'Oku Slides. » | — |
| Agent arrêté en cours de tour | « L'agent s'est arrêté. » ; groupe refermé | — |
| Port MCP indisponible | « Les outils de l'agent n'ont pas pu démarrer. » | — |
| `Op` invalide, id inconnu, gabarit invalide | — | `isError: true`, message anglais stable avec le chemin fautif |
| Permission refusée | « ✕ Refusé » dans le fil | `isError: true`, « denied by user » |
| Clé hors liste blanche, chemin refusé | — | `isError: true` |
| Jeton absent ou faux | — | HTTP 401 |

Journaux : `info` lancement et arrêt de l'agent ; `debug` nom et durée de
chaque outil (sans arguments) ; `trace` le flux ACP.

## Tests

TDD ; `_tests.rs` à côté du code ; speculoos ; `insta` pour les instantanés.

- **`oku-slides-core`** : groupe → une entrée ; `undo` du groupe restaure
  tout ; groupe vide → rien ; sérialisation du groupe.
- **`oku-slides-session`** : cinq formes → A2UI (instantanés) ; variantes
  sur clone sans toucher au deck ; variante invalide → échec de l'appel ;
  segments de différences ; bloc de contexte et `since_last_turn`.
- **`oku-slides-render`** : PNG d'une slide (dimensions, non vide).
- **`oku-slides-mcp`** : serveur réel sur `127.0.0.1:0` et client
  `rust-mcp-sdk` dans le test ; liste des outils et schémas (instantané) ;
  401 sans jeton ; `apply` dans le groupe ; permission accordée ou refusée
  (gate simulée) ; `ask_choice` bloque pendant que `Session` reste
  verrouillable ; clé hors liste blanche refusée ; chemin avec `..` refusé.
- **`oku-slides-agent`** : faux agent ACP (`tests/bin/fake_agent.rs`,
  `agent-client-protocol`, scénario JSON) ; `initialize` sans `fs` ni
  `terminal` ; `mcpServers` avec URL et en-tête ; contexte (instantané) ;
  cancel ; plantage → erreur et groupe refermé ; agent sans HTTP refusé.
- **UI (bun test)** : composants du catalogue v2 ; routage fiche / tiroir /
  post-it ; mode du micro ; raccourcis.
- **Bout à bout (`just e2e`)** : `OKU_SLIDES_AGENT__COMMAND` = faux agent,
  `OKU_SLIDES_STT__FAKE_MIC` ; dictée → reformulation → choix → variante →
  ⌘Z annule le tour.
- **Hors intégration continue** : `just agent-smoke` avec le vrai
  `claude-code-acp` sur un deck d'exemple.

## À vérifier en premier

- `rust-mcp-axum` : port `0` accepté et port réel relisible ; contrôle du
  jeton par un `AuthProvider` minimal ou une couche axum.
- `claude-code-acp` annonce bien `mcpCapabilities.http`.
- `typst-render` compatible avec la version de Typst du workspace.
