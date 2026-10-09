// Widgets de base d'Oku Slides : éléments et slides. `registry` est lu par
// l'hôte (sonde <oku-widgets>) et par le rendu du deck.

#import "tokens.typ": *
#import "params.typ" as p
#import "widget.typ": widget, field, _is-empty
#import "content.typ": _text-args, _text-over
#import "slides.typ": card, slide, title-slide, chapter, toc, end-slide, blank-slide
#import "zone.typ": free-area, zone-marker, default-grid
#import "numbers.typ": format-number, deck-lang

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

// Tableau rendu en Typst. `data` : options de `oku-slides-chart::TableOption`,
// déjà ramenées à leur forme canonique par le rendu (render/src/tables.rs) ;
// les gardes ci-dessous protègent un appel direct (aperçu, pack).
#let _table-max-rows = 100     // MAX_TABLE_ROWS
#let _table-max-columns = 20   // MAX_TABLE_COLUMNS

#let _table-placeholder(t, framed) = rect(
  width: 100%, height: if framed { 100% } else { 6em }, fill: color-of(t, "surface"), radius: radius-of(t, "sm"),
  align(center + horizon, text(size: size-of(t, "caption"), fill: color-of(t, "ink-muted"), "table")),
)

// Texte d'une cellule : nombre écrit dans la langue `lang` du deck (entier
// sans décimales), case cochée en ✓, autre type vide.
#let _cell-text(v, lang) = {
  if type(v) == str { v }
  else if type(v) == bool { if v { "✓" } else { "" } }
  else if type(v) == int or type(v) == float { format-number(v, lang) }
  else { "" }
}

// Mot insécable : un idéogramme ou un kana (suivi de sa ponctuation), ou une
// suite sans espace. Plus long que ce que Typst coupe vraiment (traits
// d'union, barres obliques, thaï) : la largeur minimale est surestimée, jamais
// sous-estimée.
#let _table-word = regex("[\\p{Han}\\p{Hiragana}\\p{Katakana}]\\p{P}*|[^\\s\\p{Han}\\p{Hiragana}\\p{Katakana}]+")

#let _numeric(v) = v == none or type(v) == int or type(v) == float

// Le drapeau `key` de `d` s'il est booléen, sinon `default`.
#let _flag(d, key, default) = {
  let v = d.at(key, default: default)
  if type(v) == bool { v } else { default }
}

