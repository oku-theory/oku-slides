op-slide-not-found = Diapositiva no encontrada: { $subject }
op-element-not-found = Elemento no encontrado: { $subject }
op-duplicate-id = Identificador ya en uso: { $subject }
op-index-out-of-range = Posición fuera de rango
op-invalid-value = Valor no válido para { $subject }
op-invalid-value-because = Valor no válido para { $subject }: { $reason }
op-value-no-value = el parámetro no tiene valor
op-value-no-dataset = el gráfico no tiene conjunto de datos
op-value-grid-tracks = una cuadrícula tiene de 1 a 24 columnas y filas
op-value-bad-item-id = un identificador de ítem es un texto no vacío, sin «/»
op-value-missing-item-id = cada ítem necesita un identificador
op-value-too-deep = los elementos se anidan demasiado
op-value-empty-path = la ruta está vacía
op-value-not-text = se espera un texto o null
op-value-empty-frame = el ancho y el alto deben ser mayores que 0
op-value-blocks-off-grid = la diapositiva dispone los slots de flujo y una cuadrícula los oculta; mueve primero cada elemento a un slot libre
op-value-outside-grid = la celda está fuera de la cuadrícula
op-value-not-tokens = se espera un objeto o null
op-value-not-a-slot = este parámetro no es un slot
op-value-out-of-bounds = el número está fuera del rango permitido
op-value-unreadable = el valor no tiene la forma esperada
op-value-valign = se espera top, middle o bottom
op-value-path = el trazado se compone de comandos M, L, C, Q y Z seguidos de sus números, empezando por M
op-unknown-widget = Widget desconocido: { $subject }
op-unknown-namespace = Esta presentación no tiene el paquete { $pack }.
op-pack-required = El paquete { $pack } lo necesita { $by }; quita ese primero.
op-pack-in-use = El paquete { $pack } todavía lo usa { $count ->
    [one] { $count } elemento
   *[other] { $count } elementos
}.
op-pack-requirement-missing = El paquete { $pack } necesita { $required }, que esta presentación no tiene.
op-base-required = Toda presentación conserva el paquete de widgets base.
op-pack-kind-conflict = Esta presentación ya tiene un paquete { $pack } del otro tipo (widgets o diseños).
op-theme-cycle = Los temas se extienden entre sí en un bucle: { $chain }.
op-theme-closure = Los temas fijados en esta presentación no son exactamente su tema y sus padres ({ $namespace }).
op-unknown-param = Parámetro desconocido: { $subject }
op-param-type-because = Valor no válido para { $subject }: { $reason }
op-param-shape-text = { $nullable ->
    [yes] se espera un texto o null
   *[no] se espera un texto
}
op-param-shape-choice = { $nullable ->
    [yes] se espera uno de sus valores o null
   *[no] se espera uno de sus valores
}
op-param-shape-paragraphs = { $nullable ->
    [yes] se espera una lista de párrafos o null
   *[no] se espera una lista de párrafos
}
op-param-shape-elements = { $nullable ->
    [yes] se espera una lista de elementos o null
   *[no] se espera una lista de elementos
}
op-param-shape-flag = { $nullable ->
    [yes] se espera un booleano o null
   *[no] se espera un booleano
}
op-param-shape-number = { $nullable ->
    [yes] se espera un número o null
   *[no] se espera un número
}
op-param-shape-bounded-number = { $nullable ->
    [yes] se espera un número dentro de sus límites o null
   *[no] se espera un número dentro de sus límites
}
op-param-shape-grid = { $nullable ->
    [yes] se espera una cuadrícula o null
   *[no] se espera una cuadrícula
}
op-param-shape-chart-option = { $nullable ->
    [yes] se espera una opción de gráfico o null
   *[no] se espera una opción de gráfico
}
op-param-shape-table-option = { $nullable ->
    [yes] se espera una opción de tabla o null
   *[no] se espera una opción de tabla
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] se espera una opción de gráfico o de tabla o null
   *[no] se espera una opción de gráfico o de tabla
}
op-param-shape-items = { $nullable ->
    [yes] se espera una lista de ítems o null
   *[no] se espera una lista de ítems
}
op-param-shape-list = { $nullable ->
    [yes] se espera una lista o null
   *[no] se espera una lista
}
op-param-shape-add-element-only = { $nullable ->
    [yes] los elementos se añaden con add_element, o null
   *[no] los elementos se añaden con add_element
}
op-param-shape-set-param-only = { $nullable ->
    [yes] el valor se da con set_param, o null
   *[no] el valor se da con set_param
}
op-param-shape-path-option = { $nullable ->
    [yes] se espera un trazado de forma (un objeto con d) o null
   *[no] se espera un trazado de forma (un objeto con d)
}
op-item-not-found = No existe el ítem de lista { $subject }
op-list-full = La lista { $subject } está llena (máximo { $max ->
   [one] { $max } ítem
  *[other] { $max } ítems
})
op-list-minimum = La lista { $subject } necesita al menos { $min ->
   [one] { $min } ítem
  *[other] { $min } ítems
}
op-list-cell-too-long = Una celda de la lista { $subject } es demasiado larga (columna { $column })
op-list-id-too-long = Un identificador de ítem de la lista { $subject } es demasiado largo
op-list-too-many-items = La lista { $subject } tiene demasiados ítems (máximo { $max })
op-slot-rejects = { $subject } no se acepta aquí
op-wrong-category = { $subject } no se puede usar aquí
op-choice-out-of-range = Valor no permitido para { $subject }
transaction-refused = Operación { $index } rechazada: { $reason }

