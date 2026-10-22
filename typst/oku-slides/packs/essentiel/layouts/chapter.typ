// Ouverture de chapitre : titre et sous-titre ; le titre entre au sommaire.

#import "/oku-slides/lib.typ": widget, field, chapter, _is-empty, params as p

#let widget = widget("chapter", category: "slide", params: (title: p.plain(), subtitle: p.plain()),
  (args, ctx) => chapter(if args.title == none { "" } else { args.title }, subtitle: args.subtitle, id: ctx.id,
    // Un titre vide rend un en-tête vide (le sommaire en dépend) : champ mesuré, vide.
    wrap: (name, body, style) => field(ctx, name, body, style: style,
      empty: if name == "title" { _is-empty(args.title) } else { auto })))
