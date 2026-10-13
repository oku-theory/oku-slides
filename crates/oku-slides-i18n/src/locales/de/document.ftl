pdf-unreadable = PDF nicht lesbar
pdf-unsupported-version = Dieses PDF wurde mit einer neueren Version von Oku Slides erstellt
pdf-corrupted = Die Quellen in diesem PDF sind beschädigt
pdf-write = Das PDF konnte nicht geschrieben werden

render-compile = Die Präsentation konnte nicht gerendert werden
render-pdf = Der PDF-Export ist fehlgeschlagen
render-no-page = Diese Seite existiert nicht
render-setup = Der Renderer konnte nicht starten
render-image = Das Bild der Folie konnte nicht erzeugt werden
render-missing-image = fehlendes Bild
diagnostic-error = Fehler
diagnostic-warning = Warnung
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = Hinweis: { $hint }
render-chart-failed = Diagramm konnte nicht gezeichnet werden

registry-bad-pack-id = Ungültige Vorlagenpaket-Kennung: { $id }
registry-compile = Die Widgets des Vorlagenpakets konnten nicht kompiliert werden
registry-missing = Das Vorlagenpaket deklariert keine Widgets
render-timeout = Das Rendern dauerte länger als { $seconds } s und wurde abgebrochen
render-out-of-memory = Das Rendern benötigte mehr als { $mib } MiB Speicher und wurde abgebrochen
render-worker-lost = Der Rendering-Prozess wurde beendet
render-frame = Unlesbare Nachricht des Rendering-Prozesses
render-page-too-large = Diese Folie ist zu groß für die Vorschau (mehr als { $mib } MiB, oft wegen eines sehr großen Bildes). Sie lässt sich weiterhin als PDF oder PNG exportieren.
render-stale = Das Rendern wurde ersetzt; bitte erneut versuchen
render-pack-disabled = Die Widgets des Pakets „{ $pack }“ sind für diese Sitzung deaktiviert
registry-schema-remote = Ungültige Widget-Registry: { $message }
probe-not-probed = Noch nicht geprüft: Die Zeit für die Widgets des Pakets ist abgelaufen. Die Prüfung folgt beim nächsten Rendern.
probe-too-many-files = Das Paket hat mehr als { $max } Widget-Dateien: Diese wird nicht geladen.
probe-too-large = Größer als { $max } KiB: nicht geladen.

