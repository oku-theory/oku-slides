# Interface de l’app (React). Chargé par ui/src/i18n.ts via la requête « messages ».
ui-loading = Laden…

## Document

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · lay-outpakket
   *[other] { $name } · widgetpakket
}
ui-title-rename = Presentatie hernoemen
ui-title-field = Titel van de presentatie
ui-status-unsaved = niet-opgeslagen wijzigingen
ui-status-saved = opgeslagen
ui-status-never-saved = nog niet opgeslagen
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count } lay-out
       *[other] { $count } lay-outs
    }
   *[other] { $count ->
        [one] { $count } widget
       *[other] { $count } widgets
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } dia
   *[other] { $count } dia's
} · { $status }
ui-confirm-quit-pack = Afsluiten zonder het gewijzigde pakket op te slaan?

## Meldingen

ui-toast-copied = Kopie opgeslagen
ui-toast-field-gone = Het veld dat je bewerkte bestaat niet meer: je invoer is verloren.
ui-toast-edit-replaced = Deze tekst is veranderd terwijl je hem bewerkte: jouw versie heeft hem vervangen. Ongedaan maken zet de andere terug.

## Werkbalk

ui-toolbar-undo = Ongedaan maken
ui-toolbar-undo-label = Ongedaan maken: { $label }
ui-toolbar-redo = Opnieuw
ui-toolbar-redo-label = Opnieuw: { $label }
ui-divider-pile = Breedte van de diastapel
ui-divider-notes = Breedte van de agentdraad
ui-divider-code = Grootte van de code
ui-divider-band = Hoogte van de instellingen
ui-toolbar-theme = Thema
ui-toolbar-theme-embedded = Thema van het bestand
ui-toolbar-theme-deck-group = Deze presentatie
ui-toolbar-finish = Klaar
ui-pack-deck-only = Alleen in deze presentatie opgeslagen
ui-toolbar-open = Openen
ui-menu = Menu
ui-toolbar-add-label = Toevoegen aan de dia
ui-add-kind = { $kind ->
    [card] Kaart
    [shape] Vorm
    [chart] Grafiek
    [table] Tabel
   *[text] Tekst
}
ui-add-kind-description = { $kind ->
    [card] Een titel en een korte tekst in een kader
    [shape] Een gekleurde vorm
    [chart] Een grafiek uit een gegevenstabel
    [table] Een gegevenstabel
   *[text] Een alinea tekst
}
ui-toolbar-remove-slide = Dia verwijderen
ui-toolbar-remove-hint = Dia { $n } · ongedaan maken met ⌘Z

## Labels voor ongedaan maken

ui-undo-add = { $kind ->
    [card] Kaart toevoegen
    [shape] Vorm toevoegen
    [chart] Grafiek toevoegen
    [table] Een tabel toevoegen
   *[text] Tekst toevoegen
}
ui-undo-add-widget = “{ $name }” toevoegen
ui-undo-new-slide = Nieuwe dia: { $name }
ui-undo-move-slide = Dia verplaatsen
ui-undo-remove-slide = Dia verwijderen
ui-undo-remove = Verwijderen
ui-undo-paste = Plakken
ui-undo-place-on-grid = Op het raster plaatsen
ui-undo-move-on-grid = Op het raster verplaatsen
ui-undo-resize-on-grid = Formaat op het raster wijzigen
ui-undo-change-anchor = Verankering wijzigen
ui-undo-move = Verplaatsen
ui-undo-resize = Formaat wijzigen
ui-undo-edit-slide = Dia bewerken
ui-undo-rename-pack = Pakket hernoemen
ui-undo-rename-deck = Presentatie hernoemen
ui-undo-chart-kind = Grafiektype wijzigen
ui-undo-chart-data = Gegevens bewerken
ui-undo-chart-color = Reeks kleuren
ui-undo-chart-stack = { $stacked ->
    [yes] Reeksen stapelen
   *[no] Reeksen scheiden
}

## Nieuwe elementen

ui-new-text = Nieuwe tekst
ui-new-card-title = Kaarttitel
ui-new-card-detail = Detail

## Diastapel

ui-pile-slides = Dia's
ui-pile-layouts = Lay-outs
ui-pile-add-slide = + Nieuwe dia
ui-pile-drag-hint = Sleep om te verplaatsen · ⌥↑ ⌥↓
ui-slide-alt = Dia { $n }

## Podium

