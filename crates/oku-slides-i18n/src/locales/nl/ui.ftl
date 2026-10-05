# Interface de l’app (React). Chargé par ui/src/i18n.ts via la requête « messages ».
ui-loading = Laden…

## Document

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · sjablonen
ui-title-rename = Presentatie hernoemen
ui-title-field = Titel van de presentatie
ui-status-unsaved = niet-opgeslagen wijzigingen
ui-status-saved = opgeslagen
ui-status-never-saved = nog niet opgeslagen
ui-caption-templates = { $count ->
    [one] { $count } sjabloon
   *[other] { $count } sjablonen
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } dia
   *[other] { $count } dia's
} · { $status }
ui-template-default-name = Sjabloon
ui-template-variant-name = { $name } (variant)
ui-confirm-quit-templates = Afsluiten zonder de gewijzigde sjablonen op te slaan?

## Meldingen

ui-toast-templates-saved = Sjablonen opgeslagen: { $name }
ui-toast-moved = Presentatie verplaatst
ui-toast-copied = Kopie opgeslagen
ui-toast-select-zone = Selecteer een gebied om te plaatsen: ‘{ $text }’
ui-toast-field-gone = Het veld dat je bewerkte bestaat niet meer: je invoer is verloren.

## Werkbalk

ui-toolbar-undo = Ongedaan maken
ui-toolbar-undo-label = Ongedaan maken: { $label }
ui-toolbar-redo = Opnieuw
ui-toolbar-redo-label = Opnieuw: { $label }
ui-toolbar-grid-hide = Raster verbergen
ui-toolbar-grid-show = Raster tonen
ui-toolbar-ghosts-hide = Geesten verbergen
ui-toolbar-ghosts-show = Geesten tonen
ui-divider-pile = Breedte van de diastapel
ui-divider-notes = Breedte van de agentdraad
ui-divider-code = Grootte van de code
ui-toolbar-theme = Thema
ui-toolbar-theme-default = Standaard
ui-toolbar-save-copy = Kopie opslaan
ui-toolbar-finish = Klaar
ui-toolbar-templates = Sjablonen
ui-toolbar-open = Openen
ui-merge = Samenvoegen met een pdf…
ui-menu = Menu
ui-menu-move = Verplaatsen…
ui-language = Taal
ui-language-system = Systeemtaal
ui-scheme = Weergave
ui-scheme-system = Systeem
ui-scheme-light = Licht
ui-scheme-dark = Donker
ui-toolbar-add-label = Toevoegen aan de dia
ui-add-kind = { $kind ->
    [card] Kaart
    [shape] Vorm
    [chart] Grafiek
   *[text] Tekst
}
ui-add-widget = Pakketwidget
ui-toolbar-remove-template = Sjabloon verwijderen
ui-toolbar-remove-slide = Dia verwijderen
ui-toolbar-remove-hint = Dia { $n } · ongedaan maken met ⌘Z

## Labels voor ongedaan maken

