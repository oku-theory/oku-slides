# Інтерфейс застосунку (React). Завантажується ui/src/i18n.ts через запит «messages».
ui-loading = Завантаження…

## Документ

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · набір макетів
   *[other] { $name } · набір віджетів
}
ui-title-rename = Перейменувати презентацію
ui-title-field = Назва презентації
ui-status-unsaved = є незбережені зміни
ui-status-saved = збережено
ui-status-never-saved = ще не збережено
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count } макет
        [few] { $count } макети
        [many] { $count } макетів
       *[other] { $count } макета
    }
   *[other] { $count ->
        [one] { $count } віджет
        [few] { $count } віджети
        [many] { $count } віджетів
       *[other] { $count } віджета
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } слайд
    [few] { $count } слайди
    [many] { $count } слайдів
   *[other] { $count } слайда
} · { $status }
ui-confirm-quit-pack = Вийти, не зберігши змінений набір?
ui-confirm-quit-theme = Вийти, не зберігши змінену тему?
ui-confirm-quit-read-only-theme = { $theme } не можна зберегти тут. Закрити цю вкладку і втратити її зміни?
ui-confirm-update-opener = Оновити «{ $deck }» до цієї версії набору «{ $pack }»?
ui-close-writes-pending = Ця вкладка ще зберігає: закрийте її знову за мить.
ui-confirm-close-writes-pending = Ця вкладка ще зберігає. Усе одно закрити її і втратити те, що не збережено?
ui-confirm-close-writes-pending-linked = Ця вкладка ще зберігає, і ці вкладки наборів закриються разом із нею без збереження: { $tabs }. Усе одно закрити їх і втратити те, що не збережено?
ui-confirm-quit-read-only-pack = { $pack } не можна зберегти тут. Закрити цю вкладку і втратити її зміни? Щоб зберегти їх, попросіть агента зберегти його як новий набір.
ui-confirm-discard-ok = Закрити без збереження
ui-confirm-update-ok = Оновити
ui-confirm-update-later = Не зараз

## Сповіщення

ui-toast-copied = Копію збережено
ui-toast-field-gone = Поля, яке ви редагували, більше немає: введений текст втрачено.
ui-toast-edit-replaced = Цей текст змінився, поки ви його редагували: ваша версія його замінила. «Скасувати» поверне іншу.

## Панель інструментів

ui-toolbar-undo = Скасувати
ui-toolbar-undo-label = Скасувати: { $label }
ui-toolbar-redo = Повторити
ui-toolbar-redo-label = Повторити: { $label }
ui-divider-pile = Ширина стосу слайдів
ui-divider-notes = Ширина стрічки агента
ui-divider-code = Розмір коду
ui-toolbar-theme = Тема
ui-toolbar-finish = Готово
ui-pack-deck-only = Зберігається лише в цій презентації
ui-toolbar-open = Відкрити
ui-menu = Меню
ui-menu-new-widget-pack = Новий пакет віджетів
ui-menu-new-layout-pack = Новий пакет макетів
ui-menu-new-theme = Нова тема
ui-toolbar-add-label = Додати на слайд
ui-toolbar-packs = Пакети презентації
ui-toolbar-packs-update = Пакети презентації — доступні оновлення
ui-toolbar-history = Показати історію
ui-toolbar-thumbs-show = Показати мініатюри
ui-toolbar-thumbs-hide = Сховати мініатюри
ui-toolbar-remove-slide = Видалити слайд
ui-toolbar-remove-hint = Слайд { $n } · скасувати: ⌘Z

## Підписи скасування

ui-undo-add-widget = Додати «{ $name }»
ui-undo-new-slide = Новий слайд: { $name }
ui-undo-move-slide = Перемістити слайд
ui-undo-remove-slide = Видалити слайд
ui-undo-remove = Видалити
ui-undo-paste = Вставити
ui-undo-place-on-grid = Розмістити на сітці
ui-undo-move-on-grid = Перемістити по сітці
ui-undo-resize-on-grid = Змінити розмір по сітці
ui-undo-change-anchor = Змінити прив’язку
ui-undo-move = Перемістити
ui-undo-resize = Змінити розмір
ui-undo-edit-slide = Змінити слайд
ui-undo-rename-pack = Перейменувати набір
ui-undo-rename-deck = Перейменувати презентацію

