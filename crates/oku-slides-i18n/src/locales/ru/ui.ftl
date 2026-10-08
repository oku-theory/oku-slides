# Интерфейс приложения (React). Загружается ui/src/i18n.ts через запрос «messages».
ui-loading = Загрузка…

## Документ

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · шаблоны
ui-title-rename = Переименовать презентацию
ui-title-field = Название презентации
ui-status-unsaved = есть несохранённые изменения
ui-status-saved = сохранено
ui-status-never-saved = ещё не сохранено
ui-caption-templates = { $count ->
    [one] { $count } шаблон
    [few] { $count } шаблона
    [many] { $count } шаблонов
   *[other] { $count } шаблона
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } слайд
    [few] { $count } слайда
    [many] { $count } слайдов
   *[other] { $count } слайда
} · { $status }
ui-template-default-name = Шаблон
ui-template-variant-name = { $name } (вариант)
ui-confirm-quit-templates = Выйти, не сохранив изменённые шаблоны?

## Уведомления

ui-toast-templates-saved = Шаблоны сохранены: { $name }
ui-toast-moved = Презентация перемещена
ui-toast-copied = Копия сохранена
ui-toast-select-zone = Выделите область, чтобы разместить: «{ $text }»
ui-toast-field-gone = Поле, которое вы редактировали, больше не существует: введённый текст потерян.

## Панель инструментов

ui-toolbar-undo = Отменить
ui-toolbar-undo-label = Отменить: { $label }
ui-toolbar-redo = Повторить
ui-toolbar-redo-label = Повторить: { $label }
ui-toolbar-grid-hide = Скрыть сетку
ui-toolbar-grid-show = Показать сетку
ui-toolbar-ghosts-hide = Скрыть призраков
ui-toolbar-ghosts-show = Показать призраков
ui-divider-pile = Ширина стопки слайдов
ui-divider-notes = Ширина ленты агента
ui-divider-code = Размер кода
ui-toolbar-theme = Тема
ui-toolbar-theme-embedded = Тема из файла
ui-toolbar-save-copy = Сохранить копию
ui-toolbar-finish = Готово
ui-menu-workshop = Мастерская
ui-toolbar-open = Открыть
ui-merge = Объединить с PDF…
ui-menu = Меню
ui-menu-move = Переместить…
ui-language = Язык
ui-language-system = Язык системы
ui-scheme = Оформление
ui-scheme-system = Системное
ui-scheme-light = Светлое
ui-scheme-dark = Тёмное
ui-toolbar-add-label = Добавить на слайд
ui-add-kind = { $kind ->
    [card] Карточка
    [shape] Фигура
    [chart] Диаграмма
   *[text] Текст
}
ui-add-widget = Виджет пакета
ui-toolbar-remove-template = Удалить шаблон
ui-toolbar-remove-slide = Удалить слайд
ui-toolbar-remove-hint = Слайд { $n } · отменить: ⌘Z

## Подписи отмены

ui-undo-add = { $kind ->
    [card] Добавить карточку
    [shape] Добавить фигуру
    [chart] Добавить диаграмму
   *[text] Добавить текст
}
ui-undo-add-widget = Добавить «{ $name }»
ui-undo-new-template = Новый шаблон: { $name }
ui-undo-new-slide = Новый слайд: { $name }
ui-undo-templates = Шаблоны: { $name }
ui-undo-move-template = Переместить шаблон
ui-undo-move-slide = Переместить слайд
ui-undo-remove-slide = Удалить слайд
ui-undo-remove = Удалить
ui-undo-cut = Вырезать
ui-undo-paste = Вставить
ui-undo-place-on-grid = Разместить на сетке
ui-undo-move-on-grid = Переместить по сетке
ui-undo-resize-on-grid = Изменить размер по сетке
ui-undo-change-anchor = Изменить привязку
ui-undo-move = Переместить
ui-undo-resize = Изменить размер
ui-undo-edit-slide = Изменить слайд
ui-undo-dictation = Диктовка
ui-undo-rename-set = Переименовать набор
ui-undo-rename-deck = Переименовать презентацию
ui-undo-chart-kind = Сменить тип диаграммы
ui-undo-chart-data = Изменить данные
ui-undo-chart-color = Задать цвет ряда
ui-undo-chart-stack = { $stacked ->
    [yes] Сложить ряды
   *[no] Разделить ряды
}

## Новые элементы

ui-new-text = Новый текст
ui-new-card-title = Заголовок карточки
ui-new-card-detail = Подробности

## Стопка слайдов

