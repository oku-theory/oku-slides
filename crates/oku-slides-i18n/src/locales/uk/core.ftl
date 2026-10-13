op-slide-not-found = Слайд не знайдено: { $subject }
op-element-not-found = Елемент не знайдено: { $subject }
op-duplicate-id = Ідентифікатор уже використовується: { $subject }
op-index-out-of-range = Позиція поза допустимим діапазоном
op-invalid-value = Недопустиме значення для { $subject }
op-invalid-value-because = Недопустиме значення для { $subject }: { $reason }
op-value-no-value = параметр не має значення
op-value-no-dataset = діаграма не має набору даних
op-value-grid-tracks = у сітці від 1 до 24 стовпців і рядків
op-value-bad-item-id = ідентифікатор пункту — непорожній текст без «/»
op-value-missing-item-id = кожен пункт потребує ідентифікатора
op-value-too-deep = елементи вкладено надто глибоко
op-value-empty-path = шлях порожній
op-value-not-text = очікується текст або null
op-value-empty-frame = ширина й висота мають бути більшими за 0
op-value-blocks-off-grid = слоти потоку розкладає слайд, а сітка їх приховує; спершу перемістіть кожен елемент у вільний слот
op-value-outside-grid = клітинка поза сіткою
op-value-not-tokens = очікується об'єкт або null
op-value-not-a-slot = цей параметр не є слотом
op-value-out-of-bounds = число поза допустимим діапазоном
op-value-unreadable = значення має неочікувану форму
op-value-valign = очікується top, middle або bottom
op-value-path = шлях складається з команд M, L, C, Q і Z з їхніми числами й починається з M
op-unknown-widget = Невідомий віджет: { $subject }
op-unknown-param = Невідомий параметр: { $subject }
op-param-type-because = Недопустиме значення для { $subject }: { $reason }
op-param-shape-text = { $nullable ->
    [yes] очікується текст або null
   *[no] очікується текст
}
op-param-shape-choice = { $nullable ->
    [yes] очікується одне з його значень або null
   *[no] очікується одне з його значень
}
op-param-shape-paragraphs = { $nullable ->
    [yes] очікується список абзаців або null
   *[no] очікується список абзаців
}
op-param-shape-elements = { $nullable ->
    [yes] очікується список елементів або null
   *[no] очікується список елементів
}
op-param-shape-flag = { $nullable ->
    [yes] очікується логічне значення або null
   *[no] очікується логічне значення
}
op-param-shape-number = { $nullable ->
    [yes] очікується число або null
   *[no] очікується число
}
op-param-shape-bounded-number = { $nullable ->
    [yes] очікується число в допустимих межах або null
   *[no] очікується число в допустимих межах
}
op-param-shape-grid = { $nullable ->
    [yes] очікується сітка або null
   *[no] очікується сітка
}
op-param-shape-chart-option = { $nullable ->
    [yes] очікується параметр діаграми або null
   *[no] очікується параметр діаграми
}
op-param-shape-table-option = { $nullable ->
    [yes] очікується параметр таблиці або null
   *[no] очікується параметр таблиці
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] очікується параметр діаграми або таблиці або null
   *[no] очікується параметр діаграми або таблиці
}
op-param-shape-items = { $nullable ->
    [yes] очікується список пунктів або null
   *[no] очікується список пунктів
}
op-param-shape-list = { $nullable ->
    [yes] очікується список або null
   *[no] очікується список
}
op-param-shape-add-element-only = { $nullable ->
    [yes] елементи додаються через add_element, або null
   *[no] елементи додаються через add_element
}
op-param-shape-set-param-only = { $nullable ->
    [yes] значення задається через set_param, або null
   *[no] значення задається через set_param
}
op-param-shape-path-option = { $nullable ->
    [yes] очікується шлях фігури (об'єкт із ключем d) або null
   *[no] очікується шлях фігури (об'єкт із ключем d)
}
op-item-not-found = Немає пункту { $subject }
op-list-full = Список { $subject } заповнено (не більше { $max ->
   [one] { $max } пункту
   [few] { $max } пунктів
   [many] { $max } пунктів
  *[other] { $max } пункту
})
op-list-minimum = У списку { $subject } має бути щонайменше { $min ->
   [one] { $min } пункт
   [few] { $min } пункти
   [many] { $min } пунктів
  *[other] { $min } пункту
}
op-list-cell-too-long = Клітинка списку { $subject } надто довга (стовпець { $column })
op-list-id-too-long = Ідентифікатор пункту списку { $subject } надто довгий
op-list-too-many-items = У списку { $subject } надто багато пунктів (не більше { $max })
op-slot-rejects = { $subject } тут не приймається
op-wrong-category = { $subject } не можна використати тут
op-choice-out-of-range = Значення не дозволене для { $subject }
transaction-refused = Операцію { $index } відхилено: { $reason }

sources-unreadable = Вихідні дані не читаються
sources-not-sources = Це не архів вихідних даних Oku Slides
sources-unsupported-version = Ці вихідні дані створено новішою версією Oku Slides
sources-bad-name = Неприпустима назва файлу у вихідних даних
sources-missing-deck = У вихідних даних немає презентації
sources-too-large = Вихідні дані в цьому файлі завеликі, щоб їх відкрити
sources-too-deep = У цій презентації елементи вкладено надто глибоко

issue-empty-slide-id = порожній ідентифікатор слайда
issue-duplicate-slide-id = повторюваний ідентифікатор слайда
issue-duplicate-element-id = повторюваний ідентифікатор елемента
issue-empty-element-id = порожній ідентифікатор
issue-cell-outside-grid = клітинка поза сіткою
issue-empty-frame = рамка нульового розміру
issue-free-without-frame = вільний елемент без рамки
issue-unknown-widget = невідомий віджет: { $kind }
issue-wrong-category = { $kind } не можна використати тут
issue-unknown-param = невідомий параметр: { $param }
issue-param-type = недійсне значення для { $param }
issue-choice-out-of-range = значення не дозволене для { $param }
issue-slot-rejects = { $kind } тут не приймається
issue-placed-in-flow = елемент із рамкою у впорядкованому списку
issue-unknown-bind = { $param } пов'язано з невідомим налаштуванням презентації
issue-slot-too-deep = елементи вкладено надто глибоко
issue-invalid-style = неприпустиме значення стилю: { $field }
issue-missing-asset = бракує ресурсу: { $asset }
issue-pack-version-mismatch = змінену версію пакета відкинуто: { $version }
issue-pack-version-missing = версії пакета немає в джерелах: { $version }
issue-theme-version-mismatch = змінену версію теми відкинуто: { $version }
issue-theme-version-missing = версії теми немає в джерелах: { $version }
issue-unsupported-link = непідтримуване посилання: { $link }
issue-highlight-zero = кольори виділення нумеруються з 1
issue-unreadable-element = нечитабельний елемент, відкладено
issue-slot-repaired = елемент без відомого слота, розміщено в { $slot }
issue-unreadable-item = нечитабельний пункт списку, відкладено
issue-unreadable-row = нечитабельний рядок таблиці, відкладено
issue-data-truncated = дані завеликі, обрізано під час читання
issue-unreadable-slide = нечитабельний слайд, відкладено
issue-unreadable-field = нечитабельне поле { $field } презентації, скинуто
op-invalid-chart = Діаграма { $subject }: { $reason }
op-invalid-table = Таблиця { $subject }: { $reason }

replica-corrupted = Історія презентації в цьому файлі пошкоджена
replica-too-large = Історія презентації в цьому файлі завелика, щоб її відкрити
replica-write = Не вдалося оновити історію презентації
replica-unrelated = Ці дві презентації не мають спільного походження
replica-unsupported-version = Ця презентація використовує формат, який ця версія Oku Slides не може прочитати

widget-unreadable = Не вдалося прочитати оголошення віджетів
widget-fault-empty-name = Віджет { $widget }: у віджета або параметра немає імені
widget-fault-duplicate-widget = Віджет { $widget } оголошено двічі
widget-fault-duplicate-param = Віджет { $widget } двічі оголошує той самий параметр
widget-fault-empty-choice = Віджет { $widget }: варіант вибору не має значень
widget-fault-default-not-in-choice = Віджет { $widget }: типове значення вибору не входить до його значень
widget-fault-default-wrong-type = Віджет { $widget }: типове значення не відповідає типу свого параметра
widget-fault-bind-not-plain = Віджет { $widget }: лише параметри звичайного тексту можна прив'язати до презентації
widget-fault-unknown-slot-kind = Віджет { $widget }: слот приймає невідомий віджет елемента
widget-fault-empty-list = Віджет { $widget }: списку потрібна принаймні одна колонка
widget-fault-column-type = Віджет { $widget }: колонка списку може бути plain, rich, number, flag, choice або asset; для довільного вмісту пункту використовуйте p.slot із дочірнім віджетом
widget-fault-reserved-name = Віджет { $widget }: зарезервоване ім'я: id, kind або ім'я, що містить /
widget-fault-list-bounds = Віджет { $widget }: максимум списку менший за його мінімум
widget-fault-list-default-too-large = Віджет { $widget }: типове значення списку перевищує обмеження розміру списків
widget-incompatible-override = Віджет { $widget } замінює вбудований, але змінює або видаляє його параметр { $param }
widget-incompatible-category = Віджет { $widget } замінює вбудований віджетом іншої категорії
issue-empty-item-id = у списку { $param } є пункт без ідентифікатора
issue-duplicate-item-id = у списку { $param } повторюється ідентифікатор пункту
issue-list-bounds = у списку { $param } забагато або замало пунктів
