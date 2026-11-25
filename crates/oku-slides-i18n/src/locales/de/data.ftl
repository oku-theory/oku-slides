data-cell-type = ein Wert passt nicht in die Spalte „{ $column }“, die { $type ->
    [number] Zahlen
    [integer] ganze Zahlen
    [bool] wahr oder falsch
    [date] Datumsangaben (JJJJ-MM-TT)
   *[text] Text bis 16 KB
} enthält
data-column-type = die Spalte „{ $column }“ kann nicht verlustfrei { $type ->
    [number] eine Zahlenspalte
    [integer] eine Ganzzahlspalte
    [bool] eine Wahr-/Falsch-Spalte
    [date] eine Datumsspalte
   *[text] eine Textspalte
} werden, siehe Zeilen { $rows }{ $more ->
    [0] {""}
    [one] {" "}und { $more } weitere Zeile
   *[other] {" "}und { $more } weitere Zeilen
}
data-no-row = die Zeile { $row } existiert nicht
data-no-column = die Spalte { $column } existiert nicht
data-id-taken = die ID { $id } wird in dieser Tabelle bereits verwendet
data-id-invalid = die ID „{ $id }“ ist ungültig: 1 bis 64 Buchstaben, Ziffern, - oder _ verwenden
data-id-missing = eine neue Zeile, Spalte oder Tabelle hat keine ID
data-too-large = { $what ->
    [rows] die Daten würden ihr Limit von { $max } { $max ->
        [one] Zeile
       *[other] Zeilen
    } überschreiten
    [columns] die Daten würden ihr Limit von { $max } { $max ->
        [one] Spalte
       *[other] Spalten
    } überschreiten
    [text] eine Textzelle würde ihr Limit von { $max } { $max ->
        [one] Byte
       *[other] Bytes
    } überschreiten
   *[overlay] die Daten würden ihr Limit von { $max } { $max ->
        [one] bearbeiteten Zelle
       *[other] bearbeiteten Zellen
    } überschreiten
}
data-invalid-name = „{ $name }“ ist kein gültiger Datenname: Kleinbuchstaben, Ziffern und _ verwenden, nicht mit einer Ziffer beginnen
data-non-finite = { $count ->
    [one] { $count } Zahl ist nicht endlich (NaN oder unendlich)
   *[other] { $count } Zahlen sind nicht endlich (NaN oder unendlich)
}
data-columns-invalid = die Spaltenliste ist leer oder nennt eine Spalte zweimal
data-row-width = eine Zeile hat mehr Zellen als die { $max } { $max ->
    [one] Spalte
   *[other] Spalten
} der Tabelle
data-needs-base = diese Daten halten ihre Zeilen in einer Basisdatei, die diese Bearbeitung noch nicht lesen kann
data-base-row-cells = eine Tabelle mit Basisdatei gibt für ihre Zeilen nur IDs an, keine Zellen
data-row-cut = { $count ->
    [one] { $count } Zeile hatte mehr Zellen als die Kopfzeile; die überzähligen Zellen wurden entfernt
   *[other] { $count } Zeilen hatten mehr Zellen als die Kopfzeile; die überzähligen Zellen wurden entfernt
}
data-base-invalid = die Basisdatei der Daten ist beschädigt oder passt nicht zu ihrer Tabelle
