# Interface do app (React). Carregada por ui/src/i18n.ts pela solicitação “messages”.
ui-loading = Carregando…

## Documento

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · modelos
ui-title-rename = Renomear a apresentação
ui-title-field = Título da apresentação
ui-status-unsaved = alterações não salvas
ui-status-saved = salvo
ui-status-never-saved = ainda não salvo
ui-caption-templates = { $count ->
    [one] { $count } modelo
   *[other] { $count } modelos
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } slide
   *[other] { $count } slides
} · { $status }
ui-template-default-name = Modelo
ui-template-variant-name = { $name } (variante)
ui-confirm-quit-templates = Sair sem salvar os modelos alterados?

## Notificações

ui-toast-templates-saved = Modelos salvos: { $name }
ui-toast-moved = Apresentação movida
ui-toast-copied = Cópia guardada
ui-toast-select-zone = Selecione uma área para posicionar: “{ $text }”
ui-toast-field-gone = O campo que você estava editando não existe mais: o texto digitado foi perdido.

## Barra de ferramentas

ui-toolbar-undo = Desfazer
ui-toolbar-undo-label = Desfazer: { $label }
ui-toolbar-redo = Refazer
ui-toolbar-redo-label = Refazer: { $label }
ui-toolbar-grid-hide = Ocultar a grade
ui-toolbar-grid-show = Mostrar a grade
ui-toolbar-ghosts-hide = Ocultar os fantasmas
ui-toolbar-ghosts-show = Mostrar os fantasmas
ui-divider-pile = Largura da pilha de slides
ui-divider-notes = Largura do fio do agente
ui-divider-code = Tamanho do código
ui-toolbar-theme = Tema
ui-toolbar-theme-default = Padrão
ui-toolbar-save-copy = Salvar uma cópia
ui-toolbar-finish = Concluir
ui-toolbar-templates = Modelos
ui-toolbar-open = Abrir
ui-merge = Mesclar com um PDF…
ui-menu = Menu
ui-menu-move = Mover…
ui-language = Idioma
ui-language-system = Idioma do sistema
ui-scheme = Aparência
ui-scheme-system = Sistema
ui-scheme-light = Claro
ui-scheme-dark = Escuro
ui-toolbar-add-label = Adicionar ao slide
ui-add-kind = { $kind ->
    [card] Cartão
    [shape] Forma
    [chart] Gráfico
   *[text] Texto
}
ui-add-widget = Widget do pacote
ui-toolbar-remove-template = Excluir o modelo
ui-toolbar-remove-slide = Excluir o slide
ui-toolbar-remove-hint = Slide { $n } · desfaça com ⌘Z

## Rótulos de desfazer

ui-undo-add = { $kind ->
    [card] Adicionar um cartão
    [shape] Adicionar uma forma
    [chart] Adicionar um gráfico
   *[text] Adicionar um texto
}
ui-undo-add-widget = Adicionar “{ $name }”
ui-undo-new-template = Novo modelo: { $name }
ui-undo-new-slide = Novo slide: { $name }
ui-undo-templates = Modelos: { $name }
ui-undo-move-template = Mover o modelo
ui-undo-move-slide = Mover o slide
ui-undo-remove-slide = Excluir o slide
ui-undo-remove = Excluir
ui-undo-cut = Recortar
ui-undo-paste = Colar
ui-undo-place-on-grid = Posicionar na grade
ui-undo-move-on-grid = Mover na grade
ui-undo-resize-on-grid = Redimensionar na grade
ui-undo-change-anchor = Alterar a ancoragem
ui-undo-move = Mover
ui-undo-resize = Redimensionar
ui-undo-edit-slide = Editar o slide
ui-undo-dictation = Ditado
ui-undo-rename-set = Renomear o conjunto
ui-undo-rename-deck = Renomear a apresentação
ui-undo-theme = Alterar o tema
ui-undo-chart-kind = Alterar o tipo de gráfico
ui-undo-chart-data = Editar os dados
ui-undo-chart-color = Colorir uma série
ui-undo-chart-stack = { $stacked ->
    [yes] Empilhar as séries
   *[no] Separar as séries
}

