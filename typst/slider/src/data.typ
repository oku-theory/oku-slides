// Rendu d'un deck décrit en JSON (le modèle de données de l'app Oku Slides).
//
// {
//   "params": { "title": "…", "author": "…", "date": "…" },
//   "vars":   { "client": "Acme" },
//   "tokens": { "color": { "accent": "#8a5a1c" } },
//   "slides": [
//     { "kind": "title" },
//     { "kind": "toc" },
//     { "kind": "chapter", "title": "Contexte" },
//     { "kind": "content", "title": "…", "layout": "columns", "blocks": [ … ] },
//     { "kind": "content", "title": "…", "elements": [ { "frame": {…}, … } ] },
//     { "kind": "end", "text": "Merci" }
//   ]
// }
//
// Deux façons de remplir une slide de contenu :
// - "blocks" : une liste d'éléments mis en page par le layout de la slide ;
// - "elements" : des éléments placés librement, chacun avec un "frame"
//   { x, y, w, h } en pt, relatif à la zone de corps (sous le titre).
// Chaque élément porte un "id" ; il est étiqueté <e-ID> pour l'introspection.

#import "tokens.typ": *
#import "core.typ": config, interp, text-probe
#import "slides.typ": slide, title-slide, chapter, toc, end-slide, blank-slide, deck, card

// Taille de la zone de corps d'une slide de contenu avec titre.
#let body-size(t, with-title: true) = {
  let pg = t.page
  let w = (pg.width - 2 * pg.margin-x) * 1pt
  let h = (pg.height - pg.margin-top - pg.margin-bottom) * 1pt
  (width: w, height: h)
}

#let _text-args(t, style) = {
  let a = (:)
  if "size" in style { a.insert("size", size-of(t, style.size)) }
  if "color" in style { a.insert("fill", color-of(t, style.color)) }
  if "weight" in style { a.insert("weight", weight-of(t, style.weight)) }
  if "font" in style { a.insert("font", font-of(t, style.font)) }
  if style.at("italic", default: false) { a.insert("style", "italic") }
  a
}

#let _runs(t, runs, vars) = runs.map(r => {
  let body = [#interp(r.at("t", default: ""), vars)]
  let a = _text-args(t, r)
  if r.at("bold", default: false) { a.insert("weight", weight-of(t, "strong")) }
  if a.len() > 0 { text(..a, body) } else { body }
}).join()

#let _paragraphs(t, paras, vars) = {
  let items = ()
  let out = ()
  for p in paras {
    let body = _runs(t, p.at("runs", default: ()), vars)
    let kind = p.at("list", default: none)
    if kind == "bullet" { out.push(list(body)) }
    else if kind == "number" { out.push(enum(body)) }
    else { out.push(par(body)) }
  }
  out.join()
}

#let _image-placeholder(t, e) = rect(
  width: 100%, height: 100%, fill: color-of(t, "surface"), radius: radius-of(t, "sm"),
  align(center + horizon, text(size: size-of(t, "caption"), fill: color-of(t, "ink-muted"),
    "image : " + e.at("asset", default: "?"))),
)

#let _chart-placeholder(t) = rect(
  width: 100%, height: 100%, fill: color-of(t, "surface"), radius: radius-of(t, "sm"),
  align(center + horizon, text(size: size-of(t, "caption"), fill: color-of(t, "ink-muted"), "chart")),
)