## Стос слайдів

ui-pile-slides = Слайди
ui-pile-layouts = Макети
ui-pile-add-slide = Новий слайд
ui-pile-drag-hint = Перетягніть, щоб перемістити · ⌥↑ ⌥↓
ui-slide-alt = Слайд { $n }

## Сцена

ui-stage-rendering = Відтворення…
ui-stage-elements = Елементи слайда
ui-stage-fields = Поля слайда
ui-stage-edit-text = Змінити текст
ui-stage-hint-plain = Enter або Tab — до наступного · Esc — скасувати
ui-stage-hint-rich = Tab — до наступного · ⌘ Enter — підтвердити · «- » для пункту списку · «# » для заголовка · ⌘B жирний, ⌘I курсив, ⌘K посилання
ui-field = { $kind ->
    [title] Заголовок
    [subtitle] Підзаголовок
    [kicker] Надзаголовок
    [text] Текст
    [body] Основний текст
    [author] Автор
    [date] Дата
    [organization] Організація
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Текст
    [card] Картка
    [image] Зображення
    [shape] Фігура
    [chart] Діаграма
    [table] Таблиця
    [stack] Стос
   *[other] { $kind }
}
ui-grid-cell = стовп. { $cols } · рядок { $rows }
ui-ghost-zone = Зона

## Нотатки до слайда

ui-notes = Інструменти слайда
ui-stepper-less = { $label }: на один менше
ui-stepper-more = { $label }: на один більше
ui-list-item = елемент
ui-list-add = Додати { $item }
ui-list-remove = Видалити { $item }
ui-list-move = Перемістити { $item }
ui-list-full = Список заповнено ({ $max })
ui-undo-list-move = Перемістити { $item }
ui-agent-color-other = Інший колір
ui-agent-thread = Агент
ui-agent-new-conversation = Нова розмова
ui-agent-undo-turn = Скасувати цей хід
ui-agent-detach = Відкрити агента в окремому вікні
ui-agent-attach = Повернути агента в головне вікно
ui-agent-window-empty = Для цієї вкладки немає агента
ui-agent-kept-in-draft = Нікуди вставити: текст збережено в чернетці
ui-draft = Чернетка
ui-draft-send = Надіслати агенту
ui-draft-send-title = Надіслати агенту (⌘↩)
ui-draft-discard = Видалити
ui-pasted-image = { $type } { $width }×{ $height }
ui-draft-images = Вставлені зображення
ui-draft-remove-image = Прибрати це зображення
ui-draft-send-needs-text = Скажіть, що з ним зробити
ui-agent-state-idle = Агент
ui-agent-state-starting = Агент запускається…
ui-agent-state-working = Агент працює…
ui-agent-dock = Триває
ui-agent-dock-settings = Налаштування
ui-agent-jump = ↓ Поточний хід
ui-agent-tool-input = Аргументи
ui-agent-tool-output = Результат
ui-agent-sheet = Запитання агента
ui-agent-sheet-keys = 1–4 вибрати · Enter підтвердити · E змінити · Esc скасувати
ui-agent-variants = Варіанти
ui-agent-drawer-keys = Клацання або 1–4 · Esc закриває
ui-agent-confirm = Підтвердження агента

## Диктування