## Novos elementos

ui-new-text = Novo texto
ui-new-card-title = Título do cartão
ui-new-card-detail = Detalhe

## Pilha de slides

ui-pile-slides = Slides
ui-pile-templates = Modelos
ui-pile-add-slide = + Novo slide
ui-pile-add-template = + Novo modelo
ui-pile-drag-hint = Arraste para mover · ⌥↑ ⌥↓
ui-slide-alt = Slide { $n }

## Palco

ui-stage-rendering = Renderizando…
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
    [group] Grupo
   *[other] { $kind }
}
ui-grid-cell = col. { $cols } · linha { $rows }
ui-ghost-zone = Zona

## Notas do slide

ui-notes = Ferramentas do slide
ui-stepper-less = { $label }: um a menos
ui-stepper-more = { $label }: um a mais
ui-agent-color-other = Outra cor
ui-agent-thread = Agente
ui-agent-new-conversation = Nova conversa
ui-draft = Rascunho
ui-draft-insert = Inserir
ui-draft-insert-title = Inserir como está (⌘⇧↩)
ui-draft-send = Enviar ao agente
ui-draft-send-title = Enviar ao agente (⌘↩)
ui-draft-discard = Descartar
ui-agent-state-idle = Agente
ui-agent-state-starting = O agente está iniciando…
ui-agent-state-working = O agente está trabalhando…
ui-agent-dock = Em andamento
ui-agent-dock-settings = Configurações
ui-agent-jump = ↓ Turno atual
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
ui-dictation-to-field = substituir o { $field }
ui-dictation-to-block = depois do bloco selecionado
ui-dictation-new-block = novo bloco de texto
ui-dictation-select-zone = selecione uma área
ui-dictation-downloading = Baixando o modelo { $name }
ui-dictation-download-progress = { $percent }% · { $received } de { $total } · só uma vez
ui-dictation-listening = Ouvindo · { $time }
ui-dictation-transcribing = Transcrevendo a última frase…
ui-dictation-stop-in = Parando em { $duration }
ui-dictation-speak = Fale, o texto aparece aqui…
ui-dictation-auto-stop = para após { $duration } de silêncio
ui-dictation-to-download = { $size } para baixar
ui-dictation-settings = Configurações do ditado
ui-dictation-model-title = Modelo de reconhecimento
ui-dictation-model-intro = Whisper, neste computador (CTranslate2). Todos os modelos entendem vários idiomas.
ui-dictation-note-turbo = Recomendado, rápido e preciso
ui-dictation-note-large = O mais preciso, mais lento
ui-dictation-note-base = Leve, para máquinas modestas
ui-dictation-installed = instalado
ui-dictation-not-installed = para baixar
ui-dictation-language = Idioma falado
ui-dictation-language-auto = Detecção automática
ui-dictation-note-apple = O mais rápido, integrado ao sistema
ui-dictation-model-unavailable = Idioma não suportado
ui-dictation-model-intro-apple = Apple, integrado ao sistema, ou Whisper, neste computador (CTranslate2). Todos entendem vários idiomas.
ui-dictation-language-auto-resolved = Automático ({ $language })
ui-dictation-to-download-language = Idioma para baixar
ui-dictation-auto-stop-label = Parar sozinho após um silêncio de
ui-dictation-silence = Duração do silêncio
ui-dictation-phrases-note = Apenas os trechos falados são transcritos, frase por frase, enquanto você fala.
ui-dictation-download-now = Baixar agora ({ $size })
ui-dictation-download-language = Baixar o idioma
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } s
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Seletor de modelos

ui-picker-new-slide = Novo slide
ui-picker-new-template = Novo modelo, baseado em…
ui-picker-set = Modelos
ui-picker-set-mine = { $name } · meus
ui-picker-keys = ← → para navegar · Enter para adicionar · Esc para fechar
ui-picker-close = Fechar
ui-picker-previous = Modelo anterior
ui-picker-next = Próximo modelo
ui-picker-list = Modelos de slide

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

