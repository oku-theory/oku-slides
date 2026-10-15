pdf-unreadable = PDF ilegible
pdf-unsupported-version = Este PDF se creó con una versión más reciente de Oku Slides
pdf-corrupted = Las fuentes de este PDF están dañadas
pdf-write = No se pudo escribir el PDF

render-compile = No se pudo generar la presentación
render-pdf = Falló la exportación a PDF
render-no-page = Esta página no existe
render-setup = No se pudo iniciar el motor de renderizado
render-image = No se pudo producir la imagen de la diapositiva
render-missing-image = imagen ausente
diagnostic-error = error
diagnostic-warning = advertencia
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = sugerencia: { $hint }
render-chart-failed = no se pudo dibujar el gráfico

registry-bad-pack-id = Identificador de paquete de plantillas no válido: { $id }
registry-compile = No se pudieron compilar los widgets del paquete de plantillas
registry-missing = El paquete de plantillas no declara sus widgets
render-timeout = El renderizado tardó más de { $seconds } s y se interrumpió
render-out-of-memory = El renderizado necesitó más de { $mib } MiB de memoria y se interrumpió
render-worker-lost = El proceso de renderizado se detuvo
render-frame = Mensaje ilegible del proceso de renderizado
render-page-too-large = Esta diapositiva es demasiado grande para la vista previa (más de { $mib } MiB, a menudo por una imagen muy grande). Aún se puede exportar como PDF o PNG.
render-stale = El renderizado fue reemplazado; inténtelo de nuevo
render-pack-disabled = Los widgets del paquete «{ $pack }» están desactivados en esta sesión
render-protocol = El proceso de renderizado pertenece a otra versión de Oku Slides; reinicie la aplicación
render-remote = El proceso de renderizado no pudo completar la solicitud
registry-schema-remote = Registro de widgets no válido: { $message }
probe-not-probed = Aún sin comprobar: se agotó el tiempo asignado a los widgets del paquete. Se comprobará en el próximo renderizado.
probe-too-many-files = El paquete tiene más de { $max } archivos de widgets: este no se carga.
probe-too-large = Más de { $max } KiB: no se carga.

