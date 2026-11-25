data-cell-type = een waarde past niet in kolom ‘{ $column }’, die { $type ->
    [number] getallen
    [integer] gehele getallen
    [bool] waar of onwaar
    [date] datums (JJJJ-MM-DD)
   *[text] tekst van hoogstens 16 KB
} bevat
data-column-type = kolom ‘{ $column }’ kan niet zonder verlies { $type ->
    [number] een getallenkolom
    [integer] een kolom met gehele getallen
    [bool] een waar/onwaar-kolom
    [date] een datumkolom
   *[text] een tekstkolom
} worden, zie rijen { $rows }{ $more ->
    [0] {""}
    [one] {" "}en nog { $more } rij
   *[other] {" "}en nog { $more } rijen
}
data-no-row = rij { $row } bestaat niet
data-no-column = kolom { $column } bestaat niet
data-id-taken = id { $id } wordt al gebruikt in deze tabel
data-id-invalid = id ‘{ $id }’ is ongeldig: gebruik 1 tot 64 letters, cijfers, - of _
data-id-missing = een nieuwe rij, kolom of tabel heeft geen id
data-too-large = { $what ->
    [rows] de gegevens zouden hun limiet van { $max } { $max ->
        [one] rij
       *[other] rijen
    } overschrijden
    [columns] de gegevens zouden hun limiet van { $max } { $max ->
        [one] kolom
       *[other] kolommen
    } overschrijden
    [text] een tekstcel zou haar limiet van { $max } { $max ->
        [one] byte
       *[other] bytes
    } overschrijden
   *[overlay] de gegevens zouden hun limiet van { $max } { $max ->
        [one] bewerkte cel
       *[other] bewerkte cellen
    } overschrijden
}
data-invalid-name = ‘{ $name }’ is geen geldige gegevensnaam: gebruik kleine letters, cijfers en _, niet beginnend met een cijfer
data-non-finite = { $count ->
    [one] { $count } getal is niet eindig (NaN of oneindig)
   *[other] { $count } getallen zijn niet eindig (NaN of oneindig)
}
data-columns-invalid = de kolomlijst is leeg of noemt een kolom twee keer
data-row-width = een rij heeft meer cellen dan de { $max } { $max ->
    [one] kolom
   *[other] kolommen
} van de tabel
data-needs-base = deze gegevens bewaren hun rijen in een basisbestand dat deze bewerking nog niet kan lezen
data-base-row-cells = een tabel met een basisbestand geeft voor zijn rijen alleen id’s, geen cellen
data-row-cut = { $count ->
    [one] { $count } rij had meer cellen dan de kopregel; de extra cellen zijn verwijderd
   *[other] { $count } rijen hadden meer cellen dan de kopregel; de extra cellen zijn verwijderd
}
data-base-invalid = het basisbestand van de gegevens is beschadigd of past niet bij de tabel
