# Interface do app (React). Carregada por ui/src/i18n.ts pela solicitação “messages”.
ui-loading = Carregando…

## Documento

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · pacote de layouts
   *[other] { $name } · pacote de widgets
}
ui-title-rename = Renomear a apresentação
ui-title-field = Título da apresentação
ui-status-unsaved = alterações não salvas
ui-status-saved = salvo
ui-status-never-saved = ainda não salvo
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count } layout
       *[other] { $count } layouts
    }
   *[other] { $count ->
        [one] { $count } widget
       *[other] { $count } widgets
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } slide
   *[other] { $count } slides
} · { $status }
ui-confirm-quit-pack = Sair sem salvar o pacote alterado?
ui-confirm-quit-theme = Sair sem salvar o tema alterado?
ui-confirm-quit-read-only-theme = Não é possível salvar { $theme } aqui. Fechar esta aba e perder suas alterações?
ui-confirm-update-opener = Atualizar “{ $deck }” para esta versão do pacote “{ $pack }”?
ui-close-writes-pending = Esta aba ainda está salvando: feche-a de novo em um instante.
ui-confirm-close-writes-pending = Esta aba ainda está salvando. Fechar mesmo assim e perder o que não foi salvo?
ui-confirm-close-writes-pending-linked = Esta aba ainda está salvando, e estas abas de pacote fecham com ela sem salvar: { $tabs }. Fechar mesmo assim e perder o que não foi salvo?
ui-confirm-quit-read-only-pack = Não é possível salvar { $pack } aqui. Fechar esta aba e perder suas alterações? Para mantê-las, peça ao agente que o salve como um novo pacote.

## Notificações

ui-toast-copied = Cópia salva
ui-toast-field-gone = O campo que você estava editando não existe mais: o texto digitado foi perdido.
ui-toast-edit-replaced = Este texto mudou enquanto você o editava: sua versão o substituiu. Desfazer restaura a outra.

## Barra de ferramentas

ui-toolbar-undo = Desfazer
ui-toolbar-undo-label = Desfazer: { $label }
ui-toolbar-redo = Refazer
ui-toolbar-redo-label = Refazer: { $label }
ui-divider-pile = Largura da pilha de slides
ui-divider-notes = Largura do fio do agente
ui-divider-code = Tamanho do código
ui-toolbar-theme = Tema
ui-toolbar-finish = Concluir
ui-pack-deck-only = Salvo apenas nesta apresentação
ui-toolbar-open = Abrir
ui-menu = Menu
ui-toolbar-add-label = Adicionar ao slide
ui-toolbar-packs = Pacotes da apresentação
ui-toolbar-packs-update = Pacotes da apresentação — atualizações disponíveis
ui-toolbar-thumbs-show = Mostrar miniaturas
ui-toolbar-thumbs-hide = Ocultar miniaturas
ui-toolbar-remove-slide = Excluir o slide
ui-toolbar-remove-hint = Slide { $n } · desfaça com ⌘Z

## Rótulos de desfazer

ui-undo-add-widget = Adicionar “{ $name }”
ui-undo-new-slide = Novo slide: { $name }
ui-undo-move-slide = Mover o slide
ui-undo-remove-slide = Excluir o slide
ui-undo-remove = Excluir
ui-undo-paste = Colar
ui-undo-place-on-grid = Posicionar na grade
ui-undo-move-on-grid = Mover na grade
ui-undo-resize-on-grid = Redimensionar na grade
ui-undo-change-anchor = Alterar a ancoragem
ui-undo-move = Mover
ui-undo-resize = Redimensionar
ui-undo-edit-slide = Editar o slide
ui-undo-rename-pack = Renomear o pacote
ui-undo-rename-deck = Renomear a apresentação

## Pilha de slides

ui-pile-slides = Slides
ui-pile-layouts = Layouts
ui-pile-add-slide = Novo slide
ui-pile-drag-hint = Arraste para mover · ⌥↑ ⌥↓
ui-slide-alt = Slide { $n }

## Palco

