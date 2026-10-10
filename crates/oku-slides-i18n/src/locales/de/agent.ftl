# Agent: Oberflächen, Bestätigungen, Verlauf.

agent-surface-cancel = Abbrechen
agent-surface-confirm = Bestätigen
agent-surface-confirm-danger = Ja, tun
agent-surface-rewrite = Umformulierung
agent-surface-pick = { $n } wählen
agent-surface-keep = Original behalten
agent-surface-edit = Vor dem Anwenden bearbeiten
agent-surface-close = Schließen
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
agent-confirm-save-templates = Vorlagensatz „{ $name }“ speichern?
agent-confirm-delete-set = Vorlagensatz „{ $name }“ löschen? Das kann nicht rückgängig gemacht werden.
agent-confirm-edit-theme = Eine neue Version des Designs „{ $name }“ speichern?
agent-confirm-copy-theme = Eine bearbeitete Kopie des integrierten Designs „{ $name }“ speichern?
agent-confirm-setting = { $key } ändern: { $old } → { $new }?

agent-not-found = Agent nicht gefunden: { $command }. Einstellung [agent] command.
agent-no-http = Dieser Agent kann sich nicht mit den Werkzeugen von Oku Slides verbinden.
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
agent-error-denied = Der Benutzer hat abgelehnt.
agent-error-no-turn = Kein Durchgang läuft: { $tool } wird nur während eines Durchgangs angezeigt.
agent-error-turn-over = Der Durchgang dieses Aufrufs ist vorbei: Es wurde nichts angezeigt.
agent-error-anchor-scope = Eine Frage in einem Widget-Gespräch kann nicht an einer Folie verankert werden: Stellen Sie sie ohne Anker.
agent-error-no-file = Die Präsentation hat noch keine Datei: Gib einen absoluten Pfad an.
agent-error-path-relative = Der Pfad muss absolut sein.
agent-error-path-parent = Der Pfad darf kein „..“ enthalten.
agent-error-setting-locked = Die Einstellung { $key } kann vom Agenten nicht geändert werden.
agent-error-setting = Die Einstellung { $key } konnte nicht geändert werden: Prüfe ihren Wert.
agent-error-rewrite-text = Die Umformulierung kam ohne Text zurück.
agent-error-paragraphs = Die Absätze der Umformulierung sind ungültig.
agent-error-draft-too-long = Der Entwurf ist zu lang.
agent-error-draft-origin = Entwurf abgelehnt: unbekannte Ansicht.
agent-error-draft-no-agent = Der Agent dieses Tabs ist noch nicht gestartet: Entwurf abgelehnt.
