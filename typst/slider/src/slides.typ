// Structure imposée : deck, slide de titre, chapitre, sommaire, slide de
// contenu, slide de fin. Chaque slide est une page.

#import "tokens.typ": *
#import "core.typ": _cfg, config, chrome, format-date, interp, chapter-number, chapter-counter, fill-template, template-fields

// ---- Deck --------------------------------------------------------------------

/// Point d'entrée : `#show: deck.with(title: [...], theme: papier, tokens: (...), vars: (...))`.
#let deck(theme: none, tokens: (:), vars: (:), ..params, body) = {
  let t = default-tokens
  if theme != none { t = merge(t, theme) }
  t = merge(t, tokens)
  let p = merge(default-params, params.named())
  for key in ("title", "subtitle", "author", "organization", "date") {
    p.insert(key, interp(p.at(key), vars))
  }

  let pg = t.page
  let lst = t.component.list

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
    lang: p.lang,
  )
  set par(leading: leading-of(t, "normal"), spacing: space-of(t, "md"))
  set list(
    marker: text(fill: color-of(t, lst.marker-color), lst.marker),
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
  _cfg.update((tokens: t, params: p, vars: vars))
  body
}

// ---- Briques communes ----------------------------------------------------------

#let _heading(level, body, ..style) = {
  show heading: set text(..style)
  heading(level: level, outlined: true, bookmarked: true, body)
}

#let kicker(body) = context {
  let t = config().tokens
  styled(t, t.component.kicker, body)
}

// Titre de slide (heading de niveau 2) précédé d'un surtitre et d'un filet optionnels.
#let _slide-title(t, title, kicker-body) = {
  let c = t.component.title
  let items = ()
  if kicker-body != none { items.push(styled(t, t.component.kicker, kicker-body)) }
  let r = c.at("rule", default: none)
  if r != none {
    items.push(rect(width: r.width * 1pt, height: r.height * 1pt, fill: color-of(t, r.color)))
  }
  items.push({
    set par(leading: leading-of(t, c.at("leading", default: "tight")))
    _heading(2, title, ..text-style(t, c))
  })
  stack(spacing: space-of(t, "sm"), ..items)
}

// Mise en page du corps : "default", "columns", "split", "center", "quote".
#let _layout(t, layout, bodies, ratio: none) = {
  let gap = space-of(t, t.component.columns.gap)
  if layout == "columns" {
    grid(columns: (1fr,) * calc.max(bodies.len(), 1), column-gutter: gap, ..bodies)
  } else if layout == "split" {
    let cols = if ratio != none { ratio } else { (1fr, 1fr) }
    grid(columns: cols, column-gutter: gap, ..bodies)
  } else if layout == "center" {
    align(center + horizon, stack(spacing: space-of(t, "md"), ..bodies))
  } else if layout == "quote" {
    let q = t.component.quote
    align(horizon, block(
      inset: (left: space-of(t, "lg")),
      stroke: (left: stroke-of(t, "accent") + color-of(t, q.bar-color)),
      text(size: size-of(t, q.size), fill: color-of(t, q.color), font: font-of(t, "display"),
        stack(spacing: space-of(t, "md"), ..bodies)),
    ))
  } else {
    stack(spacing: space-of(t, "md"), ..bodies)
  }
}

// ---- Types de slides -------------------------------------------------------------

/// Slide de contenu, avec en-tête, pied, titre et corps.
/// layout : "default" | "columns" | "split" | "center" | "quote".
#let slide(title: none, kicker: none, layout: "default", ratio: none, ..bodies) = context {
  let c = config()
  let t = c.tokens
  let title = interp(title, c.vars)
  page({
    grid(
      rows: if title != none { (auto, 1fr) } else { (1fr,) },
      row-gutter: space-of(t, t.component.title.gap),
      ..if title != none { (_slide-title(t, title, kicker),) },
      block(width: 100%, height: 100%, _layout(t, layout, bodies.pos(), ratio: ratio)),
    )
  })
}

