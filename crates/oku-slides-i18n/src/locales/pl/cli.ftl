cli-reading = odczyt { $path }
cli-invalid-deck = nieprawidłowa prezentacja: { $path }
cli-deck-valid = prawidłowa prezentacja: slajdy: { $slides }
cli-deck-issue = problem: { $issue }
cli-rendered = strony: { $pages } → { $out } (kompilacja { $compile }, SVG { $svg })
cli-asset-added = zasób { $path } → { $id }
cli-saved = { $path }: strony: { $pages }, PDF { $pdf } KB, źródła { $sources } KB
cli-opened = { $path }: slajdy: { $slides }, zasoby: { $assets }, czcionki: { $fonts }
cli-extracted = wyodrębniono do { $dir }
cli-stripped = { $path }: kopia bez źródeł
cli-pdf-modified = plik PDF został zmieniony innym narzędziem po zapisaniu w Oku Slides
cli-iterations = --iterations musi być > 0
cli-no-block = prezentacja nie ma bloku do edycji
cli-blob-missing = nie znaleziono bloba źródeł
cli-roundtrip-mismatch = ponownie odczytana prezentacja różni się od zapisanej
cli-bench-world = budowanie środowiska (czcionki)  { $time }
cli-bench-first = pierwsza kompilacja              { $time }  (strony: { $pages })
cli-bench-same = rekompilacja bez zmian          { $time }
cli-bench-edited-text = Edytowany tekst { $n }
cli-bench-edit = rekompilacja po set_text        { $time }
cli-bench-svg = SVG każdego slajdu              { $time }  ({ $each } na slajd)
cli-bench-pdf = eksport PDF                     { $time }  ({ $size } KB)
cli-bench-embed = osadzanie bloba źródeł          { $time }
cli-bench-extract = ponowny odczyt bloba            { $time }
cli-bench-roundtrip = pełny obieg prezentacji         identyczna
cli-bench-history = krok historii                    { $mean } ms średnio, p50 { $p50 } ms
cli-serving = Oku Slides (dev) pod adresem http://{ $host }:{ $port }
cli-serving-token = otwórz { $url } (token jest ważny do zatrzymania serwera)
cli-models-dir = modele dyktowania: { $dir }
cli-stopped = serwer zatrzymany
cli-not-found = nie znaleziono
cli-bad-request = nieprawidłowe żądanie: { $reason }
cli-sse-full = zbyt wiele otwartych strumieni zdarzeń
cli-forbidden = żądanie odrzucone: nie pochodzi z lokalnego interfejsu
cli-unauthorized = żądanie odrzucone: brak tokenu dostępu lub jest błędny
cli-exposed = nasłuchiwanie na { $host }: serwer jest dostępny z sieci; obsługiwani są tylko klienci, którzy przedstawią token tego uruchomienia. Zostaw [serve] host na 127.0.0.1, jeśli nie potrzebujesz sieci
cli-imported = { $path }: { $slides ->
       [one] { $slides } slajd
       [few] { $slides } slajdy
       [many] { $slides } slajdów
      *[other] { $slides } slajdu
    }, { $pages ->
       [one] { $pages } strona
       [few] { $pages } strony
       [many] { $pages } stron
      *[other] { $pages } strony
    }, { $losses ->
       [one] { $losses } strata
       [few] { $losses } straty
       [many] { $losses } strat
      *[other] { $losses } straty
    }, { $requests ->
       [one] { $requests } prośba o widżet
       [few] { $requests } prośby o widżet
       [many] { $requests } próśb o widżet
      *[other] { $requests } prośby o widżet
    }
cli-import-write-failed = nie udało się zapisać zaimportowanej prezentacji { $path }
cli-import-hifi-no-target = --pack i --theme dotyczą tylko trybu retrofit: import o wysokiej wierności (--mode hifi) tworzy własny pakiet i motyw
cli-import-reference-hifi-only = --reference należy do trybu hifi: import retrofit nie jest porównywany z renderowaniem referencyjnym
cli-import-scored = Różnica wizualna względem { $witness }: średnio { $mean }, { $above ->
    [0] żaden slajd
    [one] { $above } slajd
    [few] { $above } slajdy
   *[other] { $above } slajdów
} powyżej { $threshold }
cli-import-unscored = { $reason }

cli-about = Renderowanie i zapisywanie prezentacji Oku Slides
cli-arg-fonts = Dodatkowy folder czcionek (TTF/OTF); domyślnie: paths.fonts w ustawieniach
cli-cmd-check = Sprawdź prezentację
cli-arg-deck = Prezentacja (JSON)
cli-cmd-render = Renderuj każdy slajd do SVG, z ramkami elementów
cli-arg-input = Plik deck.json lub .oku-slides.pdf
cli-arg-out = Folder wyjściowy
cli-arg-theme = Zastąp motyw prezentacji: wbudowany lub z biblioteki, wskazany nazwą
cli-cmd-save = Zapisz prezentację jako .oku-slides.pdf (PDF + blob źródeł)
cli-arg-output = Plik do zapisania
cli-arg-assets = Folder zasobów: każdy plik staje się zasobem sha256:…
cli-arg-embed-fonts = Osadź w źródłach także czcionki z --fonts
cli-cmd-open = Otwórz ponownie plik .oku-slides.pdf: wyodrębnij prezentację i jej zasoby
cli-arg-extract = Folder, do którego zostaną zapisane deck.json, assets/ i fonts/
cli-cmd-strip = Skopiuj plik .oku-slides.pdf bez bloba źródeł
cli-cmd-bench = Zmierz czasy kompilacji i eksportu
cli-arg-iterations = Liczba pomiarów na krok
cli-cmd-schema = Wypisz schemat JSON operacji
cli-cmd-serve = Serwer deweloperski: interfejs w przeglądarce, bez Tauri
cli-arg-serve-input = Prezentacja do otwarcia (.oku-slides.pdf lub .json)
cli-arg-port = Port; domyślnie: serve.port w ustawieniach
cli-arg-ui = Folder zbudowanego interfejsu (ui/dist)
cli-cmd-import = Zaimportuj prezentację PowerPoint (.pptx) jako .oku-slides.pdf
cli-arg-pptx = Prezentacja PowerPoint (.pptx)
cli-arg-mode = retrofit: treść na istniejącym pakiecie i motywie; hifi: wierna kopia, na pakiecie i motywie wygenerowanych dla niej
cli-arg-pack = Pakiet układów dla retrofit (przestrzeń nazw, identyfikator lub folder); można powtarzać, domyślnie essentiel
cli-arg-import-theme = Motyw docelowy: wbudowany lub z biblioteki, wskazany identyfikatorem lub nazwą; domyślnie: motyw domyślny
cli-arg-report = Zapisz też raport importu (JSON): straty, zamienniki, pokrycie
cli-arg-no-system-fonts = W trybie hifi nie szukaj czcionek prezentacji wśród czcionek systemowych
cli-arg-reference = W trybie hifi porównać każdy slajd z renderowaniem referencyjnym prezentacji: auto (domyślnie: [import] witness, w przeciwnym razie office2pdf), office2pdf, soffice (LibreOffice) lub nie (none)
