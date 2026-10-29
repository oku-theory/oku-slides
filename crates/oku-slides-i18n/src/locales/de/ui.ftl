# Interface de l’app (React). Chargé par ui/src/i18n.ts via la requête « messages ».
ui-loading = Wird geladen…

## Dokument

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · Layout-Paket
   *[other] { $name } · Widget-Paket
}
ui-title-rename = Präsentation umbenennen
ui-title-field = Titel der Präsentation
ui-status-unsaved = ungespeicherte Änderungen
ui-status-saved = gespeichert
ui-status-never-saved = noch nicht gespeichert
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count } Layout
       *[other] { $count } Layouts
    }
   *[other] { $count ->
        [one] { $count } Widget
       *[other] { $count } Widgets
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } Folie
   *[other] { $count } Folien
} · { $status }
ui-confirm-quit-pack = Beenden, ohne das geänderte Paket zu speichern?
ui-confirm-quit-theme = Beenden, ohne das geänderte Design zu speichern?
ui-confirm-quit-read-only-theme = { $theme } kann hier nicht gespeichert werden. Diesen Tab schließen und seine Änderungen verlieren?
ui-confirm-update-opener = „{ $deck }“ auf diese Version des Pakets „{ $pack }“ aktualisieren?
ui-close-writes-pending = Dieser Tab speichert noch: Schließen Sie ihn gleich noch einmal.
ui-confirm-close-writes-pending = Dieser Tab speichert noch. Trotzdem schließen und verlieren, was nicht gespeichert ist?
ui-confirm-close-writes-pending-linked = Dieser Tab speichert noch, und diese Paket-Tabs werden ungespeichert mit ihm geschlossen: { $tabs }. Trotzdem schließen und verlieren, was nicht gespeichert ist?
ui-confirm-quit-read-only-pack = { $pack } kann hier nicht gespeichert werden. Diesen Tab schließen und seine Änderungen verlieren? Um sie zu behalten, bitten Sie den Agenten, es als neues Paket zu speichern.

## Meldungen

ui-toast-copied = Kopie gespeichert
ui-toast-field-gone = Das bearbeitete Feld existiert nicht mehr: Die Eingabe ist verloren.
ui-toast-edit-replaced = Dieser Text wurde während Ihrer Eingabe geändert: Ihre Fassung hat ihn ersetzt. Rückgängig stellt die andere wieder her.

## Werkzeugleiste

ui-toolbar-undo = Rückgängig
ui-toolbar-undo-label = Rückgängig: { $label }
ui-toolbar-redo = Wiederholen
ui-toolbar-redo-label = Wiederholen: { $label }
ui-divider-pile = Breite des Folienstapels
ui-divider-notes = Breite des Agenten-Verlaufs
ui-divider-code = Codegröße
ui-toolbar-theme = Design
ui-toolbar-finish = Fertig
ui-pack-deck-only = Nur in dieser Präsentation gespeichert
ui-toolbar-open = Öffnen
ui-menu = Menü
ui-menu-new-widget-pack = Neues Widget-Paket
ui-menu-new-layout-pack = Neues Layout-Paket
ui-menu-new-theme = Neues Theme
ui-toolbar-add-label = Zur Folie hinzufügen
ui-toolbar-packs = Pakete der Präsentation
ui-toolbar-packs-update = Pakete der Präsentation — Aktualisierungen verfügbar
ui-toolbar-thumbs-show = Miniaturen anzeigen
ui-toolbar-thumbs-hide = Miniaturen ausblenden
ui-toolbar-remove-slide = Folie löschen
ui-toolbar-remove-hint = Folie { $n } · rückgängig mit ⌘Z

## Rückgängig-Beschriftungen

ui-undo-add-widget = „{ $name }“ hinzufügen
ui-undo-new-slide = Neue Folie: { $name }
ui-undo-move-slide = Folie verschieben
ui-undo-remove-slide = Folie löschen
ui-undo-remove = Löschen
ui-undo-paste = Einfügen
ui-undo-place-on-grid = Im Raster platzieren
ui-undo-move-on-grid = Im Raster verschieben
ui-undo-resize-on-grid = Im Raster skalieren
ui-undo-change-anchor = Verankerung ändern
ui-undo-move = Verschieben
ui-undo-resize = Größe ändern
ui-undo-edit-slide = Folie bearbeiten
ui-undo-rename-pack = Paket umbenennen
ui-undo-rename-deck = Präsentation umbenennen

