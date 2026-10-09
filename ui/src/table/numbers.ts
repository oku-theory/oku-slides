// Écriture des nombres d'une grille dans la langue du deck, comme le rendu
// (crates/oku-slides-chart/src/numbers.rs, typst/oku-slides/src/numbers.typ) :
// séparateur décimal, milliers groupés à partir de 10 000 ; signe moins « − » dans toutes les langues.
// Comme la slide, l'écriture la plus courte qui relit le même nombre, jamais
// en notation scientifique.
//
// Pas `Intl.NumberFormat(lang)` : il écrit des chiffres arabes ou
// bengalis, groupe à l'indienne (hi, bn) ; la grille doit montrer ce que la
// slide affiche.

import { parseCell } from "./cells";

type Separators = { decimal: string; group: string };

const NNBSP = " ";
const NBSP = " ";
const MINUS = "\u2212";

const SEPARATORS: Record<string, Separators> = {
  fr: { decimal: ",", group: NNBSP },
  ...Object.fromEntries(["en", "ja", "ko", "zh", "th", "hi", "bn", "ar"].map((l) => [l, { decimal: ".", group: "," }])),
  ...Object.fromEntries(["de", "nl", "it", "es", "pt", "tr", "vi"].map((l) => [l, { decimal: ",", group: "." }])),
  ...Object.fromEntries(["pl", "uk", "ru"].map((l) => [l, { decimal: ",", group: NBSP }])),
};

function separators(lang: string): Separators | undefined {
  return SEPARATORS[lang.split(/[-_]/)[0].toLowerCase()];
}

/**
 * `n` en chiffres, sans exposant : l'écriture la plus courte de JS (comme
 * `{}` en Rust, `str` en Typst), développée quand JS écrirait « 1e+21 ».
 */
function plainNumber(n: number): string {
  const s = String(n);
  const m = /^(-?)(\d)(?:\.(\d+))?e([+-]\d+)$/.exec(s);
  if (!m) return s;
  const [, sign, lead, rest = "", exp] = m;
  const digits = lead + rest;
  const point = 1 + Number(exp);
  if (point <= 0) return `${sign}0.${"0".repeat(-point)}${digits}`;
  if (point >= digits.length) return sign + digits.padEnd(point, "0");
  return `${sign}${digits.slice(0, point)}.${digits.slice(point)}`;
}

/** `n` écrit dans la langue `lang` (étiquette BCP 47), négatif avec « − » ; langue inconnue : point décimal, sans groupes. */
export function formatNumber(n: number, lang: string): string {
  const plain = plainNumber(n);
  const sep = separators(lang);
  const m = /^(-?)(\d+)(?:\.(\d+))?$/.exec(plain);
  if (!m) return plain;
  const [, minus, int, frac] = m;
  const sign = minus && MINUS;
  if (!sep) return sign + plain.slice(minus.length);
  const grouped = int.length < 5 ? int : int.replace(/\B(?=(\d{3})+$)/g, sep.group);
  return sign + grouped + (frac === undefined ? "" : sep.decimal + frac);
}

const escape = (s: string) => s.replace(/[.]/g, "\\.");

/**
 * Saisie d'une cellule : `parseCell`, plus les milliers de la langue quand ils
 * s'écrivent « , » ou « . » (« 12,345.5 » en anglais) : un nombre affiché
 * groupé et ressaisi reste un nombre. Comme à l'affichage, il n'y a de groupes
 * qu'à partir de 10 000 : « 3.141 » en allemand reste 3,141.
 */
export function parseNumber(raw: string, lang: string): number | string | null {
  const sep = separators(lang);
  const t = raw.trim();
  if (sep && (sep.group === "," || sep.group === ".")) {
    const g = escape(sep.group);
    const d = escape(sep.decimal);
    if (new RegExp(`^[+\\-${MINUS}]?(?:[1-9]\\d{1,2}(?:${g}\\d{3})+|[1-9]\\d{0,2}(?:${g}\\d{3}){2,})(?:${d}\\d+)?$`).test(t))
      return Number(t.split(sep.group).join("").replace(sep.decimal, ".").replace(MINUS, "-"));
  }
  return parseCell(t);
}