ui-stage-rendering = Weergeven…
ui-stage-widget-pack = Open een widget uit de stapel of maak er een.
ui-stage-elements = Dia-elementen
ui-stage-fields = Diavelden
ui-stage-edit-text = Tekst bewerken
ui-stage-hint-plain = Enter of Tab voor het volgende · Esc om te annuleren
ui-stage-hint-rich = Tab voor het volgende · ⌘ Enter om te bevestigen · ‘- ’ voor een opsommingsteken · ⌘B vet, ⌘I cursief, ⌘K link
ui-field = { $kind ->
    [title] Titel
    [subtitle] Ondertitel
    [kicker] Bovenregel
    [text] Tekst
    [body] Hoofdtekst
    [author] Auteur
    [date] Datum
    [organization] Organisatie
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Tekst
    [card] Kaart
    [image] Afbeelding
    [shape] Vorm
    [chart] Grafiek
    [table] Tabel
    [stack] Stapel
   *[other] { $kind }
}
ui-grid-cell = kol. { $cols } · rij { $rows }
ui-ghost-zone = Zone

## Dianotities

ui-notes = Dia-hulpmiddelen
ui-stepper-less = { $label }: één minder
ui-stepper-more = { $label }: één meer
ui-list-item = item
ui-list-add = { $item } toevoegen
ui-list-remove = { $item } verwijderen
ui-list-move = { $item } verplaatsen
ui-list-full = De lijst is vol ({ $max })
ui-undo-list-move = { $item } verplaatsen
ui-agent-color-other = Andere kleur
ui-agent-thread = Agent
ui-agent-new-conversation = Nieuw gesprek
ui-agent-undo-turn = Deze beurt ongedaan maken
ui-agent-detach = Open de agent in een apart venster
ui-agent-attach = Breng de agent terug naar het hoofdvenster
ui-agent-window-empty = Geen agent voor dit tabblad
ui-agent-kept-in-draft = Nergens om te schrijven: tekst bewaard in het concept
ui-agent-busy-elsewhere = De agent antwoordt in { $place }…
ui-agent-scope-slides = de dia's
ui-agent-scope-atelier = het atelier
ui-draft = Concept
ui-draft-send = Naar de agent sturen
ui-draft-send-title = Naar de agent sturen (⌘↩)
ui-draft-discard = Weggooien
ui-agent-state-idle = Agent
ui-agent-state-starting = De agent start…
ui-agent-state-working = De agent is bezig…
ui-agent-dock = Bezig
ui-agent-dock-settings = Instellingen
ui-agent-jump = ↓ Huidige beurt
ui-agent-tool-input = Argumenten
ui-agent-tool-output = Resultaat
ui-agent-sheet = Vraag van de agent
ui-agent-sheet-keys = 1–4 kiezen · Enter bevestigen · E bewerken · Esc annuleren
ui-agent-variants = Varianten
ui-agent-drawer-keys = Klik of 1–4 · Esc sluit
ui-agent-confirm = Bevestiging van de agent

## Dicteren

ui-dictation = Dicteren
ui-dictation-start = Dicteren
ui-dictation-start-title = Dicteren (⌘D)
ui-dictation-type = Schrijf naar de agent
ui-dictation-type-title = Schrijf naar de agent (⌘K)
ui-dictation-agent-stop = Stop de agent (⌘⇧D of Esc)
ui-dictation-agent-working = De agent is bezig…
ui-dictation-agent-working-hint = ⌘⇧D of Esc stopt hem
ui-dictation-finish = Dicteren beëindigen
ui-dictation-finish-title = Beëindigen (⌘D)
ui-dictation-cancel = Annuleren
ui-dictation-preparing = Dicteren voorbereiden…
ui-dictation-heard-nothing = Niets gehoord: spreek iets dichter bij de microfoon
ui-dictation-to-cursor = bij de cursor
ui-dictation-to-draft = in het concept
ui-dictation-downloading = Model { $name } downloaden
ui-dictation-download-progress = { $percent }% · { $received } van { $total } · slechts één keer
ui-dictation-listening = Luisteren · { $time }
ui-dictation-transcribing = Laatste zin transcriberen…
ui-dictation-stop-in = Stopt over { $duration }
ui-dictation-speak = Spreek, de tekst verschijnt hier…
ui-dictation-auto-stop = stopt na { $duration } stilte
ui-dictation-to-download = { $size } te downloaden
ui-dictation-settings = Instellingen
ui-dictation-model-title = Herkenningsmodel
ui-dictation-model-intro = Whisper, op deze computer (CTranslate2). Elk model begrijpt meerdere talen.
ui-dictation-note-turbo = Aanbevolen, snel en nauwkeurig
ui-dictation-note-large = Het nauwkeurigst, trager
ui-dictation-note-base = Licht, voor kleine computers
ui-dictation-installed = geïnstalleerd
ui-dictation-not-installed = te downloaden
ui-dictation-language = Gesproken taal
ui-dictation-language-auto = Automatische detectie
ui-dictation-note-apple = De snelste, ingebouwd in het systeem
ui-dictation-model-unavailable = Taal niet ondersteund
ui-dictation-model-intro-apple = Apple, ingebouwd in het systeem, of Whisper, op deze computer (CTranslate2). Ze begrijpen allemaal meerdere talen.
ui-dictation-language-auto-resolved = Automatisch ({ $language })
ui-dictation-to-download-language = Taal te downloaden
ui-dictation-auto-stop-label = Vanzelf stoppen na een stilte van
ui-dictation-silence = Duur van de stilte
ui-dictation-phrases-note = Alleen gesproken passages worden getranscribeerd, zin voor zin, terwijl je spreekt.
ui-dictation-download-now = Nu downloaden ({ $size })
ui-dictation-download-language = Taal downloaden
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Sjabloonkiezer

