# Interface de l’app (React). Chargé par ui/src/i18n.ts via la requête « messages ».
ui-loading = Chargement…

## Document

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · modèles
ui-title-rename = Renommer la présentation
ui-title-field = Titre de la présentation
ui-status-unsaved = modifications non enregistrées
ui-status-saved = enregistré
ui-status-never-saved = pas encore enregistré
ui-caption-templates = { $count ->
    [one] { $count } modèle
   *[other] { $count } modèles
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } slide
   *[other] { $count } slides
} · { $status }
ui-template-default-name = Modèle
ui-template-variant-name = { $name } (variante)
ui-confirm-quit-templates = Quitter sans enregistrer les modèles modifiés ?

## Toasts

ui-toast-templates-saved = Modèles enregistrés : { $name }
ui-toast-moved = Présentation déplacée
ui-toast-copied = Copie enregistrée
ui-toast-select-zone = Sélectionnez une zone pour y placer : « { $text } »
ui-toast-field-gone = Le champ modifié n'existe plus : la saisie est perdue.

## Toolbar

ui-toolbar-undo = Annuler
ui-toolbar-undo-label = Annuler : { $label }
ui-toolbar-redo = Rétablir
ui-toolbar-redo-label = Rétablir : { $label }
ui-toolbar-grid-hide = Masquer la grille
ui-toolbar-grid-show = Afficher la grille
ui-toolbar-ghosts-hide = Masquer les fantômes
ui-toolbar-ghosts-show = Afficher les fantômes
ui-divider-pile = Largeur de la pile de slides
ui-divider-notes = Largeur du fil de l'agent
ui-toolbar-theme = Thème
ui-toolbar-theme-default = Par défaut
ui-toolbar-save-copy = Enregistrer une copie
ui-toolbar-finish = Terminer
ui-toolbar-templates = Modèles
ui-toolbar-open = Ouvrir
ui-merge = Fusionner avec un PDF…
ui-menu = Menu
ui-menu-move = Déplacer…
ui-language = Langue
ui-language-system = Langue du système
ui-scheme = Apparence
ui-scheme-system = Système
ui-scheme-light = Clair
ui-scheme-dark = Sombre
ui-toolbar-add-label = Ajouter sur la slide
ui-add-kind = { $kind ->
    [card] Carte
    [shape] Forme
    [chart] Graphique
   *[text] Texte
}
ui-add-widget = Widget du pack
ui-toolbar-remove-template = Supprimer le modèle
ui-toolbar-remove-slide = Supprimer la slide
ui-toolbar-remove-hint = Slide { $n } · annulable avec ⌘Z

## Undo labels

ui-undo-add = { $kind ->
    [card] Ajouter une carte
    [shape] Ajouter une forme
    [chart] Ajouter un graphique
   *[text] Ajouter un texte
}
ui-undo-add-widget = Ajouter « { $name } »
ui-undo-new-template = Nouveau modèle : { $name }
ui-undo-new-slide = Nouvelle slide : { $name }
ui-undo-grid = Grille { $columns } × { $rows }
ui-undo-templates = Modèles : { $name }
ui-undo-move-template = Déplacer le modèle
ui-undo-move-slide = Déplacer la slide
ui-undo-remove-slide = Supprimer la slide
ui-undo-remove = Supprimer
ui-undo-cut = Couper
ui-undo-paste = Coller
ui-undo-place-on-grid = Placer sur la grille
ui-undo-move-on-grid = Déplacer sur la grille
ui-undo-resize-on-grid = Redimensionner sur la grille
ui-undo-change-anchor = Changer l'accroche
ui-undo-move = Déplacer
ui-undo-resize = Redimensionner
ui-undo-edit-slide = Modifier la slide
ui-undo-dictation = Dictée
ui-undo-rename-set = Renommer l'ensemble
ui-undo-rename-deck = Renommer la présentation
ui-undo-theme = Changer de thème
ui-undo-rename-template = Renommer le modèle
ui-undo-describe-template = Décrire le modèle
ui-undo-chart-kind = Changer le type du graphique
ui-undo-chart-data = Modifier les données
ui-undo-chart-color = Colorer une série
ui-undo-chart-stack = { $stacked ->
    [yes] Empiler les séries
   *[no] Séparer les séries
}

## New elements

ui-new-text = Nouveau texte
ui-new-card-title = Titre de la carte
ui-new-card-detail = Détail

## Slide pile

ui-pile-slides = Slides
ui-pile-templates = Modèles
ui-pile-add-slide = + Nouvelle slide
ui-pile-add-template = + Nouveau modèle
ui-pile-drag-hint = Glisser pour déplacer · ⌥↑ ⌥↓
ui-slide-alt = Slide { $n }

## Stage

