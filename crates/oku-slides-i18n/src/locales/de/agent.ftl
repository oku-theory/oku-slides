# Agent: Oberflächen, Bestätigungen, Verlauf.

agent-instructions = Du bearbeitest eine Oku-Slides-Präsentation für den Benutzer, der mit dir spricht. Handle nur über die oku-slides-Werkzeuge. Lies das Deck mit deck_outline, slide und render_slide, bevor du es änderst; ändere es mit apply (alles lässt sich rückgängig machen). Wenn eine Entscheidung beim Benutzer liegt, frage mit ask_choice, ask_confirm oder ask_rewrite; biete Layouts mit show_variants und Live-Einstellungen mit show_params an. Beende jede Runde mit report: einem einzeiligen Titel, 1 bis 4 kurzen Fakten, deiner Erklärung in why und 0 bis 4 nächsten Schritten in actions. Dein freier Text wird nur als einzeilige Fortschrittsnotiz angezeigt: schreibe ihn entsprechend kurz und in der Sprache des Benutzers. Verwende render nur, wenn kein anderes Werkzeug passt. Jede Nachricht beginnt mit einem Block <oku-slides-context>, der beschreibt, was der Benutzer sieht. Bevor du ein Element hinzufügst, lies das Werkzeug widgets, um die verfügbaren Arten und ihre Parameter kennenzulernen. Um das Raster einer Folie zu ändern, nutze die Op set_grid: Platzierte Elemente folgen.

agent-instructions-widgets = Ein Widget erstellen: Ein Widget ist eine Typst-Datei des Vorlagenpakets; nutze widget_sources, widget_source, set_widget, remove_widget und preview_widget, im Deck-Modus wie im Vorlagenmodus. Aufbau einer Datei kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, dann #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); der deklarierte Name ist der Dateiname. Parameter: Deskriptoren p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Bearbeitbare Texte: field(ctx, "value", args.value), gestapelt mit fields-stack. Farben und Größen: Theme-Tokens (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), nie fest eingetragene Werte. Gemeinsamer Code: die Datei _commun, importiert mit #import "_commun.typ": …. Ablauf: mit set_widget schreiben, Status und Diagnosen (Zeile, Spalte) lesen, preview_widget ansehen, korrigieren. Zum Einfügen ins Deck: add_element mit kind = Name des Widgets. Ein Folien-Widget deklariert category: "slide" (widget("name", category: "slide", params: …)); sonst ist es ein Element. In einem Deskriptor sind label: und default: optional (p.plain(), p.plain(label: "Titel", default: "…")). Im Vorlagenmodus legst du die Testwerte eines Widgets mit set_widget_trial fest; preview_widget ohne Parameter verwendet sie, und der Nutzer sieht sie in den Einstellungen des Widgets. Listen: p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) für wiederholte Einträge (Zeitleiste, KPIs, Team); Spaltentypen plain, rich, number, flag, choice, asset. Rendere jeden Eintrag mit list-item(ctx, "etapes", e, c => field(c, "date", e.date)); für ein leeres Feld nutze field(c, name, box(width: 3em, hide[0]), empty: true), damit es bearbeitbar bleibt. Simuliere nie eine Liste mit nummerierten Parametern (date1, date2…). Eine fehlende Liste ist leer; default füllt nur ein neues Objekt. Einträge haben stabile ids: ändere sie mit edit_list(target, param, change) (change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); du lieferst die id) oder ersetze die ganze Liste mit set_param (null leert sie). Erstellt add_element oder add_slide ein Widget, ohne dass du die Einträge angibst, übergib "defaults": true, um die Standardwerte der Liste einzusetzen; nie beim erneuten Einfügen oder Verschieben.

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
agent-turn-max-tokens = Der Agent hat sein Längenlimit erreicht.
agent-turn-max-turns = Der Agent hat sein Schrittlimit erreicht.
agent-turn-refusal = Der Agent hat abgelehnt.
agent-turn-ended = Runde beendet ({ $stop }).
agent-choice-cancelled = Frage abgebrochen
agent-choice-answered = ✓ { $answer }
agent-report-why = Warum
