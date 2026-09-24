// Jetons de design de Slider.
//
// Tous les jetons sont des primitives JSON (chaînes, nombres, tableaux,
// dictionnaires) : un deck JSON peut donc les surcharger tels quels.
// - Couleurs : "#rrggbb" ou le nom d'une autre couleur ("accent").
// - Tailles, espaces, rayons, traits : nombres en pt (1 pt = 1 unité de la
//   grille 1920 × 1080), ou le nom d'un autre jeton de la même famille.
// - Polices : nom d'un rôle ("display", "body", "mono"), une chaîne ou un
//   tableau de familles (solutions de repli).

#let default-tokens = (
  color: (
    bg: "#ffffff",
    surface: "#f4f3f0",
    ink: "#1d1c1a",
    ink-muted: "#5f5b55",
    accent: "#2f5fb3",
    on-accent: "#ffffff",
    rule: "#d9d6d0",
  ),
  font: (
    display: ("Libertinus Serif",),
    body: ("Libertinus Serif",),
    mono: ("DejaVu Sans Mono",),
  ),
  size: (
    display: 112,
    title: 60,
    subtitle: 40,
    lead: 40,
    body: 32,
    small: 26,
    caption: 22,
    kicker: 24,
  ),
  weight: (
    display: 500,
    title: 500,
    body: 400,
    strong: 600,
    label: 500,
  ),
  // Interlignes en em.
  leading: (tight: 0.25, normal: 0.6, loose: 0.9),
  space: (xs: 8, sm: 16, md: 24, lg: 40, xl: 64, xxl: 96),
  page: (
    width: 1920,
    height: 1080,
    margin-x: 120,
    margin-top: 150,
    margin-bottom: 140,
    header-y: 56,
    footer-y: 56,
  ),
  radius: (flat: 0, sm: 4, md: 10, lg: 16),
  stroke: (hairline: 1, rule: 2, accent: 6),

  // Jetons de composants : ils référencent les jetons ci-dessus par nom.
  component: (
    // Emplacements d'en-tête et de pied : gabarits avec {champs}. Les segments
    // séparés par " · " dont un champ est vide disparaissent. Champs : title,
    // subtitle, author, date, organization, chapter, chapter-number,
    // chapter-label, n, total, page-label, et toute variable du deck.
    header: (
      left: "{chapter-label}", right: "{organization}",
      size: "caption", color: "ink-muted", weight: "label", tracking: 0, case: none,
      rule: false, rule-color: "rule",
    ),
    footer: (
      left: "{author} · {date}", right: "{page-label}",
      size: "caption", color: "ink-muted", weight: "body", tracking: 0, case: none,
      rule: false, rule-color: "rule",
    ),
    kicker: (size: "kicker", color: "accent", weight: "label", font: "body", tracking: 0, case: none),
    title: (
      size: "title", color: "ink", weight: "title", font: "display", tracking: 0,
      leading: "tight", gap: "lg",
      // Filet entre surtitre et titre : none ou (width, height, color).
      rule: none,
    ),
    title-slide: (
      // "plain" : tout sur le fond ; "band" : bande sombre en bas, bandeau clair en haut.
      layout: "plain",
      bg: "bg", title-size: "display", title-color: "ink", title-tracking: 0,
      subtitle-size: "subtitle", subtitle-color: "ink-muted",
      meta-size: "small", meta-color: "ink-muted",
      bar: true, bar-color: "accent", bar-width: 160, bar-height: 6,
      // Variante "band".
      band-bg: "ink", band-ratio: 0.62, band-title-color: "bg",
      band-subtitle-color: "bg", band-meta-color: "bg",
      top-left: "{organization}", top-right: none, top-rule-color: "accent", top-rule-width: 6,
      badge: none, badge-bg: "accent", badge-color: "on-accent",
    ),
    chapter: (
      bg: "accent", ink: "on-accent", number-size: 220, number-opacity: 0.55,
      title-size: "display", title-tracking: 0, subtitle-size: "subtitle",
    ),
    card: (
      bg: "bg", border-color: "rule", border-width: "hairline", radius: "md",
      top-rule-color: "accent", top-rule-width: 4, padding-x: "lg", padding-y: "lg",
      title-size: "lead", title-weight: "strong", title-color: "ink", gap: "sm",
      // "auto" : hauteur du contenu ; "fill" : toute la hauteur disponible.
      height: "auto",
      highlight-bg: "ink", highlight-ink: "bg", highlight-title: "bg", highlight-kicker: "accent",
    ),
    toc: (size: "lead", number-color: "accent", page-color: "ink-muted", gap: "md"),
    end-slide: (bg: "surface", size: "display", color: "ink"),
    list: (marker: "–", marker-color: "accent", indent: "md", spacing: "sm"),
    quote: (size: 56, color: "ink", bar-color: "accent"),
    columns: (gap: "xl"),
  ),
)

