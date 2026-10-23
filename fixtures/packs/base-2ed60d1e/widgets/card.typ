// Carte : surtitre, titre, corps, mise en avant ; lit les jetons `card` du pack.

#import "/oku-slides/lib.typ": widget, field, config, fields-stack, _is-empty, styled, text-style, params as p
#import "/oku-slides/lib.typ": color-of, size-of, space-of, weight-of, radius-of, stroke-of, list-marker

/// Carte : surtitre, titre et corps, avec filet d'accent en haut.
/// highlight: true passe la carte sur le fond sombre du thème.
/// fill : `auto` suit le jeton card.height ; `true` remplit la hauteur (case de grille, cadre).
/// wrap : (nom, contenu, style) => contenu ; le widget y pose ses champs.
#let card(ctx, title: none, kicker: none, highlight: false, fill: auto, wrap: (name, body, style) => body, body) = context {
  let t = config().tokens
  let k = ctx.component.card
  let bg = color-of(t, if highlight { k.highlight-bg } else { k.bg })
  let ink = if highlight { color-of(t, k.highlight-ink) } else { none }
  let border = if highlight { none } else { stroke-of(t, k.border-width) + color-of(t, k.border-color) }
  block(
    width: 100%, height: if fill == true or (fill == auto and k.height == "fill") { 100% } else { auto },
    fill: bg, radius: radius-of(t, k.radius), clip: true,
    stroke: border,
    {
      place(top + left, rect(width: 100%, height: k.top-rule-width * 1pt, fill: color-of(t, k.top-rule-color)))
      block(inset: (x: space-of(t, k.padding-x), y: space-of(t, k.padding-y)), {
        set text(fill: ink) if ink != none
        let lst = t.list
        set list(marker: list-marker(t, lst.marker, if highlight { k.highlight-kicker } else { lst.at("marker-color", default: none) }))
        let kicker-c = t.kicker + if highlight { (color: k.highlight-kicker) } else { (:) }
        let title-style = (size: size-of(t, k.title-size), weight: weight-of(t, k.title-weight),
          fill: color-of(t, if highlight { k.highlight-title } else { k.title-color }))
        fields-stack(spacing: space-of(t, k.gap),
          wrap("kicker", if not _is-empty(kicker) { styled(t, kicker-c, kicker) },
            text-style(t, kicker-c) + (case: kicker-c.at("case", default: none))),
          wrap("title", if not _is-empty(title) { text(..title-style, title) }, title-style),
          wrap("body", body, if ink != none { (fill: ink) } else { (:) }),
        )
      })
    },
  )
}

#let widget = widget("card",
  params: (kicker: p.plain(), title: p.plain(), body: p.rich(), highlight: p.flag()),
  (args, ctx) => card(ctx,
    title: args.title, kicker: args.kicker, highlight: args.highlight,
    // Dans un cadre (placement libre ou case de grille), la carte remplit sa place.
    fill: if ctx.framed { true } else { auto },
    wrap: (name, body, style) => field(ctx, name, body, style: style,
      empty: if name == "body" { args.body == [] } else { auto }),
    args.body,
  ))
