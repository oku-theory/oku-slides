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
  // Des étoiles aux deux bords de la sélection comptent comme autour.
  while (end - start > 2 && value[start] === "*" && value[end - 1] === "*") {
    start++;
    end--;
  }
  let k = 0;
  while (k < 3 && value[start - k - 1] === "*" && value[end + k] === "*") k++;
  let bold = k >= 2;
  let italic = k % 2 === 1;
  if (format === "bold") bold = !bold;
  else italic = !italic;
  const marks = "*".repeat((bold ? 2 : 0) + (italic ? 1 : 0));
  const next = value.slice(0, start - k) + marks + value.slice(start, end) + marks + value.slice(end + k);
  const shift = marks.length - k;
  return { value: next, start: start + shift, end: end + shift };
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
