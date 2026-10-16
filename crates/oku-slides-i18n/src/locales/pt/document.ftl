pdf-unreadable = PDF ilegível
pdf-unsupported-version = Este PDF foi gravado por uma versão mais recente do Oku Slides
pdf-corrupted = As fontes deste PDF estão danificadas
pdf-write = Não foi possível gravar o PDF

render-compile = Não foi possível renderizar a apresentação
render-pdf = A exportação para PDF falhou
render-no-page = Esta página não existe
render-image = Não foi possível gerar a imagem do slide

render-setup = Não foi possível iniciar o renderizador
render-missing-image = imagem ausente
diagnostic-error = erro
diagnostic-warning = aviso
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = dica: { $hint }
render-chart-failed = não foi possível desenhar o gráfico

registry-bad-pack-id = Identificador de pacote de modelos inválido: { $id }
registry-compile = Não foi possível compilar os widgets do pacote de modelos
registry-missing = O pacote de modelos não declara seus widgets
render-timeout = A renderização demorou mais de { $seconds } s e foi interrompida
render-out-of-memory = A renderização precisou de mais de { $mib } MiB de memória e foi interrompida
render-worker-lost = O processo de renderização parou
render-frame = Mensagem ilegível do processo de renderização
render-page-too-large = Este slide é grande demais para a pré-visualização (mais de { $mib } MiB, muitas vezes por causa de uma imagem muito grande). Ainda pode ser exportado como PDF ou PNG.
render-stale = A renderização foi substituída; tente novamente
render-pack-disabled = Os widgets do pacote “{ $pack }” estão desativados nesta sessão
render-protocol = O processo de renderização pertence a outra versão do Oku Slides; reinicie o aplicativo
render-remote = O processo de renderização não conseguiu concluir a solicitação
registry-schema-remote = Registro de widgets inválido: { $message }
probe-not-probed = Ainda não verificado: o tempo reservado aos widgets do pacote se esgotou. Será verificado na próxima renderização.
probe-too-many-files = O pacote tem mais de { $max } arquivos de widgets: ele não é carregado.
probe-too-large = Mais de { $max } KiB: não carregado.

