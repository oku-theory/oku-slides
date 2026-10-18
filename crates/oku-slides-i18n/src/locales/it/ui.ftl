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

ui-toast-templates-saved = Modelli salvati: { $name }
ui-toast-moved = Presentazione spostata
ui-toast-copied = Copia salvata
ui-toast-field-gone = Il campo che stavi modificando non esiste più: il testo inserito è perso.
ui-toast-edit-replaced = Questo testo è cambiato mentre lo modificavi: la tua versione l'ha sostituito. Annulla ripristina l'altra.

## Barra degli strumenti

ui-toolbar-undo = Annulla
ui-toolbar-undo-label = Annulla: { $label }
ui-toolbar-redo = Ripeti
ui-toolbar-redo-label = Ripeti: { $label }
ui-toolbar-grid-hide = Nascondi la griglia
ui-toolbar-grid-show = Mostra la griglia
ui-toolbar-ghosts-hide = Nascondi le guide
ui-toolbar-ghosts-show = Mostra le guide
ui-divider-pile = Larghezza della pila di slide
ui-divider-notes = Larghezza del filo dell'agente
ui-divider-code = Dimensione del codice
ui-divider-band = Altezza delle impostazioni
ui-toolbar-theme = Tema
ui-toolbar-theme-embedded = Tema del file
ui-toolbar-theme-deck-group = Questa presentazione
ui-toolbar-save-copy = Salva una copia
ui-toolbar-finish = Fine
ui-menu-workshop = Laboratorio
ui-templates-deck-only = Salvati solo in questa presentazione
ui-toolbar-open = Apri
ui-merge = Unisci con un PDF…
ui-menu = Menu
ui-menu-move = Sposta…
ui-language = Lingua
ui-language-system = Lingua di sistema
ui-scheme = Aspetto
ui-scheme-system = Sistema
ui-scheme-light = Chiaro
ui-scheme-dark = Scuro
ui-toolbar-add-label = Aggiungi alla diapositiva
ui-add-kind = { $kind ->
    [card] Scheda
    [shape] Forma
    [chart] Grafico
    [table] Tabella
   *[text] Testo
}
ui-add-kind-description = { $kind ->
    [card] Un titolo e un breve testo in un riquadro
    [shape] Una forma colorata
    [chart] Un grafico ricavato da una tabella di dati
    [table] Una tabella di dati
   *[text] Un paragrafo di testo
}
ui-toolbar-remove-template = Elimina il modello
ui-toolbar-remove-slide = Elimina la diapositiva
ui-toolbar-remove-hint = Diapositiva { $n } · annulla con ⌘Z

## Etichette di annullamento

