// Jetons de design d'Oku Slides.
//
// Tous les jetons sont des primitives JSON (chaînes, nombres, tableaux,
// dictionnaires) : un deck JSON peut donc les surcharger tels quels.
// - Couleurs : "#rrggbb" ou le nom d'une autre couleur ("accent").
// - Tailles, espaces, rayons, traits : nombres en pt (1 pt = 1 unité de la
//   grille 1920 × 1080), ou le nom d'un autre jeton de la même famille.
// - Polices : nom d'un rôle ("display", "body", "mono"), une chaîne ou un
//   tableau de familles (solutions de repli).

// Les valeurs par défaut vivent dans `default-tokens.json`, source unique :
// côté Rust, la session l'embarque pour les défauts de l'éditeur de thème et
// son schéma (`theme_schema.rs`), et le rendu l'expose à Typst
// (`oku-slides-render/src/world.rs`). Groupes : color, font, script-style, size, weight,
// leading, space, page, radius, stroke, chart (palette des séries), app
// (interface : none = dérivé du fond et de l'encre ; rayons en px, élévations),
// component (jetons de composants, qui référencent les jetons par nom).
// En-têtes et pieds : gabarits avec {champs} (title, subtitle, author, date,
// organization, chapter, chapter-number, chapter-label, n, total, page-label,
// et toute variable du deck) ; les segments séparés par " · " dont un champ
// est vide disparaissent. Filet du titre : none ou (width, height, color).
// Disposition de la page de titre : "plain" (tout sur le fond) ou "band".
#let default-tokens = json("default-tokens.json")

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

// ---- Alphabets hors latin --------------------------------------------------

// Langue et région Typst d'une étiquette BCP 47 ("zh-Hant-TW" → ("zh", "TW")).
#let split-lang(tag) = {
  let parts = tag.split("-")
  let region = parts.slice(1).find(p => p.len() == 2)
  (lower(parts.first()), if region == none { none } else { upper(region) })
}

// Polices livrées pour les alphabets que les polices des thèmes ne couvrent
// pas : arabe, devanagari, bengali, thaï, puis CJK. Les idéogrammes communs
// se dessinent selon la région : celle de la langue passe en tête (chinois
// simplifié par défaut). `style` "serif" : le CJK en Noto Serif d'abord
// (les autres alphabets n'ont que des Sans livrées), le Sans en dernier recours.
#let script-fallback(tag, style: "sans") = {
  assert(style in ("sans", "serif"), message: "script-style : \"sans\" ou \"serif\", pas " + repr(style))
  let parts = lower(tag).split("-")
  let first = if parts.first() == "ja" { "JP" } else if parts.first() == "ko" { "KR" } else if (
    parts.first() == "zh" and parts.any(p => p in ("hant", "tw", "hk", "mo"))
  ) { "TC" } else { "SC" }
  let regions = (first,) + ("JP", "SC", "TC", "KR").filter(r => r != first)
  let scripts = ("Arabic", "Devanagari", "Bengali", "Thai").map(s => "Noto Sans " + s)
  let sans = regions.map(r => "Noto Sans " + r)
  let cjk = if style == "serif" { regions.map(r => "Noto Serif " + r) + sans } else { sans }
  scripts + cjk
}

// Ajoute `script-fallback(tag)` à chaque famille de `t.font`, dans la
// graphie que `t.script-style` donne à son rôle.
#let with-script-fallback(t, tag) = {
  let styles = t.at("script-style", default: (:))
  for (k, v) in t.font {
    let fallback = script-fallback(tag, style: styles.at(k, default: "sans"))
    let families = if type(v) == array { v } else { (v,) }
    t.font.insert(k, families.filter(f => f not in fallback) + fallback)
  }
  t
}

// ---- Résolution des jetons -------------------------------------------------

#let color-of(t, v) = {
  if v == none { return none }
  if type(v) == color { return v }
  if type(v) == str {
    if v.starts-with("#") { return rgb(v) }
    if v in t.color { return color-of(t, t.color.at(v)) }
  }
  panic("oku-slides : couleur inconnue : " + repr(v))
}

// Puce de liste `marker` dans la couleur `c` ; sans couleur (none, jeton
// `null`), dans celle du texte.
#let list-marker(t, marker, c) = {
  let fill = color-of(t, c)
  if fill == none { [#marker] } else { text(fill: fill, marker) }
}

// Pastille n (dès 1) : couleur de la palette des graphiques, en rebouclant.
#let highlight-of(t, n) = {
  let palette = t.chart.palette
  color-of(t, palette.at(calc.rem(n - 1, palette.len())))
}

// Texte lisible sur un fond : l'encre sur un fond clair, le fond de page sinon.
#let ink-on(t, fill) = {
  let l = luma(fill).components().first()
  if l > 60% { color-of(t, "ink") } else { color-of(t, "bg") }
}

#let _num(t, family, v) = {
  if v == none { return none }
  if type(v) == length { return v }
  if type(v) == int or type(v) == float { return v * 1pt }
  if type(v) == str and v in t.at(family) { return _num(t, family, t.at(family).at(v)) }
  panic("oku-slides : valeur inconnue dans " + family + " : " + repr(v))
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
