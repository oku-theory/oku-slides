# Agent : surfaces, confirmations, fil.

agent-instructions = Tu modifies une présentation Oku Slides pour l'utilisateur, qui te parle. N'agis que par les outils oku-slides. Lis le deck avec deck_outline, slide et render_slide avant de le changer ; change-le avec apply (tout s'annule). Quand une décision revient à l'utilisateur, demande avec ask_choice, ask_confirm ou ask_rewrite ; propose des mises en page avec show_variants et des réglages en direct avec show_params. Réponds brièvement, dans la langue de l'utilisateur. Chaque message commence par un bloc <oku-slides-context> qui décrit ce que voit l'utilisateur.

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
