# Agent: oppervlakken, bevestigingen, thread.

agent-instructions = Je bewerkt een Oku Slides-presentatie voor de gebruiker, die tegen je spreekt. Handel alleen via de oku-slides-tools. Lees de deck met deck_outline, slide en render_slide voordat je iets wijzigt; wijzig met apply (elke wijziging kan ongedaan worden gemaakt). Wanneer een beslissing bij de gebruiker ligt, vraag met ask_choice, ask_confirm of ask_rewrite; bied lay-outs aan met show_variants en live-instellingen met show_params. Antwoord kort, in de taal van de gebruiker. Elk bericht begint met een <oku-slides-context>-blok dat beschrijft wat de gebruiker ziet. Bied na elk antwoord, als er nuttige vervolgstappen zijn, er 1 tot 4 aan met suggest_next (korte labels; een klik stuurt je de opdracht). Lees vóór het toevoegen van een element de tool widgets om de beschikbare soorten en hun parameters te kennen.

agent-surface-cancel = Annuleren
agent-surface-confirm = Bevestigen
agent-surface-confirm-danger = Ja, doe het
agent-surface-rewrite = Herformulering
agent-surface-pick = { $n } kiezen
agent-surface-keep = Origineel behouden
agent-surface-edit = Bewerken vóór toepassen
agent-surface-close = Sluiten
agent-surface-variants = Varianten
agent-surface-permission-always = voor altijd
agent-surface-permission-title = De agent vraagt om toestemming
agent-undo-turn = Agent: { $text }

agent-tools-unavailable = De tools van de agent konden niet worden gestart.

agent-undo-apply = Wijziging van de agent
agent-undo-variant = Variant: { $label }
agent-undo-param = Instelling: { $label }
agent-undo-rewrite = Herformulering

agent-confirm-save = Presentatie opslaan als { $path }?
agent-confirm-save-over = { $path } vervangen? Het bestaande bestand wordt overschreven.
agent-confirm-open = { $path } openen?
agent-open-elsewhere = Dit bestand is al geopend in een ander tabblad.
agent-confirm-new = Nieuwe presentatie beginnen?
agent-confirm-new-dirty = Nieuwe presentatie beginnen? Niet-opgeslagen wijzigingen gaan verloren.
agent-confirm-save-templates = Sjabloonset “{ $name }” opslaan?
agent-confirm-delete-set = Sjabloonset “{ $name }” verwijderen? Dit kan niet ongedaan worden gemaakt.
agent-confirm-setting = { $key } wijzigen: { $old } → { $new }?

agent-not-found = Agent niet gevonden: { $command }. Instelling [agent] command.
agent-no-http = Deze agent kan geen verbinding maken met de tools van Oku Slides.
agent-stopped = De agent is gestopt.
agent-failed = De agent heeft het verzoek geweigerd: { $message }
