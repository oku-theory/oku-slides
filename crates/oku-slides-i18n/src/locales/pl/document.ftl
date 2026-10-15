pdf-unreadable = Nieczytelny plik PDF
pdf-unsupported-version = Ten plik PDF został zapisany nowszą wersją Oku Slides
pdf-corrupted = Źródła w tym pliku PDF są uszkodzone
pdf-write = Nie udało się zapisać pliku PDF

render-compile = Nie udało się wyrenderować prezentacji
render-pdf = Eksport do PDF nie powiódł się
render-no-page = Ta strona nie istnieje
render-image = Nie udało się wygenerować obrazu slajdu

render-setup = Nie udało się uruchomić renderera
render-missing-image = brakujący obraz
diagnostic-error = błąd
diagnostic-warning = ostrzeżenie
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = wskazówka: { $hint }
render-chart-failed = nie udało się narysować wykresu

registry-bad-pack-id = Nieprawidłowy identyfikator pakietu szablonów: { $id }
registry-compile = Nie udało się skompilować widżetów pakietu szablonów
registry-missing = Pakiet szablonów nie deklaruje swoich widżetów
render-timeout = Renderowanie trwało dłużej niż { $seconds } s i zostało przerwane
render-out-of-memory = Renderowanie wymagało więcej niż { $mib } MiB pamięci i zostało przerwane
render-worker-lost = Proces renderowania został zatrzymany
render-frame = Nieczytelny komunikat z procesu renderowania
render-page-too-large = Ten slajd jest za duży na podgląd (ponad { $mib } MiB, często z powodu bardzo dużego obrazu). Nadal można go wyeksportować do PDF lub PNG.
render-stale = Renderowanie zostało zastąpione; spróbuj ponownie
render-pack-disabled = Widżety pakietu „{ $pack }” są wyłączone w tej sesji
render-protocol = Proces renderowania pochodzi z innej wersji Oku Slides; uruchom aplikację ponownie
render-remote = Proces renderowania nie mógł wykonać żądania
registry-schema-remote = Nieprawidłowy rejestr widżetów: { $message }
probe-not-probed = Jeszcze niesprawdzony: skończył się czas przeznaczony na widżety pakietu. Zostanie sprawdzony przy następnym renderowaniu.
probe-too-many-files = Pakiet ma więcej niż { $max } plików widżetów: ten nie jest wczytywany.
probe-too-large = Ponad { $max } KiB: nie wczytano.

