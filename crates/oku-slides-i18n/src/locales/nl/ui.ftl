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
ui-confirm-quit-theme = Afsluiten zonder het gewijzigde thema op te slaan?
ui-confirm-quit-read-only-theme = { $theme } kan hier niet worden opgeslagen. Dit tabblad sluiten en de wijzigingen verliezen?
ui-confirm-update-opener = “{ $deck }” bijwerken naar deze versie van het pakket “{ $pack }”?
ui-close-writes-pending = Dit tabblad slaat nog op: sluit het zo meteen opnieuw.
ui-confirm-close-writes-pending = Dit tabblad slaat nog op. Toch sluiten en verliezen wat niet is opgeslagen?
ui-confirm-close-writes-pending-linked = Dit tabblad slaat nog op, en deze pakkettabbladen sluiten mee zonder te zijn opgeslagen: { $tabs }. Toch sluiten en verliezen wat niet is opgeslagen?
ui-confirm-quit-read-only-pack = { $pack } kan hier niet worden opgeslagen. Dit tabblad sluiten en de wijzigingen verliezen? Vraag de agent om het als nieuw pakket op te slaan om ze te bewaren.

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
ui-toolbar-theme = Thema
ui-toolbar-finish = Klaar
ui-pack-deck-only = Alleen in deze presentatie opgeslagen
ui-toolbar-open = Openen
ui-menu = Menu
ui-menu-new-widget-pack = Nieuw widgetpakket
ui-menu-new-layout-pack = Nieuw lay-outpakket
ui-menu-new-theme = Nieuw thema
ui-toolbar-add-label = Toevoegen aan de dia
ui-toolbar-packs = Pakketten van de presentatie
ui-toolbar-packs-update = Pakketten van de presentatie — updates beschikbaar
ui-toolbar-thumbs-show = Miniaturen tonen
ui-toolbar-thumbs-hide = Miniaturen verbergen
ui-toolbar-remove-slide = Dia verwijderen
ui-toolbar-remove-hint = Dia { $n } · ongedaan maken met ⌘Z

## Labels voor ongedaan maken

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

## Diastapel

ui-pile-slides = Dia's
ui-pile-layouts = Lay-outs
ui-pile-add-slide = Nieuwe dia
ui-pile-drag-hint = Sleep om te verplaatsen · ⌥↑ ⌥↓
ui-slide-alt = Dia { $n }

## Podium

ui-stage-rendering = Weergeven…
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
ui-draft = Concept
ui-draft-send = Naar de agent sturen
ui-draft-send-title = Naar de agent sturen (⌘↩)
ui-draft-discard = Weggooien
ui-pasted-image = { $type } { $width }×{ $height }
ui-draft-images = Geplakte afbeeldingen
ui-draft-remove-image = Deze afbeelding verwijderen
ui-draft-send-needs-text = Zeg wat ermee moet gebeuren
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
ui-dictation-language-auto = Automatische detectie
ui-dictation-model-unavailable = Taal niet ondersteund
ui-dictation-language-auto-resolved = Automatisch ({ $language })
ui-dictation-to-download-language = Taal te downloaden
ui-dictation-api-key-missing = { $var } is niet ingesteld
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
ui-show-as = Weergeven als
ui-show-as-table = Weergeven als tabel
ui-show-as-chart = Weergeven als grafiek

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
ui-tab-kind-pack = { $title }, pakket
ui-tab-kind-theme = { $title }, thema
ui-tab-theme = Thema
ui-preview-theme = Voorbeeldthema
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
ui-help-close = Help sluiten
ui-help-search = Zoeken in de help
ui-help-search-placeholder = Zoeken…
ui-help-search-empty = Geen resultaten
ui-help-search-count = { $count ->
    [one] 1 resultaat
   *[other] { $count } resultaten
}
ui-widgets-common = Gedeeld
ui-widgets-new = Nieuwe widget
ui-widgets-name = Widgetnaam (kleine letters, cijfers en koppeltekens)
ui-layouts-name = Lay-outnaam (kleine letters, cijfers en koppeltekens)
ui-widgets-name-taken = Deze naam is al in gebruik
ui-widgets-name-letter = De naam moet met een letter beginnen
ui-widgets-empty = Dit pakket heeft nog geen widget.
ui-layouts-new = Nieuwe lay-out
ui-layouts-empty = Dit pakket heeft nog geen lay-out.
ui-confirm-delete-ok = Verwijderen
ui-widgets-preview = Widgetvoorbeeld
ui-widgets-no-preview = Geen voorbeeld
ui-widgets-field-bound = Gekoppeld aan de presentatie: hier alleen-lezen
ui-widgets-source = Broncode
ui-widgets-unsaved = Niet opgeslagen
ui-widgets-in-error = Widgets met fouten
ui-widgets-none-in-error = Geen enkele widget heeft fouten
ui-widgets-open = Widget { $name } openen
ui-widgets-delete-named = { $name } verwijderen
ui-widgets-confirm-delete = De widget “{ $name }” verwijderen?
ui-layouts-confirm-delete = De lay-out “{ $name }” verwijderen?
ui-widgets-rename-hint = Dubbelklik of F2 om te hernoemen
ui-theme-name = Naam van het thema
ui-theme-unsaved = Themawijzigingen niet opgeslagen
ui-widgets-removed-by-agent = De agent heeft widget { $name } verwijderd.
ui-layouts-removed-by-agent = De agent heeft lay-out { $name } verwijderd.

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
ui-update-check = Zoeken naar updates…
ui-about = Over Oku Slides
ui-about-version = Versie { $version }
ui-about-changelog = Wat is er nieuw
ui-about-breaking = niet compatibel
ui-about-close = Sluiten

ui-shortcut-theme = thema, thema's, theme, themes
