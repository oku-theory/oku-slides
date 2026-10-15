cli-reading = lecture de { $path }
cli-invalid-deck = deck invalide : { $path }
cli-deck-valid = deck valide : { $slides } slides
cli-deck-issue = problème : { $issue }
cli-rendered = { $pages } pages → { $out } (compilation { $compile }, SVG { $svg })
cli-asset-added = asset { $path } → { $id }
cli-saved = { $path } : { $pages } pages, PDF { $pdf } Ko, sources { $sources } Ko
cli-opened = { $path } : { $slides } slides, { $assets } assets, { $fonts } polices
cli-extracted = extrait dans { $dir }
cli-stripped = { $path } : copie sans sources
cli-pdf-modified = le PDF a été modifié par un autre outil après Oku Slides
cli-iterations = --iterations doit être > 0
cli-no-block = le deck n'a aucun bloc à modifier
cli-blob-missing = blob des sources introuvable
cli-roundtrip-mismatch = le deck relu diffère du deck enregistré
cli-bench-world = création du monde (polices)       { $time }
cli-bench-first = première compilation              { $time }  ({ $pages } pages)
cli-bench-same = recompilation sans changement     { $time }
cli-bench-edited-text = Texte modifié { $n }
cli-bench-edit = recompilation après set_text      { $time }
cli-bench-svg = SVG de toutes les slides          { $time }  ({ $each } par slide)
cli-bench-pdf = export PDF                        { $time }  ({ $size } Ko)
cli-bench-embed = ajout du blob des sources         { $time }
cli-bench-extract = relecture du blob                 { $time }
cli-bench-roundtrip = aller-retour du deck              identique
cli-bench-history = cran d'historique                   { $mean } ms en moyenne, p50 { $p50 } ms
cli-serving = Oku Slides (dev) sur http://{ $host }:{ $port }
cli-models-dir = modèles de dictée : { $dir }
cli-stopped = serveur arrêté
cli-not-found = introuvable
cli-bad-request = requête invalide : { $reason }
cli-sse-full = trop de flux d'évènements déjà ouverts
cli-forbidden = requête refusée : elle ne vient pas de l'interface locale
cli-exposed = écoute sur { $host } : le serveur, sans authentification, est joignable depuis le réseau ; tout client qui envoie un en-tête Host local est servi. Gardez [serve] host sur 127.0.0.1
cli-imported = { $path } : { $slides ->
       [one] { $slides } slide
      *[other] { $slides } slides
    }, { $pages ->
       [one] { $pages } page
      *[other] { $pages } pages
    }, { $losses ->
       [one] { $losses } perte
      *[other] { $losses } pertes
    }, { $requests ->
       [one] { $requests } demande de widget
      *[other] { $requests } demandes de widget
    }
cli-import-label = Import PowerPoint
cli-import-pack-required = --pack est obligatoire en mode retrofit : un id, un dossier ou un nom de pack
cli-import-unknown-pack = pack inconnu : { $pack }
cli-import-bad-pack = pack illisible { $pack } : son pack.json ne décrit pas un pack de modèles
cli-import-write-failed = impossible d'écrire le deck importé { $path }
cli-import-hifi-no-target = --pack et --theme sont propres au mode retrofit : l'import haute fidélité (--mode hifi) génère son propre pack et son propre thème
cli-import-probe-mismatch = le pack généré ne se sonde pas comme il est déclaré (widget { $widget }) : c'est un bogue de l'import
cli-import-probe-limit = le pack généré dépasse les bornes de la sonde (widget { $widget }) : relevez max_widgets, max_widget_kib ou probe_budget_ms dans la section [render] de la configuration
cli-import-reference-hifi-only = --reference appartient au mode hifi : un import retrofit n'est pas comparé à un rendu de référence
cli-import-bad-threshold = score_threshold de la section [import] doit être un nombre supérieur ou égal à zéro, pas { $value }
cli-import-scored = Écart visuel à { $witness } : { $mean } en moyenne, { $above ->
    [0] aucune slide
    [one] { $above } slide
   *[other] { $above } slides
} au-delà de { $threshold }
cli-import-unscored = { $reason }

cli-about = Rendu et stockage des decks Oku Slides
cli-arg-fonts = Dossier de polices supplémentaires (TTF/OTF) ; défaut : paths.fonts des réglages
cli-cmd-check = Vérifie un deck
cli-arg-deck = Deck (JSON)
cli-cmd-render = Rend chaque slide en SVG, avec les boîtes des éléments
cli-arg-input = deck.json ou fichier .oku-slides.pdf
cli-arg-out = Dossier de sortie
cli-arg-theme = Remplace le thème du deck : un thème intégré ou de la bibliothèque, par son nom
cli-cmd-save = Enregistre un deck en .oku-slides.pdf (PDF + blob des sources)
cli-arg-output = Fichier à écrire
cli-arg-assets = Dossier d'assets : chaque fichier devient un asset sha256:…
cli-arg-embed-fonts = Joint aussi les polices de --fonts aux sources
cli-cmd-open = Rouvre un .oku-slides.pdf : extrait le deck et les assets
cli-arg-extract = Dossier où écrire deck.json, assets/ et fonts/
cli-cmd-strip = Copie d'un .oku-slides.pdf, sans le blob des sources
cli-cmd-bench = Mesure les temps de compilation et d'export
cli-arg-iterations = Mesures par étape
cli-cmd-schema = Affiche le schéma JSON des opérations
cli-cmd-serve = Serveur de développement : l'interface dans un navigateur, sans Tauri
cli-arg-serve-input = Deck à ouvrir (.oku-slides.pdf ou .json)
cli-arg-port = Port d'écoute ; défaut : serve.port des réglages
cli-arg-ui = Dossier de l'interface compilée (ui/dist)
cli-cmd-import = Importe une présentation PowerPoint (.pptx) en deck .oku-slides.pdf
cli-arg-pptx = Présentation PowerPoint (.pptx)
cli-arg-mode = retrofit : le contenu sur un pack et un thème existants ; hifi : copie fidèle, sur un pack et un thème générés pour elle
cli-arg-pack = Pack cible : id, dossier de pack (pack.json, widgets/) ou nom
cli-arg-import-theme = Thème cible : un thème intégré ou de la bibliothèque, par son id ou son nom ; défaut : le thème par défaut
cli-arg-report = Écrit aussi le rapport d'import (JSON) : pertes, replis, couverture
cli-arg-no-system-fonts = En mode hifi, ne cherche pas les polices de la présentation parmi celles du système
cli-arg-reference = En mode hifi, comparer chaque slide à un rendu de référence de la présentation (auto, le défaut) ou non (none)
