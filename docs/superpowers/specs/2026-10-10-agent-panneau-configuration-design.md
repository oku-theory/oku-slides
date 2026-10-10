# Panneau de configuration des agents

Tâche t-775. Aujourd'hui l'app lance **un seul agent ACP**, dont la commande
et les arguments sont fixés dans `[agent]` de `config.default.toml`
(`npx -y @agentclientprotocol/claude-agent-acp@0.81.2`). On peut les
remplacer par `config.toml` ou `OKU_SLIDES_AGENT__*`, mais aucune interface
ne permet de changer d'agent. Cette spec ajoute une **liste d'agents définie
par la configuration** et un **panneau dans la zone de l'agent** pour choisir
l'agent actif.

S'appuie sur `2026-10-09-agent-detachable-design.md` (la fenêtre détachée
rend aussi `AgentPanel`) et sur le cycle de vie de `oku-slides-agent`
(`ARCHITECTURE.md` : un agent pour l'app, arrêt → `agent-stopped`, relance au
prompt suivant).

## Décisions (choisies le 2026-10-10)

1. **Choisir un agent dans une liste ; un seul actif pour toute l'app.** Pas
   d'agent par onglet ni par perspective, pas de réglage du modèle ou des
   modes de l'agent (hors périmètre).
2. **La liste vient de la configuration** (`config.default.toml`, remplaçable
   par `config.toml`). Le panneau **choisit seulement** : il n'ajoute ni
   n'édite d'agent. Lancer un sous-processus arbitraire reste une décision
   prise dans un fichier explicite.
3. **Le panneau vit dans la zone de l'agent** : un bouton de l'en-tête
   d'`AgentPanel` ouvre une fiche. Pas de fenêtre « Réglages… » (sa propre
   spec, plus tard).
4. **Bascule immédiate.** L'agent en cours est arrêté ; un tour en cours
   échoue comme pour tout arrêt (`agent-stopped`) ; le prompt suivant de
   chaque onglet lance le nouvel agent. La fiche avertit, avant le clic,
   qu'un tour sera interrompu.
5. **Agents livrés : Claude, Codex, Gemini**, chacun épinglé à une version :
   - `@agentclientprotocol/claude-agent-acp@0.81.2` (inchangé ; la montée de
     version est un autre chantier) ;
   - `@agentclientprotocol/codex-acp@2.2.2` : `@zed-industries/codex-acp` est
     déprécié (« replaced by @agentclientprotocol/codex-acp », vu sur npm le
     2026-10-10) ;
   - `@google/gemini-cli@0.63.0 --acp` : `--experimental-acp` est déprécié au
     profit de `--acp`.
   Chaque agent est vérifié (poignée de main, `agent_capabilities.mcp.http`)
   pendant l'implémentation ; un agent qui échoue est retiré de la liste.
   Vérifié le 2026-10-10 : avec un compte Google inéligible (offre gratuite,
   `IneligibleTierError`, `UNSUPPORTED_CLIENT`), Gemini CLI ne répond jamais à
   `initialize` ; le premier prompt échoue alors après le délai de démarrage, et
   changer d'agent interrompt ce lancement.
6. **L'agent ne choisit pas son successeur** : `set_setting` refuse `agent.*`.

## Configuration

`config.default.toml`, le bloc `[agent]` devient :

```toml
[agent]
# Agent actif : une clé de [agent.presets] ; choisi dans le panneau de l'agent.
selected = "claude"

[agent.presets.claude]
name = "Claude"
command = "npx"
args = ["-y", "@agentclientprotocol/claude-agent-acp@0.81.2"]

[agent.presets.codex]
name = "Codex"
command = "npx"
args = ["-y", "@agentclientprotocol/codex-acp@2.2.2"]

[agent.presets.gemini]
name = "Gemini"
command = "npx"
args = ["-y", "@google/gemini-cli@0.63.0", "--acp"]
```

Une **table nommée** (`[agent.presets.<id>]`) plutôt qu'un tableau : le crate
`config` fusionne par clé, donc un `config.toml` qui déclare
`[agent.presets.mistral]` ajoute un agent sans effacer les trois autres, et
peut en remplacer un en reprenant sa clé.

