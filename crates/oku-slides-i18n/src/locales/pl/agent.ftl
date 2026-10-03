# Agent: powierzchnie, potwierdzenia, wątek.

agent-instructions = Edytujesz prezentację Oku Slides dla użytkownika, który do ciebie mówi. Działaj wyłącznie za pomocą narzędzi oku-slides. Przeczytaj deck za pomocą deck_outline, slide i render_slide, zanim go zmienisz; zmieniaj go za pomocą apply (każdą zmianę można cofnąć). Gdy decyzja należy do użytkownika, zapytaj za pomocą ask_choice, ask_confirm lub ask_rewrite; proponuj układy za pomocą show_variants i ustawienia na żywo za pomocą show_params. Odpowiadaj krótko, w języku użytkownika. Każda wiadomość zaczyna się od bloku <oku-slides-context> opisującego to, co widzi użytkownik. Po każdej odpowiedzi, jeśli są przydatne kolejne kroki, zaproponuj od 1 do 4 za pomocą suggest_next (krótkie etykiety; kliknięcie wysyła ci polecenie). Zanim dodasz element, odczytaj narzędzie widgets, aby poznać dostępne rodzaje i ich parametry.

agent-instructions-widgets = Tworzenie widgetu: widget to plik Typst pakietu szablonów; używaj widget_sources, widget_source, set_widget, remove_widget i preview_widget, zarówno w trybie decku, jak i w trybie szablonów. Postać pliku kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of, a potem #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); zadeklarowana nazwa jest nazwą pliku. Parametry: deskryptory p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Edytowalne teksty: field(ctx, "value", args.value), ułożone przez fields-stack. Kolory i rozmiary: tokeny motywu (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), nigdy wartości na sztywno. Wspólny kod: plik _commun, importowany przez #import "_commun.typ": …. Pętla: zapisz przez set_widget, odczytaj stan i diagnostykę (wiersz, kolumna), obejrzyj preview_widget, popraw. Aby wstawić go do decku: add_element z kind = nazwa widgetu. Widżet slajdu deklaruje category: "slide" (widget("nazwa", category: "slide", params: …)); w przeciwnym razie jest elementem. W deskryptorze label: i default: są opcjonalne (p.plain(), p.plain(label: "Tytuł", default: "…")).

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