ui-picker-new-slide = Nieuwe dia
ui-picker-set-mine = { $name } · eigen
ui-picker-set-received = { $name } · ontvangen
ui-picker-keys = ← → om te bladeren · Enter om toe te voegen · Esc om te sluiten
ui-picker-close = Sluiten
ui-picker-list = Lay-outs
ui-widget-picker-filter = Filteren op naam, label of beschrijving
ui-widget-picker-empty = Geen widget komt overeen met ‘{ $query }’.
ui-widget-picker-list = Widgets
ui-widget-picker-keys = Pijltjes om te bladeren · Enter om toe te voegen · Esc om te sluiten

## Grafiek

ui-chart-panel = Grafiek
ui-chart-kinds = Grafiektype
ui-chart-kind = { $kind ->
    [line] Lijnen
    [pie] Cirkel
    [scatter] Spreiding
    [radar] Radar
    [waterfall] Waterval
   *[bar] Staven
}
ui-chart-no-data = Deze grafiek heeft geen gegevenstabel.
ui-chart-options = Grafiekopties
ui-chart-stack = Gestapeld
ui-chart-waterfall-hint = Vink Totaal aan voor een totaalbalk.
ui-chart-colors = Reekskleuren
ui-chart-color-of = Kleur van { $name }
ui-chart-color-auto = Automatisch
ui-chart-color-pick = Kleur { $n }
ui-chart-column-name = Naam van kolom { $col }
ui-chart-remove-column = Kolom verwijderen
ui-chart-cell = Rij { $row }, kolom { $col }
ui-chart-remove-row = Rij verwijderen
ui-chart-add-row = Rij
ui-chart-add-column = Kolom
ui-chart-series = Reeks { $n }

## Tabel

ui-table-panel = Tabel
ui-table-options = Tabelopties
ui-table-header-row = Koprij
ui-table-header-column = Eerste kolom als kop
ui-table-striped = Gestreepte rijen
ui-table-alignment = Kolomuitlijning
ui-table-align-of = Uitlijning van { $name }
ui-table-align = { $align ->
    [start] Begin
    [center] Gecentreerd
    [end] Einde
   *[auto] Automatisch
}
ui-table-column = Kolom { $n }
ui-table-no-data = Deze tabel bevat geen gegevens.
ui-undo-table-option = Tabelopties
ui-undo-table-align = De kolom uitlijnen
ui-undo-table-data = De tabel bewerken
ui-show-as = Weergeven als
ui-show-as-table = Weergeven als tabel
ui-show-as-chart = Weergeven als grafiek
ui-undo-show-as-table = Weergeven als tabel
ui-undo-show-as-chart = Weergeven als grafiek

## Bestanden

ui-file-open-prompt = Pad van het te openen bestand (.oku-slides.pdf, .json of .pptx)
ui-file-filter-presentations = Presentaties
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = Opslaan als (volledig pad)

## Tabbladen

