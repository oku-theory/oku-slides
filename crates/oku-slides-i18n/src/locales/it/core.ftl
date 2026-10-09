op-slide-not-found = Diapositiva non trovata: { $subject }
op-element-not-found = Elemento non trovato: { $subject }
op-duplicate-id = Identificatore già in uso: { $subject }
op-index-out-of-range = Posizione fuori intervallo
op-invalid-value = Valore non valido per { $subject }
op-invalid-value-because = Valore non valido per { $subject }: { $reason }
op-value-no-value = il parametro non ha alcun valore
op-value-no-dataset = il grafico non ha un set di dati
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
op-unknown-widget = Widget sconosciuto: { $subject }
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
sources-not-sources = Questo non è un archivio di sorgenti di Oku Slides
sources-unsupported-version = Queste sorgenti sono state scritte da una versione più recente di Oku Slides
sources-bad-name = Nome di file rifiutato nelle sorgenti
sources-missing-deck = Nelle sorgenti manca la presentazione
sources-too-large = Le sorgenti di questo file sono troppo grandi per essere aperte
sources-too-deep = Questa presentazione annida gli elementi troppo in profondità

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
issue-missing-asset = risorsa mancante: { $asset }
issue-pack-version-mismatch = versione del pacchetto alterata scartata: { $version }
issue-pack-version-missing = versione del pacchetto mancante dalle sorgenti: { $version }
issue-theme-version-mismatch = versione del tema alterata scartata: { $version }
issue-theme-version-missing = versione del tema mancante dalle sorgenti: { $version }
issue-unsupported-link = link non supportato: { $link }
issue-highlight-zero = i colori di evidenziazione si numerano da 1
issue-unreadable-element = elemento illeggibile, messo da parte
issue-unreadable-item = voce di elenco illeggibile, messa da parte
issue-unreadable-row = riga di tabella illeggibile, messa da parte
issue-data-truncated = dati troppo grandi, troncati in lettura
issue-unreadable-slide = diapositiva illeggibile, messa da parte
issue-unreadable-field = campo { $field } del deck illeggibile, reimpostato
op-invalid-chart = Grafico { $subject }: { $reason }
op-invalid-table = Tabella { $subject }: { $reason }

replica-corrupted = La cronologia della presentazione in questo file è danneggiata
replica-too-large = La cronologia della presentazione in questo file è troppo grande per essere aperta
replica-write = Impossibile aggiornare la cronologia della presentazione
replica-unrelated = Queste due presentazioni non hanno un'origine comune
replica-unsupported-version = Questa presentazione usa un formato di deck che questa versione di Oku Slides non sa leggere

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
widget-fault-column-type = Widget { $widget }: una colonna di elenco può essere plain, rich, number, flag, choice o asset; per un contenuto libero per voce, usa p.slot con un widget figlio
widget-fault-reserved-name = Widget { $widget }: nome riservato: id, kind o un nome che contiene /
widget-fault-list-bounds = Widget { $widget }: il massimo di un elenco è inferiore al suo minimo
widget-fault-list-default-too-large = Widget { $widget }: il valore predefinito di un elenco supera i limiti di dimensione degli elenchi
widget-incompatible-override = Il widget { $widget } sostituisce quello base ma modifica o rimuove il suo parametro { $param }
widget-incompatible-category = Il widget { $widget } sostituisce quello base con una categoria diversa
issue-empty-item-id = l'elenco { $param } ha una voce senza identificatore
issue-duplicate-item-id = l'elenco { $param } ha un identificatore di voce duplicato
issue-list-bounds = l'elenco { $param } ha troppo poche o troppe voci
