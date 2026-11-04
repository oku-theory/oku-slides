# Агент: поверхні, підтвердження, потік.

agent-surface-cancel = Скасувати
agent-surface-confirm = Підтвердити
agent-surface-confirm-danger = Так, зробити
agent-surface-heard = Почуто: «{ $text }»
agent-surface-say-or-click = Скажіть відповідь або натисніть кнопку.
agent-surface-said-too-early = Сказано надто рано: зачекайте на вікно й скажіть ще раз.
agent-surface-click-to-confirm = Натисніть кнопку, щоб підтвердити.
agent-surface-verb-replace = Замінити
agent-surface-verb-merge = Об'єднати
agent-surface-verb-discard = Відкинути
agent-surface-verb-delete = Видалити
agent-surface-verb-use-api = Використовувати API
agent-surface-verb-install = Встановити
agent-surface-verb-quit = Вийти із застосунку
agent-surface-verb-close = Закрити
agent-surface-verb-import = Імпортувати
agent-surface-rewrite = Переформулювання
agent-surface-pick = Вибрати { $n }
agent-surface-keep = Залишити оригінал
agent-surface-edit = Редагувати перед застосуванням
agent-surface-close = Закрити
agent-surface-themes = Теми
agent-surface-variants = Варіанти
agent-surface-permission-always = назавжди
agent-surface-permission-title = Агент просить дозвіл
agent-surface-permission-guides = Цей хід містить отримане керівництво: { $names }.
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

