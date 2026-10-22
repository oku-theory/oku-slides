pdf-unreadable = PDF не читается
pdf-unsupported-version = Этот PDF создан более новой версией Oku Slides
pdf-corrupted = Исходники в этом PDF повреждены
pdf-write = Не удалось записать PDF

render-compile = Не удалось отрисовать презентацию
render-pdf = Не удалось экспортировать PDF
render-no-page = Такой страницы нет
render-image = Не удалось создать изображение слайда

render-setup = Не удалось запустить отрисовку
render-missing-image = изображение отсутствует
diagnostic-error = ошибка
diagnostic-warning = предупреждение
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = подсказка: { $hint }
render-chart-failed = не удалось построить диаграмму

registry-bad-pack-id = Недопустимый идентификатор пакета: { $id }
registry-base-pack-broken = Встроенный пакет повреждён ({ $file }); переустановите приложение.
registry-duplicate-pack = Два пакета презентации используют одно и то же: { $pack }.
registry-compile = Не удалось скомпилировать виджеты пакета
registry-missing = Пакет не объявляет свои виджеты
render-timeout = Отрисовка заняла более { $seconds } с и была прервана
render-out-of-memory = Для отрисовки потребовалось более { $mib } МиБ памяти, она была прервана
render-worker-lost = Процесс отрисовки остановился
render-frame = Нечитаемое сообщение от процесса отрисовки
render-page-too-large = Этот слайд слишком велик для предпросмотра (более { $mib } МиБ, часто из-за очень большого изображения). Его по-прежнему можно экспортировать в PDF или PNG.
render-stale = Отрисовка была заменена; повторите попытку
render-pack-disabled = Виджеты пакета «{ $pack }» отключены в этом сеансе
render-protocol = Процесс отрисовки относится к другой версии Oku Slides; перезапустите приложение
render-remote = Процессу отрисовки не удалось выполнить запрос
registry-schema-remote = Недопустимый реестр виджетов: { $message }
probe-not-probed = Ещё не проверено: время, отведённое на виджеты пакета, истекло. Проверка будет при следующей отрисовке.
probe-too-many-files = В пакете больше { $max } файлов виджетов: этот файл не загружен.
probe-too-large = Больше { $max } КиБ: не загружено.

