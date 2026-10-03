# Fil de l'agent tout en A2UI — design

Date : 2026-10-03 · Tâche : t-416

## Objectif

Rendre le fil de l'agent nettement plus compact sans perdre en lisibilité,
par un virage massif vers A2UI : le fil n'est plus du Markdown en post-its
mais une suite de surfaces A2UI construites par l'hôte. L'agent conclut
chaque tour par un reçu compact ; ses explications restent accessibles,
repliées. Le style post-it disparaît de toute l'application.

Constat de départ (capture du 2026-10-03) : les messages intermédiaires de
l'agent ont le même poids que la réponse ; chaque post-it coûte
padding, marges, inclinaison, ombre et 12–18 % de largeur ; la police
15/21 est lourde ; la réponse finale est un compte rendu là où une ligne
de conclusion suffit.

Hors périmètre : l'action « Annuler le tour » sur un reçu (t-417).

## Décisions

- **Projection côté hôte (Rust).** L'hôte construit chaque tour comme une
  surface A2UI ; l'interface ne fait que rendre des surfaces. Un seul
  constructeur d'A2UI (Rust), comme le panneau des modèles (`panel.rs`).
- **Composition hybride.** Des outils typés (`report`, `ask_*`,
  `show_*`) pour les cas fréquents, plus un outil libre `render` validé
  côté Rust pour les cas imprévus.
- **Le texte libre de l'agent est rétrogradé en narration** : des notes de
  progression d'une ligne dans la timeline du tour. Il n'est jamais rendu
  comme une réponse, sauf dans le reçu de repli.
- **Surfaces dans leur tour.** Une surface figée s'affiche dans le tour qui
  l'a créée ; une surface vivante (question en attente, réglages) vit dans
  un dock au-dessus du micro, puis se fige dans son tour.
- **Catalogue v3**, rupture assumée (pas de v2 conservée).
- **Fin du style post-it partout** : fil, dock, fiches, tiroir, modale,
  panneau des modèles, brouillon de dictée, `ChartPanel`. Une seule notion
  la remplace : la carte.

## Modèle des tours (session, agent)

Un tour = une consigne, sa progression (outils, plan, narration), son
issue (terminé, annulé, erreur, fin anormale) et les surfaces qu'il a
créées.

- `oku-slides-session/src/agent/turn_view.rs` (pur, à côté de `forms.rs`) :
  `TurnView { n, prompt, steps, narration, state, outcome }` et ses
  constructeurs A2UI : la surface `turn-N` (Prompt, Timeline, notes) et
  le reçu (voir `report`). Pas d'I/O, pas d'état.
- `oku-slides-agent/src/turns.rs` remplace `ThreadLog` : garde les tours
  d'un onglet (au plus 50, clé de config `agent.kept_turns`), applique les
  `AgentEvent`, émet `UpdateComponents` à l'ouverture d'un tour puis
  `UpdateDataModel` pour chaque étape ou chunk (`/steps`, `/narration`,
  `/state`). Le rejeu d'un onglet renvoie les surfaces des tours gardés.
- `SurfaceHub` (mcp) : `begin_turn(n)` est appelé par `turn.rs` au début de
  chaque tour ; toute surface créée ensuite porte `turn: Some(n)`.
- `Surface` gagne `turn: Option<u32>` et `live: bool`. `Place` devient
  `Sheet | Drawer | Modal | Thread` (`Postit` et `Suggestions` supprimés).
- Fin de tour (`turn_ended`) : `turn-N` passe à « terminé », la timeline se
  replie en une ligne (« 14 outils · 2 notes · 38 s »). **Sans `report`
  pendant le tour**, l'hôte crée un reçu de repli : `title` = première
  phrase de la dernière narration, `why` = le reste (Markdown), pas
  d'actions. Sans narration, pas de reçu.

Ordre des dépendances inchangé : `oku-slides-agent` dépend déjà de `-mcp`
et de `-session`.

## Catalogue v3 (a2ui, UI)

`urn:oku-slides:a2ui:catalog:v3`. Miroir Rust (`ComponentKind`) et zod
(`catalog.tsx`) changent dans le même commit.

Conservés (seul le style change) : Column, Row, Text, Button, TextField,
Stepper, Slider, ColorField, Select, ChoiceChips, SlideThumb, DiffText.

Modifiés :
- `Card` : carte sobre, `tone?: neutral | accent | danger`.
- `Text` : variante `note` (une ligne atténuée, ellipse).

Nouveaux :

