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
agent-error-not-shown = Dieser Tab ist nicht der in seinem Fenster angezeigte: Frag erneut, sobald der Nutzer ihn anzeigt.
agent-error-themes-deck-mode = Die Themenauswahl öffnet sich nur beim Bearbeiten der Folien.
agent-error-packs-deck-mode = Der Bereich der Präsentationspakete öffnet sich nur beim Bearbeiten der Folien.
agent-error-rewrite-text = Die Umformulierung kam ohne Text zurück.
agent-error-paragraphs = Die Absätze der Umformulierung sind ungültig.
agent-error-draft-too-long = Der Entwurf ist zu lang.
agent-error-draft-origin = Entwurf abgelehnt: unbekannte Ansicht.
agent-error-draft-no-agent = Der Agent dieses Tabs ist noch nicht gestartet: Entwurf abgelehnt.
agent-error-unknown-agent = Dieser Agent ist nicht mehr in der Konfiguration.
agent-error-agent-locked = Die Wahl des Agenten ist durch die Umgebung vorgegeben.
agent-error-agent-not-saved = Die Wahl des Agenten konnte nicht gespeichert werden.
agent-picker-title = Agent
agent-picker-busy = Ein Wechsel des Agenten beendet die laufende Runde.
agent-picker-locked = Von der Umgebung vorgegeben (OKU_SLIDES_AGENT__SELECTED).
agent-tune-prompt = Die importierte Präsentation feinabstimmen
