# Sessions de l'agent par perspective, restaurées à la réouverture

Tâche t-598. Complète `2026-09-27-agent-onglets-design.md` (une session ACP
et un fil par onglet) : dans un onglet, l'agent a désormais **une session
par perspective** — les slides, l'atelier, chaque widget. Passer d'une
perspective à l'autre rappelle la session liée (son fil et la mémoire de
l'agent) ; fermer puis rouvrir l'application les restaure.

## Décisions (choisies le 2026-10-08)

1. **Trois sortes de portées.** `AgentScope::Slides` (mode deck, et
   l'historique), `AgentScope::Atelier` (mode modèles, onglets « Modèles » et
   « Thème » : le thème touche tous les slides, même conversation),
   `AgentScope::Widget(nom)` (mode modèles, sous-onglet d'un widget).
2. **Fil et mémoire séparés.** Chaque portée a sa session ACP et son fil.
   L'agent d'une portée ne voit pas ce qui s'est dit dans une autre.
3. **Une session de widget est liée au widget, pas à son sous-onglet.**
   Fermer le sous-onglet la garde ; le rouvrir rappelle son fil. Renommer le
   widget la déplace (comme `TemplateScratch::renamed` déplace sa marque) ;
   le supprimer l'efface.
4. **Un tour à la fois par onglet** (les groupes d'annulation de `Session`
   sont uniques). Un tour continue quand on change de perspective, et finit
   dans son fil. Pendant ce temps, dans une autre portée, la saisie est
   bloquée : « L'agent répond dans Slides… » et Annuler. Dans la portée du
   tour, un prompt annule puis repart, comme aujourd'hui.
5. **Restauration après redémarrage.** Les fils sont relus dès l'ouverture
   de l'onglet, sans lancer l'agent. La session ACP d'une portée n'est reprise
   qu'à son premier prompt : `session/resume` si l'agent l'annonce
   (`session_capabilities.resume`), sinon `session/load`
   (`agent_capabilities.load_session`) en ignorant son rejeu. Échec, ou ni
   l'un ni l'autre : nouvelle session, le fil reste, une note le dit.
6. **« Nouvelle conversation » ne touche que la portée courante**, en
   mémoire et sur disque. Fermer l'onglet du document oublie toutes ses
   portées (son dossier de reprise part avec lui).

## Portée (`oku-slides-session`, `agent`)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum AgentScope { Slides, Atelier, Widget(String) }
```

Partagée par `oku-slides-mcp` (le hub) et `oku-slides-agent` (l'acteur).
L'interface la calcule et l'envoie ; l'hôte ne la déduit pas du mode, pour
qu'une seule source décide de ce que l'utilisateur regarde.

## Interface

- `agentScope(editing, active: ActiveTab, tabs)` (pur, `ui/src/agent/scope.ts`) :
  deck ou historique → `slides` ; modèles, actif `null` ou `THEME_TAB` →
  `atelier` ; modèles, sous-onglet de widget → `widget` + son nom.
- `agentApi(tab)` : `prompt`, `newConversation` et `resync` prennent la
  portée ; `AgentCommand::{Prompt, NewConversation, Resync}` la portent
  (`stack.rs`).
- Quand la portée change, l'interface vide ses tours affichés et appelle
  `resync(scope)` — le chemin déjà pris au changement d'onglet.
- `AgentEvent::Status` porte la portée du tour. Tant qu'un tour tourne dans
  une autre portée, `AgentThread` bloque la saisie (décision 4) ; le
  bouton Annuler envoie `cancel` comme aujourd'hui.

## Hub (`oku-slides-mcp`)

`HubState` garde un fil par portée :

```rust
struct Thread { surfaces: Vec<Surface>, turns: u32, live_turns: BTreeSet<u32>, reported: HashSet<u32> }
// HubState : threads: BTreeMap<AgentScope, Thread>, viewing: AgentScope, turn: Option<(AgentScope, u32)>
```

- Les surfaces hors fil (scène, tiroir, modale, dock) restent communes à
  l'onglet : une question du tour en cours reste visible et peut recevoir
  une réponse depuis n'importe quelle perspective.
- `begin_turn(scope)` ouvre le tour dans le fil de `scope`. `Admit`, le gel
  des questions et le reçu de repli s'appliquent au fil du tour, quel que
  soit celui qu'on regarde.
- `resync(scope)` règle `viewing` et émet `Surfaces { list }` = surfaces
  hors fil + fil de `viewing`. Une surface du fil d'une autre portée n'est
  pas émise ; elle le sera au `resync` qui la montre.
- `clear_thread(scope)` remplace `clear_thread()`.
- La limite de 200 entrées s'applique à chaque fil.

## Acteur (`oku-slides-agent`, `tab_agent.rs`)

```rust
conversations: BTreeMap<AgentScope, Conversation>
enum Conversation { Live { generation: u64, id: SessionId }, Saved { id: SessionId } }
```

- `prompt(scope, text, ui)` : `Live` de la génération courante → la session ;
  `Saved`, ou `Live` d'un agent arrêté → reprise (ci-dessous) ; absente →
  `session/new`. `fresh` (instructions de premier tour, `turn.rs`) vaut
  pour une session nouvelle, pas pour une session reprise.
- `new_conversation(scope)` : `forget(scope)` + `hub.clear_thread(scope)` +
  `Session::forget_conversation(scope)`.
- Fin de l'acteur : oublie toutes les routes, garde le disque.

### Reprise d'une session

1. `Link::route(generation, id, Route { replay: true, .. })` **avant** la
   requête : les `session/update` du rejeu arrivent avant sa réponse.
2. `session/resume` si `connect()` a lu `session_capabilities.resume`,
   sinon `session/load` si `load_session` ; `cwd` = `document_dir()` (comme
   `open`) et le serveur MCP de l'onglet (son URL et son jeton actuels).
3. Réponse : `replay: false`, la session passe `Live`. Les notifications
   reçues pendant `replay` sont ignorées : le fil vient du disque.
4. Erreur (session inconnue, agent changé), ou aucune des deux
   capacités : route retirée, `session/new`, et une note dans le fil :
   « L'agent ne se souvient plus de cette conversation »
   (`agent-memory-lost`, 20 langues). Le tour continue sur la nouvelle
   session.

`Link::connect` garde ces deux capacités dans `Live` (elles sont lues avec
`mcp.http` à l'initialisation).

## Persistance (`oku-slides-session`, `resume.rs`)

Un fichier par portée dans le dossier de reprise de l'onglet :
`agent/slides.json`, `agent/atelier.json`, `agent/widget.<hex du nom>.json`
(`safe_name` : le nom d'un widget peut contenir n'importe quoi).
Le format est versionné et passe par `read_json`, avec sa limite de 1 Mio :

```rust
struct SavedConversation { version: u32, scope: AgentScope, agent: String, session: Option<String>, thread: SavedThread }
```

- `agent` : la commande et ses arguments (`[agent]`). Si elle diffère au
  relancement, la session n'est pas reprise (décision 5, cas d'échec), mais
  le fil est gardé.
- `thread` : les surfaces du fil, sous leur version figée (suites expirées :
  le `plain` de `Shown::Static`). Une question qui attend encore, ou un
  réglage vivant, ne sont pas gardés. `Surface` (et `Place`, `Anchor`)
  gagnent `Deserialize`.
- **Un seul écrivain** : la session de l'onglet. Le hub appelle
  `Session::keep_thread(scope, thread)` (par `TabSession::with`) à chaque
  fin de tour ; l'acteur appelle `Session::keep_agent_session(scope, id)`
  quand une session est ouverte ou remplacée, et
  `forget_conversation(scope)` lors d'une nouvelle conversation. L'écriture
  passe par le `resume_writer` (un `Job` de plus), comme `state.json`.
- Relecture : `Session::conversations()` à l'ouverture de l'onglet. `Stack`
  les passe au hub (fils) et à l'acteur (`Conversation::Saved`). Une portée
  `Widget(nom)` dont le widget n'existe plus dans le jeu de modèles est
  supprimée à la relecture.
- `rename_widget` déplace le fichier et la clé du hub ; la suppression
  d'un widget efface sa conversation.

## Erreurs

- Fichier de conversation illisible (JSON, version, taille) : ignoré et
  journalisé (`warn`), la portée repart vide ; les autres portées et l'onglet
  ne sont pas touchés.
- Écriture qui échoue : comme les autres écritures de reprise (journal, pas
  de blocage du tour).

## Tests

- `scope.test.ts` : `agentScope` pour chaque mode et onglet.
- Hub : deux fils séparés ; `resync(scope)` n'émet que le fil vu et les
  surfaces hors fil ; un tour en fond range ses surfaces dans son fil ;
  `clear_thread(scope)` laisse les autres ; 200 entrées par fil.
- Session : aller-retour `keep_thread` + `keep_agent_session` / `conversations` sans question
  vivante ; nom de widget avec `/` ou `.` ; fichier corrompu ignoré ; widget
  supprimé, renommé.
- Acteur, avec `fake_agent` (il gagne `load_session` et `session/resume`,
  activables) : une session par portée ; reprise au premier prompt
  seulement ; `resume` préféré à `load` ; le rejeu de `load` n'entre pas
  dans le fil ; échec → nouvelle session + note + instructions ; commande
  d'agent changée → pas de reprise.
- Interface : saisie bloquée dans une autre portée pendant un tour.
- e2e : prompt dans les slides, ouvrir un widget (fil vide), prompt, revenir
  aux slides (premier fil) ; relancer l'app : les deux fils sont là, et le
  prompt suivant reprend la même session (le faux agent le confirme).

## Hors périmètre

- Lister ou choisir d'anciennes conversations (`session/list`).
- Une portée par slide.
