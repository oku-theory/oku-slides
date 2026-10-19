op-slide-not-found = Slide não encontrado: { $subject }
op-element-not-found = Elemento não encontrado: { $subject }
op-duplicate-id = Identificador já em uso: { $subject }
op-index-out-of-range = Posição fora do intervalo
op-invalid-value = Valor inválido para { $subject }
op-invalid-value-because = Valor inválido para { $subject }: { $reason }
op-value-no-value = o parâmetro não tem valor
op-value-no-dataset = o gráfico não tem conjunto de dados
op-value-grid-tracks = uma grade tem de 1 a 24 colunas e linhas
op-value-bad-item-id = um identificador de item é um texto não vazio, sem “/”
op-value-missing-item-id = cada item precisa de um identificador
op-value-too-deep = os elementos estão aninhados demais
op-value-empty-path = o caminho está vazio
op-value-not-text = é esperado um texto ou null
op-value-empty-frame = a largura e a altura devem ser maiores que 0
op-value-blocks-off-grid = os slots de fluxo são dispostos pelo slide e uma grade os esconde; mova primeiro cada elemento para um slot livre
op-value-outside-grid = a célula está fora da grade
op-value-not-tokens = é esperado um objeto ou null
op-value-not-a-slot = este parâmetro não é um slot
op-value-out-of-bounds = o número está fora do intervalo permitido
op-value-unreadable = o valor não tem a forma esperada
op-value-valign = espera-se top, middle ou bottom
op-value-path = o traçado é composto por comandos M, L, C, Q e Z seguidos dos seus números, começando por M
op-unknown-widget = Widget desconhecido: { $subject }
op-unknown-namespace = Esta apresentação não tem o pacote de widgets { $pack }.
op-pack-required = O pacote de widgets { $pack } é necessário para { $by }; remova esse primeiro.
op-pack-in-use = O pacote de widgets { $pack } ainda é usado por { $count ->
    [one] { $count } item
   *[other] { $count } itens
}.
op-pack-requirement-missing = O pacote de widgets { $pack } precisa de { $required }, que esta apresentação não tem.
op-base-required = Toda apresentação mantém o pacote de widgets base.
op-unknown-param = Parâmetro desconhecido: { $subject }
op-param-type-because = Valor inválido para { $subject }: { $reason }
op-param-shape-text = { $nullable ->
    [yes] é esperado um texto ou null
   *[no] é esperado um texto
}
op-param-shape-choice = { $nullable ->
    [yes] é esperado um dos seus valores ou null
   *[no] é esperado um dos seus valores
}
op-param-shape-paragraphs = { $nullable ->
    [yes] é esperado uma lista de parágrafos ou null
   *[no] é esperado uma lista de parágrafos
}
op-param-shape-elements = { $nullable ->
    [yes] é esperado uma lista de elementos ou null
   *[no] é esperado uma lista de elementos
}
op-param-shape-flag = { $nullable ->
    [yes] é esperado um booleano ou null
   *[no] é esperado um booleano
}
op-param-shape-number = { $nullable ->
    [yes] é esperado um número ou null
   *[no] é esperado um número
}
op-param-shape-bounded-number = { $nullable ->
    [yes] é esperado um número dentro dos seus limites ou null
   *[no] é esperado um número dentro dos seus limites
}
op-param-shape-grid = { $nullable ->
    [yes] é esperado uma grade ou null
   *[no] é esperado uma grade
}
op-param-shape-chart-option = { $nullable ->
    [yes] é esperado uma opção de gráfico ou null
   *[no] é esperado uma opção de gráfico
}
op-param-shape-table-option = { $nullable ->
    [yes] é esperado uma opção de tabela ou null
   *[no] é esperado uma opção de tabela
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] é esperado uma opção de gráfico ou de tabela ou null
   *[no] é esperado uma opção de gráfico ou de tabela
}
op-param-shape-items = { $nullable ->
    [yes] é esperado uma lista de itens ou null
   *[no] é esperado uma lista de itens
}
op-param-shape-list = { $nullable ->
    [yes] é esperado uma lista ou null
   *[no] é esperado uma lista
}
op-param-shape-add-element-only = { $nullable ->
    [yes] os elementos são adicionados com add_element, ou null
   *[no] os elementos são adicionados com add_element
}
op-param-shape-set-param-only = { $nullable ->
    [yes] o valor é definido com set_param, ou null
   *[no] o valor é definido com set_param
}
op-param-shape-path-option = { $nullable ->
    [yes] espera-se um traçado de forma (um objeto com d) ou null
   *[no] espera-se um traçado de forma (um objeto com d)
}
op-item-not-found = Nenhum item { $subject }
op-list-full = A lista { $subject } está cheia (no máximo { $max ->
   [one] { $max } item
  *[other] { $max } itens
})
op-list-minimum = A lista { $subject } precisa de pelo menos { $min ->
   [one] { $min } item
  *[other] { $min } itens
}
op-list-cell-too-long = Uma célula da lista { $subject } é longa demais (coluna { $column })
op-list-id-too-long = Um identificador de item da lista { $subject } é longo demais
op-list-too-many-items = A lista { $subject } tem itens demais (no máximo { $max })
op-slot-rejects = { $subject } não é aceito aqui
op-wrong-category = { $subject } não pode ser usado aqui
op-choice-out-of-range = Valor não permitido para { $subject }
transaction-refused = Operação { $index } recusada: { $reason }

