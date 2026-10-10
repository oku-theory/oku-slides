# Interface de l’app (React). Chargé par ui/src/i18n.ts via la requête « messages ».
ui-loading = Wczytywanie…

## Dokument

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · szablony
ui-title-rename = Zmień nazwę prezentacji
ui-title-field = Tytuł prezentacji
ui-status-unsaved = niezapisane zmiany
ui-status-saved = zapisano
ui-status-never-saved = jeszcze nie zapisano
ui-caption-templates = { $count ->
    [one] { $count } szablon
    [few] { $count } szablony
    [many] { $count } szablonów
   *[other] { $count } szablonu
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } slajd
    [few] { $count } slajdy
    [many] { $count } slajdów
   *[other] { $count } slajdu
} · { $status }
ui-template-default-name = Szablon
ui-template-variant-name = { $name } (wariant)
ui-confirm-quit-templates = Zakończyć bez zapisywania zmienionych szablonów?

## Powiadomienia

ui-toast-templates-saved = Szablony zapisane: { $name }
ui-toast-moved = Prezentacja przeniesiona
ui-toast-copied = Kopia zapisana
ui-toast-select-zone = Zaznacz obszar, aby umieścić: „{ $text }”
ui-toast-field-gone = Edytowane pole już nie istnieje: wpisany tekst został utracony.

## Pasek narzędzi

ui-toolbar-undo = Cofnij
ui-toolbar-undo-label = Cofnij: { $label }
ui-toolbar-redo = Ponów
ui-toolbar-redo-label = Ponów: { $label }
ui-toolbar-grid-hide = Ukryj siatkę
ui-toolbar-grid-show = Pokaż siatkę
ui-toolbar-ghosts-hide = Ukryj duchy
ui-toolbar-ghosts-show = Pokaż duchy
ui-divider-pile = Szerokość stosu slajdów
ui-divider-notes = Szerokość wątku agenta
ui-divider-code = Rozmiar kodu
ui-divider-band = Wysokość ustawień
ui-toolbar-theme = Motyw
ui-toolbar-theme-embedded = Motyw z pliku
ui-toolbar-save-copy = Zapisz kopię
ui-toolbar-finish = Gotowe
ui-menu-workshop = Warsztat
ui-toolbar-open = Otwórz
ui-merge = Scal z plikiem PDF…
ui-menu = Menu
ui-menu-move = Przenieś…
ui-language = Język
ui-language-system = Język systemu
ui-scheme = Wygląd
ui-scheme-system = Systemowy
ui-scheme-light = Jasny
ui-scheme-dark = Ciemny
ui-toolbar-add-label = Dodaj do slajdu
ui-add-kind = { $kind ->
    [card] Karta
    [shape] Kształt
    [chart] Wykres
    [table] Tabela
   *[text] Tekst
}
ui-add-widget = Widżet pakietu
ui-toolbar-remove-template = Usuń szablon
ui-toolbar-remove-slide = Usuń slajd
ui-toolbar-remove-hint = Slajd { $n } · cofnij za pomocą ⌘Z

## Etykiety cofania

ui-undo-add = { $kind ->
    [card] Dodaj kartę
    [shape] Dodaj kształt
    [chart] Dodaj wykres
    [table] Dodaj tabelę
   *[text] Dodaj tekst
}
ui-undo-add-widget = Dodaj „{ $name }”
ui-undo-new-template = Nowy szablon: { $name }
ui-undo-new-slide = Nowy slajd: { $name }
ui-undo-templates = Szablony: { $name }
ui-undo-move-template = Przenieś szablon
ui-undo-move-slide = Przenieś slajd
ui-undo-remove-slide = Usuń slajd
ui-undo-remove = Usuń
ui-undo-cut = Wytnij
ui-undo-paste = Wklej
ui-undo-place-on-grid = Umieść na siatce
ui-undo-move-on-grid = Przenieś na siatce
ui-undo-resize-on-grid = Zmień rozmiar na siatce
ui-undo-change-anchor = Zmień zakotwiczenie
ui-undo-move = Przenieś
ui-undo-resize = Zmień rozmiar
ui-undo-edit-slide = Edytuj slajd
ui-undo-dictation = Dyktowanie
ui-undo-rename-set = Zmień nazwę zestawu
ui-undo-rename-deck = Zmień nazwę prezentacji
ui-undo-chart-kind = Zmień typ wykresu
ui-undo-chart-data = Edytuj dane
ui-undo-chart-color = Pokoloruj serię
ui-undo-chart-stack = { $stacked ->
    [yes] Skumuluj serie
   *[no] Rozdziel serie
}

