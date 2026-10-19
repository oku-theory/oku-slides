pdf-unreadable = PDF illisible
pdf-unsupported-version = Ce PDF vient d'une version plus récente d'Oku Slides
pdf-corrupted = Les sources de ce PDF sont abîmées
pdf-write = Le PDF n'a pas pu être écrit

render-compile = Le rendu du deck a échoué
render-pdf = L'export PDF a échoué
render-no-page = Cette page n'existe pas
render-setup = Le rendu n'a pas pu démarrer
render-image = L'image de la slide n'a pas pu être produite
render-missing-image = image manquante
diagnostic-error = erreur
diagnostic-warning = avertissement
diagnostic = { $level } ({ $place }) : { $message }
diagnostic-unplaced = { $level } : { $message }
diagnostic-hint = indice : { $hint }
render-chart-failed = graphique impossible à dessiner

registry-bad-pack-id = Identifiant de pack de modèles invalide : { $id }
registry-base-pack-broken = Le pack de widgets intégré est endommagé ({ $file }) ; réinstallez l'app.
registry-duplicate-pack = Deux packs de widgets de la présentation partagent { $pack }.
registry-compile = Les widgets du pack de modèles ne compilent pas
registry-missing = Le pack de modèles ne déclare pas ses widgets
render-timeout = Le rendu a pris plus de { $seconds } s ; il a été interrompu
render-out-of-memory = Le rendu a demandé plus de { $mib } Mio de mémoire ; il a été interrompu
render-worker-lost = Le processus de rendu s'est arrêté
render-frame = Message illisible du processus de rendu
render-page-too-large = Cette slide est trop grande pour l'aperçu (plus de { $mib } Mio, souvent à cause d'une très grande image). Elle s'exporte encore en PDF ou en PNG.
render-stale = Le rendu a été remplacé ; réessayez
render-pack-disabled = Les widgets du pack « { $pack } » sont désactivés pour cette session
render-protocol = Le processus de rendu appartient à une autre version d'Oku Slides ; relancez l'application
render-remote = Le processus de rendu n'a pas pu traiter la demande
registry-schema-remote = Registre des widgets invalide : { $message }
probe-not-probed = Pas encore vérifié : le temps accordé aux widgets du pack est écoulé. Il le sera au prochain rendu.
probe-too-many-files = Le pack a plus de { $max } fichiers de widgets : celui-ci n'est pas chargé.
probe-too-large = Plus de { $max } Kio : non chargé.

