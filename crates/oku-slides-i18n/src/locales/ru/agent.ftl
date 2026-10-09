# Агент: поверхности, подтверждения, поток.

agent-instructions = Ты редактируешь презентацию Oku Slides для пользователя, который с тобой говорит. Действуй только через инструменты oku-slides. Прочитай презентацию через deck_outline, slide и render_slide, прежде чем её менять; меняй её через apply (всё можно отменить). Когда решение за пользователем, спрашивай через ask_choice, ask_confirm или ask_rewrite; предлагай макеты через show_variants и настройки в реальном времени через show_params. Заканчивай каждый ход вызовом report: заголовок в одну строку, от 1 до 4 коротких фактов, твоё объяснение в why и от 0 до 4 следующих шагов в actions. Твой свободный текст показывается только как однострочная заметка о ходе работы: пиши его так, кратко и на языке пользователя. Используй render только тогда, когда ни один другой инструмент не подходит. Каждое сообщение начинается с блока <oku-slides-context>, описывающего то, что видит пользователь. Перед добавлением элемента прочитай инструмент widgets, чтобы узнать доступные виды и их параметры. Чтобы изменить сетку слайда, используй операцию set_grid: размещённые элементы следуют за ней.

agent-instructions-widgets = Создание виджета: виджет — это файл Typst пакета шаблонов; используй widget_sources, widget_source, set_widget, remove_widget и preview_widget — как в режиме презентации, так и в режиме шаблонов. Вид файла kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, затем #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); объявленное имя совпадает с именем файла. Параметры: дескрипторы p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Редактируемые тексты: field(ctx, "value", args.value), складываются через fields-stack. Цвета и размеры: токены темы (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), никогда не жёсткие значения. Общий код: файл _commun, подключается через #import "_commun.typ": …. Цикл: пиши через set_widget, читай статус и диагностику (строка, столбец), смотри preview_widget, исправляй. Чтобы вставить его в презентацию: add_element, где kind — имя виджета. Виджет слайда объявляет category: "slide" (widget("name", category: "slide", params: …)); иначе это элемент. В дескрипторе label: и default: необязательны (p.plain(), p.plain(label: "Заголовок", default: "…")). В режиме шаблонов задавай тестовые значения виджета через set_widget_trial; preview_widget без параметров использует их, а пользователь видит их в настройках виджета. Списки: p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) для повторяющихся пунктов (хронология, KPI, команда); типы столбцов plain, rich, number, flag, choice, asset. Отображай каждый пункт через list-item(ctx, "etapes", e, c => field(c, "date", e.date)); для пустого поля используй field(c, name, box(width: 3em, hide[0]), empty: true), чтобы оно оставалось редактируемым. Никогда не имитируй список нумерованными параметрами (date1, date2…). Отсутствующий список пуст; default заполняет только новый объект. У пунктов стабильные id: правь их через edit_list(target, param, change) (change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); id задаёшь ты) или замени весь список через set_param (null очищает). Если add_element или add_slide создаёт виджет без указанных тобой пунктов, передай "defaults": true, чтобы заполнить значения списка по умолчанию; никогда при повторной вставке или перемещении.

agent-surface-cancel = Отмена
agent-surface-confirm = Подтвердить
agent-surface-confirm-danger = Да, сделать
agent-surface-rewrite = Переформулировка
agent-surface-pick = Выбрать { $n }
agent-surface-keep = Оставить оригинал
agent-surface-edit = Изменить перед применением
agent-surface-close = Закрыть
agent-surface-variants = Варианты
agent-surface-permission-always = навсегда
agent-surface-permission-title = Агент запрашивает разрешение
agent-undo-turn = Агент: { $text }

agent-tools-unavailable = Не удалось запустить инструменты агента.

agent-undo-apply = Изменение агента
agent-undo-variant = Вариант: { $label }
agent-undo-param = Настройка: { $label }
agent-undo-rewrite = Переформулировка

agent-confirm-save = Сохранить презентацию как { $path }?
agent-confirm-save-over = Заменить { $path }? Существующий файл будет перезаписан.
agent-confirm-open = Открыть { $path }?
agent-open-elsewhere = Этот файл уже открыт в другой вкладке.
agent-confirm-new = Начать новую презентацию?
agent-confirm-new-dirty = Начать новую презентацию? Несохранённые изменения будут потеряны.
agent-confirm-save-templates = Сохранить набор шаблонов «{ $name }»?
agent-confirm-delete-set = Удалить набор шаблонов «{ $name }»? Это необратимо.
agent-confirm-setting = Изменить { $key }: { $old } → { $new }?

agent-not-found = Агент не найден: { $command }. Настройка [agent] command.
agent-no-http = Этот агент не может подключиться к инструментам Oku Slides.
agent-stopped = Агент остановился.
agent-failed = Агент отклонил запрос: { $message }
agent-tool-failed = Инструмент неожиданно завершился с ошибкой.

agent-turn-tools = { $tools ->
   [one] { $tools } инструмент
   [few] { $tools } инструмента
   [many] { $tools } инструментов
  *[other] { $tools } инструмента
}
agent-turn-notes = { $notes ->
   [one] { $notes } заметка
   [few] { $notes } заметки
   [many] { $notes } заметок
  *[other] { $notes } заметки
}
agent-turn-secs = { $secs } с
agent-turn-stopped = Остановлено.
agent-turn-interrupted = Прервано.
agent-memory-lost = Агент больше не помнит этот разговор: он начинает заново.
agent-turn-max-tokens = Агент достиг лимита длины.
agent-turn-max-turns = Агент достиг лимита шагов.
agent-turn-refusal = Агент отказался.
agent-turn-ended = Ход завершён ({ $stop }).
agent-choice-cancelled = Вопрос отменён
agent-choice-answered = ✓ { $answer }
agent-report-why = Почему

agent-error-file = Не удалось прочитать или записать файл.
agent-error-unexpected-action = Неожиданное действие: { $action }.
agent-error-unknown-surface = Эта карточка больше не открыта.
agent-error-expired = Срок действий прошлого хода истёк.
agent-error-incomplete = Действие этой карточки неполное.
agent-error-no-field = Этой настройки больше нет.
agent-error-no-variant = Этого варианта больше нет.
agent-error-arguments = Аргументы инструмента недопустимы ({ $detail }): сверьте их с его схемой.
agent-error-unknown-tool = Неизвестный инструмент: { $name }.
agent-error-denied = Пользователь отказался.
agent-error-no-turn = Нет текущего хода: { $tool } показывается только во время хода.
agent-error-turn-over = Ход этого вызова завершён: ничего не показано.
agent-error-anchor-scope = Вопрос в беседе о виджете нельзя привязать к слайду: задайте его без привязки.
agent-error-no-file = У презентации ещё нет файла: укажите абсолютный путь.
agent-error-path-relative = Путь должен быть абсолютным.
agent-error-path-parent = Путь не должен содержать «..».
agent-error-setting-locked = Агент не может изменить настройку { $key }.
agent-error-setting = Не удалось изменить настройку { $key }: проверьте её значение.
agent-error-rewrite-text = Перефразирование вернулось без текста.
agent-error-paragraphs = Абзацы перефразирования недопустимы.
agent-error-draft-too-long = Черновик слишком длинный.
agent-error-draft-origin = Черновик отклонён: неизвестное представление.
agent-error-draft-no-agent = Агент этой вкладки запускается: черновик не сохранён.
