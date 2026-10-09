# Interfaz de la app (React). Cargada por ui/src/i18n.ts mediante la solicitud «messages».
ui-loading = Cargando…

## Documento

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · plantillas
ui-title-rename = Cambiar el nombre de la presentación
ui-title-field = Título de la presentación
ui-status-unsaved = cambios sin guardar
ui-status-saved = guardado
ui-status-never-saved = aún no guardado
ui-caption-templates = { $count ->
    [one] { $count } plantilla
   *[other] { $count } plantillas
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } diapositiva
   *[other] { $count } diapositivas
} · { $status }
ui-template-default-name = Plantilla
ui-template-variant-name = { $name } (variante)
ui-confirm-quit-templates = ¿Salir sin guardar las plantillas modificadas?

## Avisos

ui-toast-templates-saved = Plantillas guardadas: { $name }
ui-toast-moved = Presentación movida
ui-toast-copied = Copia guardada
ui-toast-select-zone = Selecciona una zona donde colocar: «{ $text }»
ui-toast-field-gone = El campo que editabas ya no existe: se ha perdido lo escrito.

## Barra de herramientas

ui-toolbar-undo = Deshacer
ui-toolbar-undo-label = Deshacer: { $label }
ui-toolbar-redo = Rehacer
ui-toolbar-redo-label = Rehacer: { $label }
ui-toolbar-grid-hide = Ocultar la cuadrícula
ui-toolbar-grid-show = Mostrar la cuadrícula
ui-toolbar-ghosts-hide = Ocultar los fantasmas
ui-toolbar-ghosts-show = Mostrar los fantasmas
ui-divider-pile = Ancho de la pila de diapositivas
ui-divider-notes = Ancho del hilo del agente
ui-divider-code = Tamaño del código
ui-toolbar-theme = Tema
ui-toolbar-theme-embedded = Tema del archivo
ui-toolbar-save-copy = Guardar una copia
ui-toolbar-finish = Listo
ui-menu-workshop = Taller
ui-toolbar-open = Abrir
ui-merge = Fusionar con un PDF…
ui-menu = Menú
ui-menu-move = Mover…
ui-language = Idioma
ui-language-system = Idioma del sistema
ui-scheme = Apariencia
ui-scheme-system = Sistema
ui-scheme-light = Claro
ui-scheme-dark = Oscuro
ui-toolbar-add-label = Añadir a la diapositiva
ui-add-kind = { $kind ->
    [card] Tarjeta
    [shape] Forma
    [chart] Gráfico
    [table] Tabla
   *[text] Texto
}
ui-add-widget = Widget del paquete
ui-toolbar-remove-template = Eliminar la plantilla
ui-toolbar-remove-slide = Eliminar la diapositiva
ui-toolbar-remove-hint = Diapositiva { $n } · deshaz con ⌘Z

## Etiquetas de deshacer

ui-undo-add = { $kind ->
    [card] Añadir una tarjeta
    [shape] Añadir una forma
    [chart] Añadir un gráfico
    [table] Añadir una tabla
   *[text] Añadir un texto
}
ui-undo-add-widget = Añadir «{ $name }»
ui-undo-new-template = Nueva plantilla: { $name }
ui-undo-new-slide = Nueva diapositiva: { $name }
ui-undo-templates = Plantillas: { $name }
ui-undo-move-template = Mover la plantilla
ui-undo-move-slide = Mover la diapositiva
ui-undo-remove-slide = Eliminar la diapositiva
ui-undo-remove = Eliminar
ui-undo-cut = Cortar
ui-undo-paste = Pegar
ui-undo-place-on-grid = Colocar en la cuadrícula
ui-undo-move-on-grid = Mover en la cuadrícula
ui-undo-resize-on-grid = Redimensionar en la cuadrícula
ui-undo-change-anchor = Cambiar el anclaje
ui-undo-move = Mover
ui-undo-resize = Redimensionar
ui-undo-edit-slide = Editar la diapositiva
ui-undo-dictation = Dictado
ui-undo-rename-set = Cambiar el nombre del conjunto
ui-undo-rename-deck = Cambiar el nombre de la presentación
ui-undo-chart-kind = Cambiar el tipo de gráfico
ui-undo-chart-data = Editar los datos
ui-undo-chart-color = Colorear una serie
ui-undo-chart-stack = { $stacked ->
    [yes] Apilar las series
   *[no] Separar las series
}

