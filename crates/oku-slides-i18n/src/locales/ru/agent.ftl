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
agent-confirm-copy-pack = Сохранить копию пакета «{ $name }»? Сам пакет не изменится.
agent-confirm-copy-edited-theme = Сохранить копию темы «{ $name }»? Сама тема не изменится.
agent-confirm-delete-pack = Удалить пакет «{ $name }»? Это необратимо.
agent-confirm-edit-theme = Сохранить новую версию темы «{ $name }»?
agent-confirm-copy-theme = Создать вариант встроенной темы «{ $name }»? Встроенная тема не изменится.
agent-confirm-setting = Изменить { $key }: { $old } → { $new }?
agent-confirm-move = Переместить презентацию в { $path }? Старый файл будет удалён.
agent-confirm-merge = Объединить { $path } с этой презентацией? Это действие нельзя отменить.
agent-confirm-save-copy = Сохранить копию презентации как { $path }?
agent-confirm-dictation-api = Отправлять надиктованное на { $host } ({ $url }), модель { $model }, с секретом из { $var }?
agent-confirm-dictation-download = Использовать { $name } и скачать сейчас ({ $size })?
agent-confirm-dictation-language = Использовать диктовку Apple и скачать её язык сейчас?
agent-confirm-agent = Переключить агента приложения на { $name } ({ $command }), когда не будет идти ни один ответ?
agent-confirm-agent-builtin = Переключить агента приложения на { $name }, когда не будет идти ни один ответ?
agent-confirm-agent-model = Скачать модель локального агента ({ $size })?

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
agent-error-asset-unknown = Этого изображения нет ни в презентации, ни среди вставленных изображений: укажите идентификатор вставленного изображения.
agent-error-denied = Пользователь отказался.
agent-error-no-turn = Нет текущего хода: { $tool } показывается только во время хода.
agent-error-tab-no-turn = Открывайте и показывайте вкладку пакета или темы только пока отвечаете пользователю.
agent-error-tab-limit = В этом ходе вы уже открыли или показали вкладку пакета или темы; закончите там или спросите пользователя, прежде чем переходить к другой.
agent-error-continue-in-other-tab = Этот запрос продолжается во вкладке, которую вы открыли или показали: попросите пользователя продолжить там; эта вкладка пока не меняет ни одного виджета.
agent-error-pack-open-in-tab = Этот пакет редактируется в другой вкладке: попросите пользователя сначала закончить там.
agent-pack-opened-in-tab = { $shown ->
   [yes] Пакет открылся во вкладке { $tab }, которая теперь показана.
  *[no] Пакет открылся во вкладке { $tab } в фоне: в окне пользователя показана другая вкладка.
} Скажите пользователю продолжить там: агент той вкладки не знает этого запроса. Эта вкладка остаётся как есть.
agent-theme-opened-in-tab = { $shown ->
   [yes] Тема открылась во вкладке { $tab }, которая теперь показана.
  *[no] Тема открылась во вкладке { $tab } в фоне: в окне пользователя показана другая вкладка.
} Скажите пользователю продолжить там: агент той вкладки не знает этого запроса. Эта вкладка остаётся как есть.
agent-error-turn-over = Ход этого вызова завершён: ничего не показано.
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
agent-error-empty-interface = Нечего менять: укажите scheme, locale или guides.
agent-error-empty-dictation = Нечего менять: укажите хотя бы один параметр диктовки.
agent-error-not-shown = Эта вкладка не показана в своём окне: повторите, когда пользователь её откроет.
agent-error-themes-deck-mode = Выбор темы открывается только при редактировании слайдов.
agent-error-packs-deck-mode = Панель пакетов презентации открывается только при редактировании слайдов.
agent-error-rewrite-text = Перефразирование вернулось без текста.
agent-error-paragraphs = Абзацы перефразирования недопустимы.
agent-error-draft-too-long = Черновик слишком длинный.
agent-error-draft-origin = Черновик отклонён: неизвестное представление.
agent-error-draft-no-agent = Агент этой вкладки ещё не запущен: черновик отклонён.
agent-error-too-many-images = В черновике уже максимум изображений.
agent-error-image-not-base64 = Вставленное изображение пришло не целиком.
agent-error-image-gone = Вставленное изображение больше недоступно: вставьте его снова.
agent-error-unknown-agent = Этого агента больше нет в конфигурации.
agent-error-agent-locked = Выбор агента задан окружением.
agent-error-agent-not-saved = Не удалось сохранить выбор агента.
agent-error-unknown-model = Этот агент не предлагает эту модель.
agent-error-model-not-saved = Не удалось сохранить выбор модели.
voice-error-no-dictation = Диктовка здесь недоступна
voice-error-busy = Идёт диктовка: повторите, когда она завершится
voice-error-unknown-model = Неизвестная модель диктовки
voice-error-unavailable = Эта модель не поддерживает выбранный язык
voice-error-installed = Эта модель уже установлена
voice-error-not-downloadable = Эта модель не скачивается
voice-error-nothing-to-cancel = Никакая загрузка не идёт
voice-error-bad-silence = Пауза должна длиться от 0,5 до 10 секунд
voice-error-bad-language = Неизвестный язык диктовки
voice-error-bad-url = Адрес API должен начинаться с https:// (http:// только для этого компьютера), без имени пользователя, параметров и специальных символов
voice-error-bad-key-env = Переменная ключа должна быть именем вроде OPENAI_API_KEY: заглавные буквы, цифры и _
voice-error-bad-api-model = Имя модели API может содержать только буквы, цифры и . _ : / -; имя развёртывания Azure — только буквы, цифры и . _ -
voice-error-bad-api-version = Версия API Azure может содержать только буквы, цифры, . и -, не более 32 символов, например 2025-03-01-preview
voice-error-bad-api-headers = Заголовки записываются как Имя: значение; Имя: $VARIABLE, не более 8. Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection и повторы отклоняются
voice-error-api-reserved = Только пользователь задаёт адрес, переменную ключа, версию Azure и заголовки API: откройте окно через show_dictation и попросите его
voice-error-changed = Настройки изменились после вопроса: спросите снова
voice-error-nothing-to-download = Для этого агента нечего скачивать
agent-tune-prompt = Доработать импортированную презентацию
agent-settings-unknown-selected = Параметр agent.selected = { $selected } не указывает ни на одного агента из [agent.presets] (известные: { $known })
agent-settings-preset-launch = Агенту [agent.presets.{ $id }] нужен ровно один из параметров command или builtin
