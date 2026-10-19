#import "../lib.typ": *
#import "../src/widgets.typ": registry as core
#import "../packs/base/widgets/text.typ": widget as text-w
#import "../packs/base/widgets/card.typ": widget as card-w
#import "../packs/base/widgets/image.typ": widget as image-w
#import "../packs/base/widgets/shape.typ": widget as shape-w
#import "../packs/base/widgets/chart.typ": widget as chart-w
#import "../packs/base/widgets/table.typ": widget as table-w
#import "../packs/base/widgets/stack.typ": widget as stack-w
// Comme l'assemblage généré par l'app : chaque nom préfixé par `base/`.
#let base = (text-w, card-w, image-w, shape-w, chart-w, table-w, stack-w).map(w => w + (name: "base/" + w.name))
#render-deck(json("deck.json"), theme: papier, widgets: by-name(core + base))