### Types (`oku-slides-platform`)

- `AgentSettings { selected: String, presets: BTreeMap<String, AgentPreset> }`
  (ordre d'affichage stable : celui des clés).
- `AgentPreset { name: String, command: String, args: Vec<String> }`. Le nom
  est un nom propre venu de la configuration, pas une chaîne Fluent.
- `AgentPreset::new(name, command, args)` : la structure est
  `#[non_exhaustive]` comme les autres réglages, les tests des autres crates
  passent par ce constructeur.
- `AgentSettings::active() -> &AgentPreset`. La validation a lieu **au
  chargement** (`Settings::from_sources`) : un `selected` qui ne désigne aucun
  preset est une `ConfigError` (message anglais `agent.selected = "<id>" names
  no [agent.presets] entry (known: …)`), qui remonte comme toute erreur de
  réglages (`PlatformError::Settings`) : le `Localizer` n'existe pas encore à
  ce moment-là. Pas de repli silencieux sur le premier agent.
- `AgentSettings::locked: bool` (`#[serde(skip)]`) : vrai si
  `OKU_SLIDES_AGENT__SELECTED` est défini, posé par `Settings::load`.

### Disparitions (sans shim, ADR-015)

`agent.command`, `agent.args` et le découpage d'`OKU_SLIDES_AGENT__ARGS` sur
les espaces (`list_separator(" ")` / `with_list_parse_key("agent.args")` dans
`config.rs`) disparaissent. Tous les consommateurs sont mis à jour :

- `Stack::start` (lancement depuis `AgentSettings::active()`) ;
- `crates/oku-slides-agent/tests/common` et `examples/smoke.rs` ;
- `crates/oku-slides-platform/src/config_tests.rs` (test des variables
  `AGENT__COMMAND`/`ARGS`, remplacé par ceux de `selected`) ;
- la recette `e2e` du `justfile` : au lieu de
  `OKU_SLIDES_AGENT__COMMAND`/`ARGS`, elle écrit dans son dossier de données
  isolé (`$tmp/data/config.toml`) deux presets du faux agent, `e2e` et
  `e2e-bis` (scénarios `fixtures/agent/e2e.json` et un second scénario
  `e2e-bis.json`), avec `selected = "e2e"` ; le commentaire de
  `ui/e2e/agent.cjs` suit ;
- le tableau des réglages du `README.md` (`[agent] selected` et
  `[agent.presets.<id>]`).

`OKU_SLIDES_AGENT__SELECTED` fonctionne naturellement et **l'emporte** sur
`config.toml`. Dans ce cas, le choix fait dans le panneau serait écrit mais
ignoré au lancement suivant : le panneau se montre **verrouillé** (voir plus
bas). Le verrou est détecté par la présence de la variable au démarrage
(`AgentSettings::locked`).

Le message `agent-not-found` (« Réglage [agent] command ») est mis à jour dans
les 20 langues : il renvoie à `[agent.presets]`.

## Bascule

### `Link::switch(launch: Launch)` (nouveau, public, synchrone)

`Link` est cloné dans chaque onglet ; lâcher le dernier clone arrête l'agent.
On ne remplace donc pas le `Link` de `Stack` : la bascule a lieu **dans** le
`Link` partagé.

Synchrone : `Stack::handle` l'est aussi, et un prompt qui suit le choix doit
voir le nouvel agent sans course avec une tâche lancée à part.

1. Sous un verrou standard (`Inner::launch: StdMutex<(Launch, époque)>`),
   remplace le lancement et incrémente l'**époque** (`u64`). `agent_id()` et
   `fallback_cwd()` (rendu en `PathBuf`) lisent le lancement courant :
   l'empreinte change, le gardien ne tente pas la reprise d'une session de
   l'autre agent (nouvelle session, sans carte « mémoire perdue »).
2. Arrête la connexion vivante ou en cours de lancement : chaque `Live` porte son époque et un jeton
   `stop`, **enfant** du jeton `shutdown` ; `Inner::current_stop` garde celui
   de la connexion vivante ou en cours de lancement (posé avant la poignée
   de main). L'annuler tue le sous-processus de cette
   génération seulement ; `shutdown` les annule tous et reste définitif.
3. `connect` ne réutilise une connexion que si elle n'est pas close **et** de
   l'époque courante ; une connexion d'une autre époque est arrêtée (`stop`)
   puis remplacée. Un lancement en cours est interrompu par la bascule, et
   la tentative suivante lance le nouvel agent.
   Les routes de l'ancienne génération ne reçoivent plus rien.

Les tours en cours passent par le **chemin d'arrêt existant** : connexion
fermée → `agent-stopped` dans le fil, groupes d'annulation refermés,
questions relâchées. Rien de neuf côté onglets.

### `Stack::select_agent(id: &str) -> Result<(), String>` (nouveau)

1. `id` absent des presets (surface périmée) : erreur localisée, rien ne
   change. Verrou d'environnement : refus localisé.
2. Déjà l'agent actif : rien.
3. Écrit `agent.selected` dans `config.toml` par `write_value`
   (`settings_port.rs`). **Écriture refusée : pas de bascule**, le choix
   affiché et le choix enregistré ne divergent jamais.
4. `link.switch(Launch { … })` avec le preset choisi, `fallback_cwd` et
   `READY_WITHIN` inchangés.
5. Pousse la surface du panneau à jour aux vues de l'onglet qui a choisi.

`Stack` garde l'id actif dans un `Mutex<String>`, initialisé depuis
`settings.agent.selected` : `Platform.settings` est figé au chargement.

### `set_setting` refuse `agent.*`

`AppSettings::write` refuse toute clé `agent` ou `agent.*` avec le message
`agent settings are reserved to the user`, rendu à l'agent comme les autres
refus de `write` (`dictation unavailable`…). `read` ne les expose pas.

## Surface et interface

Les surfaces d'onglet passent par le `SurfaceHub` de l'onglet ; le choix
d'agent vaut pour l'app, mais l'UI ne lit que des enveloppes `{tab, event}`
(`ui/src/agent/events.ts`). Le panneau est donc **livré à l'onglet qui le
demande**, sur le canal `"agent"`, comme le brouillon (`AgentEvent::Draft`,
émis directement par `emit`, ignoré par le réducteur des tours). Les vues de
cet onglet (fenêtre principale et fenêtre détachée) le reçoivent ; rien ne
change côté hôtes ni abonnement.

### Construction (`oku-slides-agent`, nouveau module `picker`)

`picker_surface(presets, active, busy, locked, l) -> Surface` est pure.
Surface A2UI d'id `agent-picker` :

- titre localisé (« Agent ») ;
- `ChoiceChips` : une pastille par preset (libellé : le nom ; `hint` : la
  commande et ses arguments), valeur `/choice` = l'actif ; action `select`
  avec `{id: /choice}` ;
- `busy` (un tour ouvert, `hub.turn()`, dans un onglet) : ligne
  d'avertissement « Changer d'agent interrompra le tour en cours » ;
- `locked` : pastilles **sans action** et ligne « Choix imposé par
  l'environnement (OKU_SLIDES_AGENT__SELECTED) ».

Nouvel évènement `AgentEvent::Picker { surface }` (`AgentEvent` perd `Eq`,
`Surface` n'est que `PartialEq`). Deux nouvelles commandes :

- `AgentCommand::Picker { tab }` : construit la surface (`busy` évalué à cet
  instant) et l'émet à `tab`. Envoyée par l'UI à l'ouverture de la fiche :
  l'état montré est toujours celui du moment où on l'ouvre.
- `AgentCommand::SelectAgent { tab, id }` : `Stack::select_agent(id)`, puis
  émet la surface à jour à `tab`. Distincte de `SurfaceAction`, réservée aux
  surfaces d'onglet.

Comme toute commande, elles sont ignorées pour un onglet fermé ; elles ne
démarrent pas l'onglet.

### Affichage (UI)

- `AgentThread` reçoit `onPicker` : un bouton (icône `settings`, libellé
  `ui-agent-picker`) dans `agent-thread-head`, toujours présent quand
  `onPicker` est donné (l'en-tête n'est alors jamais `vacant`).
- `AgentPanel` tient `picker: Surface | null` (dernier `AgentEvent::Picker`)
  et `pickerOpen`. Le bouton ouvre la fiche et envoie `picker` ; un second
  clic, Échap, ou un choix accepté la ferment.
- Nouveau composant `ui/src/agent/AgentPicker.tsx` : rend la surface par
  `SurfaceView` (chargé paresseusement, comme `AgentThread`), sous l'en-tête.
  L'action `select` part par `agent.selectAgent(id)` ; un refus s'affiche par
  `say(…, true)` (bandeau).
- La fenêtre détachée rend `AgentPanel` : même bouton, même surface.
  `oku-slides serve` aussi (navigateur), par le même relais SSE.
- Pas de modale de confirmation : l'avertissement est visible avant le clic.

### i18n

Titre, avertissement, verrou, libellé du bouton et erreurs montrées à
l'utilisateur (`id` inconnu, écriture refusée, verrou) passent par Fluent dans
toutes les langues existantes. Les noms des agents viennent de la
configuration.

## Erreurs

| Cas | Comportement |
|---|---|
| `selected` inconnu au chargement | `ConfigError` (clé, agents connus) : l'app ne démarre pas, comme pour toute configuration invalide |
| `SelectAgent` d'un `id` inconnu | erreur localisée à l'UI (bandeau), pas de bascule |
| Écriture de `config.toml` refusée | erreur localisée, pas de bascule, surface inchangée |
| Choix imposé par l'environnement | liste désactivée ; une commande reçue quand même est refusée |
| Nouvel agent introuvable ou sans MCP HTTP | erreur existante dans le fil au premier prompt (`agent-not-found`, `agent-no-http`, délai `ready_within`) ; on peut revenir à l'agent précédent |
| `set_setting("agent.…")` par l'agent | refusé (`agent settings are reserved to the user`) |

## Tests

Test-first à chaque étape.

- **platform** : les trois presets par défaut chargés ; un `config.toml` qui
  ajoute un preset garde les autres ; `selected` inconnu → erreur ;
  `OKU_SLIDES_AGENT__SELECTED` l'emporte sur `config.toml`.
- **link** : `switch` arrête la génération vivante et la relance reste
  possible ; le `connect` suivant lance la nouvelle commande ; `agent_id`
  change ; `shutdown` après `switch` reste définitif ; une bascule pendant un
  lancement relance avec le nouvel agent.
- **stack** (faux agent, deux presets à scénarios différents) : bascule hors
  tour, le prompt suivant parle au second agent ; bascule pendant un tour, le
  tour échoue en `agent-stopped` puis le prompt suivant parle au second ;
  écriture refusée → pas de bascule ; pas de reprise ni de carte « mémoire
  perdue » d'un agent à l'autre ; verrou d'environnement → refus.
- **picker** : snapshots `insta` de la surface (normale, `busy`, `locked`).
- **settings_port** : refus de `agent.*`.
- **UI** : `AgentThread`/`AgentPanel` render tests : le bouton envoie
  `picker`, la fiche montre la surface reçue, un choix envoie `select_agent`
  puis ferme, Échap ferme.
- **e2e** : ouvrir le panneau, choisir le second faux agent, vérifier sa
  réponse ; port libre et dossier de données isolé.
- **Vérification des presets** (manuelle) : chacun des trois agents épinglés
  est lancé, sa poignée de main faite et `mcp.http` constaté ; un agent qui
  échoue est retiré de `config.default.toml` avant la fusion.

## Hors périmètre

Agent par onglet ou par perspective ; ajout ou édition d'agents depuis le
panneau ; réglages propres à l'agent (modèle, modes, consignes) ; fenêtre
« Réglages… » générale ; montée de version de `claude-agent-acp`.
