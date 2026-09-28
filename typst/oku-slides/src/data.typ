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
#import "slides.typ": deck
#import "content.typ": _text-over, _paragraphs
#import "zone.typ": default-grid
#import "widgets.typ": registry
#import "widget.typ": by-name

#let _widgets = by-name(registry)

// Arguments d'un élément v1 pour son widget. MVP: t-270, plan 2 — remplacé
// par le rendu générique du modèle v2.
#let _element-args(t, e, vars, image-loader, render) = {
  let kind = e.at("kind", default: "text")
  let body = if "text" in e { _paragraphs(t, e.text, vars) } else { [] }
  if kind == "text" { (body: body) }
  else if kind == "card" { (
    kicker: interp(e.at("kicker", default: none), vars),
    title: interp(e.at("title", default: none), vars),
    highlight: e.at("highlight", default: false),
    body: body,
  ) }
  else if kind == "image" { (asset: e.at("asset", default: none), fit: e.at("fit", default: "cover")) }
  else if kind == "chart" { (option: e.at("option", default: none)) }
  else if kind == "shape" { (shape: e.at("shape", default: "rect")) }
  else if kind == "group" {
    let children = e.at("children", default: ())
    let lay = e.at("layout", default: (type: "free"))
    if lay.type == "free" {
      (children: children.map(c => (kind: c.at("kind", default: "text"), id: c.at("id", default: none),
        frame: c.frame, cell: none, "show": f => render(t, c, vars, image-loader: image-loader))))
    } else {
      (layout: lay.type, count: lay.at("count", default: none), gap: lay.at("gap", default: "lg"),
        children: children.map(c => (kind: c.at("kind", default: "text"), id: c.at("id", default: none),
          body: render(t, c, vars, image-loader: image-loader))))
    }
  }
  else { (:) }
}

// Widget qui rend un élément v1 : le groupe se scinde en canvas et stack.
#let _widget-of(e) = {
  let kind = e.at("kind", default: "text")
  if kind != "group" { kind }
  else if e.at("layout", default: (type: "free")).type == "free" { "canvas" } else { "stack" }
}

// Taille de la zone de corps d'une slide de contenu avec titre.
#let body-size(t, with-title: true) = {
  let pg = t.page
  let w = (pg.width - 2 * pg.margin-x) * 1pt
  let h = (pg.height - pg.margin-top - pg.margin-bottom) * 1pt
  (width: w, height: h)
}

#let render-element(t, e, vars, image-loader: none) = {
  let kind = e.at("kind", default: "text")
  let style = e.at("style", default: (:))
  let ctx = (id: e.at("id", default: none), tokens: t, vars: vars, style: style, framed: "frame" in e,
    load: if image-loader != none { () => image-loader(e) } else { none })
  let w0 = _widgets.at(_widget-of(e), default: none)
  // Un widget existant mais qui n'est pas de catégorie "element" (p. ex.
  // une slide comme "title") ne rend pas ici : boîte d'élément inconnu.
  let w = if w0 != none and w0.schema.category == "element" { w0 } else { none }
  let el = if w == none { text(fill: red)[oku-slides : élément inconnu « #kind »] }
    else { (w.render)(_element-args(t, e, vars, image-loader, render-element), ctx) }
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
      let over = if kind == "text" { _text-over(t, style) } else { (:) }
      [#metadata((id: str(e.id), page: p.page, x: p.x / 1pt, y: p.y / 1pt,
        w: sz.width / 1pt, h: real-h / 1pt, ax: p.x / 1pt, aw: sz.width / 1pt, style: text-probe(over)))<oku-slides-box>]
    }
    el
  }))
  [#sized#label("e-" + str(e.id))]
}

// Entrées de slot d'une liste d'éléments v1.
#let _flow(t, list, vars, image-loader) = list.map(b => (kind: b.at("kind", default: "text"),
  id: b.at("id", default: none), body: render-element(t, b, vars, image-loader: image-loader)))
#let _placed(t, list, vars, image-loader) = list.map(e => (kind: e.at("kind", default: "text"),
  id: e.at("id", default: none), frame: e.at("frame", default: none), cell: e.at("cell", default: none),
  "show": f => render-element(t, e + (frame: f), vars, image-loader: image-loader)))

// Arguments d'une slide v1 pour son widget. MVP: t-270, plan 2 — remplacé
// par le rendu générique du modèle v2.
#let _slide-args(t, s, vars, image-loader) = {
  let kind = s.at("kind", default: "content")
  let opt(key) = s.at(key, default: auto)
  if kind == "title" { (title: opt("title"), subtitle: opt("subtitle"), author: opt("author"), date: opt("date"), kicker: auto) }
  else if kind == "chapter" { (title: s.title, subtitle: s.at("subtitle", default: none)) }
  else if kind == "toc" { (title: opt("title")) }
  else if kind == "end" { (text: interp(s.at("text", default: ""), vars)) }
  else if kind == "blank" {
    (grid: s.at("grid", default: default-grid), elements: _placed(t, s.at("elements", default: ()), vars, image-loader))
  } else {
    let free = "elements" in s or "grid" in s or s.at("blocks", default: ()).len() == 0
    (
      kicker: s.at("kicker", default: none),
      title: s.at("title", default: none),
      layout: s.at("layout", default: "default"),
      grid: s.at("grid", default: default-grid),
      blocks: if free { () } else { _flow(t, s.at("blocks", default: ()), vars, image-loader) },
      elements: if free { _placed(t, s.at("elements", default: ()), vars, image-loader) } else { () },
    )
  }
}

#let render-slide(s, image-loader: none) = context {
  let c = config()
  let t = c.tokens
  let kind = s.at("kind", default: "content")
  let name = if kind in ("title", "chapter", "toc", "end", "blank") { kind } else { "content" }
  let ctx = (id: s.at("id", default: none), tokens: t, vars: c.vars, style: (:), framed: false, load: none)
  (_widgets.at(name).render)(_slide-args(t, s, c.vars, image-loader), ctx)
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