ui-pile-slides = Слайды
ui-pile-templates = Шаблоны
ui-pile-add-slide = + Новый слайд
ui-pile-add-template = + Новый шаблон
ui-pile-drag-hint = Перетащите для перемещения · ⌥↑ ⌥↓
ui-slide-alt = Слайд { $n }

## Сцена

ui-stage-rendering = Отрисовка…
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
    [group] Группа
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
ui-draft = Черновик
ui-draft-insert = Вставить
ui-draft-insert-title = Вставить как есть (⌘⇧↩)
ui-draft-send = Отправить агенту
ui-draft-send-title = Отправить агенту (⌘↩)
ui-draft-discard = Удалить
ui-agent-state-idle = Агент
ui-agent-state-starting = Агент запускается…
ui-agent-state-working = Агент работает…
ui-agent-dock = В процессе
ui-agent-dock-settings = Настройки
ui-agent-jump = ↓ Текущий ход
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
ui-dictation-to-field = заменить: { $field }
ui-dictation-to-block = после выбранного блока
ui-dictation-new-block = новый текстовый блок
ui-dictation-select-zone = выделите область
ui-dictation-downloading = Загрузка модели { $name }
ui-dictation-download-progress = { $percent }% · { $received } из { $total } · только один раз
ui-dictation-listening = Слушаю · { $time }
ui-dictation-transcribing = Распознаю последнюю фразу…
ui-dictation-stop-in = Остановка через { $duration }
ui-dictation-speak = Говорите, текст появится здесь…
ui-dictation-auto-stop = остановится после { $duration } тишины
ui-dictation-to-download = { $size } для загрузки
ui-dictation-settings = Настройки диктовки
ui-dictation-model-title = Модель распознавания
ui-dictation-model-intro = Whisper на этом компьютере (CTranslate2). Каждая модель понимает несколько языков.
ui-dictation-note-turbo = Рекомендуется: быстрая и точная
ui-dictation-note-large = Самая точная, но медленнее
ui-dictation-note-base = Лёгкая, для слабых компьютеров
ui-dictation-installed = установлена
ui-dictation-not-installed = нужно загрузить
ui-dictation-language = Язык речи
ui-dictation-language-auto = Автоопределение
ui-dictation-note-apple = Самая быстрая, встроена в систему
ui-dictation-model-unavailable = Язык не поддерживается
ui-dictation-model-intro-apple = Apple, встроенная в систему, или Whisper на этом компьютере (CTranslate2). Все модели понимают несколько языков.
ui-dictation-language-auto-resolved = Автоматически ({ $language })
ui-dictation-to-download-language = Язык для загрузки
ui-dictation-auto-stop-label = Останавливаться самостоятельно после тишины длительностью
ui-dictation-silence = Длительность тишины
ui-dictation-phrases-note = Распознаются только произнесённые фрагменты, фраза за фразой, пока вы говорите.
ui-dictation-download-now = Загрузить сейчас ({ $size })
ui-dictation-download-language = Загрузить язык
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } с
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } ГБ
ui-size-mb = { $value } МБ

## Выбор шаблона

ui-picker-new-slide = Новый слайд
ui-picker-new-template = Новый шаблон на основе…
ui-picker-set = Шаблоны
ui-picker-set-mine = { $name } · мои
ui-picker-keys = ← → — листать · Enter — добавить · Esc — закрыть
ui-picker-close = Закрыть
ui-picker-previous = Предыдущий шаблон
ui-picker-next = Следующий шаблон
ui-picker-list = Шаблоны слайдов

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

## Файлы

ui-file-open-prompt = Путь к файлу для открытия (.oku-slides.pdf или .json)
ui-file-save-prompt = Сохранить как (полный путь)

## Вкладки

ui-tabs = Открытые презентации
ui-tab-new = Новая вкладка
ui-tab-close = Закрыть вкладку
ui-tab-unsaved = Несохранённые изменения
ui-tab-templates = Шаблоны
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
ui-toolbar-params-hide = Скрыть параметры
ui-toolbar-params-show = Показать параметры
ui-help-close = Закрыть справку
ui-help-search = Поиск по справке
ui-help-search-placeholder = Поиск…
ui-help-search-empty = Ничего не найдено
ui-help-search-count = Результатов: { $count }
ui-pack-notice-close = Закрыть
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
ui-widgets-tabs = Вкладки шаблонов
ui-widgets-templates-tab = Шаблоны
ui-theme-tab = Тема
ui-theme-picker = Тема
ui-theme-name = Название темы
ui-theme-unsaved = Изменения темы не сохранены
ui-widgets-close-tab = Закрыть { $name }
ui-widgets-removed-by-agent = Агент удалил виджет { $name }.
