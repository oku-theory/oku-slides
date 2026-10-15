op-slide-not-found = Слайд не найден: { $subject }
op-element-not-found = Элемент не найден: { $subject }
op-duplicate-id = Идентификатор уже используется: { $subject }
op-index-out-of-range = Позиция вне допустимого диапазона
op-invalid-value = Недопустимое значение для { $subject }
op-invalid-value-because = Недопустимое значение для { $subject }: { $reason }
op-value-no-value = у параметра нет значения
op-value-no-dataset = у диаграммы нет набора данных
op-value-grid-tracks = в сетке от 1 до 24 столбцов и строк
op-value-bad-item-id = идентификатор пункта — непустой текст без «/»
op-value-missing-item-id = у каждого пункта должен быть идентификатор
op-value-too-deep = элементы вложены слишком глубоко
op-value-empty-path = путь пуст
op-value-not-text = ожидается текст или null
op-value-empty-frame = ширина и высота должны быть больше 0
op-value-blocks-off-grid = слоты потока раскладывает слайд, а сетка их скрывает; сначала переместите каждый элемент в свободный слот
op-value-outside-grid = ячейка вне сетки
op-value-not-tokens = ожидается объект или null
op-value-not-a-slot = этот параметр не является слотом
op-value-out-of-bounds = число вне допустимого диапазона
op-value-unreadable = значение имеет неожиданную форму
op-value-valign = ожидается top, middle или bottom
op-value-path = путь состоит из команд M, L, C, Q и Z с их числами и начинается с M
op-unknown-widget = Неизвестный виджет: { $subject }
op-unknown-param = Неизвестный параметр: { $subject }
op-param-type-because = Недопустимое значение для { $subject }: { $reason }
op-param-shape-text = { $nullable ->
    [yes] ожидается текст или null
   *[no] ожидается текст
}
op-param-shape-choice = { $nullable ->
    [yes] ожидается одно из его значений или null
   *[no] ожидается одно из его значений
}
op-param-shape-paragraphs = { $nullable ->
    [yes] ожидается список абзацев или null
   *[no] ожидается список абзацев
}
op-param-shape-elements = { $nullable ->
    [yes] ожидается список элементов или null
   *[no] ожидается список элементов
}
op-param-shape-flag = { $nullable ->
    [yes] ожидается логическое значение или null
   *[no] ожидается логическое значение
}
op-param-shape-number = { $nullable ->
    [yes] ожидается число или null
   *[no] ожидается число
}
op-param-shape-bounded-number = { $nullable ->
    [yes] ожидается число в допустимых границах или null
   *[no] ожидается число в допустимых границах
}
op-param-shape-grid = { $nullable ->
    [yes] ожидается сетка или null
   *[no] ожидается сетка
}
op-param-shape-chart-option = { $nullable ->
    [yes] ожидается параметр диаграммы или null
   *[no] ожидается параметр диаграммы
}
op-param-shape-table-option = { $nullable ->
    [yes] ожидается параметр таблицы или null
   *[no] ожидается параметр таблицы
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] ожидается параметр диаграммы или таблицы или null
   *[no] ожидается параметр диаграммы или таблицы
}
op-param-shape-items = { $nullable ->
    [yes] ожидается список пунктов или null
   *[no] ожидается список пунктов
}
op-param-shape-list = { $nullable ->
    [yes] ожидается список или null
   *[no] ожидается список
}
op-param-shape-add-element-only = { $nullable ->
    [yes] элементы добавляются через add_element, или null
   *[no] элементы добавляются через add_element
}
op-param-shape-set-param-only = { $nullable ->
    [yes] значение задаётся через set_param, или null
   *[no] значение задаётся через set_param
}
op-param-shape-path-option = { $nullable ->
    [yes] ожидается путь фигуры (объект с ключом d) или null
   *[no] ожидается путь фигуры (объект с ключом d)
}
op-item-not-found = Нет пункта { $subject }
op-list-full = Список { $subject } заполнен (не более { $max ->
   [one] { $max } пункта
   [few] { $max } пунктов
   [many] { $max } пунктов
  *[other] { $max } пункта
})
op-list-minimum = В списке { $subject } должно быть не менее { $min ->
   [one] { $min } пункта
   [few] { $min } пунктов
   [many] { $min } пунктов
  *[other] { $min } пункта
}
op-list-cell-too-long = Ячейка списка { $subject } слишком длинная (столбец { $column })
op-list-id-too-long = Идентификатор пункта списка { $subject } слишком длинный
op-list-too-many-items = В списке { $subject } слишком много пунктов (не более { $max })
op-slot-rejects = { $subject } здесь не допускается
op-wrong-category = { $subject } нельзя использовать здесь
op-choice-out-of-range = Значение не разрешено для { $subject }
transaction-refused = Операция { $index } отклонена: { $reason }

sources-unreadable = Исходники не читаются
sources-not-sources = Это не архив исходников Oku Slides
sources-unsupported-version = Эти исходники созданы более новой версией Oku Slides
sources-bad-name = Недопустимое имя файла в исходниках
sources-missing-deck = В исходниках нет презентации
sources-too-large = Исходники в этом файле слишком велики для открытия
sources-too-deep = В этой презентации элементы вложены слишком глубоко

