// Graphique rendu en SVG par l'hôte, chargé comme une image.

#import "/oku-slides/lib.typ": widget, params as p
#import "_commun.typ": placeholder

#let widget = widget("chart", grow: true, params: (option: p.host("chart")),
  (args, ctx) => if ctx.load != none { (ctx.load)(chart: true) } else { placeholder(ctx.tokens, "chart") })
