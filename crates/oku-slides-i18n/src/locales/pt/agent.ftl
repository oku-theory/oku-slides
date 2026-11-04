# Agente: superfícies, confirmações, thread.

agent-surface-cancel = Cancelar
agent-surface-confirm = Confirmar
agent-surface-confirm-danger = Sim, fazer
agent-surface-heard = Ouvido: “{ $text }”
agent-surface-say-or-click = Diga sua resposta ou clique em um botão.
agent-surface-said-too-early = Dito cedo demais: espere a janela e diga de novo.
agent-surface-click-to-confirm = Clique no botão para confirmar.
agent-surface-verb-replace = Substituir
agent-surface-verb-merge = Mesclar
agent-surface-verb-discard = Descartar
agent-surface-verb-delete = Excluir
agent-surface-verb-use-api = Usar a API
agent-surface-verb-install = Instalar
agent-surface-verb-quit = Fechar o aplicativo
agent-surface-verb-close = Fechar
agent-surface-verb-import = Importar
agent-surface-rewrite = Reformulação
agent-surface-pick = Escolher { $n }
agent-surface-keep = Manter o original
agent-surface-edit = Editar antes de aplicar
agent-surface-close = Fechar
agent-surface-themes = Temas
agent-surface-variants = Variantes
agent-surface-permission-always = para sempre
agent-surface-permission-title = O agente pede uma permissão
agent-surface-permission-guides = Este turno inclui o guia recebido de { $names }.
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
agent-confirm-copy-pack = Salvar uma cópia do pacote “{ $name }”? O próprio pacote não muda.
agent-confirm-copy-edited-theme = Salvar uma cópia do tema “{ $name }”? O próprio tema não muda.
agent-confirm-delete-pack = Excluir o pacote “{ $name }”? Isso não pode ser desfeito.
agent-confirm-edit-theme = Salvar uma nova versão do tema “{ $name }”?
agent-confirm-copy-theme = Criar uma variante do tema integrado “{ $name }”? O tema integrado não muda.
agent-confirm-setting = Alterar { $key }: { $old } → { $new }?
agent-confirm-move = Mover a apresentação para { $path }? O arquivo antigo será excluído.
agent-confirm-merge = Mesclar { $path } com esta apresentação? Não é possível desfazer.
agent-confirm-save-copy = Salvar uma cópia da apresentação como { $path }?
agent-confirm-dictation-api = Enviar o que você dita para { $host } ({ $url }), modelo { $model }, com o segredo de { $var }?
agent-confirm-dictation-download = Usar { $name } e baixá-lo agora ({ $size })?
agent-confirm-dictation-language = Usar o ditado da Apple e baixar o idioma dele agora?
agent-confirm-agent = Trocar o agente do app para { $name } ({ $command }) quando nenhuma resposta estiver em andamento?
agent-confirm-agent-builtin = Trocar o agente do app para { $name } quando nenhuma resposta estiver em andamento?
agent-confirm-agent-model = Baixar o modelo do agente local ({ $size })?

agent-not-found = Agente não encontrado: { $command } não existe nesse caminho nem no PATH do app. Informe o caminho completo em [agent.presets].
agent-spawn-failed = Não foi possível iniciar o agente: { $command }. Configuração [agent.presets].
agent-no-http = Este agente não consegue se conectar às ferramentas do Oku Slides.
agent-local-no-program = O agente local só funciona dentro do app Oku Slides.
agent-local-needs-download = O agente local precisa primeiro baixar o modelo ({ $gigabytes } GB).
agent-local-intel-mac = O agente local precisa de um Mac com chip Apple.
agent-local-unsupported-os = O agente local ainda não está disponível neste sistema.
agent-local-apple-off = O agente local precisa do Apple Intelligence: ative-o nos Ajustes do Sistema.
agent-local-apple-not-ready = O modelo da Apple ainda está sendo preparado: tente o agente local novamente em alguns minutos.
agent-local-apple-unsupported = O modelo da Apple precisa de um Mac compatível com o Apple Intelligence, com macOS 26 ou posterior.
agent-model-download = Baixar o modelo
agent-model-downloading = Baixando o modelo… { $percent }%
agent-model-cancel = Cancelar
agent-model-download-failed = Não foi possível baixar o modelo.
agent-model-download-integrity = O modelo baixado estava corrompido e foi excluído.
agent-model-download-incomplete = Download incompleto: { $got } bytes de { $expected }.
agent-model-download-cancelled = Download cancelado.
agent-fallback-notice = Agente local — { $reason }
agent-fallback-download = { $reason } O agente local pode assumir assim que o modelo ({ $model }, { $gigabytes } GB) for baixado. Baixar agora?
agent-fallback-unusable = Nenhum agente pode funcionar. Agente principal: { $main } Agente local: { $local }
local-agent-no-route = O agente local não entendeu o que fazer. Tente dizer de outro jeito.
local-agent-tool-limit = O agente local parou depois de { $n } etapas sem terminar.
local-agent-invalid-call = O agente local não conseguiu usar as ferramentas corretamente.
local-agent-engine-failed = O modelo do agente local falhou.
local-agent-retry = Tente novamente ou escolha outro agente no painel do agente.
agent-stopped = O agente parou.
agent-failed = O agente recusou o pedido: { $message }
agent-error-work-dir = Não foi possível preparar a pasta de trabalho privada do agente para esta aba.
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
agent-memory-lost = O agente não se lembra mais desta conversa: ele recomeça do zero.
agent-turn-max-tokens = O agente atingiu o limite de tamanho.
agent-turn-max-turns = O agente atingiu o limite de etapas.
agent-turn-refusal = O agente recusou.
agent-turn-ended = Turno encerrado ({ $stop }).
agent-choice-cancelled = Pergunta cancelada
agent-choice-answered = ✓ { $answer }
agent-report-why = Por quê
agent-trace = { $name }: { $count ->
    [one] { $count } alteração
   *[other] { $count } alterações
}
agent-trace-data = { $name }: dados alterados
agent-trace-removed = { $name }: não existe mais
agent-trace-more = { $count ->
    [one] e mais { $count }
   *[other] e mais { $count }
}
agent-trace-title = Elementos alterados

