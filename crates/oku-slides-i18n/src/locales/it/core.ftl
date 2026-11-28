op-slide-not-found = Diapositiva non trovata: { $subject }
op-element-not-found = Elemento non trovato: { $subject }
op-duplicate-id = Identificatore già in uso: { $subject }
op-index-out-of-range = Posizione fuori intervallo
op-invalid-value = Valore non valido per { $subject }
op-invalid-value-because = Valore non valido per { $subject }: { $reason }
op-value-no-value = il parametro non ha alcun valore
op-value-grid-tracks = una griglia ha da 1 a 24 colonne e righe
op-value-bad-item-id = un identificatore di elemento è un testo non vuoto, senza «/»
op-value-missing-item-id = ogni elemento deve avere un identificatore
op-value-too-deep = gli elementi sono annidati troppo in profondità
op-value-empty-path = il percorso è vuoto
op-value-not-text = è atteso un testo o null
op-value-empty-frame = larghezza e altezza devono essere maggiori di 0
op-value-blocks-off-grid = gli slot a flusso sono disposti dalla diapositiva e una griglia li nasconde; sposta prima ogni elemento in uno slot libero
op-value-outside-grid = la cella è fuori dalla griglia
op-value-not-tokens = è atteso un oggetto o null
op-value-not-a-slot = questo parametro non è uno slot
op-value-out-of-bounds = il numero è fuori dall'intervallo consentito
op-value-unreadable = il valore non ha la forma attesa
op-value-valign = è atteso top, middle o bottom
op-value-path = il tracciato è composto da comandi M, L, C, Q e Z seguiti dai loro numeri, a partire da M
knowledge-path-empty = il percorso è vuoto
knowledge-path-not-absolute = il percorso deve iniziare con /
knowledge-path-bad-segment = un segmento è vuoto, termina con un punto o usa un carattere diverso da a-z, 0-9, punto, trattino basso e trattino
knowledge-path-dot-segment = un segmento non può iniziare con un punto
knowledge-path-reserved = un segmento è un nome riservato del sistema
knowledge-path-too-deep = il percorso ha troppi segmenti
knowledge-path-too-long = il percorso è troppo lungo
knowledge-path-not-markdown = il nome del file deve terminare con .md
knowledge-frontmatter-missing = il file deve iniziare con un blocco di intestazione delimitato da ---
knowledge-frontmatter-unterminated = il blocco di intestazione non ha il --- di chiusura
knowledge-frontmatter-too-large = il blocco di intestazione è troppo grande
knowledge-frontmatter-not-yaml = l'intestazione non è YAML valido
knowledge-frontmatter-not-mapping = l'intestazione deve essere una mappa con chiavi di testo
knowledge-frontmatter-duplicate-key = la chiave "{ $key }" compare due volte nell'intestazione
knowledge-frontmatter-alias = l'intestazione non può usare ancore o alias YAML
knowledge-frontmatter-tag = l'intestazione non può usare tag YAML
knowledge-frontmatter-too-deep = l'intestazione ha un annidamento troppo profondo
knowledge-frontmatter-after-document-end = l'intestazione contiene testo dopo la fine del documento YAML ...
knowledge-frontmatter-flow-mapping = l'intestazione deve essere un unico mapping a blocchi, una chiave per riga, senza fine `...`
knowledge-frontmatter-type-missing = l'intestazione richiede un tipo non vuoto
knowledge-frontmatter-bad-timestamp = il valore "{ $key }" dell'intestazione deve essere una data e ora RFC 3339 con fuso orario
knowledge-frontmatter-not-allowed = la chiave "{ $key }" non è consentita nell'intestazione di questo file
knowledge-reason-not-found = nessun file in questo percorso
knowledge-reason-exists = esiste già un file in questo percorso
knowledge-reason-id-missing = un nuovo file ha bisogno di un identificatore
knowledge-reason-id-invalid = l'identificatore di un file è di 32 cifre esadecimali minuscole
knowledge-reason-id-taken = un altro file ha già questo identificatore
knowledge-reason-id-mismatch = il file in questo percorso ha un altro identificatore
knowledge-reason-edit-empty-old = il testo da sostituire non può essere vuoto
knowledge-reason-edit-no-match = il testo da sostituire non è nel file
knowledge-reason-edit-ambiguous = il testo da sostituire compare più volte nel file
knowledge-reason-too-many-files = il pacchetto ha troppi file
knowledge-reason-file-too-large = il file è troppo grande
knowledge-reason-total-too-large = il pacchetto è troppo grande
knowledge-reason-over-limit = la base di conoscenza supera i suoi limiti: solo eliminare dei file la riporta sotto
knowledge-no-text-layer = questo PDF non ha un livello di testo (una scansione): esportalo con il testo o incollane il testo
knowledge-legacy-format = questo vecchio formato Office non può essere letto: salvalo come .pptx, .docx o .xlsx
knowledge-document-encrypted = questo documento è protetto da password: salvane una copia non protetta
knowledge-unsupported-document = questo tipo di documento non può essere importato: PDF, DOCX, PPTX, XLSX, ODS, Markdown o testo
knowledge-document-damaged = questo documento è danneggiato e non può essere letto
knowledge-document-type-mismatch = il contenuto di questo file non corrisponde al suo nome
knowledge-document-too-large = questo documento è troppo grande per essere importato
op-knowledge = File di conoscenza { $path }: { $reason }
op-unknown-widget = Widget sconosciuto: { $subject }
op-unknown-namespace = Questa presentazione non ha il pacchetto { $pack }.
op-pack-required = Il pacchetto { $pack } serve a { $by }; rimuovi prima quello.
op-pack-in-use = Il pacchetto { $pack } è ancora usato da { $count ->
    [one] { $count } elemento
   *[other] { $count } elementi
}.
op-pack-requirement-missing = Il pacchetto { $pack } richiede { $required }, che questa presentazione non ha.
op-base-required = Ogni presentazione mantiene il pacchetto di widget di base.
op-pack-kind-conflict = Questa presentazione ha già un pacchetto { $pack } dell'altro tipo (widget o layout).
op-theme-cycle = I temi si estendono a vicenda in un ciclo: { $chain }.
op-theme-closure = I temi fissati in questa presentazione non sono esattamente il suo tema e i suoi genitori ({ $namespace }).
op-unknown-param = Parametro sconosciuto: { $subject }
op-param-type-because = Valore non valido per { $subject }: { $reason }
op-param-shape-text = { $nullable ->
    [yes] serve un testo o null
   *[no] serve un testo
}
op-param-shape-choice = { $nullable ->
    [yes] serve uno dei suoi valori o null
   *[no] serve uno dei suoi valori
}
op-param-shape-paragraphs = { $nullable ->
    [yes] serve un elenco di paragrafi o null
   *[no] serve un elenco di paragrafi
}
op-param-shape-elements = { $nullable ->
    [yes] serve un elenco di elementi o null
   *[no] serve un elenco di elementi
}
op-param-shape-flag = { $nullable ->
    [yes] serve un booleano o null
   *[no] serve un booleano
}
op-param-shape-number = { $nullable ->
    [yes] serve un numero o null
   *[no] serve un numero
}
op-param-shape-bounded-number = { $nullable ->
    [yes] serve un numero entro i suoi limiti o null
   *[no] serve un numero entro i suoi limiti
}
op-param-shape-grid = { $nullable ->
    [yes] serve una griglia o null
   *[no] serve una griglia
}
op-param-shape-chart-option = { $nullable ->
    [yes] serve un'opzione del grafico o null
   *[no] serve un'opzione del grafico
}
op-param-shape-table-option = { $nullable ->
    [yes] serve un'opzione della tabella o null
   *[no] serve un'opzione della tabella
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] serve un'opzione del grafico o della tabella o null
   *[no] serve un'opzione del grafico o della tabella
}
op-param-shape-items = { $nullable ->
    [yes] serve un elenco di elementi o null
   *[no] serve un elenco di elementi
}
op-param-shape-list = { $nullable ->
    [yes] serve un elenco o null
   *[no] serve un elenco
}
op-param-shape-add-element-only = { $nullable ->
    [yes] gli elementi si aggiungono con add_element, o null
   *[no] gli elementi si aggiungono con add_element
}
op-param-shape-set-param-only = { $nullable ->
    [yes] il valore si imposta con set_param, o null
   *[no] il valore si imposta con set_param
}
op-param-shape-path-option = { $nullable ->
    [yes] è atteso un tracciato di forma (un oggetto con d) o null
   *[no] è atteso un tracciato di forma (un oggetto con d)
}
op-item-not-found = Nessuna voce { $subject }
op-list-full = L'elenco { $subject } è pieno (al massimo { $max ->
   [one] { $max } elemento
  *[other] { $max } elementi
})
op-list-minimum = L'elenco { $subject } richiede almeno { $min ->
   [one] { $min } elemento
  *[other] { $min } elementi
}
op-list-cell-too-long = Una cella dell'elenco { $subject } è troppo lunga (colonna { $column })
op-list-id-too-long = Un identificatore di elemento dell'elenco { $subject } è troppo lungo
op-list-too-many-items = L'elenco { $subject } ha troppi elementi (al massimo { $max })
op-slot-rejects = { $subject } non è accettato qui
op-wrong-category = { $subject } non può essere usato qui
op-choice-out-of-range = Valore non consentito per { $subject }
transaction-refused = Operazione { $index } rifiutata: { $reason }