import-unreadable = Não foi possível ler esta apresentação
import-not-a-presentation = Este arquivo não é uma apresentação do PowerPoint
import-protected = Esta apresentação está criptografada ou no antigo formato .ppt: salve-a primeiro como .pptx
import-too-large = Esta apresentação é grande demais para ser importada
import-output-too-large = A apresentação resultante excederia o tamanho permitido para uma importação
import-loss-list-levels = Níveis de lista reduzidos a um
import-loss-merged-cells = Células mescladas separadas
import-loss-percent-stacked = Gráfico empilhado 100% exibido como empilhado
import-loss-chart-format = Formato numérico { $format } do gráfico não mantido
import-loss-run-style = { $effect ->
    [underline] Sublinhado perdido
    [strike] Tachado perdido
    [baseline] Sobrescrito ou subscrito perdido
    [caps] Maiúsculas perdidas
    [font] Fonte de um trecho substituída pela da caixa
    [tracking] Espaçamento entre letras de um trecho substituído pelo da caixa
    [weight] Trecho normal exibido em negrito como a caixa
    [slant] Trecho reto exibido em itálico como a caixa
   *[highlight] Realce perdido
}
import-loss-transform = Rotação, espelhamento ou corte perdidos
import-loss-color-dropped = A cor { $color } não tem cor próxima no tema: cor do tema usada
import-loss-text-truncated = Texto encurtado para caber nos limites
import-loss-table-truncated = Tabela encurtada para caber nos limites
import-loss-opaque = { $reason ->
    [media] Vídeo ou som não importado (sem imagem substituta)
    [ole] Objeto incorporado não importado (sem imagem substituta)
    [math] Equação não importada (sem imagem substituta)
    [smartart] SmartArt não importado (sem imagem substituta)
   *[unknown] Objeto não suportado não importado (sem imagem substituta)
}
import-loss-smartart-layout = SmartArt sem o desenho: o texto foi disposto em cartões e o layout do diagrama foi perdido
import-loss-unsupported-image = Imagem no formato { $format } não importada
import-loss-hidden-slide = Slide oculto importado como visível
import-loss-effects = Sombras, reflexos e efeitos 3D perdidos
import-loss-series-dropped = Um gráfico de pizza mostra uma única série: { $kept } mantida, { $dropped } descartadas
import-loss-repeated-decor = { $slides ->
   [one] Cabeçalho ou rodapé “{ $text }” repetido em { $slides } slide omitido: o tema desenha os seus
  *[other] Cabeçalho ou rodapé “{ $text }” repetido em { $slides } slides omitido: o tema desenha os seus
}
import-loss-no-slot = Conteúdo omitido: o modelo escolhido não tem lugar para ele
import-loss-page-fitted = A página de origem ({ $width } × { $height } pt) não é 16:9: conteúdo redimensionado e centralizado
import-loss-paragraph-style = Alinhamento, espaçamento de linha ou de parágrafo ajustados aos da caixa
import-loss-shape-style = { $effect ->
    [gradient] Gradiente mostrado como cor sólida
    [picture] Imagem de preenchimento esticada ao quadro da forma
    [dash] Linha tracejada mostrada contínua
    [arrow] Pontas de seta da linha não desenhadas
   *[geometry] Forma desconhecida mostrada como retângulo
}
import-loss-layout-dropped = Layouts demais para um pacote: “{ $layout }” não tem modelo; seus slides usam o widget vazio e recebem sua decoração como elementos
import-loss-decor-truncated = { $omitted ->
    [one] Decoração cortada para caber no tamanho de um arquivo do pacote: { $omitted } elemento omitido
   *[other] Decoração cortada para caber no tamanho de um arquivo do pacote: { $omitted } elementos omitidos
}
import-loss-table-style = Estilo da tabela aproximado: seus fundos de célula, bordas, larguras de coluna ou tamanhos de texto próprios se perdem
import-loss-overflow = O elemento “{ $element }” ultrapassa a borda do slide
import-loss-empty-data = Tabela ou gráfico sem dados: nada é inserido
import-reference-unavailable = Sem pontuação visual: { $cause ->
    [timeout] a renderização de referência demorou demais
    [memory] a renderização de referência usou memória demais
    [pages] a renderização de referência tem { $reference ->
        [one] { $reference } página
       *[other] { $reference } páginas
    } para { $deck } na apresentação
    [missing] o LibreOffice (soffice) não foi encontrado; informe o caminho em import.soffice
   *[failed] o renderizador de referência não conseguiu ler a apresentação
}
import-font-substituted = { $family } substituída por { $by }, com métricas compatíveis
import-font-missing = { $family } não encontrada: o texto usa { $by }
import-font-restricted = { $family }: a licença proíbe incorporar algumas das suas variantes na apresentação; o texto nesses estilos usa as outras variantes
import-font-restricted-substituted = { $family } não pode ser incorporada na apresentação (licença): substituída por { $by }, com métricas compatíveis
import-font-restricted-missing = { $family } não pode ser incorporada na apresentação (licença): o texto usa { $by }
import-font-too-large = { $family }: arquivos grandes demais para incorporar no deck; o texto usa { $by }
import-font-missing-no-fallback = { $family } não encontrada, nem a fonte do corpo do tema: o texto é renderizado com a fonte padrão do motor
import-font-too-large-no-fallback = { $family }: arquivos grandes demais para incorporar no deck, e a fonte do corpo do tema também não está disponível: o texto é renderizado com a fonte padrão do motor
import-font-restricted-no-fallback = { $family } não pode ser incorporada na apresentação (licença), e a fonte do corpo do tema também não está disponível: o texto é renderizado com a fonte padrão do motor
import-warning-effect-ignored = Efeito não importado: { $detail }
import-warning-approximated = Renderização aproximada: { $detail }
import-warning-unreadable-item = Item ilegível ignorado: { $detail }
import-warning-unreadable-part = Parte ilegível: { $detail }
import-warning-missing-part = Parte ausente: { $detail }
import-warning-unsupported-chart = Gráfico não suportado: { $detail }
import-warning-unsupported-image = Formato de imagem não suportado: { $detail }
import-warning-oversized-image = Imagem grande demais para importar (número de pixels): { $detail }
import-warning-external-image = Imagem vinculada fora do arquivo, não importada: { $detail }
import-warning-unsafe-svg = Imagem SVG limpa de conteúdo inseguro, ou descartada: { $detail }
import-warning-unknown-table-style = Estilo de tabela desconhecido: { $detail }
import-warning-unsafe-link = Link recusado: { $detail }
import-warning-nesting-too-deep = Aninhamento profundo demais, cortado: { $detail }
import-warning-font-embedded-unreadable = Fonte incorporada ilegível: { $detail }
import-warning-truncated = Avisos demais: os seguintes foram omitidos
