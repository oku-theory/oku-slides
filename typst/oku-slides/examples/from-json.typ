#import "../lib.typ": *
#import "../packs/base/widgets/text.typ": widget as text-w
#import "../packs/base/widgets/card.typ": widget as card-w
#import "../packs/base/widgets/image.typ": widget as image-w
#import "../packs/base/widgets/shape.typ": widget as shape-w
#import "../packs/base/widgets/chart.typ": widget as chart-w
#import "../packs/base/widgets/table.typ": widget as table-w
#import "../packs/base/widgets/stack.typ": widget as stack-w
#import "../packs/essentiel/layouts/title.typ": widget as title-l
#import "../packs/essentiel/layouts/toc.typ": widget as toc-l
#import "../packs/essentiel/layouts/chapter.typ": widget as chapter-l
#import "../packs/essentiel/layouts/content.typ": widget as content-l
#import "../packs/essentiel/layouts/end.typ": widget as end-l
#import "../packs/essentiel/layouts/blank.typ": widget as blank-l
// Comme l'assemblage généré par l'app : chaque nom préfixé par l'espace de noms de son pack.
#let base = (text-w, card-w, image-w, shape-w, chart-w, table-w, stack-w).map(w => w + (name: "base/" + w.name))
#let essentiel = (title-l, toc-l, chapter-l, content-l, end-l, blank-l).map(w => w + (name: "essentiel/" + w.name))
// Comme la couche de thème de l'app : les jetons de composant des packs, puis le thème.
#let packs = merge(pack-tokens("base", json("../packs/base/widgets.json")),
  pack-tokens("essentiel", json("../packs/essentiel/layouts.json")))
#render-deck(json("deck.json"), theme: merge(packs, papier), widgets: by-name(essentiel + base))
