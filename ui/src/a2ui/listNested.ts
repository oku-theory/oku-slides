// Liste imbriquée d'un tableau de liste (t-495) : la colonne `list` d'une ligne s'édite dans un
// tableau de détail, dont chaque changement remonte au tableau parent adressé `ligne/colonne`.

import type { ListOp } from "../api";

/** Une ligne d'un tableau de liste : son id, puis ses valeurs par colonne. */
export type Row = { id: string } & Record<string, unknown>;

/** Un changement d'un tableau de liste : un `ListOp`, un collage, ou celui d'une liste imbriquée. */
export type Change = ListOp | { op: "replace"; items: Row[] } | { op: "nested"; path: string; change: ListOp };

/** Les sous-items de la colonne `key` de `row` (absente : aucun). */
export function subRows(row: Row, key: string): Row[] {
  const v = row[key];
  return Array.isArray(v) ? v.filter((r): r is Row => typeof r === "object" && r !== null && typeof r.id === "string") : [];
}

/**
 * Le changement du tableau parent pour un changement `change` de la liste imbriquée `key` de la
 * ligne `row` : un collage remplace la colonne entière (`set_field`), un `ListOp` part adressé
 * `ligne/colonne` ; une liste imbriquée n'en porte pas d'autre (un seul niveau) : rien ne part.
 */
export function lift(row: string, key: string, change: Change): Change | null {
  if (change.op === "replace") return { op: "set_field", item: row, field: key, value: change.items };
  if (change.op === "nested") return null;
  return { op: "nested", path: `${row}/${key}`, change };
}

/** Le markdown des cellules riches de la liste imbriquée `key` de `row` : clés `ligne/colonne/sous-id`. */
export function subMarkdown(
  markdown: Record<string, Record<string, string>>,
  row: string,
  key: string,
): Record<string, Record<string, string>> {
  const prefix = `${row}/${key}/`;
  return Object.fromEntries(Object.entries(markdown).filter(([k]) => k.startsWith(prefix)).map(([k, v]) => [k.slice(prefix.length), v]));
}
