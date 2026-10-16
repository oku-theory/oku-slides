cli-reading = lendo { $path }
cli-invalid-deck = apresentação inválida: { $path }
cli-deck-valid = apresentação válida: { $slides } slides
cli-deck-issue = problema: { $issue }
cli-rendered = { $pages } páginas → { $out } (compilação { $compile }, SVG { $svg })
cli-asset-added = recurso { $path } → { $id }
cli-saved = { $path }: { $pages } páginas, PDF { $pdf } KB, fontes { $sources } KB
cli-opened = { $path }: { $slides } slides, { $assets } recursos, { $fonts } fontes tipográficas
cli-extracted = extraído para { $dir }
cli-stripped = { $path }: cópia sem fontes
cli-pdf-modified = o PDF foi alterado por outra ferramenta depois do Oku Slides
cli-iterations = --iterations deve ser > 0
cli-no-block = a apresentação não tem nenhum bloco para editar
cli-blob-missing = blob de fontes não encontrado
cli-roundtrip-mismatch = a apresentação relida difere da salva
cli-bench-world = construção do mundo (fontes)          { $time }
cli-bench-first = primeira compilação                   { $time }  ({ $pages } páginas)
cli-bench-same = recompilação sem alteração            { $time }
cli-bench-edited-text = Texto editado { $n }
cli-bench-edit = recompilação após set_text            { $time }
cli-bench-svg = SVG de cada slide                     { $time }  ({ $each } por slide)
cli-bench-pdf = exportação PDF                        { $time }  ({ $size } KB)
cli-bench-embed = incorporação do blob de fontes        { $time }
cli-bench-extract = releitura do blob                     { $time }
cli-bench-roundtrip = ida e volta da apresentação           idêntica
cli-bench-history = passo do histórico                        { $mean } ms em média, p50 { $p50 } ms
cli-serving = Oku Slides (dev) em http://{ $host }:{ $port }
cli-serving-token = abra { $url } (o token vale até o servidor parar)
cli-models-dir = modelos de ditado: { $dir }
cli-stopped = servidor parado
cli-not-found = não encontrado
cli-bad-request = solicitação inválida: { $reason }
cli-sse-full = muitos fluxos de eventos já abertos
cli-forbidden = pedido recusado: não vem da interface local
cli-unauthorized = pedido recusado: token de acesso ausente ou incorreto
cli-exposed = escutando em { $host }: o servidor é acessível a partir da rede; só são atendidos os clientes que apresentam o token desta execução. Mantenha [serve] host em 127.0.0.1 se não precisar da rede
cli-imported = { $path }: { $slides ->
       [one] { $slides } slide
      *[other] { $slides } slides
    }, { $pages ->
       [one] { $pages } página
      *[other] { $pages } páginas
    }, { $losses ->
       [one] { $losses } perda
      *[other] { $losses } perdas
    }, { $requests ->
       [one] { $requests } pedido de widget
      *[other] { $requests } pedidos de widget
    }
cli-import-label = Importação do PowerPoint
cli-import-pack-required = --pack é obrigatório no modo retrofit: o id, a pasta ou o nome de um pacote
cli-import-unknown-pack = pacote desconhecido: { $pack }
cli-import-bad-pack = pacote ilegível { $pack }: o seu pack.json não descreve um pacote de modelos
cli-import-write-failed = não foi possível gravar a apresentação importada { $path }
cli-import-hifi-no-target = --pack e --theme são do modo retrofit: a importação de alta fidelidade (--mode hifi) gera o seu próprio pacote e tema
cli-import-probe-mismatch = o pacote gerado não corresponde à sondagem como declarado (widget { $widget }): é um erro do importador
cli-import-probe-limit = o pacote gerado excede os limites da sonda (widget { $widget }): aumente max_widgets, max_widget_kib ou probe_budget_ms na seção [render] da configuração
cli-import-probe-failed = o processo de renderização não conseguiu sondar o pacote gerado: { $error }
cli-import-pack-missing = a importação não produziu nenhum pacote para o deck: é um erro da importação
cli-import-theme-missing = a importação não produziu nenhum tema para o deck: é um erro da importação
cli-import-reference-hifi-only = --reference pertence ao modo hifi: uma importação retrofit não é comparada com uma renderização de referência
cli-import-bad-threshold = score_threshold da seção [import] deve ser um número maior ou igual a zero, não { $value }
cli-import-scored = Diferença visual para { $witness }: { $mean } em média, { $above ->
    [0] nenhum slide
    [one] { $above } slide
   *[other] { $above } slides
} acima de { $threshold }
cli-import-unscored = { $reason }

cli-about = Renderiza e armazena apresentações do Oku Slides
cli-arg-fonts = Pasta de fontes extras (TTF/OTF); padrão: paths.fonts nas configurações
cli-cmd-check = Verificar uma apresentação
cli-arg-deck = Apresentação (JSON)
cli-cmd-render = Renderiza cada slide em SVG, com as caixas dos elementos
cli-arg-input = Arquivo deck.json ou .oku-slides.pdf
cli-arg-out = Pasta de saída
cli-arg-theme = Substituir o tema da apresentação: um tema incluído ou da biblioteca, pelo nome
cli-cmd-save = Salvar uma apresentação como .oku-slides.pdf (PDF + blob de fontes)
cli-arg-output = Arquivo a gravar
cli-arg-assets = Pasta de recursos: cada arquivo se torna um recurso sha256:…
cli-arg-embed-fonts = Incorporar também no blob de fontes as fontes tipográficas de --fonts
cli-cmd-open = Reabrir um .oku-slides.pdf: extrair a apresentação e seus recursos
cli-arg-extract = Pasta onde gravar deck.json, assets/ e fonts/
cli-cmd-strip = Copiar um .oku-slides.pdf sem o blob de fontes
cli-cmd-bench = Medir os tempos de compilação e exportação
cli-arg-iterations = Medições por etapa
cli-cmd-schema = Exibir o esquema JSON das operações
cli-cmd-serve = Servidor de desenvolvimento: a interface em um navegador, sem Tauri
cli-arg-serve-input = Apresentação a abrir (.oku-slides.pdf ou .json)
cli-arg-port = Porta; padrão: serve.port nas configurações
cli-arg-ui = Pasta da interface compilada (ui/dist)
cli-cmd-import = Importar uma apresentação do PowerPoint (.pptx) como .oku-slides.pdf
cli-arg-pptx = Apresentação do PowerPoint (.pptx)
cli-arg-mode = retrofit: o conteúdo sobre um pacote e um tema existentes; hifi: cópia fiel, sobre um pacote e um tema gerados para ela
cli-arg-pack = Pacote de destino: id, pasta do pacote (pack.json, widgets/) ou nome
cli-arg-import-theme = Tema de destino: um tema incluído ou da biblioteca, pelo id ou nome; padrão: o tema padrão
cli-arg-report = Gravar também o relatório de importação (JSON): perdas, substituições, cobertura
cli-arg-no-system-fonts = No modo hifi, não procurar as fontes da apresentação entre as fontes do sistema
cli-arg-reference = No modo hifi, comparar cada slide com uma renderização de referência da apresentação (auto, o padrão) ou não (none)
