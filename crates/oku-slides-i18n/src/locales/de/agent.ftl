# Agent: Oberflächen, Bestätigungen, Verlauf.

agent-surface-cancel = Abbrechen
agent-surface-confirm = Bestätigen
agent-surface-confirm-danger = Ja, tun
agent-surface-rewrite = Umformulierung
agent-surface-pick = { $n } wählen
agent-surface-keep = Original behalten
agent-surface-edit = Vor dem Anwenden bearbeiten
agent-surface-close = Schließen
agent-surface-themes = Themen
agent-surface-variants = Varianten
agent-surface-permission-always = für immer
agent-surface-permission-title = Der Agent bittet um eine Berechtigung
agent-undo-turn = Agent: { $text }

agent-tools-unavailable = Die Werkzeuge des Agenten konnten nicht gestartet werden.

agent-undo-apply = Änderung durch den Agenten
agent-undo-variant = Variante: { $label }
agent-undo-param = Einstellung: { $label }
agent-undo-rewrite = Umformulierung

agent-confirm-save = Präsentation als { $path } speichern?
agent-confirm-save-over = { $path } ersetzen? Die vorhandene Datei wird überschrieben.
agent-confirm-open = { $path } öffnen?
agent-open-elsewhere = Diese Datei ist bereits in einem anderen Tab geöffnet.
agent-confirm-new = Neue Präsentation beginnen?
agent-confirm-new-dirty = Neue Präsentation beginnen? Nicht gespeicherte Änderungen gehen verloren.
agent-confirm-save-pack = Paket „{ $name }“ speichern?
agent-confirm-delete-pack = Paket „{ $name }“ löschen? Das kann nicht rückgängig gemacht werden.
agent-confirm-edit-theme = Eine neue Version des Designs „{ $name }“ speichern?
agent-confirm-copy-theme = Eine Variante des integrierten Designs „{ $name }“ erstellen? Das integrierte Design bleibt unverändert.
agent-confirm-setting = { $key } ändern: { $old } → { $new }?
agent-confirm-move = Die Präsentation nach { $path } verschieben? Die alte Datei wird gelöscht.
agent-confirm-merge = { $path } mit dieser Präsentation zusammenführen? Das lässt sich nicht rückgängig machen.
agent-confirm-save-copy = Eine Kopie der Präsentation als { $path } speichern?
agent-confirm-dictation-api = Das Diktierte an { $host } ({ $url }), Modell { $model }, mit dem Geheimnis aus { $var } senden?
agent-confirm-dictation-download = { $name } verwenden und jetzt herunterladen ({ $size })?
agent-confirm-dictation-language = Apple-Diktat verwenden und seine Sprache jetzt herunterladen?
agent-confirm-agent = Den App-Agent auf { $name } ({ $command }) umstellen, sobald keine Antwort läuft?
agent-confirm-agent-builtin = Den App-Agent auf { $name } umstellen, sobald keine Antwort läuft?
agent-confirm-agent-model = Das Modell des lokalen Agents herunterladen ({ $size })?

