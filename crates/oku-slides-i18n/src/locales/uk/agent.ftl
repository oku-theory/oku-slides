# Агент: поверхні, підтвердження, потік.

agent-instructions = Ти редагуєш презентацію Oku Slides для користувача, який з тобою говорить. Дій лише через інструменти oku-slides. Прочитай презентацію через deck_outline, slide і render_slide, перш ніж її змінювати; змінюй її через apply (усе можна скасувати). Коли рішення за користувачем, питай через ask_choice, ask_confirm або ask_rewrite; пропонуй макети через show_variants і налаштування наживо через show_params. Завершуй кожен хід викликом report: заголовок в один рядок, від 1 до 4 коротких фактів, твоє пояснення в why і від 0 до 4 наступних кроків у actions. Твій вільний текст показується лише як однорядкова примітка про хід роботи: пиши його так, стисло і мовою користувача. Використовуй render лише тоді, коли жоден інший інструмент не підходить. Кожне повідомлення починається з блоку <oku-slides-context>, що описує те, що бачить користувач. Перш ніж додати елемент, прочитай інструмент widgets, щоб дізнатися доступні види та їхні параметри. Щоб змінити сітку слайда, використовуй операцію set_grid: розміщені елементи йдуть за нею.

agent-instructions-widgets = Створення віджета: віджет — це файл Typst пакета шаблонів; використовуй widget_sources, widget_source, set_widget, remove_widget і preview_widget — і в режимі презентації, і в режимі шаблонів. Вигляд файлу kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of, потім #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); оголошене ім'я збігається з іменем файлу. Параметри: дескриптори p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Тексти, що редагуються: field(ctx, "value", args.value), складаються через fields-stack. Кольори й розміри: токени теми (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), ніколи не жорстко задані значення. Спільний код: файл _commun, підключається через #import "_commun.typ": …. Цикл: пиши через set_widget, читай статус і діагностику (рядок, стовпець), дивись preview_widget, виправляй. Щоб вставити його в презентацію: add_element, де kind — ім'я віджета. Віджет слайда оголошує category: "slide" (widget("name", category: "slide", params: …)); інакше це елемент. У дескрипторі label: і default: необов'язкові (p.plain(), p.plain(label: "Заголовок", default: "…")). У режимі шаблонів задавай тестові значення віджета через set_widget_trial; preview_widget без параметрів використовує їх, а користувач бачить їх у налаштуваннях віджета.

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
agent-confirm-setting = Змінити { $key }: { $old } → { $new }?

agent-not-found = Агента не знайдено: { $command }. Налаштування [agent] command.
agent-no-http = Цей агент не може підключитися до інструментів Oku Slides.
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
agent-turn-max-tokens = Агент досяг межі довжини.
agent-turn-max-turns = Агент досяг межі кроків.
agent-turn-refusal = Агент відмовився.
agent-turn-ended = Хід завершено ({ $stop }).
agent-choice-cancelled = Запитання скасовано
agent-choice-answered = ✓ { $answer }
agent-report-why = Чому
