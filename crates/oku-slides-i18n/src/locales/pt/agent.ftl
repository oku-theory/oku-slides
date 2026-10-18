# Agente: superfícies, confirmações, thread.

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
agent-confirm-edit-theme = Salvar uma nova versão do tema “{ $name }”?
agent-confirm-copy-theme = Salvar uma cópia modificada do tema integrado “{ $name }”?
agent-confirm-setting = Alterar { $key }: { $old } → { $new }?

agent-not-found = Agente não encontrado: { $command }. Configuração [agent.presets].
agent-spawn-failed = Não foi possível iniciar o agente: { $command }. Configuração [agent.presets].
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
agent-memory-lost = O agente não se lembra mais desta conversa: ele recomeça do zero.
agent-turn-max-tokens = O agente atingiu o limite de tamanho.
agent-turn-max-turns = O agente atingiu o limite de etapas.
agent-turn-refusal = O agente recusou.
agent-turn-ended = Turno encerrado ({ $stop }).
agent-choice-cancelled = Pergunta cancelada
agent-choice-answered = ✓ { $answer }
agent-report-why = Por quê

agent-error-file = Não foi possível ler ou gravar o arquivo.
agent-error-unexpected-action = Ação inesperada: { $action }.
agent-error-unknown-surface = Este cartão não está mais aberto.
agent-error-expired = As ações de um turno anterior expiraram.
agent-error-incomplete = A ação deste cartão está incompleta.
agent-error-no-field = Esta configuração não existe mais.
agent-error-no-variant = Esta variante não existe mais.
agent-error-arguments = Os argumentos da ferramenta são inválidos ({ $detail }): verifique-os segundo o seu esquema.
agent-error-unknown-tool = Ferramenta desconhecida: { $name }.
agent-error-pack-id = Isto não é o id de um conjunto de modelos: chame templates para listar os conjuntos e seus ids.
agent-error-denied = O usuário recusou.
agent-error-no-turn = Nenhum turno em andamento: { $tool } só é mostrado durante um turno.
agent-error-turn-over = O turno desta chamada terminou: nada foi mostrado.
agent-error-anchor-scope = Uma pergunta em uma conversa de widget não pode ser ancorada em um slide: faça-a sem âncora.
agent-error-no-file = A apresentação ainda não tem arquivo: indique um caminho absoluto.
agent-error-path-relative = O caminho deve ser absoluto.
agent-error-path-parent = O caminho não deve conter “..”.
agent-error-setting-locked = O agente não pode alterar a configuração { $key }.
agent-error-setting = Não foi possível alterar a configuração { $key }: verifique o valor.
agent-error-rewrite-text = A reformulação voltou sem texto.
agent-error-paragraphs = Os parágrafos da reformulação são inválidos.
agent-error-draft-too-long = O rascunho é longo demais.
agent-error-draft-origin = Rascunho recusado: visualização desconhecida.
agent-error-draft-no-agent = O agente desta aba ainda não foi iniciado: rascunho recusado.
agent-error-unknown-agent = Este agente não está mais na configuração.
agent-error-agent-locked = A escolha do agente é imposta pelo ambiente.
agent-error-agent-not-saved = Não foi possível salvar a escolha do agente.
agent-picker-title = Agente
agent-picker-busy = Trocar de agente interromperá o turno em andamento.
agent-picker-locked = Escolha imposta pelo ambiente (OKU_SLIDES_AGENT__SELECTED).
agent-tune-prompt = Afinar a apresentação importada