agent-not-found = Agent nicht gefunden: { $command }. Einstellung [agent.presets].
agent-spawn-failed = Agent konnte nicht gestartet werden: { $command }. Einstellung [agent.presets].
agent-no-http = Dieser Agent kann sich nicht mit den Werkzeugen von Oku Slides verbinden.
agent-local-no-program = Der lokale Agent läuft nur innerhalb der App Oku Slides.
agent-local-needs-download = Der lokale Agent muss zuerst sein Modell herunterladen ({ $gigabytes } GB).
agent-local-intel-mac = Der lokale Agent benötigt einen Mac mit Apple-Chip.
agent-local-unsupported-os = Der lokale Agent ist auf diesem System noch nicht verfügbar.
agent-model-download = Modell herunterladen
agent-model-downloading = Modell wird heruntergeladen… { $percent } %
agent-model-cancel = Abbrechen
agent-model-download-failed = Das Modell konnte nicht heruntergeladen werden.
agent-model-download-integrity = Das heruntergeladene Modell war beschädigt und wurde gelöscht.
agent-model-download-incomplete = Download unvollständig: { $got } von { $expected } Bytes.
agent-model-download-cancelled = Download abgebrochen.
local-agent-no-route = Der lokale Agent hat nicht verstanden, was zu tun ist. Versuchen Sie, es anders zu sagen.
local-agent-tool-limit = Der lokale Agent hat nach { $n } Schritten aufgehört, ohne fertig zu werden.
local-agent-invalid-call = Der lokale Agent konnte die Werkzeuge nicht richtig verwenden.
local-agent-engine-failed = Das Modell des lokalen Agenten ist fehlgeschlagen.
local-agent-retry = Versuchen Sie es erneut oder wählen Sie im Agent-Bereich einen anderen Agenten.
agent-stopped = Der Agent wurde beendet.
agent-failed = Der Agent hat die Anfrage abgelehnt: { $message }
agent-tool-failed = Das Werkzeug ist unerwartet fehlgeschlagen.

agent-turn-tools = { $tools ->
   [one] { $tools } Werkzeug
  *[other] { $tools } Werkzeuge
}
agent-turn-notes = { $notes ->
   [one] { $notes } Notiz
  *[other] { $notes } Notizen
}
agent-turn-secs = { $secs } s
agent-turn-stopped = Gestoppt.
agent-turn-interrupted = Unterbrochen.
agent-memory-lost = Der Agent erinnert sich nicht mehr an dieses Gespräch: Er fängt von vorn an.
agent-turn-max-tokens = Der Agent hat sein Längenlimit erreicht.
agent-turn-max-turns = Der Agent hat sein Schrittlimit erreicht.
agent-turn-refusal = Der Agent hat abgelehnt.
agent-turn-ended = Runde beendet ({ $stop }).
agent-choice-cancelled = Frage abgebrochen
agent-choice-answered = ✓ { $answer }
agent-report-why = Warum

