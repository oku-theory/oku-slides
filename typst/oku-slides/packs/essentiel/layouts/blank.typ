// Slide vide : des éléments placés librement, sur une grille.

#import "/oku-slides/lib.typ": widget, blank-slide, free-area, default-grid, params as p

#let widget = widget("blank", category: "slide",
  params: (grid: p.grid(default: default-grid), elements: p.slot(mode: "free")),
  (args, ctx) => blank-slide(free-area(ctx.tokens, args.elements, grid: args.grid), id: ctx.id))