import-unreadable = Diese Präsentation kann nicht gelesen werden
import-not-a-presentation = Diese Datei ist keine PowerPoint-Präsentation
import-protected = Diese Präsentation ist verschlüsselt oder im alten .ppt-Format: Speichern Sie sie zuerst als .pptx
import-too-large = Diese Präsentation ist zu groß für den Import
import-loss-list-levels = Listenebenen auf eine Ebene reduziert
import-loss-merged-cells = Verbundene Zellen aufgelöst
import-loss-percent-stacked = 100-%-gestapeltes Diagramm als gestapelt dargestellt
import-loss-run-style = { $effect ->
    [underline] Unterstreichung verloren
    [strike] Durchstreichung verloren
    [baseline] Hoch- oder Tiefstellung verloren
    [caps] Großbuchstaben verloren
    [font] Schriftart eines Abschnitts durch die des Felds ersetzt
    [tracking] Zeichenabstand eines Abschnitts durch den des Felds ersetzt
    [weight] Normaler Abschnitt wie das Feld fett dargestellt
    [slant] Aufrechter Abschnitt wie das Feld kursiv dargestellt
   *[highlight] Hervorhebung verloren
}
import-loss-transform = Drehung, Spiegelung oder Zuschnitt verloren
import-loss-color-dropped = Die Farbe { $color } hat keine ähnliche Designfarbe: Designfarbe verwendet
import-loss-text-truncated = Text gekürzt, um die Grenzen einzuhalten
import-loss-table-truncated = Tabelle gekürzt, um die Grenzen einzuhalten
import-loss-opaque = { $reason ->
    [media] Video oder Ton nicht importiert (kein Ersatzbild)
    [ole] Eingebettetes Objekt nicht importiert (kein Ersatzbild)
    [math] Formel nicht importiert (kein Ersatzbild)
    [smartart] SmartArt nicht importiert (kein Ersatzbild)
   *[unknown] Nicht unterstütztes Objekt nicht importiert (kein Ersatzbild)
}
import-loss-unsupported-image = Bild im Format { $format } nicht importiert
import-loss-hidden-slide = Ausgeblendete Folie als sichtbar importiert
import-loss-effects = Schatten, Spiegelungen und 3D-Effekte verloren
import-loss-series-dropped = Ein Kreisdiagramm zeigt nur eine Datenreihe: { $kept } behalten, { $dropped } verworfen
import-loss-repeated-decor = { $slides ->
   [one] Kopf- oder Fußzeile „{ $text }“ auf { $slides } Folie wiederholt, weggelassen: das Design zeichnet eigene
  *[other] Kopf- oder Fußzeile „{ $text }“ auf { $slides } Folien wiederholt, weggelassen: das Design zeichnet eigene
}
import-loss-no-slot = Inhalt weggelassen: die gewählte Vorlage hat keinen Platz dafür
import-loss-page-fitted = Quellseite { $width } × { $height } pt ist nicht 16:9: Inhalt skaliert und zentriert
import-loss-paragraph-style = Ausrichtung, Zeilen- oder Absatzabstand eines Absatzes an das Feld angeglichen
import-loss-shape-style = { $effect ->
    [gradient] Farbverlauf als Volltonfarbe dargestellt
    [picture] Füllbild auf den Rahmen der Form gestreckt
    [dash] Gestrichelte Linie durchgehend dargestellt
   *[geometry] Unbekannte Form als Rechteck dargestellt
}
import-loss-layout-dropped = Zu viele Layouts für ein Paket: „{ $layout }“ hat keine Vorlage; seine Folien nutzen das leere Widget und erhalten seine Dekoration als Elemente
import-loss-decor-truncated = Dekoration gekürzt, um in die Größe einer Paketdatei zu passen: { $omitted } Elemente ausgelassen
import-loss-table-style = Tabellenstil angenähert: eigene Zellhintergründe, Rahmen, Spaltenbreiten oder Textgrößen gehen verloren
import-font-substituted = { $family } ersetzt durch { $by } mit passenden Metriken
import-font-missing = { $family } nicht gefunden: der Text verwendet { $by }
import-font-restricted = { $family }: die Lizenz verbietet, einige Schnitte in die Präsentation einzubetten
import-font-restricted-substituted = { $family } darf nicht in die Präsentation eingebettet werden (Lizenz): ersetzt durch { $by } mit passenden Metriken
import-font-restricted-missing = { $family } darf nicht in die Präsentation eingebettet werden (Lizenz): der Text verwendet { $by }
import-font-too-large = { $family }: Dateien zu groß, um sie in das Deck einzubetten; der Text verwendet { $by }
import-font-missing-no-fallback = { $family } nicht gefunden, ebenso wenig die Fließtextschrift des Themes: der Text wird mit der Standardschrift der Engine dargestellt
import-warning-effect-ignored = Effekt nicht importiert: { $detail }
import-warning-approximated = Näherungsweise dargestellt: { $detail }
import-warning-unreadable-item = Unlesbares Element übersprungen: { $detail }
import-warning-unreadable-part = Unlesbarer Teil: { $detail }
import-warning-missing-part = Fehlender Teil: { $detail }
import-warning-unsupported-chart = Nicht unterstütztes Diagramm: { $detail }
import-warning-unsupported-image = Bildformat nicht unterstützt: { $detail }
import-warning-unknown-table-style = Unbekannte Tabellenformatvorlage: { $detail }
import-warning-unsafe-link = Link abgelehnt: { $detail }
import-warning-nesting-too-deep = Verschachtelung zu tief, abgeschnitten: { $detail }
import-warning-font-embedded-unreadable = Eingebettete Schriftart unlesbar: { $detail }
import-warning-truncated = Zu viele Warnungen: weitere werden ausgelassen