ui-stage-rendering = Rendu…
ui-stage-elements = Éléments de la slide
ui-stage-fields = Champs de la slide
ui-stage-edit-text = Modifier le texte
ui-stage-hint-plain = Entrée ou Tab pour passer au suivant · Échap pour annuler
ui-stage-hint-rich = Tab pour passer au suivant · ⌘ Entrée pour valider · « - » pour une puce · ⌘B gras, ⌘I italique, ⌘K lien
ui-field = { $kind ->
    [title] Titre
    [subtitle] Sous-titre
    [kicker] Surtitre
    [text] Texte
    [body] Corps
    [author] Auteur
    [date] Date
    [organization] Organisation
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Texte
    [card] Carte
    [image] Image
    [shape] Forme
    [group] Groupe
   *[other] { $kind }
}
ui-grid-cell = col. { $cols } · lig. { $rows }
ui-ghost-zone = Zone

## Slide notes

ui-notes = Outils de la slide
ui-stepper-less = { $label } : une de moins
ui-stepper-more = { $label } : une de plus
ui-agent-color-other = Autre couleur
ui-agent-thread = Agent
ui-agent-new-conversation = Nouvelle conversation
ui-draft = Brouillon
ui-draft-insert = Insérer
ui-draft-insert-title = Insérer tel quel (⌘⇧↩)
ui-draft-send = Envoyer à l'agent
ui-draft-send-title = Envoyer à l'agent (⌘↩)
ui-draft-discard = Effacer
ui-agent-state-idle = Agent
ui-agent-state-starting = L'agent démarre…
ui-agent-state-working = L'agent travaille…
ui-agent-dock = En cours
ui-agent-dock-settings = Réglages
ui-agent-jump = ↓ Tour en cours
ui-agent-sheet = Question de l'agent
ui-agent-sheet-keys = 1–4 choisir · Entrée valider · E retoucher · Échap abandonner
ui-agent-variants = Variantes
ui-agent-drawer-keys = Clic ou 1–4 · Échap ferme
ui-agent-confirm = Confirmation de l'agent

## Dictation

ui-dictation = Dictée
ui-dictation-start = Dicter
ui-dictation-start-title = Dicter (⌘D)
ui-dictation-type = Écrire à l'agent
ui-dictation-type-title = Écrire à l'agent (⌘K)
ui-dictation-agent-stop = Arrêter l'agent (⌘⇧D ou Échap)
ui-dictation-agent-working = L'agent travaille…
ui-dictation-agent-working-hint = ⌘⇧D ou Échap l'arrête
ui-dictation-finish = Terminer la dictée
ui-dictation-finish-title = Terminer (⌘D)
ui-dictation-cancel = Annuler
ui-dictation-preparing = Préparation de la dictée…
ui-dictation-heard-nothing = Rien entendu : parlez un peu plus près du micro
ui-dictation-to-cursor = au curseur
ui-dictation-to-draft = dans le brouillon
ui-dictation-to-field = remplace { $field }
ui-dictation-to-block = à la suite du bloc sélectionné
ui-dictation-new-block = nouveau bloc de texte
ui-dictation-select-zone = sélectionnez une zone
ui-dictation-downloading = Téléchargement du modèle { $name }
ui-dictation-download-progress = { $percent } % · { $received } sur { $total } · une seule fois
ui-dictation-listening = J'écoute · { $time }
ui-dictation-transcribing = Transcription de la dernière phrase…
ui-dictation-stop-in = Arrêt dans { $duration }
ui-dictation-speak = Parlez, le texte apparaît ici…
ui-dictation-auto-stop = arrêt après { $duration } de silence
ui-dictation-to-download = { $size } à télécharger
ui-dictation-settings = Réglages de la dictée
ui-dictation-model-title = Modèle de reconnaissance
ui-dictation-model-intro = Whisper, sur cet ordinateur (CTranslate2). Tous les modèles comprennent plusieurs langues.
ui-dictation-note-turbo = Recommandé, rapide et précis
ui-dictation-note-large = Le plus précis, plus lent
ui-dictation-note-base = Léger, pour les petites machines
ui-dictation-installed = installé
ui-dictation-not-installed = à télécharger
ui-dictation-language = Langue parlée
ui-dictation-language-auto = Détection automatique
ui-dictation-note-apple = Le plus rapide, intégré au système
ui-dictation-model-unavailable = Langue non prise en charge
ui-dictation-model-intro-apple = Apple, intégré au système, ou Whisper, sur cet ordinateur (CTranslate2). Tous comprennent plusieurs langues.
ui-dictation-language-auto-resolved = Automatique ({ $language })
ui-dictation-to-download-language = Langue à télécharger
ui-dictation-auto-stop-label = Arrêter seul après un silence de
ui-dictation-silence = Durée du silence
ui-dictation-phrases-note = Seuls les passages parlés sont transcrits, phrase par phrase, pendant que vous parlez.
ui-dictation-download-now = Télécharger maintenant ({ $size })
ui-dictation-download-language = Télécharger la langue
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } Go
ui-size-mb = { $value } Mo

