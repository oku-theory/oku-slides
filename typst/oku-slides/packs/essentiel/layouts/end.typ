// Slide de fin : un texte centré.

#import "/oku-slides/lib.typ": widget, field, end-slide, _is-empty, params as p

#let widget = widget("end", category: "slide", params: (text: p.plain()),
  (args, ctx) => end-slide(if args.text == none { "" } else { args.text }, id: ctx.id,
    wrap: (name, body, style) => field(ctx, name, body, style: style, empty: _is-empty(args.text))))
