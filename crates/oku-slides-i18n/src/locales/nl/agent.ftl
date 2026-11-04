# Agent: oppervlakken, bevestigingen, thread.

agent-surface-cancel = Annuleren
agent-surface-confirm = Bevestigen
agent-surface-confirm-danger = Ja, doe het
agent-surface-heard = Gehoord: ‘{ $text }’
agent-surface-say-or-click = Zeg uw antwoord of klik op een knop.
agent-surface-said-too-early = Te vroeg gezegd: wacht op het venster en zeg het dan opnieuw.
agent-surface-click-to-confirm = Klik op de knop om te bevestigen.
agent-surface-verb-replace = Vervangen
agent-surface-verb-merge = Samenvoegen
agent-surface-verb-discard = Weggooien
agent-surface-verb-delete = Verwijderen
agent-surface-verb-use-api = API gebruiken
agent-surface-verb-install = Installeren
agent-surface-verb-quit = Afsluiten
agent-surface-verb-close = Sluiten
agent-surface-verb-import = Importeren
agent-surface-rewrite = Herformulering
agent-surface-pick = { $n } kiezen
agent-surface-keep = Origineel behouden
agent-surface-edit = Bewerken vóór toepassen
agent-surface-close = Sluiten
agent-surface-themes = Thema's
agent-surface-variants = Varianten
agent-surface-permission-always = voor altijd
agent-surface-permission-title = De agent vraagt om toestemming
agent-surface-permission-guides = Deze beurt bevat de ontvangen gids van { $names }.
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
agent-confirm-copy-pack = Een kopie van pakket “{ $name }” opslaan? Het pakket zelf blijft ongewijzigd.
agent-confirm-copy-edited-theme = Een kopie van thema “{ $name }” opslaan? Het thema zelf blijft ongewijzigd.
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