sources-unreadable = Fuentes ilegibles
sources-not-sources = Esto no es un archivo de fuentes de Oku Slides
sources-unsupported-version = Estas fuentes se crearon con una versión más reciente de Oku Slides
sources-bad-name = Nombre de archivo rechazado en las fuentes
sources-missing-deck = Falta la presentación en las fuentes
sources-too-large = Las fuentes de este archivo son demasiado grandes para abrirlas
sources-too-deep = Esta presentación anida elementos a demasiada profundidad

issue-empty-slide-id = identificador de diapositiva vacío
issue-duplicate-slide-id = identificador de diapositiva duplicado
issue-duplicate-element-id = identificador de elemento duplicado
issue-empty-element-id = identificador vacío
issue-cell-outside-grid = celda fuera de la cuadrícula
issue-empty-frame = marco de tamaño cero
issue-free-without-frame = elemento libre sin marco
issue-unknown-widget = widget desconocido: { $kind }
issue-wrong-category = { $kind } no se puede usar aquí
issue-unknown-param = parámetro desconocido: { $param }
issue-param-type = valor no válido para { $param }
issue-choice-out-of-range = valor no permitido para { $param }
issue-slot-rejects = { $kind } no se acepta aquí
issue-placed-in-flow = elemento colocado con un marco en una lista maquetada
issue-unknown-bind = { $param } está vinculado a un ajuste del deck desconocido
issue-slot-too-deep = los elementos están anidados demasiado profundamente
issue-invalid-style = valor de estilo no válido: { $field } ({ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    })
issue-missing-asset = recurso ausente: { $asset }
issue-pack-version-mismatch = versión de paquete alterada descartada: { $version }
issue-pack-version-missing = versión de paquete ausente de las fuentes: { $version }
issue-pack-pin-mismatch = El paquete incrustado { $pack } no coincide con lo que espera la presentación.
issue-pack-tokens-invalid = El paquete incrustado { $pack } tiene un token de componente no válido: { $reason }
issue-pack-kind-mismatch = El paquete incrustado { $pack } no es del tipo que espera la presentación (widgets o diseños).
issue-pack-kind-conflict = El espacio de nombres { $pack } está fijado a la vez como paquete de widgets y como paquete de diseños: solo uno puede usarse.
issue-pack-requirement-missing = El paquete { $pack } necesita { $required }, que esta presentación no tiene.
issue-theme-version-mismatch = versión de tema alterada descartada: { $version }
issue-theme-version-missing = versión de tema ausente de las fuentes: { $version }
issue-theme-pin-mismatch = El tema incrustado { $namespace } no coincide con lo que espera la presentación.
issue-theme-closure = Los temas fijados en esta presentación no son exactamente su tema y sus padres ({ $namespace }).
pack-token-entry-invalid = Token de componente «{ $path }» no válido en el manifiesto del paquete.
pack-manifest-json = No se puede leer el manifiesto del paquete: { $reason }
issue-unsupported-link = enlace no admitido: { $link }
issue-highlight-zero = los colores de resaltado se numeran desde 1
issue-unreadable-element = elemento ilegible, apartado
issue-slot-repaired = elemento sin slot conocido, colocado en { $slot }
issue-unreadable-item = ítem de lista ilegible, apartado
issue-unreadable-row = fila de tabla ilegible, apartada
issue-data-truncated = datos demasiado grandes, truncados al leer
issue-unreadable-slide = diapositiva ilegible, apartada
issue-unreadable-field = campo { $field } del deck ilegible, restablecido
op-invalid-chart = Gráfico { $subject }: { $reason }
op-invalid-table = Tabla { $subject }: { $reason }

