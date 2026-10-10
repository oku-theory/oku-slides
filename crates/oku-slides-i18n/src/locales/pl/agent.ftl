# Agent: powierzchnie, potwierdzenia, wątek.

agent-instructions = Edytujesz prezentację Oku Slides dla użytkownika, który do ciebie mówi. Działaj wyłącznie przez narzędzia oku-slides. Przeczytaj deck za pomocą deck_outline, slide i render_slide, zanim go zmienisz; zmieniaj go przez apply (wszystko można cofnąć). Gdy decyzja należy do użytkownika, zapytaj przez ask_choice, ask_confirm lub ask_rewrite; proponuj układy przez show_variants i ustawienia na żywo przez show_params. Każdą turę zakończ narzędziem report: jednowierszowy tytuł, od 1 do 4 krótkich faktów, twoje wyjaśnienie w why i od 0 do 4 kolejnych kroków w actions. Twój swobodny tekst jest pokazywany tylko jako jednowierszowa notatka o postępie: pisz go tak, krótko i w języku użytkownika. Używaj render tylko wtedy, gdy żadne inne narzędzie nie pasuje. Każda wiadomość zaczyna się blokiem <oku-slides-context>, który opisuje to, co widzi użytkownik. Zanim dodasz element, przeczytaj narzędzie widgets, aby poznać dostępne rodzaje i ich parametry. Aby zmienić siatkę slajdu, użyj operacji set_grid: umieszczone elementy podążają za nią.

agent-instructions-widgets = Tworzenie widgetu: widget to plik Typst pakietu szablonów; używaj widget_sources, widget_source, set_widget, remove_widget i preview_widget, zarówno w trybie decku, jak i w trybie szablonów. Postać pliku kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, a potem #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); zadeklarowana nazwa jest nazwą pliku. Parametry: deskryptory p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Edytowalne teksty: field(ctx, "value", args.value), ułożone przez fields-stack. Kolory i rozmiary: tokeny motywu (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), nigdy wartości na sztywno. Wspólny kod: plik _commun, importowany przez #import "_commun.typ": …. Pętla: zapisz przez set_widget, odczytaj stan i diagnostykę (wiersz, kolumna), obejrzyj preview_widget, popraw. Aby wstawić go do decku: add_element z kind = nazwa widgetu. Widżet slajdu deklaruje category: "slide" (widget("nazwa", category: "slide", params: …)); w przeciwnym razie jest elementem. W deskryptorze label: i default: są opcjonalne (p.plain(), p.plain(label: "Tytuł", default: "…")). W trybie szablonów ustaw wartości testowe widżetu za pomocą set_widget_trial; preview_widget bez parametrów ich używa, a użytkownik widzi je w ustawieniach widżetu. Listy: p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) dla powtarzanych pozycji (oś czasu, KPI, zespół); typy kolumn plain, rich, number, flag, choice, asset. Renderuj każdą pozycję przez list-item(ctx, "etapes", e, c => field(c, "date", e.date)); dla pustego pola użyj field(c, name, box(width: 3em, hide[0]), empty: true), aby pozostało edytowalne. Nigdy nie udawaj listy numerowanymi parametrami (date1, date2…). Brak listy oznacza listę pustą; default tylko zasila nowy obiekt. Pozycje mają stałe id: edytuj je przez edit_list(target, param, change) (change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); id podajesz ty) albo zastąp całą listę przez set_param (null ją opróżnia). Gdy add_element lub add_slide tworzy widget bez podanych przez ciebie pozycji, przekaż "defaults": true, by wstawić domyślne wartości listy; nigdy przy ponownym wstawianiu ani przenoszeniu.

agent-surface-cancel = Anuluj
agent-surface-confirm = Potwierdź
agent-surface-confirm-danger = Tak, zrób to
agent-surface-rewrite = Przeformułowanie
agent-surface-pick = Wybierz { $n }
agent-surface-keep = Zachowaj oryginał
agent-surface-edit = Edytuj przed zastosowaniem
agent-surface-close = Zamknij
agent-surface-variants = Warianty
agent-surface-permission-always = na zawsze
agent-surface-permission-title = Agent prosi o uprawnienie
agent-undo-turn = Agent: { $text }

agent-tools-unavailable = Nie udało się uruchomić narzędzi agenta.