/// Slide de titre du deck. Les valeurs par défaut viennent des paramètres du deck.
/// Deux mises en page, choisies par le jeton component.title-slide.layout.
#let title-slide(title: auto, subtitle: auto, author: auto, date: auto, kicker: auto, body: none) = context {
  let c = config()
  let t = c.tokens
  let p = c.params
  let k = t.component.title-slide
  let pick(v, key) = if v == auto { p.at(key) } else { v }
  let title = interp(pick(title, "title"), c.vars)
  let subtitle = interp(pick(subtitle, "subtitle"), c.vars)
  let meta = (pick(author, "author"), format-date(pick(date, "date"), lang: p.lang)).filter(x => x != none)
  let kick = if kicker == auto { p.organization } else { kicker }
  let band = k.at("layout", default: "plain") == "band"
  let col(key, band-key) = color-of(t, if band { k.at(band-key) } else { k.at(key) })

  let bar = if k.bar {
    rect(width: k.bar-width * 1pt, height: k.bar-height * 1pt, fill: color-of(t, k.bar-color))
  }
  let title-block = {
    set par(leading: leading-of(t, "tight"))
    text(size: size-of(t, k.title-size), fill: col("title-color", "band-title-color"),
      font: font-of(t, "display"), weight: weight-of(t, "display"),
      tracking: k.title-tracking * 1pt, title)
  }
  let sub = if subtitle != none {
    text(size: size-of(t, k.subtitle-size), fill: col("subtitle-color", "band-subtitle-color"), subtitle)
  }
  let meta-line = if meta.len() > 0 {
    text(size: size-of(t, k.meta-size), fill: col("meta-color", "band-meta-color"), meta.join[ · ])
  }

  if not band {
    page(fill: color-of(t, k.bg), foreground: none, {
      grid(
        rows: (1fr, auto),
        row-gutter: space-of(t, "xl"),
        align(bottom + left, stack(
          spacing: space-of(t, "md"),
          ..(bar, if kick != none { styled(t, t.component.kicker, kick) }, title-block, sub, body)
            .filter(x => x != none),
        )),
        meta-line,
      )
    })
  } else {
    let pg = t.page
    let mx = pg.margin-x * 1pt
    let fields = template-fields((params: p, vars: c.vars))
    let badge = if k.badge != none {
      box(fill: color-of(t, k.badge-bg), inset: (x: space-of(t, "sm"), y: space-of(t, "xs")),
        radius: radius-of(t, "sm"),
        styled(t, t.component.kicker + (color: k.badge-color), k.badge))
    }
    let top-h = pg.height * (1 - k.band-ratio) * 1pt
    page(fill: color-of(t, k.bg), foreground: none, margin: 0pt, {
      // Bandeau haut.
      block(width: 100%, height: top-h, inset: (x: mx), {
        align(horizon, grid(
          columns: (1fr, auto),
          align: (left + horizon, right + horizon),
          text(size: size-of(t, "subtitle"), weight: weight-of(t, "title"), fill: color-of(t, t.component.title.color),
            fill-template(k.top-left, fields)),
          stack(dir: ltr, spacing: space-of(t, "md"),
            ..(fill-template(k.top-right, fields), badge).filter(x => x != none)
              .map(x => align(horizon, text(size: size-of(t, "small"), fill: color-of(t, "ink-muted"), x)))),
        ))
      })
      // Filet.
      place(top + left, dy: top-h - k.top-rule-width * 1pt,
        rect(width: 100%, height: k.top-rule-width * 1pt, fill: color-of(t, k.top-rule-color)))
      // Bande basse.
      block(width: 100%, height: 1fr, fill: color-of(t, k.band-bg), inset: (x: mx, y: space-of(t, "xl")),
        align(horizon, stack(spacing: space-of(t, "md"),
          ..(bar, title-block, sub, meta-line, body).filter(x => x != none))))
    })
  }
}

