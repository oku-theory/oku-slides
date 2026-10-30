# Agente: superfici, conferme, thread.

agent-surface-cancel = Annulla
agent-surface-confirm = Conferma
agent-surface-confirm-danger = Sì, fallo
agent-surface-rewrite = Riformulazione
agent-surface-pick = Scegli { $n }
agent-surface-keep = Mantieni l'originale
agent-surface-edit = Modifica prima di applicare
agent-surface-close = Chiudi
agent-surface-themes = Temi
agent-surface-variants = Varianti
agent-surface-permission-always = per sempre
agent-surface-permission-title = L'agente chiede un'autorizzazione
agent-undo-turn = Agente: { $text }

agent-tools-unavailable = Non è stato possibile avviare gli strumenti dell'agente.

agent-undo-apply = Modifica dell'agente
agent-undo-variant = Variante: { $label }
agent-undo-param = Impostazione: { $label }
agent-undo-rewrite = Riformulazione

agent-confirm-save = Salvare la presentazione come { $path }?
agent-confirm-save-over = Sostituire { $path }? Il file esistente verrà sovrascritto.
agent-confirm-open = Aprire { $path }?
agent-open-elsewhere = Questo file è già aperto in un’altra scheda.
agent-confirm-new = Iniziare una nuova presentazione?
agent-confirm-new-dirty = Iniziare una nuova presentazione? Le modifiche non salvate andranno perse.
agent-confirm-copy-pack = Salvare una copia del pacchetto «{ $name }»? Il pacchetto stesso resta invariato.
agent-confirm-copy-edited-theme = Salvare una copia del tema «{ $name }»? Il tema stesso resta invariato.
agent-confirm-delete-pack = Eliminare il pacchetto «{ $name }»? Non si può annullare.
agent-confirm-edit-theme = Salvare una nuova versione del tema «{ $name }»?
agent-confirm-copy-theme = Creare una variante del tema integrato «{ $name }»? Il tema integrato resta invariato.
agent-confirm-setting = Modificare { $key }: { $old } → { $new }?
agent-confirm-move = Spostare la presentazione in { $path }? Il vecchio file verrà eliminato.
agent-confirm-merge = Unire { $path } a questa presentazione? Non si può annullare.
agent-confirm-save-copy = Salvare una copia della presentazione come { $path }?
agent-confirm-dictation-api = Inviare ciò che detti a { $host } ({ $url }), modello { $model }, con il segreto di { $var }?
agent-confirm-dictation-download = Usare { $name } e scaricarlo ora ({ $size })?
agent-confirm-dictation-language = Usare la dettatura di Apple e scaricarne subito la lingua?
agent-confirm-agent = Passare all'agente { $name } ({ $command }) dell'app quando non è in corso nessuna risposta?
agent-confirm-agent-builtin = Passare all'agente { $name } dell'app quando non è in corso nessuna risposta?
agent-confirm-agent-model = Scaricare il modello dell'agente locale ({ $size })?

agent-not-found = Agente non trovato: { $command }. Impostazione [agent.presets].
agent-spawn-failed = Impossibile avviare l'agente: { $command }. Impostazione [agent.presets].
agent-no-http = Questo agente non riesce a connettersi agli strumenti di Oku Slides.
agent-local-no-program = L'agente locale funziona solo all'interno dell'app Oku Slides.
agent-local-needs-download = L'agente locale deve prima scaricare il suo modello ({ $gigabytes } GB).
agent-local-intel-mac = L'agente locale richiede un Mac con chip Apple.
agent-local-unsupported-os = L'agente locale non è ancora disponibile su questo sistema.
agent-model-download = Scarica il modello
agent-model-downloading = Download del modello… { $percent }%
agent-model-cancel = Annulla
agent-model-download-failed = Impossibile scaricare il modello.
agent-model-download-integrity = Il modello scaricato era danneggiato ed è stato eliminato.
agent-model-download-incomplete = Download incompleto: { $got } byte su { $expected }.
agent-model-download-cancelled = Download annullato.
local-agent-no-route = L'agente locale non ha capito cosa fare. Prova a dirlo in un altro modo.
local-agent-tool-limit = L'agente locale si è fermato dopo { $n } passaggi senza finire.
local-agent-invalid-call = L'agente locale non è riuscito a usare correttamente gli strumenti.
local-agent-engine-failed = Il modello dell'agente locale non ha funzionato.
local-agent-retry = Riprova, oppure scegli un altro agente nel pannello dell'agente.
agent-stopped = L'agente si è fermato.
agent-failed = L'agente ha rifiutato la richiesta: { $message }
agent-tool-failed = Lo strumento ha avuto un errore imprevisto.

