# Sessions de l'agent par perspective, restaurées à la réouverture

Tâche t-598. Complète `2026-09-27-agent-onglets-design.md` (une session ACP
et un fil par onglet) : dans un onglet, l'agent a désormais **une session
par perspective** — les slides, l'atelier d'un pack, chaque widget d'un
pack. Passer d'une perspective à l'autre rappelle la session liée (son fil
et la mémoire de l'agent) ; fermer puis rouvrir l'application les restaure.

Révisée après la revue d'équipe du plan (2026-10-08) : architecture
(sessions ACP hors du hub, un gardien par onglet pour les écritures),
identité des portées (pack + nom), durcissement de la relecture.

## Décisions (choisies le 2026-10-08)

1. **Trois sortes de portées.** `Slides` (mode deck, et l'historique),
   `Atelier { pack }` (mode modèles, onglets « Modèles » et « Thème » : le
   thème touche tous les slides, même conversation), `Widget { pack, name }`
   (mode modèles, sous-onglet d'un widget). `pack` est l'id du pack édité
   (`TemplateMode::set`) : deux packs édités dans le même onglet ont chacun
   leurs conversations.
2. **Fil et mémoire séparés.** Chaque portée a sa session ACP et son fil.
   L'agent d'une portée ne voit pas ce qui s'est dit dans une autre.
3. **Une session de widget est liée au widget, pas à son sous-onglet.**
   Fermer le sous-onglet la garde ; le rouvrir rappelle son fil. Renommer le
   widget la déplace. **Supprimer le widget ne l'efface pas tout de suite** :
   annuler la suppression la retrouve. Les conversations dont le widget (ou
   le pack) n'existe plus sont nettoyées à la reprise suivante. Annuler un
   renommage ne ramène pas la conversation sous l'ancien nom (limite connue).
4. **Un tour à la fois par onglet** (les groupes d'annulation de `Session`
   sont uniques). Un tour continue quand on change de perspective, et finit
   dans son fil. Pendant ce temps, dans une autre portée, l'interface bloque
   l'envoi d'une consigne (« L'agent répond dans les slides… ») ; ⌘⇧D
   l'annule où qu'il soit (Échap n'annule que le tour de la portée vue), et
   une réponse à une question vivante de ce tour reste possible. Dans la portée du tour, un prompt annule puis repart, comme
   aujourd'hui.
5. **Restauration après redémarrage.** Les fils sont relus au démarrage de
   l'agent de l'onglet, sans lancer le processus ACP. La session ACP d'une
   portée n'est reprise qu'à son premier prompt : `session/resume` si
   l'agent l'annonce (`session_capabilities.resume`), sinon `session/load`
   (`agent_capabilities.load_session`) en ignorant son rejeu. Échec, ou ni
   l'un ni l'autre : nouvelle session, le fil reste, une carte le dit.
6. **« Nouvelle conversation » ne touche que la portée courante**, en
   mémoire et sur disque. Fermer l'onglet du document oublie toutes ses
   portées (son dossier de reprise part avec lui).

## Portée (`oku-slides-session`, `agent::scope`)

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentScope { #[default] Slides, Atelier { pack: Uuid }, Widget { pack: Uuid, name: String } }
```

JSON : `{"kind":"slides"}`, `{"kind":"atelier","pack":"<uuid>"}`,
`{"kind":"widget","pack":"<uuid>","name":"frise"}`. L'interface la calcule
et l'envoie ; l'hôte ne la déduit pas du mode, pour qu'une seule source
décide de ce que l'utilisateur regarde.

Nom de fichier (`file_name`) : `slides.json`, `atelier.<uuid>.json`,
`widget.<uuid>.<32 hex de sha256(nom)>.json` — longueur fixe, sûr quel que
soit le nom.

## Interface

- `agentScope({ templates, history, pack, widget })` (pur, `ui/src/agent/scope.ts`) :
  hors mode modèles, ou historique → `slides` ; mode modèles sans widget
  actif → `atelier` ; sinon `widget`.
- `useAgentScope` (`ui/src/agent/useAgentScope.ts`) tient la portée, l'état
  de l'agent (avec la portée de son tour) et le rappel du fil : quand la
  portée change (instantané chargé, abonnement prêt), l'interface vide ses
  surfaces puis demande `resync(scope)` ; la liste qui revient est comptée
  comme une resynchronisation (elle n'est pas relue à voix haute).
  `onReady` ne resynchronise qu'aux reconnexions. `Resync` démarre les
  outils et le gardien de l'onglet (pas l'agent ACP) : un onglet rouvert
  retrouve ainsi ses fils dès qu'il est affiché.
- `agentApi(tab)` : `prompt`, `newConversation` et `resync` prennent la
  portée ; `AgentCommand::{Prompt, NewConversation, Resync}` la portent
  (`#[serde(default)]` : absente, c'est `Slides`).
- `AgentEvent::Status` porte la portée du tour (`UserText` et
  `Conversation` aussi, pour le réducteur des tours). Tant qu'un tour tourne
  dans une autre portée : le fil montre l'agent au repos, le brouillon
  affiche la note et refuse l'envoi (sauf réponse à une question), une
  suite cliquée est refusée avec la même note.

## Hub (`oku-slides-mcp`) — sans entrées-sorties

Le hub reste **une liste de surfaces par onglet** et ne touche pas à la
session. Les numéros de tour restent uniques dans l'onglet ; chaque tour
retient sa portée :

```rust
// HubState : turn_scopes: HashMap<u32, AgentScope>, viewing: AgentScope
```

- Une surface est **montrée** si elle est vivante (dock, question, scène,
  tiroir, modale), hors tour, ou si son tour est dans la portée vue (un tour
  sans portée connue n'est pas montré). Le hub
  garde tout, mais n'envoie que ce qui est montré ; une surface qui passe de
  montrée à cachée (question d'une autre portée figée) part en
  `CloseSurface`. Le relais en retard renvoie `visible_surfaces()`.
- `begin_turn(scope)` note la portée du tour et ferme les plus anciens tours
  de la portée au-delà de `KEPT_TURNS` (tours relus compris).
- `resync(scope)` règle `viewing` et émet la liste montrée.
- `clear_thread(scope)` ne libère que les questions des tours effacés.
- `restore(conversations)` remet les fils relus, **nettoyés** : surfaces
  du fil seulement, sans actions, figées ; numéros de tour et d'identifiant
  bornés ; un tour déjà pris (par une autre portée, ou par un tour ouvert)
  est ignoré et journalisé ; `KEPT_TURNS` derniers tours. Aucun tour ne
  s'ouvre avant la relecture.
- `thread_snapshot(scope)` : surfaces figées de la portée, sans actions, des
  derniers tours **clos** (jamais le tour ouvert), dans un budget d'octets
  (`THREAD_BYTES`, sous la limite de lecture de 1 Mio) ; rien ne tient dans
  le budget → `None` (on n'écrit pas, l'historique enregistré reste).
- `rename_scope(from, to)`, `turn_scope(turn)`.

`KEPT_TURNS` (50) est défini une seule fois, dans `oku-slides-session::agent`.

## Gardien de l'onglet (`oku-slides-agent`, `keeper.rs`)

Une tâche par onglet, **seul passage vers les écritures de conversation** :
elle reçoit des ordres en file et les applique un par un, hors du fil async.

- Au démarrage : relit `Session::conversations()` et les donne à
  `hub.restore`. L'acteur attend cette relecture avant son premier tour.
- Avant chaque ordre : applique au hub les renommages notés par la session
  (`take_scope_moves` → `rename_scope`), et réécrit selon eux la portée de
  l'ordre lui-même (une session ouverte pendant un renommage suit le widget).
- `Thread(turn)` (fin de tour, depuis le relais) : la portée est lue au
  moment d'écrire (`turn_scope`), l'image du fil prise puis écrite
  (`keep_thread`). Tour effacé entre-temps : rien.
- `Session { scope, id, agent }`, `Forget(scope)` (depuis l'acteur).
- `Lookup(scope, réponse)` (depuis l'acteur) : la session connue de la
  portée et l'empreinte de son agent, lues **dans l'ordre de la file** —
  donc après un oubli ou un renommage déjà demandés.
- `Show(scope)` (depuis `Resync`) : `hub.resync(scope)`. `Stack::handle` ne
  prend donc jamais la session.
- Fermeture de l'onglet ou de l'app : la file est vidée (au plus 2 s pour
  toute l'app) avant l'arrêt.

L'ordre de la file règle les courses : une écriture de fil qui précède un
oubli est effacée par lui ; un renommage est appliqué avant toute écriture.

## Acteur (`oku-slides-agent`, `tab_agent.rs`)

L'acteur ne garde que l'état vivant : `live: HashMap<String, u64>` (id de
session → lancement de l'agent). Il demande la session d'une portée au
gardien (`Lookup`) : pas de cache par portée, donc rien à tenir à jour lors
d'un renommage ou d'un oubli.

- `prompt(scope, text, ui)` : session vivante du lancement courant → elle ;
  session connue sinon (relue, ou d'un agent arrêté depuis) → reprise ;
  aucune → `session/new`. `fresh` (instructions de premier tour) vaut pour
  une session nouvelle seulement.
- `new_conversation(scope)` : `Forget(scope)` au gardien, qui rend l'id
  oublié (sa route est oubliée), puis `hub.clear_thread(scope)`.
- Après une connexion finie, la session reste connue : elle sera reprise.
- `Starting` est émis tant que la session n'est pas vivante dans le
  lancement courant.
- Un prompt d'une autre portée pendant un tour garde le comportement actuel
  (annulation puis consigne) : c'est l'interface qui bloque.

### Reprise d'une session

1. Si l'empreinte d'agent enregistrée diffère de `Link::agent_id()`
   (sha256 de la commande et de ses arguments : jamais la commande en
   clair sur le disque), pas de reprise (étape 4).
2. `session/resume` si `session_capabilities.resume`, sinon `session/load`
   si `load_session` ; `cwd` = `document_dir()` et le serveur MCP de
   l'onglet (URL et jeton actuels).
3. La route n'est posée **qu'après** la réponse : les `session/update` du
   rejeu de `session/load` arrivent avant elle et sont ignorés comme ceux
   d'une session inconnue.
4. Refus de l'agent, ou aucune capacité : `session/new`, et une carte dans
   le tour : « L'agent ne se souvient plus de cette conversation »
   (`agent-memory-lost`, 20 langues). Une connexion finie reste une erreur
   du tour.

## Persistance (`oku-slides-session`)

`agent/<file_name>` dans le dossier de reprise de l'onglet, un fichier par
portée, versionné, relu par `read_json` (1 Mio au plus) :

```rust
pub struct SavedConversation { pub scope: AgentScope, pub agent: Option<String>, pub session: Option<String>, pub thread: Vec<Surface> }
// fichier : { "version": 1, ...SavedConversation }
```

- Un seul écrivain : la session ; un seul appelant : le gardien.
  `keep_thread`, `keep_agent_session`, `forget_conversation`. Le `Job` du
  `resume_writer` porte les conversations à écrire ou effacer ; la fusion
  de deux dépôts les garde toutes (le plus récent l'emporte par portée).
- `rename_widget` déplace la conversation et note `ScopeMove`. Supprimer un
  widget ne touche à rien (décision 3).
- Relecture : à la reprise de l'onglet ; un fichier illisible, ou dont la
  portée ne correspond pas à son nom, est ignoré (journalisé). Puis
  nettoyage **prudent** : une conversation `Widget` est effacée seulement si
  des fichiers de son pack ont été trouvés (version éditée, pack du deck,
  version courante de la bibliothèque) et qu'aucun ne contient le widget. Un
  pack introuvable (deck d'une autre machine, lecture en échec) ne fait
  rien effacer ; les conversations `Atelier` ne sont pas nettoyées.
- Écriture d'une portée : refusée (journalisée) si son widget n'est connu
  d'aucun fichier du pack ou si son nom est invalide (`parse_name`) — une
  portée inventée par l'interface ne crée pas de fichier.
- `ServerMessage` (A2UI) et ses messages, `Surface` et `Place` gagnent
  `Deserialize`.

## Erreurs

- Fichier illisible : ignoré et journalisé (`warn`), la portée repart vide.
- Écriture qui échoue : journalisé (`warn`), le tour n'est pas bloqué.

## Tests

- `scope.test.ts` : `agentScope` (deck, historique, atelier, widget),
  `sameScope`, `busyElsewhere`.
- Hub : surfaces montrées par portée ; surface montrée puis cachée →
  `CloseSurface` ; `clear_thread(scope)` garde les questions des autres
  portées ; `restore` nettoie et borne ; éviction par portée ;
  `thread_snapshot` (sans actions, sans questions, budget d'octets) ;
  `rename_scope`.
- Session : aller-retour à travers un redémarrage ; nom de widget long ;
  fichier corrompu ou mal nommé ignoré ; renommage ; nettoyage à la reprise
  (widget supprimé, pack disparu) ; suppression puis annulation garde la
  conversation ; fusion des dépôts (fonction pure).
- Gardien : renommage appliqué avant l'écriture ; oubli après une écriture
  en file.
- Acteur : une session par portée ; reprise au premier prompt seulement ;
  `resume` préféré à `load` ; rejeu de `load` absent des évènements et des
  surfaces ; refus → nouvelle session + carte + instructions ; autre agent →
  pas de reprise ; reprise après arrêt de l'agent ; statut porteur de
  portée ; consigne d'une autre portée en file pendant un tour.
- Pile : redémarrage de bout en bout (session reprise d'un dossier → fil
  montré au `resync`, premier prompt en `session/resume` avec l'id relu).
- e2e : prompt dans les slides, ouvrir l'atelier (fil vide), prompt par le
  brouillon, revenir aux slides (seul le premier fil).

## Hors périmètre

- Lister ou choisir d'anciennes conversations (`session/list`).
- Une portée par slide.
- Contrôle d'`Origin` du serveur de développement (antérieur).
