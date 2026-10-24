# Interface de l’app (React). Chargé par ui/src/i18n.ts via la requête « messages ».
ui-loading = Wczytywanie…

## Dokument

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · pakiet układów
   *[other] { $name } · pakiet widżetów
}
ui-title-rename = Zmień nazwę prezentacji
ui-title-field = Tytuł prezentacji
ui-status-unsaved = niezapisane zmiany
ui-status-saved = zapisano
ui-status-never-saved = jeszcze nie zapisano
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count } układ
        [few] { $count } układy
        [many] { $count } układów
       *[other] { $count } układu
    }
   *[other] { $count ->
        [one] { $count } widżet
        [few] { $count } widżety
        [many] { $count } widżetów
       *[other] { $count } widżetu
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } slajd
    [few] { $count } slajdy
    [many] { $count } slajdów
   *[other] { $count } slajdu
} · { $status }
ui-confirm-quit-pack = Zakończyć bez zapisywania zmienionego pakietu?

## Powiadomienia

ui-toast-copied = Kopia zapisana
ui-toast-field-gone = Edytowane pole już nie istnieje: wpisany tekst został utracony.
ui-toast-edit-replaced = Ten tekst zmienił się podczas edycji: Twoja wersja go zastąpiła. Cofnij przywraca tamtą.

## Pasek narzędzi

ui-toolbar-undo = Cofnij
ui-toolbar-undo-label = Cofnij: { $label }
ui-toolbar-redo = Ponów
ui-toolbar-redo-label = Ponów: { $label }
ui-divider-pile = Szerokość stosu slajdów
ui-divider-notes = Szerokość wątku agenta
ui-divider-code = Rozmiar kodu
ui-toolbar-theme = Motyw
ui-toolbar-theme-embedded = Motyw z pliku
ui-toolbar-finish = Gotowe
ui-pack-deck-only = Zapisywany tylko w tej prezentacji
ui-toolbar-open = Otwórz
ui-menu = Menu
ui-toolbar-add-label = Dodaj do slajdu
ui-toolbar-packs = Pakiety prezentacji
ui-toolbar-packs-update = Pakiety prezentacji — dostępne aktualizacje
ui-toolbar-thumbs-show = Pokaż miniatury
ui-toolbar-thumbs-hide = Ukryj miniatury
ui-toolbar-remove-slide = Usuń slajd
ui-toolbar-remove-hint = Slajd { $n } · cofnij za pomocą ⌘Z

## Etykiety cofania

ui-undo-add-widget = Dodaj „{ $name }”
ui-undo-new-slide = Nowy slajd: { $name }
ui-undo-move-slide = Przenieś slajd
ui-undo-remove-slide = Usuń slajd
ui-undo-remove = Usuń
ui-undo-paste = Wklej
ui-undo-place-on-grid = Umieść na siatce
ui-undo-move-on-grid = Przenieś na siatce
ui-undo-resize-on-grid = Zmień rozmiar na siatce
ui-undo-change-anchor = Zmień zakotwiczenie
ui-undo-move = Przenieś
ui-undo-resize = Zmień rozmiar
ui-undo-edit-slide = Edytuj slajd
ui-undo-rename-pack = Zmień nazwę pakietu
ui-undo-rename-deck = Zmień nazwę prezentacji

## Stos slajdów

ui-pile-slides = Slajdy
ui-pile-layouts = Układy
ui-pile-add-slide = Nowy slajd
ui-pile-drag-hint = Przeciągnij, aby przenieść · ⌥↑ ⌥↓
ui-slide-alt = Slajd { $n }

## Scena