## Folienstapel

ui-pile-slides = Folien
ui-pile-layouts = Layouts
ui-pile-add-slide = Neue Folie
ui-pile-drag-hint = Zum Verschieben ziehen · ⌥↑ ⌥↓
ui-slide-alt = Folie { $n }

## Bühne

ui-stage-rendering = Wird gerendert…
ui-stage-elements = Folienelemente
ui-stage-fields = Folienfelder
ui-stage-edit-text = Text bearbeiten
ui-stage-hint-plain = Enter oder Tab zum nächsten · Esc zum Abbrechen
ui-stage-hint-rich = Tab zum nächsten · ⌘ Enter zum Bestätigen · „- “ für einen Aufzählungspunkt · ⌘B fett, ⌘I kursiv, ⌘K Link
ui-field = { $kind ->
    [title] Titel
    [subtitle] Untertitel
    [kicker] Dachzeile
    [text] Text
    [body] Textkörper
    [author] Autor
    [date] Datum
    [organization] Organisation
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Text
    [card] Karte
    [image] Bild
    [shape] Form
    [chart] Diagramm
    [table] Tabelle
    [stack] Stapel
   *[other] { $kind }
}
ui-grid-cell = Sp. { $cols } · Zeile { $rows }
ui-ghost-zone = Bereich

## Foliennotizen

ui-notes = Folienwerkzeuge
ui-stepper-less = { $label }: eins weniger
ui-stepper-more = { $label }: eins mehr
ui-list-item = Element
ui-list-add = { $item } hinzufügen
ui-list-remove = { $item } entfernen
ui-list-move = { $item } verschieben
ui-list-full = Die Liste ist voll ({ $max })
ui-undo-list-move = { $item } verschieben
ui-agent-color-other = Andere Farbe
ui-agent-thread = Agent
ui-agent-new-conversation = Neue Unterhaltung
ui-agent-undo-turn = Diese Runde rückgängig machen
ui-agent-detach = Agent in separatem Fenster öffnen
ui-agent-attach = Agent ins Hauptfenster zurückholen
ui-agent-window-empty = Kein Agent für diesen Tab
ui-agent-kept-in-draft = Kein Ziel zum Einfügen: Text im Entwurf behalten
ui-draft = Entwurf
ui-draft-send = An den Agenten senden
ui-draft-send-title = An den Agenten senden (⌘↩)
ui-draft-discard = Verwerfen
ui-pasted-image = { $type } { $width }×{ $height }
ui-draft-images = Eingefügte Bilder
ui-draft-remove-image = Dieses Bild entfernen
ui-draft-send-needs-text = Sagen Sie, was damit geschehen soll
ui-agent-state-idle = Agent
ui-agent-state-starting = Der Agent startet…
ui-agent-state-working = Der Agent arbeitet…
ui-agent-dock = In Arbeit
ui-agent-dock-settings = Einstellungen
ui-agent-jump = ↓ Aktuelle Runde
ui-agent-tool-input = Argumente
ui-agent-tool-output = Ergebnis
ui-agent-sheet = Frage des Agenten
ui-agent-sheet-keys = 1–4 wählen · Enter bestätigen · E bearbeiten · Esc abbrechen
ui-agent-variants = Varianten
ui-agent-drawer-keys = Klick oder 1–4 · Esc schließt
ui-agent-confirm = Bestätigung des Agenten

## Diktat