| Composant | Propriétés | Rôle |
|---|---|---|
| `Prompt` | `text` | Consigne de l'utilisateur, bulle alignée à droite. Hôte seul. |
| `Timeline` | `steps` (lien → `{label, status, count?}[]`), `collapsed` (bool dynamique), `summary` (texte dynamique) | Progression d'un tour ; repliée en une ligne dépliable. Hôte seul. |
| `Receipt` | `title`, `facts[1-4]`, `thumb?` (SVG), `swatch?` (hex), `child?` | Carte de conclusion d'un tour. |
| `Disclosure` | `summary`, `child` | Contenu replié par défaut (« ▸ pourquoi »). |
| `Badge` | `text`, `tone` | Compteur ou statut. |
| `Markdown` | `text` | Texte riche, rendu par `Markdown.tsx` (pas de HTML brut). |

`Prompt` et `Timeline` sont réservés à l'hôte : `agent_allowed(&ComponentKind)`
les refuse dans `render`.

## Outils de l'agent (MCP)

Nouveaux :

- **`report`** — appelé en dernier, au plus une fois par tour (un second
  appel remplace le premier) :
  `{ title, facts[1-4], slide?, swatch?, why?, actions?[0-4]{label, prompt?} }`.
  L'hôte assemble Receipt, SlideThumb de la slide (rendue par l'hôte),
  Disclosure(`why` en Markdown) et la rangée d'actions. Surface figée
  (`live: false`) ; ses actions restent cliquables tant que c'est le
  dernier tour, et envoient `prompt` (ou `label`) comme nouveau tour,
  comme les suggestions aujourd'hui.
- **`render`** — `{ components[], data? }`, catalogue v3 hors `Prompt` et
  `Timeline`. Bornes : 40 composants, profondeur 6. Validation serde +
  `agent_allowed` ; **une erreur est renvoyée à l'agent en résultat
  d'outil**, lisible, pour qu'il corrige. Seule action permise :
  `prompt {text}` (nouveau tour). Surface figée.

Changés :

| Avant | Après |
|---|---|
| `suggest_next` | Supprimé ; remplacé par `report.actions`. |
| `show_params` (`Postit`) | `Thread`, `live` ; un nouveau `show_params` ou la fermeture le fige avec ses dernières valeurs. |
| `ask_choice` sans ancre | `Thread`, `live` tant qu'en attente ; une fois répondu, figé en n'affichant que la réponse ; `cancel_asks` le fige « annulé ». |
| `ask_confirm`, `ask_rewrite`, `show_variants` | Inchangés (modale, fiche, tiroir). |

Instructions (`agent.ftl`, toutes les langues) : terminer chaque tour par
`report` ; le texte libre n'est montré qu'en note de progression d'une
ligne, l'écrire comme tel ; `render` seulement si aucun outil typé ne
convient. Les mentions de `suggest_next` disparaissent.

## Interface : fil et dock (UI)

Panneau : en-tête · fil des tours (défile) · dock (fixe) · micro.

- **Un tour** : surface `turn-N` (Prompt, Timeline), puis ses surfaces
  figées dans l'ordre de création (questions répondues, `render`, reçu en
  dernier).
