/**
 * Éditions pures de l'éditeur de texte.
 *
 * Texte et sélection en entrée, nouveau texte et nouvelle sélection en sortie :
 * raccourcis de mise en forme, dictée, et plus petit changement à appliquer.
 */

import { encodeLink } from "./markdown";

/** Texte de l'éditeur et sélection après un raccourci de mise en forme. */
export interface Edit {
  value: string;
  start: number;
  end: number;
}

/**
 * Bascule gras ou italique sur la sélection. Les étoiles autour (ou aux bords
 * de la sélection) disent le format actuel : 1 italique, 2 gras, 3 les deux.
 */
export function toggleEmphasis(value: string, start: number, end: number, format: "bold" | "italic"): Edit {
  const original = { start, end };
  // Des étoiles aux deux bords de la sélection comptent comme autour.
  while (end - start > 2 && value[start] === "*" && value[end - 1] === "*") {
    start++;
    end--;
  }
  // Les espaces aux bords ne sont jamais mises en forme (relues comme texte littéral).
  while (start < end && /\s/.test(value[start])) start++;
  while (end > start && /\s/.test(value[end - 1])) end--;
  let k = 0;
  while (k < 3 && value[start - k - 1] === "*" && value[end + k] === "*") k++;
  let bold = k >= 2;
  let italic = k % 2 === 1;
  if (format === "bold") bold = !bold;
  else italic = !italic;
  const marks = "*".repeat((bold ? 2 : 0) + (italic ? 1 : 0));
  // Les étoiles posées se lisent-elles comme écrites ? Sinon la ponctuation de bord sort.
  let [from, to] = [start, end];
  if (marks !== "") {
    for (;;) {
      while (from < to && /\s/.test(value[from])) from++;
      while (to > from && /\s/.test(value[to - 1])) to--;
      const before = from > start ? value[from - 1] : value[start - k - 1];
      const after = to < end ? value[to] : value[end + k];
      if (from < to && !opens(before, value[from])) from++;
      else if (from < to && !closes(value[to - 1], after)) to--;
      else break;
    }
    if (from === to && start < end) return { value, start: original.start, end: original.end };
  }
  const next =
    value.slice(0, start - k) +
    value.slice(start, from) +
    marks +
    value.slice(from, to) +
    marks +
    value.slice(to, end) +
    value.slice(end + k);
  const inner = start - k + (from - start) + marks.length;
  return { value: next, start: inner, end: inner + (to - from) };
}

/** Nature d'une unité UTF-16 contre un délimiteur, comme Lezer la teste. */
function kind(c: string | undefined): "space" | "punct" | "alnum" {
  if (c === undefined || /\s/.test(c)) return "space";
  return /[\p{S}\p{P}]/u.test(c) ? "punct" : "alnum";
}

/** Des étoiles entre `before` et `first` ouvrent (règles de CommonMark). */
function opens(before: string | undefined, first: string): boolean {
  const b = kind(first);
  return b !== "space" && !(b === "punct" && kind(before) === "alnum");
}

/** Des étoiles entre `last` et `after` ferment. */
function closes(last: string, after: string | undefined): boolean {
  const a = kind(last);
  return a !== "space" && !(a === "punct" && kind(after) === "alnum");
}

/**
 * Transforme la sélection en lien. Une adresse sélectionnée devient la cible ;
 * sinon le texte sélectionné devient le libellé et `url` est à remplacer.
 */
export function insertLink(value: string, start: number, end: number): Edit {
  const selected = value.slice(start, end);
  const before = value.slice(0, start);
  const after = value.slice(end);
  if (/^(https?:\/\/|mailto:)\S+$/i.test(selected)) {
    return { value: `${before}[](${encodeLink(selected)})${after}`, start: start + 1, end: start + 1 };
  }
  const url = start + selected.length + 3;
  return { value: `${before}[${selected}](url)${after}`, start: url, end: url + 3 };
}

/** Remplacement de `from` à `to` par `insert`. */
export interface Change {
  from: number;
  to: number;
  insert: string;
}

/**
 * Insère un texte dicté à la place de la sélection.
 *
 * Une espace de part et d'autre quand il en faut ; le curseur se pose après.
 */
export function insertDictated(value: string, start: number, end: number, text: string): Edit {
  const before = value.slice(0, start);
  const after = value.slice(end);
  const lead = before && !/[\s(«"']$/.test(before) ? " " : "";
  const trail = after && !/^[\s.,;:!?)»]/.test(after) ? " " : "";
  const caret = (before + lead + text).length;
  return { value: before + lead + text + trail + after, start: caret, end: caret };
}

/**
 * Plus petit remplacement qui change `before` en `after`.
 *
 * Il peut couper une paire de substitution UTF-16 : le texte obtenu reste juste.
 */
export function minimalChange(before: string, after: string): Change {
  const max = Math.min(before.length, after.length);
  let start = 0;
  while (start < max && before[start] === after[start]) start++;
  let end = 0;
  while (end < max - start && before[before.length - 1 - end] === after[after.length - 1 - end]) end++;
  return { from: start, to: before.length - end, insert: after.slice(start, after.length - end) };
}