#let render-element(t, e, vars, image-loader: none) = {
  let kind = e.at("kind", default: "text")
  let style = e.at("style", default: (:))
  let el = if kind == "text" {
    set text(.._text-args(t, style))
    if "leading" in style { set par(leading: leading-of(t, style.leading)) }
    // Sans alignement choisi, le texte part du début de la ligne : à gauche,
    // ou à droite dans une langue écrite de droite à gauche.
    let al = style.at("align", default: "start")
    let body = if "text" in e { _paragraphs(t, e.text, vars) } else { [] }
    align((left: left, center: center, right: right, start: start, end: end).at(al, default: start), body)
  } else if kind == "image" {
    if image-loader != none { image-loader(e) } else { _image-placeholder(t, e) }
  } else if kind == "chart" {
    // Rendu en SVG par l'hôte, chargé comme une image.
    if image-loader != none { image-loader(e) } else { _chart-placeholder(t) }
  } else if kind == "shape" {
    let shape = e.at("shape", default: "rect")
    let fill = color-of(t, style.at("fill", default: none))
    let stroke = if "stroke" in style { stroke-of(t, style.at("stroke-width", default: "rule")) + color-of(t, style.stroke) }
    if shape == "ellipse" { ellipse(width: 100%, height: 100%, fill: fill, stroke: stroke) }
    else if shape == "line" { line(length: 100%, stroke: if stroke != none { stroke } else { stroke-of(t, "rule") + color-of(t, "rule") }) }
    else { rect(width: 100%, height: 100%, fill: fill, stroke: stroke, radius: radius-of(t, style.at("radius", default: "flat"))) }
  } else if kind == "card" {
    card(
      title: interp(e.at("title", default: none), vars),
      kicker: interp(e.at("kicker", default: none), vars),
      highlight: e.at("highlight", default: false),
      // Dans un cadre (placement libre ou case de grille), la carte remplit sa place.
      fill: if "frame" in e { true } else { auto },
      if "text" in e { _paragraphs(t, e.text, vars) } else { [] },
    )
  } else if kind == "group" {
    let children = e.at("children", default: ())
    let lay = e.at("layout", default: (type: "free"))
    let gap = space-of(t, lay.at("gap", default: "lg"))
    if lay.type == "columns" or lay.type == "grid" {
      let n = lay.at("count", default: children.len())
      grid(columns: (1fr,) * n, column-gutter: gap, row-gutter: gap,
        ..children.map(c => render-element(t, c, vars, image-loader: image-loader)))
    } else if lay.type == "rows" {
      stack(spacing: gap, ..children.map(c => render-element(t, c, vars, image-loader: image-loader)))
    } else {
      for c in children {
        let f = c.frame
        place(top + left, dx: f.x * 1pt, dy: f.y * 1pt,
          block(width: f.w * 1pt, height: f.h * 1pt, render-element(t, c, vars, image-loader: image-loader)))
      }
    }
  } else {
    text(fill: red)[slider : élément inconnu « #kind »]
  }
  let fixed = "frame" in e
  let (w, h) = if fixed { (e.frame.w * 1pt, e.frame.h * 1pt) } else { (100%, auto) }
  if "id" not in e { return block(width: w, height: h, el) }
  // Boîte de l'élément pour l'app : page, position et taille réelles.
  let sized = block(width: w, height: h, layout(sz => {
    // Un graphique s'adapte à la hauteur disponible : il se mesure dans celle-ci.
    let real-h = if fixed { sz.height } else if kind == "chart" and sz.height < float.inf * 1pt {
      measure(block(width: sz.width, el), height: sz.height).height
    } else { measure(block(width: sz.width, el)).height }
    context {
      let p = here().position()
      let over = if kind == "text" {
        let o = _text-args(t, style)
        if "leading" in style { o.insert("leading", leading-of(t, style.leading)) }
        o.insert("align", (left: left, center: center, right: right, start: start, end: end).at(style.at("align", default: "start"), default: start))
        o
      } else { (:) }
      [#metadata((id: str(e.id), page: p.page, x: p.x / 1pt, y: p.y / 1pt,
        w: sz.width / 1pt, h: real-h / 1pt, ax: p.x / 1pt, aw: sz.width / 1pt, style: text-probe(over)))<slider-box>]
    }
    el
  }))
  [#sized#label("e-" + str(e.id))]
}

// Géométrie d'une grille : cadre (pt, relatif à la zone) d'une cellule.
// Même calcul que ui/src/grid.ts : les deux doivent rester alignés.
#let grid-frame(zone-w, zone-h, g, gap, cell) = {
  let cw = (zone-w - (g.columns - 1) * gap) / g.columns
  let rh = (zone-h - (g.rows - 1) * gap) / g.rows
  let cols = cell.at("cols", default: 1)
  let rows = cell.at("rows", default: 1)
  (
    x: cell.col * (cw + gap),
    y: cell.row * (rh + gap),
    w: cols * cw + (cols - 1) * gap,
    h: rows * rh + (rows - 1) * gap,
  )
}

