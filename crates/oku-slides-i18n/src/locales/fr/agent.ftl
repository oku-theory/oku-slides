# Agent : surfaces, confirmations, fil.

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
agent-confirm-edit-theme = Enregistrer une nouvelle version du thème « { $name } » ?
agent-confirm-copy-theme = Enregistrer une copie modifiée du thème intégré « { $name } » ?
agent-confirm-setting = Changer { $key } : { $old } → { $new } ?

agent-not-found = Agent introuvable : { $command }. Réglage [agent.presets].
agent-spawn-failed = L'agent n'a pas pu démarrer : { $command }. Réglage [agent.presets].
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
agent-error-pack-id = Ce n'est pas l'id d'un ensemble de modèles : appelez templates pour lister les ensembles et leurs ids.
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
agent-error-draft-too-long = Le brouillon est trop long.
agent-error-draft-origin = Brouillon refusé : vue inconnue.
agent-error-draft-no-agent = L'agent de cet onglet n'a pas encore démarré : brouillon refusé.
agent-error-unknown-agent = Cet agent n'est plus dans la configuration.
agent-error-agent-locked = Le choix de l'agent est imposé par l'environnement.
agent-error-agent-not-saved = Le choix de l'agent n'a pas pu être enregistré.
agent-picker-title = Agent
agent-picker-busy = Changer d'agent interrompra le tour en cours.
agent-picker-locked = Choix imposé par l'environnement (OKU_SLIDES_AGENT__SELECTED).
agent-tune-prompt = Peaufiner la présentation importée
