cli-reading = lese { $path }
cli-invalid-deck = ungültige Präsentation: { $path }
cli-deck-valid = gültige Präsentation: { $slides } Folien
cli-deck-issue = Problem: { $issue }
cli-rendered = { $pages } Seiten → { $out } (Kompilierung { $compile }, SVG { $svg })
cli-asset-added = Ressource { $path } → { $id }
cli-saved = { $path }: { $pages } Seiten, PDF { $pdf } KB, Quellen { $sources } KB
cli-opened = { $path }: { $slides } Folien, { $assets } Ressourcen, { $fonts } Schriften
cli-extracted = extrahiert nach { $dir }
cli-stripped = { $path }: Kopie ohne Quellen
cli-pdf-modified = das PDF wurde nach Oku Slides mit einem anderen Programm geändert
cli-iterations = --iterations muss > 0 sein
cli-no-block = die Präsentation hat keinen bearbeitbaren Block
cli-blob-missing = Quellen-Blob nicht gefunden
cli-roundtrip-mismatch = die erneut gelesene Präsentation weicht von der gespeicherten ab
cli-bench-world = Aufbau der Umgebung (Schriften)  { $time }
cli-bench-first = erste Kompilierung               { $time }  ({ $pages } Seiten)
cli-bench-same = Neukompilierung ohne Änderung    { $time }
cli-bench-edited-text = Bearbeiteter Text { $n }
cli-bench-edit = Neukompilierung nach set_text    { $time }
cli-bench-svg = SVG aller Folien                 { $time }  ({ $each } pro Folie)
cli-bench-pdf = PDF-Export                       { $time }  ({ $size } KB)
cli-bench-embed = Einbetten des Quellen-Blobs      { $time }
cli-bench-extract = Zurücklesen des Blobs            { $time }
cli-bench-roundtrip = Hin- und Rückweg der Präsentation identisch
cli-bench-history = Verlaufsschritt               { $mean } ms im Durchschnitt, p50 { $p50 } ms
cli-serving = Oku Slides (dev) unter http://{ $host }:{ $port }
cli-models-dir = Diktiermodelle: { $dir }
cli-stopped = Server gestoppt
cli-not-found = nicht gefunden
cli-bad-request = ungültige Anfrage: { $reason }
cli-sse-full = zu viele Ereignisströme bereits geöffnet
cli-forbidden = Anfrage abgelehnt: sie stammt nicht von der lokalen Oberfläche
cli-exposed = lauscht auf { $host }: der Server hat keine Authentifizierung und ist aus dem Netzwerk erreichbar; jeder Client, der einen lokalen Host-Header sendet, wird bedient. Belassen Sie [serve] host auf 127.0.0.1
cli-imported = { $path }: { $slides ->
       [one] { $slides } Folie
      *[other] { $slides } Folien
    }, { $pages ->
       [one] { $pages } Seite
      *[other] { $pages } Seiten
    }, { $losses ->
       [one] { $losses } Verlust
      *[other] { $losses } Verluste
    }, { $requests ->
       [one] { $requests } Widget-Anfrage
      *[other] { $requests } Widget-Anfragen
    }
cli-import-label = PowerPoint-Import
cli-import-pack-required = --pack ist im Modus retrofit erforderlich: ID, Ordner oder Name eines Vorlagenpakets
cli-import-unknown-pack = unbekanntes Vorlagenpaket: { $pack }
cli-import-bad-pack = unlesbares Vorlagenpaket { $pack }: seine pack.json beschreibt kein Vorlagenpaket
cli-import-write-failed = das importierte Deck { $path } konnte nicht geschrieben werden
cli-import-hifi-no-target = --pack und --theme gelten nur im Modus retrofit: der originalgetreue Import (--mode hifi) erzeugt sein eigenes Paket und Design
cli-import-probe-mismatch = das erzeugte Paket entspricht bei der Prüfung nicht seiner Deklaration (Widget { $widget }): ein Fehler des Imports
cli-import-probe-limit = das erzeugte Paket überschreitet die Grenzen der Prüfung (Widget { $widget }): erhöhen Sie max_widgets, max_widget_kib oder probe_budget_ms im Abschnitt [render] der Konfiguration
cli-import-reference-hifi-only = --reference gehört zum hifi-Modus: ein Retrofit-Import wird nicht mit einem Referenz-Rendering verglichen
cli-import-bad-threshold = score_threshold im Abschnitt [import] muss eine Zahl größer oder gleich null sein, nicht { $value }
cli-import-scored = Visuelle Abweichung zu { $witness }: { $mean } im Mittel, { $above ->
    [0] keine Folie
    [one] { $above } Folie
   *[other] { $above } Folien
} über { $threshold }
cli-import-unscored = { $reason }

cli-about = Oku-Slides-Präsentationen rendern und speichern
cli-arg-fonts = Zusätzlicher Schriftordner (TTF/OTF); Standard: paths.fonts in den Einstellungen
cli-cmd-check = Eine Präsentation prüfen
cli-arg-deck = Präsentation (JSON)
cli-cmd-render = Jede Folie als SVG rendern, mit den Elementrahmen
cli-arg-input = deck.json- oder .oku-slides.pdf-Datei
cli-arg-out = Ausgabeordner
cli-arg-theme = Das Design der Präsentation ersetzen: ein mitgeliefertes oder eines aus der Bibliothek, per Name
cli-cmd-save = Eine Präsentation als .oku-slides.pdf speichern (PDF + Quellen-Blob)
cli-arg-output = Zu schreibende Datei
cli-arg-assets = Ressourcenordner: Jede Datei wird zu einer sha256:…-Ressource
cli-arg-embed-fonts = Auch die Schriften aus --fonts in die Quellen einbetten
cli-cmd-open = Eine .oku-slides.pdf erneut öffnen: Präsentation und Ressourcen extrahieren
cli-arg-extract = Ordner, in den deck.json, assets/ und fonts/ geschrieben werden
cli-cmd-strip = Eine .oku-slides.pdf ohne Quellen-Blob kopieren
cli-cmd-bench = Kompilierungs- und Exportzeiten messen
cli-arg-iterations = Messungen pro Schritt
cli-cmd-schema = Das JSON-Schema der Operationen ausgeben
cli-cmd-serve = Entwicklungsserver: die Oberfläche im Browser, ohne Tauri
cli-arg-serve-input = Zu öffnende Präsentation (.oku-slides.pdf oder .json)
cli-arg-port = Port; Standard: serve.port in den Einstellungen
cli-arg-ui = Ordner der gebauten Oberfläche (ui/dist)
cli-cmd-import = Eine PowerPoint-Präsentation (.pptx) als .oku-slides.pdf importieren
cli-arg-pptx = PowerPoint-Präsentation (.pptx)
cli-arg-mode = retrofit: der Inhalt auf einem vorhandenen Vorlagenpaket und Design; hifi: originalgetreue Kopie, auf einem eigens erzeugten Paket und Design
cli-arg-pack = Ziel-Vorlagenpaket: ID, Paketordner (pack.json, widgets/) oder Name
cli-arg-import-theme = Ziel-Design: ein mitgeliefertes oder eines aus der Bibliothek, per ID oder Name; Standard: das Standarddesign
cli-arg-report = Zusätzlich den Importbericht (JSON) schreiben: Verluste, Ersatzlösungen, Abdeckung
cli-arg-no-system-fonts = Im Modus hifi die Schriften der Präsentation nicht unter den Systemschriften suchen
cli-arg-reference = Im hifi-Modus jede Folie mit einem Referenz-Rendering der Präsentation vergleichen (auto, Standard) oder nicht (none)
