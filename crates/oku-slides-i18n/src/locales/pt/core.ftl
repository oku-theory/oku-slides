op-slide-not-found = Slide não encontrado: { $subject }
op-element-not-found = Elemento não encontrado: { $subject }
op-duplicate-id = Identificador já em uso: { $subject }
op-index-out-of-range = Posição fora do intervalo
op-invalid-value = Valor inválido para { $subject }
op-invalid-value-because = Valor inválido para { $subject }: { $reason }
op-value-no-value = o parâmetro não tem valor
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
knowledge-path-empty = o caminho está vazio
knowledge-path-not-absolute = o caminho deve começar com /
knowledge-path-bad-segment = um segmento está vazio, termina com ponto ou usa um caractere diferente de a-z, 0-9, ponto, sublinhado e hífen
knowledge-path-dot-segment = um segmento não pode começar com ponto
knowledge-path-reserved = um segmento é um nome reservado do sistema
knowledge-path-too-deep = o caminho tem segmentos demais
knowledge-path-too-long = o caminho é longo demais
knowledge-path-not-markdown = o nome do arquivo deve terminar em .md
knowledge-frontmatter-missing = o arquivo deve começar com um bloco de cabeçalho delimitado por ---
knowledge-frontmatter-unterminated = o bloco de cabeçalho não tem o --- de fechamento
knowledge-frontmatter-too-large = o bloco de cabeçalho é grande demais
knowledge-frontmatter-not-yaml = o cabeçalho não é YAML válido
knowledge-frontmatter-not-mapping = o cabeçalho deve ser um mapa com chaves de texto
knowledge-frontmatter-duplicate-key = a chave "{ $key }" aparece duas vezes no cabeçalho
knowledge-frontmatter-alias = o cabeçalho não pode usar âncoras nem aliases de YAML
knowledge-frontmatter-tag = o cabeçalho não pode usar tags de YAML
knowledge-frontmatter-too-deep = o cabeçalho tem aninhamento profundo demais
knowledge-frontmatter-after-document-end = o cabeçalho tem texto depois do fim de documento YAML ...
knowledge-frontmatter-type-missing = o cabeçalho precisa de um tipo não vazio
knowledge-frontmatter-bad-timestamp = o valor "{ $key }" do cabeçalho deve ser uma data e hora RFC 3339 com deslocamento de fuso
knowledge-frontmatter-not-allowed = a chave "{ $key }" não é permitida no cabeçalho deste arquivo
knowledge-reason-not-found = nenhum arquivo neste caminho
knowledge-reason-exists = já existe um arquivo neste caminho
knowledge-reason-id-missing = um novo arquivo precisa de um identificador
knowledge-reason-id-invalid = o identificador de um arquivo tem 32 dígitos hexadecimais em minúsculas
knowledge-reason-id-taken = outro arquivo já tem este identificador
knowledge-reason-id-mismatch = o arquivo deste caminho tem outro identificador
knowledge-reason-edit-empty-old = o texto a substituir não pode estar vazio
knowledge-reason-edit-no-match = o texto a substituir não está no arquivo
knowledge-reason-edit-ambiguous = o texto a substituir aparece mais de uma vez no arquivo
knowledge-reason-too-many-files = o pacote tem arquivos demais
knowledge-reason-file-too-large = o arquivo é grande demais
knowledge-reason-total-too-large = o pacote é grande demais
knowledge-reason-over-limit = a base de conhecimento passou dos seus limites: só excluir arquivos a traz de volta para baixo deles
op-knowledge = Arquivo de conhecimento { $path }: { $reason }
op-unknown-widget = Widget desconhecido: { $subject }
op-unknown-namespace = Esta apresentação não tem o pacote { $pack }.
op-pack-required = O pacote { $pack } é necessário para { $by }; remova esse primeiro.
op-pack-in-use = O pacote { $pack } ainda é usado por { $count ->
    [one] { $count } item
   *[other] { $count } itens
}.
op-pack-requirement-missing = O pacote { $pack } precisa de { $required }, que esta apresentação não tem.
op-base-required = Toda apresentação mantém o pacote de widgets base.
op-pack-kind-conflict = Esta apresentação já tem um pacote { $pack } do outro tipo (widgets ou layouts).
op-theme-cycle = Os temas estendem uns aos outros em um ciclo: { $chain }.
op-theme-closure = Os temas fixados nesta apresentação não são exatamente o tema dela e seus pais ({ $namespace }).
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
deck-inline-data = O elemento { $element } guarda os seus dados nas suas opções: este ficheiro de deck não está no formato atual
sources-not-sources = Isto não é um arquivo de fontes do Oku Slides
sources-unsupported-version = Estas fontes foram gravadas por uma versão mais recente do Oku Slides
sources-bad-name = Nome de arquivo rejeitado nas fontes
sources-missing-deck = A apresentação está ausente das fontes
sources-too-large = As fontes deste arquivo são grandes demais para serem abertas
sources-too-deep = Esta apresentação aninha elementos com profundidade excessiva
sources-base-refused = Um ficheiro de dados desta apresentação está danificado ou não pôde ser guardado

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
issue-pack-pin-mismatch = O pacote incorporado { $pack } não corresponde ao que a apresentação espera.
issue-pack-tokens-invalid = O pacote incorporado { $pack } tem um token de componente inválido: { $reason }
issue-pack-kind-mismatch = O pacote incorporado { $pack } não é do tipo que a apresentação espera (widgets ou layouts).
issue-pack-kind-conflict = O espaço de nomes { $pack } está fixado ao mesmo tempo como pacote de widgets e como pacote de layouts: só um pode ser usado.
issue-pack-requirement-missing = O pacote { $pack } precisa de { $required }, que esta apresentação não tem.
issue-theme-version-mismatch = versão de tema alterada descartada: { $version }
issue-theme-version-missing = versão de tema ausente das fontes: { $version }
issue-theme-pin-mismatch = O tema incorporado { $namespace } não corresponde ao que a apresentação espera.
issue-theme-closure = Os temas fixados nesta apresentação não são exatamente o tema dela e seus pais ({ $namespace }).
issue-usage-line-invalid = linha { $line } do registro de uso ilegível, descartada
issue-usage-line-forged = turno de uso { $turn } registrado duas vezes com conteúdo diferente, um descartado
issue-usage-too-large = registro de uso grande demais, descartado
issue-usage-unreadable = registro de uso ilegível, descartado
issue-usage-lines-dropped = { $count ->
    [one] mais { $count } linha do registro de uso descartada
   *[other] mais { $count } linhas do registro de uso descartadas
}
usage-range-bad-date = “{ $date }” não é uma data: use AAAA-MM-DD.
usage-range-reversed = A data inicial { $since } é posterior à data final { $until }.
pack-token-entry-invalid = Token de componente “{ $path }” inválido no manifesto do pacote.
pack-manifest-json = Não foi possível ler o manifesto do pacote: { $reason }
pack-manifest-order = A ordem dos itens do pack é inválida em “{ $name }”.
pack-manifest-data-count = O pacote contém { $count ->
    [one] { $count } dado
   *[other] { $count } dados
}, no máximo { $max }.
pack-manifest-data-base = Os dados “{ $data }” do pacote remetem a um arquivo externo; os dados de um pacote contêm as próprias células.
pack-manifest-data-cells = Os dados do pacote contêm { $count ->
    [one] { $count } célula
   *[other] { $count } células
}, no máximo { $max }.
issue-unsupported-link = link não suportado: { $link }
issue-highlight-zero = as cores de destaque são numeradas a partir de 1
issue-unreadable-element = elemento ilegível, deixado de lado
issue-slot-repaired = elemento sem slot conhecido, colocado em { $slot }
issue-unreadable-item = item de lista ilegível, deixado de lado
issue-unreadable-row = linha de tabela ilegível, deixada de lado
issue-data-truncated = dados «{ $data }» grandes demais, truncados na leitura e mantidos somente leitura
issue-data-name-taken = o nome de dados «{ $data }» já está em uso
issue-data-cell-type = dados «{ $data }»: a coluna «{ $column }» contém valores que não são do tipo { $type }
issue-data-too-many = dados demais na apresentação ({ $count }); não lidos: { $dropped }
issue-data-base-missing = dados «{ $data }»: falta o arquivo base, mantidos somente leitura
issue-data-edits-lost = dados «{ $data }»: alterações em uma tabela substituída foram perdidas na mesclagem
issue-data-inline-dropped = Elemento { $element }: uma versão anterior guardou os dados do seu gráfico ou tabela nas suas opções; foram removidos
issue-more = outros problemas não exibidos: { $count }
op-data-deck-full = Os dados da apresentação passariam do limite: no máximo { $max } { $what ->
    [row-ids] IDs de linha
    [overlay] células alteradas
    [text] bytes de texto
   *[data] dados
}
op-data-missing = Dados «{ $data }» não encontrados
op-data-name-taken = O nome de dados «{ $name }» já está em uso
op-data-column-in-use = A coluna «{ $column }» é mostrada por { $views }; vincule-os primeiro a outras colunas (bind_data com columns)
op-data-in-use = Estes dados são mostrados por { $views }{ $more ->
    [0] {""}
    [one] {" "}e mais { $more } vista
   *[other] {" "}e mais { $more } vistas
}; remova-as ou vincule-as primeiro a outros dados
op-data-column-missing = Os dados «{ $data }» não têm a coluna «{ $column }»
op-data-not-view = O parâmetro «{ $param }» não pode mostrar dados: só uma opção de gráfico ou de tabela pode
op-data-truncated = Os dados «{ $data }» foram truncados na leitura e são só de leitura; substitua toda a tabela para os editar
op-data-rebind = O elemento { $subject } já mostra outros dados; use bind_data para os mudar
op-data-internal = Um lote não pode definir a base nem a sobreposição de uma tabela
op-data-batch-too-large = O lote é grande demais: número de células { $count }, no máximo { $max }
op-data-inline = O elemento { $subject } traz os seus dados nas suas opções; adicione-os como dados do deck com add_data e ligue-os com bind_data
issue-unreadable-slide = slide ilegível, deixado de lado
issue-unreadable-field = campo { $field } do deck ilegível, redefinido
issue-knowledge-unreadable = entrada de conhecimento { $id } ilegível, deixada de lado
issue-knowledge-entries-dropped = entradas de conhecimento descartadas sem leitura: { $count } ({ $cause ->
        [files] arquivos demais para ler
        [file-bytes] arquivo grande demais para ler
        [total-bytes] conjunto grande demais para ler
       *[unreadable] mais entradas ilegíveis
    })
