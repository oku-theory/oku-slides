# Agent: oppervlakken, bevestigingen, thread.

agent-surface-cancel = Annuleren
agent-surface-confirm = Bevestigen
agent-surface-confirm-danger = Ja, doe het
agent-surface-rewrite = Herformulering
agent-surface-pick = { $n } kiezen
agent-surface-keep = Origineel behouden
agent-surface-edit = Bewerken vóór toepassen
agent-surface-close = Sluiten
agent-surface-themes = Thema's
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
agent-confirm-save-pack = Pakket “{ $name }” opslaan?
agent-confirm-delete-pack = Pakket “{ $name }” verwijderen? Dit kan niet ongedaan worden gemaakt.
agent-confirm-edit-theme = Een nieuwe versie van het thema ‘{ $name }’ opslaan?
agent-confirm-copy-theme = Een variant van het ingebouwde thema ‘{ $name }’ maken? Het ingebouwde thema blijft ongewijzigd.
agent-confirm-setting = { $key } wijzigen: { $old } → { $new }?
agent-confirm-move = De presentatie naar { $path } verplaatsen? Het oude bestand wordt verwijderd.
agent-confirm-merge = { $path } samenvoegen met deze presentatie? Dit kan niet ongedaan worden gemaakt.
agent-confirm-save-copy = Een kopie van de presentatie opslaan als { $path }?
agent-confirm-dictation-api = Wat je dicteert naar { $host } ({ $url }) sturen, model { $model }, met het geheim van { $var }?
agent-confirm-dictation-download = { $name } gebruiken en nu downloaden ({ $size })?
agent-confirm-dictation-language = Dicteren van Apple gebruiken en de taal nu downloaden?
agent-confirm-agent = De app-agent wijzigen naar { $name } ({ $command }) zodra er geen antwoord loopt?
agent-confirm-agent-builtin = De app-agent wijzigen naar { $name } zodra er geen antwoord loopt?
agent-confirm-agent-model = Het model van de lokale agent downloaden ({ $size })?

agent-not-found = Agent niet gevonden: { $command }. Instelling [agent.presets].
agent-spawn-failed = Agent kon niet starten: { $command }. Instelling [agent.presets].
agent-no-http = Deze agent kan geen verbinding maken met de tools van Oku Slides.
agent-local-no-program = De lokale agent werkt alleen binnen de Oku Slides-app.
agent-local-needs-download = De lokale agent moet eerst zijn model downloaden ({ $gigabytes } GB).
agent-local-intel-mac = De lokale agent heeft een Mac met Apple-chip nodig.
agent-local-unsupported-os = De lokale agent is nog niet beschikbaar op dit systeem.
agent-model-download = Model downloaden
agent-model-downloading = Model wordt gedownload… { $percent }%
agent-model-cancel = Annuleren
agent-model-download-failed = Het model kon niet worden gedownload.
agent-model-download-integrity = Het gedownloade model was beschadigd en is verwijderd.
agent-model-download-incomplete = Onvolledige download: { $got } van { $expected } bytes.
agent-model-download-cancelled = Download geannuleerd.
local-agent-no-route = De lokale agent begreep niet wat er moest gebeuren. Probeer het anders te zeggen.
local-agent-tool-limit = De lokale agent stopte na { $n } stappen zonder klaar te zijn.
local-agent-invalid-call = De lokale agent kon de hulpmiddelen niet goed gebruiken.
local-agent-engine-failed = Het model van de lokale agent is mislukt.
local-agent-retry = Probeer het opnieuw, of kies een andere agent in het agentpaneel.
agent-stopped = De agent is gestopt.
agent-failed = De agent heeft het verzoek geweigerd: { $message }
agent-tool-failed = Het hulpmiddel is onverwacht mislukt.

agent-turn-tools = { $tools ->
   [one] { $tools } tool
  *[other] { $tools } tools
}
agent-turn-notes = { $notes ->
   [one] { $notes } notitie
  *[other] { $notes } notities
}
agent-turn-secs = { $secs } s
agent-turn-stopped = Gestopt.
agent-turn-interrupted = Onderbroken.
agent-memory-lost = De agent herinnert zich dit gesprek niet meer: hij begint opnieuw.
agent-turn-max-tokens = De agent heeft zijn lengtelimiet bereikt.
agent-turn-max-turns = De agent heeft zijn stappenlimiet bereikt.
agent-turn-refusal = De agent heeft geweigerd.
agent-turn-ended = Beurt beëindigd ({ $stop }).
agent-choice-cancelled = Vraag geannuleerd
agent-choice-answered = ✓ { $answer }
agent-report-why = Waarom

