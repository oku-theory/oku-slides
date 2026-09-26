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
ui-status-builtin = wbudowany: zapisanie utworzy kopię
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

ui-toast-saved = Prezentacja została zapisana
ui-toast-templates-saved = Szablony zapisane: { $name }
ui-toast-select-zone = Zaznacz obszar, aby umieścić: „{ $text }”

## Pasek narzędzi

ui-toolbar-undo = Cofnij
ui-toolbar-undo-label = Cofnij: { $label }
ui-toolbar-redo = Ponów
ui-toolbar-redo-label = Ponów: { $label }
ui-toolbar-grid-hide = Ukryj siatkę
ui-toolbar-grid-show = Pokaż siatkę
ui-toolbar-grid-hide-title = Ukryj siatkę (⌘')
ui-toolbar-grid-show-title = Pokaż siatkę (⌘')
ui-toolbar-theme = Motyw
ui-toolbar-theme-default = Domyślny
ui-toolbar-save-copy = Zapisz kopię
ui-toolbar-save = Zapisz
ui-toolbar-finish = Gotowe
ui-toolbar-templates = Szablony
ui-toolbar-templates-title = Twórz i edytuj szablony slajdów
ui-toolbar-open = Otwórz
ui-toolbar-add-label = Dodaj do slajdu
ui-toolbar-add = Dodaj
ui-add-kind = { $kind ->
    [card] Karta
    [shape] Kształt
    [chart] Wykres
   *[text] Tekst
}
ui-toolbar-remove-template = Usuń szablon
ui-toolbar-remove-slide = Usuń slajd
ui-toolbar-remove-hint = Slajd { $n } · cofnij za pomocą ⌘Z

## Etykiety cofania

ui-undo-add = { $kind ->
    [card] Dodaj kartę
    [shape] Dodaj kształt
    [chart] Dodaj wykres
   *[text] Dodaj tekst
}
ui-undo-new-template = Nowy szablon: { $name }
ui-undo-new-slide = Nowy slajd: { $name }
ui-undo-grid = Siatka { $columns } × { $rows }
ui-undo-templates = Szablony: { $name }
ui-undo-move-template = Przenieś szablon
ui-undo-move-slide = Przenieś slajd
ui-undo-remove-slide = Usuń slajd
ui-undo-remove = Usuń
ui-undo-place-on-grid = Umieść na siatce
ui-undo-move-on-grid = Przenieś na siatce
ui-undo-resize-on-grid = Zmień rozmiar na siatce
ui-undo-change-anchor = Zmień zakotwiczenie
ui-undo-move = Przenieś
ui-undo-resize = Zmień rozmiar
ui-undo-edit-text = Edytuj tekst
ui-undo-dictation = Dyktowanie
ui-undo-rename-set = Zmień nazwę zestawu
ui-undo-rename-deck = Zmień nazwę prezentacji
ui-undo-theme = Zmień motyw
ui-undo-rename-template = Zmień nazwę szablonu
ui-undo-describe-template = Opisz szablon
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
ui-stage-edit-text = Edytuj tekst
ui-stage-hint-field = Enter, aby zatwierdzić · Esc, aby anulować
ui-stage-hint-block = ⌘ Enter, aby zatwierdzić · jeden wiersz na akapit, „- ” dla punktu listy
ui-field = { $kind ->
    [title] Tytuł
    [subtitle] Podtytuł
    [kicker] Nadtytuł
    [text] Tekst
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

## Notatki slajdu

ui-notes = Narzędzia slajdu
ui-stepper-less = { $label }: o jeden mniej
ui-stepper-more = { $label }: o jeden więcej

## Dyktowanie

ui-dictation = Dyktowanie
ui-dictation-start = Dyktuj
ui-dictation-start-title = Dyktuj (⌘D)
ui-dictation-finish = Zakończ dyktowanie
ui-dictation-finish-title = Zakończ (⌘D)
ui-dictation-cancel = Anuluj
ui-dictation-preparing = Przygotowywanie dyktowania…
ui-dictation-heard-nothing = Nic nie usłyszano: mów nieco bliżej mikrofonu
ui-dictation-to-cursor = w miejscu kursora
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
ui-dictation-auto-stop-label = Zatrzymaj automatycznie po ciszy trwającej
ui-dictation-silence = Czas trwania ciszy
ui-dictation-phrases-note = Transkrybowane są tylko fragmenty z mową, zdanie po zdaniu, w trakcie mówienia.
ui-dictation-download-now = Pobierz teraz ({ $size })
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
ui-chart-add-row = + Wiersz
ui-chart-add-column = + Kolumna
ui-chart-series = Seria { $n }

## Pliki

ui-file-open-prompt = Ścieżka pliku do otwarcia (.oku-slides.pdf lub .json)
ui-file-save-prompt = Zapisz jako (pełna ścieżka)
