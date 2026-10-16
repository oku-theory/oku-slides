cli-reading = чтение { $path }
cli-invalid-deck = недопустимая презентация: { $path }
cli-deck-valid = презентация корректна: слайдов — { $slides }
cli-deck-issue = проблема: { $issue }
cli-rendered = страниц: { $pages } → { $out } (компиляция { $compile }, SVG { $svg })
cli-asset-added = ресурс { $path } → { $id }
cli-saved = { $path }: страниц — { $pages }, PDF { $pdf } КБ, исходники { $sources } КБ
cli-opened = { $path }: слайдов — { $slides }, ресурсов — { $assets }, шрифтов — { $fonts }
cli-extracted = извлечено в { $dir }
cli-stripped = { $path }: копия без исходников
cli-pdf-modified = PDF был изменён другой программой после Oku Slides
cli-iterations = --iterations должно быть > 0
cli-no-block = в презентации нет блока для редактирования
cli-blob-missing = блок исходников не найден
cli-roundtrip-mismatch = перечитанная презентация отличается от сохранённой
cli-bench-world = построение окружения (шрифты)   { $time }
cli-bench-first = первая компиляция               { $time }  (страниц: { $pages })
cli-bench-same = перекомпиляция без изменений    { $time }
cli-bench-edited-text = Изменённый текст { $n }
cli-bench-edit = перекомпиляция после set_text   { $time }
cli-bench-svg = SVG всех слайдов                { $time }  ({ $each } на слайд)
cli-bench-pdf = экспорт PDF                     { $time }  ({ $size } КБ)
cli-bench-embed = встраивание блока исходников    { $time }
cli-bench-extract = обратное чтение блока           { $time }
cli-bench-roundtrip = полный цикл презентации         идентично
cli-bench-history = шаг истории                     { $mean } ms в среднем, p50 { $p50 } ms
cli-serving = Oku Slides (dev) на http://{ $host }:{ $port }
cli-serving-token = откройте { $url } (токен действует до остановки сервера)
cli-models-dir = модели диктовки: { $dir }
cli-stopped = сервер остановлен
cli-not-found = не найдено
cli-bad-request = недопустимый запрос: { $reason }
cli-sse-full = слишком много уже открытых потоков событий
cli-forbidden = запрос отклонён: он поступил не из локального интерфейса
cli-unauthorized = запрос отклонён: токен доступа отсутствует или неверен
cli-exposed = прослушивание на { $host }: сервер доступен из сети; обслуживаются только клиенты, предъявившие токен этого запуска. Оставьте [serve] host на 127.0.0.1, если сеть вам не нужна
cli-imported = { $path }: { $slides ->
       [one] { $slides } слайд
       [few] { $slides } слайда
       [many] { $slides } слайдов
      *[other] { $slides } слайда
    }, { $pages ->
       [one] { $pages } страница
       [few] { $pages } страницы
       [many] { $pages } страниц
      *[other] { $pages } страницы
    }, { $losses ->
       [one] { $losses } потеря
       [few] { $losses } потери
       [many] { $losses } потерь
      *[other] { $losses } потери
    }, { $requests ->
       [one] { $requests } запрос виджета
       [few] { $requests } запроса виджетов
       [many] { $requests } запросов виджетов
      *[other] { $requests } запроса виджетов
    }
cli-import-label = Импорт из PowerPoint
cli-import-pack-required = в режиме retrofit нужен --pack: идентификатор, папка или название пакета
cli-import-unknown-pack = неизвестный пакет: { $pack }
cli-import-bad-pack = нечитаемый пакет { $pack }: его pack.json не описывает пакет шаблонов
cli-import-write-failed = не удалось записать импортированную презентацию { $path }
cli-import-hifi-no-target = --pack и --theme относятся к режиму retrofit: импорт с высокой точностью (--mode hifi) создаёт собственный пакет и тему
cli-import-probe-mismatch = созданный пакет не прошёл проверку в соответствии с объявлением (виджет { $widget }): это ошибка импорта
cli-import-probe-limit = созданный пакет превышает ограничения проверки (виджет { $widget }): увеличьте max_widgets, max_widget_kib или probe_budget_ms в разделе [render] конфигурации
cli-import-probe-failed = процесс рендеринга не смог проверить созданный пакет: { $error }
cli-import-pack-missing = импорт не создал пакет для презентации: это ошибка импорта
cli-import-theme-missing = импорт не создал тему для презентации: это ошибка импорта
cli-import-reference-hifi-only = --reference относится к режиму hifi: импорт retrofit не сравнивается с эталонным рендерингом
cli-import-bad-threshold = score_threshold в разделе [import] должен быть числом не меньше нуля, а не { $value }
cli-import-scored = Визуальное отличие от { $witness }: в среднем { $mean }, { $above ->
    [0] ни одного слайда
    [one] { $above } слайд
    [few] { $above } слайда
   *[other] { $above } слайдов
} выше { $threshold }
cli-import-unscored = { $reason }

cli-about = Отрисовка и хранение презентаций Oku Slides
cli-arg-fonts = Дополнительная папка шрифтов (TTF/OTF); по умолчанию: paths.fonts в настройках
cli-cmd-check = Проверить презентацию
cli-arg-deck = Презентация (JSON)
cli-cmd-render = Отрисовать каждый слайд в SVG с рамками элементов
cli-arg-input = Файл deck.json или .oku-slides.pdf
cli-arg-out = Папка вывода
cli-arg-theme = Заменить тему презентации: встроенную или из библиотеки, по названию
cli-cmd-save = Сохранить презентацию как .oku-slides.pdf (PDF + блок исходников)
cli-arg-output = Файл для записи
cli-arg-assets = Папка ресурсов: каждый файл становится ресурсом sha256:…
cli-arg-embed-fonts = Также встроить шрифты из --fonts в исходники
cli-cmd-open = Открыть .oku-slides.pdf заново: извлечь презентацию и её ресурсы
cli-arg-extract = Папка, куда записать deck.json, assets/ и fonts/
cli-cmd-strip = Скопировать .oku-slides.pdf без блока исходников
cli-cmd-bench = Измерить время компиляции и экспорта
cli-arg-iterations = Число замеров на шаг
cli-cmd-schema = Вывести JSON-схему операций
cli-cmd-serve = Сервер разработки: интерфейс в браузере, без Tauri
cli-arg-serve-input = Презентация для открытия (.oku-slides.pdf или .json)
cli-arg-port = Порт; по умолчанию: serve.port в настройках
cli-arg-ui = Папка собранного интерфейса (ui/dist)
cli-cmd-import = Импортировать презентацию PowerPoint (.pptx) в .oku-slides.pdf
cli-arg-pptx = Презентация PowerPoint (.pptx)
cli-arg-mode = retrofit: содержимое на существующем пакете и теме; hifi: точная копия на созданных для неё пакете и теме
cli-arg-pack = Целевой пакет: идентификатор, папка пакета (pack.json, widgets/) или название
cli-arg-import-theme = Целевая тема: встроенная или из библиотеки, по идентификатору или названию; по умолчанию: тема по умолчанию
cli-arg-report = Также записать отчёт об импорте (JSON): потери, замены, охват
cli-arg-no-system-fonts = В режиме hifi не искать шрифты презентации среди системных шрифтов
cli-arg-reference = В режиме hifi сравнивать каждый слайд с эталонным рендерингом презентации (auto, по умолчанию) или нет (none)
