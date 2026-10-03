# Édition de widgets avec l'agent, dans la zone A2UI — design

Tâche : t-423. Dépend de t-416 (`2026-10-03-agent-a2ui-design.md`), étape 5
(dock) fusionnée, pour le dock et le composant A2UI de texte riche. Suite de
t-394 (widgets en sous-onglets) et t-422 (aperçu en haut, code dessous).

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
- **Chaque champ de l'hôte est adressé par une clé**, jamais par sa
  position : `name`, `description`, `columns`, `rows` pour le modèle, le
  nom du paramètre pour un widget. La clé voyage dans l'action du champ.
- **Valeurs d'essai en session**, par widget, en mémoire, dans
  `Mode::Templates` : jamais écrites dans le pack, perdues en quittant les
  modèles ou en changeant de pack. L'aperçu, l'interface et l'agent lisent
  la même source. Un essai n'est pas un changement du deck.
- **L'agent règle les essais par un outil dédié**, `set_widget_trial`
  (voir plus bas), et non par une cible widget de `show_params` : l'hôte
  connaît le schéma, l'agent ne donne que des valeurs.
- **`set_grid` est une op du deck** (`Op::SetGrid`), visible de l'agent dans
  le schéma des ops : elle change la grille et recadre les éléments. Cela
  revient, à dessein, sur t-294, qui avait retiré `set_grid` du schéma
  quand la grille est devenue un paramètre `grid`.
- **Les frappes de l'utilisateur gagnent** sur une écriture de l'agent dans
  un fichier non enregistré ; l'agent en est informé au tour suivant. Une
  **suppression** par l'agent gagne en revanche sur les frappes en attente
  de ce fichier.

## Réglages du modèle

- `panel.rs` ne construit plus de surface A2UI : il produit un `ShowParams`
  (nom, description en `Text` ; colonnes et lignes en `Stepper`, 1 à 24),
  dont les champs portent l'op qu'ils appliquent, comme un `show_params` de
  l'agent. Nom et description : `set_slide_field` (`name`, `description`).
  Une slide à blocs n'a pas de champs de grille.
