# Агент: поверхні, підтвердження, потік.

agent-surface-cancel = Скасувати
agent-surface-confirm = Підтвердити
agent-surface-confirm-danger = Так, зробити
agent-surface-rewrite = Переформулювання
agent-surface-pick = Вибрати { $n }
agent-surface-keep = Залишити оригінал
agent-surface-edit = Редагувати перед застосуванням
agent-surface-close = Закрити
agent-surface-variants = Варіанти
agent-surface-permission-always = назавжди
agent-surface-permission-title = Агент просить дозвіл
agent-undo-turn = Агент: { $text }

agent-tools-unavailable = Не вдалося запустити інструменти агента.

agent-undo-apply = Зміна агента
agent-undo-variant = Варіант: { $label }
agent-undo-param = Налаштування: { $label }
agent-undo-rewrite = Переформулювання

agent-confirm-save = Зберегти презентацію як { $path }?
agent-confirm-save-over = Замінити { $path }? Наявний файл буде перезаписано.
agent-confirm-open = Відкрити { $path }?
agent-open-elsewhere = Цей файл уже відкрито в іншій вкладці.
agent-confirm-new = Почати нову презентацію?
agent-confirm-new-dirty = Почати нову презентацію? Незбережені зміни буде втрачено.
agent-confirm-save-templates = Зберегти набір шаблонів «{ $name }»?
agent-confirm-delete-set = Видалити набір шаблонів «{ $name }»? Це незворотно.
agent-confirm-edit-theme = Зберегти нову версію теми «{ $name }»?
agent-confirm-copy-theme = Зберегти змінену копію вбудованої теми «{ $name }»?
agent-confirm-setting = Змінити { $key }: { $old } → { $new }?

agent-not-found = Агента не знайдено: { $command }. Налаштування [agent.presets].
agent-spawn-failed = Не вдалося запустити агента: { $command }. Налаштування [agent.presets].
agent-no-http = Цей агент не може підключитися до інструментів Oku Slides.
agent-local-no-program = Локальний агент працює лише всередині застосунку Oku Slides.
agent-local-needs-download = Локальному агенту спершу потрібно завантажити свою модель ({ $gigabytes } ГБ).
agent-local-intel-mac = Локальному агенту потрібен Mac із чипом Apple.
agent-local-unsupported-os = Локальний агент поки недоступний у цій системі.
agent-model-download = Завантажити модель
agent-model-downloading = Завантаження моделі… { $percent } %
agent-model-cancel = Скасувати
agent-model-download-failed = Не вдалося завантажити модель.
agent-model-download-integrity = Завантажена модель була пошкоджена й видалена.
agent-model-download-incomplete = Завантаження не завершено: { $got } байт із { $expected }.
agent-model-download-cancelled = Завантаження скасовано.
local-agent-no-route = Локальний агент не зрозумів, що потрібно зробити. Спробуйте сказати інакше.
local-agent-tool-limit = Локальний агент зупинився після { $n } кроків, не завершивши.
local-agent-invalid-call = Локальний агент не зміг правильно скористатися інструментами.
local-agent-engine-failed = Модель локального агента дала збій.
local-agent-retry = Спробуйте ще раз або виберіть іншого агента на панелі агента.
agent-stopped = Агент зупинився.
agent-failed = Агент відхилив запит: { $message }
agent-tool-failed = Інструмент несподівано завершився з помилкою.

agent-turn-tools = { $tools ->
   [one] { $tools } інструмент
   [few] { $tools } інструменти
   [many] { $tools } інструментів
  *[other] { $tools } інструмента
}
agent-turn-notes = { $notes ->
   [one] { $notes } нотатка
   [few] { $notes } нотатки
   [many] { $notes } нотаток
  *[other] { $notes } нотатки
}
agent-turn-secs = { $secs } с
agent-turn-stopped = Зупинено.
agent-turn-interrupted = Перервано.
agent-memory-lost = Агент більше не пам'ятає цю розмову: він починає спочатку.
agent-turn-max-tokens = Агент досяг межі довжини.
agent-turn-max-turns = Агент досяг межі кроків.
agent-turn-refusal = Агент відмовився.
agent-turn-ended = Хід завершено ({ $stop }).
agent-choice-cancelled = Запитання скасовано
agent-choice-answered = ✓ { $answer }
agent-report-why = Чому

agent-error-file = Не вдалося прочитати або записати файл.
agent-error-unexpected-action = Неочікувана дія: { $action }.
agent-error-unknown-surface = Ця картка більше не відкрита.
agent-error-expired = Термін дій минулого ходу сплив.
agent-error-incomplete = Дія цієї картки неповна.
agent-error-no-field = Цього налаштування більше немає.
agent-error-no-variant = Цього варіанта більше немає.
agent-error-arguments = Аргументи інструмента недійсні ({ $detail }): перевірте їх за його схемою.
agent-error-unknown-tool = Невідомий інструмент: { $name }.
agent-error-pack-id = Це не id набору шаблонів: викличте templates, щоб отримати список наборів та їхні id.
agent-error-denied = Користувач відмовився.
agent-error-no-turn = Немає поточного ходу: { $tool } показується лише під час ходу.
agent-error-turn-over = Хід цього виклику завершено: нічого не показано.
agent-error-anchor-scope = Питання в розмові про віджет не можна прив'язати до слайда: поставте його без прив'язки.
agent-error-no-file = Презентація ще не має файлу: вкажіть абсолютний шлях.
agent-error-path-relative = Шлях має бути абсолютним.
agent-error-path-parent = Шлях не повинен містити «..».
agent-error-setting-locked = Агент не може змінити налаштування { $key }.
agent-error-setting = Не вдалося змінити налаштування { $key }: перевірте його значення.
agent-error-rewrite-text = Перефразування повернулося без тексту.
agent-error-paragraphs = Абзаци перефразування недійсні.
agent-error-draft-too-long = Чернетка занадто довга.
agent-error-draft-origin = Чернетку відхилено: невідоме подання.
agent-error-draft-no-agent = Агента цієї вкладки ще не запущено: чернетку відхилено.
agent-error-unknown-agent = Цього агента більше немає в конфігурації.
agent-error-agent-locked = Вибір агента задано середовищем.
agent-error-agent-not-saved = Не вдалося зберегти вибір агента.
agent-picker-title = Агент
agent-picker-busy = Зміна агента перерве поточний хід.
agent-picker-locked = Вибір задано середовищем (OKU_SLIDES_AGENT__SELECTED).
agent-tune-prompt = Доопрацювати імпортовану презентацію
