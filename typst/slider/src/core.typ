// État partagé, variables et en-tête / pied de page.

#import "tokens.typ": *

#let _cfg = state("slider-config", none)
#let chapter-counter = counter("slider-chapter")

#let _months-fr = (
  "janvier", "février", "mars", "avril", "mai", "juin",
  "juillet", "août", "septembre", "octobre", "novembre", "décembre",
)

// Configuration courante : (tokens, params, vars). À appeler dans un context.
#let config() = {
  let c = _cfg.get()
  if c == none { panic("slider : utilisez `#show: deck.with(..)` avant les slides") }
  c
}

// ---- Variables ---------------------------------------------------------------

// Valeur d'une variable du deck.  #var("client")
#let var(name, default: none) = context {
  let c = config()
  let val = c.vars.at(name, default: default)
  if val == none { text(fill: red)[\[#name\]] } else { [#val] }
}

// Remplace {{nom}} par la variable correspondante dans une chaîne.
#let interp(s, vars) = {
  if type(s) != str { return s }
  s.replace(regex("\{\{\s*([\w-]+)\s*\}\}"), m => {
    let key = m.captures.first()
    if key in vars { str(vars.at(key)) } else { m.text }
  })
}

#let format-date(d, lang: "fr") = {
  if d == none { return none }
  if type(d) == datetime {
    if lang == "fr" {
      let day = if d.day() == 1 { "1er" } else { str(d.day()) }
      return day + " " + _months-fr.at(d.month() - 1) + " " + str(d.year())
    }
    return d.display("[month repr:long] [day], [year]")
  }
  d
}

// ---- Contexte de page --------------------------------------------------------

// Chapitre (heading de niveau 1) en vigueur à la page courante.
#let current-chapter() = {
  let pg = here().page()
  let hs = query(heading.where(level: 1)).filter(h => h.location().page() <= pg)
  if hs.len() == 0 { none } else { hs.last() }
}

#let chapter-number(h, pattern) = {
  if pattern == none or h == none { return none }
  numbering(pattern, chapter-counter.at(h.location()).first())
}

// Contexte passé aux fonctions d'en-tête et de pied personnalisées.
#let page-context() = {
  let c = config()
  let ch = current-chapter()
  (
    tokens: c.tokens,
    params: c.params,
    vars: c.vars,
    title: c.params.title,
    chapter: if ch != none { ch.body } else { none },
    chapter-number: chapter-number(ch, c.params.chapter-numbering),
    page: counter(page).get().first(),
    total: counter(page).final().first(),
  )
}

#let _page-label(ctx) = {
  let pattern = ctx.params.numbering
  if pattern == none { return none }
  pattern.replace("{n}", str(ctx.page)).replace("{total}", str(ctx.total))
}

// Valeurs disponibles dans les gabarits d'en-tête, de pied et de couverture.
#let template-fields(ctx) = {
  let p = ctx.params
  let chapter-label = if ctx.at("chapter", default: none) != none {
    if ctx.chapter-number != none [#ctx.chapter-number · #ctx.chapter] else { ctx.chapter }
  } else { p.title }
  let f = ctx.vars
  f += (
    title: p.title, subtitle: p.subtitle, author: p.author, organization: p.organization,
    date: format-date(p.date, lang: p.lang),
    chapter: ctx.at("chapter", default: none),
    chapter-number: ctx.at("chapter-number", default: none),
    chapter-label: chapter-label,
    n: if "page" in ctx { str(ctx.page) },
    total: if "total" in ctx { str(ctx.total) },
    page-label: if "page" in ctx { _page-label(ctx) },
  )
  f
}

// "{n} · {title}" → contenu ; un segment dont un champ vaut none disparaît.
#let fill-template(tpl, fields) = {
  if tpl == none { return none }
  if type(tpl) != str { return tpl }
  let segments = tpl.split(" · ").map(seg => {
    let parts = ()
    let last = 0
    for m in seg.matches(regex("\{([\w-]+)\}")) {
      let val = fields.at(m.captures.first(), default: none)
      if val == none { return none }
      parts.push(seg.slice(last, m.start))
      parts.push(if type(val) in (str, int, float) { str(val) } else { val })
      last = m.end
    }
    parts.push(seg.slice(last))
    parts.map(x => [#x]).join()
  }).filter(x => x != none)
  if segments.len() == 0 { none } else { segments.join[ · ] }
}

#let _slots(ctx, comp) = {
  let f = template-fields(ctx)
  grid(
    columns: (1fr, auto),
    align: (left + horizon, right + horizon),
    fill-template(comp.at("left", default: none), f),
    fill-template(comp.at("right", default: none), f),
  )
}

#let default-header(ctx) = {
  let comp = ctx.tokens.component.header
  if ctx.params.logo != none {
    let f = template-fields(ctx)
    grid(columns: (1fr, auto), align: (left + horizon, right + horizon),
      fill-template(comp.left, f), ctx.params.logo)
  } else { _slots(ctx, comp) }
}

#let default-footer(ctx) = _slots(ctx, ctx.tokens.component.footer)

// Premier plan de chaque slide à chrome : en-tête et pied positionnés par jetons.
#let chrome = context {
  let c = _cfg.get()
  if c == none { return }
  let t = c.tokens
  let p = c.params
  let ctx = page-context()
  let mx = t.page.margin-x * 1pt
  let w = page.width - 2 * mx

  let zone(kind, fn-default, dy, anchor) = {
    let spec = p.at(kind)
    if spec == none { return }
    let comp = t.component.at(kind)
    let body = if type(spec) == function { spec(ctx) } else { fn-default(ctx) }
    let rule = if comp.at("rule", default: false) {
      line(length: 100%, stroke: stroke-of(t, "hairline") + color-of(t, comp.rule-color))
    }
    place(anchor, dx: mx, dy: dy, block(width: w, {
      set text(..text-style(t, comp))
      if kind == "footer" { rule; v(space-of(t, "xs")) }
      case-of(comp, body)
      if kind == "header" { v(space-of(t, "xs")); rule }
    }))
  }
  zone("header", default-header, t.page.header-y * 1pt, top + left)
  zone("footer", default-footer, -t.page.footer-y * 1pt, bottom + left)
}
