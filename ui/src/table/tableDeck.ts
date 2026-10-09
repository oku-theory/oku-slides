// Tableaux dans le deck, côté interface : lecture d'un élément `table` (param `data`).
// Le cœur Rust valide et applique ; le tableau de départ est nommé ici, dans la
// langue de l'utilisateur, faute de canonicalisation des tableaux côté session.

import type { Element } from "../api";
import type { Cell } from "../chart/chartTypes";
import { t } from "../i18n";

export type ColumnAlign = "auto" | "start" | "center" | "end";
export const COLUMN_ALIGNS: readonly ColumnAlign[] = ["auto", "start", "center", "end"];

export type TableFlag = "header-row" | "header-column" | "striped";

/** Miroir de `TableOption` (oku-slides-chart) : ligne 0 = noms de colonnes. */
export type TableOption = {
  /** `ids` : un par ligne, `columns` : un par colonne ; tirés par le cœur. */
  dataset: { source: Cell[][]; ids?: string[]; columns?: string[] };
  "header-row"?: boolean;
  "header-column"?: boolean;
  striped?: boolean;
  align?: ColumnAlign[];
  /** Options du graphique d'origine, gardées pour le restaurer ; jamais éditées par l'UI. */
  chart?: unknown;
};

/** Valeurs par défaut des drapeaux, celles de `TableOption` côté Rust. */
export const TABLE_DEFAULTS: Readonly<Record<TableFlag, boolean>> = { "header-row": true, "header-column": false, striped: false };

/** Le drapeau `key`, ou sa valeur par défaut. */
export const flag = (option: TableOption, key: TableFlag): boolean => option[key] ?? TABLE_DEFAULTS[key];

/** Options d'un élément `table`, ou `null`. */
export function tableOption(el: Element): TableOption | null {
  const data: unknown = el.params?.data;
  if (el.kind !== "table" || typeof data !== "object" || data === null) return null;
  const source = (data as { dataset?: { source?: unknown } }).dataset?.source;
  const isCell = (c: unknown) => c === null || ["string", "number", "boolean"].includes(typeof c);
  const wellFormed = Array.isArray(source) && source.length > 0 && source.every((row) => Array.isArray(row) && row.every(isCell));
  return wellFormed ? (data as TableOption) : null;
}

/** Tableau neuf : trois colonnes nommées, deux lignes. */
export function starterTable(): TableOption {
  const header = [1, 2, 3].map((n) => t("ui-table-column", { n }));
  return { dataset: { source: [header, ["", 0, 0], ["", 0, 0]] } };
}

/** Alignement de la colonne `i` ; sans entrée, `auto`. */
export const columnAlign = (option: TableOption, i: number): ColumnAlign => option.align?.[i] ?? "auto";

/** Une valeur de `<select>` ramenée à un alignement connu, sinon `auto`. */
export const asColumnAlign = (v: string): ColumnAlign => COLUMN_ALIGNS.find((a) => a === v) ?? "auto";

/**
 * Patch `set_table_option` qui aligne la colonne `i`. Un merge patch remplace
 * le tableau entier ; les `auto` de fin sont retirés (`null` : plus d'entrée).
 */
export function alignPatch(option: TableOption, i: number, a: ColumnAlign): { align: ColumnAlign[] | null } {
  const width = option.dataset.source[0]?.length ?? 0;
  const align = Array.from({ length: width }, (_, j) => (j === i ? a : columnAlign(option, j)));
  while (align.length > 0 && align[align.length - 1] === "auto") align.pop();
  return { align: align.length > 0 ? align : null };
}
