# Agente: superficies, confirmaciones, hilo.

agent-surface-cancel = Cancelar
agent-surface-confirm = Confirmar
agent-surface-confirm-danger = Sí, hazlo
agent-surface-rewrite = Reformulación
agent-surface-pick = Elegir { $n }
agent-surface-keep = Mantener el original
agent-surface-edit = Editar antes de aplicar
agent-surface-close = Cerrar
agent-surface-themes = Temas
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
agent-confirm-save-pack = ¿Guardar el paquete «{ $name }»?
agent-confirm-delete-pack = ¿Eliminar el paquete «{ $name }»? Esto no se puede deshacer.
agent-confirm-edit-theme = ¿Guardar una nueva versión del tema «{ $name }»?
agent-confirm-copy-theme = ¿Crear una variante del tema integrado «{ $name }»? El tema integrado no cambia.
agent-confirm-setting = ¿Cambiar { $key }: { $old } → { $new }?
agent-confirm-move = ¿Mover la presentación a { $path }? Se eliminará el archivo anterior.
agent-confirm-merge = ¿Fusionar { $path } con esta presentación? No se puede deshacer.
agent-confirm-save-copy = ¿Guardar una copia de la presentación como { $path }?
agent-confirm-dictation-api = ¿Enviar lo que dictas a { $host } ({ $url }), modelo { $model }, con el secreto de { $var }?
agent-confirm-dictation-download = ¿Usar { $name } y descargarlo ahora ({ $size })?
agent-confirm-dictation-language = ¿Usar el dictado de Apple y descargar su idioma ahora?
agent-confirm-agent = ¿Cambiar el agente de la app a { $name } ({ $command }) cuando no haya ninguna respuesta en curso?
agent-confirm-agent-builtin = ¿Cambiar el agente de la app a { $name } cuando no haya ninguna respuesta en curso?
agent-confirm-agent-model = ¿Descargar el modelo del agente local ({ $size })?

