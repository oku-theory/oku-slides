# Widgets en sous-onglets dans la perspective modèles — design

Date : 2026-10-03 · Tâche : t-394

## Objectif

En mode modèles, chaque fichier de widget du pack s'ouvre comme un onglet :
1 widget = 1 onglet. Cela remplace le panneau modal « Widgets du pack »
(`ui/src/WidgetWorkbench.tsx`, livré par t-329, corrigé par t-392 et t-390).

Prérequis : t-390, fusionné dans `main` (9bf45b3).

Hors périmètre : réordonner les sous-onglets par glisser, détacher un widget
dans une autre fenêtre, garder l'historique d'annulation de l'éditeur après
un redémarrage.

## Décisions

1. **Sous-onglets dans l'onglet du deck**, pas un nouveau type d'onglet du
   `Workspace`. Un onglet reste une `Session` ; le mode modèles en est un
   état (`TemplateMode`). En mode modèles, la table gagne une barre de
   sous-onglets : « Modèles » (toujours là, non fermable), puis les
   widgets ouverts. `Workspace` et `TabInfo` ne changent pas.
2. **`_commun` est un sous-onglet comme les autres**, libellé « Commun ».
   À la place de l'aperçu, il liste les widgets en erreur (ceux qu'il a pu
   casser), chacun cliquable. On ne peut pas le renommer.
3. **Ouvrir** : depuis une section « Widgets » de la pile (sous les
   miniatures des modèles), ou depuis l'étiquette de l'élément sélectionné
   sur la scène quand son widget vient du pack. Le double-clic sur un
   élément garde l'édition de son champ principal.
4. **Créer, renommer, supprimer** : tout dans la section « Widgets » de la
   pile. « + » dans son en-tête ; renommer par double-clic sur le nom ou
   F2 ; supprimer par un bouton au survol, avec confirmation.
5. **Séance et versions inchangées** : la séance est le mode modèles ; tous
   les sous-onglets écrivent dans la même version de séance. Fermer un
   sous-onglet n'achève rien (sa frappe en attente part). **Annuler /
   Rétablir** dans un sous-onglet widget agissent sur l'historique
   CodeMirror de ce fichier ; chaque pas repasse par `set_widget`.
6. **Plus de panneau en mode deck** : ☰ « Éditer les widgets » et
   l'étiquette d'un élément de widget du pack entrent en mode modèles
   (`edit_templates`). La séance de widgets du mode deck
   (`widgets_deck.rs`) reste pour l'agent et MCP.
   En mode deck, l'étiquette sait qu'un élément vient du pack par
   `pack_widgets` du snapshot ; les statuts (et leur sonde) ne sont
   chargés qu'au clic, en entrant en mode modèles.
7. **Sous-onglets gardés dans `UiState`** : restaurés au redémarrage, ils
   suivent l'onglet du deck d'une fenêtre à l'autre.

## Session (Rust)

- `UiState` gagne deux champs, `#[serde(default)]` :
  - `widgets: Vec<String>` : sous-onglets ouverts, dans l'ordre
    d'ouverture ;
  - `widget: Option<String>` : sous-onglet actif ; `None` : « Modèles ».
- `UiState::sanitized(templates)` normalise ces champs : hors du mode
  modèles, vides ; en mode modèles, noms valides (`WidgetFile::parse`),
  sans doublon, au plus `MAX_WIDGET_TABS` (64), actif parmi eux. Appelé à
  la commande `ui` (`Request::Ui`) et à la reprise. Un deck ne reprend
  donc jamais de sous-onglets, même si les modèles repris sont illisibles.
- `close_templates`, `edit_templates` (entrée, ou passage à un autre pack)
  et `load_document` vident les deux champs ; l'interface pose ensuite ses
  sous-onglets par `Request::Ui`.
- Le snapshot gagne `pack_widgets` : les fichiers de widgets du pack en
  cours, sans `_commun`, lus sans sonde. L'étiquette de la scène s'en sert
  en mode deck.
- Aucune nouvelle opération : `widget_status`, `widget_source`,
  `set_widget`, `rename_widget`, `remove_widget` et `preview_widget`
  suffisent. L'interface retire les noms qui n'existent plus dans le pack.

## Interface

### Découpage