import-unreadable = Nie można odczytać tej prezentacji
import-not-a-presentation = Ten plik nie jest prezentacją PowerPoint
import-protected = Ta prezentacja jest zaszyfrowana lub w starym formacie .ppt: najpierw zapisz ją jako .pptx
import-too-large = Ta prezentacja jest zbyt duża, aby ją zaimportować
import-output-too-large = Powstała prezentacja przekroczyłaby rozmiar dozwolony dla importu
import-loss-list-levels = Poziomy listy spłaszczone do jednego
import-loss-merged-cells = Scalone komórki rozdzielone
import-loss-percent-stacked = Wykres skumulowany 100% pokazany jako skumulowany
import-loss-run-style = { $effect ->
    [underline] Utracono podkreślenie
    [strike] Utracono przekreślenie
    [baseline] Utracono indeks górny lub dolny
    [caps] Utracono wielkie litery
    [font] Czcionkę fragmentu zastąpiono czcionką pola
    [tracking] Odstępy między literami fragmentu zastąpiono odstępami pola
    [weight] Zwykły fragment pogrubiony jak pole
    [slant] Prosty fragment pochylony jak pole
   *[highlight] Utracono wyróżnienie
}
import-loss-transform = Utracono obrót, odbicie lub przycięcie
import-loss-color-dropped = Kolor { $color } nie ma bliskiego koloru w motywie: użyto koloru motywu
import-loss-text-truncated = Tekst skrócony, aby zmieścić się w limitach
import-loss-table-truncated = Tabela skrócona, aby zmieścić się w limitach
import-loss-opaque = { $reason ->
    [media] Nie zaimportowano wideo ani dźwięku (brak obrazu zastępczego)
    [ole] Nie zaimportowano osadzonego obiektu (brak obrazu zastępczego)
    [math] Nie zaimportowano równania (brak obrazu zastępczego)
    [smartart] Nie zaimportowano grafiki SmartArt (brak obrazu zastępczego)
   *[unknown] Nie zaimportowano nieobsługiwanego obiektu (brak obrazu zastępczego)
}
import-loss-unsupported-image = Nie zaimportowano obrazu w formacie { $format }
import-loss-hidden-slide = Ukryty slajd zaimportowany jako widoczny
import-loss-effects = Utracono cienie, odbicia i efekty 3D
import-loss-series-dropped = Wykres kołowy pokazuje jedną serię: zachowano { $kept }, pominięto { $dropped }
import-loss-repeated-decor = { $slides ->
   [one] Nagłówek lub stopka „{ $text }” powtórzone na { $slides } slajdzie pominięte: motyw rysuje własne
   [few] Nagłówek lub stopka „{ $text }” powtórzone na { $slides } slajdach pominięte: motyw rysuje własne
   [many] Nagłówek lub stopka „{ $text }” powtórzone na { $slides } slajdach pominięte: motyw rysuje własne
  *[other] Nagłówek lub stopka „{ $text }” powtórzone na { $slides } slajdu pominięte: motyw rysuje własne
}
import-loss-no-slot = Treść pominięta: wybrany szablon nie ma na nią miejsca
import-loss-page-fitted = Strona źródłowa { $width } × { $height } pt nie ma proporcji 16:9: treść przeskalowana i wyśrodkowana
import-loss-paragraph-style = Wyrównanie, interlinię lub odstęp między akapitami dopasowano do pola
import-loss-shape-style = { $effect ->
    [gradient] Gradient pokazano jako jednolity kolor
    [picture] Obraz wypełnienia rozciągnięto do ramki kształtu
    [dash] Linię przerywaną pokazano jako ciągłą
    [arrow] Groty strzałek linii nie zostały narysowane
   *[geometry] Nieznany kształt pokazano jako prostokąt
}
import-loss-layout-dropped = Za dużo układów na jeden pakiet: „{ $layout }” nie ma szablonu; jego slajdy używają pustego widżetu i dostają jego dekorację jako elementy
import-loss-decor-truncated = Dekoracja przycięta, by zmieścić się w rozmiarze pliku pakietu: pominięte elementy: { $omitted }
import-loss-table-style = Styl tabeli przybliżony: jej własne tła komórek, obramowania, szerokości kolumn lub rozmiary tekstu są tracone
import-loss-overflow = Element „{ $element }” wychodzi poza krawędź slajdu
import-reference-unavailable = Brak oceny wizualnej: { $cause ->
    [timeout] renderowanie referencyjne trwało zbyt długo
    [memory] renderowanie referencyjne zużyło zbyt dużo pamięci
    [pages] renderowanie referencyjne ma { $reference ->
        [one] { $reference } stronę
        [few] { $reference } strony
        [many] { $reference } stron
       *[other] { $reference } strony
    } na { $deck } w prezentacji
   *[failed] renderer referencyjny nie mógł odczytać prezentacji
}
import-font-substituted = { $family } zastąpiona czcionką { $by } o zgodnych metrykach
import-font-missing = Nie znaleziono { $family }: tekst używa czcionki { $by }
import-font-restricted = { $family }: licencja zabrania osadzania niektórych jej odmian w prezentacji
import-font-restricted-substituted = Nie można osadzić { $family } w prezentacji (licencja): zastąpiona czcionką { $by } o zgodnych metrykach
import-font-restricted-missing = Nie można osadzić { $family } w prezentacji (licencja): tekst używa czcionki { $by }
import-font-too-large = { $family }: pliki zbyt duże, by osadzić je w prezentacji; tekst używa czcionki { $by }
import-font-missing-no-fallback = Nie znaleziono { $family } ani czcionki treści motywu: tekst jest renderowany domyślną czcionką silnika
import-warning-effect-ignored = Nie zaimportowano efektu: { $detail }
import-warning-approximated = Odwzorowanie przybliżone: { $detail }
import-warning-unreadable-item = Pominięto nieczytelny element: { $detail }
import-warning-unreadable-part = Nieczytelna część: { $detail }
import-warning-missing-part = Brakująca część: { $detail }
import-warning-unsupported-chart = Nieobsługiwany wykres: { $detail }
import-warning-unsupported-image = Nieobsługiwany format obrazu: { $detail }
import-warning-oversized-image = Obraz zbyt duży do zaimportowania (liczba pikseli): { $detail }
import-warning-external-image = Obraz połączony spoza pliku, nie zaimportowano: { $detail }
import-warning-unsafe-svg = Obraz SVG oczyszczony z niebezpiecznej treści lub pominięty: { $detail }
import-warning-unknown-table-style = Nieznany styl tabeli: { $detail }
import-warning-unsafe-link = Odrzucono łącze: { $detail }
import-warning-nesting-too-deep = Zbyt głębokie zagnieżdżenie, obcięto: { $detail }
import-warning-font-embedded-unreadable = Nieczytelna osadzona czcionka: { $detail }
import-warning-truncated = Zbyt wiele ostrzeżeń: kolejne pominięto
