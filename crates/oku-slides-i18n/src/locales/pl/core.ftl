op-slide-not-found = Nie znaleziono slajdu: { $subject }
op-element-not-found = Nie znaleziono elementu: { $subject }
op-duplicate-id = Identyfikator jest już używany: { $subject }
op-index-out-of-range = Pozycja poza zakresem
op-invalid-value = Nieprawidłowa wartość dla { $subject }
op-invalid-value-because = Nieprawidłowa wartość dla { $subject }: { $reason }
op-value-no-value = parametr nie ma wartości
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
knowledge-path-empty = ścieżka jest pusta
knowledge-path-not-absolute = ścieżka musi zaczynać się od /
knowledge-path-bad-segment = segment jest pusty, kończy się kropką lub zawiera znak inny niż a-z, 0-9, kropka, podkreślenie i łącznik
knowledge-path-dot-segment = segment nie może zaczynać się od kropki
knowledge-path-reserved = segment jest zarezerwowaną nazwą systemową
knowledge-path-too-deep = ścieżka ma zbyt wiele segmentów
knowledge-path-too-long = ścieżka jest zbyt długa
knowledge-path-not-markdown = nazwa pliku musi kończyć się na .md
knowledge-frontmatter-missing = plik musi zaczynać się blokiem nagłówka ograniczonym przez ---
knowledge-frontmatter-unterminated = blok nagłówka nie ma zamykającego ---
knowledge-frontmatter-too-large = blok nagłówka jest za duży
knowledge-frontmatter-not-yaml = nagłówek nie jest poprawnym YAML-em
knowledge-frontmatter-not-mapping = nagłówek musi być mapą z kluczami tekstowymi
knowledge-frontmatter-duplicate-key = klucz "{ $key }" występuje w nagłówku dwa razy
knowledge-frontmatter-alias = nagłówek nie może używać kotwic ani aliasów YAML
knowledge-frontmatter-tag = nagłówek nie może używać tagów YAML
knowledge-frontmatter-too-deep = nagłówek jest zagnieżdżony zbyt głęboko
knowledge-frontmatter-after-document-end = nagłówek zawiera tekst po końcu dokumentu YAML ...
knowledge-frontmatter-flow-mapping = nagłówek musi być jednym mapowaniem blokowym, jeden klucz na wiersz, bez końca `...`
knowledge-frontmatter-type-missing = nagłówek wymaga niepustego typu
knowledge-frontmatter-bad-timestamp = wartość "{ $key }" w nagłówku musi być datą i godziną RFC 3339 z przesunięciem strefy
knowledge-frontmatter-not-allowed = klucz "{ $key }" nie jest dozwolony w nagłówku tego pliku
knowledge-reason-not-found = brak pliku pod tą ścieżką
knowledge-reason-exists = pod tą ścieżką istnieje już plik
knowledge-reason-id-missing = nowy plik wymaga identyfikatora
knowledge-reason-id-invalid = identyfikator pliku to 32 małe cyfry szesnastkowe
knowledge-reason-id-taken = inny plik ma już ten identyfikator
knowledge-reason-id-mismatch = plik pod tą ścieżką ma inny identyfikator
knowledge-reason-edit-empty-old = tekst do zastąpienia nie może być pusty
knowledge-reason-edit-no-match = tekst do zastąpienia nie występuje w pliku
knowledge-reason-edit-ambiguous = tekst do zastąpienia występuje w pliku więcej niż raz
knowledge-reason-too-many-files = paczka ma za dużo plików
knowledge-reason-file-too-large = plik jest za duży
knowledge-reason-total-too-large = paczka jest za duża
knowledge-reason-over-limit = baza wiedzy przekracza swoje limity: tylko usunięcie plików sprowadzi ją z powrotem poniżej
knowledge-no-text-layer = ten PDF nie ma warstwy tekstowej (to skan): wyeksportuj go z tekstem albo wklej jego tekst
knowledge-legacy-format = tego starszego formatu Office nie można odczytać: zapisz go jako .pptx, .docx lub .xlsx
knowledge-document-encrypted = ten dokument jest chroniony hasłem: zapisz niechronioną kopię
knowledge-unsupported-document = tego rodzaju dokumentu nie można zaimportować: PDF, DOCX, PPTX, XLSX, ODS, Markdown lub tekst
knowledge-document-damaged = ten dokument jest uszkodzony i nie można go odczytać
knowledge-document-type-mismatch = zawartość tego pliku nie pasuje do jego nazwy
knowledge-document-too-large = ten dokument jest za duży, aby go zaimportować
op-knowledge = Plik wiedzy { $path }: { $reason }
op-unknown-widget = Nieznany widżet: { $subject }
op-unknown-namespace = Ta prezentacja nie ma pakietu { $pack }.
op-pack-required = Pakiet { $pack } jest potrzebny pakietowi { $by }; najpierw usuń tamten.
op-pack-in-use = Pakiet { $pack } jest nadal używany przez { $count ->
    [one] { $count } element
    [few] { $count } elementy
    [many] { $count } elementów
   *[other] { $count } elementu
}.
op-pack-requirement-missing = Pakiet { $pack } wymaga pakietu { $required }, którego ta prezentacja nie ma.
op-base-required = Każda prezentacja zachowuje podstawowy pakiet widżetów.
op-pack-kind-conflict = Ta prezentacja ma już pakiet { $pack } drugiego rodzaju (widżety lub układy).
op-theme-cycle = Motywy rozszerzają się nawzajem w pętli: { $chain }.
op-theme-closure = Motywy przypięte w tej prezentacji nie są dokładnie jej motywem i jego rodzicami ({ $namespace }).
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
deck-inline-data = Element { $element } przechowuje dane w swoich opcjach: ten plik decku nie ma bieżącego formatu
sources-not-sources = To nie jest archiwum źródeł Oku Slides
sources-unsupported-version = Te źródła zostały zapisane nowszą wersją Oku Slides
sources-bad-name = Odrzucona nazwa pliku w źródłach
sources-missing-deck = W źródłach brakuje prezentacji
sources-too-large = Źródła w tym pliku są zbyt duże, aby je otworzyć
sources-too-deep = Ta prezentacja zagnieżdża elementy zbyt głęboko
sources-base-refused = Plik danych tej prezentacji jest uszkodzony lub nie można go zapisać

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
issue-invalid-style = nieprawidłowa wartość stylu: { $field } ({ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    })
issue-missing-asset = brakujący zasób: { $asset }
issue-pack-version-mismatch = zmieniona wersja pakietu odrzucona: { $version }
issue-pack-version-missing = brak wersji pakietu w źródłach: { $version }
issue-pack-pin-mismatch = Osadzony pakiet { $pack } nie odpowiada temu, czego oczekuje prezentacja.
issue-pack-tokens-invalid = Osadzony pakiet { $pack } zawiera nieprawidłowy token komponentu: { $reason }
issue-pack-kind-mismatch = Osadzony pakiet { $pack } nie jest rodzaju, którego oczekuje prezentacja (widżety lub układy).
issue-pack-kind-conflict = Przestrzeń nazw { $pack } jest przypięta jednocześnie jako pakiet widżetów i jako pakiet układów: można użyć tylko jednego.
issue-pack-requirement-missing = Pakiet { $pack } wymaga pakietu { $required }, którego ta prezentacja nie ma.
issue-theme-version-mismatch = zmieniona wersja motywu odrzucona: { $version }
issue-theme-version-missing = brak wersji motywu w źródłach: { $version }
issue-theme-pin-mismatch = Osadzony motyw { $namespace } nie odpowiada temu, czego oczekuje prezentacja.
issue-theme-closure = Motywy przypięte w tej prezentacji nie są dokładnie jej motywem i jego rodzicami ({ $namespace }).
issue-usage-line-invalid = wiersz { $line } dziennika użycia nieczytelny, odrzucono
issue-usage-line-forged = tura użycia { $turn } zapisana dwa razy z różną treścią, jedną odrzucono
issue-usage-too-large = dziennik użycia jest za duży, odrzucono
issue-usage-unreadable = dziennik użycia jest nieczytelny, odrzucono
issue-usage-lines-dropped = { $count ->
    [one] odrzucono jeszcze { $count } wiersz dziennika użycia
    [few] odrzucono jeszcze { $count } wiersze dziennika użycia
    [many] odrzucono jeszcze { $count } wierszy dziennika użycia
   *[other] odrzucono jeszcze { $count } wiersza dziennika użycia
}
usage-range-bad-date = „{ $date }” nie jest datą: użyj RRRR-MM-DD.
usage-range-reversed = Data początkowa { $since } jest późniejsza niż data końcowa { $until }.
pack-token-entry-invalid = Nieprawidłowy token komponentu „{ $path }” w manifeście pakietu.
pack-manifest-json = Nie można odczytać manifestu pakietu: { $reason }
pack-manifest-order = Kolejność elementów paczki jest nieprawidłowa przy „{ $name }”.
pack-manifest-data-count = Pakiet zawiera { $count ->
    [one] { $count } zbiór danych
    [few] { $count } zbiory danych
   *[many] { $count } zbiorów danych
}, najwyżej { $max }.
pack-manifest-data-base = Dane „{ $data }” pakietu odwołują się do pliku zewnętrznego; dane pakietu zawierają własne komórki.
pack-manifest-data-cells = Dane pakietu zawierają { $count ->
    [one] { $count } komórkę
    [few] { $count } komórki
   *[many] { $count } komórek
}, najwyżej { $max }.
issue-unsupported-link = nieobsługiwany link: { $link }
issue-highlight-zero = kolory wyróżnienia numeruje się od 1
issue-unreadable-element = nieczytelny element, odłożony
issue-slot-repaired = element bez znanego slotu, umieszczony w { $slot }
issue-unreadable-item = nieczytelna pozycja listy, odłożona
issue-unreadable-row = nieczytelny wiersz tabeli, odłożony
issue-data-truncated = dane „{ $data }” zbyt duże, przycięte przy odczycie i zachowane tylko do odczytu
issue-data-name-taken = nazwa danych „{ $data }” jest już zajęta
issue-data-cell-type = dane „{ $data }”: kolumna „{ $column }” zawiera wartości, które nie są typu { $type }
issue-data-too-many = zbyt wiele danych w prezentacji ({ $count }); nieodczytane: { $dropped }
issue-data-base-missing = dane „{ $data }”: brak pliku bazowego, zachowane tylko do odczytu
issue-data-edits-lost = dane „{ $data }”: zmiany w zastąpionej tabeli zostały utracone przy scalaniu
issue-data-inline-dropped = Element { $element }: starsza wersja zapisała dane jego wykresu lub tabeli w jego opcjach; zostały usunięte
issue-data-kind-unknown = dane „{ $data }”: nieznany rodzaj „{ $kind }”, może z nowszej wersji; nie odczytano
issue-data-query-cycle = dane „{ $data }”: ich zapytanie odczytuje samo siebie, bezpośrednio lub przez inne zapytania; nie obliczono
issue-data-query-missing-input = dane „{ $data }”: ich zapytanie odczytuje „{ $input }”, czego nie ma w prezentacji; nie obliczono
issue-data-query-invalid = dane „{ $data }”: ich zapytanie jest nieprawidłowe ({ $reason }); nie obliczono
issue-data-query-failed = dane „{ $data }”: ich zapytanie nie powiodło się ({ $reason }); brak wyniku
issue-more = pozostałe niewyświetlone problemy: { $count }
op-data-deck-full = Dane prezentacji przekroczyłyby swój limit: najwyżej { $max } { $what ->
    [row-ids] identyfikatorów wierszy
    [overlay] zmienionych komórek
    [text] bajtów tekstu
   *[data] danych
}
op-data-missing = Nie znaleziono danych „{ $data }”
op-data-name-taken = Nazwa danych „{ $name }” jest już zajęta
op-data-column-in-use = Kolumnę „{ $column }” pokazują { $views }; najpierw powiąż je z innymi kolumnami (bind_data z columns)
op-data-in-use = Te dane pokazują { $views }{ $more ->
    [0] {""}
    [one] {" "}i { $more } inny widok
    [few] {" "}i { $more } inne widoki
    [many] {" "}i { $more } innych widoków
   *[other] {" "}i { $more } innego widoku
}; najpierw je usuń lub powiąż z innymi danymi
op-data-column-missing = Dane „{ $data }” nie mają kolumny „{ $column }”
op-data-not-view = Parametr „{ $param }” nie może pokazywać danych: potrafi to tylko opcja wykresu lub tabeli
op-data-truncated = Dane „{ $data }” zostały obcięte przy odczycie i są tylko do odczytu; aby je edytować, zastąp całą tabelę
op-data-rebind = Element { $subject } pokazuje już inne dane; użyj bind_data, aby je zmienić
op-data-internal = Partia nie może ustawić bazy ani nakładki tabeli
op-data-batch-too-large = Partia jest za duża: liczba komórek { $count }, najwyżej { $max }
op-data-inline = Element { $subject } przenosi dane w swoich opcjach; dodaj je jako dane decku przez add_data i powiąż przez bind_data
op-data-derived = dane „{ $data }” pochodzą z zapytania: edytuj zapytanie, nie wiersze
op-data-source = Dane mają tabelę albo zapytanie (sql), dokładnie jedno z nich
op-data-cycle = Dane „{ $data }” odczytywałyby same siebie przez swoje zapytania; usuń pętlę
op-data-unknown-input = Zapytanie odczytuje „{ $name }”, czego nie ma wśród danych prezentacji
op-data-not-query = Dane „{ $data }” nie są zapytaniem; edytuj ich tabelę przez edit_data
op-data-read-by-queries = Te dane odczytują zapytania { $queries }{ $shown ->
    [0] {""}
   *[other] {" "}i wyświetlają { $views }
}{ $more ->
    [0] {""}
   *[other] {" "}oraz jeszcze { $more }
}; najpierw je edytuj lub usuń
op-data-rename-captured = Zapytanie danych „{ $reader }” już nazywa jedną ze swoich tabel „{ $name }”; zmień nazwę tej tabeli w zapytaniu (set_query) lub wybierz inną nazwę
op-data-name-ambiguous = Kilka danych nosi nazwę „{ $name }”, a zapytania czytają tę nazwę i nie wiadomo, które; najpierw zmień te zapytania (set_query)
issue-unreadable-slide = nieczytelny slajd, odłożony
issue-unreadable-field = nieczytelne pole { $field } prezentacji, zresetowane
issue-knowledge-unreadable = nieczytelny wpis wiedzy { $id }, odłożony
issue-knowledge-entries-dropped = wpisy wiedzy odrzucone bez odczytu: { $count } ({ $cause ->
        [files] zbyt wiele plików do odczytu
        [file-bytes] plik zbyt duży do odczytu
        [total-bytes] zbiór zbyt duży do odczytu
       *[unreadable] kolejne nieczytelne wpisy
    })
