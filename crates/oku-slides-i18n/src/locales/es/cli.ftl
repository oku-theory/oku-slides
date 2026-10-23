cli-reading = leyendo { $path }
cli-invalid-deck = presentación no válida: { $path }
cli-deck-valid = presentación válida: { $slides } diapositivas
cli-deck-issue = problema: { $issue }
cli-rendered = { $pages } páginas → { $out } (compilación { $compile }, SVG { $svg })
cli-asset-added = recurso { $path } → { $id }
cli-saved = { $path }: { $pages } páginas, PDF { $pdf } KB, fuentes { $sources } KB
cli-opened = { $path }: { $slides } diapositivas, { $assets } recursos, { $fonts } tipografías
cli-extracted = extraído en { $dir }
cli-stripped = { $path }: copia sin fuentes
cli-pdf-modified = otra herramienta modificó el PDF después de Oku Slides
cli-iterations = --iterations debe ser > 0
cli-no-block = la presentación no tiene ningún bloque que editar
cli-blob-missing = no se encontró el blob de fuentes
cli-roundtrip-mismatch = la presentación releída difiere de la guardada
cli-bench-world = construcción del mundo (tipografías)  { $time }
cli-bench-first = primera compilación                   { $time }  ({ $pages } páginas)
cli-bench-same = recompilación sin cambios             { $time }
cli-bench-edited-text = Texto editado { $n }
cli-bench-edit = recompilación tras set_text           { $time }
cli-bench-svg = SVG de cada diapositiva               { $time }  ({ $each } por diapositiva)
cli-bench-pdf = exportación PDF                       { $time }  ({ $size } KB)
cli-bench-embed = incrustación del blob de fuentes      { $time }
cli-bench-extract = relectura del blob                    { $time }
cli-bench-roundtrip = ida y vuelta de la presentación       idéntica
cli-bench-history = paso del historial                    { $mean } ms de media, p50 { $p50 } ms
cli-serving = Oku Slides (dev) en http://{ $host }:{ $port }
cli-serving-token = abra { $url } (el token es válido hasta que el servidor se detenga)
cli-models-dir = modelos de dictado: { $dir }
cli-stopped = servidor detenido
cli-not-found = no encontrado
cli-bad-request = solicitud no válida: { $reason }
cli-sse-full = demasiados flujos de eventos ya abiertos
cli-forbidden = solicitud rechazada: no proviene de la interfaz local
cli-unauthorized = solicitud rechazada: token de acceso ausente o incorrecto
cli-exposed = escuchando en { $host }: el servidor es accesible desde la red; solo se atiende a los clientes que presentan el token de este arranque. Mantenga [serve] host en 127.0.0.1 si no necesita la red
cli-imported = { $path }: { $slides ->
       [one] { $slides } diapositiva
      *[other] { $slides } diapositivas
    }, { $pages ->
       [one] { $pages } página
      *[other] { $pages } páginas
    }, { $losses ->
       [one] { $losses } pérdida
      *[other] { $losses } pérdidas
    }, { $requests ->
       [one] { $requests } solicitud de widget
      *[other] { $requests } solicitudes de widget
    }
cli-import-write-failed = no se pudo escribir la presentación importada { $path }
cli-import-hifi-no-target = --pack y --theme son del modo retrofit: la importación de alta fidelidad (--mode hifi) genera su propio paquete y tema
cli-import-reference-hifi-only = --reference pertenece al modo hifi: una importación retrofit no se compara con un renderizado de referencia
cli-import-scored = Diferencia visual con { $witness }: { $mean } de media, { $above ->
    [0] ninguna diapositiva
    [one] { $above } diapositiva
   *[other] { $above } diapositivas
} por encima de { $threshold }
cli-import-unscored = { $reason }

cli-about = Genera y almacena presentaciones de Oku Slides
cli-arg-fonts = Carpeta de tipografías adicional (TTF/OTF); por defecto: paths.fonts en la configuración
cli-cmd-check = Comprobar una presentación
cli-arg-deck = Presentación (JSON)
cli-cmd-render = Genera cada diapositiva en SVG, con los cuadros de los elementos
cli-arg-input = Archivo deck.json o .oku-slides.pdf
cli-arg-out = Carpeta de salida
cli-arg-theme = Sustituir el tema de la presentación: uno incluido o de la biblioteca, por su espacio de nombres, id o nombre
cli-cmd-save = Guardar una presentación como .oku-slides.pdf (PDF + blob de fuentes)
cli-arg-output = Archivo que se escribirá
cli-arg-assets = Carpeta de recursos: cada archivo se convierte en un recurso sha256:…
cli-arg-embed-fonts = Incrustar también en las fuentes las tipografías de --fonts
cli-cmd-open = Reabrir un .oku-slides.pdf: extraer la presentación y sus recursos
cli-arg-extract = Carpeta donde escribir deck.json, assets/ y fonts/
cli-cmd-strip = Copiar un .oku-slides.pdf sin su blob de fuentes
cli-cmd-bench = Medir los tiempos de compilación y exportación
cli-arg-iterations = Mediciones por paso
cli-cmd-schema = Mostrar el esquema JSON de las operaciones
cli-cmd-serve = Servidor de desarrollo: la interfaz en un navegador, sin Tauri
cli-arg-serve-input = Presentación que abrir (.oku-slides.pdf o .json)
cli-arg-port = Puerto; por defecto: serve.port en la configuración
cli-arg-ui = Carpeta de la interfaz compilada (ui/dist)
cli-cmd-import = Importar una presentación de PowerPoint (.pptx) como .oku-slides.pdf
cli-arg-pptx = Presentación de PowerPoint (.pptx)
cli-arg-mode = retrofit: el contenido sobre un paquete y un tema existentes; hifi: copia fiel, sobre un paquete y un tema generados para ella
cli-arg-pack = Paquete de diseños para retrofit (espacio de nombres, id o carpeta); repetible, por defecto essentiel
cli-arg-import-theme = Tema de destino: uno incluido o de la biblioteca, por su espacio de nombres, id o nombre; por defecto: el tema predeterminado
cli-arg-report = Escribir también el informe de importación (JSON): pérdidas, sustituciones, cobertura
cli-arg-no-system-fonts = En modo hifi, no buscar las fuentes de la presentación entre las fuentes del sistema
cli-arg-reference = En modo hifi, comparar cada diapositiva con un renderizado de referencia de la presentación: auto (por defecto: [import] witness, si no office2pdf), office2pdf, soffice (LibreOffice) o no (none)
