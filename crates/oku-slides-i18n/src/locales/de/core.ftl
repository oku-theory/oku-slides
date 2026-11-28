op-slide-not-found = Folie nicht gefunden: { $subject }
op-element-not-found = Element nicht gefunden: { $subject }
op-duplicate-id = Kennung bereits vergeben: { $subject }
op-index-out-of-range = Position außerhalb des gültigen Bereichs
op-invalid-value = Ungültiger Wert für { $subject }
op-invalid-value-because = Ungültiger Wert für { $subject }: { $reason }
op-value-no-value = der Parameter hat keinen Wert
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
op-value-valign = top, middle oder bottom wird erwartet
op-value-path = der Pfad besteht aus den Befehlen M, L, C, Q und Z mit ihren Zahlen und beginnt mit M
knowledge-path-empty = der Pfad ist leer
knowledge-path-not-absolute = der Pfad muss mit / beginnen
knowledge-path-bad-segment = ein Segment ist leer, endet mit einem Punkt oder enthält ein Zeichen außer a-z, 0-9, Punkt, Unterstrich und Bindestrich
knowledge-path-dot-segment = ein Segment darf nicht mit einem Punkt beginnen
knowledge-path-reserved = ein Segment ist ein reservierter Systemname
knowledge-path-too-deep = der Pfad hat zu viele Segmente
knowledge-path-too-long = der Pfad ist zu lang
knowledge-path-not-markdown = der Dateiname muss auf .md enden
knowledge-frontmatter-missing = die Datei muss mit einem Frontmatter-Block zwischen --- beginnen
knowledge-frontmatter-unterminated = dem Frontmatter-Block fehlt das schließende ---
knowledge-frontmatter-too-large = der Frontmatter-Block ist zu groß
knowledge-frontmatter-not-yaml = das Frontmatter ist kein gültiges YAML
knowledge-frontmatter-not-mapping = das Frontmatter muss eine Zuordnung mit Textschlüsseln sein
knowledge-frontmatter-duplicate-key = der Frontmatter-Schlüssel „{ $key }“ kommt doppelt vor
knowledge-frontmatter-alias = das Frontmatter darf keine YAML-Anker oder -Aliase verwenden
knowledge-frontmatter-tag = das Frontmatter darf keine YAML-Tags verwenden
knowledge-frontmatter-too-deep = das Frontmatter ist zu tief verschachtelt
knowledge-frontmatter-after-document-end = das Frontmatter enthält Text nach dem YAML-Dokumentende ...
knowledge-frontmatter-flow-mapping = das Frontmatter muss ein einziges Block-Mapping sein, ein Schlüssel pro Zeile, ohne `...`-Ende
knowledge-frontmatter-type-missing = das Frontmatter braucht einen nicht leeren Typ
knowledge-frontmatter-bad-timestamp = der Frontmatter-Wert „{ $key }“ muss ein RFC-3339-Zeitpunkt mit Zeitzonenversatz sein
knowledge-frontmatter-not-allowed = der Frontmatter-Schlüssel „{ $key }“ ist in dieser Datei nicht erlaubt
knowledge-reason-not-found = keine Datei unter diesem Pfad
knowledge-reason-exists = unter diesem Pfad existiert bereits eine Datei
knowledge-reason-id-missing = eine neue Datei braucht eine ID
knowledge-reason-id-invalid = eine Datei-ID besteht aus 32 hexadezimalen Kleinbuchstaben und Ziffern
knowledge-reason-id-taken = eine andere Datei hat diese ID bereits
knowledge-reason-id-mismatch = die Datei unter diesem Pfad hat eine andere ID
knowledge-reason-edit-empty-old = der zu ersetzende Text darf nicht leer sein
knowledge-reason-edit-no-match = der zu ersetzende Text kommt in der Datei nicht vor
knowledge-reason-edit-ambiguous = der zu ersetzende Text kommt mehrfach in der Datei vor
knowledge-reason-too-many-files = das Bündel enthält zu viele Dateien
knowledge-reason-file-too-large = die Datei ist zu groß
knowledge-reason-total-too-large = das Bündel ist zu groß
knowledge-reason-over-limit = die Wissensbasis überschreitet ihre Grenzen: nur das Löschen von Dateien bringt sie wieder darunter
knowledge-no-text-layer = dieses PDF hat keine Textebene (ein Scan): exportieren Sie es mit Text oder fügen Sie seinen Text ein
knowledge-legacy-format = dieses ältere Office-Format kann nicht gelesen werden: speichern Sie es als .pptx, .docx oder .xlsx
knowledge-document-encrypted = dieses Dokument ist durch ein Passwort geschützt: speichern Sie eine ungeschützte Kopie
knowledge-unsupported-document = diese Art von Dokument kann nicht importiert werden: PDF, DOCX, PPTX, XLSX, ODS, Markdown oder Text
knowledge-document-damaged = dieses Dokument ist beschädigt und kann nicht gelesen werden
knowledge-document-type-mismatch = der Inhalt dieser Datei passt nicht zu ihrem Namen
knowledge-document-too-large = dieses Dokument ist zu groß für den Import
op-knowledge = Wissensdatei { $path }: { $reason }
op-unknown-widget = Unbekanntes Widget: { $subject }
op-unknown-namespace = Dieses Deck hat kein Paket { $pack }.
op-pack-required = Das Paket { $pack } wird von { $by } benötigt; entfernen Sie zuerst dieses.
op-pack-in-use = Das Paket { $pack } wird noch von { $count ->
    [one] { $count } Objekt
   *[other] { $count } Objekten
} verwendet.
op-pack-requirement-missing = Das Paket { $pack } benötigt { $required }, das dieses Deck nicht hat.
op-base-required = Jedes Deck behält das Basis-Widget-Paket.
op-pack-kind-conflict = Dieses Deck hat bereits ein Paket { $pack } der anderen Art (Widgets oder Layouts).
op-theme-cycle = Designs erweitern sich gegenseitig in einer Schleife: { $chain }.
op-theme-closure = Die in diesem Deck angehefteten Designs sind nicht genau sein Design und dessen Eltern ({ $namespace }).
op-unknown-param = Unbekannter Parameter: { $subject }
op-param-type-because = Ungültiger Wert für { $subject }: { $reason }
op-param-shape-text = { $nullable ->
    [yes] ein Text oder null wird erwartet
   *[no] ein Text wird erwartet
}
op-param-shape-choice = { $nullable ->
    [yes] einer seiner Werte oder null wird erwartet
   *[no] einer seiner Werte wird erwartet
}
op-param-shape-paragraphs = { $nullable ->
    [yes] eine Liste von Absätzen oder null wird erwartet
   *[no] eine Liste von Absätzen wird erwartet
}
op-param-shape-elements = { $nullable ->
    [yes] eine Liste von Elementen oder null wird erwartet
   *[no] eine Liste von Elementen wird erwartet
}
op-param-shape-flag = { $nullable ->
    [yes] ein Wahrheitswert oder null wird erwartet
   *[no] ein Wahrheitswert wird erwartet
}
op-param-shape-number = { $nullable ->
    [yes] eine Zahl oder null wird erwartet
   *[no] eine Zahl wird erwartet
}
op-param-shape-bounded-number = { $nullable ->
    [yes] eine Zahl innerhalb seiner Grenzen oder null wird erwartet
   *[no] eine Zahl innerhalb seiner Grenzen wird erwartet
}
op-param-shape-grid = { $nullable ->
    [yes] ein Raster oder null wird erwartet
   *[no] ein Raster wird erwartet
}
op-param-shape-chart-option = { $nullable ->
    [yes] eine Diagrammoption oder null wird erwartet
   *[no] eine Diagrammoption wird erwartet
}
op-param-shape-table-option = { $nullable ->
    [yes] eine Tabellenoption oder null wird erwartet
   *[no] eine Tabellenoption wird erwartet
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] eine Diagramm- oder Tabellenoption oder null wird erwartet
   *[no] eine Diagramm- oder Tabellenoption wird erwartet
}
op-param-shape-items = { $nullable ->
    [yes] eine Liste von Einträgen oder null wird erwartet
   *[no] eine Liste von Einträgen wird erwartet
}
op-param-shape-list = { $nullable ->
    [yes] eine Liste oder null wird erwartet
   *[no] eine Liste wird erwartet
}
op-param-shape-add-element-only = { $nullable ->
    [yes] Elemente werden mit add_element hinzugefügt, oder null
   *[no] Elemente werden mit add_element hinzugefügt
}
op-param-shape-set-param-only = { $nullable ->
    [yes] ein Wert wird mit set_param gesetzt, oder null
   *[no] ein Wert wird mit set_param gesetzt
}
op-param-shape-path-option = { $nullable ->
    [yes] ein Formpfad (Objekt mit d) oder null wird erwartet
   *[no] ein Formpfad (Objekt mit d) wird erwartet
}
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
deck-inline-data = Element { $element } speichert seine Daten in seinen Optionen: Diese Deck-Datei hat nicht das aktuelle Format
sources-not-sources = Dies ist kein Oku-Slides-Quellarchiv
sources-unsupported-version = Diese Quellen wurden mit einer neueren Version von Oku Slides erstellt
sources-bad-name = Unzulässiger Dateiname in den Quellen
sources-missing-deck = Die Präsentation fehlt in den Quellen
sources-too-large = Die Quellen in dieser Datei sind zu groß zum Öffnen
sources-too-deep = Dieses Deck verschachtelt Elemente zu tief
sources-base-refused = Eine Datendatei dieses Decks ist beschädigt oder konnte nicht gespeichert werden

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
issue-invalid-style = ungültiger Stilwert: { $field } ({ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    })
issue-missing-asset = fehlende Ressource: { $asset }
issue-pack-version-mismatch = veränderte Paketversion verworfen: { $version }
issue-pack-version-missing = Paketversion fehlt in den Quellen: { $version }
issue-pack-pin-mismatch = Das eingebettete Paket { $pack } entspricht nicht dem, was die Präsentation erwartet.
issue-pack-tokens-invalid = Das eingebettete Paket { $pack } enthält ein ungültiges Komponenten-Token: { $reason }
issue-pack-kind-mismatch = Das eingebettete Paket { $pack } ist nicht von der Art, die die Präsentation erwartet (Widgets oder Layouts).
issue-pack-kind-conflict = Der Paket-Namensraum { $pack } ist zugleich als Widget-Paket und als Layout-Paket angeheftet: Nur eines kann verwendet werden.
issue-pack-requirement-missing = Das Paket { $pack } benötigt { $required }, das dieses Deck nicht hat.
issue-theme-version-mismatch = veränderte Designversion verworfen: { $version }
issue-theme-version-missing = Designversion fehlt in den Quellen: { $version }
issue-theme-pin-mismatch = Das eingebettete Design { $namespace } entspricht nicht dem, was die Präsentation erwartet.
issue-theme-closure = Die in diesem Deck angehefteten Designs sind nicht genau sein Design und dessen Eltern ({ $namespace }).
issue-usage-line-invalid = Zeile { $line } des Nutzungsprotokolls unlesbar, verworfen
issue-usage-line-forged = Nutzungsdurchlauf { $turn } zweimal mit unterschiedlichem Inhalt erfasst, einer verworfen
issue-usage-too-large = Nutzungsprotokoll zu groß, verworfen
issue-usage-unreadable = Nutzungsprotokoll unlesbar, verworfen
issue-usage-lines-dropped = { $count ->
    [one] { $count } weitere Zeile des Nutzungsprotokolls verworfen
   *[other] { $count } weitere Zeilen des Nutzungsprotokolls verworfen
}
usage-range-bad-date = „{ $date }“ ist kein Datum: Verwenden Sie JJJJ-MM-TT.
usage-range-reversed = Das Startdatum { $since } liegt nach dem Enddatum { $until }.
pack-token-entry-invalid = Ungültiges Komponenten-Token „{ $path }“ im Paketmanifest.
pack-manifest-json = Das Paketmanifest kann nicht gelesen werden: { $reason }
pack-manifest-order = Die Reihenfolge der Elemente des Packs ist bei „{ $name }“ ungültig.
pack-manifest-data-count = Das Pack enthält { $count } Daten, höchstens { $max }.
pack-manifest-data-base = Die Daten „{ $data }“ des Packs verweisen auf eine externe Datei; die Daten eines Packs enthalten ihre eigenen Zellen.
pack-manifest-data-cells = Die Daten des Packs enthalten { $count ->
    [one] { $count } Zelle
   *[other] { $count } Zellen
}, höchstens { $max }.
issue-unsupported-link = nicht unterstützter Link: { $link }
issue-highlight-zero = Hervorhebungsfarben werden ab 1 nummeriert
issue-unreadable-element = unlesbares Element, beiseitegelegt
issue-slot-repaired = Element ohne bekannten Slot, abgelegt in { $slot }
issue-unreadable-item = unlesbarer Listeneintrag, beiseitegelegt
issue-unreadable-row = unlesbare Tabellenzeile, beiseitegelegt
issue-data-truncated = Daten „{ $data }“ zu groß, beim Lesen gekürzt und schreibgeschützt behalten
issue-data-name-taken = der Datenname „{ $data }“ ist bereits vergeben
issue-data-cell-type = Daten „{ $data }“: Spalte „{ $column }“ enthält Werte, die nicht vom Typ { $type } sind
issue-data-too-many = zu viele Daten im Deck ({ $count }); nicht gelesen: { $dropped }
issue-data-base-missing = Daten „{ $data }“: ihre Basisdatei fehlt, schreibgeschützt behalten
issue-data-edits-lost = Daten „{ $data }“: Änderungen an einer ersetzten Tabelle gingen beim Zusammenführen verloren
issue-data-inline-dropped = Element { $element }: Die Daten seines Diagramms oder seiner Tabelle wurden von einer älteren Version in seinen Optionen gespeichert; sie wurden entfernt
issue-data-kind-unknown = Daten „{ $data }“ haben die unbekannte Art „{ $kind }“, vielleicht aus einer neueren Version; nicht gelesen
issue-data-query-cycle = Daten „{ $data }“: ihre Abfrage liest sich selbst wieder, direkt oder über andere Abfragen; nicht berechnet
issue-data-query-missing-input = Daten „{ $data }“: ihre Abfrage liest „{ $input }“, das nicht im Deck ist; nicht berechnet
issue-data-query-invalid = Daten „{ $data }“: ihre Abfrage ist ungültig ({ $reason }); nicht berechnet
issue-data-query-failed = Daten „{ $data }“: ihre Abfrage ist fehlgeschlagen ({ $reason }); kein Ergebnis
issue-more = weitere, nicht angezeigte Probleme: { $count }
op-data-deck-full = Die Daten des Decks würden ihr Limit überschreiten: höchstens { $max } { $what ->
    [row-ids] Zeilen-IDs
    [overlay] geänderte Zellen
    [text] Byte Text
   *[data] Daten
}
op-data-missing = Daten „{ $data }“ nicht gefunden
op-data-name-taken = Der Datenname „{ $name }“ ist bereits vergeben
op-data-column-in-use = Die Spalte „{ $column }“ wird von { $views } angezeigt; binden Sie diese zuerst an andere Spalten (bind_data mit columns)
op-data-in-use = Diese Daten werden von { $views }{ $more ->
    [0] {""}
    [one] {" "}und { $more } weiteren Ansicht
   *[other] {" "}und { $more } weiteren Ansichten
}{" "}angezeigt; entfernen Sie diese oder binden Sie sie zuerst an andere Daten
op-data-column-missing = Die Daten „{ $data }“ haben keine Spalte „{ $column }“
op-data-not-view = Der Parameter „{ $param }“ kann keine Daten anzeigen: nur eine Diagramm- oder Tabellenoption kann das
op-data-truncated = Die Daten „{ $data }“ wurden beim Lesen gekürzt und sind schreibgeschützt; zum Bearbeiten die ganze Tabelle ersetzen
op-data-rebind = Das Element { $subject } zeigt bereits andere Daten; zum Ändern bind_data verwenden
op-data-internal = Ein Stapel kann Basis oder Überlagerung einer Tabelle nicht setzen
op-data-batch-too-large = Der Stapel ist zu groß: Zellenanzahl { $count }, höchstens { $max }
op-data-inline = Element { $subject } trägt seine Daten in seinen Optionen; fügen Sie sie mit add_data als Deck-Daten hinzu und binden Sie sie mit bind_data
op-data-derived = die Daten „{ $data }“ werden aus einer Abfrage abgeleitet: bearbeiten Sie die Abfrage, nicht die Zeilen
op-data-source = Daten haben entweder eine Tabelle oder eine Abfrage (sql), genau eines davon
op-data-cycle = Die Daten „{ $data }“ würden sich über ihre Abfragen selbst wieder lesen; entfernen Sie die Schleife
op-data-unknown-input = Die Abfrage liest „{ $name }“, das keine Daten des Decks sind
op-data-not-query = Die Daten „{ $data }“ sind keine Abfrage; bearbeiten Sie ihre Tabelle mit edit_data
op-data-read-by-queries = Diese Daten werden von den Abfragen { $queries } gelesen{ $shown ->
    [0] {""}
   *[other] {" "}und von { $views } angezeigt
}{ $more ->
    [0] {""}
   *[other] {" "}und { $more } weiteren
}; bearbeiten oder entfernen Sie diese zuerst
op-data-rename-captured = Die Abfrage von „{ $reader }“ nennt bereits eine ihrer eigenen Tabellen „{ $name }“; benennen Sie diese Tabelle in der Abfrage um (set_query) oder wählen Sie einen anderen Namen
op-data-name-ambiguous = Mehrere Daten heißen „{ $name }“, und Abfragen lesen diesen Namen, ohne dass klar ist, welche; bearbeiten Sie zuerst diese Abfragen (set_query)
issue-unreadable-slide = unlesbare Folie, beiseitegelegt
issue-unreadable-field = unlesbares Deck-Feld { $field }, zurückgesetzt
issue-knowledge-unreadable = unlesbarer Wissenseintrag { $id }, beiseitegelegt
issue-knowledge-entries-dropped = Wissenseinträge ungelesen verworfen: { $count } ({ $cause ->
        [files] zu viele Dateien zum Lesen
        [file-bytes] Datei zu groß zum Lesen
        [total-bytes] Sammlung zu groß zum Lesen
       *[unreadable] weitere unlesbare Einträge
    })
