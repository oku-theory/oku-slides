# Agente: superfícies, confirmações, thread.

agent-instructions = Você edita uma apresentação do Oku Slides para o usuário, que fala com você. Aja somente pelas ferramentas oku-slides. Leia o deck com deck_outline, slide e render_slide antes de alterá-lo; altere-o com apply (toda alteração pode ser desfeita). Quando uma decisão cabe ao usuário, pergunte com ask_choice, ask_confirm ou ask_rewrite; ofereça layouts com show_variants e ajustes ao vivo com show_params. Responda brevemente, no idioma do usuário. Cada mensagem começa com um bloco <oku-slides-context> que descreve o que o usuário vê. Depois de cada resposta, se houver próximos passos úteis, ofereça de 1 a 4 com suggest_next (rótulos curtos; um clique envia a instrução para você). Antes de adicionar um elemento, leia a ferramenta widgets para conhecer os tipos disponíveis e os seus parâmetros.

agent-instructions-widgets = Criar um widget: um widget é um ficheiro Typst do pacote de modelos; usa widget_sources, widget_source, set_widget, remove_widget e preview_widget, tanto no modo deck como no modo modelos. Forma de um ficheiro kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of, depois #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); o nome declarado é o do ficheiro. Parâmetros: descritores p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Textos editáveis: field(ctx, "value", args.value), empilhados com fields-stack. Cores e tamanhos: tokens do tema (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), nunca valores fixos. Código partilhado: o ficheiro _commun, importado com #import "_commun.typ": …. Ciclo: escreve com set_widget, lê o estado e os seus diagnósticos (linha, coluna), vê preview_widget, corrige. Para o inserir no deck: add_element com kind = nome do widget. Um widget de diapositivo declara category: "slide" (widget("nome", category: "slide", params: …)); caso contrário, é um elemento. Num descritor, label: e default: são opcionais (p.plain(), p.plain(label: "Título", default: "…")).

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