## Elementos nuevos

ui-new-text = Texto nuevo
ui-new-card-title = Título de la tarjeta
ui-new-card-detail = Detalle

## Pila de diapositivas

ui-pile-slides = Diapositivas
ui-pile-templates = Plantillas
ui-pile-add-slide = + Nueva diapositiva
ui-pile-add-template = + Nueva plantilla
ui-pile-drag-hint = Arrastra para mover · ⌥↑ ⌥↓
ui-slide-alt = Diapositiva { $n }

## Escenario

ui-stage-rendering = Generando…
ui-stage-elements = Elementos de la diapositiva
ui-stage-fields = Campos de la diapositiva
ui-stage-edit-text = Editar el texto
ui-stage-hint-plain = Intro o Tab para pasar al siguiente · Esc para cancelar
ui-stage-hint-rich = Tab para pasar al siguiente · ⌘ Intro para confirmar · «- » para una viñeta · ⌘B negrita, ⌘I cursiva, ⌘K enlace
ui-field = { $kind ->
    [title] Título
    [subtitle] Subtítulo
    [kicker] Antetítulo
    [text] Texto
    [body] Cuerpo
    [author] Autor
    [date] Fecha
    [organization] Organización
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Texto
    [card] Tarjeta
    [image] Imagen
    [shape] Forma
    [group] Grupo
   *[other] { $kind }
}
ui-grid-cell = col. { $cols } · fila { $rows }
ui-ghost-zone = Zona

## Notas de la diapositiva

ui-notes = Herramientas de la diapositiva
ui-stepper-less = { $label }: uno menos
ui-stepper-more = { $label }: uno más
ui-list-item = ítem
ui-list-add = Añadir { $item }
ui-list-remove = Quitar { $item }
ui-list-move = Mover { $item }
ui-list-full = La lista está llena ({ $max })
ui-undo-list-move = Mover { $item }
ui-agent-color-other = Otro color
ui-agent-thread = Agente
ui-agent-new-conversation = Nueva conversación
ui-agent-detach = Abrir el agente en una ventana aparte
ui-agent-attach = Devolver el agente a la ventana principal
ui-agent-window-empty = No hay agente para esta pestaña
ui-agent-kept-in-draft = Nada donde escribir: texto guardado en el borrador
ui-agent-busy-elsewhere = El agente está respondiendo en { $place }…
ui-agent-scope-slides = las diapositivas
ui-agent-scope-atelier = el taller
ui-draft = Borrador
ui-draft-insert = Insertar
ui-draft-insert-title = Insertar tal cual (⌘⇧↩)
ui-draft-send = Enviar al agente
ui-draft-send-title = Enviar al agente (⌘↩)
ui-draft-discard = Descartar
ui-agent-state-idle = Agente
ui-agent-state-starting = El agente se está iniciando…
ui-agent-state-working = El agente está trabajando…
ui-agent-dock = En curso
ui-agent-dock-settings = Ajustes
ui-agent-jump = ↓ Turno actual
ui-agent-sheet = Pregunta del agente
ui-agent-sheet-keys = 1–4 elegir · Intro confirmar · E retocar · Esc cancelar
ui-agent-variants = Variantes
ui-agent-drawer-keys = Clic o 1–4 · Esc cierra
ui-agent-confirm = Confirmación del agente

## Dictado

