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
ui-status-builtin = встроенный: при сохранении создаётся копия
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

## Панель инструментов

ui-toolbar-undo = Отменить
ui-toolbar-undo-label = Отменить: { $label }
ui-toolbar-redo = Повторить
ui-toolbar-redo-label = Повторить: { $label }
ui-toolbar-grid-hide = Скрыть сетку
ui-toolbar-grid-show = Показать сетку
ui-toolbar-theme = Тема
ui-toolbar-theme-default = По умолчанию
ui-toolbar-save-copy = Сохранить копию
ui-toolbar-finish = Готово
ui-toolbar-templates = Шаблоны
ui-toolbar-open = Открыть
ui-merge = Объединить с PDF…
ui-menu = Меню
ui-menu-move = Переместить…
ui-language = Язык
ui-language-system = Язык системы
ui-toolbar-add-label = Добавить на слайд
ui-add-kind = { $kind ->
    [card] Карточка
    [shape] Фигура
    [chart] Диаграмма
   *[text] Текст
}
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
ui-undo-new-template = Новый шаблон: { $name }
ui-undo-new-slide = Новый слайд: { $name }
ui-undo-grid = Сетка { $columns } × { $rows }
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
ui-undo-edit-text = Изменить текст
ui-undo-dictation = Диктовка
ui-undo-rename-set = Переименовать набор
ui-undo-rename-deck = Переименовать презентацию
ui-undo-theme = Сменить тему
ui-undo-rename-template = Переименовать шаблон
ui-undo-describe-template = Описать шаблон
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
ui-stage-edit-text = Изменить текст
ui-stage-hint-field = Enter — подтвердить · Esc — отменить
ui-stage-hint-block = ⌘ Enter — подтвердить · одна строка на абзац, «- » для пункта списка · ⌘B жирный, ⌘I курсив, ⌘K ссылка
ui-field = { $kind ->
    [title] Заголовок
    [subtitle] Подзаголовок
    [kicker] Надзаголовок
    [text] Текст
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

## Заметки к слайду

ui-notes = Инструменты слайда
ui-stepper-less = { $label }: на один меньше
ui-stepper-more = { $label }: на один больше
ui-agent-color-other = Другой цвет
ui-agent-thread = Агент
ui-agent-new-conversation = Новый разговор
ui-agent-tools = { $count ->
    [one] { $count } инструмент
    [few] { $count } инструмента
   *[other] { $count } инструментов
}
ui-draft = Черновик
ui-draft-insert = Вставить
ui-draft-insert-title = Вставить как есть (⌘⇧↩)
ui-draft-send = Отправить агенту
ui-draft-send-title = Отправить агенту (⌘↩)
ui-draft-discard = Удалить
ui-agent-state-idle = Агент
ui-agent-state-starting = Агент запускается…
ui-agent-state-working = Агент работает…
ui-agent-stopped = Остановлено.
ui-agent-ended = Ход завершён ({ $stop }).
ui-agent-stop-max-tokens = Агент достиг предела длины.
ui-agent-stop-max-turns = Агент достиг предела шагов.
ui-agent-stop-refusal = Агент отказался.
ui-agent-sheet = Вопрос агента
ui-agent-sheet-keys = 1–4 выбрать · Enter подтвердить · E изменить · Esc отменить
ui-agent-variants = Варианты
ui-agent-drawer-keys = Щелчок или 1–4 · Esc закрывает
ui-agent-confirm = Подтверждение агента

## Диктовка

ui-dictation = Диктовка
ui-dictation-start = Диктовать
ui-dictation-start-title = Диктовать (⌘D)
ui-dictation-agent-stop = Остановить агента (⌘⇧D или Esc)
ui-dictation-agent-working = Агент работает… ⌘⇧D или Esc остановит его
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
ui-dictation-auto-stop-label = Останавливаться самостоятельно после тишины длительностью
ui-dictation-silence = Длительность тишины
ui-dictation-phrases-note = Распознаются только произнесённые фрагменты, фраза за фразой, пока вы говорите.
ui-dictation-download-now = Загрузить сейчас ({ $size })
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
ui-chart-add-row = + Строка
ui-chart-add-column = + Столбец
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
