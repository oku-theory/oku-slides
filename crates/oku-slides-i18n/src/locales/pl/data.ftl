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
data-source-mismatch = źródło danych nie pasuje do ich rodzaju: dane ręczne zawierają tabelę, dane z zapytania zawierają zapytanie
data-inline-query = dane z zapytania nie mogą być osadzone: dane osadzone zawsze pochodzą z tabeli
data-query-parse = nie można odczytać zapytania: sprawdź wiersz { $line }, kolumnę { $column }
data-query-construct = zapytanie używa „{ $construct }”, czego zapytania danych nie obsługują
data-query-too-long = zapytanie przekracza limit: { $max } { $max ->
    [one] bajt
    [few] bajty
   *[many] bajtów
}
data-query-too-deep = zapytanie jest zagnieżdżone zbyt głęboko
data-query-rewrite = nie udało się przygotować zapytania dla silnika; to błąd w Oku Slides
data-query-unknown-function = zapytanie wywołuje „{ $function }”, czego zapytania danych nie obsługują
data-query-run = zapytanie nie powiodło się: { $reason }
data-query-input-failed = jego wejście „{ $input }” nie powiodło się
data-query-too-large = { $what ->
    [rows] wynik zapytania przekracza limit: { $max } { $max ->
        [one] wiersz
        [few] wiersze
       *[many] wierszy
    }; nigdy nie jest przycinany
    [columns] wynik zapytania przekracza limit: { $max } { $max ->
        [one] kolumna
        [few] kolumny
       *[many] kolumn
    }; nigdy nie jest przycinany
   *[text] komórka tekstowa wyniku przekracza limit: { $max } { $max ->
        [one] bajt
        [few] bajty
       *[many] bajtów
    }; wynik nigdy nie jest przycinany
}
data-query-column-type = kolumna wyniku „{ $column }” ma typ, którego dane nie mogą przechowywać: przekształć ją w tekst, liczbę, liczbę całkowitą, wartość logiczną lub datę
data-query-duplicate-column = dwie kolumny wyniku nazywają się „{ $column }”: zmień nazwę jednej z nich za pomocą AS
data-query-float-operand = „{ $function }” nie można zastosować do „{ $operand }”, który zawiera liczby dziesiętne: najpierw przekształć go w liczbę całkowitą
data-query-timeout = zapytanie trwało dłużej niż limit: { $seconds } { $seconds ->
    [one] sekunda
    [few] sekundy
   *[many] sekund
} i zostało zatrzymane
