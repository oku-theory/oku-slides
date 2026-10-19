// Carte : surtitre, titre, corps, mise en avant (`card` de la bibliothèque).

#import "/oku-slides/lib.typ": widget, field, card, params as p

#let widget = widget("card",
  params: (kicker: p.plain(), title: p.plain(), body: p.rich(), highlight: p.flag()),
  (args, ctx) => card(
    title: args.title, kicker: args.kicker, highlight: args.highlight,
    // Dans un cadre (placement libre ou case de grille), la carte remplit sa place.
    fill: if ctx.framed { true } else { auto },
    wrap: (name, body, style) => field(ctx, name, body, style: style,
      empty: if name == "body" { args.body == [] } else { auto }),
    args.body,
  ))
