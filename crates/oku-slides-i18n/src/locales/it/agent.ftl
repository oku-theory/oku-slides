# Agente: superfici, conferme, thread.

agent-surface-cancel = Annulla
agent-surface-confirm = Conferma
agent-surface-confirm-danger = Sì, fallo
agent-surface-rewrite = Riformulazione
agent-surface-pick = Scegli { $n }
agent-surface-keep = Mantieni l'originale
agent-surface-edit = Modifica prima di applicare
agent-surface-close = Chiudi
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
agent-confirm-save-templates = Salvare il set di modelli «{ $name }»?
agent-confirm-delete-set = Eliminare il set di modelli «{ $name }»? Non si può annullare.
agent-confirm-edit-theme = Salvare una nuova versione del tema «{ $name }»?
agent-confirm-copy-theme = Salvare una copia modificata del tema integrato «{ $name }»?
agent-confirm-setting = Modificare { $key }: { $old } → { $new }?

agent-not-found = Agente non trovato: { $command }. Impostazione [agent.presets].
agent-spawn-failed = Impossibile avviare l'agente: { $command }. Impostazione [agent.presets].
agent-no-http = Questo agente non riesce a connettersi agli strumenti di Oku Slides.
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
agent-error-pack-id = Questo non è l'id di un set di modelli: chiama templates per elencare i set e i loro id.
agent-error-denied = L'utente ha rifiutato.
agent-error-no-turn = Nessun turno in corso: { $tool } viene mostrato solo durante un turno.
agent-error-turn-over = Il turno di questa chiamata è terminato: non è stato mostrato nulla.
agent-error-anchor-scope = Una domanda in una conversazione di widget non può essere ancorata a una slide: ponila senza ancora.
agent-error-no-file = La presentazione non ha ancora un file: indica un percorso assoluto.
agent-error-path-relative = Il percorso deve essere assoluto.
agent-error-path-parent = Il percorso non deve contenere «..».
agent-error-setting-locked = L'agente non può modificare l'impostazione { $key }.
agent-error-setting = Impossibile modificare l'impostazione { $key }: controllane il valore.
agent-error-rewrite-text = La riformulazione è tornata senza testo.
agent-error-paragraphs = I paragrafi della riformulazione non sono validi.
agent-error-draft-too-long = La bozza è troppo lunga.
agent-error-draft-origin = Bozza rifiutata: vista sconosciuta.
agent-error-draft-no-agent = L'agente di questa scheda non è ancora avviato: bozza rifiutata.
agent-error-unknown-agent = Questo agente non è più nella configurazione.
agent-error-agent-locked = La scelta dell'agente è imposta dall'ambiente.
agent-error-agent-not-saved = Non è stato possibile salvare la scelta dell'agente.
agent-picker-title = Agente
agent-picker-busy = Cambiare agente interromperà il turno in corso.
agent-picker-locked = Scelta imposta dall'ambiente (OKU_SLIDES_AGENT__SELECTED).
agent-tune-prompt = Rifinire la presentazione importata