issue-knowledge-over-limit = zbiór wiedzy przekracza limit: { $limit ->
        [knowledge-reason-too-many-files] { knowledge-reason-too-many-files }
        [knowledge-reason-file-too-large] { knowledge-reason-file-too-large }
       *[knowledge-reason-total-too-large] { knowledge-reason-total-too-large }
    } (pliki ponad limit: { $count }, pierwszy { $id }); wszystkie pliki są zachowane; tylko usunięcie plików sprowadzi paczkę z powrotem poniżej limitu (do tego czasu zapisy i edycje są odrzucane)
issue-knowledge-path-invalid = plik wiedzy { $id } przeniesiony do /recovered: { $reason ->
        [knowledge-path-empty] { knowledge-path-empty }
        [knowledge-path-not-absolute] { knowledge-path-not-absolute }
        [knowledge-path-bad-segment] { knowledge-path-bad-segment }
        [knowledge-path-dot-segment] { knowledge-path-dot-segment }
        [knowledge-path-reserved] { knowledge-path-reserved }
        [knowledge-path-too-deep] { knowledge-path-too-deep }
        [knowledge-path-too-long] { knowledge-path-too-long }
       *[knowledge-path-not-markdown] { knowledge-path-not-markdown }
    }
issue-knowledge-path-collision = kilka plików wiedzy było pod { $path }: pozostałym zmieniono nazwy
issue-knowledge-invalid = plik wiedzy { $path } jest nieprawidłowy: { $reason ->
        [knowledge-frontmatter-missing] { knowledge-frontmatter-missing }
        [knowledge-frontmatter-unterminated] { knowledge-frontmatter-unterminated }
        [knowledge-frontmatter-too-large] { knowledge-frontmatter-too-large }
        [knowledge-frontmatter-not-yaml] { knowledge-frontmatter-not-yaml }
        [knowledge-frontmatter-not-mapping] { knowledge-frontmatter-not-mapping }
        [knowledge-frontmatter-duplicate-key] { knowledge-frontmatter-duplicate-key }
        [knowledge-frontmatter-alias] { knowledge-frontmatter-alias }
        [knowledge-frontmatter-tag] { knowledge-frontmatter-tag }
        [knowledge-frontmatter-too-deep] { knowledge-frontmatter-too-deep }
        [knowledge-frontmatter-after-document-end] { knowledge-frontmatter-after-document-end }
        [knowledge-frontmatter-flow-mapping] { knowledge-frontmatter-flow-mapping }
        [knowledge-frontmatter-type-missing] { knowledge-frontmatter-type-missing }
        [knowledge-frontmatter-bad-timestamp] { knowledge-frontmatter-bad-timestamp }
       *[knowledge-frontmatter-not-allowed] { knowledge-frontmatter-not-allowed }
    }
