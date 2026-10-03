/**
 * Place d'un diagnostic dans le texte d'un fichier : ligne et colonne de
 * Typst (à partir de 1, colonnes en caractères) vers des positions
 * CodeMirror. Le mot sous la colonne est souligné, sinon un caractère.
 */
import type { WidgetDiagnostic } from "./api";

/** Plage soulignée et ligne d'un diagnostic ; `null` s'il n'a pas de place dans `doc`. */
export function diagnosticRange(doc: string, d: WidgetDiagnostic): { from: number; to: number; line: number } | null {
  if (d.line == null) return null;
  const lines = doc.split("\n");
  if (d.line < 1 || d.line > lines.length) return null;
  let start = 0;
  for (let i = 0; i < d.line - 1; i++) start += (lines[i] ?? "").length + 1;
  const text = lines[d.line - 1] ?? "";
  const col = Math.min(Math.max((d.column ?? 1) - 1, 0), text.length);
  const word = text.slice(col).match(/^[\w-]+/)?.[0].length ?? 1;
  const end = Math.min(col + Math.max(word, 1), text.length);
  return { from: start + col, to: start + Math.max(end, col), line: d.line };
}
