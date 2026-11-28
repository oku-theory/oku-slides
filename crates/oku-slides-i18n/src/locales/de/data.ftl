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
data-source-mismatch = die Quelle der Daten passt nicht zu ihrer Art: manuelle Daten enthalten eine Tabelle, Abfragedaten eine Abfrage
data-inline-query = Abfragedaten können nicht eingebettet sein: eingebettete Daten stammen immer aus einer Tabelle
data-query-parse = die Abfrage lässt sich nicht lesen: Zeile { $line }, Spalte { $column } prüfen
data-query-construct = die Abfrage verwendet „{ $construct }“, was Datenabfragen nicht unterstützen
data-query-too-long = die Abfrage ist länger als { $max } { $max ->
    [one] Byte
   *[other] Bytes
}
data-query-too-deep = die Abfrage ist zu tief verschachtelt
data-query-rewrite = die Abfrage konnte nicht für die Engine vorbereitet werden; das ist ein Fehler in Oku Slides
data-query-unknown-function = die Abfrage ruft „{ $function }“ auf, was Datenabfragen nicht unterstützen
data-query-run = die Abfrage ist fehlgeschlagen: { $reason }
data-query-input-failed = ihre Eingabe „{ $input }“ ist fehlgeschlagen
data-query-too-large = { $what ->
    [rows] das Abfrageergebnis hat mehr als { $max } { $max ->
        [one] Zeile
       *[other] Zeilen
    }; es wird nie gekürzt
    [columns] das Abfrageergebnis hat mehr als { $max } { $max ->
        [one] Spalte
       *[other] Spalten
    }; es wird nie gekürzt
   *[text] eine Textzelle des Abfrageergebnisses ist länger als { $max } { $max ->
        [one] Byte
       *[other] Bytes
    }; es wird nie gekürzt
}
data-query-column-type = die Ergebnisspalte „{ $column }“ hat einen Typ, den Daten nicht aufnehmen können: wandle sie in Text, Zahl, Ganzzahl, Wahrheitswert oder Datum um
data-query-duplicate-column = zwei Ergebnisspalten heißen „{ $column }“: benenne eine davon mit AS um
data-query-float-operand = „{ $function }“ lässt sich nicht auf „{ $operand }“ anwenden, das Dezimalzahlen enthält: wandle es zuerst in eine Ganzzahl um
data-query-timeout = die Abfrage lief länger als { $seconds } { $seconds ->
    [one] Sekunde
   *[other] Sekunden
} und wurde abgebrochen