ui-dictation = Diktat
ui-dictation-start = Diktieren
ui-dictation-start-title = Diktieren (⌘D)
ui-dictation-type = An den Agenten schreiben
ui-dictation-type-title = An den Agenten schreiben (⌘K)
ui-dictation-agent-stop = Agent stoppen (⌘⇧D oder Esc)
ui-dictation-agent-working = Der Agent arbeitet…
ui-dictation-agent-working-hint = ⌘⇧D oder Esc stoppt ihn
ui-dictation-finish = Diktat beenden
ui-dictation-finish-title = Beenden (⌘D)
ui-dictation-cancel = Abbrechen
ui-dictation-preparing = Diktat wird vorbereitet…
ui-dictation-heard-nothing = Nichts gehört: etwas näher am Mikrofon sprechen
ui-dictation-to-cursor = an der Cursorposition
ui-dictation-to-draft = in den Entwurf
ui-dictation-downloading = Modell { $name } wird heruntergeladen
ui-dictation-download-progress = { $percent } % · { $received } von { $total } · nur einmalig
ui-dictation-listening = Zuhören · { $time }
ui-dictation-transcribing = Letzter Satz wird transkribiert…
ui-dictation-stop-in = Stopp in { $duration }
ui-dictation-speak = Sprechen, der Text erscheint hier…
ui-dictation-auto-stop = stoppt nach { $duration } Stille
ui-dictation-to-download = { $size } herunterzuladen
ui-dictation-settings = Einstellungen
ui-dictation-language-auto = Automatische Erkennung
ui-dictation-model-unavailable = Sprache nicht unterstützt
ui-dictation-language-auto-resolved = Automatisch ({ $language })
ui-dictation-to-download-language = Sprache herunterzuladen
ui-dictation-api-key-missing = { $var } ist nicht gesetzt
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Vorlagenauswahl

ui-picker-new-slide = Neue Folie
ui-picker-set-mine = { $name } · eigene
ui-picker-set-received = { $name } · empfangen
ui-picker-keys = ← → zum Blättern · Enter zum Hinzufügen · Esc zum Schließen
ui-picker-close = Schließen
ui-picker-list = Layouts
ui-widget-picker-filter = Nach Name, Bezeichnung oder Beschreibung filtern
ui-widget-picker-empty = Kein Widget passt zu „{ $query }“.
ui-widget-picker-list = Widgets
ui-widget-picker-keys = Pfeiltasten zum Blättern · Eingabe zum Hinzufügen · Esc zum Schließen

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
ui-chart-waterfall-hint = „Summe“ ankreuzen für einen Summenbalken.
ui-chart-colors = Reihenfarben
ui-chart-color-of = Farbe von { $name }
ui-chart-color-auto = Automatisch
ui-chart-color-pick = Farbe { $n }
ui-chart-column-name = Name der Spalte { $col }
ui-chart-remove-column = Spalte löschen
ui-chart-cell = Zeile { $row }, Spalte { $col }
ui-chart-remove-row = Zeile löschen
ui-chart-add-row = Zeile
ui-chart-add-column = Spalte
ui-chart-series = Reihe { $n }

## Tabelle

ui-table-panel = Tabelle
ui-table-options = Tabellenoptionen
ui-table-header-row = Kopfzeile
ui-table-header-column = Erste Spalte als Kopf
ui-table-striped = Gestreifte Zeilen
ui-table-alignment = Spaltenausrichtung
ui-table-align-of = Ausrichtung von { $name }
ui-table-align = { $align ->
    [start] Anfang
    [center] Zentriert
    [end] Ende
   *[auto] Automatisch
}
ui-table-column = Spalte { $n }
ui-table-no-data = Diese Tabelle enthält keine Daten.
ui-show-as = Anzeigen als
ui-show-as-table = Als Tabelle anzeigen
ui-show-as-chart = Als Diagramm anzeigen

## Dateien

ui-file-open-prompt = Pfad der zu öffnenden Datei (.oku-slides.pdf, .json oder .pptx)
ui-file-filter-presentations = Präsentationen
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = Speichern unter (vollständiger Pfad)

## Tabs