#let default-params = (
  title: none,
  subtitle: none,
  author: none,
  date: none,
  organization: none,
  logo: none,
  lang: "fr",
  // "{n}", "{n} / {total}" ou none.
  numbering: "{n} / {total}",
  // Motif de numérotation Typst des chapitres ("1", "01", "I", "A"…) ou none.
  chapter-numbering: "1",
  toc-title: "Sommaire",
  // auto : en-tête / pied par défaut ; none : rien ; fonction (ctx) => contenu.
  header: auto,
  footer: auto,
)

// Fusion profonde : b l'emporte sur a, clé par clé.
#let merge(a, b) = {
  if type(a) != dictionary or type(b) != dictionary { return b }
  let out = a
  for (k, v) in b {
    out.insert(k, if k in a { merge(a.at(k), v) } else { v })
  }
  out
}

// ---- Résolution des jetons -------------------------------------------------

#let color-of(t, v) = {
  if v == none { return none }
  if type(v) == color { return v }
  if type(v) == str {
    if v.starts-with("#") { return rgb(v) }
    if v in t.color { return color-of(t, t.color.at(v)) }
  }
  panic("slider : couleur inconnue : " + repr(v))
}

#let _num(t, family, v) = {
  if v == none { return none }
  if type(v) == length { return v }
  if type(v) == int or type(v) == float { return v * 1pt }
  if type(v) == str and v in t.at(family) { return _num(t, family, t.at(family).at(v)) }
  panic("slider : valeur inconnue dans " + family + " : " + repr(v))
}

#let size-of(t, v) = _num(t, "size", v)
#let space-of(t, v) = _num(t, "space", v)
#let radius-of(t, v) = _num(t, "radius", v)
#let stroke-of(t, v) = _num(t, "stroke", v)

#let font-of(t, v) = {
  if type(v) == str and v in t.font { return font-of(t, t.font.at(v)) }
  if type(v) == array { return v }
  (v,)
}

#let weight-of(t, v) = {
  if type(v) == str and v in t.weight { return t.weight.at(v) }
  v
}

#let leading-of(t, v) = {
  if type(v) == str and v in t.leading { return t.leading.at(v) * 1em }
  if type(v) == int or type(v) == float { return v * 1em }
  v
}

// Style de texte depuis un dictionnaire de jetons de composant.
#let text-style(t, c, size: "size", color: "color", weight: "weight", font: "font", tracking: "tracking") = {
  let args = (:)
  if size in c { args.insert("size", size-of(t, c.at(size))) }
  if color in c { args.insert("fill", color-of(t, c.at(color))) }
  if weight in c { args.insert("weight", weight-of(t, c.at(weight))) }
  if font in c { args.insert("font", font-of(t, c.at(font))) }
  if tracking in c and c.at(tracking) != none { args.insert("tracking", c.at(tracking) * 1pt) }
  args
}

// Applique la casse d'un composant ("upper", "lower" ou none).
#let case-of(c, body) = {
  let k = c.at("case", default: none)
  if k == "upper" { upper(body) } else if k == "lower" { lower(body) } else { body }
}

// Texte mis en forme par un composant : style + casse.
#let styled(t, c, body) = text(..text-style(t, c), case-of(c, body))
