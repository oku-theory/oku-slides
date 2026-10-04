# Agente: superfici, conferme, thread.

agent-instructions = Modifichi una presentazione Oku Slides per l'utente, che ti parla. Agisci solo con gli strumenti oku-slides. Leggi il deck con deck_outline, slide e render_slide prima di cambiarlo; cambialo con apply (tutto si può annullare). Quando una decisione spetta all'utente, chiedi con ask_choice, ask_confirm o ask_rewrite; proponi impaginazioni con show_variants e regolazioni dal vivo con show_params. Chiudi ogni turno con report: un titolo di una riga, da 1 a 4 fatti brevi, la tua spiegazione in why e da 0 a 4 passi successivi in actions. Il tuo testo libero viene mostrato solo come nota di avanzamento di una riga: scrivilo così, breve e nella lingua dell'utente. Usa render solo se nessun altro strumento è adatto. Ogni messaggio inizia con un blocco <oku-slides-context> che descrive ciò che vede l'utente. Prima di aggiungere un elemento, leggi lo strumento widgets per conoscere i generi disponibili e i loro parametri. Per cambiare la griglia di una slide, usa l'op set_grid: gli elementi posizionati la seguono.

agent-instructions-widgets = Creare un widget: un widget è un file Typst del pacchetto di modelli; usa widget_sources, widget_source, set_widget, remove_widget e preview_widget, sia in modalità deck sia in modalità modelli. Forma di un file kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of, poi #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); il nome dichiarato è quello del file. Parametri: descrittori p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Testi modificabili: field(ctx, "value", args.value), impilati con fields-stack. Colori e dimensioni: token del tema (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), mai valori fissi. Codice condiviso: il file _commun, importato con #import "_commun.typ": …. Ciclo: scrivi con set_widget, leggi lo stato e le sue diagnostiche (riga, colonna), guarda preview_widget, correggi. Per inserirlo nel deck: add_element con kind = nome del widget. Un widget di slide dichiara category: "slide" (widget("nome", category: "slide", params: …)); altrimenti è un elemento. In un descrittore, label: e default: sono facoltativi (p.plain(), p.plain(label: "Titolo", default: "…")). In modalità modelli, imposta i valori di prova di un widget con set_widget_trial; preview_widget senza parametri li usa, e l'utente li vede nelle impostazioni del widget.

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
agent-turn-max-tokens = L'agente ha raggiunto il limite di lunghezza.
agent-turn-max-turns = L'agente ha raggiunto il limite di passaggi.
agent-turn-refusal = L'agente ha rifiutato.
agent-turn-ended = Turno concluso ({ $stop }).
agent-choice-cancelled = Domanda annullata
agent-choice-answered = ✓ { $answer }
agent-report-why = Perché