## Template picker

ui-picker-new-slide = Nouvelle slide
ui-picker-new-template = Nouveau modèle, à partir de…
ui-picker-set = Modèles
ui-picker-set-mine = { $name } · à moi
ui-picker-keys = ← → pour parcourir · Entrée pour ajouter · Échap pour fermer
ui-picker-close = Fermer
ui-picker-previous = Modèle précédent
ui-picker-next = Modèle suivant
ui-picker-list = Modèles de slides

## Chart

ui-chart-panel = Graphique
ui-chart-kinds = Type de graphique
ui-chart-kind = { $kind ->
    [line] Lignes
    [pie] Secteurs
    [scatter] Nuage de points
    [radar] Radar
    [waterfall] Cascade
   *[bar] Barres
}
ui-chart-no-data = Ce graphique n'a pas de tableau de données.
ui-chart-options = Options du graphique
ui-chart-stack = Empilé
ui-chart-waterfall-hint = Cochez Total pour une barre de total.
ui-chart-colors = Couleurs des séries
ui-chart-color-of = Couleur de { $name }
ui-chart-color-auto = Automatique
ui-chart-color-pick = Couleur { $n }
ui-chart-column-name = Nom de la colonne { $col }
ui-chart-remove-column = Supprimer la colonne
ui-chart-cell = Ligne { $row }, colonne { $col }
ui-chart-remove-row = Supprimer la ligne
ui-chart-add-row = Ajouter une ligne
ui-chart-add-column = Ajouter une colonne
ui-chart-series = Série { $n }

## Files

ui-file-open-prompt = Chemin du fichier à ouvrir (.oku-slides.pdf ou .json)
ui-file-save-prompt = Enregistrer sous (chemin complet)

## Onglets

ui-tabs = Présentations ouvertes
ui-tab-new = Nouvel onglet
ui-tab-close = Fermer l'onglet
ui-tab-unsaved = Modifications non enregistrées
ui-tab-templates = Modèles
ui-tab-move-new-window = Déplacer vers une nouvelle fenêtre
ui-tab-move-to = Déplacer vers la fenêtre { $n }
ui-tab-close-others = Fermer les autres onglets
ui-close-cancel = Annuler
ui-disk-replaced-question = { $file } a été remplacé sur le disque par une autre présentation.
ui-disk-replaced-overwrite = Écraser
ui-disk-replaced-save-as = Enregistrer sous…
ui-same-deck-question = { $file } est une version de la présentation ouverte dans l'onglet « { $title } ».
ui-same-deck-merge = Fusionner dans cet onglet
ui-same-deck-apart = Ouvrir à part
ui-history = Historique
ui-history-version = Version du { $date }
ui-history-restore = Restaurer
ui-history-back = Revenir
ui-history-anonymous = Anonyme
ui-history-slider = Versions de la présentation
ui-history-removed-since = supprimée depuis
ui-history-changed-since = modifiée depuis
ui-history-added-since = slides ajoutées depuis : { $count }
ui-pack-notice-close = Fermer
ui-widgets-files = Fichiers de widgets
ui-widgets-common = Commun
ui-widgets-new = Nouveau widget
ui-widgets-name = Nom du widget (minuscules, chiffres et tirets)
ui-widgets-name-taken = Ce nom est déjà pris
ui-widgets-name-letter = Le nom doit commencer par une lettre
ui-widgets-valid = Valide
ui-widgets-errors = Erreurs
ui-widgets-empty = Ce pack n'a pas encore de widget.
ui-widgets-preview = Aperçu du widget
ui-widgets-no-preview = Pas d'aperçu
ui-widgets-source = Code source
ui-widgets-unsaved = Non enregistré
ui-widgets-in-error = Widgets en erreur
ui-widgets-none-in-error = Aucun widget en erreur
ui-widgets-section = Widgets
ui-widgets-edit = Éditer les widgets
ui-widgets-open = Ouvrir le widget { $name }
ui-widgets-delete-named = Supprimer { $name }
ui-widgets-confirm-delete = Supprimer le widget « { $name } » ?
ui-widgets-rename-hint = Double-clic ou F2 pour renommer
ui-widgets-tabs = Onglets des modèles
ui-widgets-templates-tab = Modèles
ui-widgets-close-tab = Fermer { $name }
ui-widgets-removed-by-agent = L'agent a supprimé le widget { $name }.
