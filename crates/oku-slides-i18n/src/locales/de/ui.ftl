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

ui-toast-templates-saved = Vorlagen gespeichert: { $name }
ui-toast-moved = Präsentation verschoben
ui-toast-copied = Kopie gespeichert
ui-toast-select-zone = Bereich auswählen, um „{ $text }“ zu platzieren
ui-toast-field-gone = Das bearbeitete Feld existiert nicht mehr: Die Eingabe ist verloren.
ui-toast-edit-replaced = Dieser Text wurde während Ihrer Eingabe geändert: Ihre Fassung hat ihn ersetzt. Rückgängig stellt die andere wieder her.

## Werkzeugleiste

ui-toolbar-undo = Rückgängig
ui-toolbar-undo-label = Rückgängig: { $label }
ui-toolbar-redo = Wiederholen
ui-toolbar-redo-label = Wiederholen: { $label }
ui-toolbar-grid-hide = Raster ausblenden
ui-toolbar-grid-show = Raster einblenden
ui-toolbar-ghosts-hide = Geister ausblenden
ui-toolbar-ghosts-show = Geister einblenden
ui-divider-pile = Breite des Folienstapels
ui-divider-notes = Breite des Agenten-Verlaufs
ui-divider-code = Codegröße
ui-divider-band = Höhe der Einstellungen
ui-toolbar-theme = Design
ui-toolbar-theme-embedded = Design der Datei
ui-toolbar-save-copy = Kopie speichern
ui-toolbar-finish = Fertig
ui-menu-workshop = Werkstatt
ui-toolbar-open = Öffnen
ui-merge = Mit PDF zusammenführen…
ui-menu = Menü
ui-menu-move = Verschieben…
ui-language = Sprache
ui-language-system = Systemsprache
ui-scheme = Erscheinungsbild
ui-scheme-system = System
ui-scheme-light = Hell
ui-scheme-dark = Dunkel
ui-toolbar-add-label = Zur Folie hinzufügen
ui-add-kind = { $kind ->
    [card] Karte
    [shape] Form
    [chart] Diagramm
    [table] Tabelle
   *[text] Text
}
ui-add-widget = Paket-Widget
ui-toolbar-remove-template = Vorlage löschen
ui-toolbar-remove-slide = Folie löschen
ui-toolbar-remove-hint = Folie { $n } · rückgängig mit ⌘Z

## Rückgängig-Beschriftungen

ui-undo-add = { $kind ->
    [card] Karte hinzufügen
    [shape] Form hinzufügen
    [chart] Diagramm hinzufügen
    [table] Eine Tabelle hinzufügen
   *[text] Text hinzufügen
}
ui-undo-add-widget = „{ $name }“ hinzufügen
ui-undo-new-template = Neue Vorlage: { $name }
ui-undo-new-slide = Neue Folie: { $name }
ui-undo-templates = Vorlagen: { $name }
ui-undo-move-template = Vorlage verschieben
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
ui-undo-dictation = Diktat
ui-undo-rename-set = Set umbenennen
ui-undo-rename-deck = Präsentation umbenennen
ui-undo-chart-kind = Diagrammtyp ändern
ui-undo-chart-data = Daten bearbeiten
ui-undo-chart-color = Reihe einfärben
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
    [group] Gruppe
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
ui-agent-detach = Agent in separatem Fenster öffnen
ui-agent-attach = Agent ins Hauptfenster zurückholen
ui-agent-window-empty = Kein Agent für diesen Tab
ui-agent-kept-in-draft = Kein Ziel zum Einfügen: Text im Entwurf behalten
ui-agent-busy-elsewhere = Der Agent antwortet in { $place } …
ui-agent-scope-slides = die Folien
ui-agent-scope-atelier = die Werkstatt
ui-draft = Entwurf
ui-draft-insert = Einfügen
ui-draft-insert-title = Unverändert einfügen (⌘⇧↩)
ui-draft-send = An den Agenten senden
ui-draft-send-title = An den Agenten senden (⌘↩)
ui-draft-discard = Verwerfen
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
ui-dictation-settings = Einstellungen
ui-dictation-model-title = Erkennungsmodell
ui-dictation-model-intro = Whisper, auf diesem Computer (CTranslate2). Jedes Modell versteht mehrere Sprachen.
ui-dictation-note-turbo = Empfohlen, schnell und präzise
ui-dictation-note-large = Am präzisesten, langsamer
ui-dictation-note-base = Leicht, für schwächere Rechner
ui-dictation-installed = installiert
ui-dictation-not-installed = herunterzuladen
ui-dictation-language = Gesprochene Sprache
ui-dictation-language-auto = Automatische Erkennung
ui-dictation-note-apple = Am schnellsten, ins System integriert
ui-dictation-model-unavailable = Sprache nicht unterstützt
ui-dictation-model-intro-apple = Apple, ins System integriert, oder Whisper, auf diesem Computer (CTranslate2). Alle verstehen mehrere Sprachen.
ui-dictation-language-auto-resolved = Automatisch ({ $language })
ui-dictation-to-download-language = Sprache herunterzuladen
ui-dictation-auto-stop-label = Automatisch stoppen nach einer Stille von
ui-dictation-silence = Dauer der Stille
ui-dictation-phrases-note = Nur gesprochene Passagen werden transkribiert, Satz für Satz, während gesprochen wird.
ui-dictation-download-now = Jetzt herunterladen ({ $size })
ui-dictation-download-language = Sprache herunterladen
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
ui-undo-table-option = Tabellenoptionen
ui-undo-table-align = Spalte ausrichten
ui-undo-table-data = Tabelle bearbeiten
ui-show-as = Anzeigen als
ui-show-as-table = Als Tabelle anzeigen
ui-show-as-chart = Als Diagramm anzeigen
ui-undo-show-as-table = Als Tabelle anzeigen
ui-undo-show-as-chart = Als Diagramm anzeigen

