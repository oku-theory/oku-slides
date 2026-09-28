// Zone de corps d'une slide : là où se placent les éléments libres, sur
// leur cadre ou sur une cellule de la grille.

#import "tokens.typ": *

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
// La zone est signalée à l'app (<oku-slides-zone>) : position, taille, grille.
// Grille de toute slide qui n'en précise pas (même valeur que Grid::DEFAULT).
#let default-grid = (columns: 12, rows: 6)

// Signale la zone de corps à l'app : position, taille et grille résolue.
#let zone-marker(t, grid) = layout(sz => context {
  let p = here().position()
  let gap = space-of(t, grid.at("gap", default: "lg")) / 1pt
  [#metadata((page: p.page, x: p.x / 1pt, y: p.y / 1pt, w: sz.width / 1pt, h: sz.height / 1pt,
    grid: (columns: grid.columns, rows: grid.rows, gap: gap)))<oku-slides-zone>]
})

// Zone de corps libre : place chaque entrée de slot free sur sa cellule de
// grille (résolue dans la taille réelle de la zone) ou sur son cadre, et
// signale la zone à l'app.
#let free-area(t, children, grid: none) = block(width: 100%, height: 100%, layout(sz => {
  let zw = sz.width / 1pt
  let zh = sz.height / 1pt
  let gap = if grid != none { space-of(t, grid.at("gap", default: "lg")) / 1pt } else { 0 }
  if grid != none { place(top + left, zone-marker(t, grid)) }
  for c in children {
    let f = if grid != none and c.cell != none { grid-frame(zw, zh, grid, gap, c.cell) } else { c.frame }
    place(top + left, dx: f.x * 1pt, dy: f.y * 1pt, (c.show)(f))
  }
}))
