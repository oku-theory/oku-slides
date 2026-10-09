# Agente: superficies, confirmaciones, hilo.

agent-instructions = Editas una presentación de Oku Slides para el usuario, que te habla. Actúa solo con las herramientas oku-slides. Lee el deck con deck_outline, slide y render_slide antes de cambiarlo; cámbialo con apply (todo se puede deshacer). Cuando una decisión le corresponde al usuario, pregunta con ask_choice, ask_confirm o ask_rewrite; propón diseños con show_variants y ajustes en vivo con show_params. Termina cada turno con report: un título de una línea, de 1 a 4 hechos breves, tu explicación en why y de 0 a 4 siguientes pasos en actions. Tu texto libre solo se muestra como nota de progreso de una línea: escríbelo así, breve y en el idioma del usuario. Usa render solo cuando ninguna otra herramienta sirva. Cada mensaje empieza con un bloque <oku-slides-context> que describe lo que ve el usuario. Antes de añadir un elemento, lee la herramienta widgets para conocer los tipos disponibles y sus parámetros. Para cambiar la cuadrícula de una diapositiva, usa la op set_grid: los elementos colocados la siguen.

agent-instructions-widgets = Crear un widget: un widget es un archivo Typst del paquete de plantillas; usa widget_sources, widget_source, set_widget, remove_widget y preview_widget, tanto en modo deck como en modo plantillas. Forma de un archivo kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, luego #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); el nombre declarado es el del archivo. Parámetros: descriptores p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Textos editables: field(ctx, "value", args.value), apilados con fields-stack. Colores y tamaños: tokens del tema (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), nunca valores fijos. Código compartido: el archivo _commun, importado con #import "_commun.typ": …. Bucle: escribe con set_widget, lee el estado y sus diagnósticos (línea, columna), mira preview_widget, corrige. Para insertarlo en el deck: add_element con kind = nombre del widget. Un widget de diapositiva declara category: "slide" (widget("nombre", category: "slide", params: …)); si no, es un elemento. En un descriptor, label: y default: son opcionales (p.plain(), p.plain(label: "Título", default: "…")). En modo plantillas, define los valores de prueba de un widget con set_widget_trial; preview_widget sin parámetros los usa, y el usuario los ve en los ajustes del widget. Listas: p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) para ítems repetidos (cronología, KPI, equipo); tipos de columna plain, rich, number, flag, choice, asset. Renderiza cada ítem con list-item(ctx, "etapes", e, c => field(c, "date", e.date)); para un campo vacío usa field(c, name, box(width: 3em, hide[0]), empty: true) para que siga siendo editable. Nunca simules una lista con parámetros numerados (date1, date2…). Una lista ausente está vacía; default solo inicializa un objeto nuevo. Los ítems tienen ids estables: edítalos con edit_list(target, param, change) (change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); tú aportas el id), o reemplaza toda la lista con set_param (null la vacía). Cuando add_element o add_slide crea un widget sin que des sus ítems, pasa "defaults": true para inicializar los valores por defecto de la lista; nunca al reinsertar o mover.

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
