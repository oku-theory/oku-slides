# Інтерфейс застосунку (React). Завантажується ui/src/i18n.ts через запит «messages».
ui-loading = Завантаження…

## Документ

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · шаблони
ui-title-rename = Перейменувати презентацію
ui-title-field = Назва презентації
ui-status-unsaved = є незбережені зміни
ui-status-saved = збережено
ui-status-never-saved = ще не збережено
ui-status-builtin = вбудований: під час збереження створюється копія
ui-caption-templates = { $count ->
    [one] { $count } шаблон
    [few] { $count } шаблони
    [many] { $count } шаблонів
   *[other] { $count } шаблону
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } слайд
    [few] { $count } слайди
    [many] { $count } слайдів
   *[other] { $count } слайда
} · { $status }
ui-template-default-name = Шаблон
ui-template-variant-name = { $name } (варіант)
ui-confirm-quit-templates = Вийти, не зберігши змінені шаблони?

## Сповіщення

ui-toast-saved = Презентацію збережено
ui-toast-templates-saved = Шаблони збережено: { $name }
ui-toast-select-zone = Виділіть область, щоб розмістити: «{ $text }»

## Панель інструментів

ui-toolbar-undo = Скасувати
ui-toolbar-undo-label = Скасувати: { $label }
ui-toolbar-redo = Повторити
ui-toolbar-redo-label = Повторити: { $label }
ui-toolbar-grid-hide = Сховати сітку
ui-toolbar-grid-show = Показати сітку
ui-toolbar-grid-hide-title = Сховати сітку (⌘')
ui-toolbar-grid-show-title = Показати сітку (⌘')
ui-toolbar-theme = Тема
ui-toolbar-theme-default = Типова
ui-toolbar-save-copy = Зберегти копію
ui-toolbar-save = Зберегти
ui-toolbar-finish = Готово
ui-toolbar-templates = Шаблони
ui-toolbar-templates-title = Створення та редагування шаблонів слайдів
ui-toolbar-open = Відкрити
ui-merge = Об’єднати з PDF…
ui-toolbar-add-label = Додати на слайд
ui-toolbar-add = Додати
ui-add-kind = { $kind ->
    [card] Картка
    [shape] Фігура
    [chart] Діаграма
   *[text] Текст
}
ui-toolbar-remove-template = Видалити шаблон
ui-toolbar-remove-slide = Видалити слайд
ui-toolbar-remove-hint = Слайд { $n } · скасувати: ⌘Z

## Підписи скасування

ui-undo-add = { $kind ->
    [card] Додати картку
    [shape] Додати фігуру
    [chart] Додати діаграму
   *[text] Додати текст
}
ui-undo-new-template = Новий шаблон: { $name }
ui-undo-new-slide = Новий слайд: { $name }
ui-undo-grid = Сітка { $columns } × { $rows }
ui-undo-templates = Шаблони: { $name }
ui-undo-move-template = Перемістити шаблон
ui-undo-move-slide = Перемістити слайд
ui-undo-remove-slide = Видалити слайд
ui-undo-remove = Видалити
ui-undo-cut = Вирізати
ui-undo-paste = Вставити
ui-undo-place-on-grid = Розмістити на сітці
ui-undo-move-on-grid = Перемістити по сітці
ui-undo-resize-on-grid = Змінити розмір по сітці
ui-undo-change-anchor = Змінити прив’язку
ui-undo-move = Перемістити
ui-undo-resize = Змінити розмір
ui-undo-edit-text = Змінити текст
ui-undo-dictation = Диктування
ui-undo-rename-set = Перейменувати набір
ui-undo-rename-deck = Перейменувати презентацію
ui-undo-theme = Змінити тему
ui-undo-rename-template = Перейменувати шаблон
ui-undo-describe-template = Описати шаблон
ui-undo-chart-kind = Змінити тип діаграми
ui-undo-chart-data = Змінити дані
ui-undo-chart-color = Задати колір ряду
ui-undo-chart-stack = { $stacked ->
    [yes] Скласти ряди
   *[no] Розділити ряди
}

## Нові елементи

ui-new-text = Новий текст
ui-new-card-title = Заголовок картки
ui-new-card-detail = Подробиці

## Стос слайдів

ui-pile-slides = Слайди
ui-pile-templates = Шаблони
ui-pile-add-slide = + Новий слайд
ui-pile-add-template = + Новий шаблон
ui-pile-drag-hint = Перетягніть, щоб перемістити · ⌥↑ ⌥↓
ui-slide-alt = Слайд { $n }

## Сцена

ui-stage-rendering = Відтворення…
ui-stage-elements = Елементи слайда
ui-stage-edit-text = Змінити текст
ui-stage-hint-field = Enter — підтвердити · Esc — скасувати
ui-stage-hint-block = ⌘ Enter — підтвердити · один рядок на абзац, «- » для пункту списку · ⌘B жирний, ⌘I курсив, ⌘K посилання
ui-field = { $kind ->
    [title] Заголовок
    [subtitle] Підзаголовок
    [kicker] Надзаголовок
    [text] Текст
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Текст
    [card] Картка
    [image] Зображення
    [shape] Фігура
    [group] Група
   *[other] { $kind }
}
ui-grid-cell = стовп. { $cols } · рядок { $rows }

## Нотатки до слайда

ui-notes = Інструменти слайда
ui-stepper-less = { $label }: на один менше
ui-stepper-more = { $label }: на один більше

## Диктування

ui-dictation = Диктування
ui-dictation-start = Диктувати
ui-dictation-start-title = Диктувати (⌘D)
ui-dictation-finish = Завершити диктування
ui-dictation-finish-title = Завершити (⌘D)
ui-dictation-cancel = Скасувати
ui-dictation-preparing = Підготовка диктування…
ui-dictation-heard-nothing = Нічого не чути: говоріть трохи ближче до мікрофона
ui-dictation-to-cursor = у позицію курсора
ui-dictation-to-field = замінити: { $field }
ui-dictation-to-block = після вибраного блоку
ui-dictation-new-block = новий текстовий блок
ui-dictation-select-zone = виділіть область
ui-dictation-downloading = Завантаження моделі { $name }
ui-dictation-download-progress = { $percent }% · { $received } з { $total } · лише один раз
ui-dictation-listening = Слухаю · { $time }
ui-dictation-transcribing = Розпізнаю останню фразу…
ui-dictation-stop-in = Зупинка через { $duration }
ui-dictation-speak = Говоріть, текст з’явиться тут…
ui-dictation-auto-stop = зупиниться після { $duration } тиші
ui-dictation-to-download = { $size } для завантаження
ui-dictation-settings = Налаштування диктування
ui-dictation-model-title = Модель розпізнавання
ui-dictation-model-intro = Whisper на цьому комп’ютері (CTranslate2). Кожна модель розуміє кілька мов.
ui-dictation-note-turbo = Рекомендовано: швидка й точна
ui-dictation-note-large = Найточніша, але повільніша
ui-dictation-note-base = Легка, для слабких комп’ютерів
ui-dictation-installed = встановлено
ui-dictation-not-installed = потрібно завантажити
ui-dictation-language = Мова мовлення
ui-dictation-language-auto = Автовизначення
ui-dictation-auto-stop-label = Зупинятися самостійно після тиші тривалістю
ui-dictation-silence = Тривалість тиші
ui-dictation-phrases-note = Розпізнаються лише промовлені фрагменти, фраза за фразою, поки ви говорите.
ui-dictation-download-now = Завантажити зараз ({ $size })
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } с
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } ГБ
ui-size-mb = { $value } МБ