- **Grille.** Nouvelle op `set_grid {slide, columns?, rows?}` dans
  `oku-slides-core` : un champ absent reste inchangé ; 1 à 24 pistes ; même
  grille : rien ne change. Elle écrit le paramètre `grid` (la gouttière,
  y compris celle du défaut déclaré, est gardée ; le paramètre est relu
  comme une `Grid`, d'autres clés n'y survivent pas) et recadre, avec les
  mêmes proportions arrondies (règle de l'ancien `rescaleCell`), la cellule
  des éléments des slots libres (`SlotMode::Free`) de la slide ; seules les
  cellules bougent, jamais les cadres. Elle s'annule en un pas.
- **Grille de départ** : le paramètre `grid` de la slide, sinon la grille
  par défaut déclarée par le widget de la slide (`p.grid(default: …)`, lue
  dans le registre), sinon 12 × 6 (`Slide::resolved_grid`). `set_cell`, la
  validation du deck et l'aperçu des widgets utilisent la même grille, et le
  panneau la montre. Les champs de grille n'apparaissent que si le widget de
  la slide déclare un paramètre `grid`, que la slide a une zone libre, et
  qu'elle n'a pas de blocs.
- `rescaleCell` passe en Rust et disparaît de `grid.ts` (seul
  `panelActions.ts` l'utilise) : une seule implémentation.
- `panelActionOps` (`ui/src/a2ui/panelActions.ts`) disparaît : la surface
  applique ses ops comme toute surface de réglages.
- Surface d'id `host-template`, `live: true`, `turn: None`, montrée tant
  qu'un modèle est affiché et qu'aucun sous-onglet widget n'est actif.

## Réglages du widget

- `Session::widget_settings(name)` : un champ par paramètre déclaré, dans
  l'ordre du schéma, et des lignes en lecture seule. `None` pour Commun
  (la carte du modèle reste alors affichée).

  | Type | Champ |
  |---|---|
  | `plain` | `Text` |
  | `rich` | `RichText` (nouveau composant A2UI, voir plus bas) |
  | `number` avec `min` et `max` | `Slider` (pas : centième de l'étendue si elle est ≤ 10, sinon 1) |
  | `number` sinon | `Text` (le texte est lu comme un nombre) |
  | `choice` | `Select` (ses valeurs) |
  | `flag` | `Select` (oui / non) |
  | `asset`, `slot`, `grid`, `host` | ligne en lecture seule « non réglable dans l'aperçu » |

- Valeur de chaque champ : la valeur d'essai si elle existe, sinon celle de
  `preview_params` (défaut du schéma, exemple localisé, valeur neutre).
  Ce que montre le champ est donc ce que dessine l'aperçu.
- Un champ de widget porte la clé de son paramètre ; il ne porte pas d'op
  du deck : rien n'entre dans l'historique.
- Widget sans paramètre : la carte dit « Ce widget n'a pas de paramètre ».
- **Fichier invalide** (en cours de frappe) : la carte `host-widget` reste,
  avec la seule ligne « Fichier invalide : réglages indisponibles » ; les
  essais sont gardés.
- Surface d'id `host-widget`, `live: true`, `turn: None`, montrée tant que
  le sous-onglet de ce widget est actif ; elle remplace `host-template`.

## Texte riche dans le dock

- Nouveau composant du catalogue A2UI, `RichTextField {label, value,
  action}` : un éditeur multiligne où le markdown d'édition se met en forme
  à la frappe (gras, italique, liens, pastilles, listes `- ` et `1. `),
  avec ⌘B et ⌘I. Il réutilise ce qu'utilise l'éditeur de la scène
  (`markdownDecorations`, `toggleEmphasis`).
- Le champ lit deux chemins du modèle de données : le markdown d'édition
  (`paragraphs_to_text`, montré) et les paragraphes d'origine. À la
  validation (perte du focus, ⌘↩), il écrit les deux et l'action part avec
  les paragraphes, construits par `textToParagraphs` à partir des
  paragraphes d'origine : la couleur et la taille d'une ligne inchangée sont
  gardées. Le chemin du markdown reste un texte.
- `show_params` gagne le genre de champ `rich` : l'agent donne sa valeur en
  paragraphes ; il peut ainsi proposer un réglage de texte riche du deck.

## Valeurs d'essai (session)

- `Mode::Templates` gagne les essais de chaque widget et les widgets écrits
  par l'agent (voir « Agent »), tous deux remis à zéro à l'entrée, au
  changement de pack et à la reprise d'un onglet.
- `set_widget_trials(name, {param: value})` : tout ou rien ; contrôle
  chaque valeur contre le type déclaré (erreur Snafu sinon, qui nomme le
  paramètre). `null`, un texte vide, une liste de paragraphes vide, ou une
  valeur égale à celle de `preview_params` (nombres comparés comme nombres)
  efface l'essai. Un texte `plain` de plus de
  4 Kio, ou un `rich` de plus de 64 paragraphes ou 16 Kio, est refusé.
- `preview_widget(name, params)` : paramètres donnés, puis essais, puis
  `preview_params`. L'interface n'envoie plus de paramètres.
- Renommage d'un widget : ses essais le suivent. Suppression : ils sont
  effacés. Écriture d'un widget : les essais des paramètres qui ne sont plus
  déclarés, ou dont le type a changé, sont effacés.
- Une révision des essais (`Snapshot.widget_trials`) relance les aperçus.

## Agent

- **Contexte** : `AgentUi` gagne `widget: Option<String>` (le sous-onglet
  actif). Le bloc `<oku-slides-context>` dit le widget ouvert et ses
  valeurs d'essai (`widget: carte · trial: "title" = "Météo"`), les noms
  entre guillemets JSON, chaque valeur coupée à 80 caractères, 10 essais
  au plus ; un nom de widget invalide n'est pas cité.
- **Outil `set_widget_trial`** `{name, values: {param: value}}` : règle
  les essais (tout ou rien), renvoie les valeurs retenues ; la carte
  `host-widget` et l'aperçu suivent. Refusé hors du mode modèles.
- **`preview_widget`** sans paramètres prend les essais : l'agent voit ce
  que voit l'utilisateur.
- **Provenance** : l'outil `set_widget` de l'agent appelle une fonction
  dédiée de la session (`set_widget_by_agent`) ; la requête de l'interface
  ne porte aucun drapeau d'origine. Une écriture enregistrée de l'agent
  marque le widget ; la première écriture enregistrée de l'utilisateur sur
  un widget marqué note pour l'agent « user edited widget <nom> after
  you ». Le renommage déplace la marque, la suppression l'efface.
- **L'éditeur suit l'agent** : en mode modèles seulement, `set_widget` et
  `remove_widget` réussis émettent `McpEvent::WidgetTouched {name,
  removed}`. L'interface ouvre le sous-onglet du widget écrit et l'active,
  sauf si le sous-onglet actif a des frappes non envoyées ou non
  enregistrées : il s'ouvre alors en arrière-plan. Un widget créé par
  l'agent s'ouvre dès que la liste des fichiers le contient.
- **Conflit** : si le fichier écrit par l'agent a des frappes non envoyées
  ou non enregistrées, ces frappes sont écrites (elles gagnent) ; la note
  ci-dessus en informe l'agent. **Suppression** : les frappes en attente du
  fichier supprimé sont abandonnées, le sous-onglet se ferme, et un message
  dit que l'agent a supprimé le widget.
- Instructions de l'agent : une phrase pour `set_widget_trial` et
  `preview_widget` en mode modèles, et une pour `set_grid`, dans les 20
  langues.

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
  affiché. Ils ne se figent jamais dans un tour. Ils ne sont pas montrés
  pendant l'historique.
- La surface n'est relue que si la slide, le widget, le pack, la révision
  des essais ou l'état du deck (empreinte et nom de la slide, annulation)
  changent ; une action part avec la slide et le widget de la surface
  affichée, jamais ceux d'un état plus récent.
- Un widget dont le sous-onglet est périmé (supprimé, renommé) donne la
  carte du modèle, sans erreur. Chaque réglage du modèle validé fait un pas
  d'annulation.

## Erreurs

- Valeur d'essai hors type, hors bornes ou trop longue : erreur Snafu
  localisée qui nomme le paramètre, toast côté UI ; côté agent, résultat
  d'outil en erreur.
- `set_widget_trials` sur un widget inconnu ou Commun (`UnknownWidget`),
  invalide (`WidgetInvalid`) ou hors mode modèles (`NotEditingTemplates`) :
  refus explicite.
- Réglage de l'hôte hors mode modèles, ou clé inconnue : refus.
- Aperçu en échec : diagnostics comme aujourd'hui ; les réglages restent.

## Tests

- Rust (`oku-slides-core`) : `set_grid` recadre comme l'ancien
  `rescaleCell` (arrondis compris), part de la grille déclarée par le
  widget, ne touche ni aux cadres ni à la gouttière, refuse 0 et 25,
  ne change rien à grille égale ; le schéma des ops nomme `set_grid`.
- Rust (`oku-slides-session`) : `widget_settings` donne le bon champ par
  type, les valeurs égales à `preview_params`, rien pour Commun, la carte
  vide sans paramètre, la ligne « fichier invalide » ; `set_widget_trials`
  contrôle le type et la longueur, efface l'essai égal au défaut ou vide,
  suit un renommage, élague à l'écriture ; `preview_widget` rend les
  essais ; le bloc de contexte cite le widget et ses essais, sans
  injection ; la note « after you » ; `template_params` et ses clés ;
  `set_grid` par un réglage de l'hôte s'annule en un pas.
- Rust (`oku-slides-mcp`) : `set_widget_trial` refusé hors modèles et
  émet `Changed` ; `set_widget`/`remove_widget` émettent `WidgetTouched` en
  mode modèles seulement.
- UI : le dock montre `host-template`, puis `host-widget` au changement de
  sous-onglet ; un champ changé redemande l'aperçu ; `WidgetTouched` ouvre
  et active le sous-onglet, en arrière-plan si des frappes attendent, y
  compris pour un widget nouveau ; les frappes non enregistrées ne sont pas
  écrasées ; une suppression par l'agent abandonne les frappes en attente ;
  `RichTextField` met en forme et renvoie des paragraphes.
- i18n : nouvelles clés dans les 20 langues (`i18n-keys.test` et
  `oku-slides-i18n`).
- e2e : `panel.cjs` passe par le dock ; un parcours widget (réglage →
  contenu de l'aperçu changé). Vérification visuelle en clair et en sombre.

## Hors champ

- Garder les essais entre deux sessions, ou les écrire comme exemples dans
  le pack.
- Régler `asset`, `slot`, `grid` ou `host` dans l'aperçu.
- Écrire la couleur et la taille d'un run en markdown d'édition.