agent-undo-apply = Zmiana agenta
agent-undo-variant = Wariant: { $label }
agent-undo-param = Ustawienie: { $label }
agent-undo-rewrite = Przeformułowanie

agent-confirm-save = Zapisać prezentację jako { $path }?
agent-confirm-save-over = Zastąpić { $path }? Istniejący plik zostanie nadpisany.
agent-confirm-open = Otworzyć { $path }?
agent-open-elsewhere = Ten plik jest już otwarty w innej karcie.
agent-confirm-new = Rozpocząć nową prezentację?
agent-confirm-new-dirty = Rozpocząć nową prezentację? Niezapisane zmiany zostaną utracone.
agent-confirm-save-templates = Zapisać zestaw szablonów „{ $name }”?
agent-confirm-delete-set = Usunąć zestaw szablonów „{ $name }”? Tej operacji nie można cofnąć.
agent-confirm-setting = Zmienić { $key }: { $old } → { $new }?

agent-not-found = Nie znaleziono agenta: { $command }. Ustawienie [agent] command.
agent-no-http = Ten agent nie może połączyć się z narzędziami Oku Slides.
agent-stopped = Agent zatrzymał się.
agent-failed = Agent odrzucił żądanie: { $message }
agent-tool-failed = Narzędzie nieoczekiwanie zawiodło.

agent-turn-tools = { $tools ->
   [one] { $tools } narzędzie
   [few] { $tools } narzędzia
   [many] { $tools } narzędzi
  *[other] { $tools } narzędzia
}
agent-turn-notes = { $notes ->
   [one] { $notes } notatka
   [few] { $notes } notatki
   [many] { $notes } notatek
  *[other] { $notes } notatki
}
agent-turn-secs = { $secs } s
agent-turn-stopped = Zatrzymano.
agent-turn-interrupted = Przerwano.
agent-memory-lost = Agent nie pamięta już tej rozmowy: zaczyna od nowa.
agent-turn-max-tokens = Agent osiągnął limit długości.
agent-turn-max-turns = Agent osiągnął limit kroków.
agent-turn-refusal = Agent odmówił.
agent-turn-ended = Tura zakończona ({ $stop }).
agent-choice-cancelled = Pytanie anulowane
agent-choice-answered = ✓ { $answer }
agent-report-why = Dlaczego

agent-error-file = Nie udało się odczytać ani zapisać pliku.
agent-error-unexpected-action = Nieoczekiwana akcja: { $action }.
agent-error-unknown-surface = Ta karta nie jest już otwarta.
agent-error-expired = Akcje minionej tury wygasły.
agent-error-incomplete = Akcja tej karty jest niekompletna.
agent-error-no-field = To ustawienie już nie istnieje.
agent-error-no-variant = Ten wariant już nie istnieje.
agent-error-arguments = Argumenty narzędzia są nieprawidłowe ({ $detail }): sprawdź je według jego schematu.
agent-error-unknown-tool = Nieznane narzędzie: { $name }.
agent-error-denied = Użytkownik odmówił.
agent-error-no-turn = Żadna tura nie trwa: { $tool } jest wyświetlany tylko podczas tury.
agent-error-turn-over = Tura tego wywołania dobiegła końca: nic nie zostało wyświetlone.
agent-error-anchor-scope = Pytania w rozmowie o widżecie nie można zakotwiczyć na slajdzie: zadaj je bez kotwicy.
agent-error-no-file = Prezentacja nie ma jeszcze pliku: podaj ścieżkę bezwzględną.
agent-error-path-relative = Ścieżka musi być bezwzględna.
agent-error-path-parent = Ścieżka nie może zawierać „..”.
agent-error-setting-locked = Agent nie może zmienić ustawienia { $key }.
agent-error-setting = Nie udało się zmienić ustawienia { $key }: sprawdź jego wartość.
agent-error-rewrite-text = Przeredagowanie wróciło bez tekstu.
agent-error-paragraphs = Akapity przeredagowania są nieprawidłowe.
agent-error-draft-too-long = Szkic jest za długi.
agent-error-draft-origin = Szkic odrzucony: nieznany widok.
agent-error-draft-no-agent = Agent tej karty jeszcze się nie uruchomił: szkic odrzucony.
