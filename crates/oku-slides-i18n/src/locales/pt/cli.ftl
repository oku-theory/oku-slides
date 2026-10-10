cli-reading = lendo { $path }
cli-invalid-deck = apresentação inválida: { $path }
cli-deck-valid = apresentação válida: { $slides } slides
cli-deck-issue = problema: { $issue }
cli-rendered = { $pages } páginas → { $out } (compilação { $compile }, SVG { $svg })
cli-asset-added = recurso { $path } → { $id }
cli-saved = { $path }: { $pages } páginas, PDF { $pdf } KB, fontes { $sources } KB
cli-opened = { $path }: { $slides } slides, { $assets } recursos, { $fonts } tipos de letra
cli-extracted = extraído para { $dir }
cli-stripped = { $path }: cópia sem fontes
cli-pdf-modified = o PDF foi alterado por outra ferramenta depois do Oku Slides
cli-iterations = --iterations deve ser > 0
cli-no-block = a apresentação não tem nenhum bloco para editar
cli-blob-missing = blob de fontes não encontrado
cli-roundtrip-mismatch = a apresentação relida difere da salva
cli-bench-world = construção do mundo (tipos de letra)  { $time }
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
cli-models-dir = modelos de ditado: { $dir }
cli-stopped = servidor parado
cli-not-found = não encontrado
cli-bad-request = solicitação inválida: { $reason }
cli-sse-full = muitos fluxos de eventos já abertos
cli-forbidden = pedido recusado: não vem da interface local
cli-exposed = a escutar em { $host }: o servidor não tem autenticação e é acessível a partir da rede; qualquer cliente que envie um cabeçalho Host local é servido. Mantenha [serve] host em 127.0.0.1
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
cli-import-hifi-later = a importação de alta fidelidade (--mode hifi) ainda não está disponível: use --mode retrofit

cli-about = Renderiza e armazena apresentações do Oku Slides
cli-arg-fonts = Pasta de tipos de letra extra (TTF/OTF); padrão: paths.fonts nas configurações
cli-cmd-check = Verificar uma apresentação
cli-arg-deck = Apresentação (JSON)
cli-cmd-render = Renderiza cada slide em SVG, com as caixas dos elementos
cli-arg-input = Arquivo deck.json ou .oku-slides.pdf
cli-arg-out = Pasta de saída
cli-arg-theme = Substituir o tema da apresentação: um tema incluído ou da biblioteca, pelo nome
cli-cmd-save = Salvar uma apresentação como .oku-slides.pdf (PDF + blob de fontes)
cli-arg-output = Arquivo a gravar
cli-arg-assets = Pasta de recursos: cada arquivo se torna um recurso sha256:…
cli-arg-embed-fonts = Incorporar também nas fontes os tipos de letra de --fonts
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
cli-arg-mode = retrofit: o conteúdo sobre um pacote e um tema existentes; hifi: cópia fiel (ainda não disponível)
cli-arg-pack = Pacote de destino: id, pasta do pacote (pack.json, widgets/) ou nome
cli-arg-import-theme = Tema de destino: um tema incluído ou da biblioteca, pelo id ou nome; padrão: o tema padrão
cli-arg-report = Gravar também o relatório de importação (JSON): perdas, substituições, cobertura
