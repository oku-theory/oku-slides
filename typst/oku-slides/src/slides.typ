// Structure imposée : deck, slide de titre, chapitre, sommaire, slide de
// contenu, slide de fin. Chaque slide est une page.

#import "tokens.typ": *
#import "widget.typ": fields-stack, _is-empty
#import "core.typ": _cfg, config, chrome, format-date, interp, chapter-number, chapter-counter, fill-template, template-fields

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
    lang: lang,
    region: region,
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
  styled(t, t.component.kicker, body)
}

// Titre de slide (heading de niveau 2) précédé d'un surtitre et d'un filet optionnels.
// wrap : (nom, contenu, style) => contenu ; le widget y pose ses champs.
#let _slide-title(t, title, kicker-body, wrap) = {
  let c = t.component.title
  let items = ()
  items.push(wrap("kicker", if not _is-empty(kicker-body) { styled(t, t.component.kicker, kicker-body) },
    text-style(t, t.component.kicker) + (case: t.component.kicker.at("case", default: none))))
  let r = c.at("rule", default: none)
  if r != none {
    items.push(rect(width: r.width * 1pt, height: r.height * 1pt, fill: color-of(t, r.color)))
  }
  items.push(wrap("title", {
    set par(leading: leading-of(t, c.at("leading", default: "tight")))
    _heading(2, title, ..text-style(t, c))
  }, text-style(t, c) + (leading: leading-of(t, c.at("leading", default: "tight")))))
  fields-stack(spacing: space-of(t, "sm"), ..items)
}

// Mise en page du corps : "default", "columns", "split", "center", "quote".
// grow : une valeur par corps, dans la mise en page par défaut. `true`
// (graphiques) : le corps prend une part de la hauteur qui reste. `"fit"`
// (tableaux) : il garde sa hauteur s'il tient dans une part, sinon il en
// prend une (et s'y réduit). `false` : sa hauteur.
#let _layout(t, layout, bodies, ratio: none, grow: ()) = {
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
  } else if grow.any(g => g == "fit") {
    // Part : la hauteur laissée par les corps fixes, divisée entre ceux qui
    // peuvent grandir. Un tableau plus haut qu'une part en prend une, au lieu
    // de prendre toute la hauteur restante aux corps qui le suivent.
    std.layout(size => {
      let gap = space-of(t, "md")
      let natural(b) = measure(b, width: size.width).height
      let fixed = bodies.zip(grow).filter(((b, g)) => g == false).map(((b, g)) => natural(b))
      let growing = grow.filter(g => g != false).len()
      let share = (size.height - fixed.sum(default: 0pt) - gap * (bodies.len() - 1)) / growing
      let rows = bodies.zip(grow).map(((b, g)) => if g == true { 1fr }
        else if g == "fit" and natural(b) > share { 1fr } else { auto })
      grid(columns: (1fr,), rows: rows, row-gutter: gap, ..bodies)
    })
  } else if grow.any(g => g == true) {
    grid(columns: (1fr,), rows: grow.map(g => if g == true { 1fr } else { auto }),
      row-gutter: space-of(t, "md"), ..bodies)
  } else {
    stack(spacing: space-of(t, "md"), ..bodies)
  }
}

// ---- Types de slides -------------------------------------------------------------

/// Slide de contenu, avec en-tête, pied, titre et corps.
/// layout : "default" | "columns" | "split" | "center" | "quote".
/// zone : contenu posé en haut à gauche de la zone de corps (repère pour l'app).
/// grow : corps qui prennent la hauteur restante (voir `_layout`).
/// wrap : (nom, contenu, style) => contenu ; le widget y pose ses champs.
#let slide(title: none, kicker: none, layout: "default", ratio: none, id: none, zone: none, grow: (),
  wrap: (name, body, style) => body, ..bodies) = context {
  let c = config()
  let t = c.tokens
  // Titre vide comme absent : pas de ligne de titre.
  let title = if _is-empty(title) { none } else { interp(title, c.vars) }
  page({
    _mark(id)
    grid(
      rows: if title != none { (auto, 1fr) } else { (1fr,) },
      row-gutter: space-of(t, t.component.title.gap),
      ..if title != none { (_slide-title(t, title, kicker, wrap),) },
      block(width: 100%, height: 100%, {
        if zone != none { place(top + left, zone) }
        // Sans titre, surtitre et titre restent atteignables : marqueurs hors flux.
        // `place(top + left, …)` impose `left` à son contenu : l'alignement du
        // flux (`start`, à droite en RTL) est passé dans le style des champs.
        if title == none {
          let here-align = (align: align.alignment)
          let marks = fields-stack(
            wrap("kicker", none, here-align + text-style(t, t.component.kicker)
              + (case: t.component.kicker.at("case", default: none))),
            wrap("title", none, here-align + text-style(t, t.component.title)
              + (leading: leading-of(t, t.component.title.at("leading", default: "tight")))))
          if marks != none { place(top + left, marks) }
        }
        _layout(t, layout, bodies.pos(), ratio: ratio, grow: grow)
      }),
    )
  })
}

