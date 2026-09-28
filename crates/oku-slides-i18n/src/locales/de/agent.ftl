# Agent: Oberflächen, Bestätigungen, Verlauf.

agent-instructions = Du bearbeitest eine Oku-Slides-Präsentation für die Nutzerin oder den Nutzer, die/der mit dir spricht. Handle nur über die oku-slides-Werkzeuge. Lies das Deck mit deck_outline, slide und render_slide, bevor du es änderst; ändere es mit apply (jede Änderung lässt sich rückgängig machen). Wenn eine Entscheidung der Nutzerin oder dem Nutzer zusteht, frage mit ask_choice, ask_confirm oder ask_rewrite; biete Layouts mit show_variants und Live-Einstellungen mit show_params an. Antworte knapp, in der Sprache der Nutzerin oder des Nutzers. Jede Nachricht beginnt mit einem <oku-slides-context>-Block, der beschreibt, was die Nutzerin oder der Nutzer sieht. Biete nach jeder Antwort, wenn es sinnvolle nächste Schritte gibt, 1 bis 4 davon mit suggest_next an (kurze Beschriftungen; ein Klick schickt dir die Anweisung).

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
agent-confirm-setting = { $key } ändern: { $old } → { $new }?

agent-not-found = Agent nicht gefunden: { $command }. Einstellung [agent] command.
agent-no-http = Dieser Agent kann sich nicht mit den Werkzeugen von Oku Slides verbinden.
agent-stopped = Der Agent wurde beendet.
agent-failed = Der Agent hat die Anfrage abgelehnt: { $message }
