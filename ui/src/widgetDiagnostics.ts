/**
 * Place d'un diagnostic dans le texte d'un fichier : ligne et colonne de
 * Typst (à partir de 1) vers des positions CodeMirror. Typst compte les
 * colonnes en caractères (scalaires Unicode, `chars().count()`), CodeMirror
 * en unités UTF-16 : un caractère hors du plan de base en vaut deux. Le mot
 * sous la colonne est souligné, sinon un caractère ; en fin de ligne, le
 * dernier. Une ligne vide n'a rien à souligner : la plage est un point.
 */
import type { WidgetDiagnostic } from "./api";

/** Unités UTF-16 des `chars` premiers caractères de `text` (tout `text` s'il en a moins). */
function unitsOf(text: string, chars: number): number {
  let units = 0;
  let n = 0;
  for (const ch of text) {
    if (n === chars) break;
    units += ch.length;
    n += 1;
  }
  return units;
}

/** Unités UTF-16 du caractère qui commence à `at` dans `text` (0 en fin de texte). */
function charAt(text: string, at: number): number {
  const cp = text.codePointAt(at);
  return cp === undefined ? 0 : cp > 0xffff ? 2 : 1;
}

/** Unités UTF-16 du caractère qui finit à `at` dans `text` (0 en début de texte). */
function charBefore(text: string, at: number): number {
  if (at === 0) return 0;
  const low = text.charCodeAt(at - 1);
  const high = at >= 2 ? text.charCodeAt(at - 2) : 0;
  return low >= 0xdc00 && low <= 0xdfff && high >= 0xd800 && high <= 0xdbff ? 2 : 1;
}

/** Plage soulignée et ligne d'un diagnostic ; `null` s'il n'a pas de place dans `doc`. */
export function diagnosticRange(doc: string, d: WidgetDiagnostic): { from: number; to: number; line: number } | null {
  if (d.line == null) return null;
  const lines = doc.split("\n");
  if (d.line < 1 || d.line > lines.length) return null;
  let start = 0;
  for (let i = 0; i < d.line - 1; i++) start += (lines[i] ?? "").length + 1;
  const text = lines[d.line - 1] ?? "";
  const col = unitsOf(text, Math.max((d.column ?? 1) - 1, 0));
  const word = text.slice(col).match(/^[\w-]+/)?.[0].length ?? charAt(text, col);
  // Après le dernier caractère (parenthèse non fermée…) : on souligne celui-ci.
  if (word === 0) return { from: start + col - charBefore(text, col), to: start + col, line: d.line };
  return { from: start + col, to: start + col + word, line: d.line };
}
