// Écriture des nombres selon la langue du deck (pas celle de l'interface) :
// séparateur décimal, et milliers groupés à partir de 10 000. Langue inconnue
// ou absente : écriture de Typst, sans groupes.
// Miroir de crates/oku-slides-chart/src/numbers.rs (graphiques).

#import "core.typ": _cfg

// (décimal, milliers) par sous-étiquette de langue.
#let _separators = (
  fr: (",", "\u{202F}"),
  en: (".", ","), ja: (".", ","), ko: (".", ","), zh: (".", ","),
  th: (".", ","), hi: (".", ","), bn: (".", ","), ar: (".", ","),
  de: (",", "."), nl: (",", "."), it: (",", "."), es: (",", "."),
  pt: (",", "."), tr: (",", "."), vi: (",", "."),
  pl: (",", "\u{00A0}"), uk: (",", "\u{00A0}"), ru: (",", "\u{00A0}"),
)

// Langue donnée au deck (`params.lang`), none sans deck ou sans langue.
// À appeler dans un context.
#let deck-lang() = {
  let c = _cfg.get()
  if c == none { none } else { c.at("deck-lang", default: none) }
}

// Un nombre écrit dans la langue `lang` (étiquette BCP 47, ou none) : entier
// sans décimales, sinon l'écriture la plus courte de Typst.
#let format-number(v, lang) = {
  let s = if type(v) == int { str(v) }
    else if calc.fract(v) == 0 and calc.abs(v) < 1e15 { str(int(v)) }
    else { str(v) }
  let key = if type(lang) == str { lower(lang.split(regex("[-_]")).first()) } else { "" }
  let sep = _separators.at(key, default: none)
  if sep == none { return s }
  let (decimal, group) = sep
  // Le signe de Typst (« − ») est gardé tel quel.
  let m = s.match(regex("^([^0-9]?)([0-9]+)(?:\.([0-9]+))?$"))
  if m == none { return s }
  let (sign, int-part, frac) = m.captures
  let n = int-part.len()
  let grouped = if n < 5 { int-part } else {
    range(n).map(i => (if i > 0 and calc.rem(n - i, 3) == 0 { group } else { "" }) + int-part.at(i)).join()
  }
  sign + grouped + if frac != none { decimal + frac } else { "" }
}
