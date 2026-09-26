// Graphiques dans le deck, côté interface : lecture d'un élément `chart`.
// Le cœur Rust valide, convertit et applique ; rien n'est modifié ici.

import type { Element } from "../api";
import type { Cell } from "./chartTypes";

/** Types de graphique ; en changer convertit les données (requête `chart_kind`). */
export type ChartKind = "bar" | "line" | "pie" | "scatter" | "radar" | "waterfall";
export const CHART_KINDS: readonly ChartKind[] = ["bar", "line", "pie", "scatter", "radar", "waterfall"];

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

/** Groupe d'empilement que pose l'interface. */
const STACK = "total";

const cartesian = (s: Series) => s.type === "bar" || s.type === "line";

/** Barres ou lignes à plusieurs séries : l'empilement a un sens. */
export function stackable(el: Element): boolean {
  const series = optionOf(el)?.series ?? [];
  return series.length > 1 && series.every(cartesian);
}

/** Toutes les séries partagent un même groupe d'empilement. */
export function isStacked(el: Element): boolean {
  const series = optionOf(el)?.series ?? [];
  return series.length > 0 && series.every((s) => typeof s.stack === "string" && s.stack === series[0].stack);
}

/**
 * Patch `set_chart_option` qui empile (ou non) toutes les séries. Un merge
 * patch remplace le tableau entier : chaque série garde ses autres réglages.
 */
export function stackPatch(el: Element, stacked: boolean): { series: Series[] } {
  const series = (optionOf(el)?.series ?? []).map(({ stack: _stack, ...rest }) => (stacked ? { ...rest, stack: STACK } : rest));
  return { series };
}
