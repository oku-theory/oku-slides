# Interfaccia dell’app (React). Caricata da ui/src/i18n.ts tramite la richiesta «messages».
ui-loading = Caricamento…

## Documento

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · modelli
ui-title-rename = Rinomina la presentazione
ui-title-field = Titolo della presentazione
ui-status-unsaved = modifiche non salvate
ui-status-saved = salvato
ui-status-never-saved = non ancora salvato
ui-status-builtin = integrato: salvandolo si crea una copia
ui-caption-templates = { $count ->
    [one] { $count } modello
   *[other] { $count } modelli
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } diapositiva
   *[other] { $count } diapositive
} · { $status }
ui-template-default-name = Modello
ui-template-variant-name = { $name } (variante)
ui-confirm-quit-templates = Uscire senza salvare i modelli modificati?

## Notifiche

ui-toast-saved = La presentazione è stata salvata
ui-toast-templates-saved = Modelli salvati: { $name }
ui-toast-select-zone = Seleziona un’area in cui posizionare: «{ $text }»

## Barra degli strumenti

ui-toolbar-undo = Annulla
ui-toolbar-undo-label = Annulla: { $label }
ui-toolbar-redo = Ripeti
ui-toolbar-redo-label = Ripeti: { $label }
ui-toolbar-grid-hide = Nascondi la griglia
ui-toolbar-grid-show = Mostra la griglia
ui-toolbar-grid-hide-title = Nascondi la griglia (⌘')
ui-toolbar-grid-show-title = Mostra la griglia (⌘')
ui-toolbar-theme = Tema
ui-toolbar-theme-default = Predefinito
ui-toolbar-save-copy = Salva una copia
ui-toolbar-save = Salva
ui-toolbar-finish = Fine
ui-toolbar-templates = Modelli
ui-toolbar-templates-title = Crea e modifica i modelli di diapositiva
ui-toolbar-open = Apri
ui-merge = Unisci con un PDF…
ui-toolbar-add-label = Aggiungi alla diapositiva
ui-toolbar-add = Aggiungi
ui-add-kind = { $kind ->
    [card] Scheda
    [shape] Forma
    [chart] Grafico
   *[text] Testo
}
ui-toolbar-remove-template = Elimina il modello
ui-toolbar-remove-slide = Elimina la diapositiva
ui-toolbar-remove-hint = Diapositiva { $n } · annulla con ⌘Z

## Etichette di annullamento

ui-undo-add = { $kind ->
    [card] Aggiungi una scheda
    [shape] Aggiungi una forma
    [chart] Aggiungi un grafico
   *[text] Aggiungi un testo
}
ui-undo-new-template = Nuovo modello: { $name }
ui-undo-new-slide = Nuova diapositiva: { $name }
ui-undo-grid = Griglia { $columns } × { $rows }
ui-undo-templates = Modelli: { $name }
ui-undo-move-template = Sposta il modello
ui-undo-move-slide = Sposta la diapositiva
ui-undo-remove-slide = Elimina la diapositiva
ui-undo-remove = Elimina
ui-undo-cut = Taglia
ui-undo-paste = Incolla
ui-undo-place-on-grid = Posiziona sulla griglia
ui-undo-move-on-grid = Sposta sulla griglia
ui-undo-resize-on-grid = Ridimensiona sulla griglia
ui-undo-change-anchor = Cambia l’ancoraggio
ui-undo-move = Sposta
ui-undo-resize = Ridimensiona
ui-undo-edit-text = Modifica il testo
ui-undo-dictation = Dettatura
ui-undo-rename-set = Rinomina il set
ui-undo-rename-deck = Rinomina la presentazione
ui-undo-theme = Cambia tema
ui-undo-rename-template = Rinomina il modello
ui-undo-describe-template = Descrivi il modello
ui-undo-chart-kind = Cambia il tipo di grafico
ui-undo-chart-data = Modifica i dati
ui-undo-chart-color = Colora una serie
ui-undo-chart-stack = { $stacked ->
    [yes] Impila le serie
   *[no] Separa le serie
}

## Nuovi elementi

ui-new-text = Nuovo testo
ui-new-card-title = Titolo della scheda
ui-new-card-detail = Dettaglio

## Pila delle diapositive

ui-pile-slides = Diapositive
ui-pile-templates = Modelli
ui-pile-add-slide = + Nuova diapositiva
ui-pile-add-template = + Nuovo modello
ui-pile-drag-hint = Trascina per spostare · ⌥↑ ⌥↓
ui-slide-alt = Diapositiva { $n }

## Palco

ui-stage-rendering = Rendering…
ui-stage-elements = Elementi della diapositiva
ui-stage-edit-text = Modifica il testo
ui-stage-hint-field = Invio per confermare · Esc per annullare
ui-stage-hint-block = ⌘ Invio per confermare · una riga per paragrafo, «- » per un punto elenco · ⌘B grassetto, ⌘I corsivo, ⌘K link
ui-field = { $kind ->
    [title] Titolo
    [subtitle] Sottotitolo
    [kicker] Occhiello
    [text] Testo
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Testo
    [card] Scheda
    [image] Immagine
    [shape] Forma
    [group] Gruppo
   *[other] { $kind }
}
ui-grid-cell = col. { $cols } · riga { $rows }

## Note della diapositiva

ui-notes = Strumenti della diapositiva
ui-stepper-less = { $label }: uno in meno
ui-stepper-more = { $label }: uno in più

## Dettatura

ui-dictation = Dettatura
ui-dictation-start = Detta
ui-dictation-start-title = Detta (⌘D)
ui-dictation-finish = Termina la dettatura
ui-dictation-finish-title = Termina (⌘D)
ui-dictation-cancel = Annulla
ui-dictation-preparing = Preparazione della dettatura…
ui-dictation-heard-nothing = Non si è sentito nulla: parla un po’ più vicino al microfono
ui-dictation-to-cursor = al cursore
ui-dictation-to-field = sostituisci il { $field }
ui-dictation-to-block = dopo il blocco selezionato
ui-dictation-new-block = nuovo blocco di testo
ui-dictation-select-zone = seleziona un’area
ui-dictation-downloading = Download del modello { $name }
ui-dictation-download-progress = { $percent }% · { $received } di { $total } · una sola volta
ui-dictation-listening = In ascolto · { $time }
ui-dictation-transcribing = Trascrizione dell’ultima frase…
ui-dictation-stop-in = Arresto tra { $duration }
ui-dictation-speak = Parla, il testo compare qui…
ui-dictation-auto-stop = si ferma dopo { $duration } di silenzio
ui-dictation-to-download = { $size } da scaricare
ui-dictation-settings = Impostazioni della dettatura
ui-dictation-model-title = Modello di riconoscimento
ui-dictation-model-intro = Whisper, su questo computer (CTranslate2). Ogni modello comprende più lingue.
ui-dictation-note-turbo = Consigliato, veloce e preciso
ui-dictation-note-large = Il più preciso, più lento
ui-dictation-note-base = Leggero, per computer modesti
ui-dictation-installed = installato
ui-dictation-not-installed = da scaricare
ui-dictation-language = Lingua parlata
ui-dictation-language-auto = Rilevamento automatico
ui-dictation-auto-stop-label = Fermati da solo dopo un silenzio di
ui-dictation-silence = Durata del silenzio
ui-dictation-phrases-note = Vengono trascritti solo i passaggi parlati, frase per frase, mentre parli.
ui-dictation-download-now = Scarica ora ({ $size })
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Selettore di modelli

ui-picker-new-slide = Nuova diapositiva
ui-picker-new-template = Nuovo modello, basato su…
ui-picker-set = Modelli
ui-picker-set-mine = { $name } · miei
ui-picker-keys = ← → per sfogliare · Invio per aggiungere · Esc per chiudere
ui-picker-close = Chiudi
ui-picker-previous = Modello precedente
ui-picker-next = Modello successivo
ui-picker-list = Modelli di diapositiva

## Grafico

ui-chart-panel = Grafico
ui-chart-kinds = Tipo di grafico
ui-chart-kind = { $kind ->
    [line] Linee
    [pie] Torta
    [scatter] Dispersione
    [radar] Radar
    [waterfall] Cascata
   *[bar] Barre
}
ui-chart-no-data = Questo grafico non ha una tabella di dati.
ui-chart-options = Opzioni del grafico
ui-chart-stack = In pila
ui-chart-waterfall-hint = Spunta Totale per una barra di totale.
ui-chart-colors = Colori delle serie
ui-chart-color-of = Colore di { $name }
ui-chart-color-auto = Automatico
ui-chart-color-pick = Colore { $n }
ui-chart-column-name = Nome della colonna { $col }
ui-chart-remove-column = Elimina la colonna
ui-chart-cell = Riga { $row }, colonna { $col }
ui-chart-remove-row = Elimina la riga
ui-chart-add-row = + Riga
ui-chart-add-column = + Colonna
ui-chart-series = Serie { $n }

## File

ui-file-open-prompt = Percorso del file da aprire (.oku-slides.pdf o .json)
ui-file-save-prompt = Salva con nome (percorso completo)

## Schede

ui-tabs = Presentazioni aperte
ui-tab-new = Nuova scheda
ui-tab-close = Chiudi scheda
ui-tab-unsaved = Modifiche non salvate
ui-tab-templates = Modelli
ui-tab-move-new-window = Sposta in una nuova finestra
ui-tab-move-to = Sposta nella finestra { $n }
ui-tab-close-others = Chiudi le altre schede
ui-close-unsaved = Salvare le modifiche a «{ $name }» prima di chiudere?
ui-close-save = Salva
ui-close-discard = Non salvare
ui-close-cancel = Annulla
ui-disk-replaced-question = { $file } è stato sostituito sul disco da un'altra presentazione.
ui-disk-replaced-overwrite = Sovrascrivi
ui-disk-replaced-save-as = Salva con nome…
ui-same-deck-question = { $file } è una versione della presentazione aperta nella scheda «{ $title }».
ui-same-deck-merge = Unisci in quella scheda
ui-same-deck-apart = Apri a parte