replica-corrupted = El historial de la presentación en este archivo está dañado
replica-too-large = El historial de la presentación en este archivo es demasiado grande para abrirlo
replica-write = No se pudo actualizar el historial de la presentación
replica-unrelated = Estas dos presentaciones no tienen un origen común
replica-unsupported-version = Esta presentación usa un formato de deck que esta versión de Oku Slides no puede leer
replica-too-deep = No se puede guardar esta presentación: hay elementos eliminados anidados a demasiada profundidad en su historial. Deshaga sus últimos movimientos y vuelva a guardar.

widget-unreadable = No se pudieron leer las declaraciones de widgets
widget-fault-empty-name = Widget { $widget }: un widget o parámetro no tiene nombre
widget-fault-duplicate-widget = El widget { $widget } está declarado dos veces
widget-fault-duplicate-param = El widget { $widget } declara el mismo parámetro dos veces
widget-fault-empty-choice = Widget { $widget }: una opción no tiene valores
widget-fault-default-not-in-choice = Widget { $widget }: el valor predeterminado de una opción no está entre sus valores
widget-fault-default-wrong-type = Widget { $widget }: un valor predeterminado no coincide con el tipo de su parámetro
widget-fault-bind-not-plain = Widget { $widget }: solo los parámetros de texto simple pueden vincularse a la presentación
widget-fault-unknown-slot-kind = Widget { $widget }: un slot acepta un widget de elemento desconocido
widget-fault-empty-list = Widget { $widget }: una lista necesita al menos una columna
widget-fault-column-type = Widget { $widget }: una columna de lista puede ser plain, rich, number, flag, choice o asset (o una p.list anidada, un nivel); para contenido libre por ítem, usa p.slot con un widget hijo
widget-fault-reserved-name = Widget { $widget }: nombre reservado: id, kind o un nombre que contenga /
widget-fault-list-bounds = Widget { $widget }: el máximo de una lista está por debajo de su mínimo
widget-fault-list-default-too-large = Widget { $widget }: el valor predeterminado de una lista supera los límites de tamaño de las listas
widget-fault-list-depth = Widget { $widget }: una lista anidada no puede contener una columna de lista; las listas solo se anidan un nivel
widget-fault-list-default-below-min = Widget { $widget }: el valor predeterminado de una lista anidada tiene menos elementos que su mínimo
widget-fault-slide-sizing = Widget { $widget } es una diapositiva: solo los widgets de elemento declaran grow o typography
widget-fault-pack-category = El widget { $widget } no corresponde a su pack: los packs de layouts contienen widgets de diapositiva y los packs de widgets, widgets de elemento
widget-fault-unqualified = El widget { $widget } no tiene espacio de nombres de pack
issue-empty-item-id = la lista { $param } tiene un ítem sin identificador
issue-duplicate-item-id = la lista { $param } tiene un identificador de ítem duplicado
issue-invalid-item-id = la lista { $param } tiene un identificador de elemento con una barra
issue-list-bounds = la lista { $param } tiene muy pocos o demasiados ítems
