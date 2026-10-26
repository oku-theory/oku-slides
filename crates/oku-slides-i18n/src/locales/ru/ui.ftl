# Интерфейс приложения (React). Загружается ui/src/i18n.ts через запрос «messages».
ui-loading = Загрузка…

## Документ

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · набор макетов
   *[other] { $name } · набор виджетов
}
ui-title-rename = Переименовать презентацию
ui-title-field = Название презентации
ui-status-unsaved = есть несохранённые изменения
ui-status-saved = сохранено
ui-status-never-saved = ещё не сохранено
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count } макет
        [few] { $count } макета
        [many] { $count } макетов
       *[other] { $count } макета
    }
   *[other] { $count ->
        [one] { $count } виджет
        [few] { $count } виджета
        [many] { $count } виджетов
       *[other] { $count } виджета
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } слайд
    [few] { $count } слайда
    [many] { $count } слайдов
   *[other] { $count } слайда
} · { $status }
ui-confirm-quit-pack = Выйти, не сохранив изменённый набор?
ui-confirm-update-opener = Обновить «{ $deck }» до этой версии набора «{ $pack }»?
ui-close-writes-pending = Эта вкладка ещё сохраняет: закройте её снова через мгновение.
ui-confirm-close-writes-pending = Эта вкладка ещё сохраняет. Всё равно закрыть её и потерять то, что не сохранено?
ui-confirm-quit-read-only-pack = { $pack } нельзя сохранить здесь. Закрыть эту вкладку и потерять её изменения? Чтобы сохранить их, попросите агента сохранить его как новый набор.

## Уведомления

ui-toast-copied = Копия сохранена
ui-toast-field-gone = Поле, которое вы редактировали, больше не существует: введённый текст потерян.
ui-toast-edit-replaced = Этот текст изменился, пока вы его редактировали: ваша версия его заменила. «Отменить» вернёт другую.

## Панель инструментов

ui-toolbar-undo = Отменить
ui-toolbar-undo-label = Отменить: { $label }
ui-toolbar-redo = Повторить
ui-toolbar-redo-label = Повторить: { $label }
ui-divider-pile = Ширина стопки слайдов
ui-divider-notes = Ширина ленты агента
ui-divider-code = Размер кода
ui-toolbar-theme = Тема
ui-toolbar-theme-embedded = Тема из файла
ui-toolbar-finish = Готово
ui-pack-deck-only = Сохраняется только в этой презентации
ui-toolbar-open = Открыть
ui-menu = Меню
ui-toolbar-add-label = Добавить на слайд
ui-toolbar-packs = Пакеты презентации
ui-toolbar-packs-update = Пакеты презентации — доступны обновления
ui-toolbar-thumbs-show = Показать миниатюры
ui-toolbar-thumbs-hide = Скрыть миниатюры
ui-toolbar-remove-slide = Удалить слайд
ui-toolbar-remove-hint = Слайд { $n } · отменить: ⌘Z

## Подписи отмены

ui-undo-add-widget = Добавить «{ $name }»
ui-undo-new-slide = Новый слайд: { $name }
ui-undo-move-slide = Переместить слайд
ui-undo-remove-slide = Удалить слайд
ui-undo-remove = Удалить
ui-undo-paste = Вставить
ui-undo-place-on-grid = Разместить на сетке
ui-undo-move-on-grid = Переместить по сетке
ui-undo-resize-on-grid = Изменить размер по сетке
ui-undo-change-anchor = Изменить привязку
ui-undo-move = Переместить
ui-undo-resize = Изменить размер
ui-undo-edit-slide = Изменить слайд
ui-undo-rename-pack = Переименовать набор
ui-undo-rename-deck = Переименовать презентацию

## Стопка слайдов

ui-pile-slides = Слайды
ui-pile-layouts = Макеты
ui-pile-add-slide = Новый слайд
ui-pile-drag-hint = Перетащите для перемещения · ⌥↑ ⌥↓
ui-slide-alt = Слайд { $n }

## Сцена