agent-not-found = Agent niet gevonden: { $command } bestaat niet op dat pad en staat niet in het PATH van de app. Geef het volledige pad op in [agent.presets].
agent-spawn-failed = Agent kon niet starten: { $command }. Instelling [agent.presets].
agent-no-http = Deze agent kan geen verbinding maken met de tools van Oku Slides.
agent-local-no-program = De lokale agent werkt alleen binnen de Oku Slides-app.
agent-local-needs-download = De lokale agent moet eerst zijn model downloaden ({ $gigabytes } GB).
agent-local-intel-mac = De lokale agent heeft een Mac met Apple-chip nodig.
agent-local-unsupported-os = De lokale agent is nog niet beschikbaar op dit systeem.
agent-local-apple-off = De lokale agent heeft Apple Intelligence nodig: schakel het in via Systeeminstellingen.
agent-local-apple-not-ready = Het model van Apple wordt nog voorbereid: probeer de lokale agent over een paar minuten opnieuw.
agent-local-apple-unsupported = Het model van Apple vereist een Mac die geschikt is voor Apple Intelligence, met macOS 26 of nieuwer.
agent-model-download = Model downloaden
agent-model-downloading = Model wordt gedownload… { $percent }%
agent-model-cancel = Annuleren
agent-model-download-failed = Het model kon niet worden gedownload.
agent-model-download-integrity = Het gedownloade model was beschadigd en is verwijderd.
agent-model-download-incomplete = Onvolledige download: { $got } van { $expected } bytes.
agent-model-download-cancelled = Download geannuleerd.
agent-fallback-notice = Lokale agent — { $reason }
agent-fallback-download = { $reason } De lokale agent kan het overnemen zodra zijn model ({ $model }, { $gigabytes } GB) is gedownload. Nu downloaden?
agent-fallback-unusable = Geen enkele agent kan werken. Hoofdagent: { $main } Lokale agent: { $local }
local-agent-no-route = De lokale agent begreep niet wat er moest gebeuren. Probeer het anders te zeggen.
local-agent-tool-limit = De lokale agent stopte na { $n } stappen zonder klaar te zijn.
local-agent-invalid-call = De lokale agent kon de hulpmiddelen niet goed gebruiken.
local-agent-engine-failed = Het model van de lokale agent is mislukt.
local-agent-retry = Probeer het opnieuw, of kies een andere agent in het agentpaneel.
agent-stopped = De agent is gestopt.
agent-failed = De agent heeft het verzoek geweigerd: { $message }
agent-error-work-dir = De privéwerkmap van de agent voor dit tabblad kon niet worden voorbereid.
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
agent-trace = { $name }: { $count ->
    [one] { $count } wijziging
   *[other] { $count } wijzigingen
}
agent-trace-data = { $name }: gegevens gewijzigd
agent-trace-removed = { $name }: bestaat niet meer
agent-trace-more = { $count ->
    [one] en { $count } andere
   *[other] en { $count } andere
}
agent-trace-title = Gewijzigde elementen

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
agent-error-tab-no-turn = Open, toon of verplaats een tabblad alleen terwijl je de gebruiker antwoordt.
agent-error-tab-limit = Je hebt in deze beurt al een tabblad geopend, getoond of verplaatst; rond het daar af, of vraag de gebruiker vóór een volgend.
agent-error-continue-in-other-tab = Dit verzoek gaat verder in het tabblad dat je opende of toonde: vraag de gebruiker daar verder te gaan; dit tabblad wijzigt voorlopig geen widget.
agent-error-pack-open-in-tab = Dit pakket wordt in een ander tabblad bewerkt: vraag de gebruiker eerst daar af te ronden.
agent-pack-opened-in-tab = { $shown ->
   [yes] Het pakket is geopend in tabblad { $tab }, dat nu wordt getoond.
  *[no] Het pakket is geopend in tabblad { $tab }, op de achtergrond: het venster van de gebruiker toont een ander tabblad.
} { $draft ->
   [set] Het verzoek wacht in het concept van dat tabblad: zeg de gebruiker het daar na te lezen en dan te versturen.
   [kept] Het concept van dat tabblad bevatte al iets en blijft zoals het is. Zeg de gebruiker daar verder te gaan: de agent van dat tabblad kent dit verzoek niet.
   [too_long] Het verzoek was te lang voor het concept van dat tabblad. Zeg de gebruiker daar verder te gaan: de agent van dat tabblad kent dit verzoek niet.
   [unavailable] Het verzoek kon het concept van dat tabblad niet bereiken. Zeg de gebruiker daar verder te gaan: de agent van dat tabblad kent dit verzoek niet.
  *[none] Zeg de gebruiker daar verder te gaan: de agent van dat tabblad kent dit verzoek niet.
} Dit tabblad blijft zoals het is.
agent-theme-opened-in-tab = { $shown ->
   [yes] Het thema is geopend in tabblad { $tab }, dat nu wordt getoond.
  *[no] Het thema is geopend in tabblad { $tab }, op de achtergrond: het venster van de gebruiker toont een ander tabblad.
} { $draft ->
   [set] Het verzoek wacht in het concept van dat tabblad: zeg de gebruiker het daar na te lezen en dan te versturen.
   [kept] Het concept van dat tabblad bevatte al iets en blijft zoals het is. Zeg de gebruiker daar verder te gaan: de agent van dat tabblad kent dit verzoek niet.
   [too_long] Het verzoek was te lang voor het concept van dat tabblad. Zeg de gebruiker daar verder te gaan: de agent van dat tabblad kent dit verzoek niet.
   [unavailable] Het verzoek kon het concept van dat tabblad niet bereiken. Zeg de gebruiker daar verder te gaan: de agent van dat tabblad kent dit verzoek niet.
  *[none] Zeg de gebruiker daar verder te gaan: de agent van dat tabblad kent dit verzoek niet.
} Dit tabblad blijft zoals het is.
agent-error-turn-over = De beurt van deze aanroep is voorbij: er is niets getoond.
agent-error-no-box-turn = Geen beurt om dit vak aan te koppelen: er werd niets getoond.
agent-error-no-file = De presentatie heeft nog geen bestand: geef een absoluut pad op.
agent-error-path-relative = Het pad moet absoluut zijn.
agent-error-path-parent = Het pad mag geen “..” bevatten.
agent-error-setting-locked = De instelling { $key } kan niet door de agent worden gewijzigd.
agent-error-setting = De instelling { $key } kon niet worden gewijzigd: controleer de waarde.
agent-error-history-index = Om door de geschiedenis te bladeren is de index van een versie nodig.
agent-error-no-pack = Deze presentatie pint geen pakket vast: geef pack (zie packs).
agent-error-history-not-opened = Het tabblad heeft de geschiedenis niet op tijd geopend: vraag het opnieuw.
agent-error-locale = Onbekende taalcode. Geef de code van een meegeleverde taal of system.
agent-error-empty-interface = Niets te wijzigen: geef scheme, locale, guides, folded, pile, notes of code.
agent-error-layout-range = { $field } moet tussen { $min } en { $max } px liggen.
agent-error-layout-min = { $field } moet minstens { $min } px zijn.
agent-error-empty-dictation = Niets te wijzigen: geef minstens één dicteerinstelling.
agent-error-empty-speech = Niets te wijzigen: geef minstens één voorleesinstelling op.
agent-error-not-shown = Dit tabblad is niet het getoonde in zijn venster: vraag het opnieuw zodra de gebruiker het toont.
agent-error-not-presenting = Het deck wordt hier niet gepresenteerd: start de presentatie eerst met de actie start.
agent-error-present-slide = Om een dia te tonen is de id van een dia van dit deck nodig (zie deck_outline).
agent-error-undo-open-turn = undo_turn maakt de lopende beurt niet ongedaan: gebruik undo voor de wijzigingen van deze beurt.
agent-error-app-unavailable = Alleen de app Oku Slides kan dit; hier is het niet beschikbaar.
agent-error-app-closing = Oku Slides wordt al afgesloten.
agent-error-app-no-offer = Hier is geen update om uit te stellen of te installeren: zoek eerst naar updates.
agent-error-app-failed = De update van Oku Slides is mislukt: controleer de verbinding of gebruik de updatebanner.
agent-error-after-turn-taken = Dit antwoord eindigt al met afsluiten, het installeren van een update of het sluiten van zijn tabblad.
agent-error-no-pack-update = Het packpaneel biedt deze update voor dit pack niet aan: zie show_packs.
agent-confirm-update-install = Oku Slides { $version } installeren? De app start opnieuw zodra dit antwoord klaar is.
agent-after-turn-quit-unsaved = Oku Slides is niet afgesloten: een tabblad heeft sinds je verzoek niet-opgeslagen wijzigingen.
agent-error-workspace-unavailable = Alleen de Oku Slides-app kan tabbladen en vensters wijzigen; hier is dat niet beschikbaar.
agent-error-workspace-unknown-tab = Dit venster heeft zo'n tabblad of venster niet.
agent-error-workspace-unknown-window = Zo'n venster bestaat niet.
agent-error-workspace-index = Die positie ligt voorbij het einde van de tabbladbalk.
agent-error-workspace-closing = Oku Slides wordt afgesloten: tabbladen en vensters veranderen niet meer.
agent-confirm-close-tab = Het tabblad ‘{ $tab }’ sluiten?
agent-confirm-discard-tab = Het tabblad ‘{ $tab }’ sluiten? De niet-opgeslagen wijzigingen gaan verloren.
agent-confirm-close-window = Dit venster en de tabbladen ervan sluiten?
agent-confirm-discard-window = Dit venster sluiten? Niet-opgeslagen wijzigingen in { $tabs } gaan verloren.
agent-error-window-last = Dit is het enige venster: sluit in plaats daarvan de tabbladen, of sluit Oku Slides af.
agent-error-tab-index = Zeg waar het tabblad heen moet: herschikken vraagt een positie.
agent-error-tab-unused = Alleen herschikken en verplaatsen nemen een positie, en alleen verplaatsen neemt een venster.
agent-after-turn-close-unsaved = Het tabblad is niet gesloten: het is gewijzigd sinds je verzoek.
agent-after-turn-close-not-shown = Het tabblad is niet gesloten: aan het einde van het antwoord stond een ander tabblad in zijn venster.
agent-confirm-quit = Oku Slides afsluiten? De app sluit af zodra dit antwoord eindigt.
agent-error-quit-unsaved = Tabbladen met niet-opgeslagen werk: { $count }. Oku Slides is niet afgesloten: de gebruiker slaat ze eerst op of sluit ze.
agent-error-tune-unavailable = Het fijnafstemmen kon niet starten: de agent is niet beschikbaar.
agent-error-tune-not-started = Het fijnafstemmen is niet gestart: de agent van dat tabblad is nu onbruikbaar; het tabblad zegt waarom.
agent-confirm-import = { $path } importeren in een nieuw tabblad?
agent-import-opened-in-tab = { $shown ->
   [yes] De presentatie is geopend in tabblad { $tab }, nu getoond.
  *[no] De presentatie is geopend in tabblad { $tab }, op de achtergrond: het venster van de gebruiker toont een ander tabblad.
} { $tune ->
   [started] Het fijnafstemmen begint daar, in het eigen gesprek van dat tabblad.
   [unavailable] Het fijnafstemmen kon niet starten: zeg de gebruiker het in dat tabblad te vragen.
  *[no] Zeg de gebruiker daar verder te gaan.
} Dit tabblad blijft zoals het is.
agent-error-import-hifi-target = Een hifi-import neemt geen pack of thema: laat ze weg, of importeer in retrofit-modus.
agent-error-import-extension = Alleen een PowerPoint-bestand .pptx kan worden geïmporteerd.
agent-error-a2ui-unknown = { $name } is geen component die render accepteert. Componenten: { $known }.
agent-error-a2ui-elsewhere = render toont { $name } niet: gebruik { $tool }.
agent-error-a2ui-host-only = { $name } is voorbehouden aan de app: render kan het niet tonen.
agent-error-themes-deck-mode = De themakiezer opent alleen tijdens het bewerken van de dia's.
agent-error-packs-deck-mode = Het paneel met pakketten van de presentatie opent alleen tijdens het bewerken van de dia's.
agent-error-usage-no-deck = Het gebruik van dit tabblad hoort bij geen enkele presentatie: alleen een pakket- of thematabblad dat vanuit een presentatie is geopend (door de gebruiker of de agent), of het eigen pakket van een presentatie, telt mee voor die presentatie.
agent-error-rewrite-text = De herformulering kwam terug zonder tekst.
agent-error-paragraphs = De alinea's van de herformulering zijn ongeldig.
agent-error-draft-too-long = Het concept is te lang.
agent-error-report-elsewhere = Dit venster toont dat tabblad niet.
agent-error-ui-too-large = De paginacontext is te groot.
agent-error-seq-exhausted = Het nummer van het paginarapport valt buiten het bereik.
agent-error-draft-origin = Concept geweigerd: onbekende weergave.
agent-error-draft-no-agent = De agent van dit tabblad is nog niet gestart: concept geweigerd.
agent-error-too-many-images = Het concept heeft al al zijn afbeeldingen.
agent-error-image-not-base64 = De geplakte afbeelding is niet compleet aangekomen.
agent-error-image-gone = Een geplakte afbeelding is niet meer beschikbaar: plak hem opnieuw.
agent-error-unknown-agent = Deze agent staat niet meer in de configuratie.
agent-error-agent-locked = De keuze van de agent wordt door de omgeving opgelegd.
agent-error-agent-not-saved = De keuze van de agent kon niet worden opgeslagen.
agent-error-unknown-model = Deze agent biedt dat model niet aan.
agent-error-model-not-saved = De keuze van het model kon niet worden opgeslagen.
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
voice-error-bad-api-model = De naam van het API-model mag alleen letters, cijfers en . _ : / - bevatten; een Azure-implementatienaam alleen letters, cijfers en . _ -
voice-error-bad-api-version = De Azure-API-versie bevat alleen letters, cijfers, . en -, hoogstens 32 tekens, zoals 2025-03-01-preview
voice-error-bad-api-headers = Headers schrijf je als Naam: waarde; Naam: $VARIABLE, hoogstens 8. Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection en dubbele namen worden geweigerd
voice-error-api-reserved = Alleen de gebruiker stelt het adres, de sleutelvariabele, de Azure-versie en de headers van de API in: open het venster met show_dictation en vraag het hem
voice-error-hands-free-reserved = Alleen de gebruiker zet handsfree luisteren aan: open het venster met show_dictation en vraag het hem
voice-error-changed = De instellingen zijn sinds de vraag veranderd: vraag opnieuw
voice-error-nothing-to-download = Niets te downloaden voor deze agent
voice-error-bad-rate = De spreeksnelheid moet tussen 0,5 en 2 liggen
voice-error-unknown-voice = Onbekende stem voor de taal van de interface
voice-error-speech-unavailable = Voorlezen is niet beschikbaar op deze computer
agent-tune-prompt = De geïmporteerde presentatie verfijnen
agent-settings-unknown-selected = Instelling agent.selected = { $selected } noemt geen agent uit [agent.presets] (bekend: { $known })
agent-settings-preset-launch = Agent [agent.presets.{ $id }] heeft precies één van command of builtin nodig