ui-dictation = Диктування
ui-dictation-start = Диктувати
ui-dictation-start-title = Диктувати (⌘D)
ui-dictation-type = Написати агентові
ui-dictation-type-title = Написати агентові (⌘K)
ui-dictation-agent-stop = Зупинити агента (⌘⇧D або Esc)
ui-dictation-agent-working = Агент працює…
ui-dictation-agent-working-hint = ⌘⇧D або Esc зупиняє його
ui-dictation-finish = Завершити диктування
ui-dictation-finish-title = Завершити (⌘D)
ui-dictation-cancel = Скасувати
ui-dictation-preparing = Підготовка диктування…
ui-dictation-heard-nothing = Нічого не чути: говоріть трохи ближче до мікрофона
ui-dictation-to-cursor = у позицію курсора
ui-dictation-to-draft = у чернетку
ui-dictation-downloading = Завантаження моделі { $name }
ui-dictation-download-progress = { $percent }% · { $received } з { $total } · лише один раз
ui-dictation-listening = Слухаю · { $time }
ui-dictation-transcribing = Розпізнаю останню фразу…
ui-dictation-stop-in = Зупинка через { $duration }
ui-dictation-speak = Говоріть, текст з’явиться тут…
ui-dictation-auto-stop = зупиниться після { $duration } тиші
ui-dictation-to-download = { $size } для завантаження
ui-dictation-settings = Налаштування
ui-dictation-language-auto = Автовизначення
ui-dictation-model-unavailable = Мова не підтримується
ui-dictation-language-auto-resolved = Автоматично ({ $language })
ui-dictation-to-download-language = Мова для завантаження
ui-dictation-api-key-missing = { $var } не задано
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } с
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } ГБ
ui-size-mb = { $value } МБ

## Вибір шаблону

ui-picker-new-slide = Новий слайд
ui-picker-set-mine = { $name } · мої
ui-picker-set-received = { $name } · отримані
ui-picker-keys = ← → — гортати · Enter — додати · Esc — закрити
ui-picker-close = Закрити
ui-picker-list = Макети
ui-widget-picker-filter = Фільтр за назвою, підписом або описом
ui-widget-picker-empty = Жоден віджет не відповідає «{ $query }».
ui-widget-picker-list = Віджети
ui-widget-picker-keys = Стрілки — перегляд · Enter — додати · Esc — закрити

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
ui-chart-add-row = Рядок
ui-chart-add-column = Стовпець
ui-chart-series = Ряд { $n }

## Таблиця

ui-table-panel = Таблиця
ui-table-options = Параметри таблиці
ui-table-header-row = Рядок заголовків
ui-table-header-column = Перший стовпець як заголовок
ui-table-striped = Смугасті рядки
ui-table-alignment = Вирівнювання стовпців
ui-table-align-of = Вирівнювання: { $name }
ui-table-align = { $align ->
    [start] За початком
    [center] По центру
    [end] За кінцем
   *[auto] Автоматично
}
ui-table-column = Стовпець { $n }
ui-table-no-data = У цій таблиці немає даних.
ui-show-as = Показати як
ui-show-as-table = Показати у вигляді таблиці
ui-show-as-chart = Показати у вигляді діаграми

## Файли

ui-file-open-prompt = Шлях до файлу для відкриття (.oku-slides.pdf, .json або .pptx)
ui-file-filter-presentations = Презентації
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = Зберегти як (повний шлях)

## Вкладки

