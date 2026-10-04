// Widgets de base d'Oku Slides : éléments et slides. `registry` est lu par
// l'hôte (sonde <oku-widgets>) et par le rendu du deck.

#import "tokens.typ": *
#import "params.typ" as p
#import "widget.typ": widget, field, list-item, _is-empty
#import "content.typ": _text-args, _text-over
#import "slides.typ": card, slide, title-slide, chapter, toc, end-slide, blank-slide
#import "zone.typ": free-area, zone-marker, default-grid

#let _text = widget("text", params: (body: p.rich()), (args, ctx) => {
  let t = ctx.tokens
  let style = ctx.style
  set text(.._text-args(t, style))
  if "leading" in style { set par(leading: leading-of(t, style.leading)) }
  // Sans alignement choisi, le texte part du début de la ligne : à gauche,
  // ou à droite dans une langue écrite de droite à gauche.
  let al = style.at("align", default: "start")
  field(ctx, "body", style: _text-over(t, style), empty: args.body == [],
    align((left: left, center: center, right: right, start: start, end: end).at(al, default: start), args.body))
})

#let _card = widget("card",
  params: (kicker: p.plain(), title: p.plain(), body: p.rich(), highlight: p.flag()),
  (args, ctx) => card(
    title: args.title, kicker: args.kicker, highlight: args.highlight,
    // Dans un cadre (placement libre ou case de grille), la carte remplit sa place.
    fill: if ctx.framed { true } else { auto },
    wrap: (name, body, style) => field(ctx, name, body, style: style,
      empty: if name == "body" { args.body == [] } else { auto }),
    args.body,
  ))

#let _image-placeholder(t, asset) = rect(
  width: 100%, height: 100%, fill: color-of(t, "surface"), radius: radius-of(t, "sm"),
  align(center + horizon, text(size: size-of(t, "caption"), fill: color-of(t, "ink-muted"),
    "image : " + if asset == none { "?" } else { asset })),
)

#let _chart-placeholder(t) = rect(
  width: 100%, height: 100%, fill: color-of(t, "surface"), radius: radius-of(t, "sm"),
  align(center + horizon, text(size: size-of(t, "caption"), fill: color-of(t, "ink-muted"), "chart")),
)

#let _image = widget("image",
  params: (asset: p.asset(), fit: p.choice("cover", "contain", "stretch")),
  (args, ctx) => if ctx.load != none { (ctx.load)(asset: args.asset, fit: args.fit) }
    else { _image-placeholder(ctx.tokens, args.asset) })

// Rendu en SVG par l'hôte, chargé comme une image.
#let _chart = widget("chart", params: (option: p.host("chart")),
  (args, ctx) => if ctx.load != none { (ctx.load)(chart: true) } else { _chart-placeholder(ctx.tokens) })

#let _shape = widget("shape", params: (shape: p.choice("rect", "ellipse", "line")), (args, ctx) => {
  let t = ctx.tokens
  let style = ctx.style
  let fill = color-of(t, style.at("fill", default: none))
  let stroke = if "stroke" in style { stroke-of(t, style.at("stroke-width", default: "rule")) + color-of(t, style.stroke) }
  if args.shape == "ellipse" { ellipse(width: 100%, height: 100%, fill: fill, stroke: stroke) }
  else if args.shape == "line" { line(length: 100%, stroke: if stroke != none { stroke } else { stroke-of(t, "rule") + color-of(t, "rule") }) }
  else { rect(width: 100%, height: 100%, fill: fill, stroke: stroke, radius: radius-of(t, style.at("radius", default: "flat"))) }
})

#let _stack = widget("stack",
  params: (
    layout: p.choice("rows", "columns", "grid"),
    count: p.number(min: 1),
    gap: p.choice("xs", "sm", "md", "lg", "xl", "xxl", default: "lg"),
    children: p.slot(mode: "flow"),
  ),
  (args, ctx) => {
    let t = ctx.tokens
    let gap = space-of(t, args.gap)
    let bodies = args.children.map(c => c.body)
    if args.layout == "columns" or args.layout == "grid" {
      let n = if args.count == none { bodies.len() } else { args.count }
      grid(columns: (1fr,) * n, column-gutter: gap, row-gutter: gap, ..bodies)
    } else {
      stack(spacing: gap, ..bodies)
    }
  })

// Enfants placés sur leur cadre, relatif au canevas.
#let _canvas = widget("canvas", params: (children: p.slot(mode: "free")), (args, ctx) => {
  for c in args.children {
    let f = c.frame
    place(top + left, dx: f.x * 1pt, dy: f.y * 1pt, block(width: f.w * 1pt, height: f.h * 1pt, (c.show)(f)))
  }
})