## Dateien

ui-file-open-prompt = Pfad der zu öffnenden Datei (.oku-slides.pdf oder .json)
ui-file-save-prompt = Speichern unter (vollständiger Pfad)

## Tabs

ui-tabs = Geöffnete Präsentationen
ui-tab-new = Neuer Tab
ui-tab-close = Tab schließen
ui-tab-unsaved = Nicht gespeicherte Änderungen
ui-tab-templates = Vorlagen
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
ui-present-window = Im Fenster präsentieren
ui-toolbar-params-hide = Einstellungen ausblenden
ui-toolbar-params-show = Einstellungen einblenden
ui-help-close = Hilfe schließen
ui-help-search = In der Hilfe suchen
ui-help-search-placeholder = Suchen…
ui-help-search-empty = Keine Treffer
ui-help-search-count = { $count } Treffer
ui-pack-notice-close = Schließen
ui-widgets-files = Widget-Dateien
ui-widgets-common = Gemeinsam
ui-widgets-new = Neues Widget
ui-widgets-name = Widget-Name (Kleinbuchstaben, Ziffern und Bindestriche)
ui-widgets-name-taken = Dieser Name ist bereits vergeben
ui-widgets-name-letter = Der Name muss mit einem Buchstaben beginnen
ui-widgets-valid = Gültig
ui-widgets-errors = Fehler
ui-widgets-empty = Dieses Paket hat noch kein Widget.
ui-widgets-preview = Widget-Vorschau
ui-widgets-no-preview = Keine Vorschau
ui-widgets-field-bound = Mit der Präsentation verknüpft: hier schreibgeschützt
ui-widgets-source = Quellcode
ui-widgets-unsaved = Nicht gespeichert
ui-widgets-in-error = Widgets mit Fehlern
ui-widgets-none-in-error = Kein Widget hat Fehler
ui-widgets-section = Widgets
ui-widgets-open = Widget { $name } öffnen
ui-widgets-delete-named = { $name } löschen
ui-widgets-confirm-delete = Widget „{ $name }“ löschen?
ui-widgets-rename-hint = Doppelklick oder F2 zum Umbenennen
ui-widgets-tabs = Vorlagen-Tabs
ui-widgets-templates-tab = Vorlagen
ui-theme-tab = Design
ui-theme-picker = Design
ui-theme-name = Name des Designs
ui-theme-unsaved = Designänderungen nicht gespeichert
ui-widgets-close-tab = { $name } schließen
ui-widgets-removed-by-agent = Der Agent hat das Widget { $name } gelöscht.