## Nowe elementy

ui-new-text = Nowy tekst
ui-new-card-title = Tytuł karty
ui-new-card-detail = Szczegóły

## Stos slajdów

ui-pile-slides = Slajdy
ui-pile-templates = Szablony
ui-pile-add-slide = + Nowy slajd
ui-pile-add-template = + Nowy szablon
ui-pile-drag-hint = Przeciągnij, aby przenieść · ⌥↑ ⌥↓
ui-slide-alt = Slajd { $n }

## Scena

ui-stage-rendering = Renderowanie…
ui-stage-elements = Elementy slajdu
ui-stage-fields = Pola slajdu
ui-stage-edit-text = Edytuj tekst
ui-stage-hint-plain = Enter lub Tab, aby przejść dalej · Esc, aby anulować
ui-stage-hint-rich = Tab, aby przejść dalej · ⌘ Enter, aby zatwierdzić · „- ” dla punktu listy · ⌘B pogrubienie, ⌘I kursywa, ⌘K link
ui-field = { $kind ->
    [title] Tytuł
    [subtitle] Podtytuł
    [kicker] Nadtytuł
    [text] Tekst
    [body] Treść
    [author] Autor
    [date] Data
    [organization] Organizacja
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Tekst
    [card] Karta
    [image] Obraz
    [shape] Kształt
    [group] Grupa
   *[other] { $kind }
}
ui-grid-cell = kol. { $cols } · wiersz { $rows }
ui-ghost-zone = Strefa

## Notatki slajdu

ui-notes = Narzędzia slajdu
ui-stepper-less = { $label }: o jeden mniej
ui-stepper-more = { $label }: o jeden więcej
ui-list-item = element
ui-list-add = Dodaj { $item }
ui-list-remove = Usuń { $item }
ui-list-move = Przenieś { $item }
ui-list-full = Lista jest pełna ({ $max })
ui-undo-list-move = Przenieś { $item }
ui-agent-color-other = Inny kolor
ui-agent-thread = Agent
ui-agent-new-conversation = Nowa rozmowa
ui-agent-detach = Otwórz agenta w osobnym oknie
ui-agent-attach = Przywróć agenta do głównego okna
ui-agent-window-empty = Brak agenta dla tej karty
ui-agent-kept-in-draft = Brak miejsca do wpisania: tekst zachowano w szkicu
ui-agent-busy-elsewhere = Agent odpowiada w: { $place }…
ui-agent-scope-slides = slajdy
ui-agent-scope-atelier = pracownia
ui-draft = Szkic
ui-draft-insert = Wstaw
ui-draft-insert-title = Wstaw bez zmian (⌘⇧↩)
ui-draft-send = Wyślij do agenta
ui-draft-send-title = Wyślij do agenta (⌘↩)
ui-draft-discard = Odrzuć
ui-agent-state-idle = Agent
ui-agent-state-starting = Agent się uruchamia…
ui-agent-state-working = Agent pracuje…
ui-agent-dock = W toku
ui-agent-dock-settings = Ustawienia
ui-agent-jump = ↓ Bieżąca tura
ui-agent-sheet = Pytanie agenta
ui-agent-sheet-keys = 1–4 wybierz · Enter zatwierdź · E popraw · Esc anuluj
ui-agent-variants = Warianty
ui-agent-drawer-keys = Kliknij lub 1–4 · Esc zamyka
ui-agent-confirm = Potwierdzenie agenta

## Dyktowanie