/// Slide de titre du deck. Les valeurs par défaut viennent des paramètres du deck.
/// Deux mises en page, choisies par le jeton component.title-slide.layout.
#let title-slide(title: auto, subtitle: auto, author: auto, date: auto, kicker: auto, body: none, id: none,
  wrap: (name, body, style) => body) = context {
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
  // Fond réel sous le titre (le bandeau n'est pas le fond de page) : pour l'éditeur.
  let under = if band { (bg: color-of(t, k.band-bg)) } else { (:) }

  let bar = if k.bar {
    rect(width: k.bar-width * 1pt, height: k.bar-height * 1pt, fill: color-of(t, k.bar-color))
  }
  let title-block = wrap("title", if not _is-empty(title) {
    set par(leading: leading-of(t, "tight"))
    text(size: size-of(t, k.title-size), fill: col("title-color", "band-title-color"),
      font: font-of(t, "display"), weight: weight-of(t, "display"),
      tracking: k.title-tracking * 1pt, title)
  }, (size: size-of(t, k.title-size), fill: col("title-color", "band-title-color"),
      font: font-of(t, "display"), weight: weight-of(t, "display"),
      tracking: k.title-tracking * 1pt, leading: leading-of(t, "tight")) + under)
  let sub = wrap("subtitle", if not _is-empty(subtitle) {
      text(size: size-of(t, k.subtitle-size), fill: col("subtitle-color", "band-subtitle-color"), subtitle)
    }, (size: size-of(t, k.subtitle-size), fill: col("subtitle-color", "band-subtitle-color")) + under)
  let meta-line = if meta.len() > 0 {
    text(size: size-of(t, k.meta-size), fill: col("meta-color", "band-meta-color"), meta.join[ · ])
  }

  if not band {
    page(fill: color-of(t, k.bg), foreground: none, {
      _mark(id)
      // Colonne pleine largeur (et non `auto`, taillée sur le plus large) : la
      // zone des champs (`aw`) est celle où le titre peut s'étendre en tapant.
      grid(
        columns: (100%,),
        rows: (1fr, auto),
        row-gutter: space-of(t, "xl"),
        align(bottom + start, fields-stack(
          spacing: space-of(t, "md"),
          bar,
          wrap("kicker", if not _is-empty(kick) { styled(t, t.component.kicker, kick) },
            text-style(t, t.component.kicker) + (case: t.component.kicker.at("case", default: none)) + under),
          title-block, sub, body,
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
      _mark(id)
      // Bandeau haut.
      block(width: 100%, height: top-h, inset: (x: mx), {
        align(horizon, grid(
          columns: (1fr, auto),
          align: (start + horizon, end + horizon),
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
        align(horizon, fields-stack(spacing: space-of(t, "md"),
          bar, title-block, sub, meta-line, body)))
    })
  }
}

/// Slide de chapitre : ouvre une section, alimente le sommaire et l'en-tête.
#let chapter(title, subtitle: none, id: none, wrap: (name, body, style) => body) = context {
  let c = config()
  let t = c.tokens
  let k = t.component.chapter
  let ink = color-of(t, k.ink)
  let pattern = c.params.chapter-numbering
  let title = interp(title, c.vars)

  page(fill: color-of(t, k.bg), foreground: none, {
    _mark(id)
    set text(fill: ink)
    align(start + horizon, fields-stack(
      spacing: space-of(t, "lg"),
      chapter-counter.step(),
      ..if pattern != none {
        (text(size: size-of(t, k.number-size), font: font-of(t, "display"),
          weight: weight-of(t, "display"),
          fill: color-of(t, k.at("number-color", default: k.ink)).transparentize(100% - k.number-opacity * 100%), numbering(pattern, chapter-counter.get().first() + 1)),)
      },
      wrap("title", {
        set par(leading: leading-of(t, "tight"))
        _heading(1, title, size: size-of(t, k.title-size), font: font-of(t, "display"),
          weight: weight-of(t, "display"), fill: ink, tracking: k.at("title-tracking", default: 0) * 1pt)
      }, (size: size-of(t, k.title-size), font: font-of(t, "display"), weight: weight-of(t, "display"),
        fill: ink, tracking: k.at("title-tracking", default: 0) * 1pt, leading: leading-of(t, "tight"))),
      wrap("subtitle", if not _is-empty(subtitle) { text(size: size-of(t, k.subtitle-size), interp(subtitle, c.vars)) },
        (size: size-of(t, k.subtitle-size))),
    ))
  })
}

/// Sommaire : la liste des chapitres, avec leur numéro et leur page.
#let toc(title: auto, pages: true, id: none, wrap: (name, body, style) => body) = context {
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
  slide(title: if title == auto { c.params.toc-title } else { title }, id: id, wrap: wrap, {
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
#let end-slide(body, id: none, wrap: (name, body, style) => body) = context {
  let t = config().tokens
  let k = t.component.end-slide
  page(fill: color-of(t, k.bg), foreground: none, {
    _mark(id)
    align(center + horizon, wrap("text", text(
      size: size-of(t, k.size), fill: color-of(t, k.color), font: font-of(t, "display"),
      weight: weight-of(t, "display"), body,
    ), (size: size-of(t, k.size), fill: color-of(t, k.color), font: font-of(t, "display"),
      weight: weight-of(t, "display"))))
  })
}

/// Slide nue : pas de titre ni de chrome, tout l'espace est libre.
#let blank-slide(body, chrome: false, id: none) = context {
  if chrome { page({ _mark(id); body }) } else { page(foreground: none, { _mark(id); body }) }
}

/// Carte : surtitre, titre et corps, avec filet d'accent en haut.
/// highlight: true passe la carte sur le fond sombre du thème.
/// fill : `auto` suit le jeton card.height ; `true` remplit la hauteur (case de grille, cadre).
/// wrap : (nom, contenu, style) => contenu ; le widget y pose ses champs.
#let card(title: none, kicker: none, highlight: false, fill: auto, wrap: (name, body, style) => body, body) = context {
  let t = config().tokens
  let k = t.component.card
  let bg = color-of(t, if highlight { k.highlight-bg } else { k.bg })
  let ink = if highlight { color-of(t, k.highlight-ink) } else { none }
  let border = if highlight { none } else { stroke-of(t, k.border-width) + color-of(t, k.border-color) }
  block(
    width: 100%, height: if fill == true or (fill == auto and k.height == "fill") { 100% } else { auto },
    fill: bg, radius: radius-of(t, k.radius), clip: true,
    stroke: border,
    {
      place(top + left, rect(width: 100%, height: k.top-rule-width * 1pt, fill: color-of(t, k.top-rule-color)))
      block(inset: (x: space-of(t, k.padding-x), y: space-of(t, k.padding-y)), {
        if ink != none { set text(fill: ink) }
        set list(marker: text(fill: color-of(t, if highlight { k.highlight-kicker } else { t.component.list.marker-color }), t.component.list.marker))
        let kicker-c = t.component.kicker + if highlight { (color: k.highlight-kicker) } else { (:) }
        let title-style = (size: size-of(t, k.title-size), weight: weight-of(t, k.title-weight),
          fill: color-of(t, if highlight { k.highlight-title } else { k.title-color }))
        fields-stack(spacing: space-of(t, k.gap),
          wrap("kicker", if not _is-empty(kicker) { styled(t, kicker-c, kicker) },
            text-style(t, kicker-c) + (case: kicker-c.at("case", default: none))),
          wrap("title", if not _is-empty(title) { text(..title-style, title) }, title-style),
          wrap("body", if ink != none { text(fill: ink, body) } else { body }, if ink != none { (fill: ink) } else { (:) }),
        )
      })
    },
  )
}