// Frise chronologique : une colonne par étape (date, point, texte), une ligne
// d'accent passant par les points.
#let _frise = widget("frise", label: "Frise chronologique",
  params: (etapes: p.list(label: "Étapes", item-label: "étape", min: 0, max: 8,
    item: (date: p.plain(label: "Date"), texte: p.plain(label: "Étape", default: "Nouvelle étape")),
    default: ((date: "2019", texte: "Création"), (date: "2021", texte: "Premier client"),
      (date: "2023", texte: "International"), (date: "2025", texte: "Série B")))),
  (args, ctx) => {
    let t = ctx.tokens
    let accent = color-of(t, "accent")
    set text(size: size-of(t, "body"))
    let (date-h, gap, r) = (2.4em, 0.5em, 0.45em)
    // Un champ vide (étape tout juste ajoutée) garde une boîte d'une ligne,
    // mesurée et marquée vide : l'app y pose son placeholder et l'édite.
    let cell(c, name, v) = if v == none or v == "" {
      field(c, name, box(width: 3em, hide[0]), empty: true)
    } else { field(c, name, v) }
    // Une étape : date en gras au-dessus, point, texte ; centrée dans sa colonne.
    let step(date, texte) = block(width: 100%, align(center, stack(spacing: gap,
      box(height: date-h, align(bottom, text(weight: "bold", date))),
      circle(radius: r, fill: accent),
      texte)))
    block(width: 100%, {
      // La ligne passe par le centre des points : sous la date et l'espacement.
      place(top + left, dy: date-h + gap + r, line(length: 100%, stroke: 1.5pt + accent))
      if args.etapes.len() == 0 {
        // Sans étape, la frise garde la hauteur d'une rangée : l'élément reste
        // sélectionnable et la ligne passe là où seraient les points.
        hide(step("0", "0"))
      } else {
        // Chaque item occupe toute sa colonne : sa boîte couvre l'étape centrée.
        grid(columns: (1fr,) * args.etapes.len(), column-gutter: 0.5em,
          ..args.etapes.map(e => list-item(ctx, "etapes", e,
            c => step(cell(c, "date", e.at("date", default: none)), cell(c, "texte", e.at("texte", default: none))))))
      }
    })
  })

// Champs d'une slide : `wrap` des fonctions de slide, posé par le widget.
#let _fields(ctx) = (name, body, style) => field(ctx, name, body, style: style)

#let _title = widget("title", category: "slide",
  params: (
    title: p.plain(bind: "title"),
    subtitle: p.plain(bind: "subtitle"),
    author: p.plain(bind: "author"),
    date: p.plain(bind: "date"),
    kicker: p.plain(bind: "organization"),
  ),
  (args, ctx) => title-slide(title: args.title, subtitle: args.subtitle, author: args.author,
    date: args.date, kicker: args.kicker, id: ctx.id, wrap: _fields(ctx)))

#let _toc = widget("toc", category: "slide", params: (title: p.plain()),
  // Seul `title` est un paramètre : pas de champ pour le surtitre de `slide`.
  (args, ctx) => toc(title: if args.title == none { auto } else { args.title }, id: ctx.id,
    wrap: (name, body, style) => if name == "title" { field(ctx, name, body, style: style) } else { body }))

#let _chapter = widget("chapter", category: "slide", params: (title: p.plain(), subtitle: p.plain()),
  (args, ctx) => chapter(if args.title == none { "" } else { args.title }, subtitle: args.subtitle, id: ctx.id,
    // Un titre vide rend un en-tête vide (le sommaire en dépend) : champ mesuré, vide.
    wrap: (name, body, style) => field(ctx, name, body, style: style,
      empty: if name == "title" { _is-empty(args.title) } else { auto })))

#let _content = widget("content", category: "slide",
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
      title: args.title, kicker: args.kicker, layout: args.layout, id: ctx.id, wrap: _fields(ctx),
      // Un graphique dans les blocs occupe la place qui reste sous les autres.
      grow: if free { () } else { args.blocks.map(b => b.kind == "chart") },
      ..if free { (free-area(t, args.elements, grid: args.grid),) } else { args.blocks.map(b => b.body) },
    )
  })

#let _end = widget("end", category: "slide", params: (text: p.plain()),
  (args, ctx) => end-slide(if args.text == none { "" } else { args.text }, id: ctx.id,
    wrap: (name, body, style) => field(ctx, name, body, style: style, empty: _is-empty(args.text))))

#let _blank = widget("blank", category: "slide",
  params: (grid: p.grid(default: default-grid), elements: p.slot(mode: "free")),
  (args, ctx) => blank-slide(free-area(ctx.tokens, args.elements, grid: args.grid), id: ctx.id))

#let registry = (_text, _card, _image, _shape, _chart, _stack, _canvas, _frise,
  _title, _toc, _chapter, _content, _end, _blank)