- **Dock** : les surfaces `live`, au plus deux par construction (un `ask_*`
  bloque l'agent ; `show_params` remplace le précédent). La question est la
  plus proche du micro (réponse vocale possible via `answer_pending`), les
  réglages au-dessus. Hauteur ≤ 45 % du panneau ; au-delà, les réglages se
  replient en une ligne d'en-tête. Pas de défilement interne. Les réglages
  ont un « × » (→ `live: false`) ; une question ne se ferme pas. Vide : 0 px.
- **Vivant → figé** : même `id`, mêmes `messages`, `live` change ; la
  surface rejoint son tour. Transition courte si le tour est visible,
  supprimée sous `prefers-reduced-motion`.
- **Défilement** : suit le bas pendant un tour sauf si l'utilisateur est
  remonté ; une pastille « ↓ tour en cours » ramène en bas.
- `routeSurfaces` : sheet, drawer, modal inchangés ; `Thread` se répartit en
  `dock` et `byTurn: Map<turn, Surface[]>`.
- `AgentThread` devient la liste des tours plus `<AgentDock>` (nouveau).
  `thread.ts`, `threadItems` et le JSX actuel des entrées sont supprimés.
  Les `AgentEvent` Chunk, ToolCall et Plan ne vont plus à l'interface ; ils
  sont supprimés du canal UI s'ils n'ont plus d'autre consommateur.

## Style

Une seule notion : **la carte**, définie dans `ui/src/a2ui/a2ui.css` et
exposée en classe `.card` pour les composants hors A2UI (brouillon de
dictée, `ChartPanel`).

- Jetons dérivés de l'existant : `--a2ui-surface` (`--paper-100`),
  `--a2ui-border` (`--ink` à 12 %, filet 1 px), `--a2ui-accent-bg`
  (`--accent-soft`).
- Carte : `--radius-sm`, padding 8 × 10 px, ni inclinaison ni ombre, pas de
  marge (l'espacement vient du `gap` parent, `--space-1`).
- Typographie : corps 400 14/20 ; notes, timeline, légendes 400 12/16 en
  `--ink-muted` ; titres (`h4`, reçu) 600 14/20.
- `Prompt` et brouillon de dictée : variante consigne, fond `--paper-200`,
  alignés à droite, `max-width: 85 %`. Reçus en pleine largeur.
- Puces d'action : style des suggestions actuelles, `min-height: 28px`.
- Verre (`glass.css`) : dock, fiches et tiroirs flottants seulement ; les
  cartes du fil restent opaques.
- Supprimés : `--postit`, `--postit-you`, `--postit-you-ink`,
  `--app-postit` (trois blocs de thème) ; `.postit*`, `--tilt` ;
  `--shadow-note` et `--edge-light` s'ils n'ont plus d'usage ; les
  inclinaisons `.notes … .postit` ; `.agent-entry .you/.msg`, `.tools`,
  `.tool-list` ; les neutralisations `.agent-sheet-box .postit` et
  suivantes. Les classes du catalogue deviennent `a2ui-card`,
  `a2ui-title`, `a2ui-hint`.

## Erreurs

- `render` invalide : erreur d'outil MCP avec le chemin du composant fautif
  et la règle violée ; rien n'est affiché.
- `report` invalide (bornes) : erreur d'outil, même principe.
- Surface refusée par le zod côté UI (divergence de catalogue) : toast
  existant (`onError`) ; le test de contrat (ci-dessous) doit l'empêcher.
- Tour sans `report` : reçu de repli (voir Modèle des tours).

## Migration

MVP, sans couche de compatibilité :
- catalogue v2 → v3 d'un coup ;
- suppression dans le même changement : `suggest_next`,
  `Place::Postit/Suggestions`, `ThreadLog`, `thread.ts`, `threadItems`, le
  JSX des entrées d'`AgentThread` ;
- fil gardé : en mémoire d'après `thread_log.rs` ; à vérifier en début de
  plan. S'il est persisté, les anciens fils sont effacés, pas convertis.

## Tests

TDD : chaque étape commence par un test rouge.

Rust :
- `turn_view_tests.rs` : snapshots `insta` de `turn-N` (en cours ; terminé
  replié ; annulé ; reçu de repli ; sans narration donc sans reçu) et du
  reçu `report` (complet, minimal).
- `render` (`rstest`) : refus de `Prompt`/`Timeline`, de plus de 40
  composants, d'une profondeur > 6, d'une propriété inconnue, d'une action
  autre que `prompt` ; snapshot du message d'erreur.
- `Place` et `live` : sérialisation.
- `turns_tests.rs` (remplace `thread_log_tests`) : cas existants reportés
  (fusion des chunks, repli des outils, dernier plan, éviction) ; reçu de
  repli à `turn_ended` ; rejeu d'un onglet.
- mcp : `report` et `render` exposés, `suggest_next` absent ;
  `begin_turn` attribue le tour ; `show_params` fige le précédent ; une
  question répondue est figée avec la réponse ; `cancel_asks` la fige
  « annulé ».

UI :
- `catalog.render.test` : chaque nouveau composant (Prompt, Timeline
  repliée/dépliée, Receipt, Disclosure, Badge, Markdown, Text `note`) et le
  refus strict du zod.
- `surfaces.test` : `routeSurfaces` → `dock` et `byTurn` ; question la plus
  proche du micro.
- `AgentThread.render.test` réécrit : ordre des tours, passage vivant →
  figé, dock vide à 0 px.
- `DictationDraft` : carte, variante consigne.
- **Contrat Rust ↔ TS** : `ui/src/a2ui/fixtures/agent-surfaces.json` est
  produit par les tests Rust (`turn_view`, `report`, outils) et validé par
  le catalogue zod v3.
- `i18n-keys.test` : nouvelles clés présentes dans toutes les langues.

Vérification visuelle : la conversation de la capture et une question en
attente dans le dock, en clair et en sombre, Papier et Verre.

## Ordre de livraison

1. Catalogue v3, carte, suppression du post-it partout (Rust, zod,
   `a2ui.css`, brouillon, `ChartPanel`, panneau des modèles).
2. `turn_view` : construction pure des tours et du reçu.
3. `turns.rs` à la place de `ThreadLog` ; hub : `begin_turn`, `turn`,
   `live`.
4. Outils `report` et `render` ; suppression de `suggest_next` ;
   instructions de l'agent.
5. UI : tours, `AgentDock`, `routeSurfaces`.
6. Vérification visuelle et nettoyage final.

L'étape 1 est livrable seule. Les étapes 3 à 5 forment un tout : pas de
fusion avant la fin de l'étape 5.

## Décisions d'implémentation

Précisées par le plan (`docs/superpowers/plans/2026-10-03-agent-a2ui.md`) et sa revue :

- **Canal** : les surfaces `turn-N` passent par le `SurfaceHub` comme toute surface ; la progression part en `McpEvent::SurfacePatch` (modèle de données racine du tour), que `SurfaceView` applique sans reconstruire la surface. Le canal `agent` vers l'interface ne porte plus que `Status`.
- **Ouverture du tour** : `begin_turn` et `UserText` au tout début de `TabAgent::prompt`, avant la connexion : une erreur de démarrage s'affiche dans son tour.
- **Tours gardés** : constante `KEPT_TURNS = 50` (pas de clé de configuration).
- **Actions** : les suites d'un reçu et l'unique action permise dans `render` réutilisent l'évènement existant `suggest {prompt}`. Elles expirent à l'ouverture du tour suivant : le reçu est remplacé par sa version sans actions et le hub refuse une action en retard.
- **`Receipt`** n'a pas d'enfant : les suites sont une `Row` sœur dans la colonne du reçu. Le reçu de repli est construit dans `receipt.rs`.
- **`render`** exige un arbre : identifiants uniques, chaque composant visité une fois (ni partage ni cycle), aucun orphelin, profondeur ≤ 6, 40 composants au plus.
- **Questions figées** : une question annulée n'est figée que si son tour est encore dans le fil et l'onglet ouvert ; sinon elle est fermée.
- **`render`, garde-fous** : pas de bouton `primary`/`danger`, pas de carte `tone: danger`, pas de `SlideThumb` ni de miniature de reçu (rendues par l'hôte) ; un bouton `suggest` envoie exactement son libellé ; 64 Kio au plus par appel ; erreurs `AgentForm` (Snafu). Mêmes contrôles de contenu que `report` (couleur hexadécimale, 1 à 4 faits, longueurs).
- **Longueurs** des textes de l'agent : titre et faits 120 caractères, libellé de suite 60, explication 4000, consigne d'une suite 2000.
- **Surfaces figées** : 8 au plus par tour (la plus ancienne cède) ; des réglages figés sont en lecture seule (champs sans action, boutons retirés).
- **Dock** : quand une question attend, les réglages se replient sur une ligne qui les rouvre (au lieu d'un repli au-delà de 45 %) ; le dock défile s'il dépasse 45 % du panneau.
- **Progression** : une mise à jour du tour ne part que si ses données changent ; un tour dont la fin s'est perdue est clos par le retour de l'agent au repos, ou interrompu par le tour suivant.
- **Hub** : la gestion des tours est dans `hub_turns.rs` ; `hub.rs` garde les questions, les actions et les réglages.
- **Nouvelle conversation** : l'acteur de l'onglet vide le fil lui-même (`clear_thread`), de façon synchrone, avant toute consigne suivante ; le relais n'y touche plus.
- **Hors d'un tour ouvert** (entre deux tours, tour annulé), `render` et `report` sont refusés : un appel tardif ne s'affiche dans aucun tour.
- **`render` sans modèle de données** : valeurs littérales seulement (ni `{path}` ni `data`), seul un bouton porte une action ; les composants de champ (`TextField`, `Stepper`, `Slider`, `ColorField`, `Select`, `ChoiceChips`) sont refusés (les réglages passent par `show_params`) ; `Receipt` et `SlideThumb` sont réservés à l'hôte (via `report`) ; un `Badge` `danger` aussi ; les textes de l'agent ne contiennent ni contrôle bidirectionnel ni caractère invisible.
- **Expiration des suites** : celles d'une carte `render` expirent au tour suivant, comme celles d'un reçu.
- **Question figée** : elle montre la question (en légende) et la réponse (« ✓ … »), pas la réponse seule ; elle garde sa place dans le fil.
- **Mises à jour du tour** : une écriture par clé du modèle qui change (`/narration` pendant que l'agent écrit) ; narration gardée bornée à 4000 caractères, 60 étapes au plus par tour.
