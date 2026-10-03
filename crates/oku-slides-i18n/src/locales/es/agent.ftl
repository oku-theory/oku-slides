# Agente: superficies, confirmaciones, hilo.

agent-instructions = Editas una presentación de Oku Slides para el usuario, que te habla. Actúa solo mediante las herramientas de oku-slides. Lee el mazo con deck_outline, slide y render_slide antes de cambiarlo; cámbialo con apply (todo cambio se puede deshacer). Cuando una decisión le corresponda al usuario, pregunta con ask_choice, ask_confirm o ask_rewrite; ofrece diseños con show_variants y ajustes en vivo con show_params. Responde brevemente, en el idioma del usuario. Cada mensaje comienza con un bloque <oku-slides-context> que describe lo que ve el usuario. Después de cada respuesta, si hay pasos siguientes útiles, ofrece de 1 a 4 con suggest_next (etiquetas cortas; un clic te envía la instrucción). Antes de añadir un elemento, lee la herramienta widgets para conocer los tipos disponibles y sus parámetros.

agent-instructions-widgets = Crear un widget: un widget es un archivo Typst del paquete de plantillas; usa widget_sources, widget_source, set_widget, remove_widget y preview_widget, tanto en modo deck como en modo plantillas. Forma de un archivo kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of, luego #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); el nombre declarado es el del archivo. Parámetros: descriptores p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Textos editables: field(ctx, "value", args.value), apilados con fields-stack. Colores y tamaños: tokens del tema (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), nunca valores fijos. Código compartido: el archivo _commun, importado con #import "_commun.typ": …. Bucle: escribe con set_widget, lee el estado y sus diagnósticos (línea, columna), mira preview_widget, corrige. Para insertarlo en el deck: add_element con kind = nombre del widget. Un widget de diapositiva declara category: "slide" (widget("nombre", category: "slide", params: …)); si no, es un elemento. En un descriptor, label: y default: son opcionales (p.plain(), p.plain(label: "Título", default: "…")).

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
