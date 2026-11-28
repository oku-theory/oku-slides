data-cell-type = um valor não cabe na coluna “{ $column }”, que contém { $type ->
    [number] números
    [integer] números inteiros
    [bool] verdadeiro ou falso
    [date] datas (AAAA-MM-DD)
   *[text] texto de no máximo 16 KB
}
data-column-type = a coluna “{ $column }” não pode virar sem perda { $type ->
    [number] uma coluna de números
    [integer] uma coluna de inteiros
    [bool] uma coluna de verdadeiro ou falso
    [date] uma coluna de datas
   *[text] uma coluna de texto
}, veja as linhas { $rows }{ $more ->
    [0] {""}
    [one] {" "}e mais { $more } linha
   *[other] {" "}e mais { $more } linhas
}
data-no-row = a linha { $row } não existe
data-no-column = a coluna { $column } não existe
data-id-taken = o identificador { $id } já é usado nesta tabela
data-id-invalid = o identificador “{ $id }” não é válido: use de 1 a 64 letras, dígitos, - ou _
data-id-missing = uma nova linha, coluna ou tabela não tem identificador
data-too-large = { $what ->
    [rows] os dados passariam do limite de { $max } { $max ->
        [one] linha
       *[other] linhas
    }
    [columns] os dados passariam do limite de { $max } { $max ->
        [one] coluna
       *[other] colunas
    }
    [text] uma célula de texto passaria do limite de { $max } { $max ->
        [one] byte
       *[other] bytes
    }
   *[overlay] os dados passariam do limite de { $max } { $max ->
        [one] célula editada
       *[other] células editadas
    }
}
data-invalid-name = “{ $name }” não é um nome de dados válido: use minúsculas, dígitos e _, sem começar por um dígito
data-non-finite = { $count ->
    [one] { $count } número não é finito (NaN ou infinito)
   *[other] { $count } números não são finitos (NaN ou infinito)
}
data-columns-invalid = a lista de colunas está vazia ou cita uma coluna duas vezes
data-row-width = uma linha tem mais células que as { $max } { $max ->
    [one] coluna
   *[other] colunas
} da tabela
data-needs-base = estes dados guardam as linhas em um arquivo base que esta edição ainda não consegue ler
data-base-row-cells = uma tabela com arquivo base só indica os ids das linhas, não células
data-row-cut = { $count ->
    [one] { $count } linha tinha mais células que o cabeçalho; as células a mais foram removidas
   *[other] { $count } linhas tinham mais células que o cabeçalho; as células a mais foram removidas
}
data-base-invalid = o arquivo base dos dados está danificado ou não corresponde à sua tabela
data-source-mismatch = a origem dos dados não corresponde ao tipo: dados manuais têm uma tabela, dados de consulta têm uma consulta
data-inline-query = dados de consulta não podem ser incorporados: dados incorporados vêm sempre de uma tabela
data-query-parse = a consulta não pode ser lida: verifique a linha { $line }, coluna { $column }
data-query-construct = a consulta usa “{ $construct }”, que as consultas de dados não aceitam
data-query-too-long = a consulta passa de { $max } { $max ->
    [one] byte
   *[other] bytes
}
data-query-too-deep = a consulta está aninhada demais
data-query-rewrite = não foi possível preparar a consulta para o motor; é um erro do Oku Slides
data-query-unknown-function = a consulta chama “{ $function }”, que as consultas de dados não aceitam
data-query-run = a consulta falhou: { $reason }
data-query-input-failed = a sua entrada «{ $input }» falhou
data-query-too-large = { $what ->
    [rows] o resultado da consulta passa de { $max } { $max ->
        [one] linha
       *[other] linhas
    }; ele nunca é cortado
    [columns] o resultado da consulta passa de { $max } { $max ->
        [one] coluna
       *[other] colunas
    }; ele nunca é cortado
   *[text] uma célula de texto do resultado passa de { $max } { $max ->
        [one] byte
       *[other] bytes
    }; o resultado nunca é cortado
}
data-query-column-type = a coluna de resultado “{ $column }” tem um tipo que os dados não comportam: converta-a em texto, número, inteiro, booleano ou data
data-query-duplicate-column = duas colunas de resultado se chamam “{ $column }”: renomeie uma delas com AS
data-query-float-operand = “{ $function }” não pode ser aplicado a “{ $operand }”, que tem números decimais: converta-o antes em inteiro
data-query-timeout = a consulta durou mais de { $seconds } { $seconds ->
    [one] segundo
   *[other] segundos
} e foi interrompida
