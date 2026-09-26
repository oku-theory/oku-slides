# Interfaz de la app (React). Cargada por ui/src/i18n.ts mediante la solicitud «messages».
ui-loading = Cargando…

## Documento

ui-brand = Écho
ui-brand-templates = Écho · plantillas
ui-title-rename = Cambiar el nombre de la presentación
ui-title-field = Título de la presentación
ui-status-unsaved = cambios sin guardar
ui-status-saved = guardado
ui-status-never-saved = aún no guardado
ui-status-builtin = integrado: al guardarlo se crea una copia
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

ui-toast-saved = Tu presentación se ha guardado
ui-toast-templates-saved = Plantillas guardadas: { $name }
ui-toast-select-zone = Selecciona una zona donde colocar: «{ $text }»

## Barra de herramientas

ui-toolbar-undo = Deshacer
ui-toolbar-undo-label = Deshacer: { $label }
ui-toolbar-redo = Rehacer
ui-toolbar-redo-label = Rehacer: { $label }
ui-toolbar-grid-hide = Ocultar la cuadrícula
ui-toolbar-grid-show = Mostrar la cuadrícula
ui-toolbar-grid-hide-title = Ocultar la cuadrícula (⌘')
ui-toolbar-grid-show-title = Mostrar la cuadrícula (⌘')
ui-toolbar-theme = Tema
ui-toolbar-theme-default = Predeterminado
ui-toolbar-save-copy = Guardar una copia
ui-toolbar-save = Guardar
ui-toolbar-finish = Listo
ui-toolbar-templates = Plantillas
ui-toolbar-templates-title = Crear y editar plantillas de diapositivas
ui-toolbar-open = Abrir
ui-toolbar-add-label = Añadir a la diapositiva
ui-toolbar-add = Añadir
ui-add-kind = { $kind ->
    [card] Tarjeta
    [shape] Forma
    [chart] Gráfico
   *[text] Texto
}
ui-toolbar-remove-template = Eliminar la plantilla
ui-toolbar-remove-slide = Eliminar la diapositiva
ui-toolbar-remove-hint = Diapositiva { $n } · deshaz con ⌘Z

## Etiquetas de deshacer

ui-undo-add = { $kind ->
    [card] Añadir una tarjeta
    [shape] Añadir una forma
    [chart] Añadir un gráfico
   *[text] Añadir un texto
}
ui-undo-new-template = Nueva plantilla: { $name }
ui-undo-new-slide = Nueva diapositiva: { $name }
ui-undo-grid = Cuadrícula { $columns } × { $rows }
ui-undo-templates = Plantillas: { $name }
ui-undo-move-template = Mover la plantilla
ui-undo-move-slide = Mover la diapositiva
ui-undo-remove-slide = Eliminar la diapositiva
ui-undo-remove = Eliminar
ui-undo-place-on-grid = Colocar en la cuadrícula
ui-undo-move-on-grid = Mover en la cuadrícula
ui-undo-resize-on-grid = Redimensionar en la cuadrícula
ui-undo-change-anchor = Cambiar el anclaje
ui-undo-move = Mover
ui-undo-resize = Redimensionar
ui-undo-edit-text = Editar el texto
ui-undo-dictation = Dictado
ui-undo-rename-set = Cambiar el nombre del conjunto
ui-undo-rename-deck = Cambiar el nombre de la presentación
ui-undo-theme = Cambiar el tema
ui-undo-rename-template = Cambiar el nombre de la plantilla
ui-undo-describe-template = Describir la plantilla
ui-undo-chart-kind = Cambiar el tipo de gráfico
ui-undo-chart-data = Editar los datos
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
ui-stage-edit-text = Editar el texto
ui-stage-hint-field = Intro para confirmar · Esc para cancelar
ui-stage-hint-block = ⌘ Intro para confirmar · una línea por párrafo, «- » para una viñeta
ui-field = { $kind ->
    [title] Título
    [subtitle] Subtítulo
    [kicker] Antetítulo
    [text] Texto
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

## Notas de la diapositiva

ui-notes = Herramientas de la diapositiva
ui-stepper-less = { $label }: uno menos
ui-stepper-more = { $label }: uno más

## Dictado

ui-dictation = Dictado
ui-dictation-start = Dictar
ui-dictation-start-title = Dictar (⌘D)
ui-dictation-finish = Terminar el dictado
ui-dictation-finish-title = Terminar (⌘D)
ui-dictation-cancel = Cancelar
ui-dictation-preparing = Preparando el dictado…
ui-dictation-heard-nothing = No se oyó nada: habla un poco más cerca del micrófono
ui-dictation-to-cursor = en el cursor
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
ui-dictation-auto-stop-label = Detenerse solo tras un silencio de
ui-dictation-silence = Duración del silencio
ui-dictation-phrases-note = Solo se transcriben los fragmentos hablados, frase a frase, mientras hablas.
ui-dictation-download-now = Descargar ahora ({ $size })
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
ui-chart-waterfall-hint = Deja un valor vacío para una barra de total.
ui-chart-column-name = Nombre de la columna { $col }
ui-chart-remove-column = Eliminar la columna
ui-chart-cell = Fila { $row }, columna { $col }
ui-chart-remove-row = Eliminar la fila
ui-chart-add-row = + Fila
ui-chart-add-column = + Columna
ui-chart-series = Serie { $n }

## Archivos

ui-file-open-prompt = Ruta del archivo que abrir (.echo.pdf o .json)
ui-file-save-prompt = Guardar como (ruta completa)
