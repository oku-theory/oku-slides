# Agent: powierzchnie, potwierdzenia, wątek.

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
agent-confirm-edit-theme = Zapisać nową wersję motywu „{ $name }”?
agent-confirm-copy-theme = Zapisać zmienioną kopię wbudowanego motywu „{ $name }”?
agent-confirm-setting = Zmienić { $key }: { $old } → { $new }?

agent-not-found = Nie znaleziono agenta: { $command }. Ustawienie [agent.presets].
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
agent-error-pack-id = To nie jest id zestawu szablonów: wywołaj templates, aby wypisać zestawy i ich id.
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
agent-error-unknown-agent = Tego agenta nie ma już w konfiguracji.
agent-error-agent-locked = Wybór agenta jest narzucony przez środowisko.
agent-error-agent-not-saved = Nie udało się zapisać wyboru agenta.
agent-picker-title = Agent
agent-picker-busy = Zmiana agenta przerwie bieżącą turę.
agent-picker-locked = Wybór narzucony przez środowisko (OKU_SLIDES_AGENT__SELECTED).
agent-tune-prompt = Dopracuj zaimportowaną prezentację