| Unité | Rôle |
|---|---|
| `useWidgetFiles` | État partagé du pack, **actif seulement en mode modèles** (aucune sonde en mode deck) : statuts et la `revision` pour laquelle ils ont été lus, **file unique** des opérations (écritures, renommages, suppressions, héritée de t-390), anti-rebond de 400 ms par fichier, marques « non enregistré », renommages en vol, statuts d'essai et compteurs de frappes **par fichier**. Remis à zéro à chaque changement de pack. Extrait de `WidgetWorkbench`. |
| `widgetTabs.ts` | Logique pure des sous-onglets : ouvrir (active s'il est déjà ouvert, sinon ajoute en fin), fermer (retour à gauche ; à gauche du premier, « Modèles »), renommer (même id), élaguer, reprise fusionnée avec les sous-onglets déjà ouverts. |
| `useTemplateWidgets` | Assemble fichiers et sous-onglets pour `App` : reprise une fois les statuts du pack lus, élagage seulement sur des statuts de la `revision` courante, `kept` pour `UiState` (vide hors mode modèles dès le même rendu), entrée depuis le deck, ✓ (frappes écrites avant `close_templates`), raccourcis ⌘W et ⌘Z des sous-onglets. |
| `WidgetPane.tsx` | Les sous-onglets montés, seul l'actif visible. |
| `TemplateTabs.tsx` | Barre secondaire (motif `tablist` : ←/→, Début/Fin, Suppr ferme) : clic pour activer, × ou clic du milieu pour fermer. Marque « non enregistré » sur le libellé. |
| `WidgetTab.tsx` | Un fichier : `TypstEditor`, aperçu cadré (`croppedSvg`) et diagnostics ; pour Commun, la liste des widgets en erreur. Seul le sous-onglet actif relit sa source et demande un aperçu ; un caché relit en devenant actif. |
| `PileWidgets.tsx` | Section « Widgets » de la pile : Commun en tête, pastilles valide/erreurs, « + » et nom en ligne (`nameAvailable`), renommer (double-clic, F2), supprimer confirmé. |

La file reste **unique** pour tout le pack (pas une par sous-onglet) : un
renommage depuis la pile doit attendre la frappe d'un sous-onglet, comme
dans le panneau de t-390.

Les sous-onglets ouverts restent **montés mais cachés** : chacun garde son
état CodeMirror, donc son historique d'annulation. Limites assumées :
l'historique se perd en changeant d'onglet de deck (seule l'`App` de
l'onglet actif est montée), au redémarrage, et quand l'agent change le
fichier (l'éditeur est recréé sur sa source). Un remplacement de nom
déclaré après un renommage n'entre pas dans l'historique.

### Comportements

- **Ouvrir** (pile, étiquette) : active le sous-onglet, ou l'ajoute en fin
  de barre et l'active. Depuis un deck : `edit_templates`, puis ouverture
  du sous-onglet (étiquette) ou focus sur la section Widgets (☰).
- **Sous-onglet widget actif** : la scène (cachée, gardée montée) et le
  plateau d'ajout cèdent la place à l'éditeur et à l'aperçu ; la pile
  reste. Annuler, Rétablir et ⌘Z/⌘⇧Z agissent sur l'éditeur du fichier ;
  ⌘W ferme le sous-onglet (sur « Modèles », ⌘W garde son sens). Les
  raccourcis de la scène (Suppr, flèches, Entrée) sont inactifs. Le titre
  et ☰ ne changent pas.
- **Fermer un sous-onglet** : `flush` de sa frappe, puis le sous-onglet de
  gauche devient actif.
- **✓ Terminer** : `flush` de tous les fichiers, attente de la file, puis
  `close_templates`.
- **Renommer** : le sous-onglet ouvert garde sa place sous le nouveau nom ;
  une frappe tapée pendant le renommage suit (règle `moves` de t-390).
- **Supprimer** : confirmation (« Supprimer le widget « x » ? »), puis `remove_widget` ; le sous-onglet
  ouvert se ferme. `WidgetInUse` : toast qui nomme les modèles, rien ne se
  ferme.
- **Agent** : mêmes opérations. Un changement de `revision` relit statuts et
  sources, la frappe locale en cours gardant la priorité (règles de t-390).
  Un fichier supprimé ou renommé par l'agent ferme son sous-onglet, sans
  bruit.
- **Menu ☰** : en mode modèles, l'entrée « Widgets » disparaît ; en mode
  deck, elle devient « Éditer les widgets ».

### Cas limites

- Source ou statuts illisibles : toast d'erreur, le sous-onglet reste.
- Historique ouvert (lecture seule) : barre de sous-onglets cachée.
- Restauration : noms absents des statuts retirés ; sous-onglet actif
  disparu → « Modèles ».
- Pack sans widget : section « Widgets » avec son « + » et le texte
  `ui-widgets-empty`.

### Retraits

`WidgetWorkbench.tsx`, ses tests (`WidgetWorkbench*.render.test.tsx`) et
l'aide `test/workbench.ts` (portés vers les nouvelles unités), les styles
`.workbench*` et les clés Fluent devenues inutiles (`ui-widgets-title`,
`ui-widgets-close`, `ui-widgets-pick`…), dans les 20 langues.

## Tests

Dans l'ordre de construction, en TDD :

1. **Session** : `UiState` aller-retour avec `widgets` et `widget`, valeurs
   par défaut pour un état ancien ; `close_templates` vide les deux
   champs ; reprise en mode modèles qui les rend.
2. **UI, logique (bun)** : `useWidgetTabs` (ouvrir, réactiver, fermer avec
   retour à gauche, renommage suivi, nettoyage des noms disparus, actif
   disparu → Modèles) ; `useWidgetFiles` (tests d'écriture, d'essai et de
   renommage repris de `WidgetWorkbench.writes`/`.trials`).
3. **UI, rendu** : `TemplateTabs` (activer, fermer, ⌘W) ; `PileWidgets`
   (pastilles, création, renommage, suppression confirmée, toast
   `WidgetInUse`) ; `WidgetTab` (diagnostics à la ligne, Commun et ses
   widgets en erreur, annuler par l'éditeur) ; étiquette de la scène
   seulement pour un widget du pack ; ☰ du mode deck qui entre en mode
   modèles.
4. **Régression** : une frappe en attente dans un sous-onglet caché est
   enregistrée par ✓ avant `close_templates`.
5. **i18n** : nouvelles clés dans les 20 langues, anciennes retirées
   (`i18n-keys.test.ts`).