ui-tabs = Відкриті презентації
ui-tab-new = Нова вкладка
ui-tab-close = Закрити вкладку
ui-tab-unsaved = Незбережені зміни
ui-tab-kind-pack = { $title }, набір{ $dirty ->
    [yes] , { ui-tab-unsaved }
   *[no] { "" }
}
ui-tab-kind-theme = { $title }, тема{ $dirty ->
    [yes] , { ui-tab-unsaved }
   *[no] { "" }
}
ui-tab-theme = Тема
ui-preview-theme = Тема попереднього перегляду
ui-tab-move-new-window = Перемістити в нове вікно
ui-tab-move-to = Перемістити у вікно { $n }
ui-tab-close-others = Закрити інші вкладки
ui-close-cancel = Скасувати
ui-disk-replaced-question = { $file } на диску замінено іншою презентацією.
ui-disk-replaced-overwrite = Перезаписати
ui-disk-replaced-save-as = Зберегти як…
ui-same-deck-question = { $file } — версія презентації, відкритої у вкладці «{ $title }».
ui-same-deck-merge = Об’єднати в цій вкладці
ui-same-deck-apart = Відкрити окремо
ui-history = Історія
ui-history-version = Версія від { $date }
ui-history-restore = Відновити
ui-history-close = Закрити історію
ui-history-anonymous = Анонім
ui-history-slider = Версії презентації
ui-history-removed-since = видалено відтоді
ui-history-changed-since = змінено відтоді
ui-history-added-since = слайдів додано відтоді: { $count }
ui-help = Довідка
ui-help-caption = Слайд { $index } з { $count } · лише читання
ui-slide-previous = Попередній слайд
ui-slide-next = Наступний слайд
ui-present = Презентація
ui-present-slide = Слайд { $index } з { $count }
ui-present-screen = Показати на весь екран
ui-help-close = Закрити довідку
ui-help-search = Пошук у довідці
ui-help-search-placeholder = Пошук…
ui-help-search-empty = Нічого не знайдено
ui-help-search-count = Результатів: { $count }
ui-widgets-common = Спільне
ui-widgets-new = Новий віджет
ui-widgets-name = Назва віджета (малі літери, цифри та дефіси)
ui-layouts-name = Назва макета (малі літери, цифри та дефіси)
ui-widgets-name-taken = Ця назва вже зайнята
ui-widgets-name-letter = Назва має починатися з літери
ui-widgets-empty = У цьому пакеті ще немає віджетів.
ui-layouts-new = Новий макет
ui-layouts-empty = У цьому пакеті ще немає макетів.
ui-confirm-delete-ok = Видалити
ui-widgets-preview = Попередній перегляд віджета
ui-widgets-no-preview = Немає попереднього перегляду
ui-widgets-field-bound = Пов’язано з презентацією: тут лише для читання
ui-widgets-source = Вихідний код
ui-widgets-unsaved = Не збережено
ui-widgets-in-error = Віджети з помилками
ui-widgets-none-in-error = Жоден віджет не має помилок
ui-widgets-open = Відкрити віджет { $name }
ui-widgets-delete-named = Видалити { $name }
ui-widgets-confirm-delete = Видалити віджет «{ $name }»?
ui-layouts-confirm-delete = Видалити макет «{ $name }»?
ui-widgets-rename-hint = Двічі клацніть або натисніть F2, щоб перейменувати
ui-pile-menu = Дії для { $name }
ui-pile-menu-rename = Перейменувати
ui-pile-menu-delete = Видалити
ui-theme-name = Назва теми
ui-theme-unsaved = Зміни теми не збережено
ui-widgets-removed-by-agent = Агент видалив віджет { $name }.
ui-layouts-removed-by-agent = Агент видалив макет { $name }.

## Імпортувати

ui-import-title = Імпорт презентації PowerPoint
ui-import-mode = Режим імпорту
ui-import-hifi = Точна копія
ui-import-hifi-note = Зберігає вигляд оригіналу з темою та макетами, створеними для цієї презентації.
ui-import-retrofit = Використати мої макети
ui-import-retrofit-note = Перебудовує слайди на наборах макетів і темі з бібліотеки.
ui-import-layout-packs = Набори макетів
ui-import-theme-default = Тема за замовчуванням
ui-import-no-packs = У бібліотеці немає набору макетів.
ui-import-tune = Доопрацювати з агентом
ui-import-tune-note = Агент порівнює кожен слайд з оригіналом і виправляє розбіжності.
ui-import-tune-note-retrofit = Агент порівнює кожен слайд з оригіналом і створює віджети для фігур, яких ваші макети не показують.
ui-import-tune-unavailable = Немає доступного агента: { $reason }
ui-import-submit = Імпортувати
ui-import-busy = Імпорт…
ui-update-available = Доступна версія Oku Slides { $version }.
ui-update-install = Установити й перезапустити
ui-update-see-release = Переглянути випуск
ui-update-later = Пізніше
ui-update-installing = Завантаження оновлення… { $percent }%
ui-update-downloading = Завантаження оновлення…
ui-update-up-to-date = Oku Slides має найновішу версію.
ui-update-check = Перевірити оновлення…
ui-about = Про Oku Slides
ui-about-version = Версія { $version }
ui-about-changelog = Що нового
ui-about-breaking = несумісно
ui-about-close = Закрити

ui-shortcut-theme = тема, теми, theme, themes
