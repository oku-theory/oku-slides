# Агент: поверхні, підтвердження, потік.

agent-surface-cancel = Скасувати
agent-surface-confirm = Підтвердити
agent-surface-confirm-danger = Так, зробити
agent-surface-rewrite = Переформулювання
agent-surface-pick = Вибрати { $n }
agent-surface-keep = Залишити оригінал
agent-surface-edit = Редагувати перед застосуванням
agent-surface-close = Закрити
agent-surface-themes = Теми
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
agent-confirm-copy-pack = Зберегти копію пакета «{ $name }»? Сам пакет не зміниться.
agent-confirm-copy-edited-theme = Зберегти копію теми «{ $name }»? Сама тема не зміниться.
agent-confirm-delete-pack = Видалити пакет «{ $name }»? Це незворотно.
agent-confirm-edit-theme = Зберегти нову версію теми «{ $name }»?
agent-confirm-copy-theme = Створити варіант вбудованої теми «{ $name }»? Вбудована тема не зміниться.
agent-confirm-setting = Змінити { $key }: { $old } → { $new }?
agent-confirm-move = Перемістити презентацію до { $path }? Старий файл буде видалено.
agent-confirm-merge = Об'єднати { $path } з цією презентацією? Цю дію не можна скасувати.
agent-confirm-save-copy = Зберегти копію презентації як { $path }?
agent-confirm-dictation-api = Надсилати надиктоване на { $host } ({ $url }), модель { $model }, із секретом із { $var }?
agent-confirm-dictation-download = Використати { $name } і завантажити зараз ({ $size })?
agent-confirm-dictation-language = Використати диктування Apple і завантажити його мову зараз?
agent-confirm-agent = Перемкнути агента застосунку на { $name } ({ $command }), коли жодна відповідь не триватиме?
agent-confirm-agent-builtin = Перемкнути агента застосунку на { $name }, коли жодна відповідь не триватиме?
agent-confirm-agent-model = Завантажити модель локального агента ({ $size })?

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
agent-error-pack-id = Це не id пакета: викличте packs, щоб отримати список пакетів та їхні id.
agent-error-asset-unknown = Цього зображення немає ні в презентації, ні серед вставлених зображень: вкажіть ідентифікатор вставленого зображення.
agent-error-denied = Користувач відмовився.
agent-error-no-turn = Немає поточного ходу: { $tool } показується лише під час ходу.
agent-error-tab-no-turn = Відкривайте та показуйте вкладку пакета або теми лише тоді, коли відповідаєте користувачеві.
agent-error-tab-limit = У цьому ході ви вже відкрили або показали вкладку пакета чи теми; завершіть там або запитайте користувача, перш ніж переходити до іншої.
agent-error-continue-in-other-tab = Цей запит триває у вкладці, яку ви відкрили або показали: попросіть користувача продовжити там; ця вкладка поки що не змінює жодного віджета.
agent-error-pack-open-in-tab = Цей пакет редагується в іншій вкладці: попросіть користувача спершу завершити там.
agent-pack-opened-in-tab = { $shown ->
   [yes] Пакет відкрився у вкладці { $tab }, яку тепер показано.
  *[no] Пакет відкрився у вкладці { $tab } у фоні: у вікні користувача показано іншу вкладку.
} Скажіть користувачеві продовжити там: агент тієї вкладки не знає цього запиту. Ця вкладка лишається як є.
agent-theme-opened-in-tab = { $shown ->
   [yes] Тема відкрилася у вкладці { $tab }, яку тепер показано.
  *[no] Тема відкрилася у вкладці { $tab } у фоні: у вікні користувача показано іншу вкладку.
} Скажіть користувачеві продовжити там: агент тієї вкладки не знає цього запиту. Ця вкладка лишається як є.
agent-error-turn-over = Хід цього виклику завершено: нічого не показано.
agent-error-no-file = Презентація ще не має файлу: вкажіть абсолютний шлях.
agent-error-path-relative = Шлях має бути абсолютним.
agent-error-path-parent = Шлях не повинен містити «..».
agent-error-setting-locked = Агент не може змінити налаштування { $key }.
agent-error-setting = Не вдалося змінити налаштування { $key }: перевірте його значення.
agent-error-history-index = Щоб гортати історію, потрібен номер версії.
agent-error-no-pack = Ця презентація не закріплює жодного пакета: вкажіть pack (див. packs).
agent-error-history-not-opened = Вкладка не встигла відкрити історію: повторіть запит.
agent-error-present = Тут нічого показувати: немає слайдів, або вкладка в режимі пакета, історії чи лише для читання.
agent-error-locale = Невідомий код мови. Вкажіть код доступної мови або system.
agent-error-empty-interface = Нічого змінювати: вкажіть scheme, locale, grid або guides.
agent-error-empty-dictation = Нічого змінювати: вкажіть принаймні одне налаштування диктування.
agent-error-not-shown = Ця вкладка не показана у своєму вікні: повторіть, коли користувач її відкриє.
agent-error-themes-deck-mode = Вибір теми відкривається лише під час редагування слайдів.
agent-error-packs-deck-mode = Панель пакетів презентації відкривається лише під час редагування слайдів.
agent-error-rewrite-text = Перефразування повернулося без тексту.
agent-error-paragraphs = Абзаци перефразування недійсні.
agent-error-draft-too-long = Чернетка занадто довга.
agent-error-draft-origin = Чернетку відхилено: невідоме подання.
agent-error-draft-no-agent = Агента цієї вкладки ще не запущено: чернетку відхилено.
agent-error-too-many-images = У чернетці вже максимум зображень.
agent-error-image-not-base64 = Вставлене зображення надійшло не повністю.
agent-error-image-gone = Вставлене зображення більше недоступне: вставте його знову.
agent-error-unknown-agent = Цього агента більше немає в конфігурації.
agent-error-agent-locked = Вибір агента задано середовищем.
agent-error-agent-not-saved = Не вдалося зберегти вибір агента.
agent-error-unknown-model = Цей агент не пропонує цю модель.
agent-error-model-not-saved = Не вдалося зберегти вибір моделі.
voice-error-no-dictation = Диктування тут недоступне
voice-error-busy = Триває диктування: спробуйте знову, коли воно завершиться
voice-error-unknown-model = Невідома модель диктування
voice-error-unavailable = Ця модель не підтримує вибрану мову
voice-error-installed = Ця модель уже встановлена
voice-error-not-downloadable = Ця модель не завантажується
voice-error-nothing-to-cancel = Жодне завантаження не триває
voice-error-bad-silence = Пауза має тривати від 0,5 до 10 секунд
voice-error-bad-language = Невідома мова диктування
voice-error-bad-url = Адреса API має починатися з https:// (http:// лише для цього комп’ютера), без імені користувача, параметрів і спеціальних символів
voice-error-bad-key-env = Змінна ключа має бути назвою на зразок OPENAI_API_KEY: великі літери, цифри та _
voice-error-bad-api-model = Назва моделі API може містити лише літери, цифри та . _ : / -; назва розгортання Azure — лише літери, цифри та . _ -
voice-error-bad-api-version = Версія API Azure може містити лише літери, цифри, . і -, щонайбільше 32 символи, наприклад 2025-03-01-preview
voice-error-bad-api-headers = Заголовки записуються як Назва: значення; Назва: $VARIABLE, щонайбільше 8. Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection і повтори відхиляються
voice-error-api-reserved = Лише користувач задає адресу, змінну ключа, версію Azure і заголовки API: відкрийте вікно через show_dictation і попросіть його
voice-error-changed = Налаштування змінилися після запитання: запитайте знову
voice-error-nothing-to-download = Для цього агента нічого завантажувати
agent-tune-prompt = Доопрацювати імпортовану презентацію
agent-settings-unknown-selected = Параметр agent.selected = { $selected } не вказує на жодного агента з [agent.presets] (відомі: { $known })
agent-settings-preset-launch = Агентові [agent.presets.{ $id }] потрібен рівно один із параметрів command або builtin