agent-error-file = Het bestand kon niet worden gelezen of geschreven.
agent-error-unexpected-action = Onverwachte actie: { $action }.
agent-error-unknown-surface = Deze kaart is niet meer open.
agent-error-expired = De acties van een eerdere beurt zijn verlopen.
agent-error-incomplete = De actie van deze kaart is onvolledig.
agent-error-no-field = Deze instelling bestaat niet meer.
agent-error-no-variant = Deze variant bestaat niet meer.
agent-error-arguments = De argumenten van de tool zijn ongeldig ({ $detail }): controleer ze aan de hand van het schema.
agent-error-unknown-tool = Onbekende tool: { $name }.
agent-error-pack-id = Dit is geen id van een pakket: roep packs aan om de pakketten en hun id's op te sommen.
agent-error-asset-unknown = Deze afbeelding zit niet in de presentatie en staat ook niet bij de geplakte afbeeldingen: gebruik de id van een geplakte afbeelding.
agent-error-denied = De gebruiker heeft geweigerd.
agent-error-no-turn = Geen beurt bezig: { $tool } wordt alleen tijdens een beurt getoond.
agent-error-turn-over = De beurt van deze aanroep is voorbij: er is niets getoond.
agent-error-no-file = De presentatie heeft nog geen bestand: geef een absoluut pad op.
agent-error-path-relative = Het pad moet absoluut zijn.
agent-error-path-parent = Het pad mag geen “..” bevatten.
agent-error-setting-locked = De instelling { $key } kan niet door de agent worden gewijzigd.
agent-error-setting = De instelling { $key } kon niet worden gewijzigd: controleer de waarde.
agent-error-history-index = Om door de geschiedenis te bladeren is de index van een versie nodig.
agent-error-no-pack = Deze presentatie pint geen pakket vast: geef pack (zie packs).
agent-error-history-not-opened = Het tabblad heeft de geschiedenis niet op tijd geopend: vraag het opnieuw.
agent-error-present = Hier valt niets te presenteren: geen dia, of het tabblad staat in pakket-, geschiedenis- of alleen-lezenmodus.
agent-error-locale = Onbekende taalcode. Geef de code van een meegeleverde taal of system.
agent-error-empty-interface = Niets te wijzigen: geef scheme, locale, grid of guides.
agent-error-empty-dictation = Niets te wijzigen: geef minstens één dicteerinstelling.
agent-error-not-shown = Dit tabblad is niet het getoonde in zijn venster: vraag het opnieuw zodra de gebruiker het toont.
agent-error-themes-deck-mode = De themakiezer opent alleen tijdens het bewerken van de dia's.
agent-error-packs-deck-mode = Het paneel met pakketten van de presentatie opent alleen tijdens het bewerken van de dia's.
agent-error-rewrite-text = De herformulering kwam terug zonder tekst.
agent-error-paragraphs = De alinea's van de herformulering zijn ongeldig.
agent-error-draft-too-long = Het concept is te lang.
agent-error-draft-origin = Concept geweigerd: onbekende weergave.
agent-error-draft-no-agent = De agent van dit tabblad is nog niet gestart: concept geweigerd.
agent-error-too-many-images = Het concept heeft al al zijn afbeeldingen.
agent-error-image-not-base64 = De geplakte afbeelding is niet compleet aangekomen.
agent-error-image-gone = Een geplakte afbeelding is niet meer beschikbaar: plak hem opnieuw.
agent-error-unknown-agent = Deze agent staat niet meer in de configuratie.
agent-error-agent-locked = De keuze van de agent wordt door de omgeving opgelegd.
agent-error-agent-not-saved = De keuze van de agent kon niet worden opgeslagen.
voice-error-no-dictation = Dicteren is hier niet beschikbaar
voice-error-busy = Het dicteren loopt: probeer opnieuw zodra het rust
voice-error-unknown-model = Onbekend dicteermodel
voice-error-unavailable = Dit model ondersteunt de gekozen taal niet
voice-error-installed = Dit model is al geïnstalleerd
voice-error-not-downloadable = Dit model wordt niet gedownload
voice-error-nothing-to-cancel = Er loopt geen download
voice-error-bad-silence = De stilte moet 0,5 tot 10 seconden duren
voice-error-bad-language = Onbekende dicteertaal
voice-error-bad-url = Het API-adres moet beginnen met https:// (http:// alleen voor deze computer), zonder gebruikersnaam, parameters of speciale tekens
voice-error-bad-key-env = De sleutelvariabele moet een naam zijn zoals OPENAI_API_KEY: hoofdletters, cijfers en _
voice-error-bad-api-model = De naam van het API-model mag alleen letters, cijfers en . _ : / - bevatten
voice-error-api-reserved = Alleen de gebruiker stelt het API-adres en de sleutelvariabele in: open het venster met show_dictation en vraag het hem
voice-error-changed = De instellingen zijn sinds de vraag veranderd: vraag opnieuw
voice-error-nothing-to-download = Niets te downloaden voor deze agent
agent-tune-prompt = De geïmporteerde presentatie verfijnen
agent-settings-unknown-selected = Instelling agent.selected = { $selected } noemt geen agent uit [agent.presets] (bekend: { $known })
agent-settings-preset-launch = Agent [agent.presets.{ $id }] heeft precies één van command of builtin nodig
