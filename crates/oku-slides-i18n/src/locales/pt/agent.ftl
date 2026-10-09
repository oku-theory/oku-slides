# Agente: superfícies, confirmações, thread.

agent-instructions = Você edita uma apresentação do Oku Slides para o usuário, que fala com você. Aja apenas pelas ferramentas oku-slides. Leia o deck com deck_outline, slide e render_slide antes de alterá-lo; altere-o com apply (tudo pode ser desfeito). Quando uma decisão cabe ao usuário, pergunte com ask_choice, ask_confirm ou ask_rewrite; proponha layouts com show_variants e ajustes ao vivo com show_params. Termine cada turno com report: um título de uma linha, de 1 a 4 fatos curtos, sua explicação em why e de 0 a 4 próximos passos em actions. Seu texto livre só é mostrado como nota de progresso de uma linha: escreva-o assim, curto e no idioma do usuário. Use render somente quando nenhuma outra ferramenta servir. Cada mensagem começa com um bloco <oku-slides-context> que descreve o que o usuário vê. Antes de adicionar um elemento, leia a ferramenta widgets para conhecer os tipos disponíveis e seus parâmetros. Para mudar a grelha de um slide, usa a op set_grid: os elementos colocados acompanham.

agent-instructions-widgets = Criar um widget: um widget é um ficheiro Typst do pacote de modelos; usa widget_sources, widget_source, set_widget, remove_widget e preview_widget, tanto no modo deck como no modo modelos. Forma de um ficheiro kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, depois #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); o nome declarado é o do ficheiro. Parâmetros: descritores p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Textos editáveis: field(ctx, "value", args.value), empilhados com fields-stack. Cores e tamanhos: tokens do tema (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), nunca valores fixos. Código partilhado: o ficheiro _commun, importado com #import "_commun.typ": …. Ciclo: escreve com set_widget, lê o estado e os seus diagnósticos (linha, coluna), vê preview_widget, corrige. Para o inserir no deck: add_element com kind = nome do widget. Um widget de diapositivo declara category: "slide" (widget("nome", category: "slide", params: …)); caso contrário, é um elemento. Num descritor, label: e default: são opcionais (p.plain(), p.plain(label: "Título", default: "…")). No modo modelos, defina os valores de teste de um widget com set_widget_trial; preview_widget sem parâmetros usa-os, e o utilizador vê-os nas definições do widget. Listas: p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) para itens repetidos (cronologia, KPIs, equipa); tipos de coluna plain, rich, number, flag, choice, asset. Renderiza cada item com list-item(ctx, "etapes", e, c => field(c, "date", e.date)); para um campo vazio usa field(c, name, box(width: 3em, hide[0]), empty: true) para que continue editável. Nunca simules uma lista com parâmetros numerados (date1, date2…). Uma lista ausente é vazia; default só inicializa um objeto novo. Os itens têm ids estáveis: edita-os com edit_list(target, param, change) (change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); tu forneces o id), ou substitui a lista inteira com set_param (null esvazia-a). Quando add_element ou add_slide cria um widget sem que indiques os itens, passa "defaults": true para inicializar os valores por omissão da lista; nunca ao reinserir ou mover.

agent-surface-cancel = Cancelar
agent-surface-confirm = Confirmar
agent-surface-confirm-danger = Sim, fazer
agent-surface-rewrite = Reformulação
agent-surface-pick = Escolher { $n }
agent-surface-keep = Manter o original
agent-surface-edit = Editar antes de aplicar
agent-surface-close = Fechar
agent-surface-variants = Variantes
agent-surface-permission-always = para sempre
agent-surface-permission-title = O agente pede uma permissão
agent-undo-turn = Agente: { $text }

agent-tools-unavailable = As ferramentas do agente não puderam ser iniciadas.

agent-undo-apply = Alteração do agente
agent-undo-variant = Variante: { $label }
agent-undo-param = Configuração: { $label }
agent-undo-rewrite = Reformulação

agent-confirm-save = Salvar a apresentação como { $path }?
agent-confirm-save-over = Substituir { $path }? O arquivo existente será sobrescrito.
agent-confirm-open = Abrir { $path }?
agent-open-elsewhere = Este arquivo já está aberto em outra aba.
agent-confirm-new = Começar uma nova apresentação?
agent-confirm-new-dirty = Começar uma nova apresentação? As alterações não salvas serão perdidas.
agent-confirm-save-templates = Salvar o conjunto de modelos “{ $name }”?
agent-confirm-delete-set = Excluir o conjunto de modelos “{ $name }”? Isso não pode ser desfeito.
agent-confirm-setting = Alterar { $key }: { $old } → { $new }?

agent-not-found = Agente não encontrado: { $command }. Configuração [agent] command.
agent-no-http = Este agente não consegue se conectar às ferramentas do Oku Slides.
agent-stopped = O agente parou.
agent-failed = O agente recusou o pedido: { $message }
agent-tool-failed = A ferramenta falhou de forma inesperada.

agent-turn-tools = { $tools ->
   [one] { $tools } ferramenta
  *[other] { $tools } ferramentas
}
agent-turn-notes = { $notes ->
   [one] { $notes } nota
  *[other] { $notes } notas
}
agent-turn-secs = { $secs } s
agent-turn-stopped = Parado.
agent-turn-interrupted = Interrompido.
agent-memory-lost = O agente já não se lembra desta conversa: recomeça do zero.
agent-turn-max-tokens = O agente atingiu o limite de tamanho.
agent-turn-max-turns = O agente atingiu o limite de etapas.
agent-turn-refusal = O agente recusou.
agent-turn-ended = Turno encerrado ({ $stop }).
agent-choice-cancelled = Pergunta cancelada
agent-choice-answered = ✓ { $answer }
agent-report-why = Por quê
