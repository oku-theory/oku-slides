# Interface de l’app (React). Chargé par ui/src/i18n.ts via la requête « messages ».
ui-loading = Chargement…

## Document

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · pack de layouts
   *[other] { $name } · pack de widgets
}
ui-title-rename = Renommer la présentation
ui-title-field = Titre de la présentation
ui-status-unsaved = modifications non enregistrées
ui-status-saved = enregistré
ui-status-never-saved = pas encore enregistré
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count } layout
       *[other] { $count } layouts
    }
   *[other] { $count ->
        [one] { $count } widget
       *[other] { $count } widgets
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } slide
   *[other] { $count } slides
} · { $status }
ui-confirm-quit-pack = Quitter sans enregistrer le pack modifié ?

## Toasts

ui-toast-copied = Copie enregistrée
ui-toast-field-gone = Le champ modifié n'existe plus : la saisie est perdue.
ui-toast-edit-replaced = Ce texte a changé pendant votre saisie : votre version l'a remplacé. Annuler rétablit l'autre.

## Toolbar

ui-toolbar-undo = Annuler
ui-toolbar-undo-label = Annuler : { $label }
ui-toolbar-redo = Rétablir
ui-toolbar-redo-label = Rétablir : { $label }
ui-divider-pile = Largeur de la pile de slides
ui-divider-notes = Largeur du fil de l'agent
ui-divider-code = Taille du code
ui-toolbar-theme = Thème
ui-toolbar-theme-embedded = Thème du fichier
ui-toolbar-finish = Terminer
ui-pack-deck-only = Enregistré dans cette présentation seulement
ui-toolbar-open = Ouvrir
ui-menu = Menu
ui-toolbar-add-label = Ajouter sur la slide
ui-toolbar-packs = Packs du deck
ui-toolbar-packs-update = Packs du deck — mises à jour disponibles
ui-toolbar-thumbs-show = Afficher les miniatures
ui-toolbar-thumbs-hide = Masquer les miniatures
ui-toolbar-remove-slide = Supprimer la slide
ui-toolbar-remove-hint = Slide { $n } · annulable avec ⌘Z

## Undo labels

ui-undo-add-widget = Ajouter « { $name } »
ui-undo-new-slide = Nouvelle slide : { $name }
ui-undo-move-slide = Déplacer la slide
ui-undo-remove-slide = Supprimer la slide
ui-undo-remove = Supprimer
ui-undo-paste = Coller
ui-undo-place-on-grid = Placer sur la grille
ui-undo-move-on-grid = Déplacer sur la grille
ui-undo-resize-on-grid = Redimensionner sur la grille
ui-undo-change-anchor = Changer l'accroche
ui-undo-move = Déplacer
ui-undo-resize = Redimensionner
ui-undo-edit-slide = Modifier la slide
ui-undo-rename-pack = Renommer le pack
ui-undo-rename-deck = Renommer la présentation

## Slide pile

ui-pile-slides = Slides
ui-pile-layouts = Layouts
ui-pile-add-slide = Nouvelle slide
ui-pile-drag-hint = Glisser pour déplacer · ⌥↑ ⌥↓
ui-slide-alt = Slide { $n }

## Stage

ui-stage-rendering = Rendu…
ui-stage-widget-pack = Ouvrez un widget de la pile, ou créez-en un.
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
    [chart] Graphique
    [table] Tableau
    [stack] Pile
   *[other] { $kind }
}
ui-grid-cell = col. { $cols } · lig. { $rows }
ui-ghost-zone = Zone

## Slide notes

ui-notes = Outils de la slide
ui-stepper-less = { $label } : une de moins
ui-stepper-more = { $label } : une de plus
ui-list-item = élément
ui-list-add = Ajouter { $item }
ui-list-remove = Supprimer { $item }
ui-list-move = Déplacer { $item }
ui-list-full = La liste est pleine ({ $max })
ui-undo-list-move = Déplacer { $item }
ui-agent-color-other = Autre couleur
ui-agent-thread = Agent
ui-agent-new-conversation = Nouvelle conversation
ui-agent-undo-turn = Annuler ce tour
ui-agent-detach = Ouvrir l'agent dans une fenêtre séparée
ui-agent-attach = Ramener l'agent dans la fenêtre principale
ui-agent-window-empty = Pas d'agent pour cet onglet
ui-agent-kept-in-draft = Rien où écrire : texte gardé dans le brouillon
ui-agent-busy-elsewhere = L'agent répond dans { $place }…
ui-agent-scope-slides = les slides
ui-agent-scope-atelier = l'atelier
ui-draft = Brouillon
ui-draft-send = Envoyer à l'agent
ui-draft-send-title = Envoyer à l'agent (⌘↩)
ui-draft-discard = Effacer
ui-agent-state-idle = Agent
ui-agent-state-starting = L'agent démarre…
ui-agent-state-working = L'agent travaille…
ui-agent-dock = En cours
ui-agent-dock-settings = Réglages
ui-agent-jump = ↓ Tour en cours
ui-agent-tool-input = Arguments
ui-agent-tool-output = Résultat
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
ui-dictation-downloading = Téléchargement du modèle { $name }
ui-dictation-download-progress = { $percent } % · { $received } sur { $total } · une seule fois
ui-dictation-listening = J'écoute · { $time }
ui-dictation-transcribing = Transcription de la dernière phrase…
ui-dictation-stop-in = Arrêt dans { $duration }
ui-dictation-speak = Parlez, le texte apparaît ici…
ui-dictation-auto-stop = arrêt après { $duration } de silence
ui-dictation-to-download = { $size } à télécharger
ui-dictation-settings = Réglages
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
ui-dictation-note-api = Service distant, compatible OpenAI
ui-dictation-api-ready = Clé trouvée
ui-dictation-api-key-missing = { $var } n'est pas définie
ui-dictation-api-url = Adresse de l'API
ui-dictation-api-model = Modèle
ui-dictation-api-key-env = Variable de la clé
ui-dictation-api-note = Chaque phrase est envoyée à ce service. La clé est lue dans la variable d'environnement au démarrage de la dictée ; une app ouverte depuis le Finder ne voit pas les variables du shell (utilisez launchctl setenv).
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } Go
ui-size-mb = { $value } Mo