// Zone des éléments libres. Avec une grille, les éléments qui ont une
// « cell » y sont accrochés ; les autres gardent leur « frame ».
// La zone est signalée à l'app (<slider-zone>) : position, taille, grille.
// Grille de toute slide qui n'en précise pas (même valeur que Grid::DEFAULT).
#let default-grid = (columns: 12, rows: 6)

// Signale la zone de corps à l'app : position, taille et grille résolue.
#let zone-marker(t, grid) = layout(sz => context {
  let p = here().position()
  let gap = space-of(t, grid.at("gap", default: "lg")) / 1pt
  [#metadata((page: p.page, x: p.x / 1pt, y: p.y / 1pt, w: sz.width / 1pt, h: sz.height / 1pt,
    grid: (columns: grid.columns, rows: grid.rows, gap: gap)))<slider-zone>]
})

#let _free(t, elements, vars, image-loader, grid: none) = block(width: 100%, height: 100%, layout(sz => {
  let zw = sz.width / 1pt
  let zh = sz.height / 1pt
  let gap = if grid != none { space-of(t, grid.at("gap", default: "lg")) / 1pt } else { 0 }
  if grid != none { place(top + left, zone-marker(t, grid)) }
  for e in elements {
    let f = if grid != none and "cell" in e { grid-frame(zw, zh, grid, gap, e.cell) } else { e.frame }
    let e = e + (frame: f)
    place(top + left, dx: f.x * 1pt, dy: f.y * 1pt, render-element(t, e, vars, image-loader: image-loader))
  }
}))

#let render-slide(s, image-loader: none) = context {
  let c = config()
  let t = c.tokens
  let vars = c.vars
  let kind = s.at("kind", default: "content")
  let opt(key) = s.at(key, default: auto)
  let id = s.at("id", default: none)
  if kind == "title" {
    title-slide(title: opt("title"), subtitle: opt("subtitle"), author: opt("author"), date: opt("date"), id: id)
  } else if kind == "chapter" {
    chapter(s.title, subtitle: s.at("subtitle", default: none), id: id)
  } else if kind == "toc" {
    toc(title: opt("title"), id: id)
  } else if kind == "end" {
    end-slide(interp(s.at("text", default: ""), vars), id: id)
  } else if kind == "blank" {
    blank-slide(_free(t, s.at("elements", default: ()), vars, image-loader, grid: s.at("grid", default: default-grid)), id: id)
  } else {
    // Sans blocs mis en page, la zone de corps est libre, sur sa grille.
    let bodies = if "elements" in s or "grid" in s or s.at("blocks", default: ()).len() == 0 {
      (_free(t, s.at("elements", default: ()), vars, image-loader, grid: s.at("grid", default: default-grid)),)
    } else {
      s.at("blocks", default: ()).map(b => render-element(t, b, vars, image-loader: image-loader))
    }
    // Slide à blocs mis en page : la grille existe aussi (l'app y accroche un
    // bloc dès qu'on le déplace, et la slide passe alors sur la grille).
    let zone = if "elements" in s or "grid" in s or s.at("blocks", default: ()).len() == 0 { none } else {
      zone-marker(t, s.at("grid", default: default-grid))
    }
    slide(
      zone: zone,
      title: s.at("title", default: none),
      kicker: s.at("kicker", default: none),
      layout: s.at("layout", default: "default"),
      id: id,
      // Un graphique dans les blocs occupe la place qui reste sous les autres.
      grow: if zone == none { () } else { s.at("blocks", default: ()).map(b => b.at("kind", default: "text") == "chart") },
      ..bodies,
    )
  }
}

/// Rend un deck JSON complet : `#render-deck(json("deck.json"))`.
/// image-loader : fonction (élément) => contenu, pour les images et les graphiques.
#let render-deck(data, theme: none, image-loader: none) = {
  let params = data.at("params", default: (:))
  show: deck.with(
    theme: theme,
    tokens: data.at("tokens", default: (:)),
    vars: data.at("vars", default: (:)),
    ..params,
  )
  for s in data.at("slides", default: ()) { render-slide(s, image-loader: image-loader) }
}
