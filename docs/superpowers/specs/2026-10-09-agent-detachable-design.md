# Zone agent détachable dans une fenêtre séparée

Tâche t-654. Aujourd'hui l'agent vit dans la colonne de droite (`notes`) de
chaque fenêtre de l'espace de travail : son fil, le dock des surfaces A2UI
vivantes, le brouillon et l'appareil de dictée. Cette spec permet de le
**détacher dans une fenêtre à part**, pensée pour un second écran : la
fenêtre principale rend toute sa largeur à la slide.

S'appuie sur `2026-09-26-onglets-fenetres-design.md` (fenêtres de l'espace
de travail) et `2026-10-08-sessions-par-perspective-design.md` (une session
par portée, état dans l'hôte).

## Décisions (choisies le 2026-10-09)

1. **Usage : second écran.** Une fenêtre normale, ni flottante ni « toujours
   devant » (hors périmètre).
2. **Une seule fenêtre agent pour toute l'application**, de label `agent`.
   Elle suit **l'onglet actif de la dernière fenêtre de l'espace de travail
   au premier plan** (`WorkspaceView.focused` → `active`).
3. **Ce qui part :** le fil (`AgentThread`), son dock, le brouillon
   (`DictationDraft`) et l'appareil de dictée (`Dictation`). **Ce qui
   reste** sur la scène de la fenêtre principale : les surfaces ancrées
   (`sheet`), le tiroir des variantes (`drawer`) et les modales (`modal`) —
   elles parlent de la slide.
4. **Persisté.** L'état détaché est gardé avec la mise en page de l'espace
   de travail ; taille, position et écran par `tauri-plugin-window-state`
   (label `agent`). Fermer la fenêtre agent la rattache.
5. **Le brouillon survit** au détachement, au rattachement et au changement
   d'onglet : il est tenu par l'hôte, par onglet, en mémoire (pas au
   redémarrage).
6. **L'hôte reste la seule source de vérité** (approche A) : la fenêtre agent
   s'abonne elle-même aux événements de l'agent, lit la portée et le
   brouillon à l'hôte. Le seul canal entre fenêtres porte ce qui n'existe
   que dans la fenêtre principale : où écrire un texte.
7. **Tauri seulement.** En mode navigateur (serveur de dev), ni bouton ni
   menu de détachement.

Écartées : une fenêtre « miroir » pilotée par la fenêtre principale (second
protocole, état doublé) ; une fenêtre agent qui recalcule la portée (elle
dépend d'un état d'interface propre à la fenêtre principale).

## Architecture

### Hôte (Rust)

- **Espace de travail** (`oku-slides-session`) : `agent_detached: bool`
  dans la mise en page, exposé dans `WorkspaceView`, sauvé et relu par
  `workspace_resume`. Méthodes `detach_agent()` / `attach_agent()`.
  Commandes `workspace` correspondantes ; chacune rediffuse `workspace`.
- **Focus** : `Workspace::focus` ne reçoit que les labels de fenêtres de
  l'espace de travail ; `commands::focused` ignore explicitement le label
  `agent` (aujourd'hui rejeté par accident, en « focus ignoré »). Donner le
  focus à la fenêtre agent ne change jamais la cible.
- **Fenêtres** (`src-tauri/src/windows.rs`) : `sync` réconcilie aussi la
  fenêtre `agent` — ouverte si `agent_detached`, fermée sinon. La décision
  (ouvrir / fermer / rien) est une fonction pure, testée sans Tauri. Taille
  par défaut 420×800, minimale 320×400 ; géométrie restaurée par
  window-state. Sa demande de fermeture (`CloseRequested`) appelle
  `attach_agent` au lieu de toucher aux onglets. Elle se ferme avec la
  dernière fenêtre de l'espace de travail, sans changer `agent_detached`.
- **Capability** : `src-tauri/capabilities/default.json` couvre
  `["w-*", "agent"]` (mêmes commandes : `agent`, `workspace`, `stt`, `draft`,
  et `core:event` pour `emitTo`).
- **Portée affichée** : le hub la tient déjà (`HubState.viewing`, posée par
  `resync`). Nouvelle sous-commande `agent` → `scope { tab }` qui la lit, et
  nouvel événement `AgentEvent::Scope { scope }` émis par `resync` quand
  elle change. La fenêtre principale reste seule à décider de la portée
  (`useAgentScope`) ; la fenêtre agent la suit.
- **Brouillon** : `draft: Option<String>` en mémoire dans la session de
  l'onglet. Commande `draft` : `get { tab }`, `set { tab, text | null }` ;
  `set` diffuse `oku-slides:agent` → `{ kind: "draft", text }`. Le dernier
  écrit gagne.
- **Menu** : dans le menu « Fenêtre », une case à cocher « Agent dans une
  fenêtre séparée » (`CheckMenuItem`), tenue à jour d'après
  `agent_detached`. Libellés via Fluent.

### Interface (React)

- **Racine** : `main.tsx` monte `AgentWindow` si le label de la fenêtre est
  `agent`, sinon `Workspace`.
- **`agentTarget(view)`** (pur) : l'onglet cible, `null` si aucun.
- **`AgentWindow`** : écoute `workspace`, en déduit l'onglet cible ; à chaque
  changement d'onglet : `subscribeHost(tab, …)`, `agent.scope(tab)`,
  `draft.get(tab)`, puis `agent-link:hello`. Monte `AgentThread` (surfaces
  routées par `routeSurfaces`, seules celles du fil), `DictationDraft` et
  `Dictation`. Bouton « Rattacher ». Onglet en lecture seule ou aide : état
  vide (« Pas d'agent pour cet onglet »), l'appareil de dictée reste.
- **`App`, agent détaché** : la colonne `notes` ne monte plus le fil, le
  brouillon ni la dictée ; elle se réduit à 0 hors de l'onglet « Thème »
  (`ThemePanel` reste). Les surfaces `sheet` / `drawer` / `modal` restent
  rendues ici. `App` publie son contexte d'écriture (ci-dessous) et applique
  les `insert` reçus.
- **`useDraft(tab)`** remplace le `useState` du brouillon, dans `App`
  (rattaché) comme dans `AgentWindow` : lit `draft.get`, suit l'événement
  `draft`, écrit par `draft.set` — frappe en debounce (~300 ms) ; dictée,
  Envoyer, Insérer, Abandonner tout de suite. Les compteurs `dictations` /
  `typing` (focus, défilement) restent locaux.
- **Bouton ⧉** dans l'en-tête de `AgentThread`, à côté de « Nouvelle
  conversation » : `detach_agent`. Caché hors Tauri.

Le câblage de l'agent dans `App.tsx` est à extraire en même temps (voir
t-462) : `App` ne doit pas porter deux branches (rattaché / détaché) partout.

## Canal entre fenêtres : `agent-link`

Événements Tauri ciblés ; tous portent `tab`, un message dont l'onglet ne
correspond plus est ignoré.

| Événement | Sens | Contenu | Quand |
|---|---|---|---|
| `agent-link:context` | principale → `agent` | `{ tab, editing, insertTo, vocabulary, readOnly }` | à chaque changement, et en réponse à `hello` |
| `agent-link:hello` | `agent` → toutes | `{ tab }` | ouverture, changement d'onglet cible |
| `agent-link:insert` | `agent` → `focused` | `{ tab, id, text }` | dictée vers le curseur, « Insérer » du brouillon |
| `agent-link:insert-ack` | principale → `agent` | `{ tab, id, ok }` | après `insert` |
| `agent-link:shortcut` | principale → `agent` | `{ tab, key: "dictate" \| "draft" }` | ⌘D / ⌘K pressés dans la fenêtre principale |

- `editing` : une édition de texte est en cours ; `insertTo` : le libellé
  « Insérer dans … » (`null` : nulle part où écrire) ; `vocabulary` : titres
  du deck et des slides, pour la dictée.
- Seule la fenêtre principale qui affiche l'onglet cible répond à `hello`.

## Flux

- **Dictée.** La fenêtre agent route avec `dictationRoute(mode,
  context.editing)` : vers le curseur → `insert` ; sinon dans le brouillon
  (`draft.set`).
- **Insérer.** `insert` → la fenêtre principale appelle `insertText` (curseur
  pendant l'édition, champ sélectionné, nouveau bloc, ou toast « choisir une
  zone ») et répond `insert-ack`. Sans `ack` sous 1 s, ou `ok: false` (onglet
  changé, rien où écrire), la fenêtre agent remet le texte dans le brouillon
  avec un toast « Texte gardé dans le brouillon ».
- **Envoyer.** `agent.prompt(scope, text)` depuis la fenêtre agent.
- **Raccourcis.** ⌘D dans la fenêtre principale → `shortcut: dictate` : la
  fenêtre agent démarre ou arrête l'écoute **sans prendre le focus**
  (l'édition garde son curseur). ⌘K → `shortcut: draft` : la fenêtre agent
  passe devant et ouvre le brouillon en saisie. Dans la fenêtre agent, ⌘D et
  ⌘K agissent sur place.
- **Focus perdu pendant l'édition.** Cliquer le micro de la fenêtre agent
  fait perdre le focus à la fenêtre principale : `editing` et la position du
  curseur doivent y survivre, pour que `insert` écrive au bon endroit.
- **Détacher / rattacher en cours de tour.** Le tour vit dans l'hôte : la
  nouvelle vue fait `resync` et reprend le flux. Le brouillon est relu à
  l'hôte.

## Cas limites

| Situation | Comportement |
|---|---|
| La fenêtre au premier plan change, ou son onglet actif | `AgentWindow` change de cible (réabonnement, portée, brouillon, `hello`). |
| Dernière fenêtre de l'espace de travail fermée | La fenêtre agent se ferme avec l'app ; `agent_detached` reste. |
| Onglet cible en lecture seule / aide | État vide ; dictée visible, son texte reçoit le refus de la session via `insert`. |
| Écran restauré absent | window-state replie la fenêtre sur un écran visible (vérifié). |
| Aucun onglet cible (vue pas encore reçue) | État de chargement, aucune commande envoyée. |

## Tests

**Rust**
- `detach_agent` / `attach_agent` ; `agent_detached` sauvé puis relu par la reprise.
- Le focus d'un label hors des fenêtres de l'espace de travail ne change pas `focused`.
- Décision de réconciliation de la fenêtre `agent` (ouvrir / fermer / rien).
- `resync` émet `Scope` au changement de portée, pas sinon ; `scope { tab }` la renvoie.
- `draft` : `set` puis `get` ; diffusion ; isolation entre onglets ; `null` efface.

**TypeScript (`bun test`)**
- `agentTarget` : fenêtre au premier plan, onglet actif, vue vide.
- `routeSurfaces` côté fenêtre agent : seules les surfaces du fil.
- `useDraft` : debounce de la frappe, envois immédiats, suivi de l'événement.
- Canal `agent-link` en modules purs : routage de la dictée, `ack`, repli
  dans le brouillon sur refus ou délai, message d'un autre onglet ignoré.

**e2e** (port libre, données isolées)
- Détacher → dicter (texte simulé) → Insérer dans le champ sélectionné →
  rattacher : le brouillon est conservé.
- Redémarrage : la fenêtre agent est restaurée détachée.
- Édition en cours, micro cliqué dans la fenêtre agent : le texte arrive au curseur.

## Précisions du plan (2026-10-09, révisées après la revue d'équipe et t-638)

Le plan (`docs/superpowers/plans/2026-10-09-agent-detachable.md`) précise la
mécanique. Ces points remplacent tout ce qui les contredit plus haut.

1. **Portée vue.** Le hub la tient (`HubState.viewing`) et la diffuse comme
   `McpEvent::Viewing { scope }` :
   - `resync` l'émet avant la liste ;
   - `rename_scope` l'émet quand la portée vue est renommée ;
   - le relais en retard la renvoie avec la liste.

   La commande `agent` → `follow { tab }` rejoue l'état et le brouillon, puis
   envoie au gardien l'ordre `Keep::Replay`. Le gardien lit la portée
   **quand il traite l'ordre**, si bien qu'un `resync` envoyé juste avant
   passe toujours d'abord. Il n'y a ni `AgentEvent::Scope` ni requête
   `agent.scope`.
2. **Brouillon.** La pile le tient en mémoire, dans la vue de l'onglet
   (`TabLink`). On l'écrit par `agent` → `set_draft { tab, text, origin }`,
   qui diffuse `AgentEvent::Draft { text, origin }`.
   - Chaque vue ignore l'écho de ses propres envois (`origin`) et les envoie
     dans l'ordre, un à la fois.
   - Le brouillon est plafonné à 256 Kio : au-delà, la commande est refusée.
   - `follow` et `set_draft` ne démarrent jamais un onglet. Un onglet sans
     agent démarré rejoue « rien » : repos, aucun brouillon, `slides`,
     aucune surface.
3. **Interface.** `AgentPanel` (t-638) a deux modes de portée :
   - **pilote** : la portée vient de la vue, c'est le mode de la fenêtre
     principale ;
   - **suiveur** : la portée vient de `viewing` après `follow`, c'est le mode
     de la fenêtre de l'agent.

   Une fois l'agent détaché :
   - la fenêtre principale garde un panneau pilote sans fil (fiches, tiroir,
     fenêtre de confirmation, touches rapides) ;
   - la fenêtre de l'agent monte un panneau suiveur avec le fil. Ses touches
     rapides ne visent que le dock, et elle route les fiches d'après celle
     que la scène montre (`staged`).

   Tant que l'hôte n'a pas annoncé la portée, la fenêtre de l'agent est en
   chargement, et l'envoi d'une consigne reste désactivé.
4. **Contexte d'écriture.** `agent-link:context` porte
   `{ editing, insertTo, ui, staged, keysOff }`. La fenêtre de l'agent relit
   elle-même dans l'instantané de l'onglet le vocabulaire, la lecture seule et
   les miniatures.
5. **Transport.** Un seul évènement Tauri, `agent-link`, diffusé à toutes les
   fenêtres (l'émettrice comprise).
   - Chaque page n'écoute qu'une fois et garde les messages qui lui sont
     adressés (`to`, `tab`).
   - Chaque message est validé par un schéma zod.
   - Hors Tauri, le canal est un `BroadcastChannel` et la fenêtre de l'agent
     est la page `/?window=agent` (e2e).
   - `hello` ne part qu'une fois l'écoute en place.
   - `insert` attend un accusé pendant 3 s. Faute de réponse, il envoie
     `insert-cancel`, que la fenêtre principale respecte, et le texte revient
     au brouillon avec un toast.
6. **Raccourcis.** La fenêtre principale relaie ⌘D, ⌘⇧D et ⌘K **bruts**
   (`auto`, `draft`, `type`). La fenêtre de l'agent les applique avec son
   propre état : ⌘⇧D arrête le tour seulement s'il n'y a pas d'écoute en
   cours. ⌘K met la fenêtre de l'agent devant ; ⌘D ne lui prend pas le focus.
   Limite connue : Échap dans la fenêtre principale n'arrête pas une écoute de
   la fenêtre de l'agent ; ⌘D le fait.
7. **Focus.**
   - Un changement de fenêtre au premier plan diffuse la vue `workspace`, et la
     fenêtre de l'agent la suit.
   - La fenêtre `agent` ne change jamais `focused`.
   - Elle n'est pas une cible de lâcher d'onglet.
   - L'éditeur de texte ne termine plus l'édition quand le document perd le
     focus (`document.hasFocus()` faux), pour une autre fenêtre ou une autre
     app. Ce changement vaut aussi sans agent détaché.
8. **Fermeture.** La fermeture système de la fenêtre de l'agent passe par sa
   page (`close-requested`), qui vide la frappe en attente puis rattache
   l'agent. Le bouton « Rattacher » fait de même.
9. **Menu.** Seul macOS a un menu d'app : la case à cocher n'existe que là.
   La case reflète l'espace de travail, et un clic bascule l'état lu dans
   l'espace de travail.
10. **Tests.** L'e2e tourne dans un navigateur, sans Tauri. Il couvre :
    détacher, envoyer depuis la fenêtre de l'agent, ⌘K relayé, insérer dans
    la zone sélectionnée et brouillon gardé au rattachement.

    Le reste est couvert au niveau unitaire, puis vérifié à la main dans l'app
    à la dernière tâche : redémarrage, perte de focus pendant l'édition, suivi
    du focus des fenêtres, micro dans une seconde webview, menu.

    La dictée parlée est remplacée par la saisie au clavier : l'e2e n'a pas
    de faux moteur de reconnaissance vocale.

## Hors périmètre

Fenêtre « toujours devant » ; plusieurs fenêtres agent ; détacher en mode
navigateur ; brouillon gardé au redémarrage ; détacher les surfaces de scène.
