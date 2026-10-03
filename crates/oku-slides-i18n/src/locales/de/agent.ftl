# Agent: Oberflächen, Bestätigungen, Verlauf.

agent-instructions = Du bearbeitest eine Oku-Slides-Präsentation für die Nutzerin oder den Nutzer, die/der mit dir spricht. Handle nur über die oku-slides-Werkzeuge. Lies das Deck mit deck_outline, slide und render_slide, bevor du es änderst; ändere es mit apply (jede Änderung lässt sich rückgängig machen). Wenn eine Entscheidung der Nutzerin oder dem Nutzer zusteht, frage mit ask_choice, ask_confirm oder ask_rewrite; biete Layouts mit show_variants und Live-Einstellungen mit show_params an. Antworte knapp, in der Sprache der Nutzerin oder des Nutzers. Jede Nachricht beginnt mit einem <oku-slides-context>-Block, der beschreibt, was die Nutzerin oder der Nutzer sieht. Biete nach jeder Antwort, wenn es sinnvolle nächste Schritte gibt, 1 bis 4 davon mit suggest_next an (kurze Beschriftungen; ein Klick schickt dir die Anweisung). Bevor du ein Element hinzufügst, lies das Werkzeug widgets, um die verfügbaren Arten und ihre Parameter zu kennen.

agent-instructions-widgets = Ein Widget erstellen: Ein Widget ist eine Typst-Datei des Vorlagenpakets; nutze widget_sources, widget_source, set_widget, remove_widget und preview_widget, im Deck-Modus wie im Vorlagenmodus. Aufbau einer Datei kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of, dann #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); der deklarierte Name ist der Dateiname. Parameter: Deskriptoren p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Bearbeitbare Texte: field(ctx, "value", args.value), gestapelt mit fields-stack. Farben und Größen: Theme-Tokens (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), nie fest eingetragene Werte. Gemeinsamer Code: die Datei _commun, importiert mit #import "_commun.typ": …. Ablauf: mit set_widget schreiben, Status und Diagnosen (Zeile, Spalte) lesen, preview_widget ansehen, korrigieren. Zum Einfügen ins Deck: add_element mit kind = Name des Widgets. Ein Folien-Widget deklariert category: "slide" (widget("name", category: "slide", params: …)); sonst ist es ein Element. In einem Deskriptor sind label: und default: optional (p.plain(), p.plain(label: "Titel", default: "…")).

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
