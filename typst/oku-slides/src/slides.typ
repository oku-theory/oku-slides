// Cadre du deck : `deck`, surtitre, slide nue (repli d'une slide inconnue)
// et briques communes. Les slides et la carte vivent dans leurs packs
// (`essentiel`, `base`), avec les jetons de composant qu'elles lisent.

#import "tokens.typ": *
#import "core.typ": _cfg, config, chrome, interp

// ---- Deck --------------------------------------------------------------------

/// Point d'entrée : `#show: deck.with(title: [...], theme: papier, tokens: (...), vars: (...))`.
#let deck(theme: none, tokens: (:), vars: (:), ..params, body) = {
  let t = default-tokens
  if theme != none { t = merge(t, theme) }
  t = merge(t, tokens)
  let p = merge(default-params, params.named())
  t = with-script-fallback(t, p.lang)
  let (lang, region) = split-lang(p.lang)
  for key in ("title", "subtitle", "author", "organization", "date") {
    p.insert(key, interp(p.at(key), vars))
  }

  let pg = t.page
  let lst = t.list

  set document(
    title: if type(p.title) == str { p.title } else if p.title != none { p.title } else { none },
    author: if type(p.author) == str { p.author } else { () },
  )
  set page(
    width: pg.width * 1pt,
    height: pg.height * 1pt,
    margin: (
      left: pg.margin-x * 1pt,
      right: pg.margin-x * 1pt,
      top: pg.margin-top * 1pt,
      bottom: pg.margin-bottom * 1pt,
    ),
    fill: color-of(t, "bg"),
    header: none,
    footer: none,
    foreground: chrome,
  )
  set text(
    font: font-of(t, "body"),
    size: size-of(t, "body"),
    weight: weight-of(t, "body"),
    fill: color-of(t, "ink"),
    lang: lang,
    region: region,
  )
  set par(leading: leading-of(t, "normal"), spacing: space-of(t, "md"))
  set list(
    marker: list-marker(t, lst.marker, lst.at("marker-color", default: none)),
    indent: 0pt,
    body-indent: space-of(t, lst.indent),
    spacing: space-of(t, lst.spacing),
  )
  set enum(body-indent: space-of(t, lst.indent), spacing: space-of(t, lst.spacing))
  show strong: set text(weight: weight-of(t, "strong"))
  show link: set text(fill: color-of(t, "accent"))
  show raw: set text(font: font-of(t, "mono"))
  // Les titres sont mis en forme par les fonctions de slide.
  show heading: it => it.body

  // L'état est posé dans le premier élément de page pour ne pas créer de page vide.
  // deck-lang : la langue donnée au deck, sans défaut (écriture des nombres).
  _cfg.update((tokens: t, params: p, vars: vars, deck-lang: params.named().at("lang", default: none)))
  // Jetons résolus, lus par l'app pour accorder son interface au thème.
  [#metadata(t)<oku-slides-tokens>]
  body
}

// ---- Briques communes ----------------------------------------------------------

// Marqueur de slide pour l'app : relie la page à l'identifiant de la slide.
#let _mark(id) = if id != none [#metadata((slide: id))<oku-slides-slide>]

#let _heading(level, body, ..style) = {
  show heading: set text(..style)
  heading(level: level, outlined: true, bookmarked: true, body)
}

#let kicker(body) = context {
  let t = config().tokens
  styled(t, t.kicker, body)
}

/// Slide nue : pas de titre ni de chrome, tout l'espace est libre.
#let blank-slide(body, chrome: false, id: none) = context {
  if chrome { page({ _mark(id); body }) } else { page(foreground: none, { _mark(id); body }) }
}