agent-error-file = Não foi possível ler ou gravar o arquivo.
agent-error-unexpected-action = Ação inesperada: { $action }.
agent-error-unknown-surface = Este cartão não está mais aberto.
agent-error-expired = As ações de um turno anterior expiraram.
agent-error-incomplete = A ação deste cartão está incompleta.
agent-error-no-field = Esta configuração não existe mais.
agent-error-no-variant = Esta variante não existe mais.
agent-error-arguments = Os argumentos da ferramenta são inválidos ({ $detail }): verifique-os segundo o seu esquema.
agent-error-unknown-tool = Ferramenta desconhecida: { $name }.
agent-error-pack-id = Isto não é o id de um pacote: chame packs para listar os pacotes e seus ids.
agent-error-asset-unknown = Esta imagem não está na apresentação nem entre as imagens coladas: use o identificador de uma imagem colada.
agent-error-denied = O usuário recusou.
agent-error-no-turn = Nenhum turno em andamento: { $tool } só é mostrado durante um turno.
agent-error-tab-no-turn = Abra, mostre ou mova uma aba só enquanto responde ao usuário.
agent-error-tab-limit = Você já abriu, mostrou ou moveu uma aba neste turno; termine lá, ou pergunte ao usuário antes de outra.
agent-error-continue-in-other-tab = Este pedido continua na aba que você abriu ou mostrou: peça ao usuário que continue lá; esta aba não altera nenhum widget por enquanto.
agent-error-pack-open-in-tab = Este pacote está sendo editado em outra aba: peça ao usuário que termine lá primeiro.
agent-pack-opened-in-tab = { $shown ->
   [yes] O pacote foi aberto na aba { $tab }, agora exibida.
  *[no] O pacote foi aberto na aba { $tab }, em segundo plano: a janela do usuário mostra outra aba.
} { $draft ->
   [set] O pedido aguarda no rascunho dessa aba: diga ao usuário que o revise lá e depois o envie.
   [kept] O rascunho dessa aba já tinha algo e foi mantido como está. Diga ao usuário que continue lá: o agente dessa aba não conhece este pedido.
   [too_long] O pedido era longo demais para o rascunho dessa aba. Diga ao usuário que continue lá: o agente dessa aba não conhece este pedido.
   [unavailable] O pedido não pôde chegar ao rascunho dessa aba. Diga ao usuário que continue lá: o agente dessa aba não conhece este pedido.
  *[none] Diga ao usuário que continue lá: o agente dessa aba não conhece este pedido.
} Esta aba continua como está.
agent-theme-opened-in-tab = { $shown ->
   [yes] O tema foi aberto na aba { $tab }, agora exibida.
  *[no] O tema foi aberto na aba { $tab }, em segundo plano: a janela do usuário mostra outra aba.
} { $draft ->
   [set] O pedido aguarda no rascunho dessa aba: diga ao usuário que o revise lá e depois o envie.
   [kept] O rascunho dessa aba já tinha algo e foi mantido como está. Diga ao usuário que continue lá: o agente dessa aba não conhece este pedido.
   [too_long] O pedido era longo demais para o rascunho dessa aba. Diga ao usuário que continue lá: o agente dessa aba não conhece este pedido.
   [unavailable] O pedido não pôde chegar ao rascunho dessa aba. Diga ao usuário que continue lá: o agente dessa aba não conhece este pedido.
  *[none] Diga ao usuário que continue lá: o agente dessa aba não conhece este pedido.
} Esta aba continua como está.
agent-error-turn-over = O turno desta chamada terminou: nada foi mostrado.
agent-error-no-box-turn = Nenhum turno ao qual associar esta caixa: nada foi mostrado.
agent-error-no-file = A apresentação ainda não tem arquivo: indique um caminho absoluto.
agent-error-path-relative = O caminho deve ser absoluto.
agent-error-path-parent = O caminho não deve conter “..”.
agent-error-setting-locked = O agente não pode alterar a configuração { $key }.
agent-error-setting = Não foi possível alterar a configuração { $key }: verifique o valor.
agent-error-history-index = Para percorrer o histórico, é preciso o índice de uma versão.
agent-error-no-pack = Esta apresentação não fixa nenhum pacote: informe pack (veja packs).
agent-error-history-not-opened = A aba não abriu o histórico a tempo: peça de novo.
agent-error-locale = Código de idioma desconhecido. Informe o código de um idioma incluído ou system.
agent-error-empty-interface = Nada para mudar: informe scheme, locale, guides, folded, pile, notes ou code.
agent-error-layout-range = { $field } deve ficar entre { $min } e { $max } px.
agent-error-layout-min = { $field } deve ter pelo menos { $min } px.
agent-error-empty-dictation = Nada para mudar: informe pelo menos um ajuste do ditado.
agent-error-empty-speech = Nada a alterar: indique pelo menos uma definição de leitura em voz alta.
agent-error-not-shown = Esta aba não é a exibida na janela: peça de novo quando o usuário exibi-la.
agent-error-not-presenting = O deck não está sendo apresentado aqui: inicie primeiro a apresentação com a ação start.
agent-error-present-slide = Para mostrar um slide, é preciso o id de um slide deste deck (veja deck_outline).
agent-error-undo-open-turn = undo_turn não desfaz o turno em andamento: use undo para as alterações deste turno.
agent-error-app-unavailable = Só o app Oku Slides pode fazer isso; aqui não está disponível.
agent-error-app-closing = O Oku Slides já está fechando.
agent-error-app-no-offer = Não há nenhuma atualização para adiar ou instalar aqui: procure atualizações primeiro.
agent-error-app-failed = A atualização do Oku Slides falhou: verifique a conexão ou use o aviso de atualização.
agent-error-after-turn-taken = Esta resposta já termina fechando o app, instalando uma atualização ou fechando a sua aba.
agent-error-no-pack-update = O painel de packs não oferece essa atualização para este pack: veja show_packs.
agent-confirm-update-install = Instalar o Oku Slides { $version }? O app reinicia quando esta resposta terminar.
agent-after-turn-quit-unsaved = O Oku Slides não fechou: uma aba tem alterações não salvas desde o seu pedido.
agent-error-workspace-unavailable = Só a app Oku Slides pode mudar abas e janelas; aqui não está disponível.
agent-error-workspace-unknown-tab = Esta janela não tem essa aba ou janela.
agent-error-workspace-unknown-window = Essa janela não existe.
agent-error-workspace-index = Essa posição fica além do fim da barra de abas.
agent-error-workspace-closing = O Oku Slides está fechando: abas e janelas não mudam mais.
agent-confirm-close-tab = Fechar a aba “{ $tab }”?
agent-confirm-discard-tab = Fechar a aba “{ $tab }”? As alterações não salvas dela serão perdidas.
agent-confirm-close-window = Fechar esta janela e as suas abas?
agent-confirm-discard-window = Fechar esta janela? As alterações não salvas em { $tabs } serão perdidas.
agent-error-window-last = Esta é a única janela: feche as abas dela ou saia do Oku Slides.
agent-error-tab-index = Diga onde colocar a aba: reordenar precisa de uma posição.
agent-error-tab-unused = Só reordenar e mover recebem uma posição, e só mover recebe uma janela.
agent-after-turn-close-unsaved = A aba não foi fechada: ela mudou desde o seu pedido.
agent-after-turn-close-not-shown = A aba não foi fechada: outra aba estava sendo mostrada na janela dela quando a resposta terminou.
agent-confirm-quit = Sair do Oku Slides? O app fecha quando esta resposta terminar.
agent-error-quit-unsaved = Abas com trabalho não salvo: { $count }. O Oku Slides não foi fechado: o usuário as salva ou fecha primeiro.
agent-error-tune-unavailable = O ajuste fino não pôde começar: o agente não está disponível.
agent-error-tune-not-started = O ajuste fino não começou: o agente dessa aba não pode ser usado por ora; a aba diz por quê.
agent-confirm-import = Importar { $path } numa nova aba?
agent-import-opened-in-tab = { $shown ->
   [yes] A apresentação abriu na aba { $tab }, agora exibida.
  *[no] A apresentação abriu na aba { $tab }, em segundo plano: a janela do usuário mostra outra aba.
} { $tune ->
   [started] O ajuste fino dela começa lá, na conversa própria dessa aba.
   [unavailable] O ajuste fino dela não pôde começar: diga ao usuário para pedi-lo nessa aba.
  *[no] Diga ao usuário para continuar lá.
} Esta aba fica como está.
agent-error-import-hifi-target = Uma importação hi-fi não aceita pack nem tema: retire-os ou importe no modo retrofit.
agent-error-import-extension = Só é possível importar um arquivo PowerPoint .pptx.
agent-error-a2ui-unknown = { $name } não é um componente aceito por render. Componentes: { $known }.
agent-error-a2ui-elsewhere = render não mostra { $name }: use { $tool }.
agent-error-a2ui-host-only = { $name } é reservado ao app: render não pode exibi-lo.
agent-error-themes-deck-mode = O seletor de temas só abre durante a edição dos diapositivos.
agent-error-packs-deck-mode = O painel de pacotes da apresentação só abre durante a edição dos diapositivos.
agent-error-usage-no-deck = O uso desta aba não pertence a nenhuma apresentação: só uma aba de pacote ou de tema aberta a partir de uma apresentação (pelo usuário ou pelo agente), ou o pacote próprio de uma apresentação, conta para ela.
agent-error-rewrite-text = A reformulação voltou sem texto.
agent-error-paragraphs = Os parágrafos da reformulação são inválidos.
agent-error-draft-too-long = O rascunho é longo demais.
agent-error-report-elsewhere = Esta janela não mostra esse separador.
agent-error-ui-too-large = O contexto da página é demasiado grande.
agent-error-seq-exhausted = O número do relatório da página está fora do intervalo.
agent-error-draft-origin = Rascunho recusado: visualização desconhecida.
agent-error-draft-no-agent = O agente desta aba ainda não foi iniciado: rascunho recusado.
agent-error-too-many-images = O rascunho já tem todas as imagens.
agent-error-image-not-base64 = A imagem colada não chegou inteira.
agent-error-image-gone = Uma imagem colada não está mais disponível: cole-a novamente.
agent-error-unknown-agent = Este agente não está mais na configuração.
agent-error-agent-locked = A escolha do agente é imposta pelo ambiente.
agent-error-agent-not-saved = Não foi possível salvar a escolha do agente.
agent-error-unknown-model = Este agente não oferece esse modelo.
agent-error-model-not-saved = Não foi possível salvar a escolha do modelo.
voice-error-no-dictation = O ditado não está disponível aqui
voice-error-busy = O ditado está em andamento: tente de novo quando estiver em repouso
voice-error-unknown-model = Modelo de ditado desconhecido
voice-error-unavailable = Este modelo não aceita o idioma escolhido
voice-error-installed = Este modelo já está instalado
voice-error-not-downloadable = Este modelo não é baixado
voice-error-nothing-to-cancel = Nenhum download em andamento
voice-error-bad-silence = O silêncio deve durar de 0,5 a 10 segundos
voice-error-bad-language = Idioma de ditado desconhecido
voice-error-bad-url = O endereço da API deve começar com https:// (http:// só para este computador), sem nome de usuário, parâmetros nem caracteres especiais
voice-error-bad-key-env = A variável da chave deve ser um nome como OPENAI_API_KEY: maiúsculas, dígitos e _
voice-error-bad-api-model = O nome do modelo da API só admite letras, algarismos e . _ : / -; um nome de implantação do Azure só letras, algarismos e . _ -
voice-error-bad-api-version = A versão da API do Azure só admite letras, algarismos, . e -, no máximo 32 caracteres, por exemplo 2025-03-01-preview
voice-error-bad-api-headers = Os cabeçalhos são escritos Nome: valor; Nome: $VARIABLE, no máximo 8. Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection e duplicados são recusados
voice-error-api-reserved = Só o utilizador define o endereço, a variável da chave, a versão do Azure e os cabeçalhos da API: abra a caixa com show_dictation e peça-lhe
voice-error-hands-free-reserved = Só o utilizador ativa a escuta mãos-livres: abra a caixa com show_dictation e peça-lhe
voice-error-changed = As configurações mudaram desde a pergunta: pergunte de novo
voice-error-nothing-to-download = Nada a baixar para este agente
voice-error-bad-rate = A velocidade tem de estar entre 0,5 e 2
voice-error-unknown-voice = Voz desconhecida para o idioma da interface
voice-error-speech-unavailable = A leitura em voz alta não está disponível neste computador
agent-tune-prompt = Afinar a apresentação importada
agent-settings-unknown-selected = A configuração agent.selected = { $selected } não indica nenhum agente de [agent.presets] (conhecidos: { $known })
agent-settings-preset-launch = O agente [agent.presets.{ $id }] precisa de exatamente um entre command e builtin