## Arquivos

ui-file-open-prompt = Caminho do arquivo a abrir (.oku-slides.pdf ou .json)
ui-file-save-prompt = Salvar como (caminho completo)

## Abas

ui-tabs = Apresentações abertas
ui-tab-new = Nova aba
ui-tab-close = Fechar aba
ui-tab-unsaved = Alterações não salvas
ui-tab-templates = Modelos
ui-tab-move-new-window = Mover para uma nova janela
ui-tab-move-to = Mover para a janela { $n }
ui-tab-close-others = Fechar as outras abas
ui-close-cancel = Cancelar
ui-disk-replaced-question = { $file } foi substituído no disco por outra apresentação.
ui-disk-replaced-overwrite = Substituir
ui-disk-replaced-save-as = Salvar como…
ui-same-deck-question = { $file } é uma versão da apresentação aberta na guia “{ $title }”.
ui-same-deck-merge = Mesclar nessa guia
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
ui-help-caption = Diapositivo { $index } de { $count } · só de leitura
ui-help-previous = Diapositivo anterior
ui-help-next = Diapositivo seguinte
ui-help-close = Fechar a ajuda
ui-help-search = Pesquisar na ajuda
ui-help-search-placeholder = Pesquisar…
ui-help-search-empty = Sem resultados
ui-help-search-count = { $count ->
    [one] 1 resultado
   *[other] { $count } resultados
}
ui-pack-notice-close = Fechar
ui-widgets-files = Arquivos de widgets
ui-widgets-common = Comum
ui-widgets-new = Novo widget
ui-widgets-name = Nome do widget (minúsculas, algarismos e hifens)
ui-widgets-name-taken = Este nome já está em uso
ui-widgets-name-letter = O nome deve começar por uma letra
ui-widgets-valid = Válido
ui-widgets-errors = Erros
ui-widgets-empty = Este pacote ainda não tem widgets.
ui-widgets-preview = Pré-visualização do widget
ui-widgets-no-preview = Sem pré-visualização
ui-widgets-field-bound = Ligado à apresentação: só leitura aqui
ui-widgets-source = Código-fonte
ui-widgets-code-right = Código à direita
ui-widgets-board = Prancha
ui-widgets-unsaved = Não salvo
ui-widgets-in-error = Widgets com erros
ui-widgets-none-in-error = Nenhum widget tem erros
ui-widgets-section = Widgets
ui-widgets-edit = Editar widgets
ui-widgets-open = Abrir o widget { $name }
ui-widgets-delete-named = Excluir { $name }
ui-widgets-confirm-delete = Excluir o widget “{ $name }”?
ui-widgets-rename-hint = Clique duas vezes ou F2 para renomear
ui-widgets-tabs = Separadores de modelos
ui-widgets-templates-tab = Modelos
ui-widgets-close-tab = Fechar { $name }
ui-widgets-removed-by-agent = O agente excluiu o widget { $name }.
ui-examples = Casos de exemplo
ui-examples-add = Caso
ui-examples-add-hint = Adicionar um caso, cópia do atual
ui-examples-new-name = Caso { $n }
ui-examples-name = Nome do caso
ui-examples-rename-hint = Clique duplo para renomear
ui-examples-delete-named = Excluir o caso { $name }
ui-examples-confirm-delete = Excluir o caso “{ $name }”?
ui-examples-delete = Excluir
ui-examples-cancel = Cancelar
ui-board = Prancha de casos
ui-board-summary = { $cases ->
    [one] { $cases } caso
   *[other] { $cases } casos
} · { $alerts ->
    [one] { $alerts } alerta
   *[other] { $alerts } alertas
}
ui-board-scheme = Esquema
ui-board-light = Claro
ui-board-dark = Escuro
ui-board-ratio = Formato
ui-board-ok = OK
ui-board-empty = Campo vazio
ui-board-overflow = Transborda
ui-board-new = Novo caso
ui-board-open = Abrir o caso “{ $name }” na cena
