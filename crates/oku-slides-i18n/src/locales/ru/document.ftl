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

registry-bad-pack-id = Недопустимый идентификатор пакета шаблонов: { $id }
registry-compile = Не удалось скомпилировать виджеты пакета шаблонов
registry-missing = Пакет шаблонов не объявляет свои виджеты
render-timeout = Отрисовка заняла более { $seconds } с и была прервана
render-out-of-memory = Для отрисовки потребовалось более { $mib } МиБ памяти, она была прервана
render-worker-lost = Процесс отрисовки остановился
render-frame = Нечитаемое сообщение от процесса отрисовки
render-page-too-large = Этот слайд слишком велик для предпросмотра (более { $mib } МиБ, часто из-за очень большого изображения). Его по-прежнему можно экспортировать в PDF или PNG.
render-stale = Отрисовка была заменена; повторите попытку
render-pack-disabled = Виджеты пакета «{ $pack }» отключены в этом сеансе
registry-schema-remote = Недопустимый реестр виджетов: { $message }
probe-not-probed = Ещё не проверено: время, отведённое на виджеты пакета, истекло. Проверка будет при следующей отрисовке.
probe-too-many-files = В пакете больше { $max } файлов виджетов: этот файл не загружен.
probe-too-large = Больше { $max } КиБ: не загружено.

import-unreadable = Не удаётся прочитать эту презентацию
import-not-a-presentation = Этот файл не является презентацией PowerPoint
import-protected = Эта презентация зашифрована или сохранена в старом формате .ppt: сначала сохраните её как .pptx
import-too-large = Эта презентация слишком велика для импорта
import-loss-list-levels = Уровни списка сведены к одному
import-loss-merged-cells = Объединённые ячейки разделены
import-loss-percent-stacked = Нормированная диаграмма с накоплением показана как диаграмма с накоплением
import-loss-run-style = { $effect ->
    [underline] Подчёркивание потеряно
    [strike] Зачёркивание потеряно
    [baseline] Надстрочный или подстрочный индекс потерян
    [caps] Прописные буквы потеряны
   *[highlight] Выделение цветом потеряно
}
import-loss-transform = Поворот, отражение или обрезка потеряны
import-loss-color-dropped = Для цвета { $color } нет близкого цвета темы: использован цвет темы
import-loss-text-truncated = Текст сокращён, чтобы уложиться в ограничения
import-loss-table-truncated = Таблица сокращена, чтобы уложиться в ограничения
import-loss-opaque = { $reason ->
    [media] Видео или звук не импортированы (нет запасного изображения)
    [ole] Внедрённый объект не импортирован (нет запасного изображения)
    [math] Формула не импортирована (нет запасного изображения)
    [smartart] SmartArt не импортирован (нет запасного изображения)
   *[unknown] Неподдерживаемый объект не импортирован (нет запасного изображения)
}
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
import-loss-no-slot = Содержимое опущено: в выбранном шаблоне для него нет места
import-warning-effect-ignored = Эффект не импортирован: { $detail }
import-warning-approximated = Приблизительное отображение: { $detail }
import-warning-unreadable-item = Нечитаемый элемент пропущен: { $detail }
import-warning-unreadable-part = Нечитаемая часть: { $detail }
import-warning-missing-part = Отсутствующая часть: { $detail }
import-warning-unsupported-chart = Неподдерживаемая диаграмма: { $detail }
import-warning-unsupported-image = Неподдерживаемый формат изображения: { $detail }
import-warning-unknown-table-style = Неизвестный стиль таблицы: { $detail }
import-warning-unsafe-link = Ссылка отклонена: { $detail }
import-warning-nesting-too-deep = Слишком глубокая вложенность, обрезано: { $detail }
import-warning-font-embedded-unreadable = Нечитаемый внедрённый шрифт: { $detail }
import-warning-truncated = Слишком много предупреждений: остальные пропущены
