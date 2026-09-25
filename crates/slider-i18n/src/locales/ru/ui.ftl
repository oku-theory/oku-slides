# Интерфейс приложения (React). Загружается ui/src/i18n.ts через запрос «messages».
ui-loading = Загрузка…

## Документ

ui-brand = Écho
ui-brand-templates = Écho · шаблоны
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

ui-toast-saved = Презентация сохранена
ui-toast-templates-saved = Шаблоны сохранены: { $name }
ui-toast-select-zone = Выделите область, чтобы разместить: «{ $text }»

## Панель инструментов

ui-toolbar-undo = Отменить
ui-toolbar-undo-label = Отменить: { $label }
ui-toolbar-redo = Повторить
ui-toolbar-redo-label = Повторить: { $label }
ui-toolbar-grid-hide = Скрыть сетку
ui-toolbar-grid-show = Показать сетку
ui-toolbar-grid-hide-title = Скрыть сетку (⌘')
ui-toolbar-grid-show-title = Показать сетку (⌘')
ui-toolbar-theme = Тема
ui-toolbar-theme-default = По умолчанию
ui-toolbar-save-copy = Сохранить копию
ui-toolbar-save = Сохранить
ui-toolbar-finish = Готово
ui-toolbar-templates = Шаблоны
ui-toolbar-templates-title = Создание и редактирование шаблонов слайдов
ui-toolbar-open = Открыть

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
ui-stage-hint-block = ⌘ Enter — подтвердить · одна строка на абзац, «- » для пункта списка
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
ui-note-template = Этот шаблон
ui-note-template-name = Название
ui-note-template-description = Описание
ui-note-add-label = Добавить на слайд
ui-note-add = Добавить
ui-add-kind = { $kind ->
    [card] Карточка
    [shape] Фигура
    [chart] Диаграмма
   *[text] Текст
}
ui-note-grid-label = Сетка шаблона
ui-note-grid = Сетка
ui-note-grid-columns = Столбцы
ui-note-grid-rows = Строки
ui-stepper-less = { $label }: на один меньше
ui-stepper-more = { $label }: на один больше
ui-note-remove-template = Удалить шаблон
ui-note-remove-slide = Удалить слайд
ui-note-remove-hint = Слайд { $n } · отменить: ⌘Z
ui-note-tip = Совет
ui-note-tip-text = ⌘D — диктовка. ⌘' — показать сетку. Двойной щелчок — изменить текст.

## Диктовка

ui-dictation = Диктовка
ui-dictation-start = Диктовать
ui-dictation-start-title = Диктовать (⌘D)
ui-dictation-finish = Завершить диктовку
ui-dictation-finish-title = Завершить (⌘D)
ui-dictation-cancel = Отмена
ui-dictation-preparing = Подготовка диктовки…
ui-dictation-heard-nothing = Ничего не слышно: говорите чуть ближе к микрофону
ui-dictation-to-cursor = в позицию курсора
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
   *[bar] Столбцы
}
ui-chart-no-data = У этой диаграммы нет таблицы данных.
ui-chart-column-name = Название столбца { $col }
ui-chart-remove-column = Удалить столбец
ui-chart-cell = Строка { $row }, столбец { $col }
ui-chart-remove-row = Удалить строку
ui-chart-add-row = + Строка
ui-chart-add-column = + Столбец
ui-chart-series = Ряд { $n }

## Файлы

ui-file-open-prompt = Путь к файлу для открытия (.echo.pdf или .json)
ui-file-save-prompt = Сохранить как (полный путь)
