# Interface de l’app (React). Chargé par ui/src/i18n.ts via la requête « messages ».
ui-loading = Wird geladen…

## Dokument

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · Vorlagen
ui-title-rename = Präsentation umbenennen
ui-title-field = Titel der Präsentation
ui-status-unsaved = ungespeicherte Änderungen
ui-status-saved = gespeichert
ui-status-never-saved = noch nicht gespeichert
ui-status-builtin = integriert: Speichern erstellt eine Kopie
ui-caption-templates = { $count ->
    [one] { $count } Vorlage
   *[other] { $count } Vorlagen
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } Folie
   *[other] { $count } Folien
} · { $status }
ui-template-default-name = Vorlage
ui-template-variant-name = { $name } (Variante)
ui-confirm-quit-templates = Beenden, ohne die geänderten Vorlagen zu speichern?

## Meldungen

ui-toast-saved = Die Präsentation ist gespeichert
ui-toast-templates-saved = Vorlagen gespeichert: { $name }
ui-toast-select-zone = Bereich auswählen, um „{ $text }“ zu platzieren

## Werkzeugleiste

ui-toolbar-undo = Rückgängig
ui-toolbar-undo-label = Rückgängig: { $label }
ui-toolbar-redo = Wiederholen
ui-toolbar-redo-label = Wiederholen: { $label }
ui-toolbar-grid-hide = Raster ausblenden
ui-toolbar-grid-show = Raster einblenden
ui-toolbar-grid-hide-title = Raster ausblenden (⌘')
ui-toolbar-grid-show-title = Raster einblenden (⌘')
ui-toolbar-theme = Design
ui-toolbar-theme-default = Standard
ui-toolbar-save-copy = Kopie speichern
ui-toolbar-save = Speichern
ui-toolbar-finish = Fertig
ui-toolbar-templates = Vorlagen
ui-toolbar-templates-title = Folienvorlagen erstellen und bearbeiten
ui-toolbar-open = Öffnen
ui-toolbar-add-label = Zur Folie hinzufügen
ui-toolbar-add = Hinzufügen
ui-add-kind = { $kind ->
    [card] Karte
    [shape] Form
    [chart] Diagramm
   *[text] Text
}
ui-toolbar-remove-template = Vorlage löschen
ui-toolbar-remove-slide = Folie löschen
ui-toolbar-remove-hint = Folie { $n } · rückgängig mit ⌘Z

## Rückgängig-Beschriftungen

ui-undo-add = { $kind ->
    [card] Karte hinzufügen
    [shape] Form hinzufügen
    [chart] Diagramm hinzufügen
   *[text] Text hinzufügen
}
ui-undo-new-template = Neue Vorlage: { $name }
ui-undo-new-slide = Neue Folie: { $name }
ui-undo-grid = Raster { $columns } × { $rows }
ui-undo-templates = Vorlagen: { $name }
ui-undo-move-template = Vorlage verschieben
ui-undo-move-slide = Folie verschieben
ui-undo-remove-slide = Folie löschen
ui-undo-remove = Löschen
ui-undo-place-on-grid = Im Raster platzieren
ui-undo-move-on-grid = Im Raster verschieben
ui-undo-resize-on-grid = Im Raster skalieren
ui-undo-change-anchor = Verankerung ändern
ui-undo-move = Verschieben
ui-undo-resize = Größe ändern
ui-undo-edit-text = Text bearbeiten
ui-undo-dictation = Diktat
ui-undo-rename-set = Set umbenennen
ui-undo-rename-deck = Präsentation umbenennen
ui-undo-theme = Design ändern
ui-undo-rename-template = Vorlage umbenennen
ui-undo-describe-template = Vorlage beschreiben
ui-undo-chart-kind = Diagrammtyp ändern
ui-undo-chart-data = Daten bearbeiten
ui-undo-chart-stack = { $stacked ->
    [yes] Reihen stapeln
   *[no] Reihen trennen
}

## Neue Elemente

ui-new-text = Neuer Text
ui-new-card-title = Kartentitel
ui-new-card-detail = Detail

## Folienstapel

ui-pile-slides = Folien
ui-pile-templates = Vorlagen
ui-pile-add-slide = + Neue Folie
ui-pile-add-template = + Neue Vorlage
ui-pile-drag-hint = Zum Verschieben ziehen · ⌥↑ ⌥↓
ui-slide-alt = Folie { $n }

## Bühne

ui-stage-rendering = Wird gerendert…
ui-stage-elements = Folienelemente
ui-stage-edit-text = Text bearbeiten
ui-stage-hint-field = Enter zum Bestätigen · Esc zum Abbrechen
ui-stage-hint-block = ⌘ Enter zum Bestätigen · eine Zeile pro Absatz, „- “ für einen Aufzählungspunkt
ui-field = { $kind ->
    [title] Titel
    [subtitle] Untertitel
    [kicker] Dachzeile
    [text] Text
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Text
    [card] Karte
    [image] Bild
    [shape] Form
    [group] Gruppe
   *[other] { $kind }
}
ui-grid-cell = Sp. { $cols } · Zeile { $rows }

## Foliennotizen

ui-notes = Folienwerkzeuge
ui-stepper-less = { $label }: eins weniger
ui-stepper-more = { $label }: eins mehr

## Diktat

ui-dictation = Diktat
ui-dictation-start = Diktieren
ui-dictation-start-title = Diktieren (⌘D)
ui-dictation-finish = Diktat beenden
ui-dictation-finish-title = Beenden (⌘D)
ui-dictation-cancel = Abbrechen
ui-dictation-preparing = Diktat wird vorbereitet…
ui-dictation-heard-nothing = Nichts gehört: etwas näher am Mikrofon sprechen
ui-dictation-to-cursor = an der Cursorposition
ui-dictation-to-field = { $field } ersetzen
ui-dictation-to-block = nach dem ausgewählten Block
ui-dictation-new-block = neuer Textblock
ui-dictation-select-zone = Bereich auswählen
ui-dictation-downloading = Modell { $name } wird heruntergeladen
ui-dictation-download-progress = { $percent } % · { $received } von { $total } · nur einmalig
ui-dictation-listening = Zuhören · { $time }
ui-dictation-transcribing = Letzter Satz wird transkribiert…
ui-dictation-stop-in = Stopp in { $duration }
ui-dictation-speak = Sprechen, der Text erscheint hier…
ui-dictation-auto-stop = stoppt nach { $duration } Stille
ui-dictation-to-download = { $size } herunterzuladen
ui-dictation-settings = Diktiereinstellungen
ui-dictation-model-title = Erkennungsmodell
ui-dictation-model-intro = Whisper, auf diesem Computer (CTranslate2). Jedes Modell versteht mehrere Sprachen.
ui-dictation-note-turbo = Empfohlen, schnell und präzise
ui-dictation-note-large = Am präzisesten, langsamer
ui-dictation-note-base = Leicht, für schwächere Rechner
ui-dictation-installed = installiert
ui-dictation-not-installed = herunterzuladen
ui-dictation-language = Gesprochene Sprache
ui-dictation-language-auto = Automatische Erkennung
ui-dictation-auto-stop-label = Automatisch stoppen nach einer Stille von
ui-dictation-silence = Dauer der Stille
ui-dictation-phrases-note = Nur gesprochene Passagen werden transkribiert, Satz für Satz, während gesprochen wird.
ui-dictation-download-now = Jetzt herunterladen ({ $size })
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Vorlagenauswahl

ui-picker-new-slide = Neue Folie
ui-picker-new-template = Neue Vorlage, basierend auf…
ui-picker-set = Vorlagen
ui-picker-set-mine = { $name } · eigene
ui-picker-keys = ← → zum Blättern · Enter zum Hinzufügen · Esc zum Schließen
ui-picker-close = Schließen
ui-picker-previous = Vorherige Vorlage
ui-picker-next = Nächste Vorlage
ui-picker-list = Folienvorlagen

## Diagramm

ui-chart-panel = Diagramm
ui-chart-kinds = Diagrammtyp
ui-chart-kind = { $kind ->
    [line] Linien
    [pie] Kreis
    [scatter] Punkte
    [radar] Netz
    [waterfall] Wasserfall
   *[bar] Balken
}
ui-chart-no-data = Dieses Diagramm hat keine Datentabelle.
ui-chart-options = Diagrammoptionen
ui-chart-stack = Gestapelt
ui-chart-waterfall-hint = Lassen Sie einen Wert leer für einen Summenbalken.
ui-chart-column-name = Name der Spalte { $col }
ui-chart-remove-column = Spalte löschen
ui-chart-cell = Zeile { $row }, Spalte { $col }
ui-chart-remove-row = Zeile löschen
ui-chart-add-row = + Zeile
ui-chart-add-column = + Spalte
ui-chart-series = Reihe { $n }

## Dateien

ui-file-open-prompt = Pfad der zu öffnenden Datei (.oku-slides.pdf oder .json)
ui-file-save-prompt = Speichern unter (vollständiger Pfad)
