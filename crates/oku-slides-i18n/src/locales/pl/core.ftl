op-slide-not-found = Nie znaleziono slajdu: { $subject }
op-element-not-found = Nie znaleziono elementu: { $subject }
op-duplicate-id = Identyfikator jest już używany: { $subject }
op-index-out-of-range = Pozycja poza zakresem
op-invalid-value = Nieprawidłowa wartość dla { $subject }
op-invalid-value-because = Nieprawidłowa wartość dla { $subject }: { $reason }
op-value-no-value = parametr nie ma wartości
op-value-no-dataset = wykres nie ma zbioru danych
op-value-grid-tracks = siatka ma od 1 do 24 kolumn i wierszy
op-value-bad-item-id = identyfikator pozycji to niepusty tekst bez „/”
op-value-missing-item-id = każda pozycja potrzebuje identyfikatora
op-value-too-deep = elementy są zagnieżdżone zbyt głęboko
op-value-empty-path = ścieżka jest pusta
op-value-not-text = oczekiwano tekstu lub null
op-value-empty-frame = szerokość i wysokość muszą być większe od 0
op-value-blocks-off-grid = sloty przepływu układa slajd, a siatka je ukrywa; najpierw przenieś każdy element do wolnego slotu
op-value-outside-grid = komórka jest poza siatką
op-value-not-tokens = oczekiwano obiektu lub null
op-value-not-a-slot = ten parametr nie jest slotem
op-value-out-of-bounds = liczba jest poza dozwolonym zakresem
op-value-unreadable = wartość nie ma oczekiwanej postaci
op-value-valign = oczekiwano top, middle lub bottom
op-value-path = ścieżka składa się z poleceń M, L, C, Q i Z z ich liczbami i zaczyna się od M
op-unknown-widget = Nieznany widżet: { $subject }
op-unknown-param = Nieznany parametr: { $subject }
op-param-type-because = Nieprawidłowa wartość dla { $subject }: { $reason }
op-param-shape-text = { $nullable ->
    [yes] wymagane: tekst lub null
   *[no] wymagane: tekst
}
op-param-shape-choice = { $nullable ->
    [yes] wymagane: jedna z jego wartości lub null
   *[no] wymagane: jedna z jego wartości
}
op-param-shape-paragraphs = { $nullable ->
    [yes] wymagane: lista akapitów lub null
   *[no] wymagane: lista akapitów
}
op-param-shape-elements = { $nullable ->
    [yes] wymagane: lista elementów lub null
   *[no] wymagane: lista elementów
}
op-param-shape-flag = { $nullable ->
    [yes] wymagane: wartość logiczna lub null
   *[no] wymagane: wartość logiczna
}
op-param-shape-number = { $nullable ->
    [yes] wymagane: liczba lub null
   *[no] wymagane: liczba
}
op-param-shape-bounded-number = { $nullable ->
    [yes] wymagane: liczba w dozwolonych granicach lub null
   *[no] wymagane: liczba w dozwolonych granicach
}
op-param-shape-grid = { $nullable ->
    [yes] wymagane: siatka lub null
   *[no] wymagane: siatka
}
op-param-shape-chart-option = { $nullable ->
    [yes] wymagane: opcja wykresu lub null
   *[no] wymagane: opcja wykresu
}
op-param-shape-table-option = { $nullable ->
    [yes] wymagane: opcja tabeli lub null
   *[no] wymagane: opcja tabeli
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] wymagane: opcja wykresu lub tabeli lub null
   *[no] wymagane: opcja wykresu lub tabeli
}
op-param-shape-items = { $nullable ->
    [yes] wymagane: lista pozycji lub null
   *[no] wymagane: lista pozycji
}
op-param-shape-list = { $nullable ->
    [yes] wymagane: lista lub null
   *[no] wymagane: lista
}
op-param-shape-add-element-only = { $nullable ->
    [yes] elementy dodaje się przez add_element, lub null
   *[no] elementy dodaje się przez add_element
}
op-param-shape-set-param-only = { $nullable ->
    [yes] wartość ustawia się przez set_param, lub null
   *[no] wartość ustawia się przez set_param
}
op-param-shape-path-option = { $nullable ->
    [yes] oczekiwano ścieżki kształtu (obiektu z kluczem d) lub null
   *[no] oczekiwano ścieżki kształtu (obiektu z kluczem d)
}
op-item-not-found = Brak pozycji { $subject }
op-list-full = Lista { $subject } jest pełna (najwyżej { $max ->
   [one] { $max } pozycja
   [few] { $max } pozycje
   [many] { $max } pozycji
  *[other] { $max } pozycji
})
op-list-minimum = Lista { $subject } musi mieć co najmniej { $min ->
   [one] { $min } pozycję
   [few] { $min } pozycje
   [many] { $min } pozycji
  *[other] { $min } pozycji
}
op-list-cell-too-long = Komórka listy { $subject } jest za długa (kolumna { $column })
op-list-id-too-long = Identyfikator pozycji listy { $subject } jest za długi
op-list-too-many-items = Lista { $subject } ma zbyt wiele pozycji (najwyżej { $max })
op-slot-rejects = { $subject } nie jest tu akceptowany
op-wrong-category = { $subject } nie może być tu użyty
op-choice-out-of-range = Niedozwolona wartość dla { $subject }
transaction-refused = Operacja { $index } odrzucona: { $reason }