ui-stage-rendering = Renderizando…
ui-stage-widget-pack = Abra um widget da pilha ou crie um.
ui-stage-elements = Elementos do slide
ui-stage-fields = Campos do slide
ui-stage-edit-text = Editar o texto
ui-stage-hint-plain = Enter ou Tab para o próximo · Esc para cancelar
ui-stage-hint-rich = Tab para o próximo · ⌘ Enter para confirmar · “- ” para um marcador · ⌘B negrito, ⌘I itálico, ⌘K link
ui-field = { $kind ->
    [title] Título
    [subtitle] Subtítulo
    [kicker] Chamada
    [text] Texto
    [body] Corpo
    [author] Autor
    [date] Data
    [organization] Organização
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Texto
    [card] Cartão
    [image] Imagem
    [shape] Forma
    [chart] Gráfico
    [table] Tabela
    [stack] Pilha
   *[other] { $kind }
}
ui-grid-cell = col. { $cols } · linha { $rows }
ui-ghost-zone = Zona

## Notas do slide

ui-notes = Ferramentas do slide
ui-stepper-less = { $label }: um a menos
ui-stepper-more = { $label }: um a mais
ui-list-item = item
ui-list-add = Adicionar { $item }
ui-list-remove = Remover { $item }
ui-list-move = Mover { $item }
ui-list-full = A lista está cheia ({ $max })
ui-undo-list-move = Mover { $item }
ui-agent-color-other = Outra cor
ui-agent-thread = Agente
ui-agent-new-conversation = Nova conversa
ui-agent-undo-turn = Desfazer este turno
ui-agent-detach = Abrir o agente em uma janela separada
ui-agent-attach = Trazer o agente de volta à janela principal
ui-agent-window-empty = Nenhum agente para esta aba
ui-agent-kept-in-draft = Nada onde escrever: texto mantido no rascunho
ui-draft = Rascunho
ui-draft-send = Enviar ao agente
ui-draft-send-title = Enviar ao agente (⌘↩)
ui-draft-discard = Descartar
ui-pasted-image = { $type } { $width }×{ $height }
ui-draft-images = Imagens coladas
ui-draft-remove-image = Remover esta imagem
ui-draft-send-needs-text = Diga o que fazer com ela
ui-agent-state-idle = Agente
ui-agent-state-starting = O agente está iniciando…
ui-agent-state-working = O agente está trabalhando…
ui-agent-dock = Em andamento
ui-agent-dock-settings = Configurações
ui-agent-jump = ↓ Turno atual
ui-agent-tool-input = Argumentos
ui-agent-tool-output = Resultado
ui-agent-sheet = Pergunta do agente
ui-agent-sheet-keys = 1–4 escolher · Enter confirmar · E editar · Esc cancelar
ui-agent-variants = Variantes
ui-agent-drawer-keys = Clique ou 1–4 · Esc fecha
ui-agent-confirm = Confirmação do agente

## Ditado

ui-dictation = Ditado
ui-dictation-start = Ditar
ui-dictation-start-title = Ditar (⌘D)
ui-dictation-type = Escrever ao agente
ui-dictation-type-title = Escrever ao agente (⌘K)
ui-dictation-agent-stop = Parar o agente (⌘⇧D ou Esc)
ui-dictation-agent-working = O agente está trabalhando…
ui-dictation-agent-working-hint = ⌘⇧D ou Esc o interrompe
ui-dictation-finish = Concluir o ditado
ui-dictation-finish-title = Concluir (⌘D)
ui-dictation-cancel = Cancelar
ui-dictation-preparing = Preparando o ditado…
ui-dictation-heard-nothing = Nada foi ouvido: fale um pouco mais perto do microfone
ui-dictation-to-cursor = no cursor
ui-dictation-to-draft = no rascunho
ui-dictation-downloading = Baixando o modelo { $name }
ui-dictation-download-progress = { $percent }% · { $received } de { $total } · só uma vez
ui-dictation-listening = Ouvindo · { $time }
ui-dictation-transcribing = Transcrevendo a última frase…
ui-dictation-stop-in = Parando em { $duration }
ui-dictation-speak = Fale, o texto aparece aqui…
ui-dictation-auto-stop = para após { $duration } de silêncio
ui-dictation-to-download = { $size } para baixar
ui-dictation-settings = Configurações
ui-dictation-language-auto = Detecção automática
ui-dictation-model-unavailable = Idioma não suportado
ui-dictation-language-auto-resolved = Automático ({ $language })
ui-dictation-to-download-language = Idioma para baixar
ui-dictation-api-key-missing = { $var } não está definida
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Seletor de modelos

