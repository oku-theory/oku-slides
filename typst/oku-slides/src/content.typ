// Texte du deck JSON en contenu Typst : style d'un texte, runs, paragraphes.
// Aucun texte n'est interprété comme du code Typst.

#import "tokens.typ": *
#import "core.typ": interp

#let _tracking-max = 4096   // STYLE_TRACKING_MAX (oku-slides-core)

// Style d'un texte (élément ou run) en arguments de `text`. `tracking` (pt,
// borné) n'est que celui d'un élément : un run n'en porte pas.
#let _text-args(t, style) = {
  let a = (:)
  if "size" in style { a.insert("size", size-of(t, style.size)) }
  if "color" in style { a.insert("fill", color-of(t, style.color, lenient: true)) }
  if "weight" in style { a.insert("weight", weight-of(t, style.weight)) }
  if "font" in style { a.insert("font", font-of(t, style.font)) }
  if style.at("italic", default: false) { a.insert("style", "italic") }
  let tr = style.at("tracking", default: none)
  if (type(tr) == int or type(tr) == float) and calc.abs(tr) <= _tracking-max { a.insert("tracking", tr * 1pt) }
  a
}

// Surcharges typographiques d'un élément texte, pour la sonde de style de
// sa boîte (voir `text-probe`).
#let _text-over(t, style) = {
  let o = _text-args(t, style)
  if "leading" in style { o.insert("leading", leading-of(t, style.leading)) }
  o.insert("align", (left: left, center: center, right: right, start: start, end: end)
    .at(style.at("align", default: "start"), default: start))
  o
}

#let _runs(t, runs, vars, strong: auto) = runs.map(r => {
  let body = [#interp(r.at("t", default: ""), vars)]
  let a = _text-args(t, r)
  if r.at("bold", default: false) {
    a.insert("weight", if strong == auto { weight-of(t, "strong") } else { strong })
  }
  let n = r.at("highlight", default: none)
  let fill = if n != none { highlight-of(t, n) }
  if fill != none and "fill" not in a { a.insert("fill", ink-on(t, fill)) }
  let body = if a.len() > 0 { text(..a, body) } else { body }
  // Pastille : fond arrondi qui ne se coupe pas en fin de ligne.
  let body = if fill != none {
    box(fill: fill, radius: radius-of(t, "sm"), inset: (x: 0.35em), outset: (y: 0.2em), body)
  } else { body }
  // Seuls les liens web et mail deviennent cliquables (pas de `javascript:`).
  let url = lower(r.at("link", default: ""))
  let web = ("http://", "https://", "mailto:").any(p => url.starts-with(p))
  if web { link(r.link, body) } else { body }
}).join()

// Style of a level-`h` heading (1 to 6, `heading.hN` tokens), or none outside that domain.
#let _heading-style(t, h) = if type(h) == int and h >= 1 and h <= 6 { t.heading.at("h" + str(h)) }

// Items (`list.item`, `enum.item`), not lists: Typst groups neighbouring items
// into a single list, numbered 1, 2, 3. A heading is a styled paragraph, never a
// Typst `heading`: it would end up in the TOC and the PDF bookmarks. It wins over `list`.
#let _paragraphs(t, paras, vars) = {
  let out = ()
  for p in paras {
    let runs = p.at("runs", default: ())
    let h = _heading-style(t, p.at("heading", default: none))
    if h != none {
      let style = text-style(t, h)
      // A bold run never makes the heading lighter.
      let strong = weight-of(t, "strong")
      let w = style.at("weight", default: none)
      let strong = if type(w) == int and type(strong) == int { calc.max(w, strong) } else { strong }
      out.push(par(text(..style, _runs(t, runs, vars, strong: strong))))
      continue
    }
    let body = _runs(t, runs, vars)
    let kind = p.at("list", default: none)
    if kind == "bullet" { out.push(list.item(body)) }
    else if kind == "number" { out.push(enum.item(body)) }
    else { out.push(par(body)) }
  }
  out.join()
}