issue-empty-slide-id = пустой идентификатор слайда
issue-duplicate-slide-id = повторяющийся идентификатор слайда
issue-duplicate-element-id = повторяющийся идентификатор элемента
issue-empty-element-id = пустой идентификатор
issue-cell-outside-grid = ячейка вне сетки
issue-empty-frame = рамка нулевого размера
issue-free-without-frame = свободный элемент без рамки
issue-unknown-widget = неизвестный виджет: { $kind }
issue-wrong-category = { $kind } нельзя использовать здесь
issue-unknown-param = неизвестный параметр: { $param }
issue-param-type = недопустимое значение для { $param }
issue-choice-out-of-range = значение не разрешено для { $param }
issue-slot-rejects = { $kind } здесь не принимается
issue-placed-in-flow = элемент с рамкой в размеченном списке
issue-unknown-bind = { $param } связан с неизвестной настройкой презентации
issue-slot-too-deep = элементы вложены слишком глубоко
issue-invalid-style = недопустимое значение стиля: { $field }
issue-missing-asset = отсутствует ресурс: { $asset }
issue-pack-version-mismatch = изменённая версия пакета отброшена: { $version }
issue-pack-version-missing = версия пакета отсутствует в исходниках: { $version }
issue-theme-version-mismatch = изменённая версия темы отброшена: { $version }
issue-theme-version-missing = версия темы отсутствует в исходниках: { $version }
issue-unsupported-link = неподдерживаемая ссылка: { $link }
issue-highlight-zero = цвета выделения нумеруются с 1
issue-unreadable-element = нечитаемый элемент, отложен
issue-slot-repaired = элемент без известного слота, помещён в { $slot }
issue-unreadable-item = нечитаемый пункт списка, отложен
issue-unreadable-row = нечитаемая строка таблицы, отложена
issue-data-truncated = данные слишком велики, усечены при чтении
issue-unreadable-slide = нечитаемый слайд, отложен
issue-unreadable-field = нечитаемое поле { $field } презентации, сброшено
op-invalid-chart = Диаграмма { $subject }: { $reason }
op-invalid-table = Таблица { $subject }: { $reason }

replica-corrupted = История презентации в этом файле повреждена
replica-too-large = История презентации в этом файле слишком велика для открытия
replica-write = Не удалось обновить историю презентации
replica-unrelated = У этих двух презентаций нет общего происхождения
replica-unsupported-version = Эта презентация использует формат, который эта версия Oku Slides не может прочитать
replica-too-deep = Не удаётся сохранить эту презентацию: удалённые элементы вложены в её истории слишком глубоко. Отмените последние перемещения и сохраните снова.

widget-unreadable = Не удалось прочитать объявления виджетов
widget-fault-empty-name = Виджет { $widget }: у виджета или параметра нет имени
widget-fault-duplicate-widget = Виджет { $widget } объявлен дважды
widget-fault-duplicate-param = Виджет { $widget } дважды объявляет один и тот же параметр
widget-fault-empty-choice = Виджет { $widget }: у варианта выбора нет значений
widget-fault-default-not-in-choice = Виджет { $widget }: значение по умолчанию для выбора не входит в его значения
widget-fault-default-wrong-type = Виджет { $widget }: значение по умолчанию не соответствует типу своего параметра
widget-fault-bind-not-plain = Виджет { $widget }: только параметры простого текста можно привязать к презентации
widget-fault-unknown-slot-kind = Виджет { $widget }: слот принимает неизвестный виджет элемента
widget-fault-empty-list = Виджет { $widget }: списку нужна хотя бы одна колонка
widget-fault-column-type = Виджет { $widget }: колонка списка может быть plain, rich, number, flag, choice или asset (или вложенный p.list, один уровень); для произвольного содержимого пункта используйте p.slot с дочерним виджетом
widget-fault-reserved-name = Виджет { $widget }: зарезервированное имя: id, kind или имя, содержащее /
widget-fault-list-bounds = Виджет { $widget }: максимум списка меньше его минимума
widget-fault-list-default-too-large = Виджет { $widget }: значение списка по умолчанию превышает ограничения размера списков
widget-fault-list-depth = Виджет { $widget }: вложенный список не может содержать колонку-список; списки вкладываются только на один уровень
widget-fault-list-default-below-min = Виджет { $widget }: в значении вложенного списка по умолчанию меньше пунктов, чем его минимум
widget-incompatible-override = Виджет { $widget } заменяет встроенный виджет, но изменяет или удаляет его параметр { $param }
widget-incompatible-category = Виджет { $widget } заменяет встроенный виджет виджетом другой категории
issue-empty-item-id = в списке { $param } есть пункт без идентификатора
issue-duplicate-item-id = в списке { $param } повторяется идентификатор пункта
issue-invalid-item-id = в списке { $param } есть идентификатор пункта с косой чертой
issue-list-bounds = в списке { $param } слишком мало или слишком много пунктов