import-unreadable = Не удаётся прочитать эту презентацию
import-not-a-presentation = Этот файл не является презентацией PowerPoint
import-protected = Эта презентация зашифрована или сохранена в старом формате .ppt: сначала сохраните её как .pptx
import-too-large = Эта презентация слишком велика для импорта
import-output-too-large = Полученная презентация превысила бы допустимый для импорта размер
import-no-profiles = Выбранные пакеты макетов не предлагают ни одного профиля.
import-loss-list-levels = Уровни списка сведены к одному
import-loss-merged-cells = Объединённые ячейки разделены
import-loss-percent-stacked = Нормированная диаграмма с накоплением показана как диаграмма с накоплением
import-loss-chart-format = Числовой формат { $format } диаграммы не сохранён
import-loss-run-style = { $effect ->
    [underline] Подчёркивание потеряно
    [strike] Зачёркивание потеряно
    [baseline] Надстрочный или подстрочный индекс потерян
    [caps] Прописные буквы потеряны
    [font] Шрифт фрагмента заменён шрифтом надписи
    [tracking] Межбуквенный интервал фрагмента заменён интервалом надписи
    [weight] Обычный фрагмент показан жирным, как блок
    [slant] Прямой фрагмент показан курсивом, как блок
   *[highlight] Выделение цветом потеряно
}
import-loss-transform = Поворот, отражение или обрезка потеряны
import-loss-color-dropped = Для цвета { $color } нет близкого цвета темы: использован цвет темы
import-loss-text-truncated = Текст сокращён, чтобы уложиться в ограничения
import-loss-table-truncated = Таблица сокращена, чтобы уложиться в ограничения
import-loss-chart-truncated = Данные диаграммы сокращены, чтобы уложиться в ограничения
import-loss-opaque = { $reason ->
    [media] Видео или звук не импортированы (нет запасного изображения)
    [ole] Внедрённый объект не импортирован (нет запасного изображения)
    [math] Формула не импортирована (нет запасного изображения)
    [smartart] SmartArt не импортирован (нет запасного изображения)
   *[unknown] Неподдерживаемый объект не импортирован (нет запасного изображения)
}
import-loss-smartart-layout = SmartArt без рисунка: текст размещён в карточках, макет диаграммы утрачен
import-loss-unsupported-image = Изображение в формате { $format } не импортировано
import-loss-hidden-slide = Скрытый слайд импортирован как видимый
import-loss-effects = Тени, отражения и 3D-эффекты потеряны
import-loss-series-dropped = Круговая диаграмма показывает один ряд: сохранено { $kept }, отброшено { $dropped }
import-loss-repeated-decor = { $slides ->
   [one] Колонтитул «{ $text }», повторённый на { $slides } слайде, опущен: тема рисует свой
   [few] Колонтитул «{ $text }», повторённый на { $slides } слайдах, опущен: тема рисует свой
   [many] Колонтитул «{ $text }», повторённый на { $slides } слайдах, опущен: тема рисует свой
  *[other] Колонтитул «{ $text }», повторённый на { $slides } слайда, опущен: тема рисует свой
}
import-loss-no-slot = Содержимое опущено: в выбранном макете для него нет места
import-loss-moved-to-notes = Текст перенесён в заметки докладчика: на обложке для него нет места
import-loss-items-dropped = Слишком много элементов на слайде: пропущено элементов: { $dropped }
import-loss-page-fitted = Исходная страница { $width } × { $height } pt не в формате 16:9: содержимое масштабировано и выровнено по центру
import-loss-paragraph-style = Выравнивание, межстрочный интервал, интервал между абзацами или отступ приведены к надписи
import-loss-shape-style = { $effect ->
    [gradient] Градиент показан сплошным цветом
    [picture] Рисунок заливки растянут по рамке фигуры
    [dash] Пунктирная линия показана сплошной
    [arrow] Наконечники стрелок линии не нарисованы
   *[geometry] Неизвестная фигура показана прямоугольником
}
import-loss-layout-dropped = Слишком много макетов для одного пакета: у «{ $layout }» нет виджета; его слайды используют пустой виджет и получают его оформление как элементы
import-loss-decor-truncated = Оформление урезано, чтобы уложиться в размер файла пакета: пропущено элементов: { $omitted }
import-loss-table-style = Стиль таблицы приближён: собственные фоны ячеек, границы, ширина столбцов, высота строк или размеры текста потеряны
import-loss-overflow = Элемент «{ $element }» выходит за край слайда
import-loss-empty-data = Таблица или диаграмма без данных: ничего не размещено
import-reference-unavailable = Нет визуальной оценки: { $cause ->
    [timeout] эталонный рендеринг занял слишком много времени
    [memory] эталонный рендеринг потребовал слишком много памяти
    [pages] в эталонном рендеринге { $reference ->
        [one] { $reference } страница
        [few] { $reference } страницы
        [many] { $reference } страниц
       *[other] { $reference } страницы
    } при { $deck } в презентации
    [missing] LibreOffice (soffice) не найден; укажите путь к нему в import.soffice
   *[failed] эталонный рендерер не смог прочитать презентацию
}
import-region-suspect = Расхождение отложено, известная ошибка эталонного рендерера: { $limit ->
    [autofit] он уменьшает текст, подогнанный под рамку, когда PowerPoint не сохранил масштаб
    [tracking] он рисует текст с разрядкой шире
    [chart] он рисует диаграммы лишь приблизительно
    [default_font] он рисует своим запасным шрифтом текст, шрифт которого задан только настройками презентации по умолчанию
    [no_substitute] у него нет замены для этого шрифта, и он рисует его другим
    [smart_art_style] он рисует текст SmartArt в своём собственном стиле
   *[smart_art] он рисует текст SmartArt слишком мелким
}
import-font-substituted = { $family } заменён на { $by } с совместимыми метриками
import-font-missing = { $family } не найден: текст использует шрифт { $by }
import-font-restricted = { $family }: лицензия запрещает встраивать некоторые начертания в презентацию; текст этих стилей использует остальные начертания
import-font-restricted-substituted = { $family } нельзя встроить в презентацию (лицензия): заменён на { $by } с совместимыми метриками
import-font-restricted-missing = { $family } нельзя встроить в презентацию (лицензия): текст использует шрифт { $by }
import-font-too-large = { $family }: файлы слишком велики для встраивания в презентацию; текст использует шрифт { $by }
import-font-missing-no-fallback = { $family } не найден, как и шрифт основного текста темы: текст отображается шрифтом движка по умолчанию
import-font-too-large-no-fallback = { $family }: файлы слишком велики для встраивания в презентацию, а шрифт основного текста темы тоже недоступен: текст отображается шрифтом движка по умолчанию
import-font-restricted-no-fallback = { $family } нельзя встроить в презентацию (лицензия), а шрифт основного текста темы тоже недоступен: текст отображается шрифтом движка по умолчанию
import-warning-effect-ignored = Эффект не импортирован: { $detail }
import-warning-approximated = Приблизительное отображение: { $detail }
import-warning-unreadable-item = Нечитаемый элемент пропущен: { $detail }
import-warning-unreadable-part = Нечитаемая часть: { $detail }
import-warning-missing-part = Отсутствующая часть: { $detail }
import-warning-unsupported-chart = Неподдерживаемая диаграмма: { $detail }
import-warning-unsupported-image = Неподдерживаемый формат изображения: { $detail }
import-warning-oversized-image = Изображение слишком большое для импорта (число пикселей): { $detail }
import-warning-external-image = Связанное изображение вне файла не импортировано: { $detail }
import-warning-unsafe-svg = Изображение SVG очищено от небезопасного содержимого или отброшено: { $detail }
import-warning-unknown-table-style = Неизвестный стиль таблицы: { $detail }
import-warning-unsafe-link = Ссылка отклонена: { $detail }
import-warning-nesting-too-deep = Слишком глубокая вложенность, обрезано: { $detail }
import-warning-font-embedded-unreadable = Нечитаемый внедрённый шрифт: { $detail }
import-warning-truncated = Слишком много предупреждений: остальные пропущены
