# Agente: superficies, confirmaciones, hilo.

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
agent-tool-failed = La herramienta falló de forma inesperada.

agent-turn-tools = { $tools ->
   [one] { $tools } herramienta
  *[other] { $tools } herramientas
}
agent-turn-notes = { $notes ->
   [one] { $notes } nota
  *[other] { $notes } notas
}
agent-turn-secs = { $secs } s
agent-turn-stopped = Detenido.
agent-turn-interrupted = Interrumpido.
agent-memory-lost = El agente ya no recuerda esta conversación: empieza de cero.
agent-turn-max-tokens = El agente alcanzó su límite de longitud.
agent-turn-max-turns = El agente alcanzó su límite de pasos.
agent-turn-refusal = El agente se negó.
agent-turn-ended = Turno terminado ({ $stop }).
agent-choice-cancelled = Pregunta cancelada
agent-choice-answered = ✓ { $answer }
agent-report-why = Por qué

agent-error-file = No se pudo leer ni escribir el archivo.
agent-error-unexpected-action = Acción inesperada: { $action }.
agent-error-unknown-surface = Esta tarjeta ya no está abierta.
agent-error-expired = Las acciones de un turno pasado han caducado.
agent-error-incomplete = La acción de esta tarjeta está incompleta.
agent-error-no-field = Este ajuste ya no existe.
agent-error-no-variant = Esta variante ya no existe.
agent-error-arguments = Los argumentos de la herramienta no son válidos ({ $detail }): compruébalos con su esquema.
agent-error-unknown-tool = Herramienta desconocida: { $name }.
agent-error-denied = El usuario lo rechazó.
agent-error-no-turn = No hay ningún turno en curso: { $tool } solo se muestra durante un turno.
agent-error-turn-over = El turno de esta llamada ha terminado: no se mostró nada.
agent-error-anchor-scope = Una pregunta de una conversación de widget no se puede anclar en una diapositiva: hazla sin ancla.
agent-error-no-file = La presentación aún no tiene archivo: indica una ruta absoluta.
agent-error-path-relative = La ruta debe ser absoluta.
agent-error-path-parent = La ruta no debe contener «..».
agent-error-setting-locked = El agente no puede cambiar el ajuste { $key }.
agent-error-setting = No se pudo cambiar el ajuste { $key }: comprueba su valor.
agent-error-rewrite-text = La reformulación volvió sin texto.
agent-error-paragraphs = Los párrafos de la reformulación no son válidos.
agent-error-draft-too-long = El borrador es demasiado largo.
agent-error-draft-origin = Borrador rechazado: vista desconocida.
agent-error-draft-no-agent = El agente de esta pestaña aún no se ha iniciado: borrador rechazado.
