// Sommaire : la liste des chapitres du deck, sous un titre.

#import "/oku-slides/lib.typ": widget, field, toc, params as p

#let widget = widget("toc", category: "slide", params: (title: p.plain()),
  // Seul `title` est un paramètre : pas de champ pour le surtitre de `slide`.
  (args, ctx) => toc(title: if args.title == none { auto } else { args.title }, id: ctx.id,
    wrap: (name, body, style) => if name == "title" { field(ctx, name, body, style: style) } else { body }))
