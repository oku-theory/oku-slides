// Slide de contenu : surtitre, titre, et des blocs mis en page ou des éléments libres.

#import "/oku-slides/lib.typ": widget, slide, free-area, zone-marker, default-grid, params as p
#import "_commun.typ": fields

#let widget = widget("content", category: "slide",
  params: (
    kicker: p.plain(),
    title: p.plain(),
    layout: p.choice("default", "columns", "split", "center", "quote"),
    grid: p.grid(default: default-grid),
    blocks: p.slot(mode: "flow"),
    elements: p.slot(mode: "free"),
  ),
  (args, ctx) => {
    let t = ctx.tokens
    // Sans blocs mis en page, la zone de corps est libre, sur sa grille.
    let free = args.blocks.len() == 0
    slide(
      // Slide à blocs : la grille existe aussi (l'app y accroche un bloc
      // dès qu'on le déplace, et la slide passe alors sur la grille).
      zone: if free { none } else { zone-marker(t, args.grid) },
      title: args.title, kicker: args.kicker, layout: args.layout, id: ctx.id, wrap: fields(ctx),
      // Un bloc prend la hauteur selon le `grow` de son widget (voir `_layout`).
      grow: if free { () } else { args.blocks.map(b => b.grow) },
      ..if free { (free-area(t, args.elements, grid: args.grid),) } else { args.blocks.map(b => b.body) },
    )
  })
