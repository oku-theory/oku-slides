// Graphiques dans le deck, côté interface : lecture d'un élément `chart`.
// Le cœur Rust valide, convertit et applique ; rien n'est modifié ici.

import type { Element } from "../api";
import type { Cell } from "./chartTypes";

/** Types de graphique ; en changer convertit les données (requête `chart_kind`). */
export type ChartKind = "bar" | "line" | "pie" | "scatter" | "radar" | "waterfall";
export const CHART_KINDS: readonly ChartKind[] = ["bar", "line", "pie", "scatter", "radar", "waterfall"];

type Series = { type: string; [key: string]: unknown };
type Option = { dataset?: { source: Cell[][] }; series?: Series[]; [key: string]: unknown };

const optionOf = (el: Element): Option | null => {
  const option = el.params?.option;
  return el.kind === "chart" && typeof option === "object" && option !== null ? (option as Option) : null;
};

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

/** `dataset.source` d'un graphique (ligne 0 = en-têtes), ou `null`. */
export function chartSource(el: Element): Cell[][] | null {
  return optionOf(el)?.dataset?.source ?? null;
}

/** Type de la première série d'un graphique, ou `null`. */
export function chartKind(el: Element): string | null {
  return optionOf(el)?.series?.[0]?.type ?? null;
}

/**
 * Colonnes de cases à cocher de la grille : la colonne Total d'une cascade,
 * celle qui suit les valeurs (forme canonique), si elle existe.
 */
export function checkboxColumns(el: Element): number[] {
  const width = chartSource(el)?.[0]?.length ?? 0;
  return chartKind(el) === "waterfall" && width > 2 ? [2] : [];
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

/**
 * Séries qu'on peut colorer, nommées par leur en-tête (forme canonique) :
 * barres et lignes lisent la colonne i + 1, un nuage la paire (2i, 2i + 1).
 * Secteurs, radar et cascade suivent la palette : aucune.
 */
export function seriesNames(el: Element): string[] {
  const option = optionOf(el);
  const series = option?.series ?? [];
  const header = option?.dataset?.source?.[0] ?? [];
  const label = (col: number) => String(header[col] ?? "");
  if (series.length > 0 && series.every(cartesian)) return series.map((_, i) => label(i + 1));
  if (series.length > 0 && series.every((s) => s.type === "scatter")) return series.map((_, i) => label(2 * i + 1));
  return [];
}

/**
 * Couleur `#rgb` ou `#rrggbb` ramenée à `#rrggbb` en minuscules ; `null`
 * pour toute autre valeur. Une couleur lue dans le deck passe par là avant
 * d'atteindre le CSS.
 */
export function hexColor(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const m = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(value.trim());
  if (!m) return null;
  const h = m[1].toLowerCase();
  return `#${h.length === 3 ? [...h].map((c) => c + c).join("") : h}`;
}

/** Couleur propre à la série `i` (`itemStyle.color`, normalisée), ou `null` (palette). */
export function seriesColor(el: Element, i: number): string | null {
  const style = optionOf(el)?.series?.[i]?.itemStyle as { color?: unknown } | undefined;
  return hexColor(style?.color);
}

/** Patch `set_chart_option` qui colore la série `i`, ou la rend à la palette. */
export function colorPatch(el: Element, i: number, color: string | null): { series: Series[] } {
  const series = (optionOf(el)?.series ?? []).map((s, j) => {
    if (j !== i) return s;
    const { itemStyle: _itemStyle, ...rest } = s;
    return color ? { ...rest, itemStyle: { color } } : rest;
  });
  return { series };
}

/** Palette des graphiques du thème (apparence), réduite aux couleurs valides. */
export function paletteOf(colors: readonly unknown[] | undefined): string[] {
  return (colors ?? []).map(hexColor).filter((c): c is string => c !== null);
}
