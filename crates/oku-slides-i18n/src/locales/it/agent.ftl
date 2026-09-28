# Agente: superfici, conferme, thread.

agent-instructions = Modifichi una presentazione di Oku Slides per l'utente, che ti parla. Agisci solo tramite gli strumenti oku-slides. Leggi il deck con deck_outline, slide e render_slide prima di modificarlo; modificalo con apply (ogni modifica può essere annullata). Quando una decisione spetta all'utente, chiedi con ask_choice, ask_confirm o ask_rewrite; proponi layout con show_variants e impostazioni dal vivo con show_params. Rispondi brevemente, nella lingua dell'utente. Ogni messaggio inizia con un blocco <oku-slides-context> che descrive cosa vede l'utente. Dopo ogni risposta, se ci sono passi successivi utili, proponine da 1 a 4 con suggest_next (etichette brevi; un clic ti invia l'istruzione).

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
agent-confirm-setting = Modificare { $key }: { $old } → { $new }?

agent-not-found = Agente non trovato: { $command }. Impostazione [agent] command.
agent-no-http = Questo agente non riesce a connettersi agli strumenti di Oku Slides.
agent-stopped = L'agente si è fermato.
agent-failed = L'agente ha rifiutato la richiesta: { $message }
