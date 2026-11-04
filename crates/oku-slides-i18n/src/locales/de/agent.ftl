# Agent: Oberflächen, Bestätigungen, Verlauf.

agent-surface-cancel = Abbrechen
agent-surface-confirm = Bestätigen
agent-surface-confirm-danger = Ja, tun
agent-surface-heard = Gehört: „{ $text }“
agent-surface-say-or-click = Sagen Sie Ihre Antwort oder klicken Sie auf eine Schaltfläche.
agent-surface-said-too-early = Zu früh gesagt: Warten Sie auf das Fenster und sagen Sie es dann erneut.
agent-surface-click-to-confirm = Klicken Sie zum Bestätigen auf die Schaltfläche.
agent-surface-verb-replace = Ersetzen
agent-surface-verb-merge = Zusammenführen
agent-surface-verb-discard = Verwerfen
agent-surface-verb-delete = Löschen
agent-surface-verb-use-api = API verwenden
agent-surface-verb-install = Installieren
agent-surface-verb-quit = Beenden
agent-surface-verb-close = Schließen
agent-surface-verb-import = Importieren
agent-surface-rewrite = Umformulierung
agent-surface-pick = { $n } wählen
agent-surface-keep = Original behalten
agent-surface-edit = Vor dem Anwenden bearbeiten
agent-surface-close = Schließen
agent-surface-themes = Themen
agent-surface-variants = Varianten
agent-surface-permission-always = für immer
agent-surface-permission-title = Der Agent bittet um eine Berechtigung
agent-surface-permission-guides = Dieser Zug enthält den erhaltenen Leitfaden von { $names }.
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
agent-confirm-copy-pack = Eine Kopie des Pakets „{ $name }“ speichern? Das Paket selbst bleibt unverändert.
agent-confirm-copy-edited-theme = Eine Kopie des Designs „{ $name }“ speichern? Das Design selbst bleibt unverändert.
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