sources-unreadable = Sorgenti illeggibili
deck-inline-data = L’elemento { $element } salva i suoi dati nelle sue opzioni: questo file di deck non è nel formato attuale
sources-not-sources = Questo non è un archivio di sorgenti di Oku Slides
sources-unsupported-version = Queste sorgenti sono state scritte da una versione più recente di Oku Slides
sources-bad-name = Nome di file rifiutato nelle sorgenti
sources-missing-deck = Nelle sorgenti manca la presentazione
sources-too-large = Le sorgenti di questo file sono troppo grandi per essere aperte
sources-too-deep = Questa presentazione annida gli elementi troppo in profondità
sources-base-refused = Un file di dati di questa presentazione è danneggiato o non è stato possibile salvarlo

issue-empty-slide-id = identificatore di diapositiva vuoto
issue-duplicate-slide-id = identificatore di diapositiva duplicato
issue-duplicate-element-id = identificatore di elemento duplicato
issue-empty-element-id = identificatore vuoto
issue-cell-outside-grid = cella fuori dalla griglia
issue-empty-frame = riquadro di dimensione zero
issue-free-without-frame = elemento libero senza riquadro
issue-unknown-widget = widget sconosciuto: { $kind }
issue-wrong-category = { $kind } non può essere usato qui
issue-unknown-param = parametro sconosciuto: { $param }
issue-param-type = valore non valido per { $param }
issue-choice-out-of-range = valore non consentito per { $param }
issue-slot-rejects = { $kind } non è accettato qui
issue-placed-in-flow = elemento posizionato con una cornice in un elenco impaginato
issue-unknown-bind = { $param } è collegato a un'impostazione del deck sconosciuta
issue-slot-too-deep = elementi annidati troppo in profondità
issue-invalid-style = valore di stile non valido: { $field } ({ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    })
issue-missing-asset = risorsa mancante: { $asset }
issue-pack-version-mismatch = versione del pacchetto alterata scartata: { $version }
issue-pack-version-missing = versione del pacchetto mancante dalle sorgenti: { $version }
issue-pack-pin-mismatch = Il pacchetto incorporato { $pack } non corrisponde a ciò che la presentazione si aspetta.
issue-pack-tokens-invalid = Il pacchetto incorporato { $pack } ha un token di componente non valido: { $reason }
issue-pack-kind-mismatch = Il pacchetto incorporato { $pack } non è del tipo che la presentazione si aspetta (widget o layout).
issue-pack-kind-conflict = Lo spazio dei nomi { $pack } è fissato sia come pacchetto di widget sia come pacchetto di layout: se ne può usare uno solo.
issue-pack-requirement-missing = Il pacchetto { $pack } richiede { $required }, che questa presentazione non ha.
issue-theme-version-mismatch = versione del tema alterata scartata: { $version }
issue-theme-version-missing = versione del tema mancante dalle sorgenti: { $version }
issue-theme-pin-mismatch = Il tema incorporato { $namespace } non corrisponde a ciò che la presentazione si aspetta.
issue-theme-closure = I temi fissati in questa presentazione non sono esattamente il suo tema e i suoi genitori ({ $namespace }).
issue-usage-line-invalid = riga { $line } del registro di utilizzo illeggibile, scartata
issue-usage-line-forged = turno di utilizzo { $turn } registrato due volte con contenuto diverso, uno scartato
issue-usage-too-large = registro di utilizzo troppo grande, scartato
issue-usage-unreadable = registro di utilizzo illeggibile, scartato
issue-usage-lines-dropped = { $count ->
    [one] { $count } altra riga del registro di utilizzo scartata
   *[other] { $count } altre righe del registro di utilizzo scartate
}
usage-range-bad-date = «{ $date }» non è una data: usa AAAA-MM-GG.
usage-range-reversed = La data di inizio { $since } è successiva alla data di fine { $until }.
pack-token-entry-invalid = Token di componente «{ $path }» non valido nel manifesto del pacchetto.
pack-manifest-json = Impossibile leggere il manifesto del pacchetto: { $reason }
pack-manifest-order = L'ordine degli elementi del pack non è valido in «{ $name }».
pack-manifest-data-count = Il pacchetto contiene { $count ->
    [one] { $count } dato
   *[other] { $count } dati
}, al massimo { $max }.
pack-manifest-data-base = I dati «{ $data }» del pacchetto rimandano a un file esterno; i dati di un pacchetto contengono le proprie celle.
pack-manifest-data-cells = I dati del pacchetto contengono { $count ->
    [one] { $count } cella
   *[other] { $count } celle
}, al massimo { $max }.
issue-unsupported-link = link non supportato: { $link }
issue-highlight-zero = i colori di evidenziazione si numerano da 1
issue-unreadable-element = elemento illeggibile, messo da parte
issue-slot-repaired = elemento senza slot noto, collocato in { $slot }
issue-unreadable-item = voce di elenco illeggibile, messa da parte
issue-unreadable-row = riga di tabella illeggibile, messa da parte
issue-data-truncated = dati «{ $data }» troppo grandi, troncati in lettura e tenuti in sola lettura
issue-data-name-taken = il nome di dati «{ $data }» è già in uso
issue-data-cell-type = dati «{ $data }»: la colonna «{ $column }» contiene valori che non sono di tipo { $type }
issue-data-too-many = troppi dati nella presentazione ({ $count }); non letti: { $dropped }
issue-data-base-missing = dati «{ $data }»: manca il loro file di base, tenuti in sola lettura
issue-data-edits-lost = dati «{ $data }»: modifiche a una tabella sostituita sono andate perse nell'unione
issue-data-inline-dropped = Elemento { $element }: una versione precedente ha salvato i dati del suo grafico o tabella nelle sue opzioni; sono stati rimossi
issue-data-kind-unknown = dati «{ $data }»: tipo sconosciuto «{ $kind }», forse di una versione più recente; non letti
issue-data-query-cycle = dati «{ $data }»: la loro query rilegge sé stessa, direttamente o attraverso altre query; non calcolati
issue-data-query-missing-input = dati «{ $data }»: la loro query legge «{ $input }», che non è nella presentazione; non calcolati
issue-data-query-invalid = dati «{ $data }»: la loro query non è valida ({ $reason }); non calcolati
issue-data-query-failed = dati «{ $data }»: la loro query non è riuscita ({ $reason }); nessun risultato
issue-more = altri problemi non mostrati: { $count }
op-data-deck-full = I dati della presentazione supererebbero il loro limite: al massimo { $max } { $what ->
    [row-ids] ID di riga
    [overlay] celle modificate
    [text] byte di testo
   *[data] dati
}
op-data-missing = Dati «{ $data }» non trovati
op-data-name-taken = Il nome di dati «{ $name }» è già in uso
op-data-column-in-use = La colonna «{ $column }» è mostrata da { $views }; collegali prima ad altre colonne (bind_data con columns)
op-data-in-use = Questi dati sono mostrati da { $views }{ $more ->
    [0] {""}
    [one] {" "}e da { $more } altra vista
   *[other] {" "}e da altre { $more } viste
}; rimuovile o collegale prima ad altri dati
op-data-column-missing = I dati «{ $data }» non hanno la colonna «{ $column }»
op-data-not-view = Il parametro «{ $param }» non può mostrare dati: solo un'opzione di grafico o tabella può farlo
op-data-truncated = I dati «{ $data }» sono stati troncati in lettura e sono di sola lettura; sostituisci l'intera tabella per modificarli
op-data-rebind = L'elemento { $subject } mostra già altri dati; usa bind_data per cambiarli
op-data-internal = Un lotto non può impostare la base o la sovrapposizione di una tabella
op-data-batch-too-large = Il lotto è troppo grande: numero di celle { $count }, al massimo { $max }
op-data-inline = L’elemento { $subject } porta i suoi dati nelle sue opzioni; aggiungili come dati del deck con add_data e collegali con bind_data
op-data-derived = i dati «{ $data }» derivano da una query: modifica la query, non le righe
op-data-source = Un dato ha una tabella oppure una query (sql), esattamente una delle due
op-data-cycle = I dati «{ $data }» rileggerebbero sé stessi attraverso le loro query; rimuovi il ciclo
op-data-unknown-input = La query legge «{ $name }», che non è un dato della presentazione
op-data-not-query = I dati «{ $data }» non sono una query; modifica la loro tabella con edit_data
op-data-read-by-queries = Questi dati sono letti dalle query { $queries }{ $shown ->
    [0] {""}
   *[other] {" "}e mostrati da { $views }
}{ $more ->
    [0] {""}
   *[other] {" "}e altri { $more }
}; modificali o rimuovili prima
op-data-rename-captured = La query di «{ $reader }» chiama già «{ $name }» una delle sue tabelle; rinomina quella tabella nella sua query (set_query) o scegli un altro nome
op-data-name-ambiguous = Più dati si chiamano «{ $name }» e alcune query leggono quel nome senza che si sappia quale; modifica prima quelle query (set_query)
issue-unreadable-slide = diapositiva illeggibile, messa da parte
issue-unreadable-field = campo { $field } del deck illeggibile, reimpostato
issue-knowledge-unreadable = voce di conoscenza { $id } illeggibile, messa da parte
issue-knowledge-entries-dropped = voci di conoscenza scartate senza lettura: { $count } ({ $cause ->
        [files] troppi file da leggere
        [file-bytes] file troppo grande da leggere
        [total-bytes] raccolta troppo grande da leggere
       *[unreadable] altre voci illeggibili
    })
