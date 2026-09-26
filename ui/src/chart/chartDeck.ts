// Graphiques dans le deck, côté interface : lecture d'un élément `chart`.
// Le cœur Rust valide, convertit et applique ; rien n'est modifié ici.

import type { Element } from "../api";
import type { Cell } from "./chartTypes";

/** Types de graphique ; en changer convertit les données (requête `chart_kind`). */
export type ChartKind = "bar" | "line" | "pie" | "scatter" | "radar";
export const CHART_KINDS: readonly ChartKind[] = ["bar", "line", "pie", "scatter", "radar"];

type Series = { type: string; [key: string]: unknown };
type Option = { dataset?: { source: Cell[][] }; series?: Series[]; [key: string]: unknown };

const optionOf = (el: Element): Option | null =>
  el.kind === "chart" && typeof el.option === "object" && el.option !== null ? (el.option as Option) : null;

/**
 * Corps d'un nouveau graphique : données en ligne, sans noms. La session les
 * range dans `dataset` avec des en-têtes dans la langue de l'utilisateur.
 */
export function starterChart() {
  return {
    kind: "chart" as const,
    option: {
      xAxis: { type: "category", data: ["1", "2", "3"] },
      series: [{ type: "bar", data: [3, 5, 2] }],
    },
  };
}

/** `dataset.source` d'un graphique (ligne 0 = en-têtes), ou `null`. */
export function chartSource(el: Element): Cell[][] | null {
  return optionOf(el)?.dataset?.source ?? null;
}

/** Type de la première série d'un graphique, ou `null`. */
export function chartKind(el: Element): string | null {
  return optionOf(el)?.series?.[0]?.type ?? null;
}
