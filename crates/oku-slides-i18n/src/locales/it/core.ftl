op-slide-not-found = Diapositiva non trovata: { $subject }
op-element-not-found = Elemento non trovato: { $subject }
op-duplicate-id = Identificatore già in uso: { $subject }
op-index-out-of-range = Posizione fuori intervallo
op-invalid-value = Valore non valido per { $subject }
op-unknown-widget = Widget sconosciuto: { $subject }
op-unknown-param = Parametro sconosciuto: { $subject }
op-param-type = Valore non valido per { $subject }
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
