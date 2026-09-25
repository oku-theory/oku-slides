// Graphiques dans le deck, côté interface : lecture d'un élément `chart` et
// construction des ops. Le cœur Rust valide et applique ; rien n'est modifié ici.

import type { Element } from "../api";
import type { Cell } from "./chartTypes";

/** Types proposés dans l'interface (le cœur en connaît aussi d'autres). */
export type ChartKind = "bar" | "line" | "pie";
export const CHART_KINDS: readonly ChartKind[] = ["bar", "line", "pie"];

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

/**
 * Patch `set_chart_option` qui change le type du graphique, ou `null` s'il l'a
 * déjà ou n'a pas de dataset (les séries portent alors leurs données : les
 * remplacer les effacerait).
 */
export function kindPatch(el: Element, kind: ChartKind): Record<string, unknown> | null {
  const option = optionOf(el);
  if (!option?.dataset || chartKind(el) === kind) return null;
  if (kind === "pie") return { xAxis: null, yAxis: null, series: [{ type: "pie" }] };
  // Barres et lignes : une série par colonne de valeurs, comme la grille.
  const width = option.dataset?.source[0]?.length ?? 2;
  const series = Array.from({ length: Math.max(1, width - 1) }, () => ({ type: kind }));
  return { xAxis: { type: "category" }, yAxis: { type: "value" }, series };
}
