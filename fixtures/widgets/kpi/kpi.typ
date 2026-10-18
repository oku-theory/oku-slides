// Un indicateur : une valeur en grand, sa légende dessous.
#import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of
#import "_commun.typ": carte

#let widget = widget("kpi", label: "KPI", description: "Une valeur en grand, sa légende dessous",
  params: (value: p.plain(default: "42 %"), label: p.plain(), trend: p.choice("up", "down", "flat")),
  (args, ctx) => carte(ctx.tokens, fields-stack(spacing: 8pt,
    field(ctx, "value", { set text(size: size-of(ctx.tokens, "display"), fill: color-of(ctx.tokens, "accent")); args.value }),
    field(ctx, "label", { set text(size: size-of(ctx.tokens, "body"), fill: color-of(ctx.tokens, "ink-muted")); args.label }),
  )))
