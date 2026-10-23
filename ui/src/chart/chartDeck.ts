// Graphiques côté interface : leurs types.
// Le cœur Rust lit, valide, convertit et applique ; rien n'est modifié ici.

/** Types de graphique ; en changer convertit les données (modification `kind`). */
export type ChartKind = "bar" | "line" | "pie" | "scatter" | "radar" | "waterfall";
export const CHART_KINDS: readonly ChartKind[] = ["bar", "line", "pie", "scatter", "radar", "waterfall"];

/** Le type `value` s'il est connu, sinon `null`. */
export const asChartKind = (value: string | undefined): ChartKind | null => CHART_KINDS.find((k) => k === value) ?? null;
