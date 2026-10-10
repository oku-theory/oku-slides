pdf-unreadable = PDF не читається
pdf-unsupported-version = Цей PDF створено новішою версією Oku Slides
pdf-corrupted = Вихідні дані в цьому PDF пошкоджено
pdf-write = Не вдалося записати PDF

render-compile = Не вдалося відтворити презентацію
render-pdf = Не вдалося експортувати PDF
render-no-page = Такої сторінки немає
render-image = Неможливо створити зображення слайда

render-setup = Не вдалося запустити відтворення
render-missing-image = зображення відсутнє
diagnostic-error = помилка
diagnostic-warning = попередження
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = підказка: { $hint }
render-chart-failed = не вдалося побудувати діаграму

registry-bad-pack-id = Недійсний ідентифікатор пакета шаблонів: { $id }
registry-compile = Не вдалося скомпілювати віджети пакета шаблонів
registry-missing = Пакет шаблонів не оголошує своїх віджетів
render-timeout = Відтворення тривало понад { $seconds } с і було перервано
render-out-of-memory = Для відтворення знадобилося понад { $mib } МіБ пам’яті, його було перервано
render-worker-lost = Процес відтворення зупинився
render-frame = Нечитабельне повідомлення від процесу відтворення
render-page-too-large = Цей слайд завеликий для попереднього перегляду (понад { $mib } МіБ, часто через дуже велике зображення). Його все ще можна експортувати в PDF або PNG.
render-stale = Відтворення було замінено; спробуйте ще раз
render-pack-disabled = Віджети пакета «{ $pack }» вимкнено для цього сеансу
registry-schema-remote = Недійсний реєстр віджетів: { $message }
probe-not-probed = Ще не перевірено: час, відведений на віджети пакета, вичерпано. Перевірку буде виконано під час наступного рендерингу.
probe-too-many-files = У пакеті понад { $max } файлів віджетів: цей файл не завантажено.
probe-too-large = Понад { $max } КіБ: не завантажено.

import-unreadable = Не вдається прочитати цю презентацію
import-not-a-presentation = Цей файл не є презентацією PowerPoint
import-protected = Ця презентація зашифрована або має старий формат .ppt: спершу збережіть її як .pptx
import-too-large = Ця презентація завелика для імпорту
import-loss-list-levels = Рівні списку зведено до одного
import-loss-merged-cells = Об'єднані клітинки розділено
import-loss-percent-stacked = Нормовану діаграму з накопиченням показано як діаграму з накопиченням
import-loss-run-style = { $effect ->
    [underline] Підкреслення втрачено
    [strike] Закреслення втрачено
    [baseline] Надрядковий або підрядковий індекс втрачено
    [caps] Великі літери втрачено
   *[highlight] Виділення кольором втрачено
}
import-loss-transform = Поворот, віддзеркалення або обрізання втрачено
import-loss-color-dropped = Для кольору { $color } немає близького кольору теми: використано колір теми
import-loss-text-truncated = Текст скорочено, щоб вкластися в обмеження
import-loss-table-truncated = Таблицю скорочено, щоб вкластися в обмеження
import-loss-opaque = { $reason ->
    [media] Відео або звук не імпортовано (немає запасного зображення)
    [ole] Вбудований об'єкт не імпортовано (немає запасного зображення)
    [math] Формулу не імпортовано (немає запасного зображення)
    [smartart] SmartArt не імпортовано (немає запасного зображення)
   *[unknown] Непідтримуваний об'єкт не імпортовано (немає запасного зображення)
}
import-loss-unsupported-image = Зображення у форматі { $format } не імпортовано
import-loss-hidden-slide = Прихований слайд імпортовано як видимий
import-loss-effects = Тіні, відображення та 3D-ефекти втрачено
import-loss-series-dropped = Кругова діаграма показує один ряд: збережено { $kept }, відкинуто { $dropped }
import-loss-repeated-decor = { $slides ->
   [one] Колонтитул «{ $text }», повторений на { $slides } слайді, пропущено: тема малює власний
   [few] Колонтитул «{ $text }», повторений на { $slides } слайдах, пропущено: тема малює власний
   [many] Колонтитул «{ $text }», повторений на { $slides } слайдах, пропущено: тема малює власний
  *[other] Колонтитул «{ $text }», повторений на { $slides } слайда, пропущено: тема малює власний
}
import-loss-no-slot = Вміст пропущено: у вибраному шаблоні для нього немає місця
import-warning-effect-ignored = Ефект не імпортовано: { $detail }
import-warning-approximated = Наближене відображення: { $detail }
import-warning-unreadable-item = Нечитабельний елемент пропущено: { $detail }
import-warning-unreadable-part = Нечитабельна частина: { $detail }
import-warning-missing-part = Відсутня частина: { $detail }
import-warning-unsupported-chart = Непідтримувана діаграма: { $detail }
import-warning-unsupported-image = Непідтримуваний формат зображення: { $detail }
import-warning-unknown-table-style = Невідомий стиль таблиці: { $detail }
import-warning-unsafe-link = Посилання відхилено: { $detail }
import-warning-nesting-too-deep = Надто глибоке вкладення, обрізано: { $detail }
import-warning-font-embedded-unreadable = Нечитабельний вбудований шрифт: { $detail }
import-warning-truncated = Забагато попереджень: решту пропущено
