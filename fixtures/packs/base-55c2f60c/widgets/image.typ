// Image d'un asset du deck, chargée par l'hôte ; dans le flux, à ses proportions.

#import "/oku-slides/lib.typ": widget, params as p
#import "_commun.typ": placeholder

#let widget = widget("image", grow: true,
  params: (asset: p.asset(), fit: p.choice("cover", "contain", "stretch")),
  (args, ctx) => if ctx.load != none { (ctx.load)(asset: args.asset, fit: args.fit, flow: not ctx.framed) }
    else { placeholder(ctx.tokens, "image : " + if args.asset == none { "?" } else { args.asset }) })
