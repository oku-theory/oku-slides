// Écriture des nombres d'une grille dans la langue du deck, comme le rendu
// (crates/oku-slides-chart/src/numbers.rs, typst/oku-slides/src/numbers.typ) :
// séparateur décimal, milliers groupés à partir de 10 000.
//
// Pas `Intl.NumberFormat(lang)` seul : il écrit des chiffres arabes ou
// bengalis, groupe à l'indienne (hi, bn) ; la grille doit montrer ce que la
// slide affiche. Intl ne sert qu'à l'écriture courte (10 décimales au plus).

import { parseCell } from "./cells";

type Separators = { decimal: string; group: string };

const NNBSP = " ";
const NBSP = " ";

const SEPARATORS: Record<string, Separators> = {
  fr: { decimal: ",", group: NNBSP },
  ...Object.fromEntries(["en", "ja", "ko", "zh", "th", "hi", "bn", "ar"].map((l) => [l, { decimal: ".", group: "," }])),
  ...Object.fromEntries(["de", "nl", "it", "es", "pt", "tr", "vi"].map((l) => [l, { decimal: ",", group: "." }])),
  ...Object.fromEntries(["pl", "uk", "ru"].map((l) => [l, { decimal: ",", group: NBSP }])),
};

function separators(lang: string): Separators | undefined {
  return SEPARATORS[lang.split(/[-_]/)[0].toLowerCase()];
}

/** `n` écrit dans la langue `lang` (étiquette BCP 47) ; langue inconnue : point décimal, sans groupes. */
export function formatNumber(n: number, lang: string): string {
  const plain = n.toLocaleString("en-US", { maximumFractionDigits: 10, useGrouping: false });
  const sep = separators(lang);
  const m = /^(-?)(\d+)(?:\.(\d+))?$/.exec(plain);
  if (!sep || !m) return plain;
  const [, sign, int, frac] = m;
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
    if (new RegExp(`^[+-]?(?:[1-9]\\d{1,2}(?:${g}\\d{3})+|[1-9]\\d{0,2}(?:${g}\\d{3}){2,})(?:${d}\\d+)?$`).test(t))
      return Number(t.split(sep.group).join("").replace(sep.decimal, "."));
  }
  return parseCell(t);
}