ui-stage-rendering = Отрисовка…
ui-stage-widget-pack = Откройте виджет из стопки или создайте новый.
ui-stage-elements = Элементы слайда
ui-stage-fields = Поля слайда
ui-stage-edit-text = Изменить текст
ui-stage-hint-plain = Enter или Tab — к следующему · Esc — отменить
ui-stage-hint-rich = Tab — к следующему · ⌘ Enter — подтвердить · «- » для пункта списка · ⌘B жирный, ⌘I курсив, ⌘K ссылка
ui-field = { $kind ->
    [title] Заголовок
    [subtitle] Подзаголовок
    [kicker] Надзаголовок
    [text] Текст
    [body] Основной текст
    [author] Автор
    [date] Дата
    [organization] Организация
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Текст
    [card] Карточка
    [image] Изображение
    [shape] Фигура
    [chart] Диаграмма
    [table] Таблица
    [stack] Стопка
   *[other] { $kind }
}
ui-grid-cell = стб. { $cols } · стр. { $rows }
ui-ghost-zone = Зона

## Заметки к слайду

ui-notes = Инструменты слайда
ui-stepper-less = { $label }: на один меньше
ui-stepper-more = { $label }: на один больше
ui-list-item = элемент
ui-list-add = Добавить { $item }
ui-list-remove = Удалить { $item }
ui-list-move = Переместить { $item }
ui-list-full = Список заполнен ({ $max })
ui-undo-list-move = Переместить { $item }
ui-agent-color-other = Другой цвет
ui-agent-thread = Агент
ui-agent-new-conversation = Новый разговор
ui-agent-undo-turn = Отменить этот ход
ui-agent-detach = Открыть агента в отдельном окне
ui-agent-attach = Вернуть агента в главное окно
ui-agent-window-empty = Для этой вкладки нет агента
ui-agent-kept-in-draft = Некуда вставить: текст сохранён в черновике
ui-draft = Черновик
ui-draft-send = Отправить агенту
ui-draft-send-title = Отправить агенту (⌘↩)
ui-draft-discard = Удалить
ui-pasted-image = { $type } { $width }×{ $height }
ui-draft-images = Вставленные изображения
ui-draft-remove-image = Убрать это изображение
ui-draft-send-needs-text = Скажите, что с ним сделать
ui-agent-state-idle = Агент
ui-agent-state-starting = Агент запускается…
ui-agent-state-working = Агент работает…
ui-agent-dock = В процессе
ui-agent-dock-settings = Настройки
ui-agent-jump = ↓ Текущий ход
ui-agent-tool-input = Аргументы
ui-agent-tool-output = Результат
ui-agent-sheet = Вопрос агента
ui-agent-sheet-keys = 1–4 выбрать · Enter подтвердить · E изменить · Esc отменить
ui-agent-variants = Варианты
ui-agent-drawer-keys = Щелчок или 1–4 · Esc закрывает
ui-agent-confirm = Подтверждение агента

## Диктовка

ui-dictation = Диктовка
ui-dictation-start = Диктовать
ui-dictation-start-title = Диктовать (⌘D)
ui-dictation-type = Написать агенту
ui-dictation-type-title = Написать агенту (⌘K)
ui-dictation-agent-stop = Остановить агента (⌘⇧D или Esc)
ui-dictation-agent-working = Агент работает…
ui-dictation-agent-working-hint = ⌘⇧D или Esc остановит его
ui-dictation-finish = Завершить диктовку
ui-dictation-finish-title = Завершить (⌘D)
ui-dictation-cancel = Отмена
ui-dictation-preparing = Подготовка диктовки…
ui-dictation-heard-nothing = Ничего не слышно: говорите чуть ближе к микрофону
ui-dictation-to-cursor = в позицию курсора
ui-dictation-to-draft = в черновик
ui-dictation-downloading = Загрузка модели { $name }
ui-dictation-download-progress = { $percent }% · { $received } из { $total } · только один раз
ui-dictation-listening = Слушаю · { $time }
ui-dictation-transcribing = Распознаю последнюю фразу…
ui-dictation-stop-in = Остановка через { $duration }
ui-dictation-speak = Говорите, текст появится здесь…
ui-dictation-auto-stop = остановится после { $duration } тишины
ui-dictation-to-download = { $size } для загрузки
ui-dictation-settings = Настройки
ui-dictation-language-auto = Автоопределение
ui-dictation-model-unavailable = Язык не поддерживается
ui-dictation-language-auto-resolved = Автоматически ({ $language })
ui-dictation-to-download-language = Язык для загрузки
ui-dictation-api-key-missing = { $var } не задана
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } с
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } ГБ
ui-size-mb = { $value } МБ