sources-unreadable = Fontes ilegíveis
sources-not-sources = Isto não é um arquivo de fontes do Oku Slides
sources-unsupported-version = Estas fontes foram gravadas por uma versão mais recente do Oku Slides
sources-bad-name = Nome de arquivo rejeitado nas fontes
sources-missing-deck = A apresentação está ausente das fontes
sources-too-large = As fontes deste arquivo são grandes demais para serem abertas
sources-too-deep = Esta apresentação aninha elementos com profundidade excessiva

issue-empty-slide-id = identificador de slide vazio
issue-duplicate-slide-id = identificador de slide duplicado
issue-duplicate-element-id = identificador de elemento duplicado
issue-empty-element-id = identificador vazio
issue-cell-outside-grid = célula fora da grade
issue-empty-frame = quadro de tamanho zero
issue-free-without-frame = elemento livre sem quadro
issue-unknown-widget = widget desconhecido: { $kind }
issue-wrong-category = { $kind } não pode ser usado aqui
issue-unknown-param = parâmetro desconhecido: { $param }
issue-param-type = valor inválido para { $param }
issue-choice-out-of-range = valor não permitido para { $param }
issue-slot-rejects = { $kind } não é aceito aqui
issue-placed-in-flow = elemento posicionado com um quadro em uma lista diagramada
issue-unknown-bind = { $param } está vinculado a uma configuração do deck desconhecida
issue-slot-too-deep = elementos aninhados profundamente demais
issue-invalid-style = valor de estilo inválido: { $field } ({ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    })
issue-missing-asset = recurso ausente: { $asset }
issue-pack-version-mismatch = versão de pacote alterada descartada: { $version }
issue-pack-version-missing = versão de pacote ausente das fontes: { $version }
issue-pack-pin-mismatch = O pacote de widgets incorporado { $pack } não corresponde ao que a apresentação espera.
issue-pack-requirement-missing = O pacote de widgets { $pack } precisa de { $required }, que esta apresentação não tem.
issue-theme-version-mismatch = versão de tema alterada descartada: { $version }
issue-theme-version-missing = versão de tema ausente das fontes: { $version }
issue-unsupported-link = link não suportado: { $link }
issue-highlight-zero = as cores de destaque são numeradas a partir de 1
issue-unreadable-element = elemento ilegível, deixado de lado
issue-slot-repaired = elemento sem slot conhecido, colocado em { $slot }
issue-unreadable-item = item de lista ilegível, deixado de lado
issue-unreadable-row = linha de tabela ilegível, deixada de lado
issue-data-truncated = dados grandes demais, truncados na leitura
issue-unreadable-slide = slide ilegível, deixado de lado
issue-unreadable-field = campo { $field } do deck ilegível, redefinido
op-invalid-chart = Gráfico { $subject }: { $reason }
op-invalid-table = Tabela { $subject }: { $reason }

replica-corrupted = O histórico da apresentação neste arquivo está danificado
replica-too-large = O histórico da apresentação neste arquivo é grande demais para ser aberto
replica-write = Não foi possível atualizar o histórico da apresentação
replica-unrelated = Estas duas apresentações não têm uma origem comum
replica-unsupported-version = Esta apresentação usa um formato de deck que esta versão do Oku Slides não consegue ler
replica-too-deep = Não é possível salvar esta apresentação: elementos excluídos estão aninhados profundamente demais no histórico. Desfaça suas últimas movimentações e salve novamente.

widget-unreadable = Não foi possível ler as declarações de widgets
widget-fault-empty-name = Widget { $widget }: um widget ou parâmetro não tem nome
widget-fault-duplicate-widget = O widget { $widget } está declarado duas vezes
widget-fault-duplicate-param = O widget { $widget } declara o mesmo parâmetro duas vezes
widget-fault-empty-choice = Widget { $widget }: uma escolha não tem valores
widget-fault-default-not-in-choice = Widget { $widget }: o valor padrão de uma escolha não está entre seus valores
widget-fault-default-wrong-type = Widget { $widget }: um valor padrão não corresponde ao tipo do seu parâmetro
widget-fault-bind-not-plain = Widget { $widget }: apenas parâmetros de texto simples podem ser vinculados à apresentação
widget-fault-unknown-slot-kind = Widget { $widget }: um slot aceita um widget de elemento desconhecido
widget-fault-empty-list = Widget { $widget }: uma lista precisa de pelo menos uma coluna
widget-fault-column-type = Widget { $widget }: uma coluna de lista pode ser plain, rich, number, flag, choice ou asset (ou uma p.list aninhada, um nível); para conteúdo livre por item, use p.slot com um widget filho
widget-fault-reserved-name = Widget { $widget }: nome reservado: id, kind ou um nome que contém /
widget-fault-list-bounds = Widget { $widget }: o máximo de uma lista está abaixo do seu mínimo
widget-fault-list-default-too-large = Widget { $widget }: o valor padrão de uma lista excede os limites de tamanho das listas
widget-fault-list-depth = Widget { $widget }: uma lista aninhada não pode conter uma coluna de lista; as listas só se aninham em um nível
widget-fault-list-default-below-min = Widget { $widget }: o valor padrão de uma lista aninhada tem menos itens que o seu mínimo
widget-fault-slide-sizing = Widget { $widget } é um slide: só os widgets de elemento declaram grow ou typography
issue-empty-item-id = a lista { $param } tem um item sem identificador
issue-duplicate-item-id = a lista { $param } tem um identificador de item duplicado
issue-invalid-item-id = a lista { $param } tem um identificador de item com uma barra
issue-list-bounds = a lista { $param } tem itens de menos ou de mais