issue-knowledge-over-limit = raccolta di conoscenza oltre un limite: { $limit ->
        [knowledge-reason-too-many-files] { knowledge-reason-too-many-files }
        [knowledge-reason-file-too-large] { knowledge-reason-file-too-large }
       *[knowledge-reason-total-too-large] { knowledge-reason-total-too-large }
    } (file interessati: { $count }, il primo { $id }); tutti i file sono conservati; solo eliminare dei file riporta il pacchetto sotto il limite (fino ad allora scritture e modifiche sono rifiutate)
issue-knowledge-path-invalid = file di conoscenza { $id } spostato in /recovered: { $reason ->
        [knowledge-path-empty] { knowledge-path-empty }
        [knowledge-path-not-absolute] { knowledge-path-not-absolute }
        [knowledge-path-bad-segment] { knowledge-path-bad-segment }
        [knowledge-path-dot-segment] { knowledge-path-dot-segment }
        [knowledge-path-reserved] { knowledge-path-reserved }
        [knowledge-path-too-deep] { knowledge-path-too-deep }
        [knowledge-path-too-long] { knowledge-path-too-long }
       *[knowledge-path-not-markdown] { knowledge-path-not-markdown }
    }
issue-knowledge-path-collision = più file di conoscenza erano in { $path }: gli altri sono stati rinominati
issue-knowledge-invalid = file di conoscenza { $path } non valido: { $reason ->
        [knowledge-frontmatter-missing] { knowledge-frontmatter-missing }
        [knowledge-frontmatter-unterminated] { knowledge-frontmatter-unterminated }
        [knowledge-frontmatter-too-large] { knowledge-frontmatter-too-large }
        [knowledge-frontmatter-not-yaml] { knowledge-frontmatter-not-yaml }
        [knowledge-frontmatter-not-mapping] { knowledge-frontmatter-not-mapping }
        [knowledge-frontmatter-duplicate-key] { knowledge-frontmatter-duplicate-key }
        [knowledge-frontmatter-alias] { knowledge-frontmatter-alias }
        [knowledge-frontmatter-tag] { knowledge-frontmatter-tag }
        [knowledge-frontmatter-too-deep] { knowledge-frontmatter-too-deep }
        [knowledge-frontmatter-after-document-end] { knowledge-frontmatter-after-document-end }
        [knowledge-frontmatter-flow-mapping] { knowledge-frontmatter-flow-mapping }
        [knowledge-frontmatter-type-missing] { knowledge-frontmatter-type-missing }
        [knowledge-frontmatter-bad-timestamp] { knowledge-frontmatter-bad-timestamp }
       *[knowledge-frontmatter-not-allowed] { knowledge-frontmatter-not-allowed }
    }
