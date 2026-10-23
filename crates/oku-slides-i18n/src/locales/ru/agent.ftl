# Агент: поверхности, подтверждения, поток.

agent-surface-cancel = Отмена
agent-surface-confirm = Подтвердить
agent-surface-confirm-danger = Да, сделать
agent-surface-rewrite = Переформулировка
agent-surface-pick = Выбрать { $n }
agent-surface-keep = Оставить оригинал
agent-surface-edit = Изменить перед применением
agent-surface-close = Закрыть
agent-surface-themes = Темы
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
agent-confirm-save-pack = Сохранить пакет «{ $name }»?
agent-confirm-delete-pack = Удалить пакет «{ $name }»? Это необратимо.
agent-confirm-edit-theme = Сохранить новую версию темы «{ $name }»?
agent-confirm-copy-theme = Создать вариант встроенной темы «{ $name }»? Встроенная тема не изменится.
agent-confirm-setting = Изменить { $key }: { $old } → { $new }?
agent-confirm-move = Переместить презентацию в { $path }? Старый файл будет удалён.
agent-confirm-merge = Объединить { $path } с этой презентацией? Это действие нельзя отменить.
agent-confirm-save-copy = Сохранить копию презентации как { $path }?

agent-not-found = Агент не найден: { $command }. Настройка [agent.presets].
agent-spawn-failed = Не удалось запустить агента: { $command }. Настройка [agent.presets].
agent-no-http = Этот агент не может подключиться к инструментам Oku Slides.
agent-local-no-program = Локальный агент работает только внутри приложения Oku Slides.
agent-local-needs-download = Локальному агенту сначала нужно загрузить свою модель ({ $gigabytes } ГБ).
agent-local-intel-mac = Локальному агенту нужен Mac с чипом Apple.
agent-local-unsupported-os = Локальный агент пока недоступен в этой системе.
agent-model-download = Загрузить модель
agent-model-downloading = Загрузка модели… { $percent } %
agent-model-cancel = Отмена
agent-model-download-failed = Не удалось загрузить модель.
agent-model-download-integrity = Загруженная модель была повреждена и удалена.
agent-model-download-incomplete = Загрузка не завершена: { $got } байт из { $expected }.
agent-model-download-cancelled = Загрузка отменена.
local-agent-no-route = Локальный агент не понял, что нужно сделать. Попробуйте сказать иначе.
local-agent-tool-limit = Локальный агент остановился после { $n } шагов, не закончив.
local-agent-invalid-call = Локальный агент не смог правильно воспользоваться инструментами.
local-agent-engine-failed = Модель локального агента дала сбой.
local-agent-retry = Повторите попытку или выберите другого агента на панели агента.
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
agent-error-pack-id = Это не id пакета: вызовите packs, чтобы получить список пакетов и их id.
agent-error-denied = Пользователь отказался.
agent-error-no-turn = Нет текущего хода: { $tool } показывается только во время хода.
agent-error-turn-over = Ход этого вызова завершён: ничего не показано.
agent-error-anchor-scope = Вопрос в беседе о виджете нельзя привязать к слайду: задайте его без привязки.
agent-error-no-file = У презентации ещё нет файла: укажите абсолютный путь.
agent-error-path-relative = Путь должен быть абсолютным.
agent-error-path-parent = Путь не должен содержать «..».
agent-error-setting-locked = Агент не может изменить настройку { $key }.
agent-error-setting = Не удалось изменить настройку { $key }: проверьте её значение.
agent-error-history-index = Чтобы листать историю, нужен номер версии.
agent-error-no-pack = Эта презентация не закрепляет ни одного пакета: укажите pack (см. packs).
agent-error-history-not-opened = Вкладка не успела открыть историю: повторите запрос.
agent-error-present = Здесь нечего показывать: нет слайдов, или вкладка в режиме пакета, истории или только для чтения.
agent-error-locale = Неизвестный код языка. Укажите код поставляемого языка или system.
agent-error-empty-interface = Нечего менять: укажите scheme, locale, grid или guides.
agent-error-not-shown = Эта вкладка не показана в своём окне: повторите, когда пользователь её откроет.
agent-error-themes-deck-mode = Выбор темы открывается только при редактировании слайдов.
agent-error-rewrite-text = Перефразирование вернулось без текста.
agent-error-paragraphs = Абзацы перефразирования недопустимы.
agent-error-draft-too-long = Черновик слишком длинный.
agent-error-draft-origin = Черновик отклонён: неизвестное представление.
agent-error-draft-no-agent = Агент этой вкладки ещё не запущен: черновик отклонён.
agent-error-unknown-agent = Этого агента больше нет в конфигурации.
agent-error-agent-locked = Выбор агента задан окружением.
agent-error-agent-not-saved = Не удалось сохранить выбор агента.
agent-picker-title = Агент
agent-picker-busy = Смена агента прервёт текущий ход.
agent-picker-locked = Выбор задан окружением (OKU_SLIDES_AGENT__SELECTED).
agent-tune-prompt = Доработать импортированную презентацию