ui-tabs = Geopende presentaties
ui-tab-new = Nieuw tabblad
ui-tab-close = Tabblad sluiten
ui-tab-unsaved = Niet-opgeslagen wijzigingen
ui-tab-pack = Pakket
ui-tab-move-new-window = Naar nieuw venster verplaatsen
ui-tab-move-to = Naar venster { $n } verplaatsen
ui-tab-close-others = Andere tabbladen sluiten
ui-close-cancel = Annuleren
ui-disk-replaced-question = { $file } is op de schijf vervangen door een andere presentatie.
ui-disk-replaced-overwrite = Overschrijven
ui-disk-replaced-save-as = Opslaan als…
ui-same-deck-question = { $file } is een versie van de presentatie die open is in het tabblad ‘{ $title }’.
ui-same-deck-merge = Samenvoegen in dat tabblad
ui-same-deck-apart = Apart openen
ui-history = Geschiedenis
ui-history-version = Versie van { $date }
ui-history-restore = Herstellen
ui-history-back = Terug
ui-history-anonymous = Anoniem
ui-history-slider = Versies van de presentatie
ui-history-removed-since = sindsdien verwijderd
ui-history-changed-since = sindsdien gewijzigd
ui-history-added-since = sindsdien toegevoegde dia's: { $count }
ui-help = Help
ui-help-caption = Dia { $index } van { $count } · alleen-lezen
ui-slide-previous = Vorige dia
ui-slide-next = Volgende dia
ui-present = Presentatie
ui-present-slide = Dia { $index } van { $count }
ui-present-screen = Presenteren op volledig scherm
ui-toolbar-params-hide = Instellingen verbergen
ui-toolbar-params-show = Instellingen tonen
ui-help-close = Help sluiten
ui-help-search = Zoeken in de help
ui-help-search-placeholder = Zoeken…
ui-help-search-empty = Geen resultaten
ui-help-search-count = { $count ->
    [one] 1 resultaat
   *[other] { $count } resultaten
}
ui-widgets-files = Widgetbestanden
ui-widgets-common = Gedeeld
ui-widgets-new = Nieuwe widget
ui-widgets-name = Widgetnaam (kleine letters, cijfers en koppeltekens)
ui-widgets-name-taken = Deze naam is al in gebruik
ui-widgets-name-letter = De naam moet met een letter beginnen
ui-widgets-valid = Geldig
ui-widgets-errors = Fouten
ui-widgets-empty = Dit pakket heeft nog geen widget.
ui-widgets-preview = Widgetvoorbeeld
ui-widgets-no-preview = Geen voorbeeld
ui-widgets-field-bound = Gekoppeld aan de presentatie: hier alleen-lezen
ui-widgets-source = Broncode
ui-widgets-unsaved = Niet opgeslagen
ui-widgets-in-error = Widgets met fouten
ui-widgets-none-in-error = Geen enkele widget heeft fouten
ui-widgets-section = Widgets
ui-widgets-open = Widget { $name } openen
ui-widgets-delete-named = { $name } verwijderen
ui-widgets-confirm-delete = De widget “{ $name }” verwijderen?
ui-widgets-rename-hint = Dubbelklik of F2 om te hernoemen
ui-pack-tabs = Pakkettabbladen
ui-pack-layouts-tab = Lay-outs
ui-theme-tab = Thema
ui-theme-picker = Thema
ui-theme-name = Naam van het thema
ui-theme-unsaved = Themawijzigingen niet opgeslagen
ui-widgets-close-tab = { $name } sluiten
ui-widgets-removed-by-agent = De agent heeft widget { $name } verwijderd.

## Importeren

ui-import-title = Een PowerPoint-presentatie importeren
ui-import-mode = Importmodus
ui-import-hifi = Getrouwe kopie
ui-import-hifi-note = Behoudt het uiterlijk van het origineel, met een thema en lay-outs voor deze presentatie.
ui-import-retrofit = Mijn lay-outs gebruiken
ui-import-retrofit-note = Bouwt de dia's opnieuw op met lay-outpakketten en een thema uit de bibliotheek.
ui-import-layout-packs = Lay-outpakketten
ui-import-theme-default = Standaardthema
ui-import-no-packs = Geen lay-outpakket in de bibliotheek.
ui-import-tune = Verfijnen met de agent
ui-import-tune-note = De agent vergelijkt elke dia met het origineel en corrigeert de verschillen.
ui-import-tune-note-retrofit = De agent vergelijkt elke dia met het origineel en ontwerpt widgets voor de vormen die je lay-outs niet kunnen tonen.
ui-import-tune-unavailable = Geen bruikbare agent: { $reason }
ui-import-submit = Importeren
ui-import-busy = Bezig met importeren…
ui-update-available = Oku Slides { $version } is beschikbaar.
ui-update-install = Installeren en opnieuw starten
ui-update-see-release = Release bekijken
ui-update-later = Straks
ui-update-installing = Update downloaden… { $percent }%
ui-update-downloading = Update downloaden…
ui-update-up-to-date = Oku Slides is up-to-date.

ui-shortcut-theme = thema, thema's, theme, themes