op-invalid-chart = Grafico { $subject }: { $reason }
op-invalid-table = Tabella { $subject }: { $reason }

replica-corrupted = La cronologia della presentazione in questo file è danneggiata
replica-too-large = La cronologia della presentazione in questo file è troppo grande per essere aperta
replica-write = Impossibile aggiornare la cronologia della presentazione
replica-unrelated = Queste due presentazioni non hanno un'origine comune
replica-unsupported-version = Questa presentazione usa un formato di deck che questa versione di Oku Slides non sa leggere
replica-too-deep = Impossibile salvare questa presentazione: alcuni elementi eliminati sono annidati troppo in profondità nella sua cronologia. Annulla gli ultimi spostamenti, poi salva di nuovo.

widget-unreadable = Impossibile leggere le dichiarazioni dei widget
widget-fault-empty-name = Widget { $widget }: un widget o parametro non ha nome
widget-fault-duplicate-widget = Il widget { $widget } è dichiarato due volte
widget-fault-duplicate-param = Il widget { $widget } dichiara lo stesso parametro due volte
widget-fault-empty-choice = Widget { $widget }: una scelta non ha valori
widget-fault-default-not-in-choice = Widget { $widget }: il valore predefinito di una scelta non è tra i suoi valori
widget-fault-default-wrong-type = Widget { $widget }: un valore predefinito non corrisponde al tipo del suo parametro
widget-fault-bind-not-plain = Widget { $widget }: solo i parametri di testo semplice possono essere collegati alla presentazione
widget-fault-unknown-slot-kind = Widget { $widget }: uno slot accetta un widget elemento sconosciuto
widget-fault-empty-list = Widget { $widget }: un elenco ha bisogno di almeno una colonna
widget-fault-column-type = Widget { $widget }: una colonna di elenco può essere plain, rich, number, flag, choice o asset (o una p.list annidata, un livello); per un contenuto libero per voce, usa p.slot con un widget figlio
widget-fault-reserved-name = Widget { $widget }: nome riservato: id, kind o un nome che contiene /
widget-fault-list-bounds = Widget { $widget }: il massimo di un elenco è inferiore al suo minimo
widget-fault-list-default-too-large = Widget { $widget }: il valore predefinito di un elenco supera i limiti di dimensione degli elenchi
widget-fault-list-depth = Widget { $widget }: un elenco annidato non può contenere una colonna elenco; gli elenchi si annidano su un solo livello
widget-fault-list-default-below-min = Widget { $widget }: il valore predefinito di un elenco annidato ha meno voci del suo minimo
widget-fault-slide-sizing = Widget { $widget } è una slide: solo i widget di elemento dichiarano grow o typography
widget-fault-pack-category = Il widget { $widget } non corrisponde al suo pack: i pack di layout contengono widget di slide, i pack di widget contengono widget di elemento
widget-fault-unqualified = Il widget { $widget } non ha uno spazio dei nomi di pack
widget-fault-too-long = Widget { $widget }: un nome, un'etichetta, un valore, una descrizione o un elenco supera il limite
issue-empty-item-id = l'elenco { $param } ha una voce senza identificatore
issue-duplicate-item-id = l'elenco { $param } ha un identificatore di voce duplicato
issue-invalid-item-id = l'elenco { $param } ha un identificatore di voce che contiene una barra
issue-list-bounds = l'elenco { $param } ha troppo poche o troppe voci
issue-view-truncated = { $element } mostra solo una parte dei suoi dati ({ $what ->
    [rows] righe
   *[columns] colonne
}: { $shown } su { $total })
issue-render-budget = { $element } non viene disegnato: le viste della presentazione leggono più dati di quanti un rendering ne consenta
issue-data-missing = { $element } indica i dati «{ $data }», che non sono nella presentazione
issue-view-column-missing = { $element } legge la colonna «{ $column }», che i suoi dati non hanno
issue-view-unreadable = { $element } non viene disegnato: le sue opzioni non sono leggibili
op-replica-full = La cronologia della presentazione sta per raggiungere { $max } MiB, il massimo per riaprirla: rimuovi contenuti prima di aggiungerne altri.