## Layout picker

ui-picker-new-slide = Nouvelle slide
ui-picker-set-mine = { $name } · à moi
ui-picker-set-received = { $name } · reçu
ui-picker-keys = ← → pour parcourir · Entrée pour ajouter · Échap pour fermer
ui-picker-close = Fermer
ui-picker-list = Layouts
ui-widget-picker-filter = Filtrer par nom, libellé ou description
ui-widget-picker-empty = Aucun widget ne correspond à « { $query } ».
ui-widget-picker-list = Widgets
ui-widget-picker-keys = Flèches pour parcourir · Entrée pour ajouter · Échap pour fermer

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

## Table

ui-table-panel = Tableau
ui-table-options = Options du tableau
ui-table-header-row = Ligne d'en-tête
ui-table-header-column = Première colonne en en-tête
ui-table-striped = Lignes zébrées
ui-table-alignment = Alignement des colonnes
ui-table-align-of = Alignement de { $name }
ui-table-align = { $align ->
    [start] Début
    [center] Centre
    [end] Fin
   *[auto] Automatique
}
ui-table-column = Colonne { $n }
ui-table-no-data = Ce tableau n'a pas de données.
ui-show-as = Afficher en
ui-show-as-table = Afficher en tableau
ui-show-as-chart = Afficher en graphique

## Files

ui-file-open-prompt = Chemin du fichier à ouvrir (.oku-slides.pdf, .json ou .pptx)
ui-file-filter-presentations = Présentations
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = Enregistrer sous (chemin complet)

## Onglets

ui-tabs = Présentations ouvertes
ui-tab-new = Nouvel onglet
ui-tab-close = Fermer l'onglet
ui-tab-unsaved = Modifications non enregistrées
ui-tab-pack = Pack
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
ui-help = Aide
ui-help-caption = Slide { $index } sur { $count } · lecture seule
ui-slide-previous = Slide précédente
ui-slide-next = Slide suivante
ui-present = Présentation
ui-present-slide = Slide { $index } sur { $count }
ui-present-screen = Présenter en plein écran
ui-help-close = Fermer l'aide
ui-help-search = Rechercher dans l'aide
ui-help-search-placeholder = Rechercher…
ui-help-search-empty = Aucun résultat
ui-help-search-count = { $count ->
    [one] 1 résultat
   *[other] { $count } résultats
}
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
ui-widgets-field-bound = Lié à la présentation : en lecture ici
ui-widgets-source = Code source
ui-widgets-unsaved = Non enregistré
ui-widgets-in-error = Widgets en erreur
ui-widgets-none-in-error = Aucun widget en erreur
ui-widgets-section = Widgets
ui-widgets-open = Ouvrir le widget { $name }
ui-widgets-delete-named = Supprimer { $name }
ui-widgets-confirm-delete = Supprimer le widget « { $name } » ?
ui-widgets-rename-hint = Double-clic ou F2 pour renommer
ui-pack-tabs = Onglets du pack
ui-pack-layouts-tab = Layouts
ui-theme-tab = Thème
ui-theme-picker = Thème
ui-theme-name = Nom du thème
ui-theme-unsaved = Thème non enregistré
ui-widgets-close-tab = Fermer { $name }
ui-widgets-removed-by-agent = L'agent a supprimé le widget { $name }.

## Importer

ui-import-title = Importer une présentation PowerPoint
ui-import-mode = Mode d'import
ui-import-hifi = Copie fidèle
ui-import-hifi-note = Garde l'aspect de l'original, avec un thème et des layouts faits pour cette présentation.
ui-import-retrofit = Utiliser mes layouts
ui-import-retrofit-note = Refait les slides avec des packs de layouts et un thème de la bibliothèque.
ui-import-layout-packs = Packs de layouts
ui-import-theme-default = Thème par défaut
ui-import-no-packs = Aucun pack de layouts dans la bibliothèque.
ui-import-tune = Peaufiner avec l'agent
ui-import-tune-note = L'agent compare chaque slide à l'original et corrige les écarts.
ui-import-tune-note-retrofit = L'agent compare chaque slide à l'original et dessine des widgets pour les formes que vos layouts ne montrent pas.
ui-import-tune-unavailable = Aucun agent utilisable : { $reason }
ui-import-submit = Importer
ui-import-busy = Import en cours…
ui-update-available = Oku Slides { $version } est disponible.
ui-update-install = Installer et redémarrer
ui-update-see-release = Voir la version
ui-update-later = Plus tard
ui-update-installing = Téléchargement de la mise à jour… { $percent } %
ui-update-downloading = Téléchargement de la mise à jour…
ui-update-up-to-date = Oku Slides est à jour.

# Mots qui, seuls dans une consigne, ouvrent le sélecteur de thème ; séparés par des virgules, anglais compris.
ui-shortcut-theme = thème, thèmes, theme, themes