op-invalid-chart = Wykres { $subject }: { $reason }
op-invalid-table = Tabela { $subject }: { $reason }

replica-corrupted = Historia prezentacji w tym pliku jest uszkodzona
replica-too-large = Historia prezentacji w tym pliku jest zbyt duża, aby ją otworzyć
replica-write = Nie udało się zaktualizować historii prezentacji
replica-unrelated = Te dwie prezentacje nie mają wspólnego pochodzenia
replica-unsupported-version = Ta prezentacja używa formatu, którego ta wersja Oku Slides nie potrafi odczytać
replica-too-deep = Nie można zapisać tej prezentacji: usunięte elementy są zbyt głęboko zagnieżdżone w jej historii. Cofnij ostatnie przeniesienia, a następnie zapisz ponownie.

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
widget-fault-column-type = Widżet { $widget }: kolumna listy może być typu plain, rich, number, flag, choice lub asset (lub zagnieżdżona p.list, jeden poziom); dla dowolnej zawartości pozycji użyj p.slot z widżetem potomnym
widget-fault-reserved-name = Widżet { $widget }: zarezerwowana nazwa: id, kind lub nazwa zawierająca /
widget-fault-list-bounds = Widżet { $widget }: maksimum listy jest poniżej jej minimum
widget-fault-list-default-too-large = Widżet { $widget }: wartość domyślna listy przekracza limity rozmiaru list
widget-fault-list-depth = Widżet { $widget }: zagnieżdżona lista nie może zawierać kolumny listy; listy zagnieżdżają się tylko na jednym poziomie
widget-fault-list-default-below-min = Widżet { $widget }: wartość domyślna zagnieżdżonej listy ma mniej pozycji niż jej minimum
widget-fault-slide-sizing = Widżet { $widget } jest slajdem: tylko widżety elementów deklarują grow lub typography
widget-fault-pack-category = Widżet { $widget } nie pasuje do swojego pakietu: pakiety układów zawierają widżety slajdów, pakiety widżetów zawierają widżety elementów
widget-fault-unqualified = Widżet { $widget } nie ma przestrzeni nazw pakietu
widget-fault-too-long = Widżet { $widget }: nazwa, etykieta, wartość, opis lub lista przekracza limit
issue-empty-item-id = lista { $param } ma pozycję bez identyfikatora
issue-duplicate-item-id = lista { $param } ma zduplikowany identyfikator pozycji
issue-invalid-item-id = lista { $param } ma identyfikator pozycji zawierający ukośnik
issue-list-bounds = lista { $param } ma za mało lub za dużo pozycji
issue-view-truncated = { $element } pokazuje tylko część swoich danych ({ $what ->
    [rows] wiersze
   *[columns] kolumny
}: { $shown } z { $total })
issue-render-budget = { $element } nie jest rysowany: widoki prezentacji czytają więcej danych, niż pozwala renderowanie
issue-data-missing = { $element } wskazuje dane „{ $data }”, których nie ma w prezentacji
issue-view-column-missing = { $element } czyta kolumnę „{ $column }”, której nie ma w jego danych
issue-view-unreadable = { $element } nie jest rysowany: nie można odczytać jego opcji
op-replica-full = Historia prezentacji zbliża się do { $max } MiB, maksimum pozwalającego ją ponownie otworzyć: usuń zawartość, zanim dodasz więcej.