ui-undo-add = { $kind ->
    [card] Kaart toevoegen
    [shape] Vorm toevoegen
    [chart] Grafiek toevoegen
   *[text] Tekst toevoegen
}
ui-undo-add-widget = “{ $name }” toevoegen
ui-undo-new-template = Nieuw sjabloon: { $name }
ui-undo-new-slide = Nieuwe dia: { $name }
ui-undo-templates = Sjablonen: { $name }
ui-undo-move-template = Sjabloon verplaatsen
ui-undo-move-slide = Dia verplaatsen
ui-undo-remove-slide = Dia verwijderen
ui-undo-remove = Verwijderen
ui-undo-cut = Knippen
ui-undo-paste = Plakken
ui-undo-place-on-grid = Op het raster plaatsen
ui-undo-move-on-grid = Op het raster verplaatsen
ui-undo-resize-on-grid = Formaat op het raster wijzigen
ui-undo-change-anchor = Verankering wijzigen
ui-undo-move = Verplaatsen
ui-undo-resize = Formaat wijzigen
ui-undo-edit-slide = Dia bewerken
ui-undo-dictation = Dicteren
ui-undo-rename-set = Set hernoemen
ui-undo-rename-deck = Presentatie hernoemen
ui-undo-theme = Thema wijzigen
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
ui-pile-templates = Sjablonen
ui-pile-add-slide = + Nieuwe dia
ui-pile-add-template = + Nieuw sjabloon
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
    [group] Groep
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
ui-undo-list-add = { $item } toevoegen
ui-undo-list-remove = { $item } verwijderen
ui-undo-list-move = { $item } verplaatsen
ui-agent-color-other = Andere kleur
ui-agent-thread = Agent
ui-agent-new-conversation = Nieuw gesprek
ui-draft = Concept
ui-draft-insert = Invoegen
ui-draft-insert-title = Ongewijzigd invoegen (⌘⇧↩)
ui-draft-send = Naar de agent sturen
ui-draft-send-title = Naar de agent sturen (⌘↩)
ui-draft-discard = Weggooien
ui-agent-state-idle = Agent
ui-agent-state-starting = De agent start…
ui-agent-state-working = De agent is bezig…
ui-agent-dock = Bezig
ui-agent-dock-settings = Instellingen
ui-agent-jump = ↓ Huidige beurt
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
ui-dictation-to-field = { $field } vervangen
ui-dictation-to-block = na het geselecteerde blok
ui-dictation-new-block = nieuw tekstblok
ui-dictation-select-zone = selecteer een gebied
ui-dictation-downloading = Model { $name } downloaden
ui-dictation-download-progress = { $percent }% · { $received } van { $total } · slechts één keer
ui-dictation-listening = Luisteren · { $time }
ui-dictation-transcribing = Laatste zin transcriberen…
ui-dictation-stop-in = Stopt over { $duration }
ui-dictation-speak = Spreek, de tekst verschijnt hier…
ui-dictation-auto-stop = stopt na { $duration } stilte
ui-dictation-to-download = { $size } te downloaden
ui-dictation-settings = Dicteerinstellingen
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
ui-picker-new-template = Nieuw sjabloon, gebaseerd op…
ui-picker-set = Sjablonen
ui-picker-set-mine = { $name } · eigen
ui-picker-keys = ← → om te bladeren · Enter om toe te voegen · Esc om te sluiten
ui-picker-close = Sluiten
ui-picker-previous = Vorig sjabloon
ui-picker-next = Volgend sjabloon
ui-picker-list = Diasjablonen

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

## Bestanden

ui-file-open-prompt = Pad van het te openen bestand (.oku-slides.pdf of .json)
ui-file-save-prompt = Opslaan als (volledig pad)

## Tabbladen

ui-tabs = Geopende presentaties
ui-tab-new = Nieuw tabblad
ui-tab-close = Tabblad sluiten
ui-tab-unsaved = Niet-opgeslagen wijzigingen
ui-tab-templates = Sjablonen
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
ui-help-previous = Vorige dia
ui-help-next = Volgende dia
ui-help-close = Help sluiten
ui-help-search = Zoeken in de help
ui-help-search-placeholder = Zoeken…
ui-help-search-empty = Geen resultaten
ui-help-search-count = { $count ->
    [one] 1 resultaat
   *[other] { $count } resultaten
}
ui-pack-notice-close = Sluiten
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
ui-widgets-edit = Widgets bewerken
ui-widgets-open = Widget { $name } openen
ui-widgets-delete-named = { $name } verwijderen
ui-widgets-confirm-delete = De widget “{ $name }” verwijderen?
ui-widgets-rename-hint = Dubbelklik of F2 om te hernoemen
ui-widgets-tabs = Sjabloontabbladen
ui-widgets-templates-tab = Sjablonen
ui-widgets-close-tab = { $name } sluiten
ui-widgets-removed-by-agent = De agent heeft widget { $name } verwijderd.
