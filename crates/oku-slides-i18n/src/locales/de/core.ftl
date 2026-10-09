op-slide-not-found = Folie nicht gefunden: { $subject }
op-element-not-found = Element nicht gefunden: { $subject }
op-duplicate-id = Kennung bereits vergeben: { $subject }
op-index-out-of-range = Position außerhalb des gültigen Bereichs
op-invalid-value = Ungültiger Wert für { $subject }
op-invalid-value-because = Ungültiger Wert für { $subject }: { $reason }
op-value-no-value = der Parameter hat keinen Wert
op-value-no-dataset = das Diagramm hat keinen Datensatz
op-value-grid-tracks = ein Raster hat 1 bis 24 Spalten und Zeilen
op-value-bad-item-id = eine Eintrags-ID ist ein nicht leerer Text ohne „/“
op-value-missing-item-id = jeder Eintrag braucht eine ID
op-value-too-deep = die Elemente sind zu tief verschachtelt
op-value-empty-path = der Pfad ist leer
op-value-not-text = ein Text oder null wird erwartet
op-value-empty-frame = Breite und Höhe müssen größer als 0 sein
op-value-blocks-off-grid = Flow-Slots werden von der Folie angeordnet und ein Raster verdeckt sie; verschieben Sie zuerst jedes Element in einen freien Slot
op-value-outside-grid = die Zelle liegt außerhalb des Rasters
op-value-not-tokens = ein Objekt oder null wird erwartet
op-value-not-a-slot = dieser Parameter ist kein Slot
op-value-out-of-bounds = die Zahl liegt außerhalb des erlaubten Bereichs
op-value-unreadable = der Wert hat nicht die erwartete Form
op-unknown-widget = Unbekanntes Widget: { $subject }
op-unknown-param = Unbekannter Parameter: { $subject }
op-param-type = Ungültiger Wert für { $subject }
op-item-not-found = Kein Eintrag { $subject }
op-list-full = Die Liste { $subject } ist voll (höchstens { $max ->
   [one] { $max } Eintrag
  *[other] { $max } Einträge
})
op-list-minimum = Die Liste { $subject } braucht mindestens { $min ->
   [one] { $min } Eintrag
  *[other] { $min } Einträge
}
op-list-cell-too-long = Eine Zelle der Liste { $subject } ist zu lang (Spalte { $column })
op-list-id-too-long = Eine Eintrags-ID der Liste { $subject } ist zu lang
op-list-too-many-items = Die Liste { $subject } hat zu viele Einträge (höchstens { $max })
op-slot-rejects = { $subject } ist hier nicht zulässig
op-wrong-category = { $subject } kann hier nicht verwendet werden
op-choice-out-of-range = Wert für { $subject } nicht zulässig
transaction-refused = Vorgang { $index } abgelehnt: { $reason }

sources-unreadable = Quellen nicht lesbar
sources-not-sources = Dies ist kein Oku-Slides-Quellarchiv
sources-unsupported-version = Diese Quellen wurden mit einer neueren Version von Oku Slides erstellt
sources-bad-name = Unzulässiger Dateiname in den Quellen
sources-missing-deck = Die Präsentation fehlt in den Quellen
sources-too-large = Die Quellen in dieser Datei sind zu groß zum Öffnen
sources-too-deep = Dieses Deck verschachtelt Elemente zu tief