ui-tabs = Geöffnete Präsentationen
ui-tab-new = Neuer Tab
ui-tab-close = Tab schließen
ui-tab-unsaved = Nicht gespeicherte Änderungen
ui-tab-kind-pack = { $title }, Paket
ui-tab-kind-theme = { $title }, Design
ui-tab-theme = Design
ui-preview-theme = Vorschau-Thema
ui-tab-move-new-window = In neues Fenster verschieben
ui-tab-move-to = In Fenster { $n } verschieben
ui-tab-close-others = Andere Tabs schließen
ui-close-cancel = Abbrechen
ui-disk-replaced-question = { $file } wurde auf dem Datenträger durch eine andere Präsentation ersetzt.
ui-disk-replaced-overwrite = Überschreiben
ui-disk-replaced-save-as = Speichern unter…
ui-same-deck-question = { $file } ist eine Version der Präsentation, die im Tab „{ $title }“ geöffnet ist.
ui-same-deck-merge = In diesen Tab zusammenführen
ui-same-deck-apart = Separat öffnen
ui-history = Verlauf
ui-history-version = Version vom { $date }
ui-history-restore = Wiederherstellen
ui-history-back = Zurück
ui-history-anonymous = Anonym
ui-history-slider = Versionen der Präsentation
ui-history-removed-since = seitdem gelöscht
ui-history-changed-since = seitdem geändert
ui-history-added-since = seitdem hinzugefügte Folien: { $count }
ui-help = Hilfe
ui-help-caption = Folie { $index } von { $count } · schreibgeschützt
ui-slide-previous = Vorherige Folie
ui-slide-next = Nächste Folie
ui-present = Präsentation
ui-present-slide = Folie { $index } von { $count }
ui-present-screen = Im Vollbild präsentieren
ui-help-close = Hilfe schließen
ui-help-search = In der Hilfe suchen
ui-help-search-placeholder = Suchen…
ui-help-search-empty = Keine Treffer
ui-help-search-count = { $count } Treffer
ui-widgets-common = Gemeinsam
ui-widgets-new = Neues Widget
ui-widgets-name = Widget-Name (Kleinbuchstaben, Ziffern und Bindestriche)
ui-layouts-name = Layout-Name (Kleinbuchstaben, Ziffern und Bindestriche)
ui-widgets-name-taken = Dieser Name ist bereits vergeben
ui-widgets-name-letter = Der Name muss mit einem Buchstaben beginnen
ui-widgets-empty = Dieses Paket hat noch kein Widget.
ui-layouts-new = Neues Layout
ui-layouts-empty = Dieses Paket hat noch kein Layout.
ui-confirm-delete-ok = Löschen
ui-widgets-preview = Widget-Vorschau
ui-widgets-no-preview = Keine Vorschau
ui-widgets-field-bound = Mit der Präsentation verknüpft: hier schreibgeschützt
ui-widgets-source = Quellcode
ui-widgets-unsaved = Nicht gespeichert
ui-widgets-in-error = Widgets mit Fehlern
ui-widgets-none-in-error = Kein Widget hat Fehler
ui-widgets-open = Widget { $name } öffnen
ui-widgets-delete-named = { $name } löschen
ui-widgets-confirm-delete = Widget „{ $name }“ löschen?
ui-layouts-confirm-delete = Layout „{ $name }“ löschen?
ui-widgets-rename-hint = Doppelklick oder F2 zum Umbenennen
ui-theme-name = Name des Designs
ui-theme-unsaved = Designänderungen nicht gespeichert
ui-widgets-removed-by-agent = Der Agent hat das Widget { $name } gelöscht.
ui-layouts-removed-by-agent = Der Agent hat das Layout { $name } gelöscht.

## Importieren

ui-import-title = PowerPoint-Präsentation importieren
ui-import-mode = Importmodus
ui-import-hifi = Originalgetreue Kopie
ui-import-hifi-note = Behält das Aussehen des Originals, mit einem Design und Layouts für diese Präsentation.
ui-import-retrofit = Meine Layouts verwenden
ui-import-retrofit-note = Baut die Folien mit Layout-Paketen und einem Design aus der Bibliothek neu auf.
ui-import-layout-packs = Layout-Pakete
ui-import-theme-default = Standarddesign
ui-import-no-packs = Kein Layout-Paket in der Bibliothek.
ui-import-tune = Mit dem Agenten feinabstimmen
ui-import-tune-note = Der Agent vergleicht jede Folie mit dem Original und korrigiert die Abweichungen.
ui-import-tune-note-retrofit = Der Agent vergleicht jede Folie mit dem Original und entwirft Widgets für die Formen, die Ihre Layouts nicht darstellen.
ui-import-tune-unavailable = Kein nutzbarer Agent: { $reason }
ui-import-submit = Importieren
ui-import-busy = Wird importiert…
ui-update-available = Oku Slides { $version } ist verfügbar.
ui-update-install = Installieren und neu starten
ui-update-see-release = Release ansehen
ui-update-later = Später
ui-update-installing = Update wird heruntergeladen … { $percent } %
ui-update-downloading = Update wird heruntergeladen …
ui-update-up-to-date = Oku Slides ist auf dem neuesten Stand.
ui-update-check = Nach Updates suchen …
ui-about = Über Oku Slides
ui-about-version = Version { $version }
ui-about-changelog = Neuigkeiten
ui-about-breaking = inkompatibel
ui-about-close = Schließen

ui-shortcut-theme = Thema, Themen, Design, theme, themes