## Выбор шаблона

ui-picker-new-slide = Новый слайд
ui-picker-set-mine = { $name } · мои
ui-picker-set-received = { $name } · полученные
ui-picker-keys = ← → — листать · Enter — добавить · Esc — закрыть
ui-picker-close = Закрыть
ui-picker-list = Макеты
ui-widget-picker-filter = Фильтр по имени, подписи или описанию
ui-widget-picker-empty = Нет виджетов, подходящих под «{ $query }».
ui-widget-picker-list = Виджеты
ui-widget-picker-keys = Стрелки — просмотр · Enter — добавить · Esc — закрыть

## Диаграмма

ui-chart-panel = Диаграмма
ui-chart-kinds = Тип диаграммы
ui-chart-kind = { $kind ->
    [line] Линии
    [pie] Круговая
    [scatter] Точечная
    [radar] Лепестковая
    [waterfall] Каскад
   *[bar] Столбцы
}
ui-chart-no-data = У этой диаграммы нет таблицы данных.
ui-chart-options = Параметры диаграммы
ui-chart-stack = С накоплением
ui-chart-waterfall-hint = Отметьте «Итого» для столбца итога.
ui-chart-colors = Цвета рядов
ui-chart-color-of = Цвет ряда { $name }
ui-chart-color-auto = Автоматически
ui-chart-color-pick = Цвет { $n }
ui-chart-column-name = Название столбца { $col }
ui-chart-remove-column = Удалить столбец
ui-chart-cell = Строка { $row }, столбец { $col }
ui-chart-remove-row = Удалить строку
ui-chart-add-row = Строка
ui-chart-add-column = Столбец
ui-chart-series = Ряд { $n }

## Таблица

ui-table-panel = Таблица
ui-table-options = Параметры таблицы
ui-table-header-row = Строка заголовков
ui-table-header-column = Первый столбец как заголовок
ui-table-striped = Полосатые строки
ui-table-alignment = Выравнивание столбцов
ui-table-align-of = Выравнивание: { $name }
ui-table-align = { $align ->
    [start] По началу
    [center] По центру
    [end] По концу
   *[auto] Автоматически
}
ui-table-column = Столбец { $n }
ui-table-no-data = В этой таблице нет данных.
ui-show-as = Показать как
ui-show-as-table = Показать в виде таблицы
ui-show-as-chart = Показать в виде диаграммы

## Файлы

ui-file-open-prompt = Путь к файлу для открытия (.oku-slides.pdf, .json или .pptx)
ui-file-filter-presentations = Презентации
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = Сохранить как (полный путь)

## Вкладки

