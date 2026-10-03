# Édition de widgets avec l'agent, dans la zone A2UI — design

Tâche : t-423. Dépend de t-416 (`2026-10-03-agent-a2ui-design.md`), étape 5
(dock) fusionnée. Suite de t-394 (widgets en sous-onglets) et t-422
(aperçu en haut, code dessous).

## Problème

En mode modèles, le panneau de droite montre « Ce modèle » et « Grille »
même quand un sous-onglet de widget est actif : ces réglages portent sur le
modèle caché derrière, pas sur le widget édité. Le widget, lui, n'a aucun
réglage : son aperçu prend toujours les valeurs par défaut ou d'exemple de
`preview_params`. L'agent écrit des widgets (`set_widget`) sans que
l'éditeur le suive, et ne sait pas quel widget l'utilisateur regarde.

## Décisions

- **Une seule zone de réglages : le dock de t-416.** Les réglages vivants
  de l'hôte (modèle, widget) et ceux de l'agent (`show_params`) y vivent.
  `PanelSurface` et le chemin `panel` à part de `App.tsx` disparaissent.
- **Un seul constructeur de champs** : celui de `show_params`
  (`agent/forms.rs`). Les réglages du modèle et du widget sont des
  `ShowParams` construits par l'hôte, pas un second constructeur A2UI.
- **Valeurs d'essai en session**, par widget, en mémoire, dans
  `Mode::Templates` : jamais écrites dans le pack, perdues en quittant les
  modèles. L'aperçu, l'interface et l'agent lisent la même source.
- **L'agent règle les essais par un outil dédié**, `set_widget_trial`
  (voir plus bas), et non par une cible widget de `show_params` : l'hôte
  connaît le schéma, l'agent ne donne que des valeurs.
- **Les frappes de l'utilisateur gagnent** sur une écriture de l'agent dans
  un fichier non enregistré ; l'agent en est informé au tour suivant.

## Réglages du modèle

- `panel.rs` ne construit plus de surface A2UI : il produit un `ShowParams`
  (nom, description en `Text` ; colonnes et lignes en `Stepper`), dont les
  champs portent l'op qu'ils appliquent, comme un `show_params` de l'agent.
  Nom et description : `set_slide_field` (`name`, `description`).