ui-dictation = Dictado
ui-dictation-start = Dictar
ui-dictation-start-title = Dictar (⌘D)
ui-dictation-type = Escribir al agente
ui-dictation-type-title = Escribir al agente (⌘K)
ui-dictation-agent-stop = Detener el agente (⌘⇧D o Esc)
ui-dictation-agent-working = El agente está trabajando…
ui-dictation-agent-working-hint = ⌘⇧D o Esc lo detiene
ui-dictation-finish = Terminar el dictado
ui-dictation-finish-title = Terminar (⌘D)
ui-dictation-cancel = Cancelar
ui-dictation-preparing = Preparando el dictado…
ui-dictation-heard-nothing = No se oyó nada: habla un poco más cerca del micrófono
ui-dictation-to-cursor = en el cursor
ui-dictation-to-draft = en el borrador
ui-dictation-to-field = sustituir el { $field }
ui-dictation-to-block = después del bloque seleccionado
ui-dictation-new-block = nuevo bloque de texto
ui-dictation-select-zone = selecciona una zona
ui-dictation-downloading = Descargando el modelo { $name }
ui-dictation-download-progress = { $percent } % · { $received } de { $total } · solo una vez
ui-dictation-listening = Escuchando · { $time }
ui-dictation-transcribing = Transcribiendo la última frase…
ui-dictation-stop-in = Se detiene en { $duration }
ui-dictation-speak = Habla, el texto aparece aquí…
ui-dictation-auto-stop = se detiene tras { $duration } de silencio
ui-dictation-to-download = { $size } para descargar
ui-dictation-settings = Ajustes del dictado
ui-dictation-model-title = Modelo de reconocimiento
ui-dictation-model-intro = Whisper, en este ordenador (CTranslate2). Todos los modelos entienden varios idiomas.
ui-dictation-note-turbo = Recomendado, rápido y preciso
ui-dictation-note-large = El más preciso, más lento
ui-dictation-note-base = Ligero, para equipos modestos
ui-dictation-installed = instalado
ui-dictation-not-installed = para descargar
ui-dictation-language = Idioma hablado
ui-dictation-language-auto = Detección automática
ui-dictation-note-apple = El más rápido, integrado en el sistema
ui-dictation-model-unavailable = Idioma no compatible
ui-dictation-model-intro-apple = Apple, integrado en el sistema, o Whisper, en este ordenador (CTranslate2). Todos entienden varios idiomas.
ui-dictation-language-auto-resolved = Automático ({ $language })
ui-dictation-to-download-language = Idioma para descargar
ui-dictation-auto-stop-label = Detenerse solo tras un silencio de
ui-dictation-silence = Duración del silencio
ui-dictation-phrases-note = Solo se transcriben los fragmentos hablados, frase a frase, mientras hablas.
ui-dictation-download-now = Descargar ahora ({ $size })
ui-dictation-download-language = Descargar el idioma
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Selector de plantillas

ui-picker-new-slide = Nueva diapositiva
ui-picker-new-template = Nueva plantilla, basada en…
ui-picker-set = Plantillas
ui-picker-set-mine = { $name } · mías
ui-picker-keys = ← → para explorar · Intro para añadir · Esc para cerrar
ui-picker-close = Cerrar
ui-picker-previous = Plantilla anterior
ui-picker-next = Plantilla siguiente
ui-picker-list = Plantillas de diapositivas

## Gráfico

ui-chart-panel = Gráfico
ui-chart-kinds = Tipo de gráfico
ui-chart-kind = { $kind ->
    [line] Líneas
    [pie] Circular
    [scatter] Dispersión
    [radar] Radar
    [waterfall] Cascada
   *[bar] Barras
}
ui-chart-no-data = Este gráfico no tiene tabla de datos.
ui-chart-options = Opciones del gráfico
ui-chart-stack = Apilado
ui-chart-waterfall-hint = Marque Total para una barra de total.
ui-chart-colors = Colores de las series
ui-chart-color-of = Color de { $name }
ui-chart-color-auto = Automático
ui-chart-color-pick = Color { $n }
ui-chart-column-name = Nombre de la columna { $col }
ui-chart-remove-column = Eliminar la columna
ui-chart-cell = Fila { $row }, columna { $col }
ui-chart-remove-row = Eliminar la fila
ui-chart-add-row = Fila
ui-chart-add-column = Columna
ui-chart-series = Serie { $n }

## Tabla