agent-turn-tools = { $tools ->
   [one] { $tools } strumento
  *[other] { $tools } strumenti
}
agent-turn-notes = { $notes ->
   [one] { $notes } nota
  *[other] { $notes } note
}
agent-turn-secs = { $secs } s
agent-turn-stopped = Fermato.
agent-turn-interrupted = Interrotto.
agent-memory-lost = L'agente non ricorda più questa conversazione: ricomincia da capo.
agent-turn-max-tokens = L'agente ha raggiunto il limite di lunghezza.
agent-turn-max-turns = L'agente ha raggiunto il limite di passaggi.
agent-turn-refusal = L'agente ha rifiutato.
agent-turn-ended = Turno concluso ({ $stop }).
agent-choice-cancelled = Domanda annullata
agent-choice-answered = ✓ { $answer }
agent-report-why = Perché

agent-error-file = Impossibile leggere o scrivere il file.
agent-error-unexpected-action = Azione imprevista: { $action }.
agent-error-unknown-surface = Questa scheda non è più aperta.
agent-error-expired = Le azioni di un turno passato sono scadute.
agent-error-incomplete = L'azione di questa scheda è incompleta.
agent-error-no-field = Questa impostazione non esiste più.
agent-error-no-variant = Questa variante non esiste più.
agent-error-arguments = Gli argomenti dello strumento non sono validi ({ $detail }): controllali in base al suo schema.
agent-error-unknown-tool = Strumento sconosciuto: { $name }.
agent-error-pack-id = Questo non è l'id di un pacchetto: chiama packs per elencare i pacchetti e i loro id.
agent-error-asset-unknown = Questa immagine non è né nella presentazione né tra le immagini incollate: usa l'identificatore di un'immagine incollata.
agent-error-denied = L'utente ha rifiutato.
agent-error-no-turn = Nessun turno in corso: { $tool } viene mostrato solo durante un turno.
agent-error-tab-no-turn = Apri una scheda di pacchetto o di tema, o mostrala, solo mentre rispondi all'utente.
agent-error-tab-limit = In questo turno hai già aperto o mostrato una scheda di pacchetto o di tema; termina lì, oppure chiedi all'utente prima di usarne un'altra.
agent-error-continue-in-other-tab = Questa richiesta prosegue nella scheda che hai aperto o mostrato: chiedi all'utente di continuare lì; per ora questa scheda non modifica alcun widget.
agent-error-pack-open-in-tab = Questo pacchetto è in modifica in un'altra scheda: chiedi all'utente di terminare prima lì.
agent-pack-opened-in-tab = { $shown ->
   [yes] Il pacchetto si è aperto nella scheda { $tab }, ora mostrata.
  *[no] Il pacchetto si è aperto nella scheda { $tab }, in secondo piano: la finestra dell'utente mostra un'altra scheda.
} Di' all'utente di continuare lì: l'agente di quella scheda non conosce questa richiesta. Questa scheda resta com'è.
agent-theme-opened-in-tab = { $shown ->
   [yes] Il tema si è aperto nella scheda { $tab }, ora mostrata.
  *[no] Il tema si è aperto nella scheda { $tab }, in secondo piano: la finestra dell'utente mostra un'altra scheda.
} Di' all'utente di continuare lì: l'agente di quella scheda non conosce questa richiesta. Questa scheda resta com'è.
agent-error-turn-over = Il turno di questa chiamata è terminato: non è stato mostrato nulla.
agent-error-no-file = La presentazione non ha ancora un file: indica un percorso assoluto.
agent-error-path-relative = Il percorso deve essere assoluto.
agent-error-path-parent = Il percorso non deve contenere «..».
agent-error-setting-locked = L'agente non può modificare l'impostazione { $key }.
agent-error-setting = Impossibile modificare l'impostazione { $key }: controllane il valore.
agent-error-history-index = Per scorrere la cronologia serve l'indice di una versione.
agent-error-no-pack = Questa presentazione non fissa alcun pacchetto: indica pack (vedi packs).
agent-error-history-not-opened = La scheda non ha aperto la cronologia in tempo: richiedi.
agent-error-present = Niente da presentare qui: nessuna slide, oppure la scheda è in modalità pacchetto, cronologia o sola lettura.
agent-error-locale = Codice lingua sconosciuto. Indica il codice di una lingua inclusa o system.
agent-error-empty-interface = Niente da cambiare: indica scheme, locale o guides.
agent-error-empty-dictation = Niente da cambiare: indica almeno un'impostazione della dettatura.
agent-error-not-shown = Questa scheda non è quella mostrata nella sua finestra: richiedi quando l'utente la mostra.
agent-error-themes-deck-mode = Il selettore di temi si apre solo durante la modifica delle diapositive.
agent-error-packs-deck-mode = Il pannello dei pacchetti della presentazione si apre solo durante la modifica delle diapositive.
agent-error-rewrite-text = La riformulazione è tornata senza testo.
agent-error-paragraphs = I paragrafi della riformulazione non sono validi.
agent-error-draft-too-long = La bozza è troppo lunga.
agent-error-draft-origin = Bozza rifiutata: vista sconosciuta.
agent-error-draft-no-agent = L'agente di questa scheda non è ancora avviato: bozza rifiutata.
agent-error-too-many-images = La bozza ha già tutte le sue immagini.
agent-error-image-not-base64 = L'immagine incollata non è arrivata intera.
agent-error-image-gone = Un'immagine incollata non è più disponibile: incollala di nuovo.
agent-error-unknown-agent = Questo agente non è più nella configurazione.
agent-error-agent-locked = La scelta dell'agente è imposta dall'ambiente.
agent-error-agent-not-saved = Non è stato possibile salvare la scelta dell'agente.
agent-error-unknown-model = Questo agente non offre quel modello.
agent-error-model-not-saved = Non è stato possibile salvare la scelta del modello.
voice-error-no-dictation = La dettatura non è disponibile qui
voice-error-busy = La dettatura è in corso: riprova quando è a riposo
voice-error-unknown-model = Modello di dettatura sconosciuto
voice-error-unavailable = Questo modello non supporta la lingua scelta
voice-error-installed = Questo modello è già installato
voice-error-not-downloadable = Questo modello non si scarica
voice-error-nothing-to-cancel = Nessun download in corso
voice-error-bad-silence = Il silenzio deve durare da 0,5 a 10 secondi
voice-error-bad-language = Lingua di dettatura sconosciuta
voice-error-bad-url = L'indirizzo dell'API deve iniziare con https:// (http:// solo per questo computer), senza nome utente, parametri né caratteri speciali
voice-error-bad-key-env = La variabile della chiave deve essere un nome come OPENAI_API_KEY: maiuscole, cifre e _
voice-error-bad-api-model = Il nome del modello dell'API ammette solo lettere, cifre e . _ : / -; un nome di distribuzione di Azure solo lettere, cifre e . _ -
voice-error-bad-api-version = La versione dell'API di Azure ammette solo lettere, cifre, . e -, al massimo 32 caratteri, ad esempio 2025-03-01-preview
voice-error-bad-api-headers = Le intestazioni si scrivono Nome: valore; Nome: $VARIABLE, al massimo 8. Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection e i duplicati sono rifiutati
voice-error-api-reserved = Solo l'utente imposta l'indirizzo, la variabile della chiave, la versione di Azure e le intestazioni dell'API: apri il riquadro con show_dictation e chiediglielo
voice-error-changed = Le impostazioni sono cambiate dopo la domanda: chiedi di nuovo
voice-error-nothing-to-download = Niente da scaricare per questo agente
agent-tune-prompt = Rifinire la presentazione importata
agent-settings-unknown-selected = L'impostazione agent.selected = { $selected } non indica alcun agente di [agent.presets] (noti: { $known })
agent-settings-preset-launch = L'agente [agent.presets.{ $id }] richiede esattamente uno tra command e builtin