ui-tabs = Открытые презентации
ui-tab-new = Новая вкладка
ui-tab-close = Закрыть вкладку
ui-tab-unsaved = Несохранённые изменения
ui-tab-pack = Набор
ui-tab-move-new-window = Переместить в новое окно
ui-tab-move-to = Переместить в окно { $n }
ui-tab-close-others = Закрыть другие вкладки
ui-close-cancel = Отмена
ui-disk-replaced-question = { $file } на диске заменён другой презентацией.
ui-disk-replaced-overwrite = Перезаписать
ui-disk-replaced-save-as = Сохранить как…
ui-same-deck-question = { $file } — версия презентации, открытой во вкладке «{ $title }».
ui-same-deck-merge = Объединить в этой вкладке
ui-same-deck-apart = Открыть отдельно
ui-history = История
ui-history-version = Версия от { $date }
ui-history-restore = Восстановить
ui-history-back = Назад
ui-history-anonymous = Аноним
ui-history-slider = Версии презентации
ui-history-removed-since = удалён с тех пор
ui-history-changed-since = изменён с тех пор
ui-history-added-since = слайдов добавлено с тех пор: { $count }
ui-help = Справка
ui-help-caption = Слайд { $index } из { $count } · только чтение
ui-slide-previous = Предыдущий слайд
ui-slide-next = Следующий слайд
ui-present = Презентация
ui-present-slide = Слайд { $index } из { $count }
ui-present-screen = Показать на весь экран
ui-help-close = Закрыть справку
ui-help-search = Поиск по справке
ui-help-search-placeholder = Поиск…
ui-help-search-empty = Ничего не найдено
ui-help-search-count = Результатов: { $count }
ui-widgets-files = Файлы виджетов
ui-widgets-common = Общее
ui-widgets-new = Новый виджет
ui-widgets-name = Имя виджета (строчные буквы, цифры и дефисы)
ui-widgets-name-taken = Это имя уже занято
ui-widgets-name-letter = Имя должно начинаться с буквы
ui-widgets-valid = Без ошибок
ui-widgets-errors = Ошибки
ui-widgets-empty = В этом пакете пока нет виджетов.
ui-widgets-preview = Предпросмотр виджета
ui-widgets-no-preview = Нет предпросмотра
ui-widgets-field-bound = Связано с презентацией: здесь только для чтения
ui-widgets-source = Исходный код
ui-widgets-unsaved = Не сохранено
ui-widgets-in-error = Виджеты с ошибками
ui-widgets-none-in-error = Нет виджетов с ошибками
ui-widgets-section = Виджеты
ui-widgets-open = Открыть виджет { $name }
ui-widgets-delete-named = Удалить { $name }
ui-widgets-confirm-delete = Удалить виджет «{ $name }»?
ui-widgets-rename-hint = Дважды щёлкните или нажмите F2, чтобы переименовать
ui-pack-tabs = Вкладки набора
ui-pack-layouts-tab = Макеты
ui-theme-tab = Тема
ui-theme-picker = Тема
ui-theme-name = Название темы
ui-theme-unsaved = Изменения темы не сохранены
ui-widgets-close-tab = Закрыть { $name }
ui-widgets-removed-by-agent = Агент удалил виджет { $name }.

## Импортировать

ui-import-title = Импорт презентации PowerPoint
ui-import-mode = Режим импорта
ui-import-hifi = Точная копия
ui-import-hifi-note = Сохраняет вид оригинала с темой и макетами, созданными для этой презентации.
ui-import-retrofit = Использовать мои макеты
ui-import-retrofit-note = Пересобирает слайды на наборах макетов и теме из библиотеки.
ui-import-layout-packs = Наборы макетов
ui-import-theme-default = Тема по умолчанию
ui-import-no-packs = В библиотеке нет набора макетов.
ui-import-tune = Доработать с агентом
ui-import-tune-note = Агент сравнивает каждый слайд с оригиналом и исправляет расхождения.
ui-import-tune-note-retrofit = Агент сравнивает каждый слайд с оригиналом и создаёт виджеты для фигур, которые ваши макеты не показывают.
ui-import-tune-unavailable = Нет доступного агента: { $reason }
ui-import-submit = Импортировать
ui-import-busy = Импорт…
ui-update-available = Доступна версия Oku Slides { $version }.
ui-update-install = Установить и перезапустить
ui-update-see-release = Посмотреть релиз
ui-update-later = Позже
ui-update-installing = Загрузка обновления… { $percent }%
ui-update-downloading = Загрузка обновления…
ui-update-up-to-date = Oku Slides обновлён до последней версии.
ui-update-check = Проверить обновления…

ui-shortcut-theme = тема, темы, theme, themes
