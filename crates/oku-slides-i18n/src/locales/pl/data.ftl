data-cell-type = wartość nie pasuje do kolumny „{ $column }”, która zawiera { $type ->
    [number] liczby
    [integer] liczby całkowite
    [bool] prawdę lub fałsz
    [date] daty (RRRR-MM-DD)
   *[text] tekst do 16 KB
}
data-column-type = kolumny „{ $column }” nie można bez strat zmienić na { $type ->
    [number] kolumnę liczb
    [integer] kolumnę liczb całkowitych
    [bool] kolumnę prawda/fałsz
    [date] kolumnę dat
   *[text] kolumnę tekstu
}, zobacz wiersze { $rows }{ $more ->
    [0] {""}
    [one] {" "}i jeszcze { $more } wiersz
    [few] {" "}i jeszcze { $more } wiersze
   *[many] {" "}i jeszcze { $more } wierszy
}
data-no-row = wiersz { $row } nie istnieje
data-no-column = kolumna { $column } nie istnieje
data-id-taken = identyfikator { $id } jest już używany w tej tabeli
data-id-invalid = identyfikator „{ $id }” jest nieprawidłowy: użyj od 1 do 64 liter łacińskich, cyfr, - lub _
data-id-missing = nowy wiersz, kolumna lub tabela nie ma identyfikatora
data-too-large = { $what ->
    [rows] dane przekroczyłyby limit wierszy: { $max } { $max ->
        [one] wiersz
        [few] wiersze
       *[many] wierszy
    }
    [columns] dane przekroczyłyby limit kolumn: { $max } { $max ->
        [one] kolumna
        [few] kolumny
       *[many] kolumn
    }
    [text] komórka tekstowa przekroczyłaby limit: { $max } { $max ->
        [one] bajt
        [few] bajty
       *[many] bajtów
    }
   *[overlay] dane przekroczyłyby limit edytowanych komórek: { $max } { $max ->
        [one] komórka
        [few] komórki
       *[many] komórek
    }
}
data-invalid-name = „{ $name }” nie jest prawidłową nazwą danych: użyj małych liter łacińskich, cyfr i _, nie zaczynając od cyfry
data-non-finite = { $count ->
    [one] { $count } liczba nie jest skończona (NaN lub nieskończoność)
    [few] { $count } liczby nie są skończone (NaN lub nieskończoność)
   *[many] { $count } liczb nie jest skończonych (NaN lub nieskończoność)
}
data-columns-invalid = lista kolumn jest pusta lub wymienia kolumnę dwa razy
data-row-width = wiersz ma więcej komórek niż { $max } { $max ->
    [one] kolumna
    [few] kolumny
   *[many] kolumn
} tabeli
data-needs-base = te dane trzymają wiersze w pliku bazowym, którego ta zmiana jeszcze nie umie odczytać
data-base-row-cells = tabela z plikiem bazowym podaje dla wierszy tylko identyfikatory, bez komórek
data-row-cut = { $count ->
    [one] { $count } wiersz miał więcej komórek niż nagłówek; nadmiarowe komórki usunięto
    [few] { $count } wiersze miały więcej komórek niż nagłówek; nadmiarowe komórki usunięto
   *[many] { $count } wierszy miało więcej komórek niż nagłówek; nadmiarowe komórki usunięto
}
data-base-invalid = plik bazowy danych jest uszkodzony lub nie pasuje do ich tabeli
