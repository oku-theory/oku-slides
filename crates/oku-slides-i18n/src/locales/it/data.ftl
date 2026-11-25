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
