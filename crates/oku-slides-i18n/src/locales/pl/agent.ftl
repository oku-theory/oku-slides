# Agent: powierzchnie, potwierdzenia, wątek.

agent-surface-cancel = Anuluj
agent-surface-confirm = Potwierdź
agent-surface-confirm-danger = Tak, zrób to
agent-surface-rewrite = Przeformułowanie
agent-surface-pick = Wybierz { $n }
agent-surface-keep = Zachowaj oryginał
agent-surface-edit = Edytuj przed zastosowaniem
agent-surface-close = Zamknij
agent-surface-themes = Motywy
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
agent-confirm-save-pack = Zapisać pakiet „{ $name }”?
agent-confirm-delete-pack = Usunąć pakiet „{ $name }”? Tej operacji nie można cofnąć.
agent-confirm-edit-theme = Zapisać nową wersję motywu „{ $name }”?
agent-confirm-copy-theme = Utworzyć wariant wbudowanego motywu „{ $name }”? Wbudowany motyw pozostanie bez zmian.
agent-confirm-setting = Zmienić { $key }: { $old } → { $new }?
agent-confirm-move = Przenieść prezentację do { $path }? Stary plik zostanie usunięty.
agent-confirm-merge = Scalić { $path } z tą prezentacją? Tej operacji nie można cofnąć.
agent-confirm-save-copy = Zapisać kopię prezentacji jako { $path }?
agent-confirm-dictation-api = Wysłać to, co dyktujesz, do { $host } ({ $url }), model { $model }, z sekretem z { $var }?
agent-confirm-dictation-download = Użyć { $name } i pobrać go teraz ({ $size })?
agent-confirm-dictation-language = Użyć dyktowania Apple i pobrać jego język teraz?
agent-confirm-agent = Przełączyć agenta aplikacji na { $name } ({ $command }), gdy żadna odpowiedź nie trwa?
agent-confirm-agent-builtin = Przełączyć agenta aplikacji na { $name }, gdy żadna odpowiedź nie trwa?
agent-confirm-agent-model = Pobrać model agenta lokalnego ({ $size })?

