# Interfaccia dell’app (React). Caricata da ui/src/i18n.ts tramite la richiesta «messages».
ui-loading = Caricamento…

## Documento

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · pacchetto di layout
   *[other] { $name } · pacchetto di widget
}
ui-title-rename = Rinomina la presentazione
ui-title-field = Titolo della presentazione
ui-status-unsaved = modifiche non salvate
ui-status-saved = salvato
ui-status-never-saved = non ancora salvato
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count } layout
       *[other] { $count } layout
    }
   *[other] { $count ->
        [one] { $count } widget
       *[other] { $count } widget
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } diapositiva
   *[other] { $count } diapositive
} · { $status }
ui-confirm-quit-pack = Uscire senza salvare il pacchetto modificato?
ui-confirm-update-opener = Aggiornare «{ $deck }» a questa versione del pacchetto «{ $pack }»?
ui-close-writes-pending = Questa scheda sta ancora salvando: chiudila di nuovo tra un momento.
ui-confirm-close-writes-pending = Questa scheda sta ancora salvando. Chiuderla comunque e perdere ciò che non è stato salvato?
ui-confirm-quit-read-only-pack = { $pack } non può essere salvato qui. Chiudere questa scheda e perdere le sue modifiche? Per conservarle, chiedi all'agente di salvarlo come nuovo pacchetto.

## Notifiche

ui-toast-copied = Copia salvata
ui-toast-field-gone = Il campo che stavi modificando non esiste più: il testo inserito è perso.
ui-toast-edit-replaced = Questo testo è cambiato mentre lo modificavi: la tua versione l'ha sostituito. Annulla ripristina l'altra.

## Barra degli strumenti

ui-toolbar-undo = Annulla
ui-toolbar-undo-label = Annulla: { $label }
ui-toolbar-redo = Ripeti
ui-toolbar-redo-label = Ripeti: { $label }
ui-divider-pile = Larghezza della pila di slide
ui-divider-notes = Larghezza del filo dell'agente
ui-divider-code = Dimensione del codice
ui-toolbar-theme = Tema
ui-toolbar-theme-embedded = Tema del file
ui-toolbar-finish = Fine
ui-pack-deck-only = Salvato solo in questa presentazione
ui-toolbar-open = Apri
ui-menu = Menu
ui-toolbar-add-label = Aggiungi alla diapositiva
ui-toolbar-packs = Pacchetti della presentazione
ui-toolbar-packs-update = Pacchetti della presentazione — aggiornamenti disponibili
ui-toolbar-thumbs-show = Mostra miniature
ui-toolbar-thumbs-hide = Nascondi miniature
ui-toolbar-remove-slide = Elimina la diapositiva
ui-toolbar-remove-hint = Diapositiva { $n } · annulla con ⌘Z

## Etichette di annullamento

ui-undo-add-widget = Aggiungi «{ $name }»
ui-undo-new-slide = Nuova diapositiva: { $name }
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
ui-undo-rename-pack = Rinomina il pacchetto
ui-undo-rename-deck = Rinomina la presentazione

## Pila delle diapositive

ui-pile-slides = Diapositive
ui-pile-layouts = Layout
ui-pile-add-slide = Nuova diapositiva
ui-pile-drag-hint = Trascina per spostare · ⌥↑ ⌥↓
ui-slide-alt = Diapositiva { $n }

## Palco

ui-stage-rendering = Rendering…
ui-stage-widget-pack = Apri un widget dalla pila o creane uno.
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
    [chart] Grafico
    [table] Tabella
    [stack] Pila
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
ui-draft = Bozza
ui-draft-send = Invia all'agente
ui-draft-send-title = Invia all'agente (⌘↩)
ui-draft-discard = Scarta
ui-pasted-image = { $type } { $width }×{ $height }
ui-draft-images = Immagini incollate
ui-draft-remove-image = Rimuovi questa immagine
ui-draft-send-needs-text = Dica cosa farne
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
ui-dictation-language-auto = Rilevamento automatico
ui-dictation-model-unavailable = Lingua non supportata
ui-dictation-language-auto-resolved = Automatico ({ $language })
ui-dictation-to-download-language = Lingua da scaricare
ui-dictation-api-key-missing = { $var } non è impostata
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Selettore di modelli

ui-picker-new-slide = Nuova diapositiva
ui-picker-set-mine = { $name } · miei
ui-picker-set-received = { $name } · ricevuti
ui-picker-keys = ← → per sfogliare · Invio per aggiungere · Esc per chiudere
ui-picker-close = Chiudi
ui-picker-list = Layout
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
ui-show-as = Mostra come
ui-show-as-table = Mostra come tabella
ui-show-as-chart = Mostra come grafico

## File

ui-file-open-prompt = Percorso del file da aprire (.oku-slides.pdf, .json o .pptx)
ui-file-filter-presentations = Presentazioni
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = Salva con nome (percorso completo)

## Schede

ui-tabs = Presentazioni aperte
ui-tab-new = Nuova scheda
ui-tab-close = Chiudi scheda
ui-tab-unsaved = Modifiche non salvate
ui-tab-pack = Pacchetto
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
ui-pack-tabs = Schede del pacchetto
ui-pack-layouts-tab = Layout
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
ui-import-hifi-note = Mantiene l'aspetto dell'originale, con un tema e layout creati per questa presentazione.
ui-import-retrofit = Usa i miei layout
ui-import-retrofit-note = Ricostruisce le diapositive con pacchetti di layout e un tema della libreria.
ui-import-layout-packs = Pacchetti di layout
ui-import-theme-default = Tema predefinito
ui-import-no-packs = Nessun pacchetto di layout nella libreria.
ui-import-tune = Rifinisci con l'agente
ui-import-tune-note = L'agente confronta ogni diapositiva con l'originale e corregge le differenze.
ui-import-tune-note-retrofit = L'agente confronta ogni diapositiva con l'originale e progetta widget per le forme che i tuoi layout non mostrano.
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
ui-update-check = Verifica aggiornamenti…

ui-shortcut-theme = tema, temi, theme, themes
