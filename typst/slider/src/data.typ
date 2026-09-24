// Rendu d'un deck décrit en JSON (le modèle de données de l'app Slider).
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
#import "core.typ": config, interp
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

#let render-element(t, e, vars, image-loader: none) = {
  let kind = e.at("kind", default: "text")
  let style = e.at("style", default: (:))
  let el = if kind == "text" {
    set text(.._text-args(t, style))
    if "leading" in style { set par(leading: leading-of(t, style.leading)) }
    let al = style.at("align", default: "left")
    let body = if "text" in e { _paragraphs(t, e.text, vars) } else { [] }
    align((left: left, center: center, right: right).at(al, default: left), body)
  } else if kind == "image" {
    if image-loader != none { image-loader(e) } else { _image-placeholder(t, e) }
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
    let real-h = if fixed { sz.height } else { measure(block(width: sz.width, el)).height }
    context {
      let p = here().position()
      [#metadata((id: str(e.id), page: p.page, x: p.x / 1pt, y: p.y / 1pt,
        w: sz.width / 1pt, h: real-h / 1pt))<slider-box>]
    }
    el
  }))
  [#sized#label("e-" + str(e.id))]
}

#let _free(t, elements, vars, image-loader) = block(width: 100%, height: 100%, {
  for e in elements {
    let f = e.frame
    place(top + left, dx: f.x * 1pt, dy: f.y * 1pt, render-element(t, e, vars, image-loader: image-loader))
  }
})

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
    blank-slide(_free(t, s.at("elements", default: ()), vars, image-loader), id: id)
  } else {
    let bodies = if "elements" in s {
      (_free(t, s.elements, vars, image-loader),)
    } else {
      s.at("blocks", default: ()).map(b => render-element(t, b, vars, image-loader: image-loader))
    }
    slide(
      title: s.at("title", default: none),
      kicker: s.at("kicker", default: none),
      layout: s.at("layout", default: "default"),
      id: id,
      ..bodies,
    )
  }
}

/// Rend un deck JSON complet : `#render-deck(json("deck.json"))`.
/// image-loader : fonction (élément) => contenu, pour charger les assets.
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
