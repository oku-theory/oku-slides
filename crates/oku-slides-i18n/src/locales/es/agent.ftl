# Agente: superficies, confirmaciones, hilo.

agent-instructions = Editas una presentación de Oku Slides para el usuario, que te habla. Actúa solo mediante las herramientas de oku-slides. Lee el mazo con deck_outline, slide y render_slide antes de cambiarlo; cámbialo con apply (todo cambio se puede deshacer). Cuando una decisión le corresponda al usuario, pregunta con ask_choice, ask_confirm o ask_rewrite; ofrece diseños con show_variants y ajustes en vivo con show_params. Responde brevemente, en el idioma del usuario. Cada mensaje comienza con un bloque <oku-slides-context> que describe lo que ve el usuario.

agent-surface-cancel = Cancelar
agent-surface-confirm = Confirmar
agent-surface-confirm-danger = Sí, hazlo
agent-surface-rewrite = Reformulación
agent-surface-pick = Elegir { $n }
agent-surface-keep = Mantener el original
agent-surface-edit = Editar antes de aplicar
agent-surface-close = Cerrar
agent-surface-variants = Variantes
agent-surface-permission-always = para siempre
agent-surface-permission-title = El agente pide un permiso
agent-undo-turn = Agente: { $text }

agent-tools-unavailable = Las herramientas del agente no se pudieron iniciar.

agent-undo-apply = Cambio del agente
agent-undo-variant = Variante: { $label }
agent-undo-param = Ajuste: { $label }
agent-undo-rewrite = Reformulación

agent-confirm-save = ¿Guardar la presentación como { $path }?
agent-confirm-save-over = ¿Reemplazar { $path }? Se sobrescribirá el archivo existente.
agent-confirm-open = ¿Abrir { $path }?
agent-open-elsewhere = Este archivo ya está abierto en otra pestaña.
agent-confirm-new = ¿Empezar una nueva presentación?
agent-confirm-new-dirty = ¿Empezar una nueva presentación? Se perderán los cambios sin guardar.
agent-confirm-save-templates = ¿Guardar el conjunto de plantillas «{ $name }»?
agent-confirm-delete-set = ¿Eliminar el conjunto de plantillas «{ $name }»? Esto no se puede deshacer.
agent-confirm-setting = ¿Cambiar { $key }: { $old } → { $new }?

agent-not-found = Agente no encontrado: { $command }. Ajuste [agent] command.
agent-no-http = Este agente no puede conectarse a las herramientas de Oku Slides.
agent-stopped = El agente se detuvo.
agent-failed = El agente rechazó la solicitud: { $message }