ui-picker-new-slide = Novo slide
ui-picker-set-mine = { $name } · meus
ui-picker-set-received = { $name } · recebidos
ui-picker-keys = ← → para navegar · Enter para adicionar · Esc para fechar
ui-picker-close = Fechar
ui-picker-list = Layouts
ui-widget-picker-filter = Filtrar por nome, rótulo ou descrição
ui-widget-picker-empty = Nenhum widget corresponde a “{ $query }”.
ui-widget-picker-list = Widgets
ui-widget-picker-keys = Setas para navegar · Enter para adicionar · Esc para fechar

## Gráfico

ui-chart-panel = Gráfico
ui-chart-kinds = Tipo de gráfico
ui-chart-kind = { $kind ->
    [line] Linhas
    [pie] Pizza
    [scatter] Dispersão
    [radar] Radar
    [waterfall] Cascata
   *[bar] Barras
}
ui-chart-no-data = Este gráfico não tem tabela de dados.
ui-chart-options = Opções do gráfico
ui-chart-stack = Empilhado
ui-chart-waterfall-hint = Marque Total para uma barra de total.
ui-chart-colors = Cores das séries
ui-chart-color-of = Cor de { $name }
ui-chart-color-auto = Automático
ui-chart-color-pick = Cor { $n }
ui-chart-column-name = Nome da coluna { $col }
ui-chart-remove-column = Excluir a coluna
ui-chart-cell = Linha { $row }, coluna { $col }
ui-chart-remove-row = Excluir a linha
ui-chart-add-row = Linha
ui-chart-add-column = Coluna
ui-chart-series = Série { $n }

## Tabela

ui-table-panel = Tabela
ui-table-options = Opções da tabela
ui-table-header-row = Linha de cabeçalho
ui-table-header-column = Primeira coluna como cabeçalho
ui-table-striped = Linhas alternadas
ui-table-alignment = Alinhamento das colunas
ui-table-align-of = Alinhamento de { $name }
ui-table-align = { $align ->
    [start] Início
    [center] Centro
    [end] Fim
   *[auto] Automático
}
ui-table-column = Coluna { $n }
ui-table-no-data = Esta tabela não tem dados.
ui-show-as = Mostrar como
ui-show-as-table = Mostrar como tabela
ui-show-as-chart = Mostrar como gráfico

## Arquivos

ui-file-open-prompt = Caminho do arquivo a abrir (.oku-slides.pdf, .json ou .pptx)
ui-file-filter-presentations = Apresentações
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = Salvar como (caminho completo)

## Abas