#let _table = widget("table", params: (data: p.host("table")), (args, ctx) => context {
  let t = ctx.tokens
  let lang = deck-lang()
  let d = args.data
  let dataset = if type(d) == dictionary { d.at("dataset", default: none) } else { none }
  let source = if type(dataset) == dictionary { dataset.at("source", default: none) } else { none }
  let ok = (type(source) == array and source.len() > 0 and source.len() <= _table-max-rows
    and source.all(r => type(r) == array) and source.first().len() > 0
    and source.first().len() <= _table-max-columns)
  if not ok { return _table-placeholder(t, ctx.framed) }
  let k = t.component.table
  let n = source.first().len()
  let rows = source.map(r => r.slice(0, calc.min(n, r.len())) + (none,) * calc.max(0, n - r.len()))
  let head-row = _flag(d, "header-row", true)
  let head-col = _flag(d, "header-column", false)
  let striped = _flag(d, "striped", false)
  let aligns = d.at("align", default: ())
  let aligns = if type(aligns) == array { aligns } else { () }
  let first = if head-row { 1 } else { 0 }
  let data-rows = rows.slice(first)
  // Colonne de nombres : que des nombres ou des vides, au moins un nombre.
  let numeric = range(n).map(x => data-rows.all(r => _numeric(r.at(x))) and data-rows.any(r => r.at(x) != none))
  // `auto` : à la fin pour une colonne de nombres, au début sinon.
  let col-aligns = range(n).map(x => {
    let a = aligns.at(x, default: "auto")
    if a == "start" { start } else if a == "center" { center } else if a == "end" { end }
    else if numeric.at(x) { end } else { start }
  })
  let header-bg = color-of(t, k.header-bg)
  let stripe-bg = color-of(t, k.stripe-bg)
  let header-color = color-of(t, k.header-color)
  let header-weight = weight-of(t, k.header-weight)
  // Style d'une cellule, repris pour mesurer ses mots.
  let styled-cell(x, y, body) = {
    if head-row and y == 0 { text(weight: header-weight, fill: header-color, body) }
    else if head-col and x == 0 { text(weight: header-weight, body) }
    else { body }
  }
  let texts = rows.map(r => r.map(v => _cell-text(v, lang)))
  let cells = texts.enumerate().map(((y, r)) => r.enumerate().map(((x, body)) => styled-cell(x, y, body))).flatten()
  let rule = stroke-of(t, k.rule-width) + color-of(t, k.rule-color)
  let pad-x = space-of(t, k.padding-x)
  let tbl(columns) = table(
    columns: columns,
    inset: (x: pad-x, y: space-of(t, k.padding-y)),
    align: (x, y) => col-aligns.at(x) + horizon,
    fill: (x, y) => if head-row and y == 0 { header-bg }
      else if striped and calc.odd(y - first) { stripe-bg } else { none },
    stroke: (x, y) => (bottom: rule),
    ..if head-row { (table.header(..cells.slice(0, n)),) } else { () },
    ..cells.slice(first * n),
  )
  // Largeur d'une colonne : celle de son plus long contenu (`full`) ou de son
  // plus long mot insécable (sinon), marges comprises.
  let col-width(x, full) = 2 * pad-x + 0.1pt + calc.max(0pt, ..range(rows.len()).map(y => {
    let body = texts.at(y).at(x)
    let words = if full { (body,) } else { body.matches(_table-word).map(m => m.text) }
    calc.max(0pt, ..words.map(w => measure(styled-cell(x, y, w)).width))
  }))
  set text(size: size-of(t, k.size))
  // Tient sans retour à la ligne : toute la largeur, l'excédent donné aux
  // colonnes de texte au prorata de leur largeur naturelle (à toutes s'il n'y
  // en a pas), les nombres restent serrés. Sinon, chaque colonne a au moins la
  // largeur de son plus long mot, le reste réparti selon ce qui lui manque
  // pour tenir sur une ligne ; trop large même ainsi, le tableau est posé à
  // cette largeur minimale puis réduit. Pas de pagination.
  layout(size => {
    let natural = range(n).map(x => col-width(x, true))
    let (columns, width) = if natural.sum() <= size.width {
      let grows = if numeric.all(v => v) { range(n) } else { range(n).filter(x => not numeric.at(x)) }
      let share = grows.map(x => natural.at(x)).sum()
      let spare = size.width - natural.sum()
      (range(n).map(x => natural.at(x) + if x in grows { spare * (natural.at(x) / share) } else { 0pt }), size.width)
    } else {
      let least = range(n).map(x => calc.min(col-width(x, false), natural.at(x)))
      let spare = size.width - least.sum()
      let want = natural.sum() - least.sum()
      if spare <= 0pt or want <= 0pt { (least, least.sum()) }
      else { (range(n).map(x => least.at(x) + spare * ((natural.at(x) - least.at(x)) / want)), size.width) }
    }
    let body = block(width: width, tbl(columns))
    let m = measure(body)
    if m.width <= 0pt or m.height <= 0pt { return body }
    let fit-w = size.width / width
    let fit-h = if size.height == float.inf * 1pt { 1 } else { size.height / m.height }
    let s = calc.max(0.01, calc.min(1, fit-w, fit-h))
    if s >= 1 { body } else { scale(s * 100%, reflow: true, body) }
  })
})

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
      // Un graphique dans les blocs occupe la place qui reste sous les autres ;
      // un tableau trop haut la partage avec lui (voir `_layout`).
      grow: if free { () } else { args.blocks.map(b =>
        if b.kind == "chart" { true } else if b.kind == "table" { "fit" } else { false }) },
      ..if free { (free-area(t, args.elements, grid: args.grid),) } else { args.blocks.map(b => b.body) },
    )
  })

#let _end = widget("end", category: "slide", params: (text: p.plain()),
  (args, ctx) => end-slide(if args.text == none { "" } else { args.text }, id: ctx.id,
    wrap: (name, body, style) => field(ctx, name, body, style: style, empty: _is-empty(args.text))))

#let _blank = widget("blank", category: "slide",
  params: (grid: p.grid(default: default-grid), elements: p.slot(mode: "free")),
  (args, ctx) => blank-slide(free-area(ctx.tokens, args.elements, grid: args.grid), id: ctx.id))

#let registry = (_text, _card, _image, _shape, _chart, _table, _stack, _canvas,
  _title, _toc, _chapter, _content, _end, _blank)