agent-not-found = Агента не знайдено: { $command } немає ні за цим шляхом, ні в PATH застосунку. Вкажіть повний шлях у [agent.presets].
agent-spawn-failed = Не вдалося запустити агента: { $command }. Налаштування [agent.presets].
agent-no-http = Цей агент не може підключитися до інструментів Oku Slides.
agent-local-no-program = Локальний агент працює лише всередині застосунку Oku Slides.
agent-local-needs-download = Локальному агенту спершу потрібно завантажити свою модель ({ $gigabytes } ГБ).
agent-local-intel-mac = Локальному агенту потрібен Mac із чипом Apple.
agent-local-unsupported-os = Локальний агент поки недоступний у цій системі.
agent-local-apple-off = Локальному агенту потрібен Apple Intelligence: увімкніть його в Системних параметрах.
agent-local-apple-not-ready = Модель Apple ще готується: спробуйте локального агента знову за кілька хвилин.
agent-local-apple-unsupported = Моделі Apple потрібен Mac із підтримкою Apple Intelligence та macOS 26 або новішою.
agent-model-download = Завантажити модель
agent-model-downloading = Завантаження моделі… { $percent } %
agent-model-cancel = Скасувати
agent-model-download-failed = Не вдалося завантажити модель.
agent-model-download-integrity = Завантажена модель була пошкоджена й видалена.
agent-model-download-incomplete = Завантаження не завершено: { $got } байт із { $expected }.
agent-model-download-cancelled = Завантаження скасовано.
agent-fallback-notice = Локальний агент — { $reason }
agent-fallback-download = { $reason } Локальний агент зможе підмінити його після завантаження своєї моделі ({ $model }, { $gigabytes } ГБ). Завантажити зараз?
agent-fallback-unusable = Жоден агент не може працювати. Основний агент: { $main } Локальний агент: { $local }
local-agent-no-route = Локальний агент не зрозумів, що потрібно зробити. Спробуйте сказати інакше.
local-agent-tool-limit = Локальний агент зупинився після { $n } кроків, не завершивши.
local-agent-invalid-call = Локальний агент не зміг правильно скористатися інструментами.
local-agent-engine-failed = Модель локального агента дала збій.
local-agent-retry = Спробуйте ще раз або виберіть іншого агента на панелі агента.
agent-stopped = Агент зупинився.
agent-failed = Агент відхилив запит: { $message }
agent-error-work-dir = Не вдалося підготувати особисту робочу теку агента для цієї вкладки.
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
agent-trace = { $name }: { $count ->
    [one] { $count } зміна
    [few] { $count } зміни
    [many] { $count } змін
   *[other] { $count } зміни
}
agent-trace-data = { $name }: дані змінено
agent-trace-removed = { $name }: більше не існує
agent-trace-more = { $count ->
    [one] і ще { $count }
    [few] і ще { $count }
    [many] і ще { $count }
   *[other] і ще { $count }
}
agent-trace-title = Змінені елементи

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
agent-error-tab-no-turn = Відкривайте, показуйте чи переміщуйте вкладку лише під час відповіді користувачеві.
agent-error-tab-limit = У цьому ході ви вже відкрили, показали чи перемістили вкладку; завершіть там або спитайте користувача перед наступною.
agent-error-continue-in-other-tab = Цей запит триває у вкладці, яку ви відкрили або показали: попросіть користувача продовжити там; ця вкладка поки що не змінює жодного віджета.
agent-error-pack-open-in-tab = Цей пакет редагується в іншій вкладці: попросіть користувача спершу завершити там.
agent-pack-opened-in-tab = { $shown ->
   [yes] Пакет відкрився у вкладці { $tab }, яку тепер показано.
  *[no] Пакет відкрився у вкладці { $tab } у фоні: у вікні користувача показано іншу вкладку.
} { $draft ->
   [set] Запит чекає в чернетці тієї вкладки: скажіть користувачеві перевірити його там, а потім надіслати.
   [kept] Чернетка тієї вкладки вже щось містила, її залишено як є. Скажіть користувачеві продовжити там: агент тієї вкладки не знає цього запиту.
   [too_long] Запит задовгий для чернетки тієї вкладки. Скажіть користувачеві продовжити там: агент тієї вкладки не знає цього запиту.
   [unavailable] Запит не вдалося передати в чернетку тієї вкладки. Скажіть користувачеві продовжити там: агент тієї вкладки не знає цього запиту.
  *[none] Скажіть користувачеві продовжити там: агент тієї вкладки не знає цього запиту.
} Ця вкладка лишається як є.
agent-theme-opened-in-tab = { $shown ->
   [yes] Тема відкрилася у вкладці { $tab }, яку тепер показано.
  *[no] Тема відкрилася у вкладці { $tab } у фоні: у вікні користувача показано іншу вкладку.
} { $draft ->
   [set] Запит чекає в чернетці тієї вкладки: скажіть користувачеві перевірити його там, а потім надіслати.
   [kept] Чернетка тієї вкладки вже щось містила, її залишено як є. Скажіть користувачеві продовжити там: агент тієї вкладки не знає цього запиту.
   [too_long] Запит задовгий для чернетки тієї вкладки. Скажіть користувачеві продовжити там: агент тієї вкладки не знає цього запиту.
   [unavailable] Запит не вдалося передати в чернетку тієї вкладки. Скажіть користувачеві продовжити там: агент тієї вкладки не знає цього запиту.
  *[none] Скажіть користувачеві продовжити там: агент тієї вкладки не знає цього запиту.
} Ця вкладка лишається як є.
agent-error-turn-over = Хід цього виклику завершено: нічого не показано.
agent-error-no-box-turn = Немає ходу, до якого можна прив'язати це вікно: нічого не показано.
agent-error-no-file = Презентація ще не має файлу: вкажіть абсолютний шлях.
agent-error-path-relative = Шлях має бути абсолютним.
agent-error-path-parent = Шлях не повинен містити «..».
agent-error-setting-locked = Агент не може змінити налаштування { $key }.
agent-error-setting = Не вдалося змінити налаштування { $key }: перевірте його значення.
agent-error-history-index = Щоб гортати історію, потрібен номер версії.
agent-error-no-pack = Ця презентація не закріплює жодного пакета: вкажіть pack (див. packs).
agent-error-history-not-opened = Вкладка не встигла відкрити історію: повторіть запит.
agent-error-locale = Невідомий код мови. Вкажіть код доступної мови або system.
agent-error-empty-interface = Нічого змінювати: вкажіть scheme, locale, guides, folded, pile, notes або code.
agent-error-layout-range = { $field } має бути від { $min } до { $max } px.
agent-error-layout-min = { $field } має бути щонайменше { $min } px.
agent-error-empty-dictation = Нічого змінювати: вкажіть принаймні одне налаштування диктування.
agent-error-empty-speech = Нічого змінювати: вкажіть принаймні одне налаштування читання вголос.
agent-error-not-shown = Ця вкладка не показана у своєму вікні: повторіть, коли користувач її відкриє.
agent-error-not-presenting = Тут дек не показується: спершу запустіть показ дією start.
agent-error-present-slide = Щоб показати слайд, потрібен id слайда цього деку (див. deck_outline).
agent-error-undo-open-turn = undo_turn не скасовує поточний хід: для змін цього ходу використовуйте undo.
agent-error-app-unavailable = Це може зробити лише застосунок Oku Slides; тут це недоступно.
agent-error-app-closing = Oku Slides уже закривається.
agent-error-app-no-offer = Тут немає оновлення, яке можна відкласти чи встановити: спершу перевірте оновлення.
agent-error-app-failed = Оновлення Oku Slides не вдалося: перевірте з'єднання або скористайтеся банером оновлення.
agent-error-after-turn-taken = Ця відповідь уже завершується виходом із застосунку, встановленням оновлення або закриттям своєї вкладки.
agent-error-no-pack-update = Панель пакетів не пропонує такого оновлення для цього пакета: див. show_packs.
agent-confirm-update-install = Установити Oku Slides { $version }? Застосунок перезапуститься, щойно ця відповідь завершиться.
agent-after-turn-quit-unsaved = Oku Slides не закрився: відтоді як ви попросили, вкладка має незбережені зміни.
agent-error-workspace-unavailable = Лише застосунок Oku Slides може змінювати вкладки й вікна; тут це недоступно.
agent-error-workspace-unknown-tab = У цьому вікні немає такої вкладки чи вікна.
agent-error-workspace-unknown-window = Такого вікна немає.
agent-error-workspace-index = Ця позиція за кінцем панелі вкладок.
agent-error-workspace-closing = Oku Slides закривається: вкладки й вікна більше не змінюються.
agent-confirm-close-tab = Закрити вкладку «{ $tab }»?
agent-confirm-discard-tab = Закрити вкладку «{ $tab }»? Її незбережені зміни буде втрачено.
agent-confirm-close-window = Закрити це вікно та його вкладки?
agent-confirm-discard-window = Закрити це вікно? Незбережені зміни в { $tabs } буде втрачено.
agent-error-window-last = Це єдине вікно: закрийте його вкладки або вийдіть з Oku Slides.
agent-error-tab-index = Скажіть, куди поставити вкладку: для перестановки потрібна позиція.
agent-error-tab-unused = Позицію приймають лише перестановка й переміщення, а вікно — лише переміщення.
agent-after-turn-close-unsaved = Вкладку не закрито: вона змінилася відтоді, як ви попросили.
agent-after-turn-close-not-shown = Вкладку не закрито: наприкінці відповіді в її вікні була показана інша вкладка.
agent-confirm-quit = Вийти з Oku Slides? Застосунок закриється, щойно ця відповідь завершиться.
agent-error-quit-unsaved = Вкладки з незбереженою роботою: { $count }. Oku Slides не закрито: спершу користувач зберігає або закриває їх.
agent-error-tune-unavailable = Тонке налаштування не змогло почати: агент недоступний.
agent-error-tune-not-started = Тонке налаштування не почалося: агент тієї вкладки поки непридатний; вкладка пояснює чому.
agent-confirm-import = Імпортувати { $path } у нову вкладку?
agent-import-opened-in-tab = { $shown ->
   [yes] Презентація відкрилася у вкладці { $tab }, тепер показаній.
  *[no] Презентація відкрилася у вкладці { $tab }, у фоні: вікно користувача показує іншу вкладку.
} { $tune ->
   [started] Її тонке налаштування починається там, у власній розмові тієї вкладки.
   [unavailable] Її тонке налаштування не змогло почати: скажіть користувачеві попросити про нього в тій вкладці.
  *[no] Скажіть користувачеві продовжити там.
} Ця вкладка лишається як є.
agent-error-import-hifi-target = Імпорт hi-fi не приймає ні пакета, ні теми: приберіть їх або імпортуйте в режимі retrofit.
agent-error-import-extension = Імпортувати можна лише файл PowerPoint .pptx.
agent-error-a2ui-unknown = { $name } — не компонент, який приймає render. Компоненти: { $known }.
agent-error-a2ui-elsewhere = render не показує { $name }: використайте { $tool }.
agent-error-a2ui-host-only = { $name } зарезервовано для застосунку: render не може його показати.
agent-error-themes-deck-mode = Вибір теми відкривається лише під час редагування слайдів.
agent-error-packs-deck-mode = Панель пакетів презентації відкривається лише під час редагування слайдів.
agent-error-usage-no-deck = Використання цієї вкладки не належить жодній презентації: до презентації зараховується лише вкладка пакета або теми, відкрита з неї (користувачем або агентом), або власний пакет презентації.
agent-error-rewrite-text = Перефразування повернулося без тексту.
agent-error-paragraphs = Абзаци перефразування недійсні.
agent-error-draft-too-long = Чернетка занадто довга.
agent-error-report-elsewhere = Це вікно не показує цю вкладку.
agent-error-ui-too-large = Контекст сторінки завеликий.
agent-error-seq-exhausted = Номер звіту сторінки поза допустимим діапазоном.
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
voice-error-hands-free-reserved = Лише користувач вмикає прослуховування без рук: відкрийте вікно через show_dictation і попросіть його
voice-error-changed = Налаштування змінилися після запитання: запитайте знову
voice-error-nothing-to-download = Для цього агента нічого завантажувати
voice-error-bad-rate = Швидкість мовлення має бути від 0,5 до 2
voice-error-unknown-voice = Невідомий голос для мови інтерфейсу
voice-error-speech-unavailable = Читання вголос недоступне на цьому комп’ютері
agent-tune-prompt = Доопрацювати імпортовану презентацію
agent-settings-unknown-selected = Параметр agent.selected = { $selected } не вказує на жодного агента з [agent.presets] (відомі: { $known })
agent-settings-preset-launch = Агентові [agent.presets.{ $id }] потрібен рівно один із параметрів command або builtin