agent-not-found = Nie znaleziono agenta: { $command }. Ustawienie [agent.presets].
agent-spawn-failed = Nie udało się uruchomić agenta: { $command }. Ustawienie [agent.presets].
agent-no-http = Ten agent nie może połączyć się z narzędziami Oku Slides.
agent-local-no-program = Lokalny agent działa tylko w aplikacji Oku Slides.
agent-local-needs-download = Agent lokalny musi najpierw pobrać swój model ({ $gigabytes } GB).
agent-local-intel-mac = Agent lokalny wymaga komputera Mac z układem Apple.
agent-local-unsupported-os = Agent lokalny nie jest jeszcze dostępny w tym systemie.
agent-model-download = Pobierz model
agent-model-downloading = Pobieranie modelu… { $percent }%
agent-model-cancel = Anuluj
agent-model-download-failed = Nie udało się pobrać modelu.
agent-model-download-integrity = Pobrany model był uszkodzony i został usunięty.
agent-model-download-incomplete = Niepełne pobieranie: { $got } z { $expected } bajtów.
agent-model-download-cancelled = Pobieranie anulowane.
local-agent-no-route = Lokalny agent nie zrozumiał, co zrobić. Spróbuj powiedzieć to inaczej.
local-agent-tool-limit = Lokalny agent zatrzymał się po { $n } krokach, nie kończąc zadania.
local-agent-invalid-call = Lokalny agent nie potrafił poprawnie użyć narzędzi.
local-agent-engine-failed = Model lokalnego agenta zawiódł.
local-agent-retry = Spróbuj ponownie lub wybierz innego agenta w panelu agenta.
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
agent-error-pack-id = To nie jest id pakietu: wywołaj packs, aby wypisać pakiety i ich id.
agent-error-asset-unknown = Tego obrazu nie ma ani w prezentacji, ani wśród wklejonych obrazów: użyj identyfikatora wklejonego obrazu.
agent-error-denied = Użytkownik odmówił.
agent-error-no-turn = Żadna tura nie trwa: { $tool } jest wyświetlany tylko podczas tury.
agent-error-turn-over = Tura tego wywołania dobiegła końca: nic nie zostało wyświetlone.
agent-error-no-file = Prezentacja nie ma jeszcze pliku: podaj ścieżkę bezwzględną.
agent-error-path-relative = Ścieżka musi być bezwzględna.
agent-error-path-parent = Ścieżka nie może zawierać „..”.
agent-error-setting-locked = Agent nie może zmienić ustawienia { $key }.
agent-error-setting = Nie udało się zmienić ustawienia { $key }: sprawdź jego wartość.
agent-error-history-index = Aby przeglądać historię, podaj indeks wersji.
agent-error-no-pack = Ta prezentacja nie przypina żadnego pakietu: podaj pack (zob. packs).
agent-error-history-not-opened = Karta nie otworzyła historii na czas: poproś ponownie.
agent-error-present = Nie ma tu nic do prezentowania: brak slajdów albo karta jest w trybie pakietu, historii lub tylko do odczytu.
agent-error-locale = Nieznany kod języka. Podaj kod dostępnego języka lub system.
agent-error-empty-interface = Nic do zmiany: podaj scheme, locale, grid lub guides.
agent-error-empty-dictation = Nic do zmiany: podaj co najmniej jedno ustawienie dyktowania.
agent-error-not-shown = Ta karta nie jest wyświetlana w swoim oknie: poproś ponownie, gdy użytkownik ją wyświetli.
agent-error-themes-deck-mode = Wybór motywu otwiera się tylko podczas edycji slajdów.
agent-error-packs-deck-mode = Panel pakietów prezentacji otwiera się tylko podczas edycji slajdów.
agent-error-rewrite-text = Przeredagowanie wróciło bez tekstu.
agent-error-paragraphs = Akapity przeredagowania są nieprawidłowe.
agent-error-draft-too-long = Szkic jest za długi.
agent-error-draft-origin = Szkic odrzucony: nieznany widok.
agent-error-draft-no-agent = Agent tej karty jeszcze się nie uruchomił: szkic odrzucony.
agent-error-too-many-images = Szkic ma już wszystkie obrazy.
agent-error-image-not-base64 = Wklejony obraz nie dotarł w całości.
agent-error-image-gone = Wklejony obraz nie jest już dostępny: wklej go ponownie.
agent-error-unknown-agent = Tego agenta nie ma już w konfiguracji.
agent-error-agent-locked = Wybór agenta jest narzucony przez środowisko.
agent-error-agent-not-saved = Nie udało się zapisać wyboru agenta.
voice-error-no-dictation = Dyktowanie nie jest tu dostępne
voice-error-busy = Dyktowanie trwa: spróbuj ponownie, gdy się zakończy
voice-error-unknown-model = Nieznany model dyktowania
voice-error-unavailable = Ten model nie obsługuje wybranego języka
voice-error-installed = Ten model jest już zainstalowany
voice-error-not-downloadable = Tego modelu się nie pobiera
voice-error-nothing-to-cancel = Żadne pobieranie nie trwa
voice-error-bad-silence = Cisza musi trwać od 0,5 do 10 sekund
voice-error-bad-language = Nieznany język dyktowania
voice-error-bad-url = Adres API musi zaczynać się od https:// (http:// tylko dla tego komputera), bez nazwy użytkownika, parametrów i znaków specjalnych
voice-error-bad-key-env = Zmienna klucza musi być nazwą w rodzaju OPENAI_API_KEY: wielkie litery, cyfry i _
voice-error-bad-api-model = Nazwa modelu API może zawierać tylko litery, cyfry oraz . _ : / -
voice-error-bad-api-query = Parametry zapisuje się nazwa=wartość&nazwa=wartość, w widocznym ASCII bez % i #, najwyżej 8
voice-error-bad-api-headers = Nagłówki zapisuje się Nazwa: wartość; Nazwa: $VARIABLE, najwyżej 8. Wymagana jest zmienna klucza lub nagłówek $VARIABLE; Host, Content-Type, Content-Length, Transfer-Encoding, Connection, duplikaty i Authorization przy zmiennej klucza są odrzucane
voice-error-api-reserved = Tylko użytkownik ustawia adres API, zmienną klucza, parametry zapytania i nagłówki: otwórz okno przez show_dictation i poproś go o to
voice-error-changed = Ustawienia zmieniły się od pytania: zapytaj ponownie
voice-error-nothing-to-download = Nie ma nic do pobrania dla tego agenta
agent-tune-prompt = Dopracuj zaimportowaną prezentację
agent-settings-unknown-selected = Ustawienie agent.selected = { $selected } nie wskazuje żadnego agenta z [agent.presets] (znane: { $known })
agent-settings-preset-launch = Agent [agent.presets.{ $id }] wymaga dokładnie jednego z command lub builtin
