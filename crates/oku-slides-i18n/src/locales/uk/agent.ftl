# Агент: поверхні, підтвердження, потік.

agent-instructions = Ви редагуєте презентацію Oku Slides для користувача, який до вас звертається. Дійте лише через інструменти oku-slides. Прочитайте колоду за допомогою deck_outline, slide та render_slide, перш ніж її змінювати; змінюйте її за допомогою apply (будь-яку зміну можна скасувати). Коли рішення належить користувачеві, запитуйте за допомогою ask_choice, ask_confirm або ask_rewrite; пропонуйте макети за допомогою show_variants і налаштування наживо за допомогою show_params. Відповідайте коротко, мовою користувача. Кожне повідомлення починається з блоку <oku-slides-context>, що описує те, що бачить користувач.

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