issue-knowledge-over-limit = coleção de conhecimento além de um limite: { $limit ->
        [knowledge-reason-too-many-files] { knowledge-reason-too-many-files }
        [knowledge-reason-file-too-large] { knowledge-reason-file-too-large }
       *[knowledge-reason-total-too-large] { knowledge-reason-total-too-large }
    } (arquivos afetados: { $count }, o primeiro { $id }); todos os arquivos são mantidos; só excluir arquivos traz o pacote de volta abaixo do limite (até lá, gravações e edições são recusadas)
issue-knowledge-path-invalid = arquivo de conhecimento { $id } movido para /recovered: { $reason ->
        [knowledge-path-empty] { knowledge-path-empty }
        [knowledge-path-not-absolute] { knowledge-path-not-absolute }
        [knowledge-path-bad-segment] { knowledge-path-bad-segment }
        [knowledge-path-dot-segment] { knowledge-path-dot-segment }
        [knowledge-path-reserved] { knowledge-path-reserved }
        [knowledge-path-too-deep] { knowledge-path-too-deep }
        [knowledge-path-too-long] { knowledge-path-too-long }
       *[knowledge-path-not-markdown] { knowledge-path-not-markdown }
    }
issue-knowledge-path-collision = vários arquivos de conhecimento estavam em { $path }: os outros foram renomeados
issue-knowledge-invalid = arquivo de conhecimento { $path } inválido: { $reason ->
        [knowledge-frontmatter-missing] { knowledge-frontmatter-missing }
        [knowledge-frontmatter-unterminated] { knowledge-frontmatter-unterminated }
        [knowledge-frontmatter-too-large] { knowledge-frontmatter-too-large }
        [knowledge-frontmatter-not-yaml] { knowledge-frontmatter-not-yaml }
        [knowledge-frontmatter-not-mapping] { knowledge-frontmatter-not-mapping }
        [knowledge-frontmatter-duplicate-key] { knowledge-frontmatter-duplicate-key }
        [knowledge-frontmatter-alias] { knowledge-frontmatter-alias }
        [knowledge-frontmatter-tag] { knowledge-frontmatter-tag }
        [knowledge-frontmatter-too-deep] { knowledge-frontmatter-too-deep }
        [knowledge-frontmatter-after-document-end] { knowledge-frontmatter-after-document-end }
        [knowledge-frontmatter-type-missing] { knowledge-frontmatter-type-missing }
        [knowledge-frontmatter-bad-timestamp] { knowledge-frontmatter-bad-timestamp }
       *[knowledge-frontmatter-not-allowed] { knowledge-frontmatter-not-allowed }
    }
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
widget-fault-pack-category = O widget { $widget } não corresponde ao seu pacote: os pacotes de layouts contêm widgets de slide e os pacotes de widgets, widgets de elemento
widget-fault-unqualified = O widget { $widget } não tem espaço de nomes de pacote
widget-fault-too-long = No widget { $widget }, um nome, rótulo, valor, descrição ou lista excede o limite
issue-empty-item-id = a lista { $param } tem um item sem identificador
issue-duplicate-item-id = a lista { $param } tem um identificador de item duplicado
issue-invalid-item-id = a lista { $param } tem um identificador de item com uma barra
issue-list-bounds = a lista { $param } tem itens de menos ou de mais
issue-view-truncated = { $element } mostra só parte dos seus dados ({ $what ->
    [rows] linhas
   *[columns] colunas
}: { $shown } de { $total })
issue-render-budget = { $element } não é desenhado: as vistas da apresentação leem mais dados do que uma renderização permite
issue-data-missing = { $element } nomeia os dados «{ $data }», que não estão na apresentação
issue-view-column-missing = { $element } lê a coluna «{ $column }», que os seus dados não têm
issue-view-unreadable = { $element } não é desenhado: as suas opções não podem ser lidas
op-replica-full = O histórico da apresentação está quase em { $max } MiB, o máximo para reabri-la: remova conteúdo antes de adicionar mais.