ui-dictation = Dyktowanie
ui-dictation-start = Dyktuj
ui-dictation-start-title = Dyktuj (⌘D)
ui-dictation-type = Napisz do agenta
ui-dictation-type-title = Napisz do agenta (⌘K)
ui-dictation-agent-stop = Zatrzymaj agenta (⌘⇧D lub Esc)
ui-dictation-agent-working = Agent pracuje…
ui-dictation-agent-working-hint = ⌘⇧D lub Esc go zatrzymuje
ui-dictation-finish = Zakończ dyktowanie
ui-dictation-finish-title = Zakończ (⌘D)
ui-dictation-cancel = Anuluj
ui-dictation-preparing = Przygotowywanie dyktowania…
ui-dictation-heard-nothing = Nic nie usłyszano: mów nieco bliżej mikrofonu
ui-dictation-to-cursor = w miejscu kursora
ui-dictation-to-draft = do szkicu
ui-dictation-to-field = zastąp: { $field }
ui-dictation-to-block = po zaznaczonym bloku
ui-dictation-new-block = nowy blok tekstu
ui-dictation-select-zone = zaznacz obszar
ui-dictation-downloading = Pobieranie modelu { $name }
ui-dictation-download-progress = { $percent }% · { $received } z { $total } · tylko raz
ui-dictation-listening = Słuchanie · { $time }
ui-dictation-transcribing = Transkrypcja ostatniego zdania…
ui-dictation-stop-in = Zatrzymanie za { $duration }
ui-dictation-speak = Mów, tekst pojawi się tutaj…
ui-dictation-auto-stop = zatrzymuje się po { $duration } ciszy
ui-dictation-to-download = { $size } do pobrania
ui-dictation-settings = Ustawienia dyktowania
ui-dictation-model-title = Model rozpoznawania
ui-dictation-model-intro = Whisper, na tym komputerze (CTranslate2). Każdy model rozumie wiele języków.
ui-dictation-note-turbo = Zalecany, szybki i dokładny
ui-dictation-note-large = Najdokładniejszy, wolniejszy
ui-dictation-note-base = Lekki, dla słabszych komputerów
ui-dictation-installed = zainstalowany
ui-dictation-not-installed = do pobrania
ui-dictation-language = Język mówiony
ui-dictation-language-auto = Wykrywanie automatyczne
ui-dictation-note-apple = Najszybszy, wbudowany w system
ui-dictation-model-unavailable = Język nieobsługiwany
ui-dictation-model-intro-apple = Apple, wbudowany w system, lub Whisper, na tym komputerze (CTranslate2). Wszystkie rozumieją wiele języków.
ui-dictation-language-auto-resolved = Automatycznie ({ $language })
ui-dictation-to-download-language = Język do pobrania
ui-dictation-auto-stop-label = Zatrzymaj automatycznie po ciszy trwającej
ui-dictation-silence = Czas trwania ciszy
ui-dictation-phrases-note = Transkrybowane są tylko fragmenty z mową, zdanie po zdaniu, w trakcie mówienia.
ui-dictation-download-now = Pobierz teraz ({ $size })
ui-dictation-download-language = Pobierz język
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Wybór szablonu

ui-picker-new-slide = Nowy slajd
ui-picker-new-template = Nowy szablon na podstawie…
ui-picker-set = Szablony
ui-picker-set-mine = { $name } · moje
ui-picker-keys = ← →, aby przeglądać · Enter, aby dodać · Esc, aby zamknąć
ui-picker-close = Zamknij
ui-picker-previous = Poprzedni szablon
ui-picker-next = Następny szablon
ui-picker-list = Szablony slajdów

## Wykres

ui-chart-panel = Wykres
ui-chart-kinds = Typ wykresu
ui-chart-kind = { $kind ->
    [line] Liniowy
    [pie] Kołowy
    [scatter] Punktowy
    [radar] Radarowy
    [waterfall] Kaskadowy
   *[bar] Słupkowy
}
ui-chart-no-data = Ten wykres nie ma tabeli danych.
ui-chart-options = Opcje wykresu
ui-chart-stack = Skumulowany
ui-chart-waterfall-hint = Zaznacz Suma, aby uzyskać słupek sumy.
ui-chart-colors = Kolory serii
ui-chart-color-of = Kolor serii { $name }
ui-chart-color-auto = Automatycznie
ui-chart-color-pick = Kolor { $n }
ui-chart-column-name = Nazwa kolumny { $col }
ui-chart-remove-column = Usuń kolumnę
ui-chart-cell = Wiersz { $row }, kolumna { $col }
ui-chart-remove-row = Usuń wiersz
ui-chart-add-row = Wiersz
ui-chart-add-column = Kolumna
ui-chart-series = Seria { $n }

## Tabela