/// Slide de chapitre : ouvre une section, alimente le sommaire et l'en-tête.
#let chapter(title, subtitle: none) = context {
  let c = config()
  let t = c.tokens
  let k = t.component.chapter
  let ink = color-of(t, k.ink)
  let pattern = c.params.chapter-numbering
  let title = interp(title, c.vars)

  page(fill: color-of(t, k.bg), foreground: none, {
    set text(fill: ink)
    align(left + horizon, stack(
      spacing: space-of(t, "lg"),
      chapter-counter.step(),
      ..if pattern != none {
        (text(size: size-of(t, k.number-size), font: font-of(t, "display"),
          weight: weight-of(t, "display"),
          fill: color-of(t, k.at("number-color", default: k.ink)).transparentize(100% - k.number-opacity * 100%), numbering(pattern, chapter-counter.get().first() + 1)),)
      },
      {
        set par(leading: leading-of(t, "tight"))
        _heading(1, title, size: size-of(t, k.title-size), font: font-of(t, "display"),
          weight: weight-of(t, "display"), fill: ink, tracking: k.at("title-tracking", default: 0) * 1pt)
      },
      ..if subtitle != none { (text(size: size-of(t, k.subtitle-size), interp(subtitle, c.vars)),) },
    ))
  })
}

/// Sommaire : la liste des chapitres, avec leur numéro et leur page.
#let toc(title: auto, pages: true) = context {
  let c = config()
  let t = c.tokens
  let k = t.component.toc
  let chapters = query(heading.where(level: 1))
  let rows = chapters.map(h => {
    let loc = h.location()
    (
      text(fill: color-of(t, k.number-color), chapter-number(h, c.params.chapter-numbering)),
      link(loc, text(fill: color-of(t, "ink"), h.body)),
      if pages { text(fill: color-of(t, k.page-color), str(counter(page).at(loc).first())) },
    )
  })
  slide(title: if title == auto { c.params.toc-title } else { title }, {
    set text(size: size-of(t, k.size))
    grid(
      columns: (auto, 1fr, auto),
      column-gutter: space-of(t, "lg"),
      row-gutter: space-of(t, k.gap),
      ..rows.flatten(),
    )
  })
}

/// Slide de fin, sans chrome.
#let end-slide(body) = context {
  let t = config().tokens
  let k = t.component.end-slide
  page(fill: color-of(t, k.bg), foreground: none, align(center + horizon, text(
    size: size-of(t, k.size), fill: color-of(t, k.color), font: font-of(t, "display"),
    weight: weight-of(t, "display"), body,
  )))
}

/// Slide nue : pas de titre ni de chrome, tout l'espace est libre.
#let blank-slide(body, chrome: false) = context {
  if chrome { page(body) } else { page(foreground: none, body) }
}

/// Carte : surtitre, titre et corps, avec filet d'accent en haut.
/// highlight: true passe la carte sur le fond sombre du thème.
#let card(title: none, kicker: none, highlight: false, body) = context {
  let t = config().tokens
  let k = t.component.card
  let bg = color-of(t, if highlight { k.highlight-bg } else { k.bg })
  let ink = if highlight { color-of(t, k.highlight-ink) } else { none }
  let border = if highlight { none } else { stroke-of(t, k.border-width) + color-of(t, k.border-color) }
  block(
    width: 100%, height: if k.height == "fill" { 100% } else { auto },
    fill: bg, radius: radius-of(t, k.radius), clip: true,
    stroke: border,
    {
      place(top + left, rect(width: 100%, height: k.top-rule-width * 1pt, fill: color-of(t, k.top-rule-color)))
      block(inset: (x: space-of(t, k.padding-x), y: space-of(t, k.padding-y)), {
        if ink != none { set text(fill: ink) }
        set list(marker: text(fill: color-of(t, if highlight { k.highlight-kicker } else { t.component.list.marker-color }), t.component.list.marker))
        stack(spacing: space-of(t, k.gap),
          ..if kicker != none {
            (styled(t, t.component.kicker + if highlight { (color: k.highlight-kicker) } else { (:) }, kicker),)
          },
          ..if title != none {
            (text(size: size-of(t, k.title-size), weight: weight-of(t, k.title-weight),
              fill: color-of(t, if highlight { k.highlight-title } else { k.title-color }), title),)
          },
          if ink != none { text(fill: ink, body) } else { body },
        )
      })
    },
  )
}
