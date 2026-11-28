data-cell-type = un valore non è adatto alla colonna «{ $column }», che contiene { $type ->
    [number] numeri
    [integer] numeri interi
    [bool] vero o falso
    [date] date (AAAA-MM-GG)
   *[text] testo di al massimo 16 KB
}
data-column-type = la colonna «{ $column }» non può diventare senza perdite { $type ->
    [number] una colonna di numeri
    [integer] una colonna di interi
    [bool] una colonna vero o falso
    [date] una colonna di date
   *[text] una colonna di testo
}, vedi le righe { $rows }{ $more ->
    [0] {""}
    [one] {" "}e { $more } altra riga
   *[other] {" "}e altre { $more } righe
}
data-no-row = la riga { $row } non esiste
data-no-column = la colonna { $column } non esiste
data-id-taken = l’identificatore { $id } è già usato in questa tabella
data-id-invalid = l’identificatore «{ $id }» non è valido: usa da 1 a 64 lettere, cifre, - o _
data-id-missing = una nuova riga, colonna o tabella non ha un identificatore
data-too-large = { $what ->
    [rows] i dati supererebbero il limite di { $max } { $max ->
        [one] riga
       *[other] righe
    }
    [columns] i dati supererebbero il limite di { $max } { $max ->
        [one] colonna
       *[other] colonne
    }
    [text] una cella di testo supererebbe il limite di { $max } { $max ->
        [one] byte
       *[other] byte
    }
   *[overlay] i dati supererebbero il limite di { $max } { $max ->
        [one] cella modificata
       *[other] celle modificate
    }
}
data-invalid-name = «{ $name }» non è un nome di dati valido: usa lettere minuscole, cifre e _, senza iniziare con una cifra
data-non-finite = { $count ->
    [one] { $count } numero non è finito (NaN o infinito)
   *[other] { $count } numeri non sono finiti (NaN o infinito)
}
data-columns-invalid = l’elenco delle colonne è vuoto o nomina due volte una colonna
data-row-width = una riga ha più celle delle { $max } { $max ->
    [one] colonna
   *[other] colonne
} della tabella
data-needs-base = questi dati tengono le righe in un file di base che questa modifica non sa ancora leggere
data-base-row-cells = una tabella con un file di base indica solo gli id delle righe, non le celle
data-row-cut = { $count ->
    [one] { $count } riga aveva più celle dell’intestazione; le celle in eccesso sono state rimosse
   *[other] { $count } righe avevano più celle dell’intestazione; le celle in eccesso sono state rimosse
}
data-base-invalid = il file di base dei dati è danneggiato o non corrisponde alla sua tabella
data-source-mismatch = l’origine dei dati non corrisponde al loro tipo: i dati manuali contengono una tabella, i dati di query una query
data-inline-query = i dati di query non possono essere incorporati: i dati incorporati provengono sempre da una tabella
data-query-parse = la query non si legge: controlla la riga { $line }, colonna { $column }
data-query-construct = la query usa «{ $construct }», che le query sui dati non supportano
data-query-too-long = la query supera { $max } { $max ->
    [one] byte
   *[other] byte
}
data-query-too-deep = la query è annidata troppo in profondità
data-query-rewrite = non è stato possibile preparare la query per il motore; è un bug di Oku Slides
data-query-unknown-function = la query chiama «{ $function }», che le query sui dati non supportano
data-query-run = la query non è riuscita: { $reason }
data-query-input-failed = il suo input «{ $input }» non è riuscito
data-query-too-large = { $what ->
    [rows] il risultato della query supera { $max } { $max ->
        [one] riga
       *[other] righe
    }; non viene mai troncato
    [columns] il risultato della query supera { $max } { $max ->
        [one] colonna
       *[other] colonne
    }; non viene mai troncato
   *[text] una cella di testo del risultato supera { $max } { $max ->
        [one] byte
       *[other] byte
    }; il risultato non viene mai troncato
}
data-query-column-type = la colonna di risultato «{ $column }» ha un tipo che i dati non possono contenere: convertila in testo, numero, intero, booleano o data
data-query-duplicate-column = due colonne di risultato si chiamano «{ $column }»: rinominane una con AS
data-query-float-operand = «{ $function }» non si può applicare a «{ $operand }», che contiene numeri decimali: convertilo prima in intero
data-query-timeout = la query è durata più di { $seconds } { $seconds ->
    [one] secondo
   *[other] secondi
} ed è stata interrotta