ui-table-panel = Tabela
ui-table-options = Opcje tabeli
ui-table-header-row = Wiersz nagłówka
ui-table-header-column = Pierwsza kolumna jako nagłówek
ui-table-striped = Wiersze w paski
ui-table-alignment = Wyrównanie kolumn
ui-table-align-of = Wyrównanie kolumny { $name }
ui-table-align = { $align ->
    [start] Do początku
    [center] Do środka
    [end] Do końca
   *[auto] Automatyczne
}
ui-table-column = Kolumna { $n }
ui-table-no-data = Ta tabela nie zawiera danych.
ui-undo-table-option = Opcje tabeli
ui-undo-table-align = Wyrównaj kolumnę
ui-undo-table-data = Edytuj tabelę
ui-show-as = Pokaż jako
ui-show-as-table = Pokaż jako tabelę
ui-show-as-chart = Pokaż jako wykres
ui-undo-show-as-table = Pokaż jako tabelę
ui-undo-show-as-chart = Pokaż jako wykres

## Pliki

ui-file-open-prompt = Ścieżka pliku do otwarcia (.oku-slides.pdf lub .json)
ui-file-save-prompt = Zapisz jako (pełna ścieżka)

## Karty

ui-tabs = Otwarte prezentacje
ui-tab-new = Nowa karta
ui-tab-close = Zamknij kartę
ui-tab-unsaved = Niezapisane zmiany
ui-tab-templates = Szablony
ui-tab-move-new-window = Przenieś do nowego okna
ui-tab-move-to = Przenieś do okna { $n }
ui-tab-close-others = Zamknij pozostałe karty
ui-close-cancel = Anuluj
ui-disk-replaced-question = { $file } został zastąpiony na dysku inną prezentacją.
ui-disk-replaced-overwrite = Nadpisz
ui-disk-replaced-save-as = Zapisz jako…
ui-same-deck-question = { $file } to wersja prezentacji otwartej na karcie „{ $title }”.
ui-same-deck-merge = Scal w tej karcie
ui-same-deck-apart = Otwórz osobno
ui-history = Historia
ui-history-version = Wersja z { $date }
ui-history-restore = Przywróć
ui-history-back = Wróć
ui-history-anonymous = Anonim
ui-history-slider = Wersje prezentacji
ui-history-removed-since = usunięty od tego czasu
ui-history-changed-since = zmieniony od tego czasu
ui-history-added-since = slajdy dodane od tego czasu: { $count }
ui-help = Pomoc
ui-help-caption = Slajd { $index } z { $count } · tylko do odczytu
ui-slide-previous = Poprzedni slajd
ui-slide-next = Następny slajd
ui-present = Prezentacja
ui-present-slide = Slajd { $index } z { $count }
ui-present-screen = Prezentuj na pełnym ekranie
ui-present-window = Prezentuj w oknie
ui-toolbar-params-hide = Ukryj ustawienia
ui-toolbar-params-show = Pokaż ustawienia
ui-help-close = Zamknij pomoc
ui-help-search = Szukaj w pomocy
ui-help-search-placeholder = Szukaj…
ui-help-search-empty = Brak wyników
ui-help-search-count = Wyniki: { $count }
ui-pack-notice-close = Zamknij
ui-widgets-files = Pliki widżetów
ui-widgets-common = Wspólne
ui-widgets-new = Nowy widget
ui-widgets-name = Nazwa widżetu (małe litery, cyfry i łączniki)
ui-widgets-name-taken = Ta nazwa jest już zajęta
ui-widgets-name-letter = Nazwa musi zaczynać się od litery
ui-widgets-valid = Poprawny
ui-widgets-errors = Błędy
ui-widgets-empty = Ten pakiet nie ma jeszcze widżetów.
ui-widgets-preview = Podgląd widżetu
ui-widgets-no-preview = Brak podglądu
ui-widgets-field-bound = Powiązane z prezentacją: tu tylko do odczytu
ui-widgets-source = Kod źródłowy
ui-widgets-unsaved = Nie zapisano
ui-widgets-in-error = Widgety z błędami
ui-widgets-none-in-error = Żaden widget nie ma błędów
ui-widgets-section = Widgety
ui-widgets-open = Otwórz widżet { $name }
ui-widgets-delete-named = Usuń { $name }
ui-widgets-confirm-delete = Usunąć widget „{ $name }”?
ui-widgets-rename-hint = Kliknij dwukrotnie lub naciśnij F2, aby zmienić nazwę
ui-widgets-tabs = Karty szablonów
ui-widgets-templates-tab = Szablony
ui-theme-tab = Motyw
ui-theme-picker = Motyw
ui-theme-name = Nazwa motywu
ui-theme-unsaved = Niezapisane zmiany motywu
ui-widgets-close-tab = Zamknij { $name }
ui-widgets-removed-by-agent = Agent usunął widżet { $name }.