issue-empty-slide-id = leere Folienkennung
issue-duplicate-slide-id = doppelte Folienkennung
issue-duplicate-element-id = doppelte Elementkennung
issue-empty-element-id = leere Kennung
issue-cell-outside-grid = Zelle außerhalb des Rasters
issue-empty-frame = Rahmen ohne Größe
issue-free-without-frame = freies Element ohne Rahmen
issue-unknown-widget = unbekanntes Widget: { $kind }
issue-wrong-category = { $kind } kann hier nicht verwendet werden
issue-unknown-param = unbekannter Parameter: { $param }
issue-param-type = ungültiger Wert für { $param }
issue-choice-out-of-range = Wert für { $param } nicht erlaubt
issue-slot-rejects = { $kind } wird hier nicht akzeptiert
issue-placed-in-flow = Element mit Rahmen in einer angeordneten Liste
issue-unknown-bind = { $param } ist mit einer unbekannten Deck-Einstellung verknüpft
issue-slot-too-deep = Elemente sind zu tief verschachtelt
issue-missing-asset = fehlende Ressource: { $asset }
issue-pack-version-mismatch = veränderte Paketversion verworfen: { $version }
issue-pack-version-missing = Paketversion fehlt in den Quellen: { $version }
issue-theme-version-mismatch = veränderte Designversion verworfen: { $version }
issue-theme-version-missing = Designversion fehlt in den Quellen: { $version }
issue-unsupported-link = nicht unterstützter Link: { $link }
issue-highlight-zero = Hervorhebungsfarben werden ab 1 nummeriert
issue-unreadable-element = unlesbares Element, beiseitegelegt
issue-unreadable-item = unlesbarer Listeneintrag, beiseitegelegt
issue-unreadable-row = unlesbare Tabellenzeile, beiseitegelegt
issue-data-truncated = Daten zu groß, beim Lesen gekürzt
issue-unreadable-slide = unlesbare Folie, beiseitegelegt
issue-unreadable-field = unlesbares Deck-Feld { $field }, zurückgesetzt
op-invalid-chart = Diagramm { $subject }: { $reason }
op-invalid-table = Tabelle { $subject }: { $reason }

replica-corrupted = Der Verlauf der Präsentation in dieser Datei ist beschädigt
replica-too-large = Der Verlauf der Präsentation in dieser Datei ist zu groß zum Öffnen
replica-write = Der Verlauf der Präsentation konnte nicht aktualisiert werden
replica-unrelated = Diese beiden Präsentationen haben keinen gemeinsamen Ursprung
replica-unsupported-version = Diese Präsentation verwendet ein Deck-Format, das diese Version von Oku Slides nicht lesen kann

widget-unreadable = Die Widget-Deklarationen konnten nicht gelesen werden
widget-fault-empty-name = Widget { $widget }: Ein Widget oder Parameter hat keinen Namen
widget-fault-duplicate-widget = Widget { $widget } ist doppelt deklariert
widget-fault-duplicate-param = Widget { $widget } deklariert denselben Parameter zweimal
widget-fault-empty-choice = Widget { $widget }: Eine Auswahl hat keine Werte
widget-fault-default-not-in-choice = Widget { $widget }: Der Standardwert einer Auswahl gehört nicht zu ihren Werten
widget-fault-default-wrong-type = Widget { $widget }: Ein Standardwert entspricht nicht dem Typ seines Parameters
widget-fault-bind-not-plain = Widget { $widget }: Nur einfache Textparameter können an die Präsentation gebunden werden
widget-fault-unknown-slot-kind = Widget { $widget }: Ein Slot akzeptiert ein unbekanntes Element-Widget
widget-fault-empty-list = Widget { $widget }: Eine Liste braucht mindestens eine Spalte
widget-fault-column-type = Widget { $widget }: Eine Listenspalte kann plain, rich, number, flag, choice oder asset sein; für freien Inhalt pro Eintrag p.slot mit einem Kind-Widget verwenden
widget-fault-reserved-name = Widget { $widget }: Reservierter Name: id, kind oder ein Name mit /
widget-fault-list-bounds = Widget { $widget }: Das Maximum einer Liste liegt unter ihrem Minimum
widget-fault-list-default-too-large = Widget { $widget }: Der Standardwert einer Liste überschreitet die Größengrenzen von Listen
widget-incompatible-override = Widget { $widget } ersetzt das eingebaute Widget, ändert oder entfernt aber dessen Parameter { $param }
widget-incompatible-category = Widget { $widget } ersetzt das eingebaute Widget mit einer anderen Kategorie
issue-empty-item-id = Liste { $param } hat einen Eintrag ohne Kennung
issue-duplicate-item-id = Liste { $param } hat eine doppelte Eintragskennung
issue-list-bounds = Liste { $param } hat zu wenige oder zu viele Einträge