agent-error-file = Die Datei konnte nicht gelesen oder geschrieben werden.
agent-error-unexpected-action = Unerwartete Aktion: { $action }.
agent-error-unknown-surface = Diese Karte ist nicht mehr geöffnet.
agent-error-expired = Die Aktionen eines vergangenen Durchgangs sind abgelaufen.
agent-error-incomplete = Die Aktion dieser Karte ist unvollständig.
agent-error-no-field = Diese Einstellung gibt es nicht mehr.
agent-error-no-variant = Diese Variante gibt es nicht mehr.
agent-error-arguments = Die Argumente des Werkzeugs sind ungültig ({ $detail }): Prüfe sie anhand seines Schemas.
agent-error-unknown-tool = Unbekanntes Werkzeug: { $name }.
agent-error-pack-id = Das ist nicht die ID eines Pakets: Rufe packs auf, um die Pakete und ihre IDs aufzulisten.
agent-error-asset-unknown = Dieses Bild ist weder in der Präsentation noch unter den eingefügten Bildern: Verwende die Kennung eines eingefügten Bildes.
agent-error-denied = Der Benutzer hat abgelehnt.
agent-error-no-turn = Kein Durchgang läuft: { $tool } wird nur während eines Durchgangs angezeigt.
agent-error-turn-over = Der Durchgang dieses Aufrufs ist vorbei: Es wurde nichts angezeigt.
agent-error-anchor-scope = Eine Frage in einem Widget-Gespräch kann nicht an einer Folie verankert werden: Stellen Sie sie ohne Anker.
agent-error-no-file = Die Präsentation hat noch keine Datei: Gib einen absoluten Pfad an.
agent-error-path-relative = Der Pfad muss absolut sein.
agent-error-path-parent = Der Pfad darf kein „..“ enthalten.
agent-error-setting-locked = Die Einstellung { $key } kann vom Agenten nicht geändert werden.
agent-error-setting = Die Einstellung { $key } konnte nicht geändert werden: Prüfe ihren Wert.
agent-error-history-index = Zum Blättern im Verlauf wird der Index einer Version benötigt.
agent-error-no-pack = Dieses Deck heftet kein Paket an: Gib pack an (siehe packs).
agent-error-history-not-opened = Der Tab hat den Verlauf nicht rechtzeitig geöffnet: Frag erneut.
agent-error-present = Hier gibt es nichts zu präsentieren: keine Folie, oder der Tab ist im Paket-, Verlaufs- oder Nur-Lesen-Modus.
agent-error-locale = Unbekannter Sprachcode. Gib den Code einer mitgelieferten Sprache oder system an.
agent-error-empty-interface = Nichts zu ändern: Gib scheme, locale, grid oder guides an.
agent-error-empty-dictation = Nichts zu ändern: Gib mindestens eine Diktier-Einstellung an.
agent-error-not-shown = Dieser Tab ist nicht der in seinem Fenster angezeigte: Frag erneut, sobald der Nutzer ihn anzeigt.
agent-error-themes-deck-mode = Die Themenauswahl öffnet sich nur beim Bearbeiten der Folien.
agent-error-packs-deck-mode = Der Bereich der Präsentationspakete öffnet sich nur beim Bearbeiten der Folien.
agent-error-rewrite-text = Die Umformulierung kam ohne Text zurück.
agent-error-paragraphs = Die Absätze der Umformulierung sind ungültig.
agent-error-draft-too-long = Der Entwurf ist zu lang.
agent-error-draft-origin = Entwurf abgelehnt: unbekannte Ansicht.
agent-error-draft-no-agent = Der Agent dieses Tabs ist noch nicht gestartet: Entwurf abgelehnt.
agent-error-too-many-images = Der Entwurf enthält bereits alle Bilder.
agent-error-image-not-base64 = Das eingefügte Bild ist nicht vollständig angekommen.
agent-error-image-gone = Ein eingefügtes Bild ist nicht mehr verfügbar: Füge es erneut ein.
agent-error-unknown-agent = Dieser Agent ist nicht mehr in der Konfiguration.
agent-error-agent-locked = Die Wahl des Agenten ist durch die Umgebung vorgegeben.
agent-error-agent-not-saved = Die Wahl des Agenten konnte nicht gespeichert werden.
voice-error-no-dictation = Diktat ist hier nicht verfügbar
voice-error-busy = Das Diktat läuft: versuchen Sie es erneut, sobald es ruht
voice-error-unknown-model = Unbekanntes Diktiermodell
voice-error-unavailable = Dieses Modell unterstützt die gewählte Sprache nicht
voice-error-installed = Dieses Modell ist bereits installiert
voice-error-not-downloadable = Dieses Modell wird nicht heruntergeladen
voice-error-nothing-to-cancel = Es läuft kein Download
voice-error-bad-silence = Die Stille muss 0,5 bis 10 Sekunden dauern
voice-error-bad-language = Unbekannte Diktiersprache
voice-error-bad-url = Die API-Adresse muss mit https:// beginnen (http:// nur für diesen Computer), ohne Benutzernamen, Parameter oder Sonderzeichen
voice-error-bad-key-env = Die Schlüsselvariable muss ein Name wie OPENAI_API_KEY sein: Großbuchstaben, Ziffern und _
voice-error-bad-api-model = Der Name des API-Modells darf nur Buchstaben, Ziffern und . _ : / - enthalten
voice-error-api-reserved = Nur der Benutzer legt API-Adresse und Schlüsselvariable fest: öffnen Sie die Box mit show_dictation und fragen Sie ihn
voice-error-changed = Die Einstellungen haben sich seit der Frage geändert: fragen Sie erneut
voice-error-nothing-to-download = Für diesen Agenten gibt es nichts herunterzuladen
agent-tune-prompt = Die importierte Präsentation feinabstimmen
