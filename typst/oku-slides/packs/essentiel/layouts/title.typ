// Couverture : titre, sous-titre, auteur, date et surtitre, liés aux paramètres du deck.

#import "/oku-slides/lib.typ": widget, title-slide, params as p
#import "_commun.typ": fields

#let widget = widget("title", category: "slide",
  params: (
    title: p.plain(bind: "title"),
    subtitle: p.plain(bind: "subtitle"),
    author: p.plain(bind: "author"),
    date: p.plain(bind: "date"),
    kicker: p.plain(bind: "organization"),
  ),
  (args, ctx) => title-slide(title: args.title, subtitle: args.subtitle, author: args.author,
    date: args.date, kicker: args.kicker, id: ctx.id, wrap: fields(ctx)))