ui-tabs = Apresentações abertas
ui-tab-new = Nova aba
ui-tab-close = Fechar aba
ui-tab-unsaved = Alterações não salvas
ui-tab-pack = Pacote
ui-tab-theme = Tema
ui-tab-move-new-window = Mover para uma nova janela
ui-tab-move-to = Mover para a janela { $n }
ui-tab-close-others = Fechar as outras abas
ui-close-cancel = Cancelar
ui-disk-replaced-question = { $file } foi substituído no disco por outra apresentação.
ui-disk-replaced-overwrite = Substituir
ui-disk-replaced-save-as = Salvar como…
ui-same-deck-question = { $file } é uma versão da apresentação aberta na aba “{ $title }”.
ui-same-deck-merge = Mesclar nessa aba
ui-same-deck-apart = Abrir separadamente
ui-history = Histórico
ui-history-version = Versão de { $date }
ui-history-restore = Restaurar
ui-history-back = Voltar
ui-history-anonymous = Anônimo
ui-history-slider = Versões da apresentação
ui-history-removed-since = excluído desde então
ui-history-changed-since = alterado desde então
ui-history-added-since = slides adicionados desde então: { $count }
ui-help = Ajuda
ui-help-caption = Slide { $index } de { $count } · somente leitura
ui-slide-previous = Slide anterior
ui-slide-next = Próximo slide
ui-present = Apresentação
ui-present-slide = Slide { $index } de { $count }
ui-present-screen = Apresentar em tela cheia
ui-help-close = Fechar a ajuda
ui-help-search = Pesquisar na ajuda
ui-help-search-placeholder = Pesquisar…
ui-help-search-empty = Sem resultados
ui-help-search-count = { $count ->
    [one] { $count } resultado
   *[other] { $count } resultados
}
ui-widgets-files = Arquivos de widgets
ui-widgets-common = Comum
ui-widgets-new = Novo widget
ui-widgets-name = Nome do widget (minúsculas, algarismos e hifens)
ui-widgets-name-taken = Este nome já está em uso
ui-widgets-name-letter = O nome deve começar com uma letra
ui-widgets-valid = Válido
ui-widgets-errors = Erros
ui-widgets-empty = Este pacote ainda não tem widgets.
ui-widgets-preview = Pré-visualização do widget
ui-widgets-no-preview = Sem pré-visualização
ui-widgets-field-bound = Vinculado à apresentação: somente leitura aqui
ui-widgets-source = Código-fonte
ui-widgets-unsaved = Não salvo
ui-widgets-in-error = Widgets com erros
ui-widgets-none-in-error = Nenhum widget tem erros
ui-widgets-section = Widgets
ui-widgets-open = Abrir o widget { $name }
ui-widgets-delete-named = Excluir { $name }
ui-widgets-confirm-delete = Excluir o widget “{ $name }”?
ui-widgets-rename-hint = Clique duas vezes ou F2 para renomear
ui-pack-tabs = Abas do pacote
ui-pack-layouts-tab = Layouts
ui-theme-name = Nome do tema
ui-theme-unsaved = Alterações do tema não salvas
ui-widgets-close-tab = Fechar { $name }
ui-widgets-removed-by-agent = O agente excluiu o widget { $name }.

## Importar

ui-import-title = Importar uma apresentação do PowerPoint
ui-import-mode = Modo de importação
ui-import-hifi = Cópia fiel
ui-import-hifi-note = Mantém a aparência do original, com um tema e layouts feitos para esta apresentação.
ui-import-retrofit = Usar meus layouts
ui-import-retrofit-note = Refaz os slides com pacotes de layouts e um tema da biblioteca.
ui-import-layout-packs = Pacotes de layouts
ui-import-theme-default = Tema padrão
ui-import-no-packs = Nenhum pacote de layouts na biblioteca.
ui-import-tune = Afinar com o agente
ui-import-tune-note = O agente compara cada slide com o original e corrige as diferenças.
ui-import-tune-note-retrofit = O agente compara cada slide com o original e cria widgets para as formas que seus layouts não mostram.
ui-import-tune-unavailable = Nenhum agente disponível: { $reason }
ui-import-submit = Importar
ui-import-busy = Importando…
ui-update-available = O Oku Slides { $version } está disponível.
ui-update-install = Instalar e reiniciar
ui-update-see-release = Ver a versão
ui-update-later = Mais tarde
ui-update-installing = Baixando a atualização… { $percent }%
ui-update-downloading = Baixando a atualização…
ui-update-up-to-date = O Oku Slides está atualizado.
ui-update-check = Verificar atualizações…
ui-about = Sobre o Oku Slides
ui-about-version = Versão { $version }
ui-about-changelog = Novidades
ui-about-breaking = incompatível
ui-about-close = Fechar

ui-shortcut-theme = tema, temas, theme, themes
