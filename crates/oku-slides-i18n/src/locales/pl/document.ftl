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
registry-schema-remote = Nieprawidłowy rejestr widżetów: { $message }
probe-not-probed = Jeszcze niesprawdzony: skończył się czas przeznaczony na widżety pakietu. Zostanie sprawdzony przy następnym renderowaniu.
probe-too-many-files = Pakiet ma więcej niż { $max } plików widżetów: ten nie jest wczytywany.
probe-too-large = Ponad { $max } KiB: nie wczytano.

import-unreadable = Nie można odczytać tej prezentacji
import-not-a-presentation = Ten plik nie jest prezentacją PowerPoint
import-protected = Ta prezentacja jest zaszyfrowana lub w starym formacie .ppt: najpierw zapisz ją jako .pptx
import-too-large = Ta prezentacja jest zbyt duża, aby ją zaimportować
import-loss-list-levels = Poziomy listy spłaszczone do jednego
import-loss-merged-cells = Scalone komórki rozdzielone
import-loss-percent-stacked = Wykres skumulowany 100% pokazany jako skumulowany
import-loss-run-style = { $effect ->
    [underline] Utracono podkreślenie
    [strike] Utracono przekreślenie
    [baseline] Utracono indeks górny lub dolny
    [caps] Utracono wielkie litery
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
import-warning-effect-ignored = Nie zaimportowano efektu: { $detail }
import-warning-approximated = Odwzorowanie przybliżone: { $detail }
import-warning-unreadable-item = Pominięto nieczytelny element: { $detail }
import-warning-unreadable-part = Nieczytelna część: { $detail }
import-warning-missing-part = Brakująca część: { $detail }
import-warning-unsupported-chart = Nieobsługiwany wykres: { $detail }
import-warning-unsupported-image = Nieobsługiwany format obrazu: { $detail }
import-warning-unknown-table-style = Nieznany styl tabeli: { $detail }
import-warning-unsafe-link = Odrzucono łącze: { $detail }
import-warning-nesting-too-deep = Zbyt głębokie zagnieżdżenie, obcięto: { $detail }
import-warning-font-embedded-unreadable = Nieczytelna osadzona czcionka: { $detail }
import-warning-truncated = Zbyt wiele ostrzeżeń: kolejne pominięto