agent-not-found = Agent nicht gefunden: { $command } existiert weder unter diesem Pfad noch im PATH der App. Geben Sie in [agent.presets] den vollständigen Pfad an.
agent-spawn-failed = Agent konnte nicht gestartet werden: { $command }. Einstellung [agent.presets].
agent-no-http = Dieser Agent kann sich nicht mit den Werkzeugen von Oku Slides verbinden.
agent-local-no-program = Der lokale Agent läuft nur innerhalb der App Oku Slides.
agent-local-needs-download = Der lokale Agent muss zuerst sein Modell herunterladen ({ $gigabytes } GB).
agent-local-intel-mac = Der lokale Agent benötigt einen Mac mit Apple-Chip.
agent-local-unsupported-os = Der lokale Agent ist auf diesem System noch nicht verfügbar.
agent-local-apple-off = Der lokale Agent benötigt Apple Intelligence: Aktiviere sie in den Systemeinstellungen.
agent-local-apple-not-ready = Das Modell von Apple wird noch vorbereitet: Versuche den lokalen Agenten in ein paar Minuten erneut.
agent-local-apple-unsupported = Das Modell von Apple benötigt einen Mac, der Apple Intelligence unterstützt, mit macOS 26 oder neuer.
agent-model-download = Modell herunterladen
agent-model-downloading = Modell wird heruntergeladen… { $percent } %
agent-model-cancel = Abbrechen
agent-model-download-failed = Das Modell konnte nicht heruntergeladen werden.
agent-model-download-integrity = Das heruntergeladene Modell war beschädigt und wurde gelöscht.
agent-model-download-incomplete = Download unvollständig: { $got } von { $expected } Bytes.
agent-model-download-cancelled = Download abgebrochen.
agent-fallback-notice = Lokaler Agent — { $reason }
agent-fallback-download = { $reason } Der lokale Agent kann übernehmen, sobald sein Modell ({ $model }, { $gigabytes } GB) heruntergeladen ist. Jetzt herunterladen?
agent-fallback-unusable = Kein Agent kann laufen. Hauptagent: { $main } Lokaler Agent: { $local }
local-agent-no-route = Der lokale Agent hat nicht verstanden, was zu tun ist. Versuchen Sie, es anders zu sagen.
local-agent-tool-limit = Der lokale Agent hat nach { $n } Schritten aufgehört, ohne fertig zu werden.
local-agent-invalid-call = Der lokale Agent konnte die Werkzeuge nicht richtig verwenden.
local-agent-engine-failed = Das Modell des lokalen Agenten ist fehlgeschlagen.
local-agent-retry = Versuchen Sie es erneut oder wählen Sie im Agent-Bereich einen anderen Agenten.
agent-stopped = Der Agent wurde beendet.
agent-failed = Der Agent hat die Anfrage abgelehnt: { $message }
agent-error-work-dir = Der private Arbeitsordner des Agenten für diesen Tab konnte nicht vorbereitet werden.
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
agent-trace = { $name }: { $count ->
    [one] { $count } Änderung
   *[other] { $count } Änderungen
}
agent-trace-data = { $name }: Daten geändert
agent-trace-removed = { $name }: existiert nicht mehr
agent-trace-more = { $count ->
    [one] und { $count } weiteres
   *[other] und { $count } weitere
}
agent-trace-title = Geänderte Elemente

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
agent-error-tab-no-turn = Öffne, zeige oder verschiebe einen Tab nur, während du dem Nutzer antwortest.
agent-error-tab-limit = Du hast in dieser Runde schon einen Tab geöffnet, gezeigt oder verschoben; mach dort weiter oder frag den Nutzer vor einem weiteren.
agent-error-continue-in-other-tab = Diese Anfrage geht in dem Tab weiter, den du geöffnet oder gezeigt hast: Bitte den Nutzer, dort fortzufahren; dieser Tab ändert vorerst kein Widget.
agent-error-pack-open-in-tab = Dieses Paket wird in einem anderen Tab bearbeitet: Bitte den Nutzer, zuerst dort fertig zu werden.
agent-pack-opened-in-tab = { $shown ->
   [yes] Das Paket hat sich im Tab { $tab } geöffnet, der jetzt angezeigt wird.
  *[no] Das Paket hat sich im Tab { $tab } im Hintergrund geöffnet: Das Fenster des Nutzers zeigt einen anderen Tab.
} { $draft ->
   [set] Die Anfrage wartet im Entwurf dieses Tabs: Sag dem Nutzer, sie dort zu prüfen und dann zu senden.
   [kept] Der Entwurf dieses Tabs enthielt schon etwas und bleibt unverändert. Sag dem Nutzer, dort weiterzumachen: Der Agent dieses Tabs kennt diese Anfrage nicht.
   [too_long] Die Anfrage war zu lang für den Entwurf dieses Tabs. Sag dem Nutzer, dort weiterzumachen: Der Agent dieses Tabs kennt diese Anfrage nicht.
   [unavailable] Die Anfrage konnte den Entwurf dieses Tabs nicht erreichen. Sag dem Nutzer, dort weiterzumachen: Der Agent dieses Tabs kennt diese Anfrage nicht.
  *[none] Sag dem Nutzer, dort weiterzumachen: Der Agent dieses Tabs kennt diese Anfrage nicht.
} Dieser Tab bleibt, wie er ist.
agent-theme-opened-in-tab = { $shown ->
   [yes] Das Design hat sich im Tab { $tab } geöffnet, der jetzt angezeigt wird.
  *[no] Das Design hat sich im Tab { $tab } im Hintergrund geöffnet: Das Fenster des Nutzers zeigt einen anderen Tab.
} { $draft ->
   [set] Die Anfrage wartet im Entwurf dieses Tabs: Sag dem Nutzer, sie dort zu prüfen und dann zu senden.
   [kept] Der Entwurf dieses Tabs enthielt schon etwas und bleibt unverändert. Sag dem Nutzer, dort weiterzumachen: Der Agent dieses Tabs kennt diese Anfrage nicht.
   [too_long] Die Anfrage war zu lang für den Entwurf dieses Tabs. Sag dem Nutzer, dort weiterzumachen: Der Agent dieses Tabs kennt diese Anfrage nicht.
   [unavailable] Die Anfrage konnte den Entwurf dieses Tabs nicht erreichen. Sag dem Nutzer, dort weiterzumachen: Der Agent dieses Tabs kennt diese Anfrage nicht.
  *[none] Sag dem Nutzer, dort weiterzumachen: Der Agent dieses Tabs kennt diese Anfrage nicht.
} Dieser Tab bleibt, wie er ist.
agent-error-turn-over = Der Durchgang dieses Aufrufs ist vorbei: Es wurde nichts angezeigt.
agent-error-no-box-turn = Kein Durchgang, dem dieses Feld zugeordnet werden kann: Es wurde nichts angezeigt.
agent-error-no-file = Die Präsentation hat noch keine Datei: Gib einen absoluten Pfad an.
agent-error-path-relative = Der Pfad muss absolut sein.
agent-error-path-parent = Der Pfad darf kein „..“ enthalten.
agent-error-setting-locked = Die Einstellung { $key } kann vom Agenten nicht geändert werden.
agent-error-setting = Die Einstellung { $key } konnte nicht geändert werden: Prüfe ihren Wert.
agent-error-history-index = Zum Blättern im Verlauf wird der Index einer Version benötigt.
agent-error-no-pack = Dieses Deck heftet kein Paket an: Gib pack an (siehe packs).
agent-error-history-not-opened = Der Tab hat den Verlauf nicht rechtzeitig geöffnet: Frag erneut.
agent-error-locale = Unbekannter Sprachcode. Gib den Code einer mitgelieferten Sprache oder system an.
agent-error-empty-interface = Nichts zu ändern: Gib scheme, locale, guides, folded, pile, notes oder code an.
agent-error-layout-range = { $field } muss zwischen { $min } und { $max } px liegen.
agent-error-layout-min = { $field } muss mindestens { $min } px betragen.
agent-error-empty-dictation = Nichts zu ändern: Gib mindestens eine Diktier-Einstellung an.
agent-error-empty-speech = Nichts zu ändern: Geben Sie mindestens eine Vorlese-Einstellung an.
agent-error-not-shown = Dieser Tab ist nicht der in seinem Fenster angezeigte: Frag erneut, sobald der Nutzer ihn anzeigt.
agent-error-not-presenting = Das Deck wird hier nicht präsentiert: Starte die Präsentation zuerst mit der Aktion start.
agent-error-present-slide = Zum Zeigen einer Folie brauchst du die ID einer Folie dieses Decks (siehe deck_outline).
agent-error-undo-open-turn = undo_turn macht den laufenden Zug nicht rückgängig: nutze undo für die Änderungen dieses Zugs.
agent-error-app-unavailable = Das kann nur die App Oku Slides; hier ist es nicht verfügbar.
agent-error-app-closing = Oku Slides wird bereits beendet.
agent-error-app-no-offer = Hier gibt es kein Update zum Aufschieben oder Installieren: Suche zuerst nach Updates.
agent-error-app-failed = Das Update von Oku Slides ist fehlgeschlagen: Prüfe die Verbindung oder nutze das Update-Banner.
agent-error-after-turn-taken = Diese Antwort endet bereits mit dem Beenden, der Installation eines Updates oder dem Schließen ihres Tabs.
agent-error-no-pack-update = Das Pack-Panel bietet für dieses Pack kein solches Update an: siehe show_packs.
agent-confirm-update-install = Oku Slides { $version } installieren? Die App startet neu, sobald diese Antwort endet.
agent-after-turn-quit-unsaved = Oku Slides wurde nicht beendet: Ein Tab hat seit deiner Bitte ungespeicherte Änderungen.
agent-error-workspace-unavailable = Nur die Oku-Slides-App kann Tabs und Fenster ändern; hier ist das nicht verfügbar.
agent-error-workspace-unknown-tab = Dieses Fenster hat keinen solchen Tab und kein solches Fenster.
agent-error-workspace-unknown-window = Ein solches Fenster gibt es nicht.
agent-error-workspace-index = Diese Position liegt hinter dem Ende der Tableiste.
agent-error-workspace-closing = Oku Slides wird beendet: Tabs und Fenster ändern sich nicht mehr.
agent-confirm-close-tab = Den Tab „{ $tab }“ schließen?
agent-confirm-discard-tab = Den Tab „{ $tab }“ schließen? Seine nicht gespeicherten Änderungen gehen verloren.
agent-confirm-close-window = Dieses Fenster und seine Tabs schließen?
agent-confirm-discard-window = Dieses Fenster schließen? Nicht gespeicherte Änderungen in { $tabs } gehen verloren.
agent-error-window-last = Dies ist das einzige Fenster: Schließe stattdessen seine Tabs oder beende Oku Slides.
agent-error-tab-index = Sag, wohin der Tab soll: Zum Umordnen braucht es eine Position.
agent-error-tab-unused = Nur Umordnen und Verschieben nehmen eine Position, und nur Verschieben nimmt ein Fenster.
agent-after-turn-close-unsaved = Der Tab wurde nicht geschlossen: Er hat sich seit deiner Bitte geändert.
agent-after-turn-close-not-shown = Der Tab wurde nicht geschlossen: Am Ende der Antwort war ein anderer Tab in seinem Fenster zu sehen.
agent-confirm-quit = Oku Slides beenden? Die App wird beendet, sobald diese Antwort endet.
agent-error-quit-unsaved = Tabs mit ungespeicherter Arbeit: { $count }. Oku Slides wurde nicht beendet: Der Benutzer speichert oder schließt sie zuerst.
agent-error-tune-unavailable = Die Feinabstimmung konnte nicht starten: Der Agent ist nicht verfügbar.
agent-error-tune-not-started = Die Feinabstimmung hat nicht begonnen: Der Agent dieses Tabs ist gerade nicht nutzbar; der Tab sagt, warum.
agent-confirm-import = { $path } in einem neuen Tab importieren?
agent-import-opened-in-tab = { $shown ->
   [yes] Die Präsentation wurde im Tab { $tab } geöffnet und wird jetzt angezeigt.
  *[no] Die Präsentation wurde im Tab { $tab } im Hintergrund geöffnet: Das Fenster des Nutzers zeigt einen anderen Tab.
} { $tune ->
   [started] Ihre Feinabstimmung beginnt dort, im eigenen Gespräch dieses Tabs.
   [unavailable] Ihre Feinabstimmung konnte nicht starten: Sag dem Nutzer, er soll sie in diesem Tab anfordern.
  *[no] Sag dem Nutzer, er soll dort weitermachen.
} Dieser Tab bleibt, wie er ist.
agent-error-import-hifi-target = Ein Hi-Fi-Import nimmt weder Pack noch Theme: Lass sie weg oder importiere im Retrofit-Modus.
agent-error-import-extension = Nur eine PowerPoint-Datei .pptx kann importiert werden.
agent-error-a2ui-unknown = { $name } ist keine Komponente, die render akzeptiert. Komponenten: { $known }.
agent-error-a2ui-elsewhere = render zeigt { $name } nicht an: Verwende { $tool }.
agent-error-a2ui-host-only = { $name } ist der App vorbehalten: render kann es nicht anzeigen.
agent-error-themes-deck-mode = Die Themenauswahl öffnet sich nur beim Bearbeiten der Folien.
agent-error-packs-deck-mode = Der Bereich der Präsentationspakete öffnet sich nur beim Bearbeiten der Folien.
agent-error-usage-no-deck = Die Nutzung dieses Tabs gehört zu keiner Präsentation: Nur ein Paket- oder Design-Tab, der aus einer Präsentation geöffnet wurde (vom Nutzer oder vom Agenten), oder das eigene Paket einer Präsentation zählt für diese Präsentation.
agent-error-rewrite-text = Die Umformulierung kam ohne Text zurück.
agent-error-paragraphs = Die Absätze der Umformulierung sind ungültig.
agent-error-draft-too-long = Der Entwurf ist zu lang.
agent-error-report-elsewhere = Dieses Fenster zeigt diesen Tab nicht.
agent-error-ui-too-large = Der Seitenkontext ist zu groß.
agent-error-seq-exhausted = Die Nummer der Seitenmeldung liegt außerhalb des Bereichs.
agent-error-draft-origin = Entwurf abgelehnt: unbekannte Ansicht.
agent-error-draft-no-agent = Der Agent dieses Tabs ist noch nicht gestartet: Entwurf abgelehnt.
agent-error-too-many-images = Der Entwurf enthält bereits alle Bilder.
agent-error-image-not-base64 = Das eingefügte Bild ist nicht vollständig angekommen.
agent-error-image-gone = Ein eingefügtes Bild ist nicht mehr verfügbar: Füge es erneut ein.
agent-error-unknown-agent = Dieser Agent ist nicht mehr in der Konfiguration.
agent-error-agent-locked = Die Wahl des Agenten ist durch die Umgebung vorgegeben.
agent-error-agent-not-saved = Die Wahl des Agenten konnte nicht gespeichert werden.
agent-error-unknown-model = Dieser Agent bietet dieses Modell nicht an.
agent-error-model-not-saved = Die Wahl des Modells konnte nicht gespeichert werden.
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
voice-error-bad-api-model = Der Name des API-Modells darf nur Buchstaben, Ziffern und . _ : / - enthalten; ein Azure-Bereitstellungsname nur Buchstaben, Ziffern und . _ -
voice-error-bad-api-version = Die Azure-API-Version darf nur Buchstaben, Ziffern, . und - enthalten, höchstens 32 Zeichen, etwa 2025-03-01-preview
voice-error-bad-api-headers = Header werden Name: Wert; Name: $VARIABLE geschrieben, höchstens 8. Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection und Duplikate werden abgelehnt
voice-error-api-reserved = Nur der Benutzer legt Adresse, Schlüsselvariable, Azure-Version und Header der API fest: Öffne das Fenster mit show_dictation und frag ihn
voice-error-hands-free-reserved = Nur der Benutzer schaltet das freihändige Zuhören ein: Öffnen Sie das Feld mit show_dictation und bitten Sie ihn darum
voice-error-changed = Die Einstellungen haben sich seit der Frage geändert: fragen Sie erneut
voice-error-nothing-to-download = Für diesen Agenten gibt es nichts herunterzuladen
voice-error-bad-rate = Das Sprechtempo muss zwischen 0,5 und 2 liegen
voice-error-unknown-voice = Unbekannte Stimme für die Sprache der Oberfläche
voice-error-speech-unavailable = Vorlesen ist auf diesem Computer nicht verfügbar
agent-tune-prompt = Die importierte Präsentation feinabstimmen
agent-settings-unknown-selected = Einstellung agent.selected = { $selected } nennt keinen Agenten aus [agent.presets] (bekannt: { $known })
agent-settings-preset-launch = Agent [agent.presets.{ $id }] braucht genau eines von command oder builtin
