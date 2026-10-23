// Couverture : titre, sous-titre, auteur, date et surtitre, liés aux paramètres du deck.

#import "/oku-slides/lib.typ": widget, params as p
#import "_commun.typ": fields, title-slide

#let widget = widget("title", category: "slide",
  params: (
    title: p.plain(bind: "title"),
    subtitle: p.plain(bind: "subtitle"),
    author: p.plain(bind: "author"),
    date: p.plain(bind: "date"),
    kicker: p.plain(bind: "organization"),
  ),
  (args, ctx) => title-slide(ctx, title: args.title, subtitle: args.subtitle, author: args.author,
    date: args.date, kicker: args.kicker, id: ctx.id, wrap: fields(ctx)))
