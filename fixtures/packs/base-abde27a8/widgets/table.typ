// Tableau rendu en Typst, à la largeur de sa place, réduit s'il déborde.

#import "/oku-slides/lib.typ": widget, color-of, size-of, space-of, weight-of, stroke-of, params as p
#import "/oku-slides/lib.typ": format-number, deck-lang
#import "_commun.typ": placeholder

// Tableau rendu en Typst. `data` : options de `oku-slides-chart::TableOption`,
// déjà ramenées à leur forme canonique par le rendu (render/src/tables.rs) ;
// les gardes ci-dessous protègent un appel direct (aperçu, pack).
#let _table-max-rows = 100     // MAX_TABLE_ROWS
#let _table-max-columns = 20   // MAX_TABLE_COLUMNS

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

#let widget = widget("table", grow: "fit", params: (data: p.host("table")), (args, ctx) => context {
  let t = ctx.tokens
  let lang = deck-lang()
  let d = args.data
  let dataset = if type(d) == dictionary { d.at("dataset", default: none) } else { none }
  let source = if type(dataset) == dictionary { dataset.at("source", default: none) } else { none }
  let ok = (type(source) == array and source.len() > 0 and source.len() <= _table-max-rows
    and source.all(r => type(r) == array) and source.first().len() > 0
    and source.first().len() <= _table-max-columns)
  if not ok { return placeholder(t, "table", height: if ctx.framed { 100% } else { 6em }) }
  let k = ctx.component.table
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
  // Fond des lignes du corps hors bandes ; `none` (le défaut) : sans fond.
  let body-bg = color-of(t, k.at("body-bg", default: none))
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
      else if striped and calc.odd(y - first) { stripe-bg } else { body-bg },
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