sources-unreadable = Nieczytelne źródła
sources-not-sources = To nie jest archiwum źródeł Oku Slides
sources-unsupported-version = Te źródła zostały zapisane nowszą wersją Oku Slides
sources-bad-name = Odrzucona nazwa pliku w źródłach
sources-missing-deck = W źródłach brakuje prezentacji
sources-too-large = Źródła w tym pliku są zbyt duże, aby je otworzyć
sources-too-deep = Ta prezentacja zagnieżdża elementy zbyt głęboko

issue-empty-slide-id = pusty identyfikator slajdu
issue-duplicate-slide-id = zduplikowany identyfikator slajdu
issue-duplicate-element-id = zduplikowany identyfikator elementu
issue-empty-element-id = pusty identyfikator
issue-cell-outside-grid = komórka poza siatką
issue-empty-frame = ramka o zerowym rozmiarze
issue-free-without-frame = swobodny element bez ramki
issue-unknown-widget = nieznany widżet: { $kind }
issue-wrong-category = { $kind } nie może być tu użyty
issue-unknown-param = nieznany parametr: { $param }
issue-param-type = nieprawidłowa wartość dla { $param }
issue-choice-out-of-range = niedozwolona wartość dla { $param }
issue-slot-rejects = { $kind } nie jest tu akceptowany
issue-placed-in-flow = element umieszczony z ramką na liście z układem
issue-unknown-bind = { $param } jest powiązany z nieznanym ustawieniem prezentacji
issue-slot-too-deep = elementy są zagnieżdżone zbyt głęboko
issue-invalid-style = nieprawidłowa wartość stylu: { $field }
issue-missing-asset = brakujący zasób: { $asset }
issue-pack-version-mismatch = zmieniona wersja pakietu odrzucona: { $version }
issue-pack-version-missing = brak wersji pakietu w źródłach: { $version }
issue-theme-version-mismatch = zmieniona wersja motywu odrzucona: { $version }
issue-theme-version-missing = brak wersji motywu w źródłach: { $version }
issue-unsupported-link = nieobsługiwany link: { $link }
issue-highlight-zero = kolory wyróżnienia numeruje się od 1
issue-unreadable-element = nieczytelny element, odłożony
issue-slot-repaired = element bez znanego slotu, umieszczony w { $slot }
issue-unreadable-item = nieczytelna pozycja listy, odłożona
issue-unreadable-row = nieczytelny wiersz tabeli, odłożony
issue-data-truncated = dane zbyt duże, przycięte przy odczycie
issue-unreadable-slide = nieczytelny slajd, odłożony
issue-unreadable-field = nieczytelne pole { $field } prezentacji, zresetowane
op-invalid-chart = Wykres { $subject }: { $reason }
op-invalid-table = Tabela { $subject }: { $reason }

replica-corrupted = Historia prezentacji w tym pliku jest uszkodzona
replica-too-large = Historia prezentacji w tym pliku jest zbyt duża, aby ją otworzyć
replica-write = Nie udało się zaktualizować historii prezentacji
replica-unrelated = Te dwie prezentacje nie mają wspólnego pochodzenia
replica-unsupported-version = Ta prezentacja używa formatu, którego ta wersja Oku Slides nie potrafi odczytać

widget-unreadable = Nie udało się odczytać deklaracji widżetów
widget-fault-empty-name = Widżet { $widget }: widżet lub parametr nie ma nazwy
widget-fault-duplicate-widget = Widżet { $widget } jest zadeklarowany dwukrotnie
widget-fault-duplicate-param = Widżet { $widget } deklaruje ten sam parametr dwukrotnie
widget-fault-empty-choice = Widżet { $widget }: wybór nie ma żadnych wartości
widget-fault-default-not-in-choice = Widżet { $widget }: domyślna wartość wyboru nie należy do jego wartości
widget-fault-default-wrong-type = Widżet { $widget }: wartość domyślna nie odpowiada typowi swojego parametru
widget-fault-bind-not-plain = Widżet { $widget }: tylko parametry zwykłego tekstu mogą być powiązane z prezentacją
widget-fault-unknown-slot-kind = Widżet { $widget }: slot akceptuje nieznany widżet elementu
widget-fault-empty-list = Widżet { $widget }: lista potrzebuje co najmniej jednej kolumny
widget-fault-column-type = Widżet { $widget }: kolumna listy może być typu plain, rich, number, flag, choice lub asset; dla dowolnej zawartości pozycji użyj p.slot z widżetem potomnym
widget-fault-reserved-name = Widżet { $widget }: zarezerwowana nazwa: id, kind lub nazwa zawierająca /
widget-fault-list-bounds = Widżet { $widget }: maksimum listy jest poniżej jej minimum
widget-fault-list-default-too-large = Widżet { $widget }: wartość domyślna listy przekracza limity rozmiaru list
widget-incompatible-override = Widżet { $widget } zastępuje wbudowany widżet, ale zmienia lub usuwa jego parametr { $param }
widget-incompatible-category = Widżet { $widget } zastępuje wbudowany widżet inną kategorią
issue-empty-item-id = lista { $param } ma pozycję bez identyfikatora
issue-duplicate-item-id = lista { $param } ma zduplikowany identyfikator pozycji
issue-list-bounds = lista { $param } ma za mało lub za dużo pozycji
