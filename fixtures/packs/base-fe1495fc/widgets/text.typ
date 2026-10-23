// Texte riche : corps en paragraphes et listes, style de l'élément (taille,
// interligne, espacement, alignement, retraits, ancrage dans le cadre).

#import "/oku-slides/lib.typ": widget, field, leading-of, list-marker, _text-args, _text-over, params as p

#let _inset-max = 4096   // STYLE_INSET_MAX (oku-slides-core)
#let _spacing-max = 4096   // STYLE_SPACING_MAX (oku-slides-core)
#let _indent-max = 4096   // STYLE_INDENT_MAX (oku-slides-core)

// Marges intérieures d'un style (`inset` : pt haut, droite, bas, gauche),
// ou none si la clé manque ou n'a pas cette forme.
#let _inset(v) = {
  let ok = (type(v) == array and v.len() == 4
    and v.all(x => (type(x) == int or type(x) == float) and x >= 0 and x <= _inset-max))
  if ok { (top: v.at(0) * 1pt, right: v.at(1) * 1pt, bottom: v.at(2) * 1pt, left: v.at(3) * 1pt) }
}

// Espace entre paragraphes d'un style (`spacing` : pt), ou none si la clé
// manque ou n'est pas un nombre permis.
#let _spacing(v) = {
  if (type(v) == int or type(v) == float) and v >= 0 and v <= _spacing-max { v * 1pt }
}

// Retraits d'un style (`indent` : pt, marge gauche puis décalage de la
// première ligne), ou none si la clé manque ou n'a pas cette forme.
#let _indent(v) = {
  let num(x) = type(x) == int or type(x) == float
  let ok = (type(v) == array and v.len() == 2 and v.all(num)
    and v.at(0) >= 0 and v.at(0) <= _indent-max and calc.abs(v.at(1)) <= _indent-max)
  if ok { (margin: v.at(0) * 1pt, first: v.at(1) * 1pt) }
}

// Un texte aux retraits `ind` (`_indent`), comme PowerPoint : le texte part
// de la marge, la première ligne (et la puce d'un item) en est décalée ; un
// décalage négatif suspend la puce ou le numéro dans une boîte de sa largeur,
// le texte de l'item part de la marge. Rien ne part avant le bord.
#let _indented(t, ind, body) = {
  if ind == none { return body }
  let from = calc.max(0pt, ind.margin + calc.min(0pt, ind.first))
  let hang = -calc.min(0pt, ind.first)
  let lst = t.component.list
  let marker = list-marker(t, lst.marker, lst.at("marker-color", default: none))
  set par(hanging-indent: hang) if hang > 0pt
  set par(first-line-indent: (amount: ind.first, all: true)) if ind.first > 0pt
  set list(indent: 0pt, body-indent: 0pt, marker: box(width: hang, align(start, marker))) if hang > 0pt
  set enum(indent: 0pt, body-indent: 0pt, numbering: n => box(width: hang, align(start, numbering("1.", n)))) if hang > 0pt
  set list(indent: ind.first) if ind.first > 0pt
  set enum(indent: ind.first) if ind.first > 0pt
  // Le texte d'un item ne se décale plus : la puce a pris le retrait.
  show list: set par(hanging-indent: 0pt, first-line-indent: 0pt)
  show enum: set par(hanging-indent: 0pt, first-line-indent: 0pt)
  pad(left: from, body)
}

// Ancrage d'un texte : `inset` l'écarte des bords, `valign` (`top`,
// `middle`, `bottom`) le place en hauteur dans son cadre fixe. Le champ
// est dans `body` : sa boîte suit le texte déplacé. Sans ces clés (ou sans
// cadre pour `valign`), le corps tel quel.
#let _anchored(style, framed, body) = {
  let inset = _inset(style.at("inset", default: none))
  let v = (top: top, middle: horizon, bottom: bottom).at(str(style.at("valign", default: "")), default: none)
  let v = if framed { v }
  if inset == none and v == none { return body }
  let body = if inset != none { pad(..inset, body) } else { body }
  if v == none { body } else { block(width: 100%, height: 100%, align(v, body)) }
}

#let widget = widget("text", typography: true, params: (body: p.rich()), (args, ctx) => {
  let t = ctx.tokens
  let style = ctx.style
  set text(.._text-args(t, style))
  // Forme conditionnelle : un `set` dans le bloc d'un `if` ne vaudrait que dans ce bloc.
  set par(leading: leading-of(t, style.leading)) if "leading" in style
  // L'espace entre paragraphes vaut aussi entre items de liste, et entre un
  // paragraphe et une liste (Typst y prend celui des paragraphes).
  let gap = _spacing(style.at("spacing", default: none))
  set par(spacing: gap) if gap != none
  set list(spacing: gap) if gap != none
  set enum(spacing: gap) if gap != none
  // Sans alignement choisi, le texte part du début de la ligne : à gauche,
  // ou à droite dans une langue écrite de droite à gauche.
  let al = style.at("align", default: "start")
  let ind = _indent(style.at("indent", default: none))
  _anchored(style, ctx.framed, _indented(t, ind, field(ctx, "body", style: _text-over(t, style),
    empty: args.body == [],
    align((left: left, center: center, right: right, start: start, end: end).at(al, default: start), args.body))))
})
