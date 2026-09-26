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
ui-status-builtin = intégré : l'enregistrer crée une copie
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

ui-toast-saved = Votre présentation est enregistrée
ui-toast-templates-saved = Modèles enregistrés : { $name }
ui-toast-select-zone = Sélectionnez une zone pour y placer : « { $text } »

## Toolbar

ui-toolbar-undo = Annuler
ui-toolbar-undo-label = Annuler : { $label }
ui-toolbar-redo = Rétablir
ui-toolbar-redo-label = Rétablir : { $label }
ui-toolbar-grid-hide = Masquer la grille
ui-toolbar-grid-show = Afficher la grille
ui-toolbar-grid-hide-title = Masquer la grille (⌘')
ui-toolbar-grid-show-title = Afficher la grille (⌘')
ui-toolbar-theme = Thème
ui-toolbar-theme-default = Par défaut
ui-toolbar-save-copy = Enregistrer une copie
ui-toolbar-save = Enregistrer
ui-toolbar-finish = Terminer
ui-toolbar-templates = Modèles
ui-toolbar-templates-title = Créer et modifier les modèles de slides
ui-toolbar-open = Ouvrir
ui-toolbar-add-label = Ajouter sur la slide
ui-toolbar-add = Ajouter
ui-add-kind = { $kind ->
    [card] Carte
    [shape] Forme
    [chart] Graphique
   *[text] Texte
}
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
ui-undo-edit-text = Modifier le texte
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
ui-stage-edit-text = Modifier le texte
ui-stage-hint-field = Entrée pour valider · Échap pour annuler
ui-stage-hint-block = ⌘ Entrée pour valider · une ligne par paragraphe, « - » pour une puce
ui-field = { $kind ->
    [title] Titre
    [subtitle] Sous-titre
    [kicker] Surtitre
    [text] Texte
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

## Slide notes

ui-notes = Outils de la slide
ui-stepper-less = { $label } : une de moins
ui-stepper-more = { $label } : une de plus

## Dictation

ui-dictation = Dictée
ui-dictation-start = Dicter
ui-dictation-start-title = Dicter (⌘D)
ui-dictation-finish = Terminer la dictée
ui-dictation-finish-title = Terminer (⌘D)
ui-dictation-cancel = Annuler
ui-dictation-preparing = Préparation de la dictée…
ui-dictation-heard-nothing = Rien entendu : parlez un peu plus près du micro
ui-dictation-to-cursor = au curseur
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
ui-dictation-auto-stop-label = Arrêter seul après un silence de
ui-dictation-silence = Durée du silence
ui-dictation-phrases-note = Seuls les passages parlés sont transcrits, phrase par phrase, pendant que vous parlez.
ui-dictation-download-now = Télécharger maintenant ({ $size })
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
ui-chart-add-row = + Ligne
ui-chart-add-column = + Colonne
ui-chart-series = Série { $n }

## Files

ui-file-open-prompt = Chemin du fichier à ouvrir (.oku-slides.pdf ou .json)
ui-file-save-prompt = Enregistrer sous (chemin complet)