import-unreadable = Cette présentation est illisible
import-not-a-presentation = Ce fichier n'est pas une présentation PowerPoint
import-protected = Cette présentation est chiffrée ou à l'ancien format .ppt : enregistrez-la d'abord en .pptx
import-too-large = Cette présentation est trop volumineuse pour être importée
import-output-too-large = Le deck obtenu dépasserait la taille admise pour un import
import-loss-list-levels = Niveaux de liste ramenés à un seul
import-loss-merged-cells = Cellules fusionnées séparées
import-loss-percent-stacked = Graphique empilé à 100 % rendu en empilé simple
import-loss-chart-format = Format des nombres { $format } du graphique non gardé
import-loss-run-style = { $effect ->
    [underline] Soulignement perdu
    [strike] Barré perdu
    [baseline] Exposant ou indice perdu
    [caps] Capitales perdues
    [font] Police d'un passage remplacée par celle de la boîte
    [tracking] Espacement des lettres d'un passage remplacé par celui de la boîte
    [weight] Passage maigre rendu gras comme la boîte
    [slant] Passage droit rendu en italique comme la boîte
   *[highlight] Surlignage perdu
}
import-loss-transform = Rotation, miroir ou rognage perdus
import-loss-color-dropped = La couleur { $color } n'a pas de couleur proche dans le thème : couleur du thème utilisée
import-loss-text-truncated = Texte raccourci pour tenir dans les limites
import-loss-table-truncated = Tableau raccourci pour tenir dans les limites
import-loss-chart-truncated = Données du graphique raccourcies pour tenir dans les limites
import-loss-opaque = { $reason ->
    [media] Vidéo ou son non importé (pas d'image de repli)
    [ole] Objet incorporé non importé (pas d'image de repli)
    [math] Équation non importée (pas d'image de repli)
    [smartart] SmartArt non importé (pas d'image de repli)
   *[unknown] Objet non pris en charge non importé (pas d'image de repli)
}
import-loss-smartart-layout = SmartArt sans son dessin : son texte est posé en cartes, la disposition du diagramme est perdue
import-loss-unsupported-image = Image au format { $format } non importée
import-loss-hidden-slide = Slide masquée importée comme visible
import-loss-effects = Ombres, reflets et effets 3D perdus
import-loss-series-dropped = Un graphique en secteurs ne montre qu'une série : { $kept } gardée, { $dropped } écartées
import-loss-repeated-decor = { $slides ->
   [one] En-tête ou pied « { $text } » répété sur { $slides } slide omis : le thème dessine les siens
  *[other] En-tête ou pied « { $text } » répété sur { $slides } slides omis : le thème dessine les siens
}
import-loss-no-slot = Contenu omis : le modèle choisi n'a pas de place pour lui
import-loss-moved-to-notes = Texte déplacé dans les notes de l'orateur : la couverture n'a pas de place pour lui
import-loss-items-dropped = Trop d'éléments sur la slide : { $dropped } éléments omis
import-loss-page-fitted = Page source de { $width } × { $height } pt, d'un autre format que 16:9 : contenu mis à l'échelle et centré
import-loss-paragraph-style = Alignement, interligne, espacement ou retrait d'un paragraphe ramené à celui de la boîte
import-loss-shape-style = { $effect ->
    [gradient] Dégradé rendu en couleur unie
    [picture] Image de remplissage étirée au cadre de la forme
    [dash] Trait en tirets rendu continu
    [arrow] Pointes de flèche du trait non dessinées
   *[geometry] Forme inconnue rendue en rectangle
}
import-loss-layout-dropped = Trop de layouts pour un pack : « { $layout } » n'a pas de modèle, ses slides prennent le widget vierge et reçoivent son décor en éléments
import-loss-decor-truncated = { $omitted ->
    [one] Décor coupé pour tenir dans la taille d'un fichier de pack : { $omitted } élément omis
   *[other] Décor coupé pour tenir dans la taille d'un fichier de pack : { $omitted } éléments omis
}
import-loss-table-style = Style du tableau approché : ses fonds de cellule, bordures, largeurs de colonnes, hauteurs de lignes ou tailles de texte propres sont perdus
import-loss-overflow = L'élément « { $element } » dépasse le bord de la slide
import-loss-empty-data = Tableau ou graphique sans données : rien n'est posé
import-reference-unavailable = Pas d'écart visuel : { $cause ->
    [timeout] le rendu de référence a pris trop de temps
    [memory] le rendu de référence a demandé trop de mémoire
    [pages] le rendu de référence a { $reference ->
        [one] { $reference } page
       *[other] { $reference } pages
    } pour { $deck } dans le deck
    [missing] LibreOffice (soffice) est introuvable ; indiquez son chemin dans import.soffice
   *[failed] le témoin n'a pas pu lire la présentation
}
import-region-suspect = Écart mis de côté, erreur connue du témoin : { $limit ->
    [autofit] il rétrécit un texte ajusté à sa boîte quand PowerPoint n'a enregistré aucune échelle
    [tracking] il dessine plus large un texte interlettré
    [chart] il ne dessine les graphiques qu'à peu près
    [default_font] il dessine dans sa police de repli un texte dont seuls les défauts de la présentation nomment la police
    [no_substitute] il n'a pas de substitut pour cette police et en dessine une autre
    [smart_art_style] il dessine le texte des SmartArt dans son propre style
   *[smart_art] il dessine le texte des SmartArt trop petit
}
import-font-substituted = { $family } remplacée par { $by }, aux métriques compatibles
import-font-missing = { $family } introuvable : le texte prend { $by }
import-font-restricted = { $family } : sa licence interdit d'embarquer certaines de ses faces dans le deck ; le texte de ces styles prend ses autres faces
import-font-restricted-substituted = { $family } ne peut pas être embarquée dans le deck (licence) : remplacée par { $by }, aux métriques compatibles
import-font-restricted-missing = { $family } ne peut pas être embarquée dans le deck (licence) : le texte prend { $by }
import-font-too-large = { $family } : fichiers trop gros pour être embarqués dans le deck ; le texte prend { $by }
import-font-missing-no-fallback = { $family } introuvable, comme la police du corps du thème : le texte est rendu avec la police par défaut du moteur
import-font-too-large-no-fallback = { $family } : fichiers trop gros pour être embarqués dans le deck, et la police du corps du thème est introuvable : le texte est rendu avec la police par défaut du moteur
import-font-restricted-no-fallback = { $family } ne peut pas être embarquée dans le deck (licence), et la police du corps du thème est introuvable : le texte est rendu avec la police par défaut du moteur
import-warning-effect-ignored = Effet non importé : { $detail }
import-warning-approximated = Rendu approché : { $detail }
import-warning-unreadable-item = Élément illisible ignoré : { $detail }
import-warning-unreadable-part = Partie illisible : { $detail }
import-warning-missing-part = Partie manquante : { $detail }
import-warning-unsupported-chart = Graphique non pris en charge : { $detail }
import-warning-unsupported-image = Format d'image non pris en charge : { $detail }
import-warning-oversized-image = Image trop grande pour être importée (nombre de pixels) : { $detail }
import-warning-external-image = Image liée hors du fichier, non importée : { $detail }
import-warning-unsafe-svg = Image SVG nettoyée de son contenu risqué, ou écartée : { $detail }
import-warning-unknown-table-style = Style de tableau inconnu : { $detail }
import-warning-unsafe-link = Lien refusé : { $detail }
import-warning-nesting-too-deep = Imbrication trop profonde, coupée : { $detail }
import-warning-font-embedded-unreadable = Police incorporée illisible : { $detail }
import-warning-truncated = Trop d'avertissements : les suivants sont omis
