// Cellules d'une grille : collage TSV et conversion d'une saisie, partagés par
// les tableaux du dock, la grille des graphiques et celle du widget tableau.

// Décimal sans ambiguïté : signe, partie entière sans zéro initial (hors « 0 »),
// milliers par espace / NBSP / fine insécable (groupes de 3), partie décimale « . » ou « , ».
const NUMBER =
  /^[+-]?(0|[1-9]\d*|[1-9]\d{0,2}(?:[ \u00a0\u202f]\d{3})+)(?:[.,]\d+)?$/;

/** "1 234,5" → 1234.5 ; vide → null ; sinon (« 007 », « 1e3 »…) texte. */
export function parseCell(raw: string): number | string | null {
  const t = raw.trim();
  if (t === "") return null;
  if (!NUMBER.test(t)) return t;
  return Number(t.replace(/[ \u00a0\u202f]/g, "").replace(",", "."));
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
export type ColumnKind =
  "text" | "rich" | "number" | "flag" | "select" | "asset";

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
  if (kind === "select")
    return options.includes(t) ? { ok: true, value: t } : { ok: false };
  if (kind === "asset") return { ok: false };
  return { ok: true, value: t };
}