ui-stage-rendering = Renderowanie…
ui-stage-widget-pack = Otwórz widżet ze stosu lub utwórz nowy.
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
    [chart] Wykres
    [table] Tabela
    [stack] Stos
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
ui-agent-undo-turn = Cofnij tę turę
ui-agent-detach = Otwórz agenta w osobnym oknie
ui-agent-attach = Przywróć agenta do głównego okna
ui-agent-window-empty = Brak agenta dla tej karty
ui-agent-kept-in-draft = Brak miejsca do wpisania: tekst zachowano w szkicu
ui-agent-busy-elsewhere = Agent odpowiada w: { $place }…
ui-agent-scope-slides = slajdy
ui-agent-scope-atelier = pracownia
ui-draft = Szkic
ui-draft-send = Wyślij do agenta
ui-draft-send-title = Wyślij do agenta (⌘↩)
ui-draft-discard = Odrzuć
ui-agent-state-idle = Agent
ui-agent-state-starting = Agent się uruchamia…
ui-agent-state-working = Agent pracuje…
ui-agent-dock = W toku
ui-agent-dock-settings = Ustawienia
ui-agent-jump = ↓ Bieżąca tura
ui-agent-tool-input = Argumenty
ui-agent-tool-output = Wynik
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
ui-dictation-downloading = Pobieranie modelu { $name }
ui-dictation-download-progress = { $percent }% · { $received } z { $total } · tylko raz
ui-dictation-listening = Słuchanie · { $time }
ui-dictation-transcribing = Transkrypcja ostatniego zdania…
ui-dictation-stop-in = Zatrzymanie za { $duration }
ui-dictation-speak = Mów, tekst pojawi się tutaj…
ui-dictation-auto-stop = zatrzymuje się po { $duration } ciszy
ui-dictation-to-download = { $size } do pobrania
ui-dictation-settings = Ustawienia
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
ui-dictation-note-api = Usługa zdalna, zgodna z OpenAI
ui-dictation-api-ready = Znaleziono klucz
ui-dictation-api-key-missing = Nie ustawiono { $var }
ui-dictation-api-url = Adres API
ui-dictation-api-model = Model
ui-dictation-api-key-env = Zmienna klucza
ui-dictation-api-note = Każde zdanie jest wysyłane do tej usługi. Klucz jest odczytywany ze zmiennej środowiskowej przy rozpoczęciu dyktowania; aplikacja otwarta z Findera nie widzi zmiennych powłoki (użyj launchctl setenv).
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Wybór szablonu

ui-picker-new-slide = Nowy slajd
ui-picker-set-mine = { $name } · moje
ui-picker-set-received = { $name } · otrzymane
ui-picker-keys = ← →, aby przeglądać · Enter, aby dodać · Esc, aby zamknąć
ui-picker-close = Zamknij
ui-picker-list = Układy
ui-widget-picker-filter = Filtruj według nazwy, etykiety lub opisu
ui-widget-picker-empty = Żaden widżet nie pasuje do „{ $query }”.
ui-widget-picker-list = Widżety
ui-widget-picker-keys = Strzałki, aby przeglądać · Enter, aby dodać · Esc, aby zamknąć

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
ui-show-as = Pokaż jako
ui-show-as-table = Pokaż jako tabelę
ui-show-as-chart = Pokaż jako wykres

## Pliki

ui-file-open-prompt = Ścieżka pliku do otwarcia (.oku-slides.pdf, .json lub .pptx)
ui-file-filter-presentations = Prezentacje
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = Zapisz jako (pełna ścieżka)

## Karty

ui-tabs = Otwarte prezentacje
ui-tab-new = Nowa karta
ui-tab-close = Zamknij kartę
ui-tab-unsaved = Niezapisane zmiany
ui-tab-pack = Pakiet
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
ui-help-close = Zamknij pomoc
ui-help-search = Szukaj w pomocy
ui-help-search-placeholder = Szukaj…
ui-help-search-empty = Brak wyników
ui-help-search-count = Wyniki: { $count }
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
ui-pack-tabs = Karty pakietu
ui-pack-layouts-tab = Układy
ui-theme-tab = Motyw
ui-theme-picker = Motyw
ui-theme-name = Nazwa motywu
ui-theme-unsaved = Niezapisane zmiany motywu
ui-widgets-close-tab = Zamknij { $name }
ui-widgets-removed-by-agent = Agent usunął widżet { $name }.

## Importuj

ui-import-title = Importuj prezentację PowerPoint
ui-import-mode = Tryb importu
ui-import-hifi = Wierna kopia
ui-import-hifi-note = Zachowuje wygląd oryginału, z motywem i układami utworzonymi dla tej prezentacji.
ui-import-retrofit = Użyj moich układów
ui-import-retrofit-note = Odtwarza slajdy z pakietami układów i motywem z biblioteki.
ui-import-layout-packs = Pakiety układów
ui-import-theme-default = Motyw domyślny
ui-import-no-packs = Brak pakietu układów w bibliotece.
ui-import-tune = Dopracuj z agentem
ui-import-tune-note = Agent porównuje każdy slajd z oryginałem i poprawia różnice.
ui-import-tune-note-retrofit = Agent porównuje każdy slajd z oryginałem i projektuje widżety dla kształtów, których Twoje układy nie pokazują.
ui-import-tune-unavailable = Brak dostępnego agenta: { $reason }
ui-import-submit = Importuj
ui-import-busy = Importowanie…
ui-update-available = Dostępna jest wersja { $version } aplikacji Oku Slides.
ui-update-install = Zainstaluj i uruchom ponownie
ui-update-see-release = Zobacz wydanie
ui-update-later = Później
ui-update-installing = Pobieranie aktualizacji… { $percent }%
ui-update-downloading = Pobieranie aktualizacji…
ui-update-up-to-date = Oku Slides jest aktualny.
ui-update-check = Sprawdź dostępność aktualizacji…

ui-shortcut-theme = motyw, motywy, theme, themes
