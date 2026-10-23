// Tableaux côté interface : leurs options (alignements, drapeaux).
// Le cœur Rust lit, valide et applique ; le tableau de départ vient du pack (`examples`).

export type ColumnAlign = "auto" | "start" | "center" | "end";
export const COLUMN_ALIGNS: readonly ColumnAlign[] = ["auto", "start", "center", "end"];

export type TableFlag = "header-row" | "header-column" | "striped";

/** Une valeur de `<select>` ou de la session ramenée à un alignement connu, sinon `auto`. */
export const asColumnAlign = (v: string | undefined): ColumnAlign => COLUMN_ALIGNS.find((a) => a === v) ?? "auto";
