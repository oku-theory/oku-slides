# Агент: поверхности, подтверждения, поток.

agent-instructions = Вы редактируете презентацию Oku Slides для пользователя, который с вами говорит. Действуйте только через инструменты oku-slides. Прочитайте колоду с помощью deck_outline, slide и render_slide, прежде чем её менять; изменяйте её с помощью apply (любое изменение можно отменить). Когда решение принадлежит пользователю, спрашивайте с помощью ask_choice, ask_confirm или ask_rewrite; предлагайте макеты с помощью show_variants и настройки в реальном времени с помощью show_params. Отвечайте кратко, на языке пользователя. Каждое сообщение начинается с блока <oku-slides-context>, описывающего то, что видит пользователь.

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