ui-undo-add = { $kind ->
    [card] Aggiungi una scheda
    [shape] Aggiungi una forma
    [chart] Aggiungi un grafico
    [table] Aggiungi una tabella
   *[text] Aggiungi un testo
}
ui-undo-add-widget = Aggiungi «{ $name }»
ui-undo-new-template = Nuovo modello: { $name }
ui-undo-new-slide = Nuova diapositiva: { $name }
ui-undo-templates = Modelli: { $name }
ui-undo-move-template = Sposta il modello
ui-undo-move-slide = Sposta la diapositiva
ui-undo-remove-slide = Elimina la diapositiva
ui-undo-remove = Elimina
ui-undo-paste = Incolla
ui-undo-place-on-grid = Posiziona sulla griglia
ui-undo-move-on-grid = Sposta sulla griglia
ui-undo-resize-on-grid = Ridimensiona sulla griglia
ui-undo-change-anchor = Cambia l’ancoraggio
ui-undo-move = Sposta
ui-undo-resize = Ridimensiona
ui-undo-edit-slide = Modifica la diapositiva
ui-undo-rename-set = Rinomina il set
ui-undo-rename-deck = Rinomina la presentazione
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
ui-stage-fields = Campi della diapositiva
ui-stage-edit-text = Modifica il testo
ui-stage-hint-plain = Invio o Tab per passare al successivo · Esc per annullare
ui-stage-hint-rich = Tab per passare al successivo · ⌘ Invio per confermare · «- » per un punto elenco · ⌘B grassetto, ⌘I corsivo, ⌘K link
ui-field = { $kind ->
    [title] Titolo
    [subtitle] Sottotitolo
    [kicker] Occhiello
    [text] Testo
    [body] Corpo
    [author] Autore
    [date] Data
    [organization] Organizzazione
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
ui-ghost-zone = Zona

## Note della diapositiva

ui-notes = Strumenti della diapositiva
ui-stepper-less = { $label }: uno in meno
ui-stepper-more = { $label }: uno in più
ui-list-item = elemento
ui-list-add = Aggiungi { $item }
ui-list-remove = Rimuovi { $item }
ui-list-move = Sposta { $item }
ui-list-full = L'elenco è pieno ({ $max })
ui-undo-list-move = Sposta { $item }
ui-agent-color-other = Altro colore
ui-agent-thread = Agente
ui-agent-new-conversation = Nuova conversazione
ui-agent-undo-turn = Annulla questo turno
ui-agent-detach = Apri l'agente in una finestra separata
ui-agent-attach = Riporta l'agente nella finestra principale
ui-agent-window-empty = Nessun agente per questa scheda
ui-agent-kept-in-draft = Nessun posto dove scrivere: testo conservato nella bozza
ui-agent-busy-elsewhere = L'agente sta rispondendo in { $place }…
ui-agent-scope-slides = le slide
ui-agent-scope-atelier = l'officina
ui-draft = Bozza
ui-draft-send = Invia all'agente
ui-draft-send-title = Invia all'agente (⌘↩)
ui-draft-discard = Scarta
ui-agent-state-idle = Agente
ui-agent-state-starting = L'agente si avvia…
ui-agent-state-working = L'agente sta lavorando…
ui-agent-dock = In corso
ui-agent-dock-settings = Impostazioni
ui-agent-jump = ↓ Turno corrente
ui-agent-tool-input = Argomenti
ui-agent-tool-output = Risultato
ui-agent-sheet = Domanda dell'agente
ui-agent-sheet-keys = 1–4 scegli · Invio conferma · E ritocca · Esc annulla
ui-agent-variants = Varianti
ui-agent-drawer-keys = Clic o 1–4 · Esc chiude
ui-agent-confirm = Conferma dell'agente

## Dettatura

ui-dictation = Dettatura
ui-dictation-start = Detta
ui-dictation-start-title = Detta (⌘D)
ui-dictation-type = Scrivi all'agente
ui-dictation-type-title = Scrivi all'agente (⌘K)
ui-dictation-agent-stop = Ferma l'agente (⌘⇧D o Esc)
ui-dictation-agent-working = L'agente sta lavorando…
ui-dictation-agent-working-hint = ⌘⇧D o Esc lo ferma
ui-dictation-finish = Termina la dettatura
ui-dictation-finish-title = Termina (⌘D)
ui-dictation-cancel = Annulla
ui-dictation-preparing = Preparazione della dettatura…
ui-dictation-heard-nothing = Non si è sentito nulla: parla un po’ più vicino al microfono
ui-dictation-to-cursor = al cursore
ui-dictation-to-draft = nella bozza
ui-dictation-downloading = Download del modello { $name }
ui-dictation-download-progress = { $percent }% · { $received } di { $total } · una sola volta
ui-dictation-listening = In ascolto · { $time }
ui-dictation-transcribing = Trascrizione dell’ultima frase…
ui-dictation-stop-in = Arresto tra { $duration }
ui-dictation-speak = Parla, il testo compare qui…
ui-dictation-auto-stop = si ferma dopo { $duration } di silenzio
ui-dictation-to-download = { $size } da scaricare
ui-dictation-settings = Impostazioni
ui-dictation-model-title = Modello di riconoscimento
ui-dictation-model-intro = Whisper, su questo computer (CTranslate2). Ogni modello comprende più lingue.
ui-dictation-note-turbo = Consigliato, veloce e preciso
ui-dictation-note-large = Il più preciso, più lento
ui-dictation-note-base = Leggero, per computer modesti
ui-dictation-installed = installato
ui-dictation-not-installed = da scaricare
ui-dictation-language = Lingua parlata
ui-dictation-language-auto = Rilevamento automatico
ui-dictation-note-apple = Il più veloce, integrato nel sistema
ui-dictation-model-unavailable = Lingua non supportata
ui-dictation-model-intro-apple = Apple, integrato nel sistema, o Whisper, su questo computer (CTranslate2). Tutti comprendono più lingue.
ui-dictation-language-auto-resolved = Automatico ({ $language })
ui-dictation-to-download-language = Lingua da scaricare
ui-dictation-auto-stop-label = Fermati da solo dopo un silenzio di
ui-dictation-silence = Durata del silenzio
ui-dictation-phrases-note = Vengono trascritti solo i passaggi parlati, frase per frase, mentre parli.
ui-dictation-download-now = Scarica ora ({ $size })
ui-dictation-download-language = Scarica la lingua
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Selettore di modelli

ui-picker-new-slide = Nuova diapositiva
ui-picker-new-template = Nuovo modello, basato su…
ui-picker-set = Modelli
ui-picker-set-mine = { $name } · miei
ui-picker-set-received = { $name } · ricevuti
ui-picker-keys = ← → per sfogliare · Invio per aggiungere · Esc per chiudere
ui-picker-close = Chiudi
ui-picker-previous = Modello precedente
ui-picker-next = Modello successivo
ui-picker-list = Modelli di diapositiva
ui-widget-picker-filter = Filtra per nome, etichetta o descrizione
ui-widget-picker-empty = Nessun widget corrisponde a «{ $query }».
ui-widget-picker-list = Widget
ui-widget-picker-keys = Frecce per scorrere · Invio per aggiungere · Esc per chiudere

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
ui-chart-add-row = Riga
ui-chart-add-column = Colonna
ui-chart-series = Serie { $n }

## Tabella

ui-table-panel = Tabella
ui-table-options = Opzioni della tabella
ui-table-header-row = Riga di intestazione
ui-table-header-column = Prima colonna come intestazione
ui-table-striped = Righe alternate
ui-table-alignment = Allineamento delle colonne
ui-table-align-of = Allineamento di { $name }
ui-table-align = { $align ->
    [start] Inizio
    [center] Centro
    [end] Fine
   *[auto] Automatico
}
ui-table-column = Colonna { $n }
ui-table-no-data = Questa tabella non ha dati.
ui-undo-table-option = Opzioni della tabella
ui-undo-table-align = Allinea la colonna
ui-undo-table-data = Modifica la tabella
ui-show-as = Mostra come
ui-show-as-table = Mostra come tabella
ui-show-as-chart = Mostra come grafico
ui-undo-show-as-table = Mostra come tabella
ui-undo-show-as-chart = Mostra come grafico

## File

ui-file-open-prompt = Percorso del file da aprire (.oku-slides.pdf, .json o .pptx)
ui-file-merge-prompt = Percorso della presentazione da unire a questa (.oku-slides.pdf o .json)
ui-file-filter-presentations = Presentazioni
ui-file-filter-pptx = PowerPoint
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
ui-close-cancel = Annulla
ui-disk-replaced-question = { $file } è stato sostituito sul disco da un'altra presentazione.
ui-disk-replaced-overwrite = Sovrascrivi
ui-disk-replaced-save-as = Salva con nome…
ui-same-deck-question = { $file } è una versione della presentazione aperta nella scheda «{ $title }».
ui-same-deck-merge = Unisci in quella scheda
ui-same-deck-apart = Apri a parte
ui-history = Cronologia
ui-history-version = Versione del { $date }
ui-history-restore = Ripristina
ui-history-back = Indietro
ui-history-anonymous = Anonimo
ui-history-slider = Versioni della presentazione
ui-history-removed-since = eliminata da allora
ui-history-changed-since = modificata da allora
ui-history-added-since = diapositive aggiunte da allora: { $count }
ui-help = Guida
ui-help-caption = Diapositiva { $index } di { $count } · sola lettura
ui-slide-previous = Diapositiva precedente
ui-slide-next = Diapositiva successiva
ui-present = Presentazione
ui-present-slide = Slide { $index } di { $count }
ui-present-screen = Presenta a schermo intero
ui-present-window = Presenta nella finestra
ui-toolbar-params-hide = Nascondi le impostazioni
ui-toolbar-params-show = Mostra le impostazioni
ui-help-close = Chiudi la guida
ui-help-search = Cerca nella guida
ui-help-search-placeholder = Cerca…
ui-help-search-empty = Nessun risultato
ui-help-search-count = { $count ->
    [one] 1 risultato
   *[other] { $count } risultati
}
ui-widgets-files = File dei widget
ui-widgets-common = Comune
ui-widgets-new = Nuovo widget
ui-widgets-name = Nome del widget (minuscole, cifre e trattini)
ui-widgets-name-taken = Questo nome è già in uso
ui-widgets-name-letter = Il nome deve iniziare con una lettera
ui-widgets-valid = Valido
ui-widgets-errors = Errori
ui-widgets-empty = Questo pacchetto non ha ancora widget.
ui-widgets-preview = Anteprima del widget
ui-widgets-no-preview = Nessuna anteprima
ui-widgets-field-bound = Collegato alla presentazione: qui in sola lettura
ui-widgets-source = Codice sorgente
ui-widgets-unsaved = Non salvato
ui-widgets-in-error = Widget con errori
ui-widgets-none-in-error = Nessun widget ha errori
ui-widgets-section = Widget
ui-widgets-open = Apri il widget { $name }
ui-widgets-delete-named = Elimina { $name }
ui-widgets-confirm-delete = Eliminare il widget «{ $name }»?
ui-widgets-rename-hint = Doppio clic o F2 per rinominare
ui-widgets-tabs = Schede dei modelli
ui-widgets-templates-tab = Modelli
ui-theme-tab = Tema
ui-theme-picker = Tema
ui-theme-name = Nome del tema
ui-theme-unsaved = Modifiche al tema non salvate
ui-widgets-close-tab = Chiudi { $name }
ui-widgets-removed-by-agent = L'agente ha eliminato il widget { $name }.

## Importa

ui-import-title = Importa una presentazione PowerPoint
ui-import-mode = Modalità di importazione
ui-import-hifi = Copia fedele
ui-import-hifi-note = Mantiene l'aspetto dell'originale, con un tema e modelli creati per questa presentazione.
ui-import-retrofit = Usa i miei modelli
ui-import-retrofit-note = Ricostruisce le diapositive con modelli e un tema della libreria.
ui-import-theme-default = Tema predefinito
ui-import-no-packs = Nessun set di modelli nella libreria.
ui-import-tune = Rifinisci con l'agente
ui-import-tune-note = L'agente confronta ogni diapositiva con l'originale e corregge le differenze.
ui-import-tune-note-retrofit = L'agente confronta ogni diapositiva con l'originale e progetta widget per le forme che i tuoi modelli non mostrano.
ui-import-tune-unavailable = Nessun agente utilizzabile: { $reason }
ui-import-submit = Importa
ui-import-busy = Importazione in corso…
ui-update-available = Oku Slides { $version } è disponibile.
ui-update-install = Installa e riavvia
ui-update-see-release = Vedi la versione
ui-update-later = Più tardi
ui-update-installing = Download dell'aggiornamento… { $percent }%
ui-update-downloading = Download dell'aggiornamento…
ui-update-up-to-date = Oku Slides è aggiornato.
ui-menu-check-updates = Verifica aggiornamenti…
