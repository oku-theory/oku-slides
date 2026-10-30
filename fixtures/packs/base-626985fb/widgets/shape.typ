// Forme : rectangle, ellipse, ligne ou chemin, au style de l'élément.

#import "/oku-slides/lib.typ": widget, color-of, radius-of, stroke-of, params as p

#let _path-max = 65536        // SHAPE_PATH_MAX (oku-slides-core)
#let _path-coord-max = 1000   // SHAPE_PATH_COORD_MAX
// Nombre de valeurs de chaque commande d'un chemin.
#let _path-arity = (M: 2, L: 2, C: 6, Q: 4, Z: 0)
#let _path-number = regex("^[-+]?([0-9]+(\\.[0-9]*)?|\\.[0-9]+)$")

// Commandes du chemin `d` : (lettre, nombres), ou none s'il est vide ou mal
// formé. Même syntaxe que `oku_slides_core::ShapePath` : `M x y`, `L x y`,
// `C x1 y1 x2 y2 x y`, `Q x1 y1 x y`, `Z`, nombres décimaux, mots séparés
// par des espaces, `M` en tête ; coordonnées relatives au cadre.
#let _path-commands(d) = {
  if type(d) != str or d.len() > _path-max { return none }
  let out = ()
  for w in d.split(regex("\\s+")).filter(w => w != "") {
    if w in _path-arity { out.push((w, ())) }
    else if out.len() == 0 or w.match(_path-number) == none { return none }
    else {
      let n = float(w)
      if calc.abs(n) > _path-coord-max { return none }
      out.last().at(1).push(n)
    }
  }
  let ok = (out.len() > 0 and out.first().at(0) == "M"
    and out.all(((c, n)) => n.len() == _path-arity.at(c)))
  if ok { out }
}

// Tracé d'un chemin mis à l'échelle du cadre (`w` × `h`) ; none s'il est
// vide ou mal formé. Remplissage non nul : un sous-chemin de sens contraire
// évide (anneau).
#let _path-curve(d, w, h, fill, stroke) = {
  let cmds = _path-commands(d)
  if cmds == none { return none }
  let pt(n, i) = (n.at(i) * w, n.at(i + 1) * h)
  let parts = cmds.map(((c, n)) => {
    if c == "M" { curve.move(pt(n, 0)) }
    else if c == "L" { curve.line(pt(n, 0)) }
    else if c == "C" { curve.cubic(pt(n, 0), pt(n, 2), pt(n, 4)) }
    else if c == "Q" { curve.quad(pt(n, 0), pt(n, 2)) }
    else { curve.close(mode: "straight") }
  })
  curve(fill: fill, fill-rule: "non-zero", stroke: stroke, ..parts)
}

// `path` : `{ d }` (voir `_path-commands`), lu avec `shape: "path"`.
#let widget = widget("shape", params: (shape: p.choice("rect", "ellipse", "line", "path"), path: p.host("path")),
  (args, ctx) => {
  let t = ctx.tokens
  let style = ctx.style
  let fill = color-of(t, style.at("fill", default: none))
  let stroke = if "stroke" in style { stroke-of(t, style.at("stroke-width", default: "rule")) + color-of(t, style.stroke) }
  // Hors cadre, la hauteur est infinie : le chemin s'inscrit dans un carré de la largeur.
  let d = if type(args.path) == dictionary { args.path.at("d", default: none) }
  if args.shape == "path" { layout(sz => _path-curve(d, sz.width,
    if sz.height == float.inf * 1pt { sz.width } else { sz.height }, fill, stroke)) }
  else if args.shape == "ellipse" { ellipse(width: 100%, height: 100%, fill: fill, stroke: stroke) }
  else if args.shape == "line" { line(length: 100%, stroke: if stroke != none { stroke } else { stroke-of(t, "rule") + color-of(t, "rule") }) }
  else { rect(width: 100%, height: 100%, fill: fill, stroke: stroke, radius: radius-of(t, style.at("radius", default: "flat"))) }
})