agent-not-found = Agente no encontrado: { $command }. Ajuste [agent.presets].
agent-spawn-failed = No se pudo iniciar el agente: { $command }. Ajuste [agent.presets].
agent-no-http = Este agente no puede conectarse a las herramientas de Oku Slides.
agent-local-no-program = El agente local solo funciona dentro de la app Oku Slides.
agent-local-needs-download = El agente local necesita primero descargar su modelo ({ $gigabytes } GB).
agent-local-intel-mac = El agente local necesita un Mac con chip de Apple.
agent-local-unsupported-os = El agente local aún no está disponible en este sistema.
agent-model-download = Descargar el modelo
agent-model-downloading = Descargando el modelo… { $percent } %
agent-model-cancel = Cancelar
agent-model-download-failed = No se pudo descargar el modelo.
agent-model-download-integrity = El modelo descargado estaba dañado y se ha eliminado.
agent-model-download-incomplete = Descarga incompleta: { $got } bytes de { $expected }.
agent-model-download-cancelled = Descarga cancelada.
local-agent-no-route = El agente local no entendió qué hacer. Intenta decirlo de otra manera.
local-agent-tool-limit = El agente local se detuvo tras { $n } pasos sin terminar.
local-agent-invalid-call = El agente local no supo usar las herramientas correctamente.
local-agent-engine-failed = El modelo del agente local falló.
local-agent-retry = Inténtalo de nuevo o elige otro agente en el panel del agente.
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
agent-error-pack-id = Esto no es el id de un paquete: llama a packs para listar los paquetes y sus ids.
agent-error-asset-unknown = Esta imagen no está ni en la presentación ni entre las imágenes pegadas: usa el identificador de una imagen pegada.
agent-error-denied = El usuario lo rechazó.
agent-error-no-turn = No hay ningún turno en curso: { $tool } solo se muestra durante un turno.
agent-error-pack-tab-no-turn = Abre una pestaña de paquete, o cambia a ella, solo mientras respondes al usuario.
agent-error-pack-tab-limit = Ya abriste una pestaña de paquete en este turno; termina allí, o pregunta al usuario antes de abrir otra.
agent-error-continue-in-pack-tab = Esta petición continúa en la pestaña del propio paquete: pide al usuario que siga allí; esta pestaña no modifica ningún widget por ahora.
agent-error-pack-open-in-tab = Este paquete se está editando en otra pestaña: pide al usuario que termine allí primero.
agent-pack-opened-in-tab = El paquete se abrió en la pestaña { $tab }, que ahora tiene el foco. Dile al usuario que continúe allí: el agente de esa pestaña no conoce esta petición. Esta pestaña se queda en su presentación.
agent-error-turn-over = El turno de esta llamada ha terminado: no se mostró nada.
agent-error-no-file = La presentación aún no tiene archivo: indica una ruta absoluta.
agent-error-path-relative = La ruta debe ser absoluta.
agent-error-path-parent = La ruta no debe contener «..».
agent-error-setting-locked = El agente no puede cambiar el ajuste { $key }.
agent-error-setting = No se pudo cambiar el ajuste { $key }: comprueba su valor.
agent-error-history-index = Para recorrer el historial hace falta el índice de una versión.
agent-error-no-pack = Esta presentación no fija ningún paquete: indica pack (ver packs).
agent-error-history-not-opened = La pestaña no abrió el historial a tiempo: vuelve a pedirlo.
agent-error-present = No hay nada que presentar aquí: ninguna diapositiva, o la pestaña está en modo de paquete, historial o solo lectura.
agent-error-locale = Código de idioma desconocido. Indica el código de un idioma incluido o system.
agent-error-empty-interface = Nada que cambiar: indica scheme, locale, grid o guides.
agent-error-empty-dictation = Nada que cambiar: indica al menos un ajuste del dictado.
agent-error-not-shown = Esta pestaña no es la que se muestra en su ventana: vuelve a pedirlo cuando el usuario la muestre.
agent-error-themes-deck-mode = El selector de temas solo se abre al editar las diapositivas.
agent-error-packs-deck-mode = El panel de paquetes de la presentación solo se abre al editar las diapositivas.
agent-error-rewrite-text = La reformulación volvió sin texto.
agent-error-paragraphs = Los párrafos de la reformulación no son válidos.
agent-error-draft-too-long = El borrador es demasiado largo.
agent-error-draft-origin = Borrador rechazado: vista desconocida.
agent-error-draft-no-agent = El agente de esta pestaña aún no se ha iniciado: borrador rechazado.
agent-error-too-many-images = El borrador ya tiene todas sus imágenes.
agent-error-image-not-base64 = La imagen pegada no llegó completa.
agent-error-image-gone = Una imagen pegada ya no está disponible: pégala de nuevo.
agent-error-unknown-agent = Este agente ya no está en la configuración.
agent-error-agent-locked = La elección del agente la impone el entorno.
agent-error-agent-not-saved = No se pudo guardar la elección del agente.
voice-error-no-dictation = El dictado no está disponible aquí
voice-error-busy = El dictado está en curso: inténtelo de nuevo cuando esté en reposo
voice-error-unknown-model = Modelo de dictado desconocido
voice-error-unavailable = Este modelo no admite el idioma elegido
voice-error-installed = Este modelo ya está instalado
voice-error-not-downloadable = Este modelo no se descarga
voice-error-nothing-to-cancel = No hay ninguna descarga en curso
voice-error-bad-silence = El silencio debe durar de 0,5 a 10 segundos
voice-error-bad-language = Idioma de dictado desconocido
voice-error-bad-url = La dirección de la API debe empezar por https:// (http:// solo para este ordenador), sin nombre de usuario, parámetros ni caracteres especiales
voice-error-bad-key-env = La variable de la clave debe ser un nombre como OPENAI_API_KEY: mayúsculas, cifras y _
voice-error-bad-api-model = El nombre del modelo de la API solo admite letras, cifras y . _ : / -
voice-error-bad-api-query = Los parámetros se escriben nombre=valor&nombre=valor, en ASCII visible sin % ni #, 8 como máximo
voice-error-bad-api-headers = Los encabezados se escriben Nombre: valor; Nombre: $VARIABLE, 8 como máximo. Se necesita una variable de clave o un encabezado $VARIABLE; se rechazan Host, Content-Type, Content-Length, Transfer-Encoding, Connection, los duplicados y Authorization con variable de clave
voice-error-api-reserved = Solo el usuario configura la dirección, la variable de la clave, los parámetros y los encabezados de la API: abra el cuadro con show_dictation y pídaselo
voice-error-changed = Los ajustes han cambiado desde la pregunta: vuelva a preguntar
voice-error-nothing-to-download = No hay nada que descargar para este agente
agent-tune-prompt = Afinar la presentación importada
agent-settings-unknown-selected = El ajuste agent.selected = { $selected } no nombra ningún agente de [agent.presets] (conocidos: { $known })
agent-settings-preset-launch = El agente [agent.presets.{ $id }] necesita exactamente uno de command o builtin
