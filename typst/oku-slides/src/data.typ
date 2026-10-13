// Rendu d'un deck JSON v5 (le modèle de données de l'app Oku Slides).
//
// {
//   "version": 5,
//   "params": { "title": "…", "author": "…", "date": "…" },
//   "vars":   { "client": "Acme" },
//   "tokens": { "color": { "accent": "#8a5a1c" } },
//   "slides": [
//     { "id": "s1", "kind": "title" },
//     { "id": "s2", "kind": "chapter", "params": { "title": "Contexte" } },
//     { "id": "s3", "kind": "content", "params": { "title": "…", "layout": "columns",
//       "blocks": [ { "id": "e1", "kind": "text", "params": { "body": [ … ] } } ] } },
//     { "id": "s4", "kind": "end", "params": { "text": "Merci" } }
//   ]
// }
//
// Chaque slide et chaque élément sont rendus par le widget de leur `kind`,
// avec les arguments tirés de leur `params` selon le schéma du widget
// (`_args`). Un élément d'un slot libre porte un "frame" { x, y, w, h } en
// pt, relatif à la zone de corps, ou une "cell" de la grille.
// Chaque élément porte un "id" ; il est étiqueté <e-ID> pour l'introspection.

#import "tokens.typ": *
#import "core.typ": config, interp, text-probe
#import "slides.typ": deck, blank-slide
#import "content.typ": _text-over, _paragraphs
#import "zone.typ": default-grid
#import "widgets.typ": registry
#import "widget.typ": by-name

// Widgets de base, par nom : ceux du rendu quand `widgets` vaut none.
#let _base = by-name(registry)

// Entrées de slot : (kind, id, body) dans le flux ; (kind, id, frame, cell,
// show) en placement libre. `render` rend un élément (Typst ne connaît pas
// les fonctions déclarées plus bas : `render-element` est passé ici).
#let _flow(t, list, vars, image-loader, widgets, deck-params, render) = list.map(b => (
  kind: b.at("kind", default: none), id: b.at("id", default: none),
  body: render(t, b, vars, image-loader: image-loader, widgets: widgets, deck-params: deck-params)))
#let _placed(t, list, vars, image-loader, widgets, deck-params, render) = list.map(e => (
  kind: e.at("kind", default: none), id: e.at("id", default: none),
  frame: e.at("frame", default: none), cell: e.at("cell", default: none),
  "show": f => render(t, e + (frame: f), vars, image-loader: image-loader, widgets: widgets, deck-params: deck-params)))

// Un run de paragraphe (côté Typst) : un objet dont, s'il est présent, `t`
// est une chaîne. Mirroir de `RUN_KEYS`/`is_paragraphs` côté Rust
// (crates/oku-slides-core/src/params.rs).
#let _is-run(r) = type(r) == dictionary and (r.at("t", default: none) == none or type(r.at("t", default: none)) == str)

// Paragraphes (côté Typst) : chaque objet n'a que des clés `runs`/`list`, et
// ses `runs`, quand présents, sont un tableau de runs valides. Mirroir de
// `is_paragraphs` côté Rust.
#let _is-paragraphs(v) = type(v) == array and v.all(x => {
  type(x) == dictionary and x.keys().all(k => k in ("runs", "list")) and {
    let runs = x.at("runs", default: none)
    runs == none or (type(runs) == array and runs.all(_is-run))
  }
})

// La valeur `v` (non none) a-t-elle le type Typst du descripteur `d` ?
// Un deck incohérent s'ouvre et se rend : `validate` signale la valeur.
#let _fits(d, v) = {
  let ty = d.type
  if ty == "plain" or ty == "asset" { type(v) == str }
  else if ty == "rich" { _is-paragraphs(v) }
  else if ty == "slot" { type(v) == array and v.all(x => type(x) == dictionary) }
  else if ty == "flag" { type(v) == bool }
  else if ty == "number" { type(v) == int or type(v) == float }
  else if ty == "choice" { type(v) == str and v in d.values }
  else if ty == "grid" { type(v) == dictionary }
  else if ty == "list" { type(v) == array and v.all(x => type(x) == dictionary) }
  else { true }
}

// Valeur prête à rendre d'un paramètre ou d'une colonne `d` (hors slot et liste).
#let _ready(t, d, v, vars) = {
  if d.type == "plain" { interp(v, vars) }
  else if d.type == "rich" { if v == none or v == () { [] } else { _paragraphs(t, v, vars) } }
  else { v }
}

// Items d'une liste : id (`d<i>` s'il manque ou n'est pas une chaîne),
// colonnes au défaut, valeurs prêtes.
#let _items(t, d, v, vars) = (if v == none { () } else { v }).enumerate().map(((i, it)) => {
  let id = it.at("id", default: none)
  let out = (id: if type(id) == str { id } else { "d" + str(i) })
  for c in d.item {
    let cv = it.at(c.name, default: c.default)
    let cv = if cv == none or _fits(c, cv) { cv } else { c.default }
    out.insert(c.name, _ready(t, c, cv, vars))
  }
  out
})

