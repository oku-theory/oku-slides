// Graphiques côté interface : leurs types et l'option d'un graphique neuf.
// Le cœur Rust lit, valide, convertit et applique ; rien n'est modifié ici.

/** Types de graphique ; en changer convertit les données (modification `kind`). */
export type ChartKind = "bar" | "line" | "pie" | "scatter" | "radar" | "waterfall";
export const CHART_KINDS: readonly ChartKind[] = ["bar", "line", "pie", "scatter", "radar", "waterfall"];

/** Le type `value` s'il est connu, sinon `null`. */
export const asChartKind = (value: string | undefined): ChartKind | null => CHART_KINDS.find((k) => k === value) ?? null;

/**
 * Option d'un nouveau graphique : données en ligne, sans noms. La session les
 * range dans `dataset` avec des en-têtes dans la langue de l'utilisateur.
 */
export function starterChart() {
  return {
    xAxis: { type: "category", data: ["1", "2", "3"] },
    series: [{ type: "bar", data: [3, 5, 2] }],
  };
}
