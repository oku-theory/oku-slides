# Agent : surfaces, confirmations, fil.

agent-instructions = Tu modifies une présentation Oku Slides pour l'utilisateur, qui te parle. N'agis que par les outils oku-slides. Lis le deck avec deck_outline, slide et render_slide avant de le changer ; change-le avec apply (tout s'annule). Quand une décision revient à l'utilisateur, demande avec ask_choice, ask_confirm ou ask_rewrite ; propose des mises en page avec show_variants et des réglages en direct avec show_params. Termine chaque tour par report : un titre d'une ligne, 1 à 4 faits courts, ton explication dans why, et 0 à 4 suites dans actions. Ton texte libre n'est montré qu'en note de progression d'une ligne : écris-le ainsi, bref, dans la langue de l'utilisateur. N'utilise render que si aucun autre outil ne convient. Chaque message commence par un bloc <oku-slides-context> qui décrit ce que voit l'utilisateur. Avant d'ajouter un élément, lis l'outil widgets pour connaître les genres disponibles et leurs paramètres. Pour changer la grille d'une slide, utilise l'op set_grid : les éléments placés suivent.

agent-instructions-widgets = Créer un widget : un widget est un fichier Typst du pack de modèles ; utilise widget_sources, widget_source, set_widget, remove_widget et preview_widget, en mode deck comme en mode modèles. Forme d'un fichier kpi : #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, puis #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …) ; le nom déclaré est celui du fichier. Paramètres : descripteurs p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Textes éditables : field(ctx, "value", args.value), empilés par fields-stack. Couleurs et tailles : jetons du thème (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), jamais de valeurs en dur. Code partagé : le fichier _commun, importé par #import "_commun.typ": …. Boucle : écrire avec set_widget, lire le statut et ses diagnostics (ligne, colonne), regarder preview_widget, corriger. Pour l'insérer dans le deck : add_element avec kind = nom du widget. Un widget de slide déclare category: "slide" (widget("nom", category: "slide", params: …)) ; sinon c'est un élément. Dans un descripteur, label: et default: sont facultatifs (p.plain(), p.plain(label: "Titre", default: "…")). En mode modèles, règle les valeurs d'essai d'un widget avec set_widget_trial ; preview_widget sans paramètres les utilise, et l'utilisateur les voit dans les réglages du widget. Listes : p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) pour des éléments répétés (frise, KPI, équipe) ; types de colonne plain, rich, number, flag, choice, asset. Rends chaque item avec list-item(ctx, "etapes", e, c => field(c, "date", e.date)) ; pour un champ vide, utilise field(c, name, box(width: 3em, hide[0]), empty: true) afin qu'il reste éditable. Ne simule jamais une liste avec des paramètres numérotés (date1, date2…). Une liste absente est vide ; default n'amorce qu'un nouvel objet. Les items ont des ids stables : modifie-les avec edit_list(target, param, change) (change : insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value) ; tu fournis l'id), ou remplace toute la liste avec set_param (null la vide). Quand add_element ou add_slide crée un widget sans que tu donnes ses items, passe "defaults": true pour amorcer les défauts de liste ; jamais en réinsérant ou en déplaçant.

agent-surface-cancel = Annuler
agent-surface-confirm = Confirmer
agent-surface-confirm-danger = Oui, le faire
agent-surface-rewrite = Reformulation
agent-surface-pick = Choisir { $n }
agent-surface-keep = Garder l'original
agent-surface-edit = Retoucher avant d'appliquer
agent-surface-close = Fermer
agent-surface-variants = Variantes
agent-surface-permission-always = pour toujours
agent-surface-permission-title = L'agent demande une permission
agent-undo-turn = Agent : { $text }

agent-tools-unavailable = Les outils de l'agent n'ont pas pu démarrer.

agent-undo-apply = Modification de l'agent
agent-undo-variant = Variante : { $label }
agent-undo-param = Réglage : { $label }
agent-undo-rewrite = Reformulation

agent-confirm-save = Enregistrer la présentation sous { $path } ?
agent-confirm-save-over = Remplacer { $path } ? Le fichier existant sera écrasé.
agent-confirm-open = Ouvrir { $path } ?
agent-open-elsewhere = Ce fichier est déjà ouvert dans un autre onglet.
agent-confirm-new = Commencer une nouvelle présentation ?
agent-confirm-new-dirty = Commencer une nouvelle présentation ? Les modifications non enregistrées seront perdues.
agent-confirm-save-templates = Enregistrer l'ensemble de modèles « { $name } » ?
agent-confirm-delete-set = Supprimer l'ensemble de modèles « { $name } » ? C'est définitif.
agent-confirm-setting = Changer { $key } : { $old } → { $new } ?

agent-not-found = Agent introuvable : { $command }. Réglage [agent] command.
agent-no-http = Cet agent ne sait pas se connecter aux outils d'Oku Slides.
agent-stopped = L'agent s'est arrêté.
agent-failed = L'agent a refusé la demande : { $message }
agent-tool-failed = L'outil a échoué de façon inattendue.

agent-turn-tools = { $tools ->
   [one] { $tools } outil
  *[other] { $tools } outils
}
agent-turn-notes = { $notes ->
   [one] { $notes } note
  *[other] { $notes } notes
}
agent-turn-secs = { $secs } s
agent-turn-stopped = Arrêté.
agent-turn-interrupted = Interrompu.
agent-memory-lost = L'agent ne se souvient plus de cette conversation : il repart de zéro.
agent-turn-max-tokens = L'agent a atteint sa limite de longueur.
agent-turn-max-turns = L'agent a atteint sa limite d'étapes.
agent-turn-refusal = L'agent a refusé.
agent-turn-ended = Tour terminé ({ $stop }).
agent-choice-cancelled = Question annulée
agent-choice-answered = ✓ { $answer }
agent-report-why = Pourquoi

agent-error-file = Le fichier n'a pas pu être lu ou écrit.
agent-error-unexpected-action = Action inattendue : { $action }.
agent-error-unknown-surface = Cette carte n'est plus ouverte.
agent-error-expired = Les actions d'un tour passé ont expiré.
agent-error-incomplete = L'action de cette carte est incomplète.
agent-error-no-field = Ce réglage n'existe plus.
agent-error-no-variant = Cette variante n'existe plus.
agent-error-arguments = Les arguments de l'outil sont invalides ({ $detail }) : vérifiez-les d'après son schéma.
agent-error-unknown-tool = Outil inconnu : { $name }.
agent-error-denied = L'utilisateur a refusé.
agent-error-no-turn = Aucun tour en cours : { $tool } ne s'affiche que pendant un tour.
agent-error-turn-over = Le tour de cet appel est terminé : rien n'a été affiché.
agent-error-anchor-scope = Une question d'une conversation de widget ne s'ancre pas sur une slide : posez-la sans ancre.
agent-error-no-file = La présentation n'a pas encore de fichier : donnez un chemin absolu.
agent-error-path-relative = Le chemin doit être absolu.
agent-error-path-parent = Le chemin ne doit pas contenir « .. ».
agent-error-setting-locked = Le réglage { $key } ne peut pas être changé par l'agent.
agent-error-setting = Le réglage { $key } n'a pas pu être changé : vérifiez sa valeur.
agent-error-rewrite-text = La reformulation est revenue sans texte.
agent-error-paragraphs = Les paragraphes de la reformulation sont invalides.