issue-knowledge-over-limit = Wissenssammlung über einer Grenze: { $limit ->
        [knowledge-reason-too-many-files] { knowledge-reason-too-many-files }
        [knowledge-reason-file-too-large] { knowledge-reason-file-too-large }
       *[knowledge-reason-total-too-large] { knowledge-reason-total-too-large }
    } (betroffene Dateien: { $count }, die erste { $id }); alle Dateien bleiben erhalten; nur das Löschen von Dateien bringt das Bündel wieder unter die Grenze (Schreib- und Bearbeitungsvorgänge werden bis dahin abgelehnt)
issue-knowledge-path-invalid = Wissensdatei { $id } nach /recovered verschoben: { $reason ->
        [knowledge-path-empty] { knowledge-path-empty }
        [knowledge-path-not-absolute] { knowledge-path-not-absolute }
        [knowledge-path-bad-segment] { knowledge-path-bad-segment }
        [knowledge-path-dot-segment] { knowledge-path-dot-segment }
        [knowledge-path-reserved] { knowledge-path-reserved }
        [knowledge-path-too-deep] { knowledge-path-too-deep }
        [knowledge-path-too-long] { knowledge-path-too-long }
       *[knowledge-path-not-markdown] { knowledge-path-not-markdown }
    }
issue-knowledge-path-collision = mehrere Wissensdateien lagen unter { $path }: die anderen wurden umbenannt
issue-knowledge-invalid = Wissensdatei { $path } ist ungültig: { $reason ->
        [knowledge-frontmatter-missing] { knowledge-frontmatter-missing }
        [knowledge-frontmatter-unterminated] { knowledge-frontmatter-unterminated }
        [knowledge-frontmatter-too-large] { knowledge-frontmatter-too-large }
        [knowledge-frontmatter-not-yaml] { knowledge-frontmatter-not-yaml }
        [knowledge-frontmatter-not-mapping] { knowledge-frontmatter-not-mapping }
        [knowledge-frontmatter-duplicate-key] { knowledge-frontmatter-duplicate-key }
        [knowledge-frontmatter-alias] { knowledge-frontmatter-alias }
        [knowledge-frontmatter-tag] { knowledge-frontmatter-tag }
        [knowledge-frontmatter-too-deep] { knowledge-frontmatter-too-deep }
        [knowledge-frontmatter-after-document-end] { knowledge-frontmatter-after-document-end }
        [knowledge-frontmatter-flow-mapping] { knowledge-frontmatter-flow-mapping }
        [knowledge-frontmatter-type-missing] { knowledge-frontmatter-type-missing }
        [knowledge-frontmatter-bad-timestamp] { knowledge-frontmatter-bad-timestamp }
       *[knowledge-frontmatter-not-allowed] { knowledge-frontmatter-not-allowed }
    }