// Arguments d'un widget depuis son schéma : défauts, liens au deck, textes
// interpolés, paragraphes rendus, slots rendus en entrées, listes en items.
// Une valeur d'un autre type que celui déclaré est remplacée par le défaut ;
// une liste absente est vide (son défaut n'amorce qu'un objet créé).
#let _args(t, schema, params, vars, deck-params, image-loader, widgets, render) = {
  let out = (:)
  for d in schema.params {
    let bind = d.at("bind", default: none)
    let v = if bind != none { deck-params.at(bind, default: none) }
      else if d.type == "list" { params.at(d.name, default: none) }
      else { params.at(d.name, default: d.default) }
    let v = if v == none or _fits(d, v) { v } else if d.type == "list" { none } else { d.default }
    out.insert(d.name, if d.type == "slot" {
        let list = if v == none { () } else { v }
        let entries = if d.mode == "free" { _placed } else { _flow }
        entries(t, list, vars, image-loader, widgets, deck-params, render)
      }
      else if d.type == "list" { _items(t, d, v, vars) }
      else { _ready(t, d, v, vars) })
  }
  out
}

// Le widget `name` s'il est de catégorie `category`, sinon none.
#let _widget(widgets, name, category) = {
  let w = widgets.at(if type(name) == str { name } else { "" }, default: none)
  if w != none and w.schema.category == category { w } else { none }
}

// Taille de la zone de corps d'une slide de contenu avec titre.
#let body-size(t, with-title: true) = {
  let pg = t.page
  let w = (pg.width - 2 * pg.margin-x) * 1pt
  let h = (pg.height - pg.margin-top - pg.margin-bottom) * 1pt
  (width: w, height: h)
}

/// Rend un élément avec le widget de son `kind` (catégorie "element") ;
/// sans widget, un encart « élément inconnu ». widgets : dictionnaire nom →
/// widget (none : ceux de base). deck-params : paramètres du deck, pour les
/// paramètres liés (`bind`).
#let render-element(t, e, vars, image-loader: none, widgets: none, deck-params: (:)) = {
  let widgets = if widgets == none { _base } else { widgets }
  let kind = e.at("kind", default: none)
  let style = e.at("style", default: (:))
  let ctx = (id: e.at("id", default: none), tokens: t, vars: vars, style: style, framed: "frame" in e,
    load: if image-loader != none { (..a) => image-loader(e, ..a) } else { none })
  // Un widget existant mais qui n'est pas de catégorie "element" (p. ex.
  // une slide comme "title") ne rend pas ici : boîte d'élément inconnu.
  let w = _widget(widgets, kind, "element")
  let el = if w == none { text(fill: red)[oku-slides : élément inconnu « #kind »] }
    else { (w.render)(_args(t, w.schema, e.at("params", default: (:)), vars, deck-params, image-loader, widgets,
      render-element), ctx) }
  let fixed = "frame" in e
  let (w, h) = if fixed { (e.frame.w * 1pt, e.frame.h * 1pt) } else { (100%, auto) }
  if "id" not in e { return block(width: w, height: h, el) }
  // Boîte de l'élément pour l'app : page, position et taille réelles.
  let sized = block(width: w, height: h, layout(sz => {
    // Un graphique ou un tableau s'adapte à la hauteur disponible : il se mesure dans celle-ci.
    let real-h = if fixed { sz.height } else if kind in ("chart", "table") and sz.height < float.inf * 1pt {
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

/// Rend une slide avec le widget de son `kind` (catégorie "slide") ; sans
/// widget, une page qui le signale. `ctx.load` charge une image d'un asset
/// du deck (le décor d'un widget de slide, `(ctx.load)(asset: …, fit: …)`).
#let render-slide(s, image-loader: none, widgets: none) = context {
  let c = config()
  let t = c.tokens
  let widgets = if widgets == none { _base } else { widgets }
  let kind = s.at("kind", default: none)
  let id = s.at("id", default: none)
  let w = _widget(widgets, kind, "slide")
  if w == none { return blank-slide(text(fill: red)[oku-slides : slide inconnue « #kind »], id: id) }
  let ctx = (id: id, tokens: t, vars: c.vars, style: (:), framed: false,
    load: if image-loader != none { (..a) => image-loader(s, ..a) } else { none })
  (w.render)(_args(t, w.schema, s.at("params", default: (:)), c.vars, c.params, image-loader, widgets,
    render-element), ctx)
}

/// Rend un deck JSON complet : `#render-deck(json("deck.json"))`.
/// image-loader : fonction (élément, asset: none, fit: "cover", chart: false)
/// => contenu, pour les images et les graphiques.
/// widgets : dictionnaire nom → widget (`by-name(registre)`) ; none : ceux de base.
#let render-deck(data, theme: none, image-loader: none, widgets: none) = {
  let widgets = if widgets == none { _base } else { widgets }
  let params = data.at("params", default: (:))
  show: deck.with(
    theme: theme,
    tokens: data.at("tokens", default: (:)),
    vars: data.at("vars", default: (:)),
    ..params,
  )
  for s in data.at("slides", default: ()) { render-slide(s, image-loader: image-loader, widgets: widgets) }
}
