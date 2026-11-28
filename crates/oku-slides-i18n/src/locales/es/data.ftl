data-cell-type = un valor no encaja en la columna «{ $column }», que contiene { $type ->
    [number] números
    [integer] números enteros
    [bool] verdadero o falso
    [date] fechas (AAAA-MM-DD)
   *[text] texto de 16 KB como máximo
}
data-column-type = la columna «{ $column }» no puede convertirse sin pérdida en { $type ->
    [number] una columna de números
    [integer] una columna de enteros
    [bool] una columna de verdadero o falso
    [date] una columna de fechas
   *[text] una columna de texto
}, ver las filas { $rows }{ $more ->
    [0] {""}
    [one] {" "}y { $more } fila más
   *[other] {" "}y { $more } filas más
}
data-no-row = la fila { $row } no existe
data-no-column = la columna { $column } no existe
data-id-taken = el identificador { $id } ya se usa en esta tabla
data-id-invalid = el identificador «{ $id }» no es válido: usa de 1 a 64 letras, dígitos, - o _
data-id-missing = una nueva fila, columna o tabla no tiene identificador
data-too-large = { $what ->
    [rows] los datos superarían su límite de { $max } { $max ->
        [one] fila
       *[other] filas
    }
    [columns] los datos superarían su límite de { $max } { $max ->
        [one] columna
       *[other] columnas
    }
    [text] una celda de texto superaría su límite de { $max } { $max ->
        [one] byte
       *[other] bytes
    }
   *[overlay] los datos superarían su límite de { $max } { $max ->
        [one] celda editada
       *[other] celdas editadas
    }
}
data-invalid-name = «{ $name }» no es un nombre de datos válido: usa minúsculas, dígitos y _, sin empezar por un dígito
data-non-finite = { $count ->
    [one] { $count } número no es finito (NaN o infinito)
   *[other] { $count } números no son finitos (NaN o infinito)
}
data-columns-invalid = la lista de columnas está vacía o nombra una columna dos veces
data-row-width = una fila tiene más celdas que las { $max } { $max ->
    [one] columna
   *[other] columnas
} de la tabla
data-needs-base = estos datos guardan sus filas en un archivo base que esta edición aún no sabe leer
data-base-row-cells = una tabla con archivo base solo da los id de sus filas, no celdas
data-row-cut = { $count ->
    [one] { $count } fila tenía más celdas que el encabezado; se quitaron las celdas sobrantes
   *[other] { $count } filas tenían más celdas que el encabezado; se quitaron las celdas sobrantes
}
data-base-invalid = el archivo base de los datos está dañado o no coincide con su tabla
data-source-mismatch = el origen de los datos no coincide con su tipo: unos datos manuales llevan una tabla, unos datos de consulta una consulta
data-inline-query = unos datos de consulta no pueden estar incrustados: los datos incrustados siempre provienen de una tabla
data-query-parse = la consulta no se puede leer: revisa la línea { $line }, columna { $column }
data-query-construct = la consulta usa «{ $construct }», que las consultas de datos no admiten
data-query-too-long = la consulta supera { $max } { $max ->
    [one] byte
   *[other] bytes
}
data-query-too-deep = la consulta está demasiado anidada
data-query-rewrite = no se pudo preparar la consulta para el motor; es un error de Oku Slides
data-query-unknown-function = la consulta llama a «{ $function }», que las consultas de datos no admiten
data-query-run = la consulta falló: { $reason }
data-query-input-failed = su entrada «{ $input }» falló
data-query-too-large = { $what ->
    [rows] el resultado de la consulta supera { $max } { $max ->
        [one] fila
       *[other] filas
    }; nunca se recorta
    [columns] el resultado de la consulta supera { $max } { $max ->
        [one] columna
       *[other] columnas
    }; nunca se recorta
   *[text] una celda de texto del resultado supera { $max } { $max ->
        [one] byte
       *[other] bytes
    }; el resultado nunca se recorta
}
data-query-column-type = la columna de resultado «{ $column }» tiene un tipo que los datos no pueden guardar: conviértela en texto, número, entero, booleano o fecha
data-query-duplicate-column = dos columnas de resultado se llaman «{ $column }»: cambia el nombre de una con AS
data-query-float-operand = «{ $function }» no se puede aplicar a «{ $operand }», que contiene números decimales: conviértelo antes en entero
data-query-timeout = la consulta duró más de { $seconds } { $seconds ->
    [one] segundo
   *[other] segundos
} y se detuvo
