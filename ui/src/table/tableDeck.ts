// Tableaux côté interface : leurs options et le tableau de départ, nommé ici dans la
// langue de l'utilisateur, faute de canonicalisation des tableaux côté session.
// Le cœur Rust lit, valide et applique.

import type { Cell } from "../chart/chartTypes";
import { t } from "../i18n";

export type ColumnAlign = "auto" | "start" | "center" | "end";
export const COLUMN_ALIGNS: readonly ColumnAlign[] = ["auto", "start", "center", "end"];

export type TableFlag = "header-row" | "header-column" | "striped";

/** Une valeur de `<select>` ou de la session ramenée à un alignement connu, sinon `auto`. */
export const asColumnAlign = (v: string | undefined): ColumnAlign => COLUMN_ALIGNS.find((a) => a === v) ?? "auto";

/** Tableau neuf : trois colonnes nommées, deux lignes. */
export function starterTable(): { dataset: { source: Cell[][] } } {
  const header = [1, 2, 3].map((n) => t("ui-table-column", { n }));
  return { dataset: { source: [header, ["", 0, 0], ["", 0, 0]] } };
}