- La grille demande plus qu'une op : changer de grille recadre aussi la
  cellule de chaque élément (`rescaleCell`, `ui/src/grid.ts`). Nouvelle op
  du deck `set_grid {slide, columns, rows}`, développée dans
  `oku-slides-core` en `set_param` de `grid` puis un `set_cell` par élément
  placé. `rescaleCell` passe en Rust et disparaît de `grid.ts` (seul
  `panelActions.ts` l'utilise) : une seule implémentation. L'agent y gagne
  la même op.
- `panelActionOps` (`ui/src/a2ui/panelActions.ts`) disparaît : la surface
  applique ses ops comme toute surface de réglages.
- Surface d'id `host-template`, `live: true`, `turn: None`, montrée tant
  qu'un modèle est affiché et qu'aucun sous-onglet widget n'est actif.

## Réglages du widget

- `Session::widget_params(name) -> Option<ShowParams>` : un champ par
  paramètre déclaré, dans l'ordre du schéma. `None` pour Commun ou un
  widget sans schéma valide.

  | Type | Champ |
  |---|---|
  | `plain`, `rich` | `Text` (un `rich` reçoit un paragraphe simple) |
  | `number` avec `min` et `max` | `Slider` |
  | `number` sinon | `Stepper` |
  | `choice` | `Select` (ses valeurs) |
  | `flag` | `Select` (oui / non) |
  | `asset`, `slot`, `grid`, `host` | ligne en lecture seule « non réglable dans l'aperçu » |

- Valeur de chaque champ : la valeur d'essai si elle existe, sinon celle de
  `preview_params` (défaut du schéma, exemple localisé, valeur neutre).
  Ce que montre le champ est donc ce que dessine l'aperçu.
- Un champ porte une op d'essai, `set_widget_trial {widget, param,
  "$value"}`, au lieu d'une op du deck : rien n'entre dans l'historique.
- Widget sans paramètre : la carte dit « Ce widget n'a pas de paramètre ».
- Surface d'id `host-widget`, `live: true`, `turn: None`, montrée tant que
  le sous-onglet de ce widget est actif ; elle remplace `host-template`.

## Valeurs d'essai (session)

- `Mode::Templates` gagne `trials: BTreeMap<WidgetFile, Map<String, Value>>`.
- `set_widget_trial(name, param, value)` : contrôle la valeur contre le
  type déclaré (erreur Snafu sinon), l'enregistre, renvoie le `ShowParams`
  à jour. Une valeur égale à celle de `preview_params` efface l'essai.
- `preview_widget(name, params)` : paramètres donnés, puis essais, puis
  `preview_params`. L'interface n'envoie plus de paramètres.
- Renommage d'un widget : ses essais le suivent. Suppression, ou paramètre
  retiré du schéma : les essais orphelins sont effacés.

## Agent

- **Contexte** : `AgentUi` gagne `widget: Option<String>` (le sous-onglet
  actif). Le bloc `<oku-slides-context>` dit le widget ouvert et ses
  valeurs d'essai (`widget: carte · essai: title = "Météo"`).
- **Outil `set_widget_trial`** `{widget, values: {param: value}}` : règle
  les essais, met à jour `host-widget` si ce widget est ouvert, renvoie les
  valeurs retenues. Refusé hors du mode modèles.
- **`preview_widget`** sans paramètres prend les essais : l'agent voit ce
  que voit l'utilisateur.
- **L'éditeur suit l'agent** : en mode modèles, `set_widget` et
  `remove_widget` émettent `McpEvent::WidgetTouched {name, removed}`.
  L'interface ouvre le sous-onglet du widget écrit (`useTemplateWidgets.open`)
  et l'active, sauf si le sous-onglet actif a des frappes non enregistrées :
  il s'ouvre alors en arrière-plan. Un widget supprimé passe par `removed`.
- **Conflit** : si le fichier écrit par l'agent a des frappes non
  enregistrées, ces frappes sont écrites (elles gagnent) et la session note
  pour l'agent « l'utilisateur a modifié <widget> après toi »
  (`Session::note`). La garde `stale` de `useWidgetFiles` reste la seule
  règle côté interface.
- Instructions de l'agent : une phrase pour `set_widget_trial` et
  `preview_widget` en mode modèles, dans les 20 langues.

## Dock (UI)

- `AgentDock` (t-416) accepte, en plus des surfaces vivantes de l'agent,
  la surface de réglages de l'hôte (`host-template` ou `host-widget`),
  fournie par `App`.
- Ordre, du micro vers le haut : question en attente, réglages de l'agent,
  réglages de l'hôte. Quand une question attend, les réglages se replient
  sur leur ligne d'en-tête (règle de t-416), ceux de l'hôte compris.
- Un `show_params` de l'agent sur la slide affichée et `host-template`
  coexistent ; il n'y a jamais deux surfaces de réglages pour un même
  widget (l'agent passe par `set_widget_trial`, qui met à jour
  `host-widget`).
- Les réglages de l'hôte n'ont pas de « × » : ils suivent ce qui est
  affiché. Ils ne se figent jamais dans un tour.
- Après une action d'essai, l'interface redemande l'aperçu du widget
  (même garde de séquence que `WidgetTab`).

## Erreurs

- Valeur d'essai hors type ou hors bornes : erreur Snafu, toast côté UI ;
  côté agent, résultat d'outil en erreur avec le paramètre fautif.
- `set_widget_trial` sur un widget inconnu, Commun ou hors mode modèles :
  refus explicite.
- Aperçu en échec : diagnostics comme aujourd'hui ; les réglages restent.

## Tests

- Rust (`oku-slides-session`) : `widget_params` donne le bon champ par type,
  les valeurs égales à `preview_params`, rien pour Commun, la carte vide
  sans paramètre ; `set_widget_trial` contrôle le type, efface l'essai égal
  au défaut, suit un renommage, élague à la suppression ; `preview_widget`
  lit les essais ; le bloc de contexte cite le widget et ses essais ;
  `panel` donne un `ShowParams` dont les ops sont celles d'avant.
- Rust (`oku-slides-core`) : `set_grid` recadre les cellules comme
  `rescaleCell` (cas repris de ses tests), s'annule en un pas.
- Rust (`oku-slides-mcp`) : `set_widget_trial` refusé hors modèles ;
  `set_widget` émet `WidgetTouched`.
- UI : le dock montre `host-template`, puis `host-widget` au changement de
  sous-onglet ; un champ changé redemande l'aperçu ; `WidgetTouched` ouvre
  et active le sous-onglet, en arrière-plan si des frappes attendent ; les
  frappes non enregistrées ne sont pas écrasées.
- `i18n-keys.test` : nouvelles clés dans les 20 langues.
- e2e : `panel.cjs` passe par le dock ; un parcours widget (réglage →
  aperçu changé). Vérification visuelle en clair et en sombre.

## Hors champ

- Garder les essais entre deux sessions, ou les écrire comme exemples dans
  le pack.
- Régler `asset`, `slot`, `grid` ou `host` dans l'aperçu.