import-unreadable = No se puede leer esta presentación
import-not-a-presentation = Este archivo no es una presentación de PowerPoint
import-protected = Esta presentación está cifrada o en el antiguo formato .ppt: guárdela primero como .pptx
import-too-large = Esta presentación es demasiado grande para importarla
import-output-too-large = La presentación resultante superaría el tamaño permitido para una importación
import-loss-list-levels = Niveles de lista reducidos a uno
import-loss-merged-cells = Celdas combinadas separadas
import-loss-percent-stacked = Gráfico apilado al 100 % mostrado como apilado
import-loss-run-style = { $effect ->
    [underline] Subrayado perdido
    [strike] Tachado perdido
    [baseline] Superíndice o subíndice perdido
    [caps] Mayúsculas perdidas
    [font] Fuente de un fragmento sustituida por la del cuadro
    [tracking] Espaciado de letras de un fragmento sustituido por el del cuadro
    [weight] Fragmento normal mostrado en negrita como el cuadro
    [slant] Fragmento recto mostrado en cursiva como el cuadro
   *[highlight] Resaltado perdido
}
import-loss-transform = Rotación, volteo o recorte perdidos
import-loss-color-dropped = El color { $color } no tiene un color cercano en el tema: se usa un color del tema
import-loss-text-truncated = Texto acortado para respetar los límites
import-loss-table-truncated = Tabla acortada para respetar los límites
import-loss-opaque = { $reason ->
    [media] Vídeo o sonido no importado (sin imagen de respaldo)
    [ole] Objeto incrustado no importado (sin imagen de respaldo)
    [math] Ecuación no importada (sin imagen de respaldo)
    [smartart] SmartArt no importado (sin imagen de respaldo)
   *[unknown] Objeto no compatible no importado (sin imagen de respaldo)
}
import-loss-unsupported-image = Imagen en formato { $format } no importada
import-loss-hidden-slide = Diapositiva oculta importada como visible
import-loss-effects = Sombras, reflejos y efectos 3D perdidos
import-loss-series-dropped = Un gráfico circular muestra una sola serie: { $kept } conservada, { $dropped } descartadas
import-loss-repeated-decor = { $slides ->
   [one] Encabezado o pie «{ $text }» repetido en { $slides } diapositiva omitido: el tema dibuja los suyos
  *[other] Encabezado o pie «{ $text }» repetido en { $slides } diapositivas omitido: el tema dibuja los suyos
}
import-loss-no-slot = Contenido omitido: la plantilla elegida no tiene sitio para él
import-loss-page-fitted = La página de origen ({ $width } × { $height } pt) no es 16:9: contenido escalado y centrado
import-loss-paragraph-style = Alineación, interlineado o espaciado de un párrafo ajustados a los del cuadro
import-loss-shape-style = { $effect ->
    [gradient] Degradado mostrado como color sólido
    [picture] Imagen de relleno estirada al marco de la forma
    [dash] Línea discontinua mostrada continua
   *[geometry] Forma desconocida mostrada como rectángulo
}
import-loss-layout-dropped = Demasiados diseños para un paquete: «{ $layout }» no tiene plantilla; sus diapositivas usan el widget vacío y reciben su decoración como elementos
import-loss-decor-truncated = Decoración recortada para caber en el tamaño de un archivo del paquete: { $omitted } elementos omitidos
import-loss-table-style = Estilo de tabla aproximado: se pierden sus fondos de celda, bordes, anchos de columna o tamaños de texto propios
import-loss-overflow = El elemento «{ $element }» sobrepasa el borde de la diapositiva
import-reference-unavailable = Sin puntuación visual: { $cause ->
    [timeout] el renderizado de referencia tardó demasiado
    [memory] el renderizado de referencia usó demasiada memoria
    [pages] el renderizado de referencia tiene { $reference ->
        [one] { $reference } página
       *[other] { $reference } páginas
    } para { $deck } en la presentación
   *[failed] el renderizador de referencia no pudo leer la presentación
}
import-font-substituted = { $family } sustituida por { $by }, con métricas compatibles
import-font-missing = { $family } no encontrada: el texto usa { $by }
import-font-restricted = { $family }: su licencia prohíbe incrustar algunas de sus variantes en la presentación
import-font-restricted-substituted = { $family } no se puede incrustar en la presentación (licencia): sustituida por { $by }, con métricas compatibles
import-font-restricted-missing = { $family } no se puede incrustar en la presentación (licencia): el texto usa { $by }
import-font-too-large = { $family }: archivos demasiado grandes para incrustarlos en el deck; el texto usa { $by }
import-font-missing-no-fallback = No se encuentra { $family } ni la fuente del cuerpo del tema: el texto se muestra con la fuente predeterminada del motor
import-warning-effect-ignored = Efecto no importado: { $detail }
import-warning-approximated = Representación aproximada: { $detail }
import-warning-unreadable-item = Elemento ilegible omitido: { $detail }
import-warning-unreadable-part = Parte ilegible: { $detail }
import-warning-missing-part = Falta una parte: { $detail }
import-warning-unsupported-chart = Gráfico no compatible: { $detail }
import-warning-unsupported-image = Formato de imagen no compatible: { $detail }
import-warning-oversized-image = Imagen demasiado grande para importarla (número de píxeles): { $detail }
import-warning-external-image = Imagen vinculada fuera del archivo, no importada: { $detail }
import-warning-unsafe-svg = Imagen SVG depurada de contenido inseguro, o descartada: { $detail }
import-warning-unknown-table-style = Estilo de tabla desconocido: { $detail }
import-warning-unsafe-link = Vínculo rechazado: { $detail }
import-warning-nesting-too-deep = Anidamiento demasiado profundo, cortado: { $detail }
import-warning-font-embedded-unreadable = Fuente incrustada ilegible: { $detail }
import-warning-truncated = Demasiadas advertencias: se omiten las siguientes