## Вибір шаблону

ui-picker-new-slide = Новий слайд
ui-picker-new-template = Новий шаблон на основі…
ui-picker-set = Шаблони
ui-picker-set-mine = { $name } · мої
ui-picker-keys = ← → — гортати · Enter — додати · Esc — закрити
ui-picker-close = Закрити
ui-picker-previous = Попередній шаблон
ui-picker-next = Наступний шаблон
ui-picker-list = Шаблони слайдів

## Діаграма

ui-chart-panel = Діаграма
ui-chart-kinds = Тип діаграми
ui-chart-kind = { $kind ->
    [line] Лінії
    [pie] Кругова
    [scatter] Точкова
    [radar] Пелюсткова
    [waterfall] Каскад
   *[bar] Стовпці
}
ui-chart-no-data = Ця діаграма не має таблиці даних.
ui-chart-options = Параметри діаграми
ui-chart-stack = З накопиченням
ui-chart-waterfall-hint = Позначте «Разом» для стовпця підсумку.
ui-chart-colors = Кольори рядів
ui-chart-color-of = Колір ряду { $name }
ui-chart-color-auto = Автоматично
ui-chart-color-pick = Колір { $n }
ui-chart-column-name = Назва стовпця { $col }
ui-chart-remove-column = Видалити стовпець
ui-chart-cell = Рядок { $row }, стовпець { $col }
ui-chart-remove-row = Видалити рядок
ui-chart-add-row = + Рядок
ui-chart-add-column = + Стовпець
ui-chart-series = Ряд { $n }

## Файли

ui-file-open-prompt = Шлях до файлу для відкриття (.oku-slides.pdf або .json)
ui-file-save-prompt = Зберегти як (повний шлях)

## Вкладки

ui-tabs = Відкриті презентації
ui-tab-new = Нова вкладка
ui-tab-close = Закрити вкладку
ui-tab-unsaved = Незбережені зміни
ui-tab-templates = Шаблони
ui-tab-move-new-window = Перемістити в нове вікно
ui-tab-move-to = Перемістити у вікно { $n }
ui-tab-close-others = Закрити інші вкладки
ui-close-unsaved = Зберегти зміни в «{ $name }» перед закриттям?
ui-close-save = Зберегти
ui-close-discard = Не зберігати
ui-close-cancel = Скасувати
ui-disk-replaced-question = { $file } на диску замінено іншою презентацією.
ui-disk-replaced-overwrite = Перезаписати
ui-disk-replaced-save-as = Зберегти як…
ui-same-deck-question = { $file } — версія презентації, відкритої у вкладці «{ $title }».
ui-same-deck-merge = Об’єднати в цій вкладці
ui-same-deck-apart = Відкрити окремо
