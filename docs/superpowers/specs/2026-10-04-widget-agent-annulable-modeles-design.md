# Écriture d'un widget annulable en mode modèles — design

Date : 2026-10-04 · Tâche : t-504

## Problème

En mode modèles, quand l'agent modifie le code d'un widget (`set_widget`),
le bouton annuler reste grisé. Un tour de l'agent doit être **une** étape
d'annulation, quel que soit le nombre d'appels MCP qu'il fait.

Cause : `adopt_session_version` (`templates_mode.rs`) fait passer le
document des modèles à la nouvelle version du pack par `apply_silent`.
Aucune étape n'est empilée sur ce document ; le groupe du tour
(`begin_turn`) n'a rien à attraper. Seul le deck mis de côté (`stash`)
reçoit une étape, invisible tant qu'on reste en mode modèles. En mode
deck, la même écriture passe par `apply(SetTemplates)` et rejoint le tour.

Reproduit par `set_widget_by_agent_in_templates_turn_can_be_undone`
(`widgets_edit_tests.rs`).

## Décisions

1. **Une écriture de widget en mode modèles est une étape du document des
   modèles**, comme en mode deck : `apply(SetTemplates)` sous le libellé
   `widgets-edit-label`, avec la clé de séance `adopt_merge_key(id, parent)`.
   Dans un tour, le groupe l'emporte : tout le tour est une étape. Hors
   tour, des écritures successives sans autre action entre elles fusionnent
   en une étape (comme en mode deck).
2. **L'enregistrement des modèles (`save_templates`) reste silencieux** :
   il fige l'état, il ne le change pas (les modèles édités sont déjà des
   étapes). Hors périmètre de cette tâche.
3. **La version éditée se relit dans le document**, plus dans un champ :
   `TemplateMode::base` = la version de `deck.templates` du document des
   modèles ; `session_version` = cette version si elle n'est pas `parent`,
   sinon `None`. Un annuler ou un rétablir les remet donc d'aplomb sans code
   propre. `TemplateMode` reste la vue envoyée à l'interface (`editing`),
   calculée à la demande ; seuls `set`, `name`, `builtin` et `parent` sont
   gardés.
4. **Versions remplacées : la règle du deck mis de côté, appliquée aux deux
   documents.** Écriture fusionnée dans l'étape d'avant → la version
   remplacée n'est plus atteignable, elle quitte les sources. Nouvelle étape
   → annuler y revient : elle reste dans les sources et revient dans la
   bibliothèque (`add_version`) si `replace_version` l'en avait retirée.
5. **Le deck mis de côté suit le document des modèles.** Après un annuler
   ou un rétablir en mode modèles qui change la version du pack, le deck
   mis de côté passe à cette version (`apply` avec la même clé de séance) :
   à la fermeture, le deck embarque la version que l'utilisateur voyait.
6. **La bibliothèque garde la version créée** à l'annulation (comme en mode
   deck) : annuler ne change pas sa version courante.
7. **Interface : rien à ajouter.** `useTemplateWidgets` relit les fichiers
   quand `editing.base` change (`revision`) ; avec la décision 3, un annuler
   recharge les onglets de widgets.

## Comportement attendu

- Tour de l'agent en mode modèles avec une ou plusieurs écritures de
  widgets (et éventuellement des éditions de modèles) → un seul ⌘Z, libellé
  « Agent : … », qui ramène le code de chaque widget, et les modèles, à leur
  état d'avant le tour.
- ⌘⇧Z rétablit le tour entier.
- Après annuler, fermer les modèles : le deck embarque la version d'avant
  le tour.
- Écriture de l'utilisateur sur un widget (onglet de code, enregistrée) :
  devient aussi une étape du document des modèles. L'historique propre de
  l'éditeur de code (CodeMirror) reste celui de la frappe dans l'onglet.

## Tests (RED d'abord)

Session (`widgets_edit_tests.rs`, `templates_mode_tests.rs`) :
- tour avec deux `set_widget_by_agent` → `can_undo`, une étape, libellé du
  tour ; `undo` → source du widget d'avant le tour, `editing.base` = parent ;
- `redo` → source du tour, `editing.base` = version du tour ;
- annuler puis fermer les modèles → le deck embarque la version d'avant ;
- annuler, puis nouvelle écriture → nouvelle version, l'ancienne reste dans
  la bibliothèque si une étape y mène encore ;
- deux écritures hors tour sans action entre elles → une étape.

MCP (`tests/widgets.rs`) : `set_widget` deux fois dans un tour en mode
modèles → `undo_label` du tour.

Les tests existants de séance de modèles (`session_version`, une version
par séance) restent verts.