op-invalid-chart = Diagramm { $subject }: { $reason }
op-invalid-table = Tabelle { $subject }: { $reason }

replica-corrupted = Der Verlauf der Präsentation in dieser Datei ist beschädigt
replica-too-large = Der Verlauf der Präsentation in dieser Datei ist zu groß zum Öffnen
replica-write = Der Verlauf der Präsentation konnte nicht aktualisiert werden
replica-unrelated = Diese beiden Präsentationen haben keinen gemeinsamen Ursprung
replica-unsupported-version = Diese Präsentation verwendet ein Deck-Format, das diese Version von Oku Slides nicht lesen kann
replica-too-deep = Diese Präsentation kann nicht gespeichert werden: Gelöschte Elemente sind in ihrem Verlauf zu tief verschachtelt. Machen Sie Ihre letzten Verschiebungen rückgängig und speichern Sie dann erneut.

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
widget-fault-column-type = Widget { $widget }: Eine Listenspalte kann plain, rich, number, flag, choice oder asset (oder eine verschachtelte p.list, eine Ebene) sein; für freien Inhalt pro Eintrag p.slot mit einem Kind-Widget verwenden
widget-fault-reserved-name = Widget { $widget }: Reservierter Name: id, kind oder ein Name mit /
widget-fault-list-bounds = Widget { $widget }: Das Maximum einer Liste liegt unter ihrem Minimum
widget-fault-list-default-too-large = Widget { $widget }: Der Standardwert einer Liste überschreitet die Größengrenzen von Listen
widget-fault-list-depth = Widget { $widget }: Eine verschachtelte Liste darf keine Listenspalte enthalten; Listen lassen sich nur eine Ebene tief verschachteln
widget-fault-list-default-below-min = Widget { $widget }: Der Standardwert einer verschachtelten Liste hat weniger Einträge als ihr Minimum
widget-fault-slide-sizing = Widget { $widget } ist eine Folie: nur Element-Widgets deklarieren grow oder typography
widget-fault-pack-category = Widget { $widget } passt nicht zu seinem Pack: Layout-Packs enthalten Folien-Widgets, Widget-Packs Element-Widgets
widget-fault-unqualified = Widget { $widget } hat keinen Pack-Namensraum
widget-fault-too-long = Widget { $widget }: ein Name, eine Beschriftung, ein Wert, eine Beschreibung oder eine Liste überschreitet die Grenze
issue-empty-item-id = Liste { $param } hat einen Eintrag ohne Kennung
issue-duplicate-item-id = Liste { $param } hat eine doppelte Eintragskennung
issue-invalid-item-id = Liste { $param } hat eine Eintragskennung mit einem Schrägstrich
issue-list-bounds = Liste { $param } hat zu wenige oder zu viele Einträge
issue-view-truncated = { $element } zeigt nur einen Teil seiner Daten ({ $what ->
    [rows] Zeilen
   *[columns] Spalten
}: { $shown } von { $total })
issue-render-budget = { $element } wird nicht gezeichnet: die Ansichten des Decks lesen mehr Daten, als ein Rendern erlaubt
issue-data-missing = { $element } nennt die Daten „{ $data }“, die nicht im Deck sind
issue-view-column-missing = { $element } liest die Spalte „{ $column }“, die seine Daten nicht haben
issue-view-unreadable = { $element } wird nicht gezeichnet: seine Optionen sind nicht lesbar
op-replica-full = Der Verlauf der Präsentation erreicht fast { $max } MiB, das Maximum zum erneuten Öffnen: Entfernen Sie Inhalte, bevor Sie weitere hinzufügen.
