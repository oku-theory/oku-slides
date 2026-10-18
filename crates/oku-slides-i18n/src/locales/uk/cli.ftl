cli-reading = читання { $path }
cli-invalid-deck = недопустима презентація: { $path }
cli-deck-valid = презентація коректна: слайдів — { $slides }
cli-deck-issue = проблема: { $issue }
cli-rendered = сторінок: { $pages } → { $out } (компіляція { $compile }, SVG { $svg })
cli-asset-added = ресурс { $path } → { $id }
cli-saved = { $path }: сторінок — { $pages }, PDF { $pdf } КБ, вихідні дані { $sources } КБ
cli-opened = { $path }: слайдів — { $slides }, ресурсів — { $assets }, шрифтів — { $fonts }
cli-extracted = видобуто до { $dir }
cli-stripped = { $path }: копія без вихідних даних
cli-pdf-modified = PDF змінено іншою програмою після Oku Slides
cli-iterations = --iterations має бути > 0
cli-no-block = у презентації немає блоку для редагування
cli-blob-missing = блок вихідних даних не знайдено
cli-roundtrip-mismatch = перечитана презентація відрізняється від збереженої
cli-bench-world = побудова середовища (шрифти)    { $time }
cli-bench-first = перша компіляція                { $time }  (сторінок: { $pages })
cli-bench-same = перекомпіляція без змін         { $time }
cli-bench-edited-text = Змінений текст { $n }
cli-bench-edit = перекомпіляція після set_text   { $time }
cli-bench-svg = SVG усіх слайдів                { $time }  ({ $each } на слайд)
cli-bench-pdf = експорт PDF                     { $time }  ({ $size } КБ)
cli-bench-embed = вбудовування блоку даних        { $time }
cli-bench-extract = зворотне читання блоку          { $time }
cli-bench-roundtrip = повний цикл презентації         ідентично
cli-bench-history = крок історії                    { $mean } ms в середньому, p50 { $p50 } ms
cli-serving = Oku Slides (dev) на http://{ $host }:{ $port }
cli-serving-token = відкрийте { $url } (токен дійсний до зупинки сервера)
cli-models-dir = моделі диктування: { $dir }
cli-stopped = сервер зупинено
cli-not-found = не знайдено
cli-bad-request = недопустимий запит: { $reason }
cli-sse-full = забагато вже відкритих потоків подій
cli-forbidden = запит відхилено: він надійшов не з локального інтерфейсу
cli-unauthorized = запит відхилено: токен доступу відсутній або хибний
cli-exposed = прослуховування на { $host }: сервер доступний із мережі; обслуговуються лише клієнти, що пред'являють токен цього запуску. Залиште [serve] host на 127.0.0.1, якщо мережа вам не потрібна
cli-imported = { $path }: { $slides ->
       [one] { $slides } слайд
       [few] { $slides } слайди
       [many] { $slides } слайдів
      *[other] { $slides } слайда
    }, { $pages ->
       [one] { $pages } сторінка
       [few] { $pages } сторінки
       [many] { $pages } сторінок
      *[other] { $pages } сторінки
    }, { $losses ->
       [one] { $losses } втрата
       [few] { $losses } втрати
       [many] { $losses } втрат
      *[other] { $losses } втрати
    }, { $requests ->
       [one] { $requests } запит віджета
       [few] { $requests } запити віджетів
       [many] { $requests } запитів віджетів
      *[other] { $requests } запиту віджетів
    }
cli-import-pack-required = у режимі retrofit потрібен --pack: ідентифікатор, тека або назва пакета
cli-import-write-failed = не вдалося записати імпортовану презентацію { $path }
cli-import-hifi-no-target = --pack і --theme стосуються режиму retrofit: імпорт із високою точністю (--mode hifi) створює власний пакет і тему
cli-import-reference-hifi-only = --reference належить до режиму hifi: імпорт retrofit не порівнюється з еталонним рендерингом
cli-import-scored = Візуальна відмінність від { $witness }: у середньому { $mean }, { $above ->
    [0] жодного слайда
    [one] { $above } слайд
    [few] { $above } слайди
   *[other] { $above } слайдів
} вище { $threshold }
cli-import-unscored = { $reason }

cli-about = Відтворення та зберігання презентацій Oku Slides
cli-arg-fonts = Додаткова тека шрифтів (TTF/OTF); типово: paths.fonts у налаштуваннях
cli-cmd-check = Перевірити презентацію
cli-arg-deck = Презентація (JSON)
cli-cmd-render = Відтворити кожен слайд у SVG з рамками елементів
cli-arg-input = Файл deck.json або .oku-slides.pdf
cli-arg-out = Тека виводу
cli-arg-theme = Замінити тему презентації: вбудовану або з бібліотеки, за назвою
cli-cmd-save = Зберегти презентацію як .oku-slides.pdf (PDF + блок вихідних даних)
cli-arg-output = Файл для запису
cli-arg-assets = Тека ресурсів: кожен файл стає ресурсом sha256:…
cli-arg-embed-fonts = Також вбудувати шрифти з --fonts у вихідні дані
cli-cmd-open = Повторно відкрити .oku-slides.pdf: видобути презентацію та її ресурси
cli-arg-extract = Тека, куди записати deck.json, assets/ і fonts/
cli-cmd-strip = Скопіювати .oku-slides.pdf без блоку вихідних даних
cli-cmd-bench = Виміряти час компіляції та експорту
cli-arg-iterations = Кількість вимірювань на крок
cli-cmd-schema = Вивести JSON-схему операцій
cli-cmd-serve = Сервер розробки: інтерфейс у браузері, без Tauri
cli-arg-serve-input = Презентація для відкриття (.oku-slides.pdf або .json)
cli-arg-port = Порт; типово: serve.port у налаштуваннях
cli-arg-ui = Тека зібраного інтерфейсу (ui/dist)
cli-cmd-import = Імпортувати презентацію PowerPoint (.pptx) у .oku-slides.pdf
cli-arg-pptx = Презентація PowerPoint (.pptx)
cli-arg-mode = retrofit: вміст на наявному пакеті й темі; hifi: точна копія на створених для неї пакеті й темі
cli-arg-pack = Цільовий пакет: ідентифікатор, тека пакета (pack.json, widgets/) або назва
cli-arg-import-theme = Цільова тема: вбудована або з бібліотеки, за ідентифікатором чи назвою; типово: тема за замовчуванням
cli-arg-report = Також записати звіт про імпорт (JSON): втрати, заміни, охоплення
cli-arg-no-system-fonts = У режимі hifi не шукати шрифти презентації серед системних шрифтів
cli-arg-reference = У режимі hifi порівнювати кожен слайд з еталонним рендерингом презентації: auto (типово: [import] witness, інакше office2pdf), office2pdf, soffice (LibreOffice) чи ні (none)
