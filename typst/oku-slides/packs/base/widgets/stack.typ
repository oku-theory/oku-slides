// Pile d'éléments du flux, en lignes, en colonnes ou en grille.

#import "/oku-slides/lib.typ": widget, space-of, params as p

#let widget = widget("stack",
  params: (
    layout: p.choice("rows", "columns", "grid"),
    count: p.number(min: 1),
    gap: p.choice("xs", "sm", "md", "lg", "xl", "xxl", default: "lg"),
    children: p.slot(mode: "flow"),
  ),
  (args, ctx) => {
    let t = ctx.tokens
    let gap = space-of(t, args.gap)
    let bodies = args.children.map(c => c.body)
    if args.layout == "columns" or args.layout == "grid" {
      let n = if args.count == none { bodies.len() } else { args.count }
      grid(columns: (1fr,) * n, column-gutter: gap, row-gutter: gap, ..bodies)
    } else {
      stack(spacing: gap, ..bodies)
    }
  })