ui-table-panel = Tabla
ui-table-options = Opciones de la tabla
ui-table-header-row = Fila de encabezado
ui-table-header-column = Primera columna como encabezado
ui-table-striped = Filas alternas
ui-table-alignment = Alineación de las columnas
ui-table-align-of = Alineación de { $name }
ui-table-align = { $align ->
    [start] Inicio
    [center] Centro
    [end] Fin
   *[auto] Automática
}
ui-table-column = Columna { $n }
ui-table-no-data = Esta tabla no tiene datos.
ui-undo-table-option = Opciones de la tabla
ui-undo-table-align = Alinear la columna
ui-undo-table-data = Editar la tabla
ui-show-as = Mostrar como
ui-show-as-table = Mostrar como tabla
ui-show-as-chart = Mostrar como gráfico
ui-undo-show-as-table = Mostrar como tabla
ui-undo-show-as-chart = Mostrar como gráfico

## Archivos

ui-file-open-prompt = Ruta del archivo que abrir (.oku-slides.pdf o .json)
ui-file-save-prompt = Guardar como (ruta completa)

## Pestañas

ui-tabs = Presentaciones abiertas
ui-tab-new = Nueva pestaña
ui-tab-close = Cerrar pestaña
ui-tab-unsaved = Cambios sin guardar
ui-tab-templates = Plantillas
ui-tab-move-new-window = Mover a una ventana nueva
ui-tab-move-to = Mover a la ventana { $n }
ui-tab-close-others = Cerrar las demás pestañas
ui-close-cancel = Cancelar
ui-disk-replaced-question = { $file } fue reemplazado en el disco por otra presentación.
ui-disk-replaced-overwrite = Sobrescribir
ui-disk-replaced-save-as = Guardar como…
ui-same-deck-question = { $file } es una versión de la presentación abierta en la pestaña «{ $title }».
ui-same-deck-merge = Fusionar en esa pestaña
ui-same-deck-apart = Abrir por separado
ui-history = Historial
ui-history-version = Versión del { $date }
ui-history-restore = Restaurar
ui-history-back = Volver
ui-history-anonymous = Anónimo
ui-history-slider = Versiones de la presentación
ui-history-removed-since = eliminada desde entonces
ui-history-changed-since = modificada desde entonces
ui-history-added-since = diapositivas añadidas desde entonces: { $count }
ui-help = Ayuda
ui-help-caption = Diapositiva { $index } de { $count } · solo lectura
ui-slide-previous = Diapositiva anterior
ui-slide-next = Diapositiva siguiente
ui-toolbar-params-hide = Ocultar los ajustes
ui-toolbar-params-show = Mostrar los ajustes
ui-help-close = Cerrar la ayuda
ui-help-search = Buscar en la ayuda
ui-help-search-placeholder = Buscar…
ui-help-search-empty = Sin resultados
ui-help-search-count = { $count ->
    [one] 1 resultado
   *[other] { $count } resultados
}
ui-pack-notice-close = Cerrar
ui-widgets-files = Archivos de widgets
ui-widgets-common = Común
ui-widgets-new = Nuevo widget
ui-widgets-name = Nombre del widget (minúsculas, cifras y guiones)
ui-widgets-name-taken = Este nombre ya está en uso
ui-widgets-name-letter = El nombre debe empezar por una letra
ui-widgets-valid = Válido
ui-widgets-errors = Errores
ui-widgets-empty = Este paquete aún no tiene widgets.
ui-widgets-preview = Vista previa del widget
ui-widgets-no-preview = Sin vista previa
ui-widgets-field-bound = Vinculado a la presentación: solo lectura aquí
ui-widgets-source = Código fuente
ui-widgets-unsaved = Sin guardar
ui-widgets-in-error = Widgets con errores
ui-widgets-none-in-error = Ningún widget tiene errores
ui-widgets-section = Widgets
ui-widgets-open = Abrir el widget { $name }
ui-widgets-delete-named = Eliminar { $name }
ui-widgets-confirm-delete = ¿Eliminar el widget «{ $name }»?
ui-widgets-rename-hint = Doble clic o F2 para renombrar
ui-widgets-tabs = Pestañas de plantillas
ui-widgets-templates-tab = Plantillas
ui-theme-tab = Tema
ui-theme-picker = Tema
ui-theme-name = Nombre del tema
ui-theme-unsaved = Cambios del tema sin guardar
ui-widgets-close-tab = Cerrar { $name }
ui-widgets-removed-by-agent = El agente eliminó el widget { $name }.
