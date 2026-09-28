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
ui-status-builtin = ingebouwd: opslaan maakt een kopie
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

## Werkbalk

ui-toolbar-undo = Ongedaan maken
ui-toolbar-undo-label = Ongedaan maken: { $label }
ui-toolbar-redo = Opnieuw
ui-toolbar-redo-label = Opnieuw: { $label }
ui-toolbar-grid-hide = Raster verbergen
ui-toolbar-grid-show = Raster tonen
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
ui-toolbar-add-label = Toevoegen aan de dia
ui-add-kind = { $kind ->
    [card] Kaart
    [shape] Vorm
    [chart] Grafiek
   *[text] Tekst
}
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
ui-undo-new-template = Nieuw sjabloon: { $name }
ui-undo-new-slide = Nieuwe dia: { $name }
ui-undo-grid = Raster { $columns } × { $rows }
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
ui-undo-edit-text = Tekst bewerken
ui-undo-dictation = Dicteren
ui-undo-rename-set = Set hernoemen
ui-undo-rename-deck = Presentatie hernoemen
ui-undo-theme = Thema wijzigen
ui-undo-rename-template = Sjabloon hernoemen
ui-undo-describe-template = Sjabloon beschrijven
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
ui-stage-edit-text = Tekst bewerken
ui-stage-hint-field = Enter om te bevestigen · Esc om te annuleren
ui-stage-hint-block = ⌘ Enter om te bevestigen · één regel per alinea, ‘- ’ voor een opsommingsteken · ⌘B vet, ⌘I cursief, ⌘K link
ui-field = { $kind ->
    [title] Titel
    [subtitle] Ondertitel
    [kicker] Bovenregel
    [text] Tekst
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

## Dianotities

ui-notes = Dia-hulpmiddelen
ui-stepper-less = { $label }: één minder
ui-stepper-more = { $label }: één meer
ui-agent-color-other = Andere kleur
ui-agent-thread = Agent
ui-agent-new-conversation = Nieuw gesprek
ui-agent-tools = { $count ->
    [one] { $count } tool
   *[other] { $count } tools
}
ui-draft = Concept
ui-draft-insert = Invoegen
ui-draft-insert-title = Ongewijzigd invoegen (⌘⇧↩)
ui-draft-send = Naar de agent sturen
ui-draft-send-title = Naar de agent sturen (⌘↩)
ui-draft-discard = Weggooien
ui-agent-state-idle = Agent
ui-agent-state-starting = De agent start…
ui-agent-state-working = De agent is bezig…
ui-agent-stopped = Gestopt.
ui-agent-ended = Beurt beëindigd ({ $stop }).
ui-agent-stop-max-tokens = De agent heeft zijn lengtelimiet bereikt.
ui-agent-stop-max-turns = De agent heeft zijn stappenlimiet bereikt.
ui-agent-stop-refusal = De agent heeft geweigerd.
ui-agent-sheet = Vraag van de agent
ui-agent-sheet-keys = 1–4 kiezen · Enter bevestigen · E bewerken · Esc annuleren
ui-agent-variants = Varianten
ui-agent-drawer-keys = Klik of 1–4 · Esc sluit
ui-agent-confirm = Bevestiging van de agent

## Dicteren

ui-dictation = Dicteren
ui-dictation-start = Dicteren
ui-dictation-start-title = Dicteren (⌘D)
ui-dictation-agent-stop = Stop de agent (⌘⇧D of Esc)
ui-dictation-agent-working = De agent is bezig… ⌘⇧D of Esc stopt hem
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
ui-dictation-auto-stop-label = Vanzelf stoppen na een stilte van
ui-dictation-silence = Duur van de stilte
ui-dictation-phrases-note = Alleen gesproken passages worden getranscribeerd, zin voor zin, terwijl je spreekt.
ui-dictation-download-now = Nu downloaden ({ $size })
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
