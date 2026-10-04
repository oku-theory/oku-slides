// Cellules d'une grille : collage TSV et conversion d'une saisie, partagés par
// les tableaux du dock et des graphiques.

/** "1 234,5" → 1234.5 ; vide → null ; sinon texte. */
export function parseCell(raw: string): number | string | null {
  const t = raw.trim();
  if (t === "") return null;
  const n = Number(t.replace(/[\s  ]/g, "").replace(",", "."));
  return Number.isFinite(n) ? n : t;
}

/** Case collée : cochée sauf si vide ou fausse (« false », « faux », « 0 », « non »…). */
export function parseCheck(raw: string): boolean {
  const t = raw.trim().toLowerCase();
  return t !== "" && !["false", "faux", "0", "non", "no"].includes(t);
}

/** Texte collé (tableur) : une ligne par saut de ligne, une cellule par tabulation. */
export function parseTsv(text: string): string[][] {
  return text
    .replace(/\r\n?/g, "\n")
    .replace(/\n$/, "")
    .split("\n")
    .map((line) => line.split("\t"));
}

/** Saisie d'une colonne de liste (miroir de `ColumnKind` côté Rust). */
export type ColumnKind = "text" | "rich" | "number" | "flag" | "select" | "asset";

/** Saisie `raw` convertie au type de colonne ; `ok: false` si elle n'y tient pas. */
export function coerce(
  kind: ColumnKind,
  raw: string,
  options: readonly string[] = [],
): { ok: true; value: unknown } | { ok: false } {
  const t = raw.trim();
  if (kind === "number") {
    const c = parseCell(t);
    return typeof c === "number" ? { ok: true, value: c } : { ok: false };
  }
  if (kind === "flag") return { ok: true, value: parseCheck(t) };
  if (kind === "select") return options.includes(t) ? { ok: true, value: t } : { ok: false };
  if (kind === "asset") return { ok: false };
  return { ok: true, value: t };
}
